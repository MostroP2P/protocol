# Transport migration (v1 → v2)

Mostro is moving its wire transport from **protocol v1** (NIP-59 gift wrap,
kind `1059`) to **protocol v2** (NIP-44 direct message, kind `14`). This
page is the practical guide for **client developers**: what changes, how to
detect which transport a node speaks, and how to support both during the
transition.

The logical messages, key derivation, indexing and rotation rules are
unchanged — only the envelope differs. The two formats are documented
side by side in [Keys management](./key_management.md) (the v2 wire format
is under *Protocol v2 — NIP-44 direct messages*) and the message tuples in
[Overview](./overview.md#the-content-array-v1-vs-v2).

## Why the change

Gift wraps give strong metadata privacy, but their outer event is signed by
a random throwaway key, so neither relays nor the daemon can tell legitimate
traffic from garbage without paying the full NIP-44 decrypt cost — a spam
flood ("Gift Wrap Apocalypse") cannot be rate-limited by sender. Protocol v2
makes the **trade key** the visible author of the event. Because trade keys
are already single-trade and rotated, exposing one leaks little, while
enabling relay-side rate limiting by sender and cheap daemon-side
pre-validation before decryption. See the threat model in
[issue #626](https://github.com/MostroP2P/mostro/issues/626).

## Capability discovery

A node speaks **exactly one** transport — there is no dual mode. It
advertises which in its [instance-info event](./other_events.md#mostro-instance-status)
(kind `38385`) via the `protocol_version` tag:

- `["protocol_version", "1"]` → gift wrap (kind `1059`)
- `["protocol_version", "2"]` → NIP-44 direct (kind `14`)

A client should read this tag **before** sending anything and use the
matching wire format. Old daemons that predate the tag emit nothing; treat
their absence as v1.

## What a client must change

1. **Read `protocol_version`** from the node's kind-`38385` event and
   branch on it.
2. **Subscribe to the right kind**: `1059` for v1, `14` for v2 (authored by
   the node, `#p`-tagged to your trade keys for node replies).
3. **Wrap/unwrap with the matching path.** `mostro-core` **0.13.0** ships
   both — `wrap_message_with(transport, …)` / `unwrap_incoming(event, …)`
   dispatch on the transport (or event kind), so a client holding both
   paths needs only to pass the node's transport.
4. **Set `version: 2`** in the message on the v2 transport (`1` on v1).
5. **On v2, build the 3-element content tuple** — message, trade signature
   (or `null`), identity proof `["<identity pubkey>", "<identity sig>"]` (or
   `null` for full-privacy mode). The identity proof is a signature over the
   domain-tagged payload `mostro-transport-v2-identity:<trade pubkey hex>:<message JSON>`;
   see [Keys management → Identity proof](./key_management.md#identity-proof).
6. **On v2, add a NIP-40 `expiration` tag** to outgoing events. Mostro fills
   a default (the node's `dm_days`, 30 days) on its own messages when none
   is supplied.
7. **Read the node's proof-of-work tags** and mine the outer event accordingly —
   a new order or take is charged at the higher `pow_first_contact` rate. See
   [Proof of work and the first-contact gate](#proof-of-work-and-the-first-contact-gate)
   below.

Full-privacy mode and reputation mode work the same way as in v1: omit the
identity key (proof and trade signature both `null`) for full privacy, or
include them to maintain reputation.

## Proof of work and the first-contact gate

Making the trade key the visible author is what lets a node filter before
decrypting, and proof of work ([NIP-13](https://github.com/nostr-protocol/nips/blob/master/13.md))
is how it charges for that first look. Both difficulties are published in the
[instance-info event](./other_events.md#mostro-instance-status) and are chosen
per instance — many run with `0` and require nothing.

- **`pow`** — required of every event the client sends, on either transport.
- **`pow_first_contact`** — required of an event whose visible sender is a trade
  key the node does not currently associate with an active order or dispute.
  In practice that is the first event of a trade: creating an order, or taking
  one. It is never lower than `pow` and is typically higher, because that lane
  is where spam concentrates. Once the node has matched a trade key to a live
  order, its later messages are back to needing only `pow`.

Two consequences for a client:

1. **Mine on the outer event.** The difficulty is counted in leading zero bits
   of the *event id* — the kind-`14` event's own id on v2, the gift wrap's id on
   v1 — not of the inner message. Grind the `nonce` tag
   (`["nonce", "<counter>", "<target bits>"]`) as NIP-13 describes; most Nostr
   libraries expose this as a "pow" option on the event builder.
2. **Under-powered events vanish.** The check happens before the node decrypts
   anything, so there is no `cant-do` message and no error of any kind — the
   event is simply dropped. A client that mines against `pow` when the node
   asked for `pow_first_contact` sees its order creation silently do nothing.
   Read the info event before sending, and treat an absent `pow_first_contact`
   tag (older daemons) as equal to `pow`.

On v2 a node also drops a re-sent **identical** event id for a short window as
replay defense, so a retry must be a freshly built event rather than a
rebroadcast of the same one. And independently of PoW, on both transports it
discards messages whose inner `created_at` is older than a short freshness
window (ten seconds in the current daemon) — so mine and publish promptly rather
than preparing events far in advance.

## Release timeline

- **v0.18.0** — protocol v2 ships. Default `transport = "gift-wrap"`, so
  nothing changes for existing clients. **Protocol v1 is DEPRECATED.**
  Client developers have the 0.18.x cycle to ship v2 support.
- **v0.19.0** — protocol v2 becomes the **default and only** protocol.
  mostrod removes the v1 path entirely. `mostro-core` keeps its gift-wrap
  helpers so clients can still migrate at their own pace, but nodes will no
  longer accept kind-`1059` traffic.

The recommendation is therefore: **keep both wrap paths now** and select per
node from `protocol_version`. A client that supports both will work against
every node throughout the transition, and against v2-only nodes after the
v0.19.0 cutover with no further change.
