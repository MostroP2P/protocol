# Payment Failed

After the seller releases the sats, Mostro attempts to pay the buyer's Lightning invoice. If this payment fails, Mostro sends a `payment-failed` action to the buyer.

> ⚠️ **Important:** `payment-failed` is an **Action** (notification), NOT an order **Status**. The order remains in `settled-hold-invoice` status while Mostro retries the payment.

## When Payment Fails

When Mostro cannot pay the buyer's Lightning invoice, it sends this message to the buyer:

```json
[
  {
    "order": {
      "version": 1,
      "id": "<Order Id>",
      "action": "payment-failed",
      "payload": {
        "payment_failed": {
          "payment_attempts": 3,
          "payment_retries_interval": 5
        }
      }
    }
  },
  null
]
```

### Payload Fields

| Field | Type | Description |
|-------|------|-------------|
| `payment_attempts` | integer | Number of retry attempts remaining |
| `payment_retries_interval` | integer | Minutes between retry attempts |

## What Happens Next

1. **Automatic retries**: Mostro will automatically retry the payment according to `payment_attempts` and `payment_retries_interval`

2. **Order status unchanged**: The order remains in `settled-hold-invoice` status during retries

3. **All retries failed**: If all payment attempts fail, Mostro sends `add-invoice` action requesting the buyer to provide a new Lightning invoice:

```json
[
  {
    "order": {
      "version": 1,
      "id": "<Order Id>",
      "action": "add-invoice",
      "payload": {
        "order": {
          "id": "<Order Id>",
          "amount": 50000,
          ...
        }
      }
    }
  },
  null
]
```

4. **Buyer provides new invoice**: The buyer must submit a new Lightning invoice for the sats to be delivered

## Client Implementation Notes

- **Do NOT create a "PaymentFailed" order status** — this action is a notification only
- Display a message to the buyer explaining the payment failed and retries are in progress
- When `add-invoice` is received after failed retries, prompt the buyer to provide a new invoice
- The sats remain safely locked in escrow (`settled-hold-invoice`) throughout this process

## Flow Diagram

```text
Seller releases sats
        │
        ▼
Order status: settled-hold-invoice
        │
        ▼
Mostro attempts Lightning payment
        │
        ├─── Success ──────────────► Order status: success
        │                            (buyer receives purchase-completed)
        │
        └─── Failure
              │
              ▼
        Action: payment-failed (to buyer)
        Order status: still settled-hold-invoice
              │
              ▼
        Mostro retries (up to payment_attempts times)
              │
              ├─── Retry succeeds ──► Order status: success
              │
              └─── All retries fail
                    │
                    ▼
              Action: add-invoice (to buyer)
              Buyer provides new invoice
                    │
                    ▼
              Mostro pays new invoice ──► Order status: success
```

## Related Actions

- [release](./release.md) - Seller releases sats
- [add-invoice](./take_sell.md) - Buyer provides Lightning invoice
