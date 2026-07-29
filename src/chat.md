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
| `K_conv` | NIP-44 encryption and decryption of the payload. `pub(K_conv)` is the conversation address carried in the `p` tag. | Both parties. Disclosed to a solver during a dispute. |
| `K_sign` | Signs the outer event. `pub(K_sign)` is the author every client filters on. | Both parties only. **Never disclosed.** |

Separating the two is what makes a **read-only** dispute disclosure possible: a solver given `K_conv` can read the entire conversation but cannot publish into it. See [Dispute disclosure](#dispute-disclosure).

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

### Validation order

Each incoming event MUST be validated cheapest-check-first, so that an abusive peer cannot force expensive work:

1. **Author** is `pub(K_sign)` — otherwise discard.
2. **Size** is within the client's limit (64 KiB is a reasonable default) — otherwise discard.
3. **Outer event id** has not been seen before (bounded LRU) — otherwise discard.
4. **Rate-limit budget** for this conversation is available — otherwise discard.
5. **Outer signature** verifies.
6. Only now, **NIP-44 decrypt** with `K_conv`.
7. **Inner signature** verifies — this is the sender authentication and MUST NOT be skipped. Reading the inner `pubkey` field without verifying the signature accepts forged senders.
8. **Inner pubkey** is the buyer's or the seller's trade key for this order — otherwise discard. No other signer is accepted, including a dispute solver.
9. **Inner kind** is 1 — otherwise discard.
10. **Inner event id** has not been seen before — otherwise discard.
11. **Timestamps**: `|inner.created_at − outer.created_at|` is within tolerance (60 seconds is a reasonable default) — otherwise discard.

### Replay protection

Both parties hold `K_sign`, so either can re-publish a previously received inner event inside a fresh wrapper. The inner signature is genuine, so it verifies — a peer could reinject an old "I sent the fiat" message, or reshape the transcript a solver will read during a dispute.

Steps 10 and 11 together close this completely:

- Replaying the **inner** event inside a new wrapper fails the timestamp check, because the old inner `created_at` no longer matches the new outer one.
- Replaying the **whole outer event** verbatim fails the event-id check, because the id is unchanged.

Both checks are therefore mandatory, not advisory.

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

Either party may voluntarily disclose **`K_conv`** to the solver, who can then decrypt the whole conversation and verify every inner signature to establish who wrote what.

Because `K_sign` is never disclosed, this grant is **read-only**: the solver cannot publish into the conversation. The chat therefore keeps only the messages exchanged by the two parties, and a solver who needs to talk to them does so privately with each side, outside this channel. Even a solver who obtained `K_sign` could not impersonate a party, since forging a message would require a trade private key.

`pub(K_sign)` is public data — it is the author of every event — and MAY be given to the solver alongside `K_conv` so they can filter efficiently by author. It is a locator, not a secret.

## Relay considerations

Kind 14 falls outside the range NIP-01 defines as regular (`1000 ≤ n < 10000`), and [NIP-17](https://github.com/nostr-protocol/nips/blob/master/17.md) specifies kind 14 as an unsigned rumor that is never published directly. Relay behaviour for a signed, published kind 14 is therefore not guaranteed by any NIP.

**Offline delivery depends on relays storing these events.** Operators and client implementers MUST verify empirically that the relays they target store and serve kind 14, and MUST NOT assume it.

Proof of work per [NIP-13](https://github.com/nostr-protocol/nips/blob/master/13.md) remains optional. It is markedly less relevant than it was under gift wrap: with a stable author, relays can apply their usual per-author rate limiting, which was impossible when every event carried a fresh ephemeral key.

## Migration

This is a breaking wire change affecting `mostrod`, the mobile clients, `mostro-cli`, and any solver tooling. Implementations SHOULD accept both the gift-wrapped form and the form specified here during a transition window, and MUST fix a deprecation date after which only this form is produced. Trades already in flight at the cutover keep the format they started with.

## Code Example

### Rust

```rust
// Leading `::` selects the `hkdf` crate: `nostr_sdk::prelude` also exports a
// module by that name, so a plain `use hkdf::Hkdf` is ambiguous.
use ::hkdf::Hkdf;
use nostr::util::generate_shared_key;
use nostr_sdk::prelude::*;
use sha2::Sha256;

/// HKDF `info` strings. Changing either value changes the wire format.
const CONV_INFO: &[u8] = b"mostro:chat:conv:v1";
const SIGN_INFO: &[u8] = b"mostro:chat:sign:v1";

/// Maximum accepted difference between the inner and outer `created_at`.
const MAX_CLOCK_SKEW_SECS: u64 = 60;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Per-order trade keys.
    let alice_trade =
        Keys::parse("548f68890c49fa42f104c60352395e60ff030b0b407e955f1eed1400d6c0347a")?;
    let bob_trade =
        Keys::parse("f258e73f07386d37133718b6127f873dd7c391b8f43b331ff8254034a13d2943")?;

    // Both sides derive the same conversation and signing keys.
    let (alice_conv, alice_sign) = derive_chat_keys(&alice_trade, &bob_trade.public_key())?;
    let (bob_conv, bob_sign) = derive_chat_keys(&bob_trade, &alice_trade.public_key())?;
    assert_eq!(alice_conv.public_key(), bob_conv.public_key());
    assert_eq!(alice_sign.public_key(), bob_sign.public_key());

    println!("Conversation pubkey (p tag): {}", alice_conv.public_key());
    println!("Signing pubkey (author):     {}", alice_sign.public_key());

    // Alice sends a message.
    let message = "Let’s reestablish the peer-to-peer nature of Bitcoin!";
    let outer = mostro_wrap(&alice_trade, &alice_conv, &alice_sign, message, vec![]).await?;
    println!("Outer event: {outer:#?}");

    // Bob subscribes with `authors = [pub(K_sign)]` and validates what arrives.
    // Only the two trade keys of this order are accepted as inner signers.
    let allowed = [alice_trade.public_key(), bob_trade.public_key()];
    let inner = mostro_unwrap(&bob_conv, &bob_sign.public_key(), &allowed, &outer)?;
    println!("Inner event: {inner:#?}");
    assert_eq!(inner.pubkey, alice_trade.public_key());

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
    let hkdf = Hkdf::<Sha256>::new(None, &shared);

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
/// Every check here is mandatory; see "Client security requirements". Rate
/// limiting and the event-id caches belong to the caller, which owns the
/// per-conversation state.
///
/// # Arguments
/// - `conv`: `K_conv`, used to decrypt.
/// - `sign_pubkey`: `pub(K_sign)` of this conversation.
/// - `allowed_signers`: the buyer's and the seller's trade pubkeys.
/// - `outer`: the received kind 14 event.
pub fn mostro_unwrap(
    conv: &Keys,
    sign_pubkey: &PublicKey,
    allowed_signers: &[PublicKey],
    outer: &Event,
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

    // Replay guard: a resent inner event keeps its original timestamp and no
    // longer matches the wrapper it arrives in.
    let skew = inner.created_at.as_secs().abs_diff(outer.created_at.as_secs());
    if skew > MAX_CLOCK_SKEW_SECS {
        return Err("inner and outer timestamps disagree — possible replay".into());
    }

    Ok(inner)
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

More details about this implementation can be found in this [repository](https://github.com/MostroP2P/mostro-chat).
