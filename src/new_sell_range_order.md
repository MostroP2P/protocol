# Creating a new sell range order

To create a new range order the user should send a Gift wrap Nostr event to Mostro with the following message:

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
        "min_amount": 10,
        "max_amount": 20,
        "fiat_amount": 0,
        "payment_method": "face to face",
        "premium": 1,
        "created_at": 0
      }
    }
  }
}
```

Here we have two new fields, `min_amount` and `max_amount`, to define the range of the order. The `fiat_amount` field is set to 0 to indicate that the order is for a range of amounts.

When a taker takes the order, the amount will be set on the message.

## Optional: anti-abuse maker bond

When the Mostro node has bonds enabled and `apply_to` is `"make"` or `"both"`, the maker of a range order must lock a bond **before** the order is published — same as non-range orders, with one key difference: **the bond is sized against `max_amount`**, not the minimum.

Mostro responds to the `new-order` with a [`pay-bond-invoice`](./pay_bond_invoice.md#maker-bond) instead of the confirmation below. The order is not published to Nostr until the bond HTLC is accepted.

When a taker takes a slice of the range order, the bond obligation is reduced proportionally (the slice's fiat amount relative to `max_amount`). The bond hold invoice remains `Locked` for the lifetime of the range order and is settled once in full when the range closes; partial slashes are tracked as child bond rows rather than early partial settlements. See [Pay bond invoice — Maker bond](./pay_bond_invoice.md#maker-bond) for details.

## Confirmation message

Mostro will send back a nip59 event as a confirmation message to the user like the following:

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
          "min_amount": 10,
          "max_amount": 20,
          "fiat_amount": 0,
          "payment_method": "face to face",
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
      ["fa", "10", "20"],
      ["pm", "face to face"],
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
