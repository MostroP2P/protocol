# User rating

After a successful trade Mostro send a NIP-44 direct message (kind `14`) to both parties to let them know they can rate each other, here an example how the message look like:

```json
[
  {
    "order": {
      "version": 2,
      "id": "<Order Id>",
      "action": "rate",
      "payload": null
    }
  },
  null,
  null
]
```

After a Mostro client receive this message, the user can rate the other party, the rating is a number between 1 and 5, to rate the client must receive user's input and create a new NIP-44 direct message (kind `14`) to send to Mostro with this content:

```json
[
  {
    "order": {
      "version": 2,
      "id": "<Order Id>",
      "action": "rate-user",
      "payload": {
        "rating_user": 5 // User input
      }
    }
  },
  null,
  null
]
```

## Confirmation message

If Mostro received the correct message, it will send back a confirmation message to the user with the action `rate-received`:

```json
[
  {
    "order": {
      "version": 2,
      "id": "<Order Id>",
      "action": "rate-received",
      "payload": {
        "rating_user": 5
      }
    }
  },
  null,
  null
]
```

Mostro updates the addressable rating event, in this event the `d` tag will be the user pubkey `<Seller's trade pubkey>` and looks like this:

```json
[
  "EVENT",
  "RAND",
  {
    "id": "<Event id>",
    "pubkey": "<Mostro's pubkey>",
    "created_at": 1702637077,
    "kind": 38384,
    "tags": [
      ["d", "<Seller's trade pubkey>"],
      ["total_reviews", "1"],
      ["total_rating", "2"],
      ["last_rating", "1"],
      ["max_rate", "5"],
      ["min_rate", "1"],
      ["since", "1700784000"],
      ["days", "21"],
      ["y", "mostro", "[Mostro instance name]"],
      ["z", "rating"]
    ],
    "content": "",
    "sig": "<Mostro's signature>"
  }
]
```

## Tags

- `d` < User trade pubkey >: The trade pubkey of the rated user.
- `total_reviews` < Total reviews >: The total number of reviews the user has received.
- `total_rating` < Total rating >: The overall reputation rating of the user.
- `last_rating` < Last rating >: The rating received in the most recent review.
- `max_rate` < Max rate >: The highest rating the user has received.
- `min_rate` < Min rate >: The lowest rating the user has received.
- `since` < Since >: Unix timestamp of the user's first trade, **truncated to the start of its UTC day** (`created_at - created_at % 86400`). Clients compute the age at display time (`now - since`). Day precision carries exactly the information `days` carried, without turning the tag into a per-user fingerprint: second precision, published on every event of the same user, would make their trade pubkeys trivially correlatable.
- `days` [Days]: **DEPRECATED.** The number of days since the user's first trade, computed at publish time — so it is stale on any event that lives on relays for a while. Superseded by `since`. Mostro publishes both for one deprecation window and then drops `days`; it will be removed in the minor release after the one that first publishes `since`. Clients MUST read `since` when present and MAY fall back to `days` while the window lasts.
- `y` < Platform >: Platform identifier tag values. Mostro publishes `"mostro"` and MAY include a second value with the Mostro instance name from settings.
- `z` < Document >: `rating`.
