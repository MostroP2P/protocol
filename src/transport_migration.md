# Transport migration (v1 → v2)

Mostro is moving its wire transport from **protocol v1** (NIP-59 gift wrap,
kind `1059`) to **protocol v2** (NIP-44 direct message, kind `14`). This
page is the practical guide for **client developers**: what changes, how to
detect which transport a node speaks, and how to support both during the
transition.

The logical messages, key derivation, indexing and rotation rules are
unchanged — only the envelope differs. Even so, **v1 and v2 are
incompatible**: a v1 node never reads a kind-`14` event, a v2 node never
reads a gift wrap, and neither answers a message in the other format. The two formats are documented
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
matching wire format. Old daemons that predate the tag (mostrod before
v0.18.0) emit nothing; treat their absence as v1.

## v1 and v2 are incompatible

There is no negotiation, fallback or translation between the two
protocols, neither in the daemon nor on the wire. A client that sends v2
to a v1 node, or v1 to a v2 node, gets **no answer and no error**: the
node subscribes to the other event kind and never sees the message. The
`protocol_version` tag is therefore the only way to get it right, and a
client must read it before it talks to a node.

What the Mostro clients do with it:

| Client | v1 node | v2 node |
|--------|---------|---------|
| Mostro Mobile (app v1) ≥ v1.3.0 | works — reads the tag and uses gift wrap | works — reads the tag and uses kind `14` |
| Mostro app v2 | **not supported** — the app tells the user the node speaks a protocol it does not | works (kind `14` only) |
| mostro-cli, mostrix | works | works |

App v1 carries users across the transition: it keeps both wrap paths and
follows whatever the node announces, so the same install keeps working
when a node moves from v1 to v2. App v2 is v2-native and never implements
v1; users who need a v1 node have to use app v1 until that node upgrades.

A new client can make either choice. Supporting both is only worth it
while v1 nodes are still around; a v2-only client must treat a missing or
`"1"` tag as a node it cannot use and say so, rather than send messages
that will never be answered.

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
  is where spam concentrates. Once the node associates the trade key with an
  active order or dispute, its later messages are back to needing only `pow`.

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
   Read the info event before sending.

An **absent `pow_first_contact` tag means unknown, not zero and not `pow`.** Some
protocol-v2 daemons enforce a configured first-contact difficulty but predate the
tag, so assuming `pow` there is exactly the mistake that produces a silent drop.
When the tag is missing and the node speaks v2, mine at least `pow` and, if a
first contact goes unanswered, retry a freshly built event at a higher difficulty
(doubling the bits up to a cap you choose) before concluding the node is
unreachable — silence is the only feedback the gate gives. Nodes that publish the
tag need none of this guesswork, which is the reason to prefer them.

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
- **v0.18.5** — the daemon default becomes v2. A node still speaks v1 only
  if its operator sets `transport = "gift-wrap"` explicitly.
- **v0.19.0** — protocol v2 is the **only** protocol. mostrod removes every
  trace of v1: the gift-wrap path is gone, and a node configured with
  `transport = "gift-wrap"` does not start until the operator removes that
  line. Nodes keep publishing
  `["protocol_version", "2"]`. A later breaking release of `mostro-core`
  removes its gift-wrap code too; clients that still need to reach v1 nodes
  should stay on an earlier `mostro-core` until those nodes upgrade.

A node that upgrades from v1 to v0.19.0 keeps its open trades: orders and
trade keys do not depend on the transport, so a client that follows the
`protocol_version` tag (app v1, mostro-cli, mostrix) continues the trade on
kind `14`. Messages the node sent as gift wraps before the upgrade stay on
the relays; a client that restores trades from relay history needs its v1
reader for that part.

On v2, every message carries a NIP-40 `expiration` (the node's `dm_days`, 30
days by default), and relays that honor it delete older messages. A client
that rebuilds trades or disputes from relay history cannot recover messages
past that point.

## Future protocol versions

No new protocol version is planned. If one ever comes, it will follow the
same path as this migration: the node advertises it in `protocol_version`,
nodes switch one at a time, and a client picks per node. What a client must
do today so that such a change does not break it:

- **Treat an unknown `protocol_version` as unsupported.** If the node
  advertises a value the client does not implement, the client
  must not guess a transport. A guess fails silently: the node never
  answers a message in a format it does not speak. Tell the user that the
  node needs a newer client.
- **A missing tag is not unknown.** It comes from a daemon older than
  v0.18.0, which speaks v1.
- **Read the tag every time you connect to a node, and follow changes.**
  A node can move to a newer protocol with trades still open.

