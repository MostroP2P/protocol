# User info (own reputation)

Defines the `user-info` action, used by a client to read the reputation Mostro holds for **its own** identity key: the same aggregate that Mostro shows other users about it, but without a pending order in the book and without publishing anything.

Today a client can only see its own reputation in the `rating` tag of its own pending order events ([P2P Order event](./order_event.md)), so a user with no pending order cannot see it at all. Mostro sends the reputation of the **counterpart** in the `peer` payload during a trade, but never the user's own.

## Request

The client sends a NIP-44 direct message (kind `14`) to Mostro, signed with a trade key and carrying the **identity proof** (see [Transport migration](./transport_migration.md) and [Keys management → Identity proof](./key_management.md#identity-proof)), with the following decrypted content:

```json
[
  {
    "restore": {
      "version": 2,
      "request_id": 123456,
      "action": "user-info",
      "payload": null
    }
  },
  "<trade signature>",
  ["<identity pubkey>", "<identity sig>"]
]
```

The identity proof is **required**: the reputation belongs to the identity key, never to a trade key. A request without it has no reputation to read; see [Errors](#errors).

`user-info` belongs to no order, so when the sending trade key has no active order or dispute on the node, the request is a **first contact**: mine it at the `pow_first_contact` difficulty, not just `pow`, or the node drops it without any reply (see [Proof of work and the first-contact gate](./transport_migration.md#proof-of-work-and-the-first-contact-gate)).

`request_id` is optional; when present, Mostro echoes it in the response so the client can match it.

## Response

Mostro looks up the user by the identity key from the proof and answers with a direct message to the **trade key** that sent the request. The payload is a `user_info` object with the same fields Mostro sends about a counterpart in the `peer` payload:

```json
{
  "restore": {
    "version": 2,
    "request_id": 123456,
    "action": "user-info",
    "payload": {
      "user_info": {
        "rating": 4.8,
        "reviews": 23,
        "operating_days": 142,
        "since": 1700784000
      }
    }
  }
}
```

A user Mostro has no record of yet (an identity that never traded on this node) gets zeros and no `since`, not an error: "no reputation yet" is a valid answer.

```json
{
  "restore": {
    "version": 2,
    "action": "user-info",
    "payload": {
      "user_info": {
        "rating": 0.0,
        "reviews": 0,
        "operating_days": 0
      }
    }
  }
}
```

### Fields

* `restore.action`: Must be `user-info`.
* `restore.payload.user_info.rating`: The user's aggregated rating, the same value Mostro publishes as `total_rating` in the order event's `rating` tag. `0` when there are no reviews.
* `restore.payload.user_info.reviews`: Total number of ratings the user has received. Clients SHOULD treat `0` as "no reputation yet" and not show `rating`.
* `restore.payload.user_info.since`: Unix timestamp of the user's first trade, **truncated to the start of its UTC day**, exactly as in the [rating event](./user_rating.md). Absent when the user has no trades. Clients compute the age at display time.
* `restore.payload.user_info.operating_days`: **DEPRECATED**, as `days` in the rating event: the age in days, computed when the message was built. Clients MUST read `since` when present.

## Errors

Mostro answers with [`cant-do`](./message_suggestions_for_actions.md#cant-do-reasons) instead of `user-info`:

* `reputation_identity_required`: the request carries no identity proof. A client in full privacy mode never has one to send, and a full-privacy user has no reputation by design; clients SHOULD not send `user-info` in that mode, and SHOULD explain that reputation needs reputation mode instead of showing zeros.
* `invalid_signature`: the identity proof does not verify.

## Notes

* **Scope.** The answer is the reputation on **this** Mostro node: each node keeps its own users. A client connected to several nodes asks each one.
* **Privacy.** Nothing is published: the answer travels only as a direct message to the requesting trade key. Mostro MUST NOT answer for any identity other than the one proven in the request, so a client cannot read another user's aggregate through this action (it already sees a counterpart's through `peer`, and a maker's through the order event).
* **Freshness.** The value is read at request time. A client that shows it SHOULD refresh it when the user opens the screen that displays it, and after a trade it took part in reaches `success` (a rating may follow), rather than polling.
* **Cost.** Like `last-trade-index`, the action is read-only and touches no order, escrow or Lightning state, so Mostro MAY serve it in every escrow mode.

## Example

Alice, in reputation mode, opens her account screen. Her client sends `user-info` with her identity proof. Mostro answers 23 reviews averaging `4.8`, first trade on 2023-11-24 (`since: 1700784000`), and her client shows "★ 4.8 · 23 reviews · since Nov 2023". The count is of ratings received, not of trades: a trade nobody rated adds nothing to it.

Bob uses full privacy mode. His client does not send `user-info`; it tells him that reputation is only kept in reputation mode.
