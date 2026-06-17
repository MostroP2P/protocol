# Keys management

_It is required to read [NIP-59 (gift wrap)](https://github.com/nostr-protocol/nips/blob/master/59.md) and [NIP-06](https://github.com/nostr-protocol/nips/blob/master/06.md) to fully understand this document_

Mostro clients should implement nip59 which creates newly fresh keys on each message to Mostro, but the key management is a bit more complex, here we will explain how to manage keys in Mostro clients.

### Objectives:

- Facilitate portability by using a deterministic key generation mechanism based on NIP-06.
- Prevent users from mistakenly entering key material already used in other Nostr social media apps.
- Rotate keys for every trade.

When a user started a Mostro client for first time, the client should create a new mnemonic seed phrase which is the only information users will need to share with other client to have the same `Mostro session`. Mostro clients should use the derivation path `m/44'/1237'/38383'/0/0`.

Clients will always use the first key (zero) `m/44'/1237'/38383'/0/0` to identify itself with mostrod, users who wants to maintain reputation can send an event to Mostro signed with the `zero` key, the identity key, used to update their rating, for every new order created or taken the client will start deriving new keys from `m/44'/1237'/38383'/0/1`, users who don't want to maintain reputation simply don't send the identity key to mostrod, let's see it in more detail with an example:

- Alice starts a Mostro client for first time, at that moment the client creates a new mnemonic seed phrase and derive two keys, the identity key (index `0`) and the next trade key with index `1` `m/44'/1237'/38383'/0/1`, we will use identiy key to sign the gift wrap seal event and the trade key to sign the first element of the content of the rumor event.

- Alice wants to buy some bitcoin and take a sell order, the client send a message in a Gift wrap Nostr event to mostrod with the seal signed with index `0` key and in the rumor we should demostrate we own the trade key (index `1`), let's see a `take-sell` example in an unencrypted gift wrap event:

```json
// external wrap layer
{
  "id": "<id>",
  "kind": 1059,
  "pubkey": "<Buyer's ephemeral pubkey>",
  "content": {
    // seal
    "id": "<seal's id>",
    "pubkey": "<index 0 pubkey (identity key)>",
    "content": {
      // rumor
      "id": "<rumor's id>",
      "pubkey": "<Index 1 pubkey (trade key)>",
      "kind": 1,
      "content": [
        {
          "order": {
            "version": 1,
            "id": "<Order Id>",
            "trade_index": 1,
            "action": "take-sell",
            "payload": null
          }
        },
        "<index 1 signature of the sha256 hash of the serialized first element of content>"
      ],
      "created_at": 1691518405,
      "tags": []
    },
    "kind": 13,
    "created_at": 1686840217,
    "tags": [],
    "sig": "<index 0 pubkey (identity key) signature>"
  },
  "tags": [["p", "<Mostro's pubkey>"]],
  "created_at": 1234567890,
  "sig": "<Buyer's ephemeral pubkey signature>"
}
```

- After finish the deal the [rate](./user_rating.md) each other.

Then Alice wants to create a new buy order:

- The client derives the next key, new key is index `2` (`m/44'/1237'/38383'/0/2`) and send a message in a Gift wrap Nostr event to mostrod with the seal signed with index `0` key, but let's see the complete example with a full unencrypted gift wrap:

```json
// external wrap layer
{
  "id": "<id>",
  "kind": 1059,
  "pubkey": "<Buyer's ephemeral pubkey>",
  "content": {
    // seal
    "id": "<seal's id>",
    "pubkey": "<index 0 pubkey (identity key)>",
    "content": {
      // rumor
      "id": "<rumor's id>",
      "pubkey": "<Index 2 pubkey (trade key)>",
      "kind": 1,
      "content": [
        {
          "order": {
            "version": 1,
            "trade_index": 2,
            "action": "new-order",
            "payload": {
              "order": {
                "kind": "buy",
                "status": "pending",
                "amount": 0,
                "fiat_code": "VES",
                "fiat_amount": 100,
                "payment_method": "face to face",
                "premium": 1,
                "created_at": 1691518405
              }
            }
          }
        },
        "<index 2 signature of the sha256 hash of the serialized first element of content>"
      ],
      "created_at": 1691518405,
      "tags": []
    },
    "kind": 13,
    "created_at": 1686840217,
    "tags": [],
    "sig": "<index 0 pubkey (identity key) signature>"
  },
  "tags": [["p", "<Mostro's pubkey>"]],
  "created_at": 1234567890,
  "sig": "<Buyer's ephemeral pubkey signature>"
}
```

Now Alice waits for some seller to take her order, mostrod will show Alice's reputation but not Alice pubkey.

### Full privacy mode

Clients must offer a more private version where the client never send the identity key to mostrod, in that case mostrod can't link orders to users, the tradeoff is that users who choose this option cannot have a reputation, let's see a `take-sell` example in an unencrypted gift wrap event:

```json
// external wrap layer
{
  "id": "<id>",
  "kind": 1059,
  "pubkey": "<Buyer's ephemeral pubkey>",
  "content": {
    // seal
    "id": "<seal's id>",
    "pubkey": "<index N pubkey (trade key)>",
    "content": {
      // rumor
      "id": "<rumor's id>",
      "pubkey": "<index N pubkey (trade key)>",
      "kind": 1,
      "content": [
        {
          "order": {
            "version": 1,
            "id": "<Order Id>",
            // "trade_index": 1, // not needed
            "action": "take-sell",
            "payload": null
          }
        },
        null // Signature is not needed in full privacy mode
      ],
      "created_at": 1691518405,
      "tags": []
    },
    "kind": 13,
    "created_at": 1686840217,
    "tags": [],
    "sig": "<index N pubkey (trade key) signature>"
  },
  "tags": [["p", "<Mostro's pubkey>"]],
  "created_at": 1234567890,
  "sig": "<Buyer's ephemeral pubkey signature>"
}
```

## Protocol v2 — NIP-44 direct messages

Everything above describes **protocol v1** (NIP-59 gift wrap, kind `1059`),
which is **DEPRECATED**. Nodes that advertise `protocol_version = "2"` in
their [instance-info event](./other_events.md#mostro-instance-status) speak
**protocol v2** instead: a single signed [NIP-44](https://github.com/nostr-protocol/nips/blob/master/44.md)
direct message of kind `14`, with no gift-wrap or seal layer. The key
derivation, indexing and rotation rules are **unchanged** — the same
identity key (index `0`) and per-trade keys (index `1`, `2`, …); only the
envelope and how the identity key is proven differ.

What changes on the wire:

- **The trade key authors the event.** The visible `pubkey`/`sig` are the
  trade key's (not a throwaway ephemeral key as in v1). This is deliberate:
  it lets relays rate-limit by sender and lets the node cheaply pre-filter
  spam before decrypting. The exposure is bounded because trade keys are
  single-trade and rotated.
- **`content` is NIP-44 ciphertext** of the message array. The conversation
  key is derived from the trade key ↔ Mostro pair, so only those two can
  decrypt.
- **The array gains a third element**, the identity proof — because there is
  no seal to carry the identity key authenticated. See below.
- **An `expiration` tag** ([NIP-40](https://github.com/nostr-protocol/nips/blob/master/40.md))
  is always present so trade messages do not linger on relays forever
  (default 30 days, the node's `dm_days` setting).
- **`version` is `2`** in the message.

> **Note (deliberate NIP-17 deviation):** [NIP-17](https://github.com/nostr-protocol/nips/blob/master/17.md)
> defines kind `14` as an *unsigned* rumor that only ever travels inside a
> gift wrap. Mostro publishes it *signed*, because the author is an
> ephemeral single-trade key — the association NIP-17 protects against is
> here intentional and bounded. These are not standard NIP-17 chat messages.

### Reputation mode (`take-sell` example)

Reusing Alice's `take-sell` from above (trade key index `1`, identity key
index `0`), the v2 event looks like this:

```json
{
  "id": "<id>",
  "kind": 14,
  "pubkey": "<index 1 pubkey (trade key)>",
  "content": "<NIP-44 ciphertext of the array below>",
  "tags": [
    ["p", "<Mostro's pubkey>"],
    ["expiration", "<unix timestamp>"]
  ],
  "created_at": 1691518405,
  "sig": "<index 1 (trade key) signature>"
}
```

The decrypted `content` is the message array — the same first two elements
as v1, plus the identity proof as a third element:

```json
[
  {
    "order": {
      "version": 2,
      "id": "<Order Id>",
      "trade_index": 1,
      "action": "take-sell",
      "payload": null
    }
  },
  "<index 1 signature of the sha256 hash of the serialized first element>",
  ["<index 0 pubkey (identity key)>", "<index 0 identity proof signature>"]
]
```

### Identity proof

The identity signature (third element) is **not** taken over the message
alone. The identity key signs a domain-tagged payload that binds the proof
to the *specific trade key* authoring the event:

```text
mostro-transport-v2-identity:<trade pubkey hex>:<message JSON>
```

where `<message JSON>` is the serialized first element and `<trade pubkey
hex>` is this event's `pubkey`. The receiver recomputes the payload from
`event.pubkey`, so a proof grafted onto an event authored by a different
trade key fails verification. This binding is what the gift-wrap seal
signature provided implicitly in v1. The signing scheme is the existing
Schnorr-over-sha256 `Message::sign` / `verify_signature`, and the identity
key signs once per message — the same custody model as v1, where it signs
every seal.

### Full privacy mode

As in v1, a client that does not want to maintain reputation never sends
the identity key. In v2 that means **both** the trade signature and the
identity proof are `null`, and the identity is taken to be the trade key
itself:

```json
{
  "id": "<id>",
  "kind": 14,
  "pubkey": "<index N pubkey (trade key)>",
  "content": "<NIP-44 ciphertext of the array below>",
  "tags": [
    ["p", "<Mostro's pubkey>"],
    ["expiration", "<unix timestamp>"]
  ],
  "created_at": 1691518405,
  "sig": "<index N (trade key) signature>"
}
```

Decrypted `content`:

```json
[
  {
    "order": {
      "version": 2,
      "id": "<Order Id>",
      "action": "take-sell",
      "payload": null
    }
  },
  null,
  null
]
```

### Messages from Mostro

Mostro authors its replies with its own well-known key and NIP-44 encrypts
them to the user's trade key (`p`-tagged). As in v1, Mostro's own messages
are unsigned: the trade signature and identity proof are both `null`.
Clients can subscribe with `authors=[mostro] AND #p=[their trade keys]`.
