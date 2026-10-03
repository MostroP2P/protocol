# Reputation export and import

A user can copy the reputation earned on one venue — another Mostro instance or lnp2pBot
— into a Mostro instance: the source *exports* a [reputation attestation](./reputation_attestation.md)
for the user's identity, and the destination *imports* it. This chapter defines the
messages between a client and a Mostro instance; what an attestation contains, how an
issuer decides to sign one and the checks a destination runs are in
[Reputation attestation](./reputation_attestation.md).

Four actions, used verbatim on the wire:

| Action | Direction | Payload |
|---|---|---|
| `export-reputation` | client → issuer | `reputation_export_request` |
| `reputation-exported` | issuer → client | `reputation_attestation` |
| `import-reputation` | client → destination | `reputation_attestation` |
| `reputation-imported` | destination → client | `null` |

All four travel in the `order` wrapper with no `id`, like [`orders`](./orders.md), over
the node's ordinary transport: a NIP-44 direct message (kind `14`) authored by one of the
user's trade keys. A reply echoes the request's `request_id`. Both requests act on the
user's **identity**, so they MUST carry the identity proof (the third element of the
[content tuple](./key_management.md#identity-proof)); in full privacy mode there is no
identity-bound reputation to export or import into, and the node refuses with
`reputation_identity_required`.

## Discovery

A node advertises what it supports in its [info event](./other_events.md#reputation-tags):
`reputation_import_issuers` when it imports, `reputation_issuer` when it exports. A
client offers the import or export only for a node that advertises it, and never sends
these actions to one that does not: a node that predates them cannot parse them, and
would not answer at all.

## Export

The client asks the node to attest the user's reputation there for a destination
identity. That is normally the same identity — the client uses one identity key for every
node — but it is named explicitly, because a request may also move the binding to a new
identity with a rebind authorisation.

```json
[
  {
    "order": {
      "version": 2,
      "request_id": 4126,
      "action": "export-reputation",
      "payload": {
        "reputation_export_request": {
          "destination": "<Destination identity pubkey>",
          "rebind": null
        }
      }
    }
  },
  "<Trade key signature of the first element>",
  ["<Identity pubkey>", "<Identity proof signature>"]
]
```

- `destination`: the identity the attestation will name, 64 lowercase hex characters.
- `rebind`: `null`, or a [rebind authorisation](./reputation_attestation.md#rebinding)
  serialised as a JSON string, when `destination` differs from the identity the user's
  account is bound to on this node.

The source account is always the identity the transport proved, never a field of the
payload. Before sending the first export for an identity, the client shows that identity
as an `npub` and asks the user to confirm it: the request itself is the confirmation of
the binding.

The node answers with the attestation, serialised as a JSON string:

```json
[
  {
    "order": {
      "version": 2,
      "request_id": 4126,
      "action": "reputation-exported",
      "payload": {
        "reputation_attestation": "{\"id\":\"<Event id>\",\"pubkey\":\"<Issuer key>\",\"created_at\":1790899200,\"kind\":38388,\"tags\":[[\"p\",\"<Destination identity pubkey>\"],[\"subject\",\"<Source identity pubkey>\"],[\"reviews\",\"214\"],[\"rating\",\"4.87\"],[\"since\",\"1696204800\"],[\"expiration\",\"1791504000\"],[\"z\",\"reputation-attestation\"]],\"content\":\"\",\"sig\":\"<Issuer's signature>\"}"
      }
    }
  },
  null,
  null
]
```

The client verifies the attestation as a destination would before keeping it, shows the
user the figures, and stores it until it is imported. A lost or expired attestation is
simply requested again: a request for the identity already bound is always answered.

lnp2pBot issues attestations too, through Telegram rather than these messages: see
[Telegram hand-off](./reputation_attestation.md#telegram-hand-off).

## Import

The client sends the attestation to the destination, unchanged, in the same
serialisation it was received:

```json
[
  {
    "order": {
      "version": 2,
      "request_id": 4127,
      "action": "import-reputation",
      "payload": {
        "reputation_attestation": "<Attestation as a JSON string>"
      }
    }
  },
  "<Trade key signature of the first element>",
  ["<Identity pubkey>", "<Identity proof signature>"]
]
```

The destination runs the [redemption checks](./reputation_attestation.md#redemption),
records the import and merges it into the user's reputation, then confirms:

```json
[
  {
    "order": {
      "version": 2,
      "request_id": 4127,
      "action": "reputation-imported",
      "payload": null
    }
  },
  null,
  null
]
```

and republishes the user's [rating event](./user_rating.md) with the merged figures. The
order events published from then on carry them in their `rating` tag.

## Refusals

Either request is refused with a `cant-do` whose reason says why (see
[Cant Do Reasons](./message_suggestions_for_actions.md#cant-do-reasons)):

| Reason | Request | When |
|---|---|---|
| `invalid_action` | both | the node does not import (or does not export) |
| `invalid_payload` | both | the payload is missing or is not the one the action takes |
| `reputation_identity_required` | both | the request carries no identity proof |
| `not_eligible_for_reputation_export` | export | the account is banned, or has fewer than 10 completed trades or 5 ratings received |
| `reputation_bound_to_other_identity` | export | the account is bound to another identity and the request carries no rebind authorisation |
| `invalid_reputation_rebind` | export | the rebind authorisation is malformed, badly signed, not signed by the bound identity, names another issuer or has expired |
| `invalid_reputation_attestation` | import | the attestation does not parse, is badly signed, breaks a tag rule, is dated in the future or lives longer than the destination's cap |
| `untrusted_reputation_issuer` | import | the signing key is in no entry of the destination's trust list, or is the destination's own issuer key |
| `expired_reputation_attestation` | import | the attestation has expired |
| `reputation_identity_mismatch` | import | the attestation names another identity than the one the request proves |
| `reputation_already_imported` | import | this source account, or another account from the same issuer for this identity, was imported before |

## Compatibility

These actions do not change the protocol version: `version` stays `2`. A client that does
not implement them never sends them and is never sent their replies, because they are
only ever answers to its own request; nothing about them is broadcast. A client that does
implement them reads the info event first (see [Discovery](#discovery)), so it never
sends them to a node that cannot parse them. A client built on a `mostro-core` older than
the one that adds the new `cant-do` reasons reads them as `unknown`.
