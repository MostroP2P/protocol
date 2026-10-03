# Fiat sent

After the buyer sends the fiat money to the seller, the buyer should send a message in a NIP-44 direct message (kind `14`) to Mostro indicating that the fiat money was sent, message in the first element of the decrypted content would look like this:

```json
{
  "order": {
    "version": 2,
    "id": "<Order Id>",
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
    "version": 2,
    "id": "<Order Id>",
    "action": "fiat-sent",
    "payload": {
      "next_trade": ["<trade pubkey>", <trade index>]
    }
  }
}
```

## Payer declaration

On a node that advertises `payer_history_enabled = "true"`, the buyer may declare, before `fiat-sent`, the account it pays from, and Mostro pushes a `payment-history` message to the seller right after `fiat-sent-ok`. When the node also advertises `payer_declaration_required = "true"`, `fiat-sent` without a prior declaration is refused with the `cant-do` reason `payer_not_declared`. See [Payer declaration and payment history](./payer_declaration.md).

## Mostro response

Mostro send messages to both parties confirming `fiat-sent` action and sending again the counterpart pubkey, here an example of the message to the buyer:

```json
{
  "order": {
    "version": 2,
    "id": "<Order Id>",
    "action": "fiat-sent-ok",
    "payload": {
      "peer": {
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
    "version": 2,
    "id": "<Order Id>",
    "pubkey": "<Seller's trade pubkey>",
    "action": "fiat-sent-ok",
    "payload": {
      "peer": {
        "pubkey": "<Buyer's trade pubkey>"
      }
    }
  }
}
```
