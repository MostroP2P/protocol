# Pay bond invoice

When the receiving Mostro node has the anti-abuse bond feature enabled, takers must lock a small Lightning hold invoice as a security deposit before the trade flow proceeds. Mostro asks for this payment with the `pay-bond-invoice` action.

The bond is **independent from the trade escrow**: it is a second hold invoice with its own payment hash, and it is **released on every normal exit path** — completion, cancel before timeout, or dispute resolution where the solver does not direct otherwise. Bonds are typically around **1% of the trade amount** (with an operator-configured floor), much smaller than the trade hold invoice that may follow.

## Direction and trigger

- **Direction:** Mostro → user (the taker).
- **Trigger:** Sent immediately after a successful `take-buy` / `take-sell` when the operator has bonds enabled.
- **Order status:** Transitions to `waiting-taker-bond` at this point. See [statuses](./order_event.md#tags) for context — `waiting-taker-bond` is a sub-state of the publicly advertised `in-progress` status on the addressable order event.

## Mostro message to the taker

The rumor's content has the same shape as `pay-invoice`; only the action discriminator differs:

```json
[
  {
    "order": {
      "version": 1,
      "id": "<Order Id>",
      "action": "pay-bond-invoice",
      "payload": {
        "payment_request": [
          {
            "id": "<Order Id>",
            "kind": "sell",
            "status": "waiting-taker-bond",
            "amount": 7851,
            "fiat_code": "VES",
            "fiat_amount": 100,
            "payment_method": "face to face",
            "premium": 1,
            "created_at": 1698937797
          },
          "lnbcrt780n1pj59wmepp5..."
        ]
      }
    }
  },
  null
]
```

## Expected client behaviour

- Decode the bolt11 and surface it to the user **explicitly as a bond**, not as the trade escrow. Do not reuse the same wording or UI step that you use for `pay-invoice`.
- Pay it. The HTLC enters `Accepted` state — the sats are locked in the taker's wallet, not captured.
- Do not collapse this into the same UI step as any subsequent `pay-invoice` on the same order. They are independent HTLCs (different payment hashes), and the user must explicitly approve each.

## Follow-up flow

Once the bond HTLC is `Accepted`, Mostro proceeds with the normal trade flow:

- **Sell order taken (taker = buyer):** order status moves to `waiting-buyer-invoice`; the taker next receives `add-invoice` to provide a payout invoice.
- **Buy order taken (taker = seller):** order status moves to `waiting-payment`; the taker next receives `pay-invoice` for the **trade hold invoice**.

> **Important — buy order taken (seller-as-taker):** this is the only flow on which a single user pays **two hold invoices in sequence on the same order** — first the bond (`pay-bond-invoice`), then the trade escrow (`pay-invoice`). They arrive as distinct actions and must be presented to the user as separate steps. Do not auto-pay either, do not coalesce them, and make the distinction obvious in the UI; this is the most error-prone path for client developers.

## Order status: `waiting-taker-bond`

`waiting-taker-bond` is a sub-state of the publicly advertised `in-progress` status on the addressable order event. It means: a taker has been matched to this order, but Mostro is waiting for them to pay the anti-abuse bond hold invoice before the trade flow begins.

It sits between two existing statuses:

- `pending` — order is advertised, no taker matched yet.
- `waiting-taker-bond` — taker matched, waiting on the bond payment.
- `waiting-payment` — trade hold invoice expected from the seller.
- `waiting-buyer-invoice` — payout invoice expected from the buyer.

Transitions in:

- From `pending`, after a successful `take-buy` / `take-sell` when bonds are enabled.

Transitions out:

- To `waiting-payment` (buy order taken) or `waiting-buyer-invoice` (sell order taken) — once the bond HTLC is `Accepted`.
- Back to `pending` — if the bond bolt11 is never paid and expires, or the taker cancels before locking the bond.

## Failure modes

- The user never pays the bond bolt11 → the invoice expires, the order is rolled back to `pending`, and the take is undone.
- The user pays the bond and then cancels before trade completion → the bond HTLC is cancelled and the funds return to the taker.
- Slashing conditions (solver-directed dispute resolution, or timeout while in a waiting state) can settle the bond rather than release it. These paths are documented under [Admin Settle order](./admin_settle_order.md) and [Admin Cancel order](./admin_cancel_order.md), and in the Mostro daemon's anti-abuse bond specification.

## Backwards compatibility

Clients that do not recognise `pay-bond-invoice` will silently drop the message. From the user's perspective the take will appear stuck and eventually time out. Clients should:

- Recognise the action explicitly and surface it to the user.
- If unable to handle it (e.g. an older build talking to a bond-enabled node), present a clear error rather than silently retrying the take.

The Mostro info event will gain bond-related tags so clients can detect bond-enabled nodes ahead of a take — see [Other events published by Mostro](./other_events.md#mostro-instance-status).
