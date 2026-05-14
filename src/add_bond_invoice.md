# Bond payout invoice

The `add-bond-invoice` action is the counterparty-direction dual of [`pay-bond-invoice`](./pay_bond_invoice.md): where `pay-bond-invoice` asks a taker to lock a bond at the start of a trade, `add-bond-invoice` asks the non-slashed counterparty to provide a Lightning invoice for their share of a bond that has just been slashed. The flow is bi-directional — Mostro first sends a request DM, the counterparty replies with a bolt11 — and it only fires after a solver-directed slash via [Admin Settle order](./admin_settle_order.md) / [Admin Cancel order](./admin_cancel_order.md) (or, in a future release, a timeout-driven slash). Although the request DM mirrors the shape of `add-invoice`, the routing layer is intentionally disjoint: clients must dispatch on the action discriminator alone — `add-invoice` is the trade-payout invoice request to the buyer, `add-bond-invoice` is the bond-payout invoice request to whichever side was not slashed. Do not collapse the two handlers using order status, memo content, or any other heuristic.

## Direction and trigger

- **Mostro → counterparty (request).** The daemon emits this DM after a `BondResolution` settles the *other* party's bond. The scheduler re-emits the request once per `payout_invoice_window_seconds` until the counterparty replies or the claim window expires; clients must treat the repeats as idempotent reminders, not as new requests for a fresh invoice.
- **Counterparty → Mostro (reply).** The counterparty replies with a bolt11 for `amount_sats − node_share_sats`, where `node_share_sats` is the slice the operator retains via `slash_node_share_pct`. The reply must arrive before the forfeit deadline anchored on `slashed_at + payout_claim_window_days × 86_400`.

## Mostro → counterparty (request)

Mostro sends two DMs back-to-back to the non-slashed counterparty. The first carries the `add-bond-invoice` action and a `SmallOrder` echo whose `amount` is the counterparty's share of the slashed bond:

```json
[
  {
    "order": {
      "version": 1,
      "id": "<Order Id>",
      "action": "add-bond-invoice",
      "payload": {
        "order": {
          "id": "<Order Id>",
          "kind": "sell",
          "status": "settled-hold-invoice",
          "amount": 500,
          "fiat_code": "VES",
          "fiat_amount": 100,
          "payment_method": "face to face",
          "premium": 1,
          "buyer_trade_pubkey": null,
          "seller_trade_pubkey": null
        }
      }
    }
  },
  null
]
```

Alongside it, the daemon enqueues a `send-dm` text message that carries the human-readable forfeit deadline:

```json
[
  {
    "order": {
      "version": 1,
      "id": "<Order Id>",
      "action": "send-dm",
      "payload": {
        "text_message": "Bond payout pending. Submit a Lightning invoice for 500 sats via add-bond-invoice before 2026-05-29T18:00:00Z or your share will be forfeited."
      }
    }
  },
  null
]
```

These are two distinct messages arriving in sequence — the first drives the dispatch and carries the requested amount; the second is a user-facing reminder of the deadline. Clients should render them together but must not assume they are merged on the wire.

## Counterparty → Mostro (reply)

The counterparty replies with a Gift wrap Nostr event whose rumor content carries the bolt11 inside the standard `payment_request` 3-tuple. The third element is `null` because the invoice carries its own amount:

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
          "lnbcrt5u1pj59wmepp5...",
          null
        ]
      }
    }
  },
  "<index N signature of the sha256 hash of the serialized first element of content>"
]
```

The reply is signed with the **trade key** of the counterparty side — the side that was *not* slashed — exactly as in any other order-scoped action; see [Keys management](./key_management.md).

## Recipient resolution

The recipient of the request DM is the non-slashed counterparty of the trade, derived from the order's `buyer_pubkey` / `seller_pubkey` and the side the solver flagged in `BondResolution`:

| Order kind | Solver flag                | Bond on … | Recipient        |
|------------|----------------------------|-----------|------------------|
| `sell`     | `slash_buyer = true`       | taker     | maker (seller)   |
| `sell`     | `slash_seller = true`      | maker     | taker (buyer)    |
| `buy`      | `slash_buyer = true`       | maker     | taker (seller)   |
| `buy`      | `slash_seller = true`      | taker     | maker (buyer)    |

This is the same §3.1 mapping used elsewhere in the bond specification.

## Expected client behaviour

- Dispatch on the action discriminator. The `add-bond-invoice` request must not be routed through any handler used for `add-invoice`; collapsing the two paths will misclassify the invoice as a trade-payout invoice and lead to user-visible accounting errors.
- Surface the requested `amount` and the forfeit deadline prominently. The deadline only appears in the companion `send-dm` text message; clients should join the two messages in the UI rather than dropping the text DM.
- Treat re-deliveries inside `payout_invoice_window_seconds` as idempotent reminders. Re-prompting the user with a fresh invoice request each tick is wrong; the same outstanding request is being repeated.
- Sign the reply with the trade key of the counterparty side — the side that was *not* slashed — as documented under [Keys management](./key_management.md).

## Failure modes

- The counterparty never replies before the forfeit deadline (`slashed_at + payout_claim_window_days × 86_400`) → the bond row transitions to `forfeited` and the daemon stops emitting the request; the slashed funds remain with the node.
- A reply arrives after forfeit, or after another reply already won the race, or from a sender other than the resolved recipient → Mostro rejects with `cant-do` reason `not-allowed-by-status`.
- The bolt11 amount does not match the counterparty share, or the invoice is otherwise undecodable / expired → Mostro rejects with `cant-do` reason `invalid-invoice`.
- The operator has configured `slash_node_share_pct = 1.0` → no counterparty leg exists; the daemon settles the bond to itself and the row goes straight to `slashed`. No `add-bond-invoice` DM is emitted on such nodes.

On success, the next scheduler tick (≤ 60 s) performs `settle_hold_invoice(preimage)` followed by `send_payment` capped at `MostroSettings::max_routing_fee`, and the bond row transitions to `slashed`.

## Backwards compatibility

`Action::AddBondInvoice` was introduced in `mostro-core` 0.11.2 and is serde-additive: clients still on 0.11.0 / 0.11.1 will fail to deserialize the request DM and silently drop it. From the counterparty's perspective the bond payout simply never arrives, and the bond row eventually transitions to `forfeited` once the claim window elapses. Operators must not enable a slash regime with `slash_node_share_pct < 1.0` in production until clients in the wild have adopted the `mostro-core` release that ships `Action::AddBondInvoice`; otherwise users on older builds will silently lose their share. Clients should:

- Recognise the action explicitly and surface the request to the user as a bond payout, distinct from any trade-payout invoice request.
- If unable to handle it (e.g. an older build talking to a Phase 3 bond-enabled node), present a clear error rather than silently dropping the DM.

The Mostro info event will gain bond-related tags so clients can detect bond-enabled nodes ahead of a take — see [Other events published by Mostro](./other_events.md#mostro-instance-status).
