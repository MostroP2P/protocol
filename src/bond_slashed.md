# Bond slashed notification

The `bond-slashed` action is a notification Mostro sends to a bonded party when their anti-abuse bond has been **settled due to a waiting-state timeout**. It is a forfeiture notice: the bond HTLC has already been claimed into Mostro's wallet by the time this message is sent.

> **Scope.** This action is only emitted on the **timeout slash** path (scheduler-driven, gated by `bond_slash_on_waiting_timeout = "true"` in the Mostro info event — see [Other events published by Mostro](./other_events.md#anti-abuse-bond-policy-tags)). It is **not** sent on the dispute-slash path — when a solver slashes a bond via [`admin-settle`](./admin_settle_order.md) or [`admin-cancel`](./admin_cancel_order.md), the slashed party receives the `admin-settled` / `admin-canceled` confirmation instead.

## Direction and trigger

- **Direction:** Mostro → the party whose bond was slashed.
- **Trigger:** The waiting-state timeout elapsed while the responsible party had not performed their expected trade action (e.g. the seller never paid the hold invoice while in `waiting-payment`, or the buyer never submitted an invoice in `waiting-buyer-invoice`), and the operator has configured `bond_slash_on_waiting_timeout = "true"` in the info event.
- The `amount` in the payload is the **slashed bond amount in satoshis** — not the trade amount.

## Wire format

```json
[
  {
    "order": {
      "version": 1,
      "id": "<Order Id>",
      "action": "bond-slashed",
      "payload": {
        "order": {
          "id": "<Order Id>",
          "kind": "sell",
          "status": null,
          "amount": 785,
          "fiat_code": "VES",
          "fiat_amount": 100,
          "payment_method": "face to face",
          "premium": 1,
          "created_at": null
        }
      }
    }
  },
  null
]
```

The `amount` field in the embedded `SmallOrder` is the **slashed bond amount** (not the original trade amount). For a range-order maker bond, this is the proportional slice amount for the taken sub-order. The `status` field is `null` on this message — the `SmallOrder` carries only the bond context (id, kind, amounts), not an order status; clients should rely on the `bond-slashed` action itself, and on the separate order-status messages that follow (`canceled` for the order, plus a republished NIP-33 event when the order returns to the book).

## What happens next

After `bond-slashed` is sent to the responsible party, Mostro also:

1. **Cancels or republishes the order** depending on which party was responsible:
   - **Maker responsible** (e.g. maker-as-seller never paid the hold invoice): the order is **canceled** (since the maker cannot be trusted to fulfil it). The maker receives the `bond-slashed` notice followed by a `canceled` confirmation.
   - **Taker responsible** (e.g. taker-as-buyer never submitted their invoice): the order is **republished** to the book as `pending` so the maker can be matched again. The taker receives `bond-slashed` followed by `canceled`. The maker's bond (if any) remains `Locked`.

2. **Asks the winning counterparty for a payout invoice** by sending them an [`add-bond-invoice`](./add_bond_invoice.md) message for their share of the slashed bond (governed by `bond_slash_node_share_pct` — the node retains its share, the rest goes to the counterparty).

## Expected client behaviour

- Surface `bond-slashed` to the user as an explicit forfeiture notice, not as a generic cancellation. Suggested wording: *"Your bond of `amount` Sats has been forfeited due to a waiting-state timeout."*
- This action is distinct from `canceled` — clients should not suppress it or merge it into the cancel flow.
- A `canceled` message will typically follow for the order itself; clients should handle both.

## Note on cancels vs. slashes

A cancel sent by either party **before** the timeout elapses never triggers `bond-slashed`. Bonds are always released (never slashed) on user-initiated cancels. Only the automated scheduler-driven timeout slash path emits this action, and only when `slash_on_waiting_timeout = true` is set by the operator.
