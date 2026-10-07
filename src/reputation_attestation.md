# Reputation attestation

Kind `38388` is reserved for **reputation attestations**: a statement, signed by an
*issuer*, of the reputation a user earned there, addressed to one identity on a Mostro
instance that will import it. The issuer is a Mostro instance or lnp2pBot; the
instance that imports it is the *destination*.

An attestation is **never published to relays**. It travels inside the
[`reputation-exported`](./reputation_transfer.md#export) reply and the
[`import-reputation`](./reputation_transfer.md#import) request, both encrypted direct
messages, or through the Telegram hand-off described below. Only the issuer, the user
and the destinations the user chooses ever see it. The number sits in the addressable
range only to keep the Mostro kinds together; since nothing publishes it, it carries no
`d` tag.

Importing is opt-in, and it links the two accounts: the issuer learns the destination
identity it signs for, the destination learns the source account the attestation names,
and anyone who knows the user's figures on the source can recognise them, merged, on the
destination. A user who does not want that simply does not import.

## Why a Nostr event

Every implementation already has the canonical serialisation of a Nostr event, its id
and BIP-340 verification. Using an event means nothing new has to be specified byte by
byte, and the kind gives the signature its own domain: an issuer key that signs anything
else can never have it accepted as an attestation.

## The event

```json
{
  "id": "<Event id>",
  "pubkey": "<Issuer key>",
  "created_at": 1790899200,
  "kind": 38388,
  "tags": [
    ["p", "<Destination identity pubkey>"],
    ["subject", "<Source account id>"],
    ["reviews", "214"],
    ["rating", "4.87"],
    ["since", "1696204800"],
    ["expiration", "1791504000"],
    ["z", "reputation-attestation"]
  ],
  "content": "",
  "sig": "<Issuer's signature>"
}
```

### Tags

Each of these tags MUST appear exactly once. A receiver ignores any other tag and the
`content`, which an issuer leaves empty.

| Tag | Rule |
|---|---|
| `p` | The destination identity pubkey: 64 lowercase hex characters (32-byte x-only key) |
| `subject` | Stable, opaque id of the source account **within this issuer**: 1 to 64 characters from `[0-9A-Za-z_-]`. A Mostro issuer uses the user's identity pubkey in lowercase hex. lnp2pBot uses its internal user id, never the Telegram id: the destination needs a deduplication key, not the Telegram account |
| `reviews` | Ratings received on the source, native only: a decimal integer, `5 ≤ reviews ≤ 4294967295` |
| `rating` | Average of those ratings: one digit, a dot and exactly two digits, between `1.00` and `5.00` inclusive |
| `since` | Day of the first completed trade on the source: Unix seconds, a multiple of `86400`, `≥ 1577836800` (2020-01-01) and `≤ created_at` |
| `expiration` | [NIP-40](https://github.com/nostr-protocol/nips/blob/master/40.md) Unix seconds, greater than `created_at`. Issuers set `created_at` plus 7 days |
| `z` | `reputation-attestation` |

Decimal integers are written in base 10 without sign, leading zeros or surrounding
whitespace.

### The figures

The three figures are the user's own reputation on the source. A Mostro issuer leaves
out everything the user imported there, so an import never travels twice:

| Figure | Mostro issuer | lnp2pBot issuer |
|---|---|---|
| `reviews` | ratings received natively (`total_reviews - seeded_reviews`) | `total_reviews` |
| `rating` | the native average (`native_rating_sum / native_reviews`) | `total_rating` |
| `since` | earliest `success` order with the identity as master buyer or seller, day-truncated | earliest completed order of the user, day-truncated |

`reviews` counts **ratings received**, not completed trades: on Mostro it is also the
weight of the running average. `since` is the **first completed trade**, not when the
account was created, so a dormant account cannot carry an age it never traded. It is
truncated to the start of its UTC day (`t - t % 86400`), the same precision as the public
[`since`](./user_rating.md) of the rating event.

The issuer computes `rating` from its internal average `avg` as

```text
rating = clamp(round(avg × 100), 100, 500) / 100
```

in IEEE 754 double precision, where `round` rounds half away from zero, and writes it with
exactly two decimals. Fixing the arithmetic makes every implementation agree even where
the decimal value is not representable: `4.895 × 100` is `489.49999999999994` in double
precision, so `4.895` gives `4.89`, not `4.90`. The clamp covers legacy Mostro rows, whose
stored average is damped by the first vote and can sit below `1`.

### Eligibility

An issuer refuses to attest an account that is banned, or that has fewer than **10
completed trades** or fewer than **5 ratings received**, both counted on its native
history only. Completed trades decide eligibility and are not carried.

### Issuer key

The issuer signs with a key **dedicated** to issuance: lnp2pBot's
`REPUTATION_ISSUER_SK`, kept apart from its `NOSTR_SK`, and on a Mostro a separate key,
never the daemon key. Either can then be rotated without touching the other, and the
daemon key never signs an attestation next to its public events. A Mostro that issues
attestations names its current issuer key in its
[info event](./other_events.md#reputation-tags).

## Issuance

An issuer answers one request with one attestation; there is no session.

1. **Authenticate the source account.** The issuer attests only the account that asks:
   - A Mostro issuer requires the identity the transport proved (on protocol v2, the
     [identity proof](./key_management.md#identity-proof) in the content tuple), and
     sets `subject` to that identity. It never reads the source account from the payload,
     and refuses a request without an identity proof: a `full_privacy` user or a bare
     trade key has no identity-bound reputation to export.
   - lnp2pBot takes the source account from the Telegram user who sent the command,
     never from the link.
2. **Check eligibility** (above).
3. **Confirm a first binding.** If the source account is not bound to any identity yet,
   the user confirms the destination identity, shown as an `npub`, before anything is
   recorded. A link opened by mistake, or crafted by someone else, binds nothing without
   that confirmation. On a Mostro issuer the request is signed by the source identity
   itself, so the client asks for the confirmation before sending it.
4. **Bind.** The issuer records the destination identity against the source account
   with an atomic compare-and-set that succeeds only if no identity is bound yet or the
   same one already is. Two concurrent requests for different identities therefore cannot
   both bind. A later request for the bound identity is how a user gets a lost, expired
   or fresher attestation; a request for any other identity is refused unless it carries a
   rebind authorisation.
5. **Sign** the attestation with the issuer key and return it.
6. The client verifies the attestation as a destination would (below) and shows the user
   the figures before importing them.

### Rebinding

A binding moves only with the consent of the identity that holds it, or by an operator.

A **rebind authorisation** is an event of kind `38388` signed by the *currently bound*
identity, sent along with the request for the new one:

```json
{
  "id": "<Event id>",
  "pubkey": "<Currently bound identity pubkey>",
  "created_at": 1790899200,
  "kind": 38388,
  "tags": [
    ["p", "<New identity pubkey>"],
    ["issuer", "<Issuer key>"],
    ["expiration", "1790902800"],
    ["z", "reputation-rebind"]
  ],
  "content": "",
  "sig": "<Signature of the bound identity>"
}
```

Each of these tags MUST appear exactly once; `p` and `issuer` are 64 lowercase hex
characters, and `expiration` is greater than `created_at` and at most
`created_at + 3600`. The `z` tag keeps an authorisation and an attestation from ever
being taken for each other, although they share the kind.

The issuer checks the signature, that `pubkey` equals the bound identity, that `issuer`
is its own issuer key and that `p` equals the destination identity of the request it
came with: the bound identity consents to one new identity, so the request cannot name
another. It then checks the times with the same 300-second clock
skew as [redemption](#redemption) step 4: `created_at` is not in the future and
`expiration` has not passed. Only then does it move the binding with the same
compare-and-set conditioned on the old identity, and attest the new one. The `issuer` tag
stops an authorisation from being replayed at another issuer; the compare-and-set makes a
replay at the same issuer a no-op. The future-date check keeps the one-hour cap
meaningful: without it, an authorisation dated years ahead would stay usable until then.

A user who lost the bound identity cannot sign, so an operator MAY rebind by hand. Every
such rebind is logged with the old and the new identity and the reason.

A rebind revokes nothing already issued. The old identity keeps what it imported, and an
attestation issued to it stays redeemable until it expires. Across instances, a rebound
account can therefore end up backing the old identity on some and the new one on others;
on any one instance it backs only one, because the destination never imports the same
source account twice (step 7 below).

### Telegram hand-off

lnp2pBot cannot receive a Mostro message, so the request is a Telegram deep link,
`https://t.me/lnp2pbot?start=rep_<identity>`, where `<identity>` is the destination
identity as 43 characters of unpadded base64url: Telegram caps the start parameter at 64
characters, which a hex key with a prefix would exceed. The bot shows the `npub` with a
confirm button (step 3), then answers with a button that opens the client's deep link
with the attestation; on desktop and web the user pastes it into the client instead. A
rebind authorisation does not fit in the start parameter, so the client shows it for the
user to paste into the bot chat, which answers with the same confirmation.

Telegram sees everything in that exchange: the destination identity, the source account
and the figures.

## Redemption

A destination that receives an attestation in an
[`import-reputation`](./reputation_transfer.md#import) request runs these
checks in order, and refuses the import at the first one that fails:

1. The event parses, its `id` is the hash of its canonical serialisation and its
   signature is valid.
2. `kind` is `38388`, the `z` tag is `reputation-attestation`, and every tag rule above
   holds.
3. `pubkey` is one of the keys of an entry of the destination's **trust list**, and is not
   the destination's own issuer key: an instance trusting itself would let a user import
   their reputation on the same instance and double it. From here on, the issuer is the
   entry's **name**, not the key (see below).
4. With a clock skew of 300 seconds: `created_at` is not in the future, `expiration` has
   not passed, and `expiration - created_at` does not exceed the destination's maximum
   lifetime (7 days by default). The cap keeps a faulty or compromised issuer from minting
   attestations that live for years.
5. The request carries an identity proof, and the `p` tag equals the identity it proves —
   never a pubkey the sender merely claims. There is no fallback to the trade key.
6. The import is applied to the user record of that identity and no other.
7. Neither `(issuer, subject)` nor `(issuer, identity)` has been imported before on this
   instance. The first stops one source account from backing two identities here, even
   after a rebind or if an issuer broke its own binding; the second stops an identity from
   importing two accounts from one issuer. Both MUST be enforced by unique constraints, so
   two concurrent imports cannot both pass.
8. The import is recorded and applied in a single transaction.

### Trust list and key rotation

A destination identifies an issuer by a local, stable **name** with one or more keys,
never by a key alone. The name is what step 7 deduplicates on. Were it the key, an issuer
rotating its key would look like a new issuer, and every account imported under the old
key could import again under the new one.

A key belongs to one entry only, and the destination **persists which name every key
belongs to**, independently of its imports and of its current configuration. The first
time it loads a configured key, it records the key with the name of its entry; it keeps
that record after the key leaves the trust list, and refuses to start if a configured
key is recorded under a different name. Renaming an entry or moving a key therefore
cannot reset deduplication, and neither can doing either after a rotation: a key added
to an entry is recorded under that entry's name before it signs anything, so an entry
rotated to a new key, stripped of the old one and then renamed is refused at startup,
although the new key has no imports. Deriving the check from imports alone would miss
exactly that case. An entry whose name and keys are all new is a different issuer to the
destination; trusting an issuer under a fresh name and fresh keys is a deliberate
operator decision to restart its deduplication, never a side effect of editing an
existing entry.

- **Planned rotation.** The operator adds the issuer's new key to its entry, which
  records it under the entry's name, and removes the old one once every attestation it
  signed has expired.
- **Compromise.** The operator removes the key at once. Removing a key stops new imports
  but does not undo earlier ones; to undo them, the operator selects the imports made
  with that key by the destination's own import time — never by the attestation's
  `created_at`, which the key holder chooses — reverses them and deletes their records, so
  their users can import again with an attestation from the new key.

## Merging on the destination

An import is added to the user's reputation on the destination, never written over it:

- **Ratings received add up**: `total_reviews += reviews`.
- **The average is weighted** by the review count of each origin:
  `total_rating = (total_rating × total_reviews + rating × reviews) / (total_reviews + reviews)`,
  with the values from before the import.
- **The age is the earliest date**, never a sum: `since = min(since, since_imported)`.
  Time passes in parallel on every venue, so adding day counts would count the same
  month twice.

The first import into an identity with no reputation on the destination — no rating
received and no completed trade yet, which is the common case for a user who moves to a
new instance — is the base case of these rules. With no rating received
(`total_reviews = 0`), the import sets `total_reviews = reviews` and
`total_rating = rating`. With no `since` on the destination, the import sets
`since = since_imported`. A destination with no record of the identity at all creates
one. An implementation that stores an absent value as null, or as a sentinel, never feeds
it into the formulas above: the weighted average of a null is null, and the minimum of a
sentinel is not a date.

The destination publishes the merged figures in its ordinary
[rating event](./user_rating.md) and order [`rating` tag](./order_event.md), with no
marker telling imported reputation from native. It keeps what it imported apart
internally, so that it never re-exports an import as native reputation and can reverse an
import exactly.
