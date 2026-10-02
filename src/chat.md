# Peer-to-peer Chat

To communicate directly, the buyer and the seller do not use the `Message` scheme explained [here](https://mostro.network/protocol/overview.html), because this communication excludes the Mostro daemon.

Messages are **not** gift wrapped. Each party publishes a **kind 14** event signed with a key derived from the ECDH secret shared by the two trade keys, carrying a NIP-44 encrypted **kind 1** event signed with the sender's trade key. No ephemeral keys are involved.

## Why not NIP-59

Earlier revisions of this document used a simplified NIP-59 gift wrap: the outer event was signed with a **random ephemeral key** and `p`-tagged to the shared pubkey. That construction was replaced because it is vulnerable and buys nothing.

**It buys nothing.** In standard NIP-59 the ephemeral key matters because the `p` tag points at the recipient's **real identity key**: without it, an observer would see "identity X writes to identity Y". In this protocol the `p` tag already points at a shared key that is anonymous and unique per order — it is nobody's identity. An observer saw `ephemeral → shared`; it now sees `shared → shared`. **Neither form ever exposes a trade key, and both group events identically.** No privacy is lost.

**It is vulnerable.** The shared pubkey travels in clear text in the `p` tag of every event. Any observer scraping relays for these events harvests the shared pubkeys of every active conversation. Because the outer event is signed by a fresh random key, an attacker's events are **indistinguishable from genuine ones** until the recipient has already downloaded and attempted to decrypt them — one ECDH per event. There is nothing cheap to filter on, and no author to rate-limit: by design, every event appears to come from a different sender.

That enables a cheap, anonymous, and profitable attack. Flooding the shared pubkey of a trade that is waiting on a counterparty can exhaust the victim's client until the trade's deadline passes, and the attacker does not even need to be a party to that trade.

Signing the outer event with the shared key removes the attack at its root. The shared pubkey is still observable, but **producing a valid event requires the shared private scalar**, which can only be computed from one of the two trade private keys. A third party is cryptographically unable to publish into the conversation, so clients can filter on the author and the relay discards everything else before it ever reaches the client.

The only party who can still flood is the counterparty, who is a single, stable, attributable author — see [Client security requirements](#client-security-requirements).

## Shared Key

Messages are not addressed to the counterparty's trade pubkey but to a key known only to both parties.

We use **Elliptic Curve Diffie-Hellman** (ECDH) over the two trade keys to obtain a shared secret:

```
Alice                            Bob
-----                            -----
Private Key: a                   Private Key: b
Public Key: A = a * G            Public Key: B = b * G
   (G is the curve’s base point)

1. Alice sends A to Bob  ----->  Bob receives A
2. Bob sends B to Alice  <-----  Alice receives B

Alice computes:                  Bob computes:
Shared Secret = a * B            Shared Secret = b * A
              = a * (b * G)      = b * (a * G)
              = ab * G           = ba * G
              = Same Shared Secret!
```

Because trade keys are derived per order, the shared secret is unique per order and carries no link to any long-lived identity.

### Key derivation

The shared secret is **not** used directly. Two keys are derived from it with domain separation, using HKDF-SHA256 (RFC 5869) with an empty salt:

```
shared  = ECDH(own_trade_privkey, peer_trade_pubkey)          // 32 bytes

K_conv  = HKDF-SHA256(ikm = shared, info = "mostro:chat:conv:v1", L = 32)
K_sign  = HKDF-SHA256(ikm = shared, info = "mostro:chat:sign:v1", L = 32)
```

Both outputs are interpreted as secp256k1 secret keys. In the negligible event that an output is not a valid secret key (zero, or greater than or equal to the curve order), implementations MUST re-derive by appending a single incrementing byte to `info` until a valid key is produced.

| Key | Role | Who holds it |
|-----|------|--------------|
| `K_conv` | NIP-44 encryption and decryption of the payload. `pub(K_conv)` is the conversation address carried in the `p` tag. | Both parties. Reconstructed by a solver from a [channel proof](#dispute-disclosure). |
| `K_sign` | Signs the outer event. `pub(K_sign)` is the author every client filters on. | Both parties. Reconstructed by a solver from a [channel proof](#dispute-disclosure). |

Separating the two keeps the address a client publishes in the clear distinct from the key that authorises writing to it, and makes a read-only share possible in general: anyone handed `K_conv` alone can read the conversation without being able to publish into it. An **evidentiary** disclosure to a dispute solver is a different matter — it has to be verifiable, and verifiability necessarily reveals the ECDH secret and with it both keys. See [Dispute disclosure](#dispute-disclosure).

## Event structure

### 1. Inner event

A kind 1 event with the message, signed by the sender's **trade key**, timestamped at the moment the message is sent:

```json
{
  "id": "<Event Id>",
  "pubkey": "<Index N pubkey (trade key)>",
  "kind": 1,
  "created_at": 1691518405,
  "content": "Let’s reestablish the peer-to-peer nature of Bitcoin!",
  "tags": [],
  "sig": "<Index N (trade key) signature>"
}
```

The inner signature is the **only** authentication of the sender. Both parties hold `K_sign`, so the outer signature proves the event came from the conversation but not which side wrote it.

### 2. Outer event

The JSON-encoded inner event is NIP-44 encrypted under `K_conv` and placed in the `content` of a kind 14 event, `p`-tagged to `pub(K_conv)` and signed with `K_sign`:

```json
{
  "id": "<Event Id>",
  "pubkey": "<pub(K_sign)>",
  "kind": 14,
  "created_at": 1691518405,
  "content": "<NIP-44 encrypted inner event>",
  "tags": [["p", "<pub(K_conv)>"]],
  "sig": "<K_sign signature>"
}
```

The outer `created_at` MUST be the real time the message is sent, and MUST equal the inner `created_at` up to a small clock tolerance. The timestamp tweaking that NIP-59 recommends does not apply here: it would break `since`-based synchronization, and with no identity key exposed there is nothing for time analysis to correlate to.

## Encrypting payloads

Encryption follows [NIP-44](https://github.com/nostr-protocol/nips/blob/master/44.md) v2 over the JSON-encoded inner event, using `K_conv` as **both** sides of the key exchange:

```
conversation_key = NIP-44_conversation_key(K_conv_privkey, pub(K_conv))
```

This is self-encryption: the NIP-44 conversation key is derived from `ECDH(k, k·G)`, which is deterministic and computed identically by both parties, and by anyone holding `K_conv`. It is an unusual but valid use of NIP-44 — implementations MUST NOT reject an encryption or decryption call where the secret key and the public key belong to the same keypair. This is the most likely point of divergence between implementations; verify against the [test vector](#test-vector).

## Disambiguating from protocol v2 messages

Kind 14 is also used by the [protocol v2 transport](https://mostro.network/protocol/overview.html) for client↔daemon messages. The two never collide, because the **author** differs:

| Traffic | Author | `p` tag |
|---------|--------|---------|
| Client → daemon | Sender's trade key | Mostro node pubkey |
| Daemon → client | Mostro node pubkey | Recipient's trade key |
| Peer chat | `pub(K_sign)` | `pub(K_conv)` |

Clients MUST route incoming kind 14 events by author:

- author is the active Mostro node pubkey → daemon message;
- author is `pub(K_sign)` of an active conversation → peer chat;
- anything else → ignore.

Routing by `p` tag alone is not sufficient, and reintroduces the vulnerability described above.

## Client security requirements

### Subscription

**Clients MUST subscribe with `authors = [pub(K_sign)]`.**

This single rule is what eliminates third-party flooding: the relay drops every event that is not signed by the conversation key, so junk never reaches the client and costs it nothing. A client that filters only by `#p` is fully exposed to the attack this design exists to prevent, even though it will appear to work correctly.

Clients MUST also bound the backlog: subscribe with `since` set to the last processed timestamp, persisted locally, together with a `limit`. An unbounded subscription re-downloads the entire stored history on every reconnection and every application start, which turns a one-off flood into permanent damage that survives reinstalling the application.

**The persisted `since` cursor MUST NOT be advanced beyond the client's own clock.** A counterparty signs both events, so it can set both timestamps to the same far-future value and satisfy the relative check in step 13. If the client then stores that value as its cursor, its own subscription filters out every honest message that follows, and the conversation stays dead until that future date — a permanent denial of service from a single message, surviving restarts. Clamping the cursor to `min(accepted_timestamp, local_now)`, together with the absolute bound in step 3, closes this.

### Validation order

Each incoming event MUST be validated cheapest-check-first, so that an abusive peer cannot force expensive work:

1. **Author** is `pub(K_sign)` — otherwise discard.
2. **`p` tag**: exactly one, equal to `pub(K_conv)` — otherwise discard. See [below](#the-p-tag-is-part-of-the-contract).
3. **Absolute timestamp bound**: `outer.created_at` is not further into the future than the client's tolerance for clock skew (60 seconds is a reasonable default), measured against the client's own clock — otherwise discard. Timestamps in the past are always acceptable, since offline catch-up is legitimate.
4. **Size** is within the client's limit (64 KiB is a reasonable default) — otherwise discard.
5. **Outer event id** has not been seen before (bounded LRU) — otherwise discard.
6. **Rate-limit budget** for this conversation is available — otherwise discard.
7. **Outer signature** verifies.
8. Only now, **NIP-44 decrypt** with `K_conv`.
9. **Inner signature** verifies — this is the sender authentication and MUST NOT be skipped. Reading the inner `pubkey` field without verifying the signature accepts forged senders.
10. **Inner pubkey** is the buyer's or the seller's trade key for this order — otherwise discard. No other signer is accepted, including a dispute solver.
11. **Inner kind** is 1 — otherwise discard.
12. **Inner event id** has not been seen before, checked against **durable** state — otherwise discard.
13. **Relative timestamp bound**: `|inner.created_at − outer.created_at|` is within the same tolerance — otherwise discard.

Steps 1 through 6 are all reachable without any cryptographic work, and steps 2 and 3 in particular cost a tag comparison and an integer comparison.

### The `p` tag is part of the contract

Producers MUST emit exactly one `p` tag, set to `pub(K_conv)`, and MUST NOT allow application-supplied tags to add or override it. Recipients MUST reject anything else.

This is not hygiene. A recipient filters by author, so a message carrying a wrong or missing `p` tag still reaches them and decrypts correctly — but a dispute solver locates the conversation by querying `#p = pub(K_conv)`. Without this rule a party could send messages that their counterparty sees normally yet are **invisible in the transcript the solver retrieves**, letting them shape the evidence after the fact.

### Replay protection

Both parties hold `K_sign`, so either can re-publish a previously received inner event inside a fresh wrapper. The inner signature is genuine, so it verifies — a peer could reinject an old "I sent the fiat" message, or reshape the transcript a solver will read during a dispute.

**Deduplication on the inner event id (step 12) is what rejects this**, in every case: the inner id is a hash over the sender's pubkey, content and `created_at`, so a re-wrapped message keeps the id it had the first time. The relative timestamp bound (step 13) is not the defence — a peer who re-wraps within the tolerance window would satisfy it. What that bound does is **limit how far back deduplication state has to reach**, by making a re-wrap detectable as stale once it falls outside the window.

Because of that, dedup state on the inner id MUST be **durable**, not a bounded in-memory cache: an entry evicted from an LRU makes the corresponding message replayable again. In practice this is free — clients already persist message history in order to advance the `since` cursor, so retaining each accepted inner event id alongside its message is one identifier per stored message. The bounded LRU in step 5 is a different thing: it applies to the *outer* id, is only a cheap pre-decryption filter against duplicate relay deliveries, and carries no security requirement.

Retention MUST cover at least every timestamp the client will still accept, which follows from the `since` cursor: an event older than the cursor is not requested, and one newer is inside the retained range.

### Rate limiting

The counterparty is the only party who can flood, and is now a single stable author. Clients SHOULD apply a token bucket per conversation — on the order of 30 messages per minute sustained with a burst of 60 — and discard excess events **before decrypting** them.

On sustained violation, a client SHOULD mark the conversation as flooded, stop processing it, and inform the user, while leaving the trade fully operational.

Clients SHOULD also cap the number of messages and total bytes stored per trade.

### Isolation

**Chat processing MUST NOT be able to block, delay, or crash the order state machine, the daemon transport, or the ability to open a dispute.** Chat must run on its own bounded queue; under pressure a client drops chat, never trade traffic. Processing the chat backlog MUST NOT block application startup.

This is the invariant that prevents any future flaw in this channel from costing a user their funds: a chat that stops working is an inconvenience, a trade that cannot be disputed is a loss.

### Evidence

A flood is attributable to `pub(K_sign)`, and every accepted message to a trade key. Clients SHOULD retain a bounded sample and counters, which are usable as evidence in a dispute.

### Presentation

Clients SHOULD order messages by the validated inner `created_at`. Because that value is chosen by the sender, it MUST NOT be trusted beyond the tolerance enforced in step 11.

## Dispute disclosure

Either party may voluntarily disclose the conversation to the solver who took the dispute. Nothing in this protocol lets a solver read a channel the parties have not handed over.

A disclosure MUST be a **channel proof**: evidence that the disclosed conversation is the one derived from the two trade keys of the disputed order. A solver MUST NOT accept a bare `K_conv`. The rest of this section explains why, then specifies the proof.

### A bare key is not evidence

`pub(K_conv)` is derived from the ECDH secret of the two trade keys, and trade keys are per order, so the address is unique to one trade. **A solver cannot check that.** Deriving the shared secret requires one of the two trade *private* keys and the solver holds neither, so 32 bytes handed over by a party decrypt *a* conversation with nothing tying it to *this* order.

That gap is exploitable. Take Alice, trading with Bob on order X and meaning to defraud him:

> In parallel Alice runs order Y with Carol — an account she also controls — and plays out a conversation in which "Carol" behaves like the scammer. She then opens a dispute on order X and gives the solver `K_conv` of the order-Y channel.

Nothing in that transcript is forged. The inner signatures verify, the timestamps are real, the counterparty exists. It is an authentic conversation that happens to belong to a different trade, and reading it more carefully does not help — the solver sees a victim.

Applying step 10 of the [validation order](#validation-order) on the solver's side — every inner signer must be the buyer's or the seller's trade key **for the disputed order**, taken from the dispute the daemon published — defeats that particular construction, because the order-Y messages are signed by order-Y trade keys. It does not defeat the general case. Alice can derive a channel against any pubkey she controls, sign the inner events with her genuine order-X trade key, and disclose that instead. Every signer then checks out.

What she cannot do is forge Bob's messages, which would need his trade private key. Her fabricated channel is therefore always a monologue — and a monologue is enough. It lets her bury a real conversation in which Bob answered and put in its place one in which he never did. "The counterparty stopped replying" is an ordinary outcome, and nothing distinguishes the two.

Note what does **not** fix this. Requiring both parties to disclose fails precisely when it is needed, since an honest counterparty may be offline, and the attack is built around a counterparty who says nothing. Carrying the order id inside the chat fails too: Alice signs her own inner events, so she would simply put the right order id in the fabricated ones.

The proof below removes the choice: the only channel a party is able to prove is the real one.

### What the proof establishes

Write `A` and `B` for the buyer's and the seller's trade pubkeys of the disputed order, and `A'`, `B'` for their even-Y lifts (BIP-340 `lift_x`), so that `A' = α·G` and `B' = β·G`. The conversation's ECDH point is

```
S = α·B' = β·A' = αβ·G
```

and the shared secret both parties feed to HKDF is `x(S)`, its x-coordinate.

A discloser proves knowledge of a scalar `α` satisfying **both** relations at once:

```
A' = α·G          and          S = α·B'
```

This is a statement of equality of discrete logarithms — `log_G(A') = log_B'(S)` — which the classical Chaum-Pedersen sigma protocol proves, made non-interactive with Fiat-Shamir. Two properties carry the whole argument:

- **`S` is forced.** Given `α` and `B'` there is exactly one `α·B'`. A prover cannot aim the proof at another point, so they cannot aim it at another conversation.
- **`B'` comes from the verifier.** The solver takes both trade pubkeys from the dispute the daemon published, never from the party disclosing. Proving a channel with a sockpuppet would require the discrete log of the real counterparty's trade key.

Either party can produce the proof independently, with the mirrored statement `B' = β·G ∧ S = β·A'`, and both reach the identical point. A silent, absent or hostile counterparty therefore costs nothing: one party alone establishes which conversation belongs to the order.

The proof reveals nothing beyond the conversation it opens. It is zero-knowledge with respect to `α`: the trade private key is not recoverable from it, so a disclosure never endangers the funds of the trade it discloses.

### Construction

`H_dleq` and `H_nonce` are BIP-340 tagged hashes, `SHA256(SHA256(tag) || SHA256(tag) || m)`, with tags `mostro:chat:dleq:v1` and `mostro:chat:dleq:nonce:v1`. Points are serialized **compressed** (33 bytes), `order_id` is the 16 raw bytes of the order's UUID, and `n` is the order of the secp256k1 group.

```
prove(own_trade_privkey, own, peer, order_id):

    α     = scalar of lift_x(own)              # negate the trade secret if its point is odd-Y
    own'  = α·G                                # equals lift_x(own)
    peer' = lift_x(peer)
    S     = α·peer'

    k     = int(H_nonce(α || order_id || own' || peer' || S)) mod n
    R1    = k·G
    R2    = k·peer'
    e     = int(H_dleq(order_id || own' || peer' || S || R1 || R2)) mod n
    s     = (k + e·α) mod n

    return (S, e, s)
```

```
verify(own, peer, order_id, S, e, s):

    own'  = lift_x(own)
    peer' = lift_x(peer)
    reject unless 0 < e < n and 0 < s < n
    R1    = s·G     − e·own'
    R2    = s·peer' − e·S
    reject if R1 or R2 is the point at infinity
    accept iff e == int(H_dleq(order_id || own' || peer' || S || R1 || R2)) mod n

    shared = x(S)                              # the input to Key derivation
```

Four details decide whether two implementations interoperate:

**Parity.** Nostr pubkeys are x-only, so a verifier can only reconstruct the even-Y point, and a prover whose trade key has odd-Y must use `n − a` in place of `a`. This does not change the shared secret — negating the scalar negates the product and leaves the x-coordinate untouched — which is also why the proof agrees with a plain ECDH that does not normalize the secret key at all.

**`S` travels compressed, with its parity byte.** `x(S)` alone is not enough: the verification equation needs the point, and lifting `x(S)` to even Y yields `−S` half the time.

**The nonce is derived deterministically.** Reusing `k` across two different challenges reveals `α`, which is the trade private key. Deriving it from the secret and the statement removes that failure mode, and makes proofs byte-reproducible, which is what lets the [test vector](#test-vector) pin exact values.

**The challenge commits to the order id and to both pubkeys in role order.** A proof is therefore not reusable for another order, and not replayable with the two roles swapped.

### Disclosure payload

A disclosure is sent to the solver **inside the [dispute chat](./dispute_chat.md)**, which is already NIP-44 encrypted to that solver. `shared_point` is secret-bearing — its x-coordinate *is* the ECDH secret — so a disclosure MUST NOT travel in the clear.

```json
{
  "channel_proof": "v1",
  "order_id": "9f2c0e7a-4b18-4d3e-9a51-6d2c8374e105",
  "role": "buyer",
  "shared_point": "02def6633a53d07d1e829484c4d4bdbbeed2f4b14c21743e63871c174338e39475",
  "e": "f634a28af956b8e14ad166ea7340884ee517c725bc5db37c2e0bf598e3e4f48a",
  "s": "c8dbf0716a2839a814241a7790767b3a019170f1bf360a5fbcabd1491daba9af"
}
```

`role` tells the solver which of the two trade pubkeys plays `own`. **No key is transmitted:** the solver reconstructs `K_conv` and `K_sign` from `x(shared_point)` with the same HKDF the parties use.

The proof authenticates its producer as a holder of that trade key, but it is replayable by anyone who has seen it — it establishes the statement, not the freshness of the sender. That is not a gap here: the dispute chat already authenticates the sender, and a replay conveys only a secret that was disclosed already.

### Solver requirements

A solver MUST:

1. Take `A` and `B` from the dispute published by the daemon. Trade pubkeys supplied by a party MUST be ignored — supplying them is the verifier's job, and it is what the whole check rests on.
2. Verify the proof. A disclosure that does not verify **carries no evidentiary weight**, and neither does a bare `K_conv`, however convincing the transcript it decrypts.
3. Derive `K_conv` and `K_sign` from `x(S)` exactly as in [Key derivation](#key-derivation).
4. Retrieve the transcript themselves, subscribing with `authors = [pub(K_sign)]`. A transcript uploaded by a party MUST NOT be used: a party can leave out of it whatever they like.
5. Apply the full [validation order](#validation-order) to every event retrieved, step 10 included, against `A` and `B`. An event that fails step 2 — missing or wrong `p` tag — MUST NOT be dropped silently: per [The `p` tag is part of the contract](#the-p-tag-is-part-of-the-contract) that is a message engineered to stay out of a `#p` query, and it is itself evidence.

Filtering by author rather than by `#p` is what makes point 5 work. An event carrying a tampered `p` tag still reaches a solver who filters by author, so the tampering becomes visible instead of achieving what it was for.

If both parties disclose, the two proofs necessarily yield the same point, because neither can prove any other. Disclosures corroborate; they cannot contradict. One side's disclosure is complete on its own.

### Why a verifiable disclosure cannot stay read-only

Earlier revisions of this document described the disclosure as read-only: the solver received `K_conv` and never `K_sign`, so they could decrypt but not publish. A verifiable disclosure cannot keep that shape, and it is worth recording why, so that implementers do not go hunting for a cleverer scheme.

Both keys come from the same 32 bytes, `x(S)`. For a solver to check that a `K_conv` they were handed is the right one they have to recompute it, which means learning `x(S)` — and `K_sign` falls out of that same value. Proving the derivation *without* revealing `x(S)` would mean proving an HKDF-SHA256 evaluation in zero knowledge, wildly out of proportion for this channel. Nor is there a second bilateral secret to hang `K_sign` on: `αβ·G` is the only value the two trade keys jointly determine, and no additional shared randomness exists without an online handshake that would itself need a channel.

The trade-off is therefore real, and worth taking: **an unverifiable read-only key is worse than a verifiable full one.** A key the solver cannot check is exactly the primitive the substitution attack needs.

What is lost is narrower than it appears, because the read-only property moves from key custody to a validation rule that already exists. Step 10 accepts an inner signer only if it is the buyer's or the seller's trade key for the order — *including a dispute solver* — so a solver holding `K_sign` can publish events that reach both clients and every one of them is discarded. Forging a message still requires a trade private key, exactly as before.

What a solver does gain is the ability to put junk on the channel. The per-conversation [rate limit](#rate-limiting) bounds the volume, but during a dispute `pub(K_sign)` has three holders instead of two, so a flood is no longer attributable to the counterparty by elimination. Clients SHOULD NOT treat authorship by `pub(K_sign)` as evidence that the counterparty sent something once a dispute has been opened on the order.

## Relay considerations

Kind 14 falls outside the range NIP-01 defines as regular (`1000 ≤ n < 10000`), and [NIP-17](https://github.com/nostr-protocol/nips/blob/master/17.md) specifies kind 14 as an unsigned rumor that is never published directly. Relay behaviour for a signed, published kind 14 is therefore not guaranteed by any NIP.

**Offline delivery depends on relays storing these events.** Operators and client implementers MUST verify empirically that the relays they target store and serve kind 14, and MUST NOT assume it.

Proof of work per [NIP-13](https://github.com/nostr-protocol/nips/blob/master/13.md) remains optional. It is markedly less relevant than it was under gift wrap: with a stable author, relays can apply their usual per-author rate limiting, which was impossible when every event carried a fresh ephemeral key.

## Migration

This is a breaking wire change affecting `mostrod`, the mobile clients, `mostro-cli`, and any solver tooling. Implementations SHOULD accept both the gift-wrapped form and the form specified here during a transition window, and MUST fix a deprecation date after which only this form is produced. Trades already in flight at the cutover keep the format they started with.

The [channel proof](#dispute-disclosure) is a separate, additive change: it does not alter the wire format of a chat message, only what a disclosure to a solver has to carry. Clients need a `prove` implementation and solver tooling needs `verify`. Until both ends ship it, a solver receiving a bare `K_conv` can still read a conversation — but MUST treat it as unverified, because the substitution described in [A bare key is not evidence](#a-bare-key-is-not-evidence) is what an unverified disclosure permits.

## Code Example

### Rust

Built against `nostr-sdk = { version = "0.44", features = ["nip44"] }`, `hkdf = "0.12"`, `sha2 = "0.10"` and `tokio` with the `full` feature.

```rust
// Leading `::` selects the `hkdf` crate: `nostr_sdk::prelude` also exports a
// module by that name, so a plain `use hkdf::Hkdf` is ambiguous.
use ::hkdf::Hkdf;
use nostr::util::generate_shared_key;
use nostr_sdk::nips::nip44;
use nostr_sdk::prelude::*;
use nostr_sdk::secp256k1::{
    All, Parity, PublicKey as CurvePoint, Scalar as Tweak, Secp256k1, SecretKey as CurveScalar,
};
use sha2::{Digest, Sha256};

/// HKDF `info` strings. Changing either value changes the wire format.
const CONV_INFO: &[u8] = b"mostro:chat:conv:v1";
const SIGN_INFO: &[u8] = b"mostro:chat:sign:v1";

/// Tagged-hash domains for the channel proof. Changing either value changes
/// the wire format.
const DLEQ_TAG: &[u8] = b"mostro:chat:dleq:v1";
const DLEQ_NONCE_TAG: &[u8] = b"mostro:chat:dleq:nonce:v1";

/// Tolerance for clock skew, applied both between the inner and outer
/// `created_at` and against the recipient's own clock.
const MAX_CLOCK_SKEW_SECS: u64 = 60;

/// Upper bound on the encrypted payload, enforced before decrypting.
const MAX_CONTENT_BYTES: usize = 64 * 1024;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let secp = Secp256k1::new();

    // Per-order trade keys.
    let alice_trade =
        Keys::parse("548f68890c49fa42f104c60352395e60ff030b0b407e955f1eed1400d6c0347a")?;
    let bob_trade =
        Keys::parse("f258e73f07386d37133718b6127f873dd7c391b8f43b331ff8254034a13d2943")?;

    // The order these trade keys belong to, as raw UUID bytes.
    let order_id: [u8; 16] = [
        0x9f, 0x2c, 0x0e, 0x7a, 0x4b, 0x18, 0x4d, 0x3e, 0x9a, 0x51, 0x6d, 0x2c, 0x83, 0x74, 0xe1,
        0x05,
    ];

    // Both sides derive the same conversation and signing keys.
    let (alice_conv, alice_sign) = derive_chat_keys(&alice_trade, &bob_trade.public_key())?;
    let (bob_conv, bob_sign) = derive_chat_keys(&bob_trade, &alice_trade.public_key())?;
    assert_eq!(alice_conv.public_key(), bob_conv.public_key());
    assert_eq!(alice_sign.public_key(), bob_sign.public_key());

    println!("Conversation pubkey (p tag): {}", alice_conv.public_key());
    println!("Signing pubkey (author):     {}", alice_sign.public_key());

    // Alice sends a message.
    let message = "Let's reestablish the peer-to-peer nature of Bitcoin!";
    let outer = mostro_wrap(&alice_trade, &alice_conv, &alice_sign, message, vec![]).await?;

    // Bob subscribes with `authors = [pub(K_sign)]` and validates what arrives.
    // Only the two trade keys of this order are accepted as inner signers.
    let allowed = [alice_trade.public_key(), bob_trade.public_key()];
    let inner = mostro_unwrap(
        &bob_conv,
        &bob_sign.public_key(),
        &allowed,
        &outer,
        Timestamp::now(),
    )?;
    assert_eq!(inner.pubkey, alice_trade.public_key());

    // ---------------------------------------------------------------- //
    // Dispute disclosure                                                //
    // ---------------------------------------------------------------- //

    // Alice hands the solver a proof instead of a bare key. The proof is
    // deterministic: the same inputs always yield the same bytes.
    let disclosure = prove_channel(&secp, &alice_trade, &bob_trade.public_key(), &order_id)?;
    assert_eq!(
        (disclosure.e, disclosure.s),
        {
            let again = prove_channel(&secp, &alice_trade, &bob_trade.public_key(), &order_id)?;
            (again.e, again.s)
        },
        "the nonce must be deterministic"
    );

    println!(
        "Shared point:                {}",
        hex(&disclosure.shared_point.serialize())
    );
    println!("Proof e:                     {}", hex(&disclosure.e));
    println!("Proof s:                     {}", hex(&disclosure.s));

    // The solver takes both trade pubkeys from the dispute the daemon
    // published — never from the party making the disclosure.
    let shared = verify_channel(
        &secp,
        &alice_trade.public_key(),
        &bob_trade.public_key(),
        &order_id,
        &disclosure,
    )
    .ok_or("channel proof rejected")?;

    // The proof alone reconstructs the channel: no key was transmitted.
    let (solver_conv, solver_sign) = chat_keys_from_shared(&shared)?;
    assert_eq!(solver_conv.public_key(), alice_conv.public_key());
    assert_eq!(solver_sign.public_key(), alice_sign.public_key());

    // ...and the solver reads the transcript under exactly the rules a party
    // applies, with the order's trade keys as the only accepted signers.
    let seen = mostro_unwrap(
        &solver_conv,
        &solver_sign.public_key(),
        &allowed,
        &outer,
        Timestamp::now(),
    )?;
    assert_eq!(seen.content, message);
    println!("Solver reconstructed the channel and read the transcript.");

    // Alice cannot point the solver at a channel she runs with a sockpuppet:
    // proving it would require the discrete log of Bob's trade key.
    let carol_trade = Keys::generate();
    let sockpuppet = prove_channel(&secp, &alice_trade, &carol_trade.public_key(), &order_id)?;
    assert!(verify_channel(
        &secp,
        &alice_trade.public_key(),
        &bob_trade.public_key(),
        &order_id,
        &sockpuppet,
    )
    .is_none());

    // A proof is bound to the role of its author and to the order.
    assert!(verify_channel(
        &secp,
        &bob_trade.public_key(),
        &alice_trade.public_key(),
        &order_id,
        &disclosure,
    )
    .is_none());
    let mut another_order = order_id;
    another_order[0] ^= 1;
    assert!(verify_channel(
        &secp,
        &alice_trade.public_key(),
        &bob_trade.public_key(),
        &another_order,
        &disclosure,
    )
    .is_none());

    // Swapping the point for one the prover controls invalidates the proof.
    let mut tampered = disclosure.clone();
    tampered.shared_point = sockpuppet.shared_point;
    assert!(verify_channel(
        &secp,
        &alice_trade.public_key(),
        &bob_trade.public_key(),
        &order_id,
        &tampered,
    )
    .is_none());

    // Either party can prove the same channel, independently.
    let from_bob = prove_channel(&secp, &bob_trade, &alice_trade.public_key(), &order_id)?;
    assert_eq!(from_bob.shared_point, disclosure.shared_point);
    assert!(verify_channel(
        &secp,
        &bob_trade.public_key(),
        &alice_trade.public_key(),
        &order_id,
        &from_bob,
    )
    .is_some());

    println!("Bob's proof e:               {}", hex(&from_bob.e));
    println!("Bob's proof s:               {}", hex(&from_bob.s));
    println!("Sockpuppet channel, role swap, wrong order and swapped point: all rejected.");
    Ok(())
}

/// Derives the domain-separated conversation and signing keys for one order.
///
/// Both parties reach the same pair: the ECDH secret is symmetric, and HKDF is
/// deterministic.
///
/// # Arguments
/// - `own_trade`: this party's trade keys for the order.
/// - `peer_trade`: the counterparty's trade pubkey for the order.
///
/// # Returns
/// `(K_conv, K_sign)`.
pub fn derive_chat_keys(
    own_trade: &Keys,
    peer_trade: &PublicKey,
) -> Result<(Keys, Keys), Box<dyn std::error::Error>> {
    let shared = generate_shared_key(own_trade.secret_key(), peer_trade)?;
    chat_keys_from_shared(&shared)
}

/// Derives `(K_conv, K_sign)` from an ECDH secret that is already known.
///
/// A party reaches that secret with [`derive_chat_keys`]; a solver reaches the
/// same value by verifying a [`ChannelProof`], which is why this step is split
/// out.
pub fn chat_keys_from_shared(shared: &[u8; 32]) -> Result<(Keys, Keys), Box<dyn std::error::Error>> {
    let hkdf = Hkdf::<Sha256>::new(None, shared);

    let derive = |info: &[u8]| -> Result<Keys, Box<dyn std::error::Error>> {
        // Retry with a counter byte on the negligible chance that the output
        // is not a valid secp256k1 secret key.
        for counter in 0u16..=255 {
            let mut labelled = info.to_vec();
            if counter > 0 {
                labelled.push(counter as u8);
            }
            let mut out = [0u8; 32];
            hkdf.expand(&labelled, &mut out)
                .map_err(|e| format!("HKDF expand failed: {e}"))?;
            if let Ok(sk) = SecretKey::from_slice(&out) {
                return Ok(Keys::new(sk));
            }
        }
        Err("HKDF failed to produce a valid secret key".into())
    };

    Ok((derive(CONV_INFO)?, derive(SIGN_INFO)?))
}

/// Builds the outer kind 14 event carrying an encrypted, trade-key-signed
/// kind 1 event.
///
/// The inner event authenticates the sender; the outer event authenticates the
/// conversation and is what clients filter on.
///
/// # Arguments
/// - `sender_trade`: the sender's trade keys, used to sign the inner event.
/// - `conv`: `K_conv`, used to encrypt and as the `p` tag.
/// - `sign`: `K_sign`, used to sign the outer event.
/// - `message`: the plaintext message.
/// - `extra_tags`: additional tags for the outer event.
pub async fn mostro_wrap(
    sender_trade: &Keys,
    conv: &Keys,
    sign: &Keys,
    message: &str,
    extra_tags: Vec<Tag>,
) -> Result<Event, Box<dyn std::error::Error>> {
    // One timestamp for both events: the real moment the message is sent.
    // Recipients reject a mismatch, which is what defeats replays.
    let now = Timestamp::now();

    let inner = EventBuilder::text_note(message)
        .custom_created_at(now)
        .build(sender_trade.public_key())
        .sign(sender_trade)
        .await?;

    // NIP-44 self-encryption: K_conv is both sides of the key exchange.
    let content = nip44::encrypt(
        conv.secret_key(),
        &conv.public_key(),
        inner.as_json(),
        nip44::Version::V2,
    )?;

    // Exactly one `p` tag, ours. A caller-supplied one could hide the message
    // from the `#p` query a dispute solver uses to rebuild the transcript.
    if extra_tags.iter().any(|t| t.kind() == TagKind::p()) {
        return Err("extra_tags must not contain a p tag".into());
    }
    let mut tags = vec![Tag::public_key(conv.public_key())];
    tags.extend(extra_tags);

    let outer = EventBuilder::new(Kind::PrivateDirectMessage, content)
        .tags(tags)
        .custom_created_at(now)
        .sign_with_keys(sign)?;

    Ok(outer)
}

/// Validates an incoming outer event and returns the inner event.
///
/// Every check here is mandatory; see "Client security requirements".
///
/// Two of the thirteen steps are the caller's, because they need state this
/// function does not own: the **rate-limit budget** and the **event-id
/// caches** (a bounded LRU on the outer id, and durable storage for the inner
/// id). A caller that skips the durable inner-id check accepts replays.
///
/// The caller is also expected to have applied a byte limit when the raw event
/// was read off the wire; the payload bound re-checked here only caps the
/// decryption work.
///
/// # Arguments
/// - `conv`: `K_conv`, used to decrypt.
/// - `sign_pubkey`: `pub(K_sign)` of this conversation.
/// - `allowed_signers`: the buyer's and the seller's trade pubkeys.
/// - `outer`: the received kind 14 event.
/// - `now`: the recipient's current time, for the absolute timestamp bound.
pub fn mostro_unwrap(
    conv: &Keys,
    sign_pubkey: &PublicKey,
    allowed_signers: &[PublicKey],
    outer: &Event,
    now: Timestamp,
) -> Result<Event, Box<dyn std::error::Error>> {
    // A third party cannot produce a valid signature for this author, so this
    // check is what makes flooding impossible. Relays enforce it too, via the
    // `authors` filter; clients re-check it locally.
    if outer.pubkey != *sign_pubkey {
        return Err("outer event is not authored by the conversation signing key".into());
    }
    if outer.kind != Kind::PrivateDirectMessage {
        return Err("outer event is not kind 14".into());
    }

    // Exactly one `p` tag, addressing this conversation. Anything else could be
    // a message engineered to stay out of a dispute solver's `#p` query.
    let mut p_tags = outer.tags.iter().filter(|t| t.kind() == TagKind::p());
    match (p_tags.next().and_then(|t| t.content()), p_tags.next()) {
        (Some(pk), None) if pk == conv.public_key().to_hex() => {}
        _ => return Err("outer event must carry exactly one p tag for this conversation".into()),
    }

    // Absolute bound against our own clock. Without it a counterparty can date
    // both events far in the future — they agree with each other, so the
    // relative check below passes — and poison the `since` cursor, silencing
    // the conversation until that date. The past is unbounded: catching up
    // after being offline is legitimate.
    if outer.created_at.as_secs() > now.as_secs().saturating_add(MAX_CLOCK_SKEW_SECS) {
        return Err("outer event is dated too far in the future".into());
    }

    if outer.content.len() > MAX_CONTENT_BYTES {
        return Err("encrypted payload exceeds the accepted size".into());
    }

    outer.verify()?;

    let decrypted = nip44::decrypt(conv.secret_key(), &conv.public_key(), &outer.content)?;
    let inner = Event::from_json(&decrypted)?;

    // The only authentication of who wrote the message: both parties can sign
    // the outer event, so it cannot tell the two sides apart.
    inner.verify()?;
    if !allowed_signers.contains(&inner.pubkey) {
        return Err("inner event is signed by a key that is not a party to this order".into());
    }
    if inner.kind != Kind::TextNote {
        return Err("inner event is not kind 1".into());
    }

    // Bounds how far back the caller's durable inner-id dedup has to reach: a
    // re-wrap older than the tolerance is stale and rejected here, while one
    // inside the window is caught by that dedup, never by this check.
    let skew = inner
        .created_at
        .as_secs()
        .abs_diff(outer.created_at.as_secs());
    if skew > MAX_CLOCK_SKEW_SECS {
        return Err("inner and outer timestamps disagree — stale re-wrap".into());
    }

    Ok(inner)
}

/// A party's proof that a conversation belongs to a given order.
///
/// `shared_point` is the ECDH point of the two trade keys; `(e, s)` is the
/// Chaum-Pedersen transcript that ties it to them. Everything here is
/// secret-bearing — `x(shared_point)` **is** the ECDH secret — so a disclosure
/// travels inside the dispute chat, never in the clear.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelProof {
    /// Compressed secp256k1 point, 33 bytes.
    pub shared_point: CurvePoint,
    /// Fiat-Shamir challenge, 32 bytes.
    pub e: [u8; 32],
    /// Response scalar, 32 bytes.
    pub s: [u8; 32],
}

/// BIP-340 style tagged hash: `SHA256(SHA256(tag) || SHA256(tag) || parts)`.
fn tagged_hash(tag: &[u8], parts: &[&[u8]]) -> [u8; 32] {
    let tag_hash = Sha256::digest(tag);
    let mut h = Sha256::new();
    h.update(tag_hash);
    h.update(tag_hash);
    for part in parts {
        h.update(part);
    }
    h.finalize().into()
}

/// The scalar whose point is the even-Y lift of this key's x-only pubkey.
///
/// Nostr pubkeys are x-only, so the verifier can only reconstruct the even-Y
/// point. The prover has to work with the matching scalar, which is the
/// negation of the secret key when its own point is odd-Y. This does not
/// change the ECDH secret: negating the scalar negates the product, and
/// negation leaves the x-coordinate untouched.
fn even_scalar(secp: &Secp256k1<All>, sk: &CurveScalar) -> CurveScalar {
    match sk.x_only_public_key(secp).1 {
        Parity::Odd => sk.negate(),
        Parity::Even => *sk,
    }
}

/// Even-Y lift of an x-only nostr pubkey (BIP-340 `lift_x`).
fn lift_x(pk: &PublicKey) -> Result<CurvePoint, Box<dyn std::error::Error>> {
    Ok(pk.xonly()?.public_key(Parity::Even))
}

/// Fiat-Shamir challenge. Committing to the order id, to both trade keys in
/// role order, and to the point makes a proof unusable for another order, for
/// the other party, or for a substituted point.
fn challenge(
    order_id: &[u8; 16],
    own: &CurvePoint,
    peer: &CurvePoint,
    shared_point: &CurvePoint,
    r1: &CurvePoint,
    r2: &CurvePoint,
) -> [u8; 32] {
    tagged_hash(
        DLEQ_TAG,
        &[
            order_id,
            &own.serialize(),
            &peer.serialize(),
            &shared_point.serialize(),
            &r1.serialize(),
            &r2.serialize(),
        ],
    )
}

/// Derives the proof nonce deterministically from the secret and the statement.
///
/// A repeated nonce across two different challenges leaks the trade key, so it
/// is not left to an RNG. Determinism also makes proofs reproducible, which is
/// what lets the test vector below pin exact bytes.
fn proof_nonce(
    alpha: &CurveScalar,
    order_id: &[u8; 16],
    own: &CurvePoint,
    peer: &CurvePoint,
    shared_point: &CurvePoint,
) -> Result<CurveScalar, Box<dyn std::error::Error>> {
    for counter in 0u16..=255 {
        let extra = if counter == 0 {
            Vec::new()
        } else {
            vec![counter as u8]
        };
        let candidate = tagged_hash(
            DLEQ_NONCE_TAG,
            &[
                &alpha.secret_bytes(),
                order_id,
                &own.serialize(),
                &peer.serialize(),
                &shared_point.serialize(),
                &extra,
            ],
        );
        if let Ok(k) = CurveScalar::from_slice(&candidate) {
            return Ok(k);
        }
    }
    Err("nonce derivation failed".into())
}

/// Proves that the chat channel of `order_id` is the one derived from this
/// party's trade key and the counterparty's.
///
/// Produces a Chaum-Pedersen proof of knowledge of `alpha` such that
/// `own_even = alpha*G` and `shared_point = alpha*peer_even`. Only a holder of
/// one of the two trade secret keys can produce it.
///
/// # Arguments
/// - `own_trade`: this party's trade keys for the order.
/// - `peer_trade`: the counterparty's trade pubkey for the order.
/// - `order_id`: the order's UUID, as raw bytes.
pub fn prove_channel(
    secp: &Secp256k1<All>,
    own_trade: &Keys,
    peer_trade: &PublicKey,
    order_id: &[u8; 16],
) -> Result<ChannelProof, Box<dyn std::error::Error>> {
    let alpha = even_scalar(secp, own_trade.secret_key());
    let own_even = CurvePoint::from_secret_key(secp, &alpha);
    let peer_even = lift_x(peer_trade)?;

    // The point is forced: there is exactly one value of alpha*peer_even.
    let shared_point = peer_even.mul_tweak(secp, &Tweak::from(alpha))?;

    let k = proof_nonce(&alpha, order_id, &own_even, &peer_even, &shared_point)?;
    let r1 = CurvePoint::from_secret_key(secp, &k);
    let r2 = peer_even.mul_tweak(secp, &Tweak::from(k))?;

    let e = challenge(order_id, &own_even, &peer_even, &shared_point, &r1, &r2);
    // s = k + e*alpha  (mod n)
    let e_alpha = alpha.mul_tweak(&Tweak::from_be_bytes(e)?)?;
    let s = k.add_tweak(&Tweak::from(e_alpha))?;

    Ok(ChannelProof {
        shared_point,
        e,
        s: s.secret_bytes(),
    })
}

/// Verifies a [`ChannelProof`] and returns the ECDH secret it establishes.
///
/// `own_trade` and `peer_trade` MUST come from the dispute the daemon
/// published, never from the party making the disclosure — supplying them is
/// the whole point of the check.
///
/// Returns `None` on any failure. The 32 bytes returned on success are the
/// input to [`chat_keys_from_shared`].
pub fn verify_channel(
    secp: &Secp256k1<All>,
    own_trade: &PublicKey,
    peer_trade: &PublicKey,
    order_id: &[u8; 16],
    proof: &ChannelProof,
) -> Option<[u8; 32]> {
    let own_even = lift_x(own_trade).ok()?;
    let peer_even = lift_x(peer_trade).ok()?;
    let e = Tweak::from_be_bytes(proof.e).ok()?;
    // Rejects s = 0 and s >= n, so the scalar is always in range.
    let s = CurveScalar::from_slice(&proof.s).ok()?;
    let shared_point = &proof.shared_point;

    // R1 = s*G - e*own_even ; `combine` fails on the point at infinity, which
    // is exactly the degenerate case to reject.
    let r1 = CurvePoint::from_secret_key(secp, &s)
        .combine(&own_even.mul_tweak(secp, &e).ok()?.negate(secp))
        .ok()?;
    // R2 = s*peer_even - e*shared_point
    let r2 = peer_even
        .mul_tweak(secp, &Tweak::from(s))
        .ok()?
        .combine(&shared_point.mul_tweak(secp, &e).ok()?.negate(secp))
        .ok()?;

    if challenge(order_id, &own_even, &peer_even, shared_point, &r1, &r2) != proof.e {
        return None;
    }

    // The ECDH secret is the x-coordinate, which is what both parties already
    // feed to HKDF.
    let mut shared = [0u8; 32];
    shared.copy_from_slice(&shared_point.serialize()[1..]);
    Some(shared)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
```

### Test vector

Derived from the trade keys used above:

```
Alice trade private key: 548f68890c49fa42f104c60352395e60ff030b0b407e955f1eed1400d6c0347a
Alice trade public key:  000053c3b4773182e7c4c1b72b272d34be01bf4414a6a25c998977c516a46a01
Bob trade private key:   f258e73f07386d37133718b6127f873dd7c391b8f43b331ff8254034a13d2943
Bob trade public key:    000009ae5cff9f6ba9b05159ec5ed58c187f5882ea77c81ed5dd19163272a5d7

ECDH shared secret:      def6633a53d07d1e829484c4d4bdbbeed2f4b14c21743e63871c174338e39475

pub(K_conv), the p tag:  bceb1cd2a8e98ee9729122a1693edcc39c3ace04582ff96a26705c5e4078a6f2
pub(K_sign), the author: 1dba04571059183f76b148119cfa6f8004dad30cb4e810180a6df17386a7f0b4
```

Both parties derive that pair from their own side of the ECDH, and these values are what the example above prints. NIP-44 v2 uses a random nonce, so ciphertexts are not reproducible: check the **derived pubkeys** against this vector first, then verify a round trip through `mostro_wrap` / `mostro_unwrap`.

### Channel proof test vector

For the same trade keys and the order `9f2c0e7a-4b18-4d3e-9a51-6d2c8374e105`, whose 16 raw UUID bytes are `9f2c0e7a4b184d3e9a516d2c8374e105`:

```
shared_point:            02def6633a53d07d1e829484c4d4bdbbeed2f4b14c21743e63871c174338e39475

Alice's proof (own = Alice, peer = Bob)
  e:                     f634a28af956b8e14ad166ea7340884ee517c725bc5db37c2e0bf598e3e4f48a
  s:                     c8dbf0716a2839a814241a7790767b3a019170f1bf360a5fbcabd1491daba9af

Bob's proof (own = Bob, peer = Alice)
  e:                     3f6ca73af08b9603008021eaf3ee4b854d1c3b020a0635e0c9da265cee825021
  s:                     16a11149135acd3d694713b86c108ed0f6d9b4d54c317ae88267ed6f6e73cea1
```

Three things to check against this vector. The x-coordinate of `shared_point` is the ECDH shared secret above, so a `verify` that returns anything else has gone wrong before the HKDF. The two proofs differ while `shared_point` does not, which is the mirrored statement reaching the same point. And because the nonce is deterministic, `(e, s)` must reproduce byte for byte on every run — an implementation whose values move between runs is deriving the nonce from an RNG and needs fixing.

## Reference implementation

[mostro-chat](https://github.com/MostroP2P/mostro-chat) is a terminal (TUI) chat client that implements this specification end to end and serves as the reference implementation: shared-key derivation with domain separation, the kind 14 outer / kind 1 inner event structure, NIP-44 self-encryption under `K_conv`, and the full validation order described in [Client security requirements](#client-security-requirements). Its code is kept up to date with this document — when in doubt about how the P2P chat should behave, consult that repository alongside the [test vector](#test-vector).
