# Pay bond invoice

When the receiving Mostro node has the anti-abuse bond feature enabled, makers and/or takers must lock a small Lightning hold invoice as a security deposit before the trade flow proceeds. Mostro asks for this payment with the `pay-bond-invoice` action.

The bond is **independent from the trade escrow**: it is a second hold invoice with its own payment hash, and it is **released on every normal exit path** — completion, cancel before timeout, or dispute resolution where the solver does not direct otherwise. Bonds are typically around **1% of the trade amount** (with an operator-configured floor), much smaller than the trade hold invoice that may follow.

The `bond_apply_to` tag in the Mostro info event (kind 38385) tells clients which sides must post a bond: `"take"`, `"make"`, or `"both"` — see [Other events published by Mostro](./other_events.md#anti-abuse-bond-policy-tags).

## Taker bond

### Direction and trigger

- **Direction:** Mostro → user (the taker).
- **Trigger:** Sent immediately after a successful `take-buy` / `take-sell` when the operator has bonds enabled for takers.
- **Order status:** The published NIP-33 order event keeps the `s` tag at `pending` while the bond is outstanding (per NIP-69's four-bucket model — see [Peer-to-peer Order events. NIP-69](./order_event.md)). The DM payload's embedded `SmallOrder` echo carries the daemon-internal status `waiting-taker-bond` so the recipient client can render the bond-payment phase distinctly, but external observers (other potential takers, order-book aggregators) continue to see the order as `pending` and may still attempt to take it. This is deliberate: a taker who never pays the bond cannot park the order off the book.

### Mostro message to the taker

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

> Note: the `status` value `"waiting-taker-bond"` here is the daemon-internal state echoed in the DM payload. The corresponding NIP-33 addressable order event's `s` tag is still `pending`.

### Expected client behaviour

- Decode the bolt11 and surface it to the user **explicitly as a bond**, not as the trade escrow. Do not reuse the same wording or UI step that you use for `pay-invoice`.
- Pay it. The HTLC enters `Accepted` state — the sats are locked in the taker's wallet, not captured.
- Do not collapse this into the same UI step as any subsequent `pay-invoice` on the same order. They are independent HTLCs (different payment hashes), and the user must explicitly approve each.

### Follow-up flow

Once the bond HTLC is `Accepted`, Mostro proceeds with the normal trade flow:

- **Sell order taken (taker = buyer):** order status moves to `waiting-buyer-invoice`; the taker next receives `add-invoice` to provide a payout invoice.
- **Buy order taken (taker = seller):** order status moves to `waiting-payment`; the taker next receives `pay-invoice` for the **trade hold invoice**.

> **Important — buy order taken (seller-as-taker):** this is the only flow on which a single user pays **two hold invoices in sequence on the same order** — first the bond (`pay-bond-invoice`), then the trade escrow (`pay-invoice`). They arrive as distinct actions and must be presented to the user as separate steps. Do not auto-pay either, do not coalesce them, and make the distinction obvious in the UI; this is the most error-prone path for client developers.

### Daemon status `waiting-taker-bond` (DM payloads only)

In addition to the NIP-69 wire status (`pending` while the bond is outstanding), Mostro tags the order's internal state as `waiting-taker-bond` so it can route subsequent messages correctly. Clients see this value in the `SmallOrder` echo embedded in `pay-bond-invoice` payloads and may use it to drive UI ("Waiting for bond payment"). It does **not** appear on the addressable NIP-33 order event — that one continues to advertise the order as `pending`.

Internal transitions (visible only in DM payload echoes):

- From `pending` → `waiting-taker-bond`, after a successful `take-buy` / `take-sell` when bonds are enabled.
- From `waiting-taker-bond` → `waiting-payment` (buy order taken) or → `waiting-buyer-invoice` (sell order taken), once the bond HTLC is `Accepted`.
- From `waiting-taker-bond` → `pending`, if the bond bolt11 is never paid and expires, or the taker cancels before locking. The published `s` tag was `pending` throughout — observers see only that the take attempt left no trace.

### Failure modes

- The user never pays the bond bolt11 → the invoice expires; the order's NIP-33 status was `pending` throughout, so the rollback only undoes the daemon-internal take state. The order remains takeable.
- The user pays the bond and then cancels before trade completion → the bond HTLC is cancelled and the funds return to the taker.
- Slashing conditions (solver-directed dispute resolution, or timeout while in a waiting state) can settle the bond rather than release it. The solver directs slashing via the `bond_resolution` payload documented under [Admin Settle order](./admin_settle_order.md) and [Admin Cancel order](./admin_cancel_order.md), and the non-slashed counterparty is then asked for their share of the bond via [Bond payout invoice](./add_bond_invoice.md). When a timeout slash fires, the slashed party first receives a [`bond-slashed`](./bond_slashed.md) notification.

---

## Maker bond

When `apply_to` is `"make"` or `"both"`, the **maker** must lock a bond before the order is published to Nostr. The same `pay-bond-invoice` action is used, but the trigger and the order lifecycle differ from the taker case.

### Direction and trigger

- **Direction:** Mostro → user (the maker).
- **Trigger:** Sent in response to a `new-order` message when the operator has bonds enabled for makers, **before** the order is published.
- **Order visibility:** The order is **not published** to Nostr while the maker bond is outstanding. External observers see nothing — no `pending` order event, no order in the book. This differs from the taker bond, where the order remains visible and re-takeable throughout. The daemon tracks this internally as `waiting-maker-bond`, a status visible only in DM payload echoes.

### Mostro message to the maker

The action and wire shape are identical to the taker case; only the embedded `status` field in the `SmallOrder` differs:

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
            "status": "waiting-maker-bond",
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

> Note: `"waiting-maker-bond"` is the daemon-internal status echoed in the DM payload. No NIP-33 order event has been emitted yet at this point.

For **range sell orders**, the bond is sized against `max_amount` (not the individual fiat_amount). The `SmallOrder` echo will reflect the range order fields (`min_amount`, `max_amount`).

### Follow-up after bond locks

Once the maker's bond HTLC is `Accepted`:

1. Mostro publishes the order to Nostr with status `pending` for the first time.
2. Mostro sends the maker the `new-order` confirmation message (same as in the no-bond flow — see [Creating a new sell order](./new_sell_order.md) and [Creating a new buy order](./new_buy_order.md)).

### Daemon status `waiting-maker-bond` (DM payloads only)

Internal transitions (visible only in DM payload echoes):

- On `new-order` receipt → `waiting-maker-bond`, before any NIP-33 event is emitted.
- `waiting-maker-bond` → `pending` (and order published), once the bond HTLC is `Accepted`.
- `waiting-maker-bond` discarded (no NIP-33 ever emitted), if the bond invoice expires without payment.

### Failure modes

- Maker never pays → invoice expires, order discarded silently. No NIP-33 event was ever emitted so the order book is unaffected.
- Maker pays the bond, order publishes as `pending`, then maker cancels → bond HTLC released, funds return to maker.
- A waiting-state timeout slash (when `slash_on_waiting_timeout = true`) settles the maker's bond HTLC instead of releasing it. The maker first receives a [`bond-slashed`](./bond_slashed.md) notification, then the order is canceled. The winning counterparty (the taker) then receives [`add-bond-invoice`](./add_bond_invoice.md) for their share.

---

## Backwards compatibility

Clients running an older `mostro-core` version that does not yet know `pay-bond-invoice` will fail to deserialize the message and silently drop it; from the user's perspective the operation stalls and eventually times out without surfacing a useful error. Operators are responsible for not enabling bonds in production until clients in the wild have adopted the `mostro-core` release that ships `Action::PayBondInvoice`. Clients should:

- Recognise the action explicitly and surface it to the user.
- If unable to handle it (e.g. an older build talking to a bond-enabled node), present a clear error rather than silently retrying.

The Mostro info event carries bond-related tags so clients can detect bond-enabled nodes ahead of a take or create — see [Other events published by Mostro](./other_events.md#anti-abuse-bond-policy-tags).
