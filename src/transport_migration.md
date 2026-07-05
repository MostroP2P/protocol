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
3. **Wrap/unwrap with the matching path.** `mostro-core` (since **0.13.0**)
   ships both — `wrap_message_with(transport, …)` / `unwrap_incoming(event, …)`
   dispatch on the transport (or event kind), so a client holding both
   paths needs only to pass the node's transport. Since **0.14.1** the
   gift-wrap variant is marked deprecated at the library level as well.
4. **Set `version: 2`** in the message on the v2 transport (`1` on v1).
   Note that the event **kind is authoritative**: the receiver picks the
   unwrap path from the event kind (`1059` vs `14`), and does not reject a
   message whose inner `version` value doesn't match the transport — the
   field is informative on the wire. Set it correctly anyway; future
   versions may validate it.
5. **On v2, build the 3-element content tuple** — message, trade signature
   (or `null`), identity proof `["<identity pubkey>", "<identity sig>"]` (or
   `null` for full-privacy mode). The identity proof is a signature over the
   domain-tagged payload `mostro-transport-v2-identity:<trade pubkey hex>:<message JSON>`;
   see [Keys management → Identity proof](./key_management.md#identity-proof).
6. **On v2, add a NIP-40 `expiration` tag** to outgoing events. Mostro fills
   a default (the node's `dm_days`, 30 days) on its own messages when none
   is supplied.

Full-privacy mode and reputation mode work the same way as in v1: omit the
identity key (proof and trade signature both `null`) for full privacy, or
include them to maintain reputation.

## Anti-spam gates on v2

The point of making the trade key the visible author is that a node can
filter garbage **before** paying the NIP-44 decrypt cost. Since v0.18.0
mostrod applies the following gates to every incoming kind-`14` event, in
this order:

1. **Base proof of work** — the event must meet the node's NIP-13
   difficulty, advertised as the `pow` tag on the
   [kind-`38385` info event](./other_events.md#mostro-instance-status).
2. **Replay guard** — an event `id` already seen within the last 60
   seconds is dropped.
3. **First-contact proof of work** — if the sender pubkey (the trade key)
   is not in the node's *active set*, the event must meet a separate,
   typically higher difficulty (`pow_first_contact`, falling back to the
   base `pow` when the operator doesn't set one). The active set contains
   the trade keys of every non-terminal order — orders under dispute
   included — plus the solver keys of active disputes, and is refreshed
   periodically (every 60 seconds by default), so keys engaged in a trade
   keep the cheap fast path for follow-up messages.
4. **Freshness window** — after decryption, the inner message timestamp
   must be at most 10 seconds old, or the message is dropped. (This check
   predates v2 and applies on both transports.)

**All gate rejections are silent.** The node drops the event without
sending any `cant-do` reply — answering unauthenticated spam would itself
be an amplification vector — so a client cannot rely on an error response
to detect that it was gated. If a first message gets no reply, mine more
proof of work and retry with a fresh timestamp.

> **Discovery caveat:** the info event currently advertises only the base
> `pow`; there is no tag for the first-contact difficulty. A brand-new
> trade key that mines only the advertised `pow` can be silently dropped
> by a node configured with a stricter `pow_first_contact`.

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
