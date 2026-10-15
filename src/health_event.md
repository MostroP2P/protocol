# Health event

Kind `38389` is reserved for the **health event**: a signed report, republished every
minute, of whether a Mostro instance is not just *running* but *working*: whether its
Lightning node is synced, whether buyers are being paid, whether it reaches its relays
and its database.

The [info event](./other_events.md#mostro-instance-status) (kind `38385`) answers "is
this instance alive and how is it configured". The health event answers "can this
instance do its job right now". An instance can keep publishing a perfectly normal info
event while every buyer payout fails, so the info event alone can't answer the second
question.

The same kind also carries the heartbeat of a **watcher**, a monitoring tool such as
mostro-watchdog, so that the watcher itself can be watched. See
[Watcher heartbeat](#watcher-heartbeat).

## Why a Nostr event

The reasoning is the same as for [Announcement events](./announcement_event.md#why-a-nostr-event).
The event is signed with the instance's key, so a relay can withhold it or serve a stale
copy but can't forge one. Nobody has to expose an HTTP endpoint, open an account or run
central monitoring: any instance can be watched by anyone who can read a relay, the same
way order and info events are already consumed. Because the event is addressable, relays
keep only the latest one per instance.

## The event

An [addressable event](https://github.com/nostr-protocol/nips/blob/master/01.md#kinds)
published by the Mostro daemon with its own key:

```json
[
  "EVENT",
  "RAND",
  {
    "id": "<Event id>",
    "pubkey": "<Mostro's pubkey>",
    "created_at": 1792000000,
    "kind": 38389,
    "tags": [
      ["d", "<Mostro's pubkey>"],
      ["y", "mostro", "[Mostro instance name]"],
      ["z", "health"],
      ["status", "down"],
      ["check", "lnd_sync", "fail", "1791952200"],
      ["check", "payouts", "fail", "1791959400"],
      ["check", "relays", "ok", "1791900000"],
      ["check", "database", "ok", "1791900000"],
      ["escrow_mode", "lightning"],
      ["lnd_block_height", "971349"],
      ["lnd_synced_to_chain", "false"],
      ["lnd_network", "mainnet"],
      ["maintenance_mode", "true"],
      ["mostro_version", "0.19.3"],
      ["interval", "60"],
      ["seq", "1532"],
      ["p", "<Watchdog's pubkey>"],
      ["expiration", "1792000180"]
    ],
    "content": "<NIP-44 v2 payload encrypted to the `p` pubkey>",
    "sig": "<Mostro's signature>"
  }
]
```

### Tags

Every value is a string, as in all Nostr tags. "Integer" below means a base-10 integer
with no sign, no leading zeros and no fraction, for example `"971349"`. "Unix seconds" is
an integer number of seconds since 1970-01-01T00:00:00Z.

| Tag | Required | Value |
|---|---|---|
| `d` | yes | the publisher's own pubkey, lowercase hex, equal to the event's `pubkey`. Same convention as the info event: one health event per instance |
| `y` | yes | `["y", "mostro"]`, optionally with the instance name as a third element, exactly as in the info event |
| `z` | yes | `health` |
| `status` | yes | the daemon's self-assessed overall state: `ok`, `degraded` or `down`. See [Deriving `status`](#deriving-status) |
| `check` | yes, one per check | `["check", <name>, <state>, <since>]`. See [Checks](#checks) |
| `escrow_mode` | yes | `lightning` or `cashu`, same values as in the info event |
| `lnd_block_height` | see below | integer, the block height LND reports in `GetInfo` |
| `lnd_synced_to_chain` | see below | `true` or `false`, as LND reports it in `GetInfo` |
| `lnd_network` | see below | the Bitcoin network LND reports in `GetInfo` (the `network` of its `chains` entry): `mainnet`, `testnet`, `testnet4`, `signet`, `regtest` or `simnet` |
| `maintenance_mode` | yes | `true` or `false`, same meaning as in the [info event](./other_events.md#maintenance-mode-tag) |
| `mostro_version` | yes | the daemon version, same value as in the info event |
| `interval` | yes | integer seconds between publications, between `30` and `3600` inclusive. Default `60` |
| `seq` | yes | integer between `1` and `9007199254740991` (2<sup>53</sup> − 1). See [`seq`](#seq) |
| `p` | no | lowercase hex pubkey the encrypted `content` is addressed to. Present if and only if `content` is not empty. At most one `p` tag |
| `expiration` | yes | [NIP-40](https://github.com/nostr-protocol/nips/blob/master/40.md) unix seconds, **exactly** `created_at + 3 × interval` |

Every tag in this table appears **at most once**, except `check`, which appears once per
check name and never twice with the same name. A tag marked "yes" appears exactly once. A
reader rejects an event that repeats any other tag or repeats a check name: Nostr allows
repeated tags, and parsers that read the first or the last occurrence would otherwise
disagree on its freshness, order or state.

`lnd_block_height`, `lnd_synced_to_chain` and `lnd_network` are present when `escrow_mode` is
`lightning` and the last `GetInfo` call succeeded. They are omitted when `escrow_mode` is
`cashu` (there is no LND node) or when LND could not be reached. In that second case the
`lnd_sync` check is `unknown`. A Lightning instance that omits them is therefore saying
"I can't reach my node", not "I have none".

The block height is public on purpose: it is what an outside observer needs to
[cross-check](#what-a-reader-owes) the instance against a public block explorer or
against other instances, and it reveals nothing that the chain doesn't.

### Checks

Each `check` tag reports one aspect of the instance:

- `<name>` is one of the names below.
- `<state>` is one of `ok`, `warn`, `fail`, `unknown`.
- `<since>` is the unix seconds at which the check entered its current state. It lets a
  reader tell "failing for three minutes" from "failing for 21 hours" without keeping
  any history. A publisher that doesn't know when the state began, for example right
  after a restart, uses the moment it first observed it. `<since>` is never later than
  the event's `created_at`.

`unknown` means the publisher could not evaluate the check. It never means "probably
fine".

Each check appears at most once per event. Which checks a publisher emits depends on
`escrow_mode`:

| Check | `lightning` | `cashu` |
|---|---|---|
| `lnd_sync` | yes | no |
| `payouts` | yes | no |
| `relays` | yes | yes |
| `database` | yes | yes |

This list is closed for this version of the event: a publisher MUST NOT emit other
names. A reader MUST ignore names it doesn't know, so that later versions can add
checks without breaking anyone, and MUST treat a state value it doesn't know as
`unknown`.

The thresholds in the rules below are defaults. A daemon MAY let its operator change
them. What each state means does not change.

#### `lnd_sync`

From LND's `GetInfo`:

| State | When |
|---|---|
| `ok` | `synced_to_chain` is `true` |
| `warn` | `synced_to_chain` has been `false` for less than 600 seconds without interruption |
| `fail` | `synced_to_chain` has been `false` for 600 seconds or more without interruption |
| `unknown` | the `GetInfo` call failed or timed out |

`warn` exists because LND reports `synced_to_chain = false` for a few seconds whenever a
new block arrives; a single `false` is normal. The 600-second threshold is the same one
that turns on [automatic maintenance](#relation-to-maintenance-mode) in mostrod.

#### `payouts`

About **buyer payouts**: the Lightning payments the daemon makes to the buyer's invoice
or Lightning address after the seller releases, including retries. Dev-fee payments and
bond payouts are not buyer payouts and are not counted here.

A **payout attempt** is one payment to the buyer that reached a final state in LND,
succeeded or failed. A payout the daemon deferred without sending it, for example
because LND was not synced, is not an attempt.

A **settled unpaid order** is an order in status `settled-hold-invoice` (the seller's
hold invoice was settled, the buyer has not been paid yet) that the daemon is still
trying to pay. Orders for which the daemon has used up its payment attempts and asked
the buyer for a new invoice are counted separately (`awaiting_buyer_invoice_orders`
[below](#encrypted-content)) and are not settled unpaid orders: they are waiting on the
buyer, not on the daemon.

| State | When |
|---|---|
| `fail` | (a) there is at least one settled unpaid order older than 7200 seconds, and no payout attempt succeeded in the last 7200 seconds; **or** (b) at least 3 payout attempts reached a final state in the last 3600 seconds and more than half of them failed |
| `warn` | not `fail`, and there is at least one settled unpaid order older than 900 seconds |
| `ok` | neither of the above, including when there were no payouts at all |
| `unknown` | the publisher could not evaluate the rules, for example because the database query failed |

The age of a settled unpaid order is measured from the moment its hold invoice was
settled.

#### `relays`

Over the relays the daemon is configured to publish to:

| State | When |
|---|---|
| `ok` | every configured relay is connected |
| `warn` | at least one is connected and at least one is not |
| `fail` | none is connected |

While no relay is connected nothing can be published, so a `fail` normally never reaches
readers: they see the event go stale and report the instance as unknown, which is the
right signal. When a relay is back, the check reflects the current connections again,
and the outage is reported through `relays.last_outage_started_at` and
`relays.last_outage_ended_at` in the [encrypted content](#encrypted-content), which the
daemon persists so that it survives a restart during the outage.

#### `database`

| State | When |
|---|---|
| `ok` | a trivial query (such as `SELECT 1`) succeeded within 5 seconds |
| `fail` | it failed or took longer |

### Deriving `status`

`status` is a function of the checks and of `maintenance_mode`, applied in this order:

1. `down` if any of these holds: `lnd_sync` is `fail` or `unknown`; `database` is
   `fail`; `relays` is `fail`. The instance cannot do its basic job: escrow and pay
   out, or even talk to its users.
2. Otherwise `degraded` if any check is `warn`, `fail` or `unknown`, or
   `maintenance_mode` is `true`. The instance works, but something needs attention or it
   is deliberately not taking new trades.
3. Otherwise `ok`.

A reader can recompute `status` from the tags, so it never needs to trust it more than
the checks it summarises.

### `seq`

`seq` increases strictly on every publication by a given key, across restarts, and never
goes down. It can be a persisted counter, or a timestamp in milliseconds, as long as it
increases strictly. A reader uses it to discard an older revision a relay may still
serve. See [What a reader owes](#what-a-reader-owes).

### Encrypted content

When the operator configures a recipient, usually their own watchdog, `content` is a
[NIP-44](https://github.com/nostr-protocol/nips/blob/master/44.md) version 2 payload,
encrypted with the conversation key of the Mostro private key and the `p` pubkey. When no
recipient is configured, `content` is the empty string `""` and there is no `p` tag.

There is exactly one recipient. NIP-44 is one-to-one, and one event per recipient would
break "one addressable event per instance". An operator who runs several watchdogs gives
them the same key, or has one forward to the others.

The plaintext is a JSON object, UTF-8, no larger than 32 KiB:

```json
{
  "v": 1,
  "generated_at": 1792000000,
  "lnd": {
    "reachable": true,
    "synced_to_chain": false,
    "unsynced_since": 1791951600,
    "block_height": 971349,
    "best_header_timestamp": 1791951597,
    "peers": 19,
    "channels_active": 8,
    "channels_inactive": 1,
    "channels_pending": 0,
    "outbound_liquidity_sats": 52341870,
    "inbound_liquidity_sats": 13874104,
    "pending_htlcs": 37,
    "nearest_htlc_expiry_blocks": 291
  },
  "payouts": {
    "window_secs": 3600,
    "payouts_succeeded_1h": 0,
    "payouts_failed_1h": 37,
    "failure_reasons_1h": {
      "incorrect_payment_details": 31,
      "no_route": 6
    },
    "last_payout_success_at": 1791950000,
    "settled_unpaid_orders": 4,
    "oldest_settled_unpaid_order_age_secs": 61200,
    "awaiting_buyer_invoice_orders": 2
  },
  "dev_fee": {
    "pending": 12,
    "failed_1h": 410
  },
  "relays": {
    "configured": 4,
    "connected": 4,
    "last_outage_started_at": null,
    "last_outage_ended_at": null
  },
  "maintenance": {
    "enabled": true,
    "reason": "lnd_unsynced"
  }
}
```

The values in both examples are illustrative.

Every top-level field below is required in a version-1 payload. A nested field
(`lnd.*`, `payouts.*`, `dev_fee.*`, `relays.*`, `maintenance.*`) is required when its
parent object is present and not `null`; when the parent is `null`, it has no fields. A
value that is unknown is `null`, never omitted and never `0`: `0` is a measurement,
`null` is "I don't know".

| Field | Type | Meaning |
|---|---|---|
| `v` | integer | schema version, `1` |
| `generated_at` | unix seconds | when the values were collected. Equal to or slightly earlier than the event's `created_at` |
| `lnd` | object or `null` | `null` when `escrow_mode` is `cashu` |
| `lnd.reachable` | boolean | `false` when the last `GetInfo` call failed. Every other `lnd` field is then `null` |
| `lnd.synced_to_chain` | boolean or `null` | from `GetInfo` |
| `lnd.unsynced_since` | unix seconds or `null` | when the current uninterrupted run of `synced_to_chain = false` began; `null` while synced |
| `lnd.block_height` | integer or `null` | from `GetInfo` |
| `lnd.best_header_timestamp` | unix seconds or `null` | from `GetInfo` |
| `lnd.peers` | integer or `null` | number of connected Lightning peers |
| `lnd.channels_active` | integer or `null` | from `GetInfo` |
| `lnd.channels_inactive` | integer or `null` | from `GetInfo` |
| `lnd.channels_pending` | integer or `null` | from `GetInfo` |
| `lnd.outbound_liquidity_sats` | integer or `null` | sum of `local_balance` over **active** channels, in sats |
| `lnd.inbound_liquidity_sats` | integer or `null` | sum of `remote_balance` over **active** channels, in sats |
| `lnd.pending_htlcs` | integer or `null` | number of pending HTLCs over all channels |
| `lnd.nearest_htlc_expiry_blocks` | integer or `null` | minimum of `expiration_height − block_height` over all pending HTLCs, in blocks. `null` when there are no pending HTLCs. Zero or negative when an HTLC has already reached its expiry |
| `payouts` | object or `null` | `null` when `escrow_mode` is `cashu` |
| `payouts.window_secs` | integer | the window of the `_1h` fields: always `3600` in version 1 |
| `payouts.payouts_succeeded_1h` | integer | payout attempts that succeeded in the last `window_secs` |
| `payouts.payouts_failed_1h` | integer | payout attempts that failed in the last `window_secs` |
| `payouts.failure_reasons_1h` | object | failed attempts in the window by reason: key = reason, value = count. Keys with a zero count MAY be omitted, so `{}` means no failures |
| `payouts.last_payout_success_at` | unix seconds or `null` | the last successful payout attempt, persisted across restarts. `null` if there was never one |
| `payouts.settled_unpaid_orders` | integer | settled unpaid orders, as defined [for the `payouts` check](#payouts) |
| `payouts.oldest_settled_unpaid_order_age_secs` | integer or `null` | age of the oldest settled unpaid order, in seconds. `null` when there are none |
| `payouts.awaiting_buyer_invoice_orders` | integer | orders in `settled-hold-invoice` for which the daemon used up its attempts and asked the buyer for a new invoice |
| `dev_fee` | object or `null` | `null` when the instance pays no dev fee |
| `dev_fee.pending` | integer | completed orders whose dev fee is not paid yet |
| `dev_fee.failed_1h` | integer | dev-fee payment attempts that failed in the last 3600 seconds |
| `relays` | object | always present, never `null` |
| `relays.configured` | integer | relays the daemon is configured to publish to |
| `relays.connected` | integer | of those, how many are connected |
| `relays.last_outage_started_at` | unix seconds or `null` | when the daemon last lost its last connected relay, persisted across restarts. `null` if that never happened |
| `relays.last_outage_ended_at` | unix seconds or `null` | when a relay was connected again after that outage. `null` if there was no outage, or while it is still going on (which a published event can't show) |
| `maintenance` | object | always present, never `null` |
| `maintenance.enabled` | boolean | same value as the `maintenance_mode` tag |
| `maintenance.reason` | string or `null` | why maintenance is on. `null` when it is off or no reason was given. `lnd_unsynced` is reserved for maintenance the daemon turned on by itself because LND was not synced; any other value was set by the operator |

Failure reason keys are LND's `PaymentFailureReason` values, lowercase, without the
`FAILURE_REASON_` prefix: `timeout`, `no_route`, `error`, `incorrect_payment_details`,
`insufficient_balance`, `canceled`. A failure the daemon cannot map to one of them is
counted under `other`. A reader MUST accept a `failure_reasons_1h` key it doesn't
recognize, from a newer LND or daemon: it counts it as `other` or ignores it, and never
rejects the payload because of it.

A reader:

- MUST ignore fields it doesn't know.
- MUST ignore the whole payload, and use only the public tags, when `v` is not a
  version it knows, the payload doesn't decrypt or the JSON doesn't parse.
- MUST NOT trust the payload over the public tags when they disagree. They are produced
  together, so a disagreement is a bug to report, not a fact to act on.

### Relation to maintenance mode

The `maintenance_mode` tag has the same meaning as in the info event. A daemon may turn
maintenance on by itself, with reason `lnd_unsynced`, when its `lnd_sync` check reaches
`fail`. It turns it off again only if the reason is still `lnd_unsynced`, so that it
never lifts maintenance an operator set. The tag and `maintenance.reason` let a reader
tell the two apart.

## Publishing

- The daemon publishes the event every `interval` seconds, to the same relays as its
  info event. `interval` defaults to `60`.
- It MAY also republish immediately when `status`, the state of any check or
  `maintenance_mode` changes, but not more often than once every 10 seconds. Every
  publication, scheduled or immediate, gets a new `seq` and a new `expiration`.
- `created_at` is the publisher's clock at publication time, but never lower than the
  `created_at` of its previous publication plus one. The publisher persists the last
  `created_at` it used, alongside `seq`. Relays keep the latest revision of an
  addressable event by `created_at` (NIP-01), not by `seq`, so a clock that steps back
  (an NTP correction, a restored VM, a reboot with a wrong clock) would otherwise make
  relays keep serving the previous revision.
- If that rule puts `created_at` more than 300 seconds ahead of the publisher's own clock,
  readers will reject the event as coming from the future (rule 3 of
  [What a reader owes](#what-a-reader-owes)). The publisher still publishes, so the event
  becomes valid again once its clock catches up, and logs an error: its clock moved
  backwards by more than readers tolerate, and the operator has to look at it. Until
  then the instance shows as unknown, which is the right signal for a host whose clock
  can't be trusted.

Because `expiration` is exactly three intervals ahead, a daemon that stops publishing
stops being shown as healthy within three intervals, even by a reader that does no
freshness check of its own. Relays that honour NIP-40 drop the event at that point.

## What a reader owes

A missing health event must never look like a healthy instance. Clients, status pages
and watchdogs:

1. **Verify before reading.** Check the signature (NIP-01), that `pubkey` is the instance
   you meant to watch, that `kind` is `38389`, that `z` is `health`, that `d` equals
   `pubkey`, and that every required tag is present and well-formed as described in
   [Tags](#tags). In particular, `interval` must be within `30`–`3600` and `expiration`
   must equal `created_at + 3 × interval`. Ignore anything else.
2. **Take the newest valid revision.** Query every relay you use. Among the events that
   pass these rules, keep the one with the highest `seq`. Ignore an event whose `seq` is
   not greater than the highest `seq` already accepted from that pubkey. A reader that
   runs continuously, such as a watcher, MUST persist, per pubkey, the highest accepted
   `seq` together with the `id` of the event that carried it, and restore both before
   processing events after a restart. After a restart it accepts an event with that same
   `seq` only if its `id` matches the persisted one (the revision it had already
   accepted, which restores the current state at once), and rejects any lower `seq`, so
   that an older event that has not expired yet is not accepted again. A stateless reader, such as a
   status page loaded in a browser, can't keep it; rule 4 then limits a replay to an
   event at most `3 × interval` seconds old.
3. **Tolerate clock skew, but not much.** Ignore an event whose `created_at` is more than
   300 seconds in the future.
4. **Treat absence and staleness as unknown, never as `ok`.** The instance's state is
   **unknown** when:
   - no valid health event was found;
   - the current time is at or past its `expiration`;
   - its `created_at` is more than `3 × interval` seconds in the past.
5. **Don't take `status = ok` as proof.** The daemon can only report what its own node
   tells it, and a node on a stale chain doesn't necessarily know it is stale. Watchers
   SHOULD compare `lnd_block_height` with an independent reference: one or more public
   block explorers, or the median `lnd_block_height` of other instances' fresh health
   events. A lag of 3 blocks or more that lasts is a failure whatever `status` says.
   Heights are only comparable within one chain: use an explorer for the instance's
   `lnd_network`, and take the median only over instances with the same `lnd_network`.
   When there is no independent reference for that network (for example `regtest`),
   skip the cross-check rather than compare against another chain.
6. **Show the age.** Display when the last valid event was seen, not just its state.

A reader that discovers instances, for example a status page listing every Mostro
instance, filters on the `z` tag (`{"kinds": [38389], "#z": ["health"]}`) so that
watcher heartbeats are not taken for instances.

## Watcher heartbeat

A monitoring tool publishes its own event of kind `38389`, with its own key, so that its
liveness can be checked the same way. Two watchers on different machines can then watch
each other, and the watcher can use its own heartbeat as a relay round-trip probe: if it
can't read back the event it just published, the relay is the problem, not the instance
it watches.

```json
[
  "EVENT",
  "RAND",
  {
    "id": "<Event id>",
    "pubkey": "<Watcher's pubkey>",
    "created_at": 1792000000,
    "kind": 38389,
    "tags": [
      ["d", "<Watcher's pubkey>"],
      ["y", "mostro-watchdog"],
      ["z", "watcher"],
      ["watching", "<Mostro's pubkey>"],
      ["interval", "60"],
      ["seq", "88213"],
      ["expiration", "1792000180"]
    ],
    "content": "",
    "sig": "<Watcher's signature>"
  }
]
```

| Tag | Required | Value |
|---|---|---|
| `d` | yes | the watcher's own pubkey, equal to the event's `pubkey` |
| `y` | yes | the name of the monitoring software, for example `mostro-watchdog` |
| `z` | yes | `watcher` |
| `watching` | no | lowercase hex pubkey of a Mostro instance this watcher monitors, one tag per instance. Optional, because it publicly links the watcher to the instances |
| `interval`, `seq`, `expiration` | yes | same rules as for the instance's health event |

A watcher sets `created_at` with the same rule as the daemon (see [Publishing](#publishing)):
never lower than its previous heartbeat's `created_at` plus one, persisted with `seq`, so
that relays keep its newest heartbeat after a clock step.

`content` is the empty string. A watcher heartbeat carries no `status` and no `check`
tags: it only says "this watcher is alive". Readers validate it against the table above,
not against the instance's [Tags](#tags) table: rule 1 of
[What a reader owes](#what-a-reader-owes) applies with `z` = `watcher` and with the
required tags of this table, so the instance-only tags (`status`, `check`,
`escrow_mode`, `maintenance_mode`, `mostro_version`) are neither required nor expected.
The uniqueness rule of [Tags](#tags) applies here too: `d`, `y`, `z`, `interval`, `seq`
and `expiration` appear exactly once, `watching` once per instance and never twice with
the same pubkey, and a reader rejects a heartbeat that repeats any of them.
Rules 2 to 4 (`seq`, clock skew and freshness) apply unchanged.

## Privacy

- The public tags say whether an instance is working, and nothing more: a coarse state
  per check, the block height and the maintenance flag. That is what a user or a status
  page needs.
- Liquidity, channel counts, failure rates and the number of stuck orders are
  operational detail. In the clear they would leak business information, volume and
  liquidity over time, and would tell an attacker exactly when an instance is weakest.
  They travel encrypted to the operator's own monitoring key.
- The `p` tag and the optional `watching` tag publicly link an instance to its
  watchdog's key. An operator who doesn't want that link can leave `content` empty and
  the `watching` tags out, and monitor from the public tags alone.
- An operator who wants more transparency can publish the numbers elsewhere. The
  protocol's default is coarse in public, detailed in private.

## Compatibility

The kind is new: existing clients, indexers and relays are unaffected, and the info
event does not change. Implementations that predate this page simply publish no health
event, which readers treat as **unknown**, never as `ok`.
