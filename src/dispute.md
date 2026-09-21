# Dispute

## Dispute statuses

A dispute has its own lifecycle, separate from the order's. Its current status travels in the `s` tag of the addressable dispute event (kind `38386`), and in the `status` field of the disputes returned by [restore-session](./restore_session.md) — which only returns open ones.

| Status | Meaning | Set when |
|---|---|---|
| `initiated` | Open, waiting for a solver. | Either party sends `dispute` (below). |
| `in-progress` | A solver has taken it and is working on it. | A solver sends [`admin-take-dispute`](#taking-the-dispute). |
| `settled` | A **solver** resolved it in the buyer's favour: the seller's hold invoice was settled and the buyer is paid. | A solver sends [`admin-settle`](./admin_settle_order.md). |
| `seller-refunded` | A **solver** resolved it in the seller's favour: the hold invoice was canceled and the sats return to the seller. | A solver sends [`admin-cancel`](./admin_cancel_order.md). |
| `released` | The **users** resolved it: the seller released the funds, and the buyer is paid. | The seller sends [`release`](./release.md) while the dispute is open. |
| `cooperatively-canceled` | The **users** resolved it: both agreed to cancel, and the seller is refunded. | Both parties agree to a [cooperative cancel](./cancel.md) while the dispute is open. |

`initiated` and `in-progress` are open; the other four are final.

The final statuses come in pairs with the same outcome, told apart by **who** resolved the dispute:

| Outcome | Resolved by a solver | Resolved by the users |
|---|---|---|
| Buyer paid | `settled` | `released` |
| Seller refunded | `seller-refunded` | `cooperatively-canceled` |

So a client can read the outcome and the resolver straight from the status, without looking at the order.

**Older daemons.** Before `released` and `cooperatively-canceled` were emitted, a release during a dispute closed it as `settled`, and a cooperative cancel as `seller-refunded` — so on those daemons `settled` and `seller-refunded` do not prove a solver decided. A client that must know who resolved a dispute should treat that pair as ambiguous when talking to a daemon that predates them, and must parse all four final statuses either way.

## Opening a dispute

A user can start a dispute in an order with status `active` or `fiat-sent` sending action `dispute`, here is an example where the seller initiates a dispute:

```json
[
  {
    "order": {
      "version": 2,
      "id": "<Order Id>",
      "action": "dispute",
      "payload": null
    }
  },
  "<index N signature of the sha256 hash of the serialized first element of content>",
  ["<index 0 pubkey (identity key)>", "<index 0 identity proof signature>"]
]
```

## Mostro response

Mostro will send this message to the seller:

```json
[
  {
    "order": {
      "version": 2,
      "id": "<Order Id>",
      "action": "dispute-initiated-by-you",
      "payload": {
        "dispute": "<Dispute Id>"
      }
    }
  },
  null,
  null
]
```

And here is the message to the buyer:

```json
[
  {
    "order": {
      "version": 2,
      "id": "<Order Id>",
      "action": "dispute-initiated-by-peer",
      "payload": {
        "dispute": "<Dispute Id>"
      }
    }
  },
  null,
  null
]
```

Mostro will not update the addressable event with `d` tag `<Order Id>` to change the status to `dispute`, this is because the order is still active, the dispute is just a way to let the admins and the other party know that there is a problem with the order.

## Mostro sends an addressable event to show the dispute

Here is an example of the event sent by Mostro:

```json
[
  "EVENT",
  "RAND",
  {
    "id": "<Event id>",
    "pubkey": "<Mostro's pubkey>",
    "created_at": 1703016565,
    "kind": 38386,
    "tags": [
      ["d", "<Dispute Id>"],
      ["s", "initiated"],
      ["initiator", "seller"], // seller or buyer
      ["y", "mostro", "[Mostro instance name]"],
      ["z", "dispute"]
    ],
    "content": "",
    "sig": "<Mostro's signature>"
  }
]
```

## Taking the dispute

Mostro admin will see the dispute and can take it using the dispute `Id` from `d` tag, here how should look the message sent by the admin:

```json
[
  {
    "dispute": {
      "version": 2,
      "id": "<Dispute Id>",
      "action": "admin-take-dispute",
      "payload": null
    }
  },
  null,
  null
]
```

Mostro will send a confirmation message to the admin with the order details:

```json
[
  {
    "dispute": {
      "version": 2,
      "id": "<Dispute Id>",
      "action": "admin-took-dispute",
      "payload": {
        "order": {
          "id": "<Order Id>",
          "kind": "sell",
          "status": "active",
          "amount": 7851,
          "fiat_code": "VES",
          "fiat_amount": 100,
          "payment_method": "face to face",
          "premium": 1,
          "buyer_trade_pubkey": "<Buyer's trade pubkey>",
          "seller_trade_pubkey": "<Seller's trade pubkey>",
          "buyer_invoice": "lnbcrt11020n1pjcypj3pp58m3d9gcu4cc8l3jgkpfn7zhqv2jfw7p3t6z3tq2nmk9cjqam2c3sdqqcqzzsxqyz5vqsp5mew44wzjs0a58d9sfpkrdpyrytswna6gftlfrv8xghkc6fexu6sq9qyyssqnwfkqdxm66lxjv8z68ysaf0fmm50ztvv773jzuyf8a5tat3lnhks6468ngpv3lk5m7yr7vsg97jh6artva5qhd95vafqhxupyuawmrcqnthl9y",
          "created_at": 1698870173
        }
      }
    }
  },
  null,
  null
]
```

Then mostrod send messages to each trade participant, the buyer and seller for them to know the pubkey of the admin who took the dispute, that way the client can start listening events from that specific pubkey, by default clients should discard any messages received from any pubkey different than Mostro node or dispute solver, the message looks like this:

```json
[
  {
    "order": {
      "version": 2,
      "id": "<Order Id>",
      "action": "admin-took-dispute",
      "payload": {
        "peer": {
          "pubkey": "<Solver's pubkey>"
        }
      }
    }
  },
  null,
  null
]
```

Also Mostro will broadcast a new addressable dispute event to update the dispute `status` to `in-progress`:

```json
[
  "EVENT",
  "RAND",
  {
    "id": "<Event id>",
    "pubkey": "<Mostro's pubkey>",
    "created_at": 1703020540,
    "kind": 38386,
    "tags": [
      ["d", "<Dispute Id>"],
      ["s", "in-progress"],
      ["initiator", "seller"], // seller or buyer
      ["y", "mostro", "[Mostro instance name]"],
      ["z", "dispute"]
    ],
    "content": "",
    "sig": "<Mostro's signature>"
  }
]
```
