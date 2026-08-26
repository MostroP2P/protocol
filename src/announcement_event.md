# Announcement events

Kind `38387` is reserved for **announcements**: a signed, one-way notice from a project
to the people already running its client. It carries things like "version 2.1 is out",
"the public node is down for maintenance tonight", or "this default relay is being
retired".

It is the only kind in the `3838x` block that is **not published by a Mostro daemon**.
A node neither publishes nor reads it. It is published by the keys of the project that
ships a client, and read by that client, which is why the number is registered here
rather than defined here: the registry is what stops two Mostro clients from picking
`38387` for two different things.

## Why a Nostr event

A client that speaks Nostr already has everything a broadcast channel needs. A Nostr
event is signed, so the publisher's key is the whole trust model — a hostile relay can
withhold an announcement or serve a stale one, but it cannot forge one. No server, no
account, no email address and no push token is required, which matters for clients that
deliberately have none of those.

## The event

An [addressable event](https://github.com/nostr-protocol/nips/blob/master/01.md#kinds),
so a correction is a republish under the same `d` rather than a second announcement:

```json
[
  "EVENT",
  "RAND",
  {
    "id": "<Event id>",
    "pubkey": "<Publisher's pubkey>",
    "created_at": 1756200000,
    "kind": 38387,
    "tags": [
      ["d", "2026-08-release-2-1"],
      ["expiration", "1758792000"],
      ["max_version", "2.1"],
      ["z", "announcement"],
      ["y", "mostro", "[Publishing project name]"]
    ],
    "content": "{\"v\":1,\"severity\":\"info\",\"locales\":{\"en\":{\"title\":\"Mostro 2.1 is out\",\"body\":\"It fixes the invoice timeout.\"},\"es\":{\"title\":\"Mostro 2.1 ya está disponible\",\"body\":\"Corrige el timeout de la factura.\"}},\"url\":\"https://mostro.network/\"}",
    "sig": "<Publisher's signature>"
  }
]
```

### Tags

| Tag | Required | Meaning |
|---|---|---|
| `d` | yes | announcement id — stable, opaque, unique per announcement and per publisher |
| `expiration` | yes | [NIP-40](https://github.com/nostr-protocol/nips/blob/master/40.md) unix seconds. **Required**, so a publisher cannot accidentally create something permanent |
| `min_version` | no | show only to client versions ≥ this — **inclusive** lower bound |
| `max_version` | no | show only to client versions < this — **exclusive** upper bound |
| `z` | yes | `announcement` |
| `y` | no | platform identifier, and optionally the publishing project's name |

`min_version` / `max_version` exist for the announcement that is *about* the client. "2.1
is out, it fixes X" must not reach someone already on 2.1, which is why the upper bound is
exclusive: the publisher writes the version the announcement is *about* and everyone below
it sees it. The lower bound is inclusive, for the mirror case: "2.1 changed how X works" is
for people who have 2.1.

Bounds are [semver](https://semver.org). Missing components are `0`, so `2.1` means
`2.1.0`. **Build metadata (`2.1.0+454`) is not accepted**: semver excludes it from
precedence, so honouring such a bound would mean silently ignoring part of what the
publisher wrote. A client that cannot parse a bound MUST treat the announcement as
invalid rather than as unbounded — a targeting instruction nobody can read has failed, and
showing the message to everyone is the wrong way to fail it.

### Content

```json
{
  "v": 1,
  "severity": "critical",
  "locales": {
    "en": { "title": "…", "body": "…" },
    "es": { "title": "…", "body": "…" }
  },
  "url": "https://example.com/…"
}
```

| Field | Required | Rule |
|---|---|---|
| `v` | yes | schema version, `1` today. A client MUST ignore an event whose `v` it does not know, rather than render it best-effort |
| `severity` | yes | one of `info`, `warning`, `critical` — see below |
| `locales` | yes | map of locale code → `{title, body}` |
| `locales[x].title` | yes | ≤ 80 characters after trimming |
| `locales[x].body` | yes | ≤ 500 characters after trimming |
| `url` | no | exactly one action link, `https` only |

Every translation rides in **one event**. One event per language, tagged `["l", "es"]`,
is more idiomatic Nostr and worse here: it turns one publish into several, lets a user's
relay set deliver two languages and not the others, and leaves the client deciding whether
three events are one announcement or three. One event carrying every translation cannot
half-arrive.

**Which locales are required is a policy of the publishing project, not of this
document** — it depends on what its client ships. A client SHOULD require every locale it
supports and reject an announcement missing one, rather than falling back to English: a
silent fallback is a bug the publisher never finds out about.

Both strings are **plain text**. A client MUST NOT parse markup in them or auto-detect
links inside `body`; `url` is the only thing that is ever actionable. The publisher is
trusted with authorship, not with rendering arbitrary content inside a Bitcoin exchange
client.

### Severity

"2.1 is out, it has a nicer order book" and "2.0.3 fixes a bug that can leak your trade
key — update now" are not the same message, and a client that renders them identically
makes the second look like the first.

| `severity` | For |
|---|---|
| `info` | releases, new features, events |
| `warning` | outages, a relay being retired, anything with a deadline |
| `critical` | a security issue the user must act on now |

**A publisher declares a severity, never a colour, an icon or any other presentation
value.** The mapping to a visual treatment belongs to the client: palettes differ between
themes, contrast pairs are something a design system has checked and an arbitrary hex is
not, and the publisher is trusted with authorship rather than with rendering — the same
reason `title` and `body` are plain text. A client MUST NOT accept presentation
instructions from this event, and SHOULD NOT rely on colour alone to convey the level.

**A client that does not recognise a `severity` value MUST still render the announcement**,
treating it as `warning`. This is the one field where the "unknown means ignore" rule of
`v` does not apply: `v` and the locale set decide whether a message is intelligible, while
severity only decides how it is presented, and dropping a security notice because a later
revision of this document added a level is the worst outcome available. `warning` rather
than `critical`, because a client cannot know which direction an unknown token sits in —
and because "any unrecognised string renders as the loudest level" is an escalation path a
careless or compromised publisher would use.

Severity is a claim about urgency, and it decays with misuse: a `critical` spent on a
release announcement teaches users that the level means nothing on the day it is true.
This document cannot enforce that and does not try — it is a rule for whoever holds the
key.

## What a reader owes

The publisher's signature is the trust model, so a client MUST NOT relax any of this:

0. **Never treat the event as presentation.** No markup in `title` / `body`, no colour or
   layout taken from `content`, and `url` the only actionable element.
1. **An allowlist of publisher keys, compiled into the client.** A list rather than a
   single key, so a successor can ship before it is needed, and not updatable over the
   wire — a remotely updatable allowlist is a channel for taking over the channel. The
   allowlist is **client-specific**: each project trusts its own keys, and this document
   does not define a global one.
2. **Explicit signature verification**, asserted where the event is used rather than
   assumed from whatever the relay pool verifies on its own.
3. **Freshness.** A relay can serve an old event forever. Reject `created_at` older than
   a bounded window (the reference client uses 30 days) and more than a few minutes in
   the future.
4. **Expiry, re-checked against the clock rather than the network.** An offline client
   receives nothing to displace what it holds, so an outage notice must be swept when its
   `expiration` passes even if no event arrives.
5. **Address by `(kind, pubkey, d)`, never `d` alone.** The allowlist is a list, so two
   publishers can pick the same `d`.

A correction republished under the same `d` from the same key supersedes the earlier
revision; a client SHOULD treat it as unread again, or it loses the correction in exactly
the case it was published for.

## Failure is silent

No relay, no announcement, bad JSON, a failed signature, an unknown `v`: all of it is
dropped without a word of UI. A user who never receives an announcement should not be able
to tell that they did not, and "an announcement failed to verify" is itself a message from
an untrusted source.

## Consent

Because this is a relay subscription rather than a push channel, a client that offers an
off switch MUST make "off" mean *the subscription is never opened*, not *arriving events
are hidden*. A relay must not be able to distinguish a user who opted out from a user who
closed the app.

Nothing is ever published back — no read receipt, no acknowledgement, no delivery report.
The channel is one-way toward the client.
