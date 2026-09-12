# Reputation portability

A user with reputation on another venue — lnp2pBot, or another Mostro
instance — can carry it to a Mostro instance without giving any operator, or
anyone reading relays, a way to link the two identities. This page specifies
the wire format: the byte encoding, the token, the four actions and the checks
a destination runs before accepting one.

Portability is a **copy**, not a move: the source keeps its reputation.
Reputation travels as a [band](./reputation_bands.md), never as an exact
figure, and it is verified against the issuer's
[keyset event](./reputation_keyset.md).

## Why blind signatures

Two attacks decide the design.

**Fingerprinting by exact values.** Answered by bands, on their own page.

**The issuer sees what it signs.** If the issuer signed a plaintext containing
the destination pubkey, or even a hash of it, it could recognise that value
when the token is redeemed and link the two identities itself. So the issuer
signs **blind**: it never sees the message it puts its signature on.

The token must then be verified by the **destination**, which is not the party
that issued it. That rules out Cashu's BDHKE, where the mint verifies its own
tokens with the private key: a third party holding only `K = k·G` cannot decide
whether `C = k·Y` without solving Decisional Diffie-Hellman. The scheme is
therefore **blind Schnorr on secp256k1**, which is blind at issuance and
*publicly* verifiable at redemption.

Concurrent blind Schnorr sessions are subject to the ROS attack, so issuers use
the **clause variant** (Fuchsbauer–Kiltz–Loss): the issuer opens two nonce
clauses, the client prepares a challenge for each, and the issuer answers only
one, chosen at random. An issuer MUST additionally allow at most one open
session per user at a time.

### What this does not hide

The cryptography makes the token unlinkable, but it cannot hide *when* things
happen. With few migrations per day, a colluding source and destination can
match "issued at 14:02" with "redeemed at 14:05". This is bounded, not solved:

- A client MUST wait a random delay, drawn from a window at least 24 hours
  wide, between obtaining a token and redeeming it.
- An issuer MUST NOT store an issuance timestamp finer than the day, so the
  correlation has to be done live and cannot be reconstructed from a leaked
  database later.
- None of this helps the very first migrant. The anonymity set is the number
  of migrations from the same issuer inside the delay window.

## Canonical encoding

Everything hashed or transmitted has exactly one byte encoding, so independent
implementations cannot derive different challenges for the same token. Every
field is fixed length, which makes concatenation unambiguous without framing.

