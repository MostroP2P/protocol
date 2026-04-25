# Restore Session

To restore a session from the mnemonic seed on a new device (e.g., moving from mobile to desktop), the client sends a `restore-session` message. Mostro will respond with the relevant orders and disputes so the client can rebuild the session state using the same `trade_index` values.

## Request

Client sends a Gift wrap Nostr event to Mostro with the following rumor's content:

```json
[
  {
    "restore": {
      "version": 1,
      "action": "restore-session",
      "payload": null
    }
  },
  null
]
```

## Response

Mostro will respond with a message containing all non-finalized orders (e.g., statuses such as `pending`, `active`, `fiat-sent`, `waiting-buyer-invoice`, `waiting-payment`) and any active disputes. Orders currently under dispute are not included in `orders`; their information is reported through the `disputes` array instead. The response format will be:

```json
[
  {
    "restore": {
      "version": 1,
      "action": "restore-session",
      "payload": {
        "restore_data": {
          "orders": [
            {
              "order_id": "<Order Id>",
              "trade_index": 1,
              "status": "pending"
            },
            {
              "order_id": "<Order Id>",
              "trade_index": 2,
              "status": "active"
            },
            {
              "order_id": "<Order Id>",
              "trade_index": 3,
              "status": "fiat-sent"
            }
          ],
          "disputes": [
            {
              "dispute_id": "<Dispute Id>",
              "order_id": "<Order Id>",
              "trade_index": 4,
              "status": "initiated",
              "initiator": "seller",
              "solver_pubkey": null
            },
            {
              "dispute_id": "<Dispute Id>",
              "order_id": "<Order Id>",
              "trade_index": 5,
              "status": "in-progress",
              "initiator": "buyer",
              "solver_pubkey": "<Solver Pubkey>"
            }
          ]
        }
      }
    }
  },
  null
]
```

### Fields

* `restore_data`: Wrapper object that contains the session recovery data.
* `restore_data.orders`: An array of active or ongoing orders. Each entry contains:
  * `order_id`: unique identifier of the order.
  * `trade_index`: user's trade index for the order. Used with the mnemonic seed to derive the trade key pair subscribed to it.
  * `status`: current order state (e.g., `pending`, `active`, `fiat-sent`, `waiting-buyer-invoice`, `waiting-payment`).
* `restore_data.disputes`: An array of ongoing disputes. Each entry contains:
  * `dispute_id`: unique identifier of the dispute.
  * `order_id`: id of the order in dispute.
  * `trade_index`: trade index of that order. Used with the mnemonic seed to derive the same trade key pair already subscribed to it.
  * `status`: current state of the dispute (e.g., `initiated`, `in-progress`).
  * `initiator`: party that opened the dispute (`buyer` or `seller`). Tells the client whether the user or the counterparty started it.
  * `solver_pubkey`: hex pubkey of the assigned solver, or `null` if none yet. Set once an admin assigns one via `admin-add-solver`. Also used to derive the ECDH shared key for the dispute chat.

> **Note:** When an order is in dispute it does not appear in `orders`, it appears in `disputes`. In each dispute entry:
>
> * `trade_index` is the order's `trade_index`.
> * `order_id` is the id of that order.
> * The order's `status` is implicitly `dispute`.

## Example Use Case

A user has the following:

* Two `pending` orders (trade index 1 and 2)
* One `active` order (trade index 3)
* One dispute just opened, still waiting for a solver (trade index 4)
* One dispute already taken by a solver (trade index 5)

When switching to desktop, after restoring the mnemonic, the client sends `restore-session` and receives:

```json
[
  {
    "restore": {
      "version": 1,
      "action": "restore-session",
      "payload": {
        "restore_data": {
          "orders": [
            { "order_id": "abc-123", "trade_index": 1, "status": "pending" },
            { "order_id": "def-456", "trade_index": 2, "status": "pending" },
            { "order_id": "ghi-789", "trade_index": 3, "status": "active" }
          ],
          "disputes": [
            { "dispute_id": "dis-001", "order_id": "xyz-999", "trade_index": 4, "status": "initiated", "initiator": "seller", "solver_pubkey": null },
            { "dispute_id": "dis-002", "order_id": "uvw-555", "trade_index": 5, "status": "in-progress", "initiator": "buyer", "solver_pubkey": "<Solver Pubkey>" }
          ]
        }
      }
    }
  },
  null
]
```
