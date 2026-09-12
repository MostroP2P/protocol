# Reputation keyset event

An issuer of portable reputation holds one secret scalar per cell of the
[band grid](./reputation_bands.md) and publishes the matching public points as
an addressable Nostr event. A destination needs this event, and nothing else
from the issuer, to verify a token: the key that verifies a token is what
states its band.

## The event

Kind `38387`, an [addressable
event](https://github.com/nostr-protocol/nips/blob/master/01.md#kinds), signed
with the issuer's own key:

```json
[
  "EVENT",
  "RAND",
  {
    "id": "<Event id>",
    "pubkey": "<Issuer's pubkey>",
    "created_at": 1767225600,
    "kind": 38387,
    "tags": [
      ["d", "reputation-keyset:2026"],
      ["epoch", "2026"],
      ["y", "mostro", "[Mostro instance name]"],
      ["z", "reputation-keyset"]
    ],
    "content": "{\"cells\":{\"reviews:200+|rating:4.5+|age:24m+\":\"02a1b2…\",\"reviews:50-200|rating:4.5+|age:24m+\":\"03c3d4…\"},\"merges\":{\"reviews:200+|rating:0-4.0|age:24m+\":\"reviews:50-200|rating:0-4.0|age:24m+\"}}",
    "sig": "<Issuer's signature>"
  }
]
```

## Tags

- `d` < Keyset identifier >: `reputation-keyset:<epoch>`.
- `epoch` < Epoch >: the epoch label, which MUST equal the `<epoch>` part of
  the `d` tag.
- `y` < Platform >: platform identifier, as on every other Mostro event.
- `z` < Document >: `reputation-keyset`.

The epoch is part of the `d` tag and not only a separate tag. Addressable
events replace on `(kind, pubkey, d)`, so a fixed `d` would make every
rotation **delete the previous keyset** from relays and leave a destination
unable to verify tokens from an epoch it still accepts.

## Content

A stringified JSON object with two members:

```json
{
  "cells":  { "<cell id>": "<compressed point, lowercase hex>" },
  "merges": { "<raw cell id>": "<effective cell id>" }
}
```

- `cells` holds only the **effective** cells, the ones that actually have a
  key. Each value is the public point `P_cell = x_cell·G` in 33-byte
  compressed SEC1 encoding, lowercase hex, 66 characters.
- `merges` is the published raw → effective map from the
  [K-anonymity merge](./reputation_bands.md#k-anonymity-merge). It lets a
  client reproduce the issuer's assignment instead of trusting it.

Every one of the 36 raw cells MUST appear as a key of exactly one of the two
maps. A client folds its own raw cell through `merges` **transitively**, and
MUST terminate rather than loop if an issuer publishes a cyclic map.

## Verifying a keyset before using it

A consumer MUST discard an event, and MUST NOT cache it, unless all of the
following hold. Otherwise a relay or a third party could substitute a
`P_cell` and have tokens verify against a key the issuer never chose.

1. The Nostr signature is valid.
2. `pubkey` equals the issuer key the destination trusts — the same 32-byte
   x-only key that fills the `issuer` field of every token it signs.
3. `kind` is `38387`.
4. `d` is `reputation-keyset:<epoch>` and the `epoch` tag equals that
   `<epoch>`.
5. `content` parses into `cells` and `merges`, every key of both is a valid
   [cell id](./reputation_bands.md#cell-id), every value of `cells` is a valid
   compressed point on secp256k1, and every value of `merges` is a cell id.

## Epochs

The epoch label is a **protocol constant, not an issuer choice**: the UTC
calendar year, as four ASCII digits. Every destination therefore means the
same thing by "the current epoch", whichever issuer minted the token.

A destination MUST accept the **current and the previous epoch only**. Stale
reputation cannot be imported years later, and an issuer can retire keys by
letting an epoch age out.

### A keyset is immutable within its epoch

The merge depends on cell populations, and populations move. An issuer that
recomputed the merge on every restart would replace the epoch's event with one
whose `cells` no longer carries the `P_cell` of a cell folded away since, and
every token minted against that cell would become unverifiable while its epoch
is still accepted.

An issuer therefore MUST compute the merge once, when the epoch opens, persist
the resulting raw → effective map, and republish byte-identical `cells` and
`merges` on every later startup within that epoch. Population changes shape
only the next epoch's keyset.

A client that has a keyset cached SHOULD compare a freshly fetched one against
it and treat changed `cells` or `merges` under the same `d` as an issuer
fault.

## Issuer identity is one key

The key that signs the keyset event, the key a destination lists as trusted,
and the `issuer` field of every token are the **same key**. How an issuer
derives or stores the per-cell secret scalars behind the published points is
its own business and not part of this protocol; only the points are.

An issuer SHOULD keep this key separate from any other Nostr key it uses, so
reputation issuance can be rotated or revoked on its own. A Mostro instance
acting as issuer uses its daemon key.
