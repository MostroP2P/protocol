# Dispute Chat

The dispute chat uses the same shared key encryption scheme as the [Peer-to-peer Chat](./chat.md). Instead of computing a shared key between buyer and seller, each party computes an independent shared key with the admin who took the dispute.

## Establishing the shared key

When an admin takes a dispute, Mostro sends an `admin-took-dispute` message to each party (buyer and seller) containing the admin's pubkey:

```json
[
  {
    "order": {
      "version": 2,
      "id": "<Order Id>",
      "action": "admin-took-dispute",
      "payload": {
        "peer": {
          "pubkey": "<Admin's pubkey>"
        }
      }
    }
  },
  null,
  null
]
```

Upon receiving this message, the client computes the shared key using ECDH:

```
Shared Key = ECDH(tradeKey.private, adminPubkey)
```

The admin computes the same shared key from their side:

```
Shared Key = ECDH(adminPrivateKey, tradeKey.public)
```

Each party (buyer and seller) has its own independent shared key with the admin. A session can have both a peer shared key (for the P2P chat) and an admin shared key (for the dispute chat) simultaneously.

That shared secret is then split into `K_conv` and `K_sign` exactly as in [Key derivation](./chat.md#key-derivation), using the same HKDF `info` strings:

```
shared  = ECDH(tradeKey.private, adminPubkey)     // admin: ECDH(adminPrivateKey, tradeKey.public)

K_conv  = HKDF-SHA256(ikm = shared, info = "mostro:chat:conv:v1", L = 32)
K_sign  = HKDF-SHA256(ikm = shared, info = "mostro:chat:sign:v1", L = 32)
```

## Sending and receiving messages

Messages use the same envelope as the [Peer-to-peer Chat](./chat.md#event-structure): a kind 1 inner event signed by the sender's own key, NIP-44 encrypted under `K_conv`, carried in a **kind 14 event signed with `K_sign`** and `p`-tagged to `pub(K_conv)`. There is no gift wrap and no ephemeral key.

```json
{
  "id": "<Event Id>",
  "pubkey": "<pub(K_sign) of the admin shared key>",
  "kind": 14,
  "created_at": 1703021488,
  "content": "<NIP-44 encrypted inner event>",
  "tags": [["p", "<pub(K_conv) of the admin shared key>"]],
  "sig": "<K_sign signature>"
}
```

Unlike the peer chat, **the admin is a legitimate writer here** — this channel is how a solver talks to each party privately. The accepted inner signers are therefore that party's trade key and the admin's pubkey, and nothing else.

## Subscribing to messages

Clients subscribe by **author**, not by `p` tag:

```json
{
  "kinds": [14],
  "authors": ["<pub(K_sign) of the admin shared key>"]
}
```

Only the party and the admin can compute `K_sign`, so no third party can publish into this conversation and the relay discards everything else. Filtering by `#p` instead would let anyone who has observed that tag flood the channel; see [Why not NIP-59](./chat.md#why-not-nip-59).

## Client requirements

Every rule in [Client security requirements](./chat.md#client-security-requirements) applies unchanged — mandatory author filter, bounded backlog with a cursor never advanced past the local clock, the cheapest-check-first validation order, durable inner-id deduplication, rate limiting, and the isolation invariant — with one substitution: the accepted inner signers are the party's trade key **and the admin's pubkey**, the latter learned from the `admin-took-dispute` message above.