| Element | Encoding |
|---|---|
| Curve points | 33-byte compressed SEC1 (`0x02`/`0x03` prefix) |
| Scalars | 32-byte big-endian integer in `1..n-1`. A parser MUST reject `0` and any value `≥ n` rather than reducing it, so each scalar has exactly one encoding. Reduction mod `n` happens only inside arithmetic |
| Tagged hash | `H_tag(x) = SHA256(SHA256(tag) ‖ SHA256(tag) ‖ x)`, as in [BIP-340](https://github.com/bitcoin/bips/blob/master/bip-0340.mediawiki) |
| Challenge | `c = int(H_"mostro/reputation/challenge/v1"(R ‖ m)) mod n`, with `R` compressed |
| `m` | `"repv1:"` (6 ASCII bytes) ‖ 32-byte x-only destination identity pubkey ‖ 32-byte random nonce — **70 bytes exactly** |
| Token id | `H_"mostro/reputation/token/v1"(m)` |
| Cell id | As defined in [Reputation bands](./reputation_bands.md#cell-id) |

Binary fields travel as **lowercase hex**.

Points are compressed rather than x-only, and the challenge is a plain tagged
hash rather than BIP-340's, on purpose. BIP-340 normalises `R` to even Y, and
the client's blinded `R_i = R'_i + α_i·G + β_i·P_cell` has unpredictable parity
that the signer cannot compensate for, so x-only encoding would break the
blinding.

## The token

| Field | Type | Bytes | Meaning |
|---|---|---|---|
| `issuer` | hex | 32 | x-only pubkey of the issuer's identity key, the one that signed the keyset |
| `epoch` | string | – | epoch label; MUST match the keyset's `epoch` tag |
| `cell` | string | – | effective [cell id](./reputation_bands.md#cell-id); MUST be a key of the keyset's `cells` |
| `m` | hex | 70 | the message above; the `repv1:` prefix is checked before use |
| `r_point` | hex | 33 | compressed `R_b` |
| `s` | hex | 32 | the unblinded scalar |

A parser MUST reject the token **before any curve arithmetic** when a field is
missing, duplicated, of the wrong length, not lowercase hex, when a point does
not decode onto the curve, or when a scalar is zero or `≥ n`.

Field order in JSON is not significant and nothing is hashed over the JSON: the
token id and the challenge are computed from the decoded bytes only, so the
same token serialised by any implementation verifies identically.

## Actions

Four actions, carried by a `reputation` message like every other non-order
channel:

| Wire name | Direction | Payload |
|---|---|---|
| `export-reputation` | client → issuer | `blinded_reputation_request`, or `null` to open |
| `reputation-exported` | issuer → client | `blinded_reputation_response` |
| `import-reputation` | client → destination | `reputation_token` |
| `reputation-imported` | destination → client | `null` |

These messages are strictly request/response over targeted encrypted direct
messages, so a client that does not implement them is never handed one. The
protocol version stays `2` and the actions do not gate on it.

## Export: obtaining a token

Two round trips against the issuer. The blinding runs on the user's device;
the issuer never sees the message it signs.

**1. The client opens a session.** It has already built
`m = "repv1:" ‖ destination_identity_pubkey ‖ nonce`, which it keeps to itself.

```json
[
  {
    "reputation": {
      "version": 2,
      "action": "export-reputation",
      "payload": null
    }
  },
  "<signature>",
  null
]
```

**2. The issuer checks eligibility and answers with the cell and two nonce
points** `R'_0 = k_0·G` and `R'_1 = k_1·G`:

```json
[
  {
    "reputation": {
      "version": 2,
      "action": "reputation-exported",
      "payload": {
        "blinded_reputation_response": {
          "session": "<session id>",
          "cell": "reviews:200+|rating:4.5+|age:24m+",
          "r_points": ["02a1b2…", "03c3d4…"]
        }
      }
    }
  },
  null,
  null
]
```

**3. The client validates the cell against its own statistics.** The user knows
their own review count, rating and account age, so the client computes its raw
cell, folds it through the keyset's published `merges` map transitively, and
MUST abort if the result differs from the cell the issuer named.

This check is not optional. Without it an adversarial issuer could tag a user
by assigning a deliberately rare cell and recognise it at redemption, and no
proof about the signature itself would catch it, because the signature would be
perfectly valid.

**4. The client blinds both clauses** with random `α_i`, `β_i`, computing
`R_i = R'_i + α_i·G + β_i·P_cell` and `c'_i = H(R_i ‖ m) + β_i`:

```json
[
  {
    "reputation": {
      "version": 2,
      "action": "export-reputation",
      "payload": {
        "blinded_reputation_request": {
          "session": "<session id>",
          "challenges": ["<c'_0>", "<c'_1>"]
        }
      }
    }
  },
  "<signature>",
  null
]
```

**5. The issuer picks `b ∈ {0,1}` at random** and answers only that clause with
`s'_b = k_b + c'_b·x_cell`. Answering one clause is what defeats ROS:

```json
[
  {
    "reputation": {
      "version": 2,
      "action": "reputation-exported",
      "payload": {
        "blinded_reputation_response": {
          "session": "<session id>",
          "clause": 1,
          "s": "<s'_b>"
        }
      }
    }
  },
  null,
  null
]
```

**6. The client unblinds** `s = s'_b + α_b`, and MUST verify
`s·G == R_b + H(R_b ‖ m)·P_cell` against the published keyset **before**
storing the token. This is the same check the destination will run, so a token
that would be rejected later is caught at once, as is an issuer that signed
with a key outside its keyset.

The session is stateful across the two round trips, so both sides persist it.
An abandoned session expires and MUST NOT consume the user's once-only
issuance flag.

### Once only

An issuer MUST issue at most one token per user, and MUST record that it did
so with a day-truncated timestamp — never finer, since a second-precision
value in a leaked database is exactly what the timing correlation above needs.
A user who wants reputation from two issuers redeems one token from each.

## Import: redeeming a token

The client sends the token to the destination, after the random delay:

```json
[
  {
    "reputation": {
      "version": 2,
      "action": "import-reputation",
      "payload": {
        "reputation_token": {
          "issuer": "<issuer x-only pubkey>",
          "epoch": "2026",
          "cell": "reviews:200+|rating:4.5+|age:24m+",
          "m": "<70 bytes, hex>",
          "r_point": "<33 bytes, hex>",
          "s": "<32 bytes, hex>"
        }
      }
    }
  },
  "<signature>",
  null
]
```

The destination MUST perform all of the following, and MUST apply the seed and
record the redemption in a single transaction:

1. The `issuer` is in its trusted list, and `epoch` is the current or the
   previous one.
2. `s·G == R + H(R ‖ m)·P_cell` verifies, with `P_cell` taken from the
   `cells` map published for that issuer and epoch. This needs only public
   data, which is the whole reason for blind Schnorr; **the cell whose key
   verifies is the band**, so a token cannot claim a band it was not signed
   for.
3. The pubkey embedded in `m` equals the identity the transport *proved* —
   not one the sender claims. On protocol v2 that proof is the identity
   signature bound to the trade key authoring the event, so it cannot be
   grafted from another sender.
4. The token id is not already in the destination's redeemed set, and this
   `(issuer, identity)` pair has not been redeemed before.
5. The seed for the token's cell is applied as specified in
   [what a band means on the destination](./reputation_bands.md#what-a-band-means-on-the-destination).

Check 3 binds the token to one identity. Selling a token therefore means
handing over the identity key, which is the same risk as selling the account,
and cannot be prevented cryptographically since the issuer signs a pubkey it
never sees.

On success the destination replies `reputation-imported` and republishes the
user's [rating event](./user_rating.md). On failure it replies `cant-do`.

### One token, many destinations

The token binds an **identity**, not an instance. The issuer never learns
where it is spent, and the same identity may redeem the same token on every
instance that trusts the issuer — reputation is per-instance, so this grants
exactly what the user had, once each.

Issuing per-destination tokens would tell the issuer which instances a user
uses, and restricting the number of destinations is unenforceable without a
shared ledger. Conversely, a fresh identity per instance is **not** supported:
it would need one token per identity, and nothing could then stop one person
redeeming two of them under two identities on the same instance. Sybil
resistance wins.

## cant-do reasons

| Reason | Meaning |
|---|---|
| `untrusted-reputation-issuer` | The token's issuer is not in the destination's trusted list |
| `invalid-reputation-token` | Malformed token, or the signature does not verify against the cell's key |
| `reputation-already-redeemed` | This token id, or this `(issuer, identity)` pair, was already redeemed here |
| `reputation-identity-mismatch` | The pubkey inside `m` is not the identity the transport proved |
| `expired-reputation-keyset` | The token's epoch is neither the current nor the previous one |
| `not-eligible-for-reputation-export` | The user does not meet the [eligibility rules](./reputation_bands.md#eligibility) |
| `reputation-already-exported` | This user has already been issued a token |

A client that does not recognise a reason MUST degrade it to `unknown` rather
than fail.
