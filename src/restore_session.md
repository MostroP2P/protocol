# Restore Session

To restore a session from the mnemonic seed on a new device (e.g., moving from mobile to desktop), the client sends a `restore-session` message. Mostro will respond with the relevant orders and disputes so the client can rebuild the session state using the same `trade_index` values.

## Request

Client sends a NIP-44 direct message (kind `14`) to Mostro with the following decrypted content:

```json
[
  {
    "restore": {
      "version": 2,
      "action": "restore-session",
      "payload": null
    }
  },
  null,
  null
]
```

## Response

Mostro will respond with a message containing all non-finalized orders (e.g., statuses such as `pending`, `active`, `fiat-sent`, `waiting-buyer-invoice`, `waiting-payment`, `settled-hold-invoice`) and any active disputes. The response format will be:

```json
[
  {
    "restore": {
      "version": 2,
      "action": "restore-session",
      "payload": {
        "restore_data": {
          "orders": [
            {
              "order_id": "<Order Id>",
              "trade_index": 1,
              "status": "pending",
              "counterparty_trade_pubkey": null
            },
            {
              "order_id": "<Order Id>",
              "trade_index": 2,
              "status": "active",
              "counterparty_trade_pubkey": "<Peer Trade Pubkey>"
            },
            {
              "order_id": "<Order Id>",
              "trade_index": 3,
              "status": "fiat-sent",
              "counterparty_trade_pubkey": "<Peer Trade Pubkey>"
            }
          ],
          "disputes": [
            {
              "dispute_id": "<Dispute Id>",
              "order_id": "<Order Id>",
              "trade_index": 4,
              "status": "initiated",
              "initiator": "seller"
            }
          ]
        }
      }
    }
  },
  null,
  null
]
```

### Fields

* `restore_data`: Wrapper object that contains the session recovery data.
* `restore_data.orders`: An array of active or ongoing orders with their `order_id`, `trade_index`, current `status`, and `counterparty_trade_pubkey`.
* `restore_data.orders[].counterparty_trade_pubkey`: The **other** party's trade pubkey on that order, or `null` while nobody has taken it. See [Rebuilding the chat](#rebuilding-the-chat).
* `restore_data.disputes`: An array of ongoing disputes with `dispute_id`, the associated `order_id`, `trade_index`, current `status`, and `initiator` (`"buyer"`, `"seller"`, or `null` if unknown).

## Rebuilding the chat

The [peer-to-peer chat](./chat.md) never reaches Mostro: both the conversation key and the signing key of a conversation are derived from the ECDH secret shared by the two **trade keys** of that order, so the daemon holds none of it and cannot replay a single message.

A client restoring onto an empty database can still rebuild the conversation, because the events themselves are on the relays. It needs both halves of that derivation:

* **Its own trade key** — re-derived from the mnemonic and the `trade_index` this response carries.
* **The peer's trade pubkey** — `counterparty_trade_pubkey`.

With the pair it derives `K_conv` and `K_sign` exactly as in a live trade, subscribes to kind `14` events authored by `pub(K_sign)`, and decrypts the history the relays still hold. Without it the conversation is unreachable: the author to filter on cannot be computed, so there is nothing to ask the relays for.

A restored subscription is a chat subscription like any other, and the [client security requirements](./chat.md#client-security-requirements) apply to it unchanged — every event goes through the same validation order before it is accepted. Restore is only the one case where there is no persisted `since` cursor yet, since the point is to pull the backlog the client no longer has:

* The subscription MUST still carry a `limit`. Unbounded is what turns a flood into permanent damage.
* The client persists the cursor from the accepted messages as it normally would, clamped to `min(accepted_timestamp, local_now)`. From the next reconnect on, the subscription is bounded by that cursor again.
* Dedup state on the inner event id MUST be durable, as for a live conversation.

Rebuilding is therefore a one-off larger read, not a licence to subscribe unbounded from then on.

Mostro knows both trade pubkeys of every order it matched, so it returns the one the requesting client does not hold. It is `null` on an order nobody has taken — there is no counterparty yet, and no conversation to rebuild.

How far back the history goes is a property of the relays, not of the protocol: whatever they have pruned is gone for both parties alike.

## Example Use Case

A user has the following:

* Two `pending` orders (trade index 1 and 2)
* One `active` order (trade index 3)
* One active dispute (trade index 4)

When switching to desktop, after restoring the mnemonic, the client sends `restore-session` and receives:

```json
[
  {
    "restore": {
      "version": 2,
      "action": "restore-session",
      "payload": {
        "restore_data": {
          "orders": [
            { "order_id": "abc-123", "trade_index": 1, "status": "pending", "counterparty_trade_pubkey": null },
            { "order_id": "def-456", "trade_index": 2, "status": "pending", "counterparty_trade_pubkey": null },
            { "order_id": "ghi-789", "trade_index": 3, "status": "active", "counterparty_trade_pubkey": "e1b2...c3d4" },
            { "order_id": "xyz-999", "trade_index": 4, "status": "dispute", "counterparty_trade_pubkey": "9a8b...7c6d" }
          ],
          "disputes": [
            { "dispute_id": "dis-001", "order_id": "xyz-999", "trade_index": 4, "status": "initiated", "initiator": "seller" }
          ]
        }
      }
    }
  },
  null,
  null
]
```
