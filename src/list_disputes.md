# Listing Disputes

Mostro publishes new disputes with event kind `38386` and status `initiated`:

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
      ["initiator", "buyer"],
      ["published_at", "1703016565"],
      ["y", "mostro", "[Mostro instance name]"],
      ["z", "dispute"]
    ],
    "content": "",
    "sig": "<Mostro's signature>"
  }
]
```

## Tags

- `d` < Dispute ID >: A unique identifier for the dispute.
- `s` < Status >: The dispute status, see [Dispute](./dispute.md).
- `initiator` < Initiator >: Who opened the dispute, `buyer` or `seller`.
- `published_at` < Published At >: The unix timestamp when the dispute was opened, named as in [NIP-23](https://github.com/nostr-protocol/nips/blob/master/23.md). Unlike the event's `created_at`, which changes on every update of this addressable event, it stays the same across updates, so clients can show the dispute's age and sort by it.

  Mostro nodes before this rename publish the same value in a `created_at` tag. Clients SHOULD read `published_at`, then fall back to a `created_at` tag, then to the event's `created_at`.
- `y` < Platform >: `mostro`, optionally followed by the Mostro instance name.
- `z` < Document >: `dispute`.

Clients can query these events by nostr event kind `38386`, nostr event author, dispute status (`s`), type (`z`)
