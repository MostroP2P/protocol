# Creating a new sell order

To create a new sell order the user should send a Gift wrap Nostr event to Mostro, the message should look like this:

```json
{
  "order": {
    "version": 1,
    "action": "new-order",
    "trade_index": 1,
    "payload": {
      "order": {
        "kind": "sell",
        "status": "pending",
        "amount": 0,
        "fiat_code": "VES",
        "min_amount": null,
        "max_amount": null,
        "fiat_amount": 100,
        "payment_method": "face to face,bank transfer,mobile",
        "premium": 1,
        "created_at": 0
      }
    }
  }
}
```

Let's explain some of the fields:

- kind: `sell` or `buy`
- status: Is always `pending` when creating a new order
- amount: `0` for when we want to sell with at market price, otherwise the amount in satoshis
- created_at: No need to send the correct unix timestamp, Mostro will replace it with the current time

The event to send to Mostro would look like this:

```json
{
  "id": "<Event id>",
  "kind": 1059,
  "pubkey": "<Seller's ephemeral pubkey>",
  "content": "<sealed-rumor-content>",
  "tags": [["p", "Mostro's pubkey"]],
  "created_at": 1234567890,
  "sig": "<Signature of ephemeral pubkey>"
}
```

## Optional: anti-abuse maker bond

When the Mostro node has bonds enabled and `apply_to` is `"make"` or `"both"`, the maker must lock a bond **before** the order is published. Instead of a `new-order` confirmation, Mostro first responds with a [`pay-bond-invoice`](./pay_bond_invoice.md#maker-bond) message asking the maker to pay a small hold invoice (typically ~1% of the trade amount). The order is **not visible on Nostr** until the bond HTLC is accepted.

Only after the maker pays the bond does Mostro:
1. Publish the order to Nostr with status `pending`.
2. Send the `new-order` confirmation (shown below).

If the maker never pays the bond invoice it expires and no order is created. See [Pay bond invoice — Maker bond](./pay_bond_invoice.md#maker-bond) for details.

## Confirmation message

Mostro will send back a nip59 event as a confirmation message to the user like the following (unencrypted rumor's content example):

```json
[
  {
    "order": {
      "version": 1,
      "id": "<Order id>",
      "action": "new-order",
      "payload": {
        "order": {
          "id": "<Order id>",
          "kind": "sell",
          "status": "pending",
          "amount": 0,
          "fiat_code": "VES",
          "fiat_amount": 100,
          "payment_method": "face to face,bank transfer,mobile",
          "premium": 1,
          "created_at": 1698870173
        }
      }
    }
  },
  null
]
```

Mostro publishes this order as an event kind `38383` with status `pending`:

```json
[
  "EVENT",
  "RAND",
  {
    "id": "<Event id>",
    "pubkey": "<Mostro's pubkey>",
    "created_at": 1702548701,
    "kind": 38383,
    "tags": [
      ["d", "<Order id>"],
      ["k", "sell"],
      ["f", "VES"],
      ["s", "pending"],
      ["amt", "0"],
      ["fa", "100"],
      ["pm", "face to face", "bank transfer", "mobile"],
      ["premium", "1"],
      ["rating", "[\"rating\",{\"days\":10,\"total_rating\":4.5,\"total_reviews\":7}]"],
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
