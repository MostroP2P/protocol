# Fiat sent

After the buyer sends the fiat money to the seller, the buyer should send a message in a Gift wrap Nostr event to Mostro indicating that the fiat money was sent, message in the first element of the rumor's content would look like this:

```json
{
  "order": {
    "version": 1,
    "id": "ede61c96-4c13-4519-bf3a-dcf7f1e9d842",
    "action": "fiat-sent",
    "payload": null
  }
}
```

## When the maker is the buyer on a range order

In most of the cases after complete a range order, a child order needs to be created, the client is rotating keys favoring privacy so Mostro can't know which would be the next `trade pubkey` of the maker, to solve this the client needs to send `trade pubkey` and `trade index` of the child order on the `fiat-sent` message, the message looks like this:

```json
{
  "order": {
    "version": 1,
    "id": "ede61c96-4c13-4519-bf3a-dcf7f1e9d842",
    "action": "fiat-sent",
    "payload": {
      "next_trade": ["<trade pubkey>", <trade index>]
    }
  }
}
```

## Mostro response

Mostro send messages to both parties confirming `fiat-sent` action and sending again the counterpart pubkey, here an example of the message to the buyer:

```json
{
  "order": {
    "version": 1,
    "id": "ede61c96-4c13-4519-bf3a-dcf7f1e9d842",
    "action": "fiat-sent-ok",
    "payload": {
      "Peer": {
        "pubkey": "<Seller's trade pubkey>"
      }
    }
  }
}
```

And here an example of the message from Mostro to the seller:

```json
{
  "order": {
    "version": 1,
    "id": "ede61c96-4c13-4519-bf3a-dcf7f1e9d842",
    "pubkey": "<Seller's trade pubkey>",
    "action": "fiat-sent-ok",
    "payload": {
      "Peer": {
        "pubkey": "<Buyer's trade pubkey>"
      }
    }
  }
}
```

Mostro updates the addressable event with `d` tag `ede61c96-4c13-4519-bf3a-dcf7f1e9d842` to change the status to `fiat-sent`:

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
      ["d", "ede61c96-4c13-4519-bf3a-dcf7f1e9d842"],
      ["k", "sell"],
      ["f", "VES"],
      ["s", "fiat-sent"],
      ["amt", "7851"],
      ["fa", "100"],
      ["pm", "face to face"],
      ["premium", "1"],
      ["network", "mainnet"],
      ["layer", "lightning"],
      ["expiration", "1719391096"],
      ["y", "mostrop2p"],
      ["z", "order"]
    ],
    "content": "",
    "sig": "<Mostro's signature>"
  }
]
```
