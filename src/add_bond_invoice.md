# Bond payout invoice

The `add-bond-invoice` action is the counterparty-direction dual of [`pay-bond-invoice`](./pay_bond_invoice.md): where `pay-bond-invoice` asks a taker to lock a bond at the start of a trade, `add-bond-invoice` asks the non-slashed counterparty to provide a Lightning invoice for their share of a bond that has just been slashed. The flow is bi-directional — Mostro first sends a request message, the counterparty replies with a bolt11 — and it only fires after a solver-directed slash via [Admin Settle order](./admin_settle_order.md) / [Admin Cancel order](./admin_cancel_order.md) or a timeout-driven slash.

## Direction and trigger

- **Mostro → counterparty (request).** Mostro emits this message after a [`bond_resolution`](./admin_settle_order.md#bond-resolution-payload) settles the *other* party's bond. The request is re-emitted periodically until the counterparty replies or the claim window expires; clients must treat the repeats as idempotent reminders, not as new requests for a fresh invoice.

- **Counterparty → Mostro (reply).** The counterparty replies with a bolt11 sized at the counterparty share (`order.amount` in the request payload). The reply must arrive before the forfeit deadline (see [Forfeit deadline](#forfeit-deadline) below).

## Mostro → counterparty (request)

Mostro sends a single `add-bond-invoice` message to the non-slashed counterparty. The payload is a `bond_payout_request` variant that bundles the order context with the slash anchor:

```json
[
  {
    "order": {
      "version": 1,
      "id": "<Order Id>",
      "action": "add-bond-invoice",
      "payload": {
        "bond_payout_request": {
          "order": {
            "id": "<Order Id>",
            "kind": "sell",
            "amount": 500,
            "fiat_code": "VES",
            "fiat_amount": 100,
            "payment_method": "face to face",
            "premium": 1
          },
          "slashed_at": 1734000000
        }
      }
    }
  },
  null
]
```

- `bond_payout_request.slashed_at` is the Unix timestamp (seconds, UTC) at which the slash was recorded — see [Forfeit deadline](#forfeit-deadline).

The message carries no hardcoded human-readable deadline text; the client renders that warning locally in the user's own locale from `slashed_at` and the info-event window tag.

## Forfeit deadline

The client computes the forfeit deadline as:

```text
deadline = slashed_at + bond_payout_claim_window_days * 86_400
```

- `slashed_at` is read from the `bond_payout_request` payload above.
- `bond_payout_claim_window_days` is a tag published on the Mostro instance's kind-38385 info event — see [Other events published by Mostro](./other_events.md).

`slashed_at` is shipped on the wire (rather than derived from the time the message lands) so that the deadline is stable across periodic re-deliveries and accurate even if the recipient — or their relay — was offline for several days. A deadline computed from "now + window" at receipt time would silently drift into the future on every retry; reading `slashed_at` from the payload pins it to the moment Mostro actually slashed. Clients **must** read `slashed_at` from the payload and **must not** substitute their local clock.

## Counterparty → Mostro (reply)

The counterparty replies with a Gift wrap Nostr event whose rumor content carries the bolt11 inside the standard `payment_request` array. The invoice carries its own amount, so the array has two elements (per [Payment Request Array Structure](./overview.md#payment-request-array-structure)):

```json
[
  {
    "order": {
      "version": 1,
      "id": "<Order Id>",
      "action": "add-bond-invoice",
      "payload": {
        "payment_request": [
          null,
          "lnbcrt5u1pj59wmepp5..."
        ]
      }
    }
  },
  "<index N signature of the sha256 hash of the serialized first element of content>"
]
```

The reply is signed with the **trade key** of the counterparty side, the side that was *not* slashed, exactly as in any other order-scoped action; see [Keys management](./key_management.md).

## Recipient resolution

The recipient of the request message is the non-slashed counterparty of the trade, derived from the order's `buyer_pubkey` / `seller_pubkey` and the side the solver flagged in [`bond_resolution`](./admin_settle_order.md#bond-resolution-payload):

| Order kind | Solver flag                | Bond on … | Recipient        |
|------------|----------------------------|-----------|------------------|
| `sell`     | `slash_buyer = true`       | taker     | maker (seller)   |
| `sell`     | `slash_seller = true`      | maker     | taker (buyer)    |
| `buy`      | `slash_buyer = true`       | maker     | taker (seller)   |
| `buy`      | `slash_seller = true`      | taker     | maker (buyer)    |

## Expected client behaviour

- **Dispatch on the action discriminator.** Route `add-bond-invoice` through its own handler. Collapsing it with `add-invoice` will misclassify the bolt11 as a trade-payout invoice and lead to user-visible accounting errors.
- **Render the deadline locally.** Combine `slashed_at` from the `bond_payout_request` payload with `bond_payout_claim_window_days` from the kind-38385 info event ([Other events published by Mostro](./other_events.md)). Do not derive the deadline from receive time.
- **Treat re-deliveries as idempotent reminders.** The same outstanding request is being repeated; do not re-prompt the user for a fresh invoice on every retry. `slashed_at` is re-emitted unchanged, so the deadline the client shows must not shift across retries.
- **Sign the reply with the non-slashed side's trade key.** See [Keys management](./key_management.md).

## Failure modes

- The counterparty never replies before the deadline → no Lightning payment arrives and the share is forfeited; the slashed funds remain with the node.
- A reply arrives after the deadline, or from a sender other than the resolved recipient, or after another reply already won the race → Mostro responds with a `cant-do` action carrying reason `not-allowed-by-status`.
- The bolt11 principal does not match the requested counterparty share, or the invoice is otherwise undecodable / expired → Mostro responds with `cant-do` reason `invalid-invoice`.
- On a node where the operator retains 100% of slashed bonds, **no `add-bond-invoice` message is emitted at all**. Clients should not surface a phantom payout request.

On success, the counterparty receives their share as a Lightning payment from the node's wallet. The routing fee is paid separately from that wallet and is not deducted from the principal.
