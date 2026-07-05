# Taking a buy range order

If the order fiat amount is a range like `10-20` the seller must indicate a fiat amount to take the order, seller will send a message to Mostro (wrapped in the [active transport](./overview.md#transports)) with the following decrypted content:

```json
[
  {
    "order": {
      "version": 2,
      "id": "<Order Id>",
      "action": "take-buy",
      "trade_index": 1,
      "payload": {
        "amount": 15
      }
    }
  },
  null,
  null
]
```

## Mostro response

Response is the same as we explained in the [Taking a buy order](./take_buy.md) section with the seller receiving a hold invoice to pay and the buyer waiting for that payment.
