# Release a range order

In most of the cases after release a range order a child order needs to be created, the client is rotating keys favoring privacy so Mostro can't know which would be the next `trade pubkey` of the maker, also the client needs to know the new id and amounts of the child order, for this case Mostro send a message with the new order, the message is the same we saw on [new range sell](./new_sell_range_order.md), it looks like this:

```json
{
  "order": {
    "version": 1,
    "id": "4fd93fc9-e909-4fc9-acef-9976122b5dfa",
    "action": "new-order",
    "request_id": "123456",
    "payload": {
      "order": {
        "id": "4fd93fc9-e909-4fc9-acef-9976122b5dfa",
        "kind": "sell",
        "status": "pending",
        "amount": 0,
        "fiat_code": "VES",
        "min_amount": 10,
        "max_amount": 20,
        "fiat_amount": 0,
        "payment_method": "face to face",
        "premium": 1,
        "created_at": 1698870173
      }
    }
  }
}
```

The client will be waiting for a message with the same request_id that was sent on the release message, then the client will know this new order is related to the previous one.

## TradePubkey action

Now that the client have updated the child order, it's time to let know to Mostro for the next `trade_pubkey`and `trade_index` of that child order, the message looks like this:

```json
{
  "order": {
    "version": 1,
    "id": "4fd93fc9-e909-4fc9-acef-9976122b5dfa",
    "trade_index": 2,
    "action": "trade-pubkey",
    "payload": null
  }
}
```

As we explained in [keys management section](./key_management.md), the client will wrap the `trade_pubkey` in the `pubkey` field inside the rumor.
