# Settle order

An admin can settle an order, most of the time this is done when admin is solving a dispute, for this the admin will need to send an `order` message to Mostro with action `admin-settle` with the `id` of the order like this:

```json
[
  {
    "order": {
      "version": 1,
      "id": "<Order Id>",
      "action": "admin-settle",
      "payload": null
    }
  },
  null
]
```

## Bond resolution payload

When solving a dispute, the admin can optionally slash one or both parties' bonds independently from the trade outcome (settle vs. cancel). To do this, the `payload` is set to a `bond_resolution` object with two booleans, `slash_seller` and `slash_buyer`:

```json
[
  {
    "order": {
      "version": 1,
      "id": "<Order Id>",
      "action": "admin-settle",
      "payload": {
        "bond_resolution": {
          "slash_seller": false,
          "slash_buyer": true
        }
      }
    }
  },
  null
]
```

Accepted combinations on `admin-settle`:

- `{ "slash_seller": false, "slash_buyer": false }` — settle without slashing any bond
- `{ "slash_seller": true,  "slash_buyer": false }` — settle and slash the seller's bond
- `{ "slash_seller": false, "slash_buyer": true }`  — settle and slash the buyer's bond
- `{ "slash_seller": true,  "slash_buyer": true }`  — settle and slash both bonds
- `payload: null` — legacy clients; interpreted server-side as "no slash"

`bond_resolution` is only valid on `admin-settle` and `admin-cancel`; it is rejected on every other action.

Whenever a side is slashed and the operator has configured `slash_node_share_pct < 1.0`, the non-slashed counterparty is subsequently asked to provide a Lightning invoice for their share of the bond via [Bond payout invoice](./add_bond_invoice.md).

Each slashed party is also sent a [`bond-slashed`](./bond_slashed.md) forfeiture notice for the slashed bond amount, in addition to the `admin-settled` confirmation below.

## Mostro response

Mostro will send this message to the both parties buyer/seller and to the admin:

```json
[
  {
    "order": {
      "version": 1,
      "id": "<Order Id>",
      "action": "admin-settled",
      "payload": null
    }
  },
  null
]
```

## Mostro updates addressable dispute event

Mostro updates the addressable dispute event with status `settled`:

```json
[
  "EVENT",
  "RAND",
  {
    "id": "098e8622eae022a79bc793984fccbc5ea3f6641bdcdffaa031c00d3bd33ca5a0",
    "pubkey": "<Mostro's pubkey>",
    "created_at": 1703274022,
    "kind": 38386,
    "tags": [
      ["d", "efc75871-2568-40b9-a6ee-c382d4d6de01"],
      ["s", "settled"],
      ["y", "mostro", "[Mostro instance name]"],
      ["z", "dispute"]
    ],
    "content": "",
    "sig": "<Mostro's signature>"
  }
]
```

## Payment of the buyer's invoice

At this point Mostro is trying to pay the buyer's invoice, right after complete the payment Mostro will update the status of the order addressable event to `success`:

```json
[
  "EVENT",
  "RAND",
  {
    "id": "6170892aca6a73906142e58a9c29734d49b399a3811f6216ce553b4a77a8a11e",
    "pubkey": "<Mostro's pubkey>",
    "created_at": 1703274032,
    "kind": 38383,
    "tags": [
      ["d", "<Order Id>"],
      ["k", "sell"],
      ["f", "VES"],
      ["s", "success"],
      ["amt", "7851"],
      ["fa", "100"],
      ["pm", "face to face"],
      ["premium", "1"],
      ["y", "mostro", "[Mostro instance name]"],
      ["z", "order"]
    ],
    "content": "",
    "sig": "<Mostro's signature>"
  }
]
```
