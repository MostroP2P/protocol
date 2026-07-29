# Taking a sell order with a lightning address

The buyer can use a [lightning address](https://github.com/andrerfneves/lightning-address) to receive funds and avoid to manually create and send lightning invoices on each trade, to acomplish this the buyer will send a message in a NIP-44 direct message (kind `14`) to Mostro with the following decrypted content:

```json
[
  {
    "order": {
      "version": 2,
      "id": "<Order Id>",
      "action": "take-sell",
      "trade_index": 1,
      "payload": {
        "payment_request": [null, "mostro_p2p@ln.tips", null]
      }
    }
  },
  "<index N signature of the sha256 hash of the serialized first element of content>",
  ["<index 0 pubkey (identity key)>", "<index 0 identity proof signature>"]
]
```

The event to send to Mostro would look like this:

```json
{
  "id": "<Event id>",
  "kind": 14,
  "pubkey": "<Buyer's trade pubkey>",
  "content": "<NIP-44 ciphertext of the content array>",
  "tags": [
    ["p", "<Mostro's pubkey>"],
    ["expiration", "<unix timestamp>"]
  ],
  "created_at": 1234567890,
  "sig": "<Buyer's trade key signature>"
}
```

## Mostro response

Mostro send a NIP-44 direct message (kind `14`) to the buyer with a wrapped `order` in the decrypted content, it would look like this:

```json
[
  {
    "order": {
      "version": 2,
      "id": "<Order Id>",
      "action": "waiting-seller-to-pay",
      "payload": null
    }
  },
  null,
  null
]
```

Mostro updates the addressable event with `d` tag `<Order Id>` to change the status to `in-progress`:

```json
[
  "EVENT",
  "RAND",
  {
    "id": "<Event id>",
    "pubkey": "<Mostro's pubkey>",
    "created_at": 1702549437,
    "kind": 38383,
    "tags": [
      ["d", "<Order Id>"],
      ["k", "sell"],
      ["f", "VES"],
      ["s", "in-progress"],
      ["amt", "7851"],
      ["fa", "100"],
      ["pm", "face to face"],
      ["premium", "1"],
      ["network", "mainnet"],
      ["layer", "lightning"],
      ["expiration", "1719391096"],
      ["y", "mostro", "[Mostro instance name]"],
      ["z", "order"]
    ],
    "content": "",
    "sig": "<Mostro's signature>"
  }
]
```
