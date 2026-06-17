## Overview

**See also:** [User Guide (English)](https://mostro.network/docs-english/) | [Guía de Usuario (Español)](https://mostro.network/docs-spanish/) | [Run Your Own Node](https://mostro.community/guide)

Mostro uses [Addressable Events](https://github.com/nostr-protocol/nips/blob/master/01.md#kinds) to publish different types of information. Each event type has its own `kind`:

| Event Type | Kind  | Description |
|------------|-------|-------------|
| Orders     | 38383 | P2P order events for the shared order book |
| Ratings    | 38384 | User rating events |
| Info       | 38385 | Mostro instance status and information |
| Disputes   | 38386 | Dispute events |

You can find more details about the order event [here](./order_event.md)

## The Message

### Transports

Mostro messages travel over one of two interchangeable wire transports. A
given node speaks **exactly one** of them — there is no dual mode — and
advertises which in its [instance-info event](./other_events.md#mostro-instance-status)
(kind `38385`) through the `protocol_version` tag (`"1"` or `"2"`):

| Protocol | Transport | Event kind | Status |
|----------|-----------|------------|--------|
| **v1** | [NIP-59 Gift Wrap](https://github.com/nostr-protocol/nips/blob/master/59.md) | `1059` | **DEPRECATED** (default through v0.18.x) |
| **v2** | NIP-44 direct message | `14` | current (default from v0.19.0) |

Both transports carry the **same logical message** and, once unwrapped,
yield the same structure to the daemon's handlers — only the envelope and
how the identity key is proven differ. Client developers should support
both during the transition and pick per node from the `protocol_version`
tag; see the [client migration guide](./transport_migration.md).

The logical message itself (the first tuple element described below) is
identical across transports, except for the `version` field: `1` on the
gift-wrap transport, `2` on the NIP-44 direct transport.

### The logical message

In **protocol v1** all **_messages_** from/to Mostro are
[Gift wrap Nostr events](https://github.com/nostr-protocol/nips/blob/master/59.md);
the `content` of the `rumor` event is a JSON-serialized `array` as a string
(with no white space or line breaks), the first element is the message, the
second element is the `signature` of the sha256 hash of the serialized
first element. In **protocol v2** the same array travels NIP-44 encrypted
inside a signed kind-`14` event and gains a third element (the identity
proof) — see [Keys management](./key_management.md#protocol-v2--nip-44-direct-messages)
for the v2 wire format. Here the structure of the first element (the
logical message, shared by both transports):

- [Wrapper](https://docs.rs/mostro-core/latest/mostro_core/message/enum.Message.html): Wrapper of the **_Message_**
  - <`version` integer>: Version of the protocol — `1` on the gift-wrap transport, `2` on the NIP-44 direct transport
  - [`id` integer]: (optional) Wrapper Id
  - [`request_id` integer]: (optional) Mostro daemon should send back this same id in the response
  - [`trade_index` integer]: (optional) This field is used by users who wants to maintain reputation, it should be the index of the trade in the user's trade history
  - <`action` string>: [Action](https://docs.rs/mostro-core/latest/mostro_core/message/enum.Action.html) to be performed by Mostro daemon
  - [`payload` string]: (optional) [Payload](https://docs.rs/mostro-core/latest/mostro_core/message/enum.Content.html) of the message, it should be a JSON-serialized string. The content of this field depends on the `action` field.

Here an example of a `new-order` order **_message_**:

```json
{
  "order": {
    "version": 1,
    "id": "<Order id>",
    "request_id": 123456,
    "trade_index": 1,
    "action": "new-order",
    "payload": {
      "order": {
        "id": "<Order id>",
        "kind": "sell",
        "status": "pending",
        "amount": 0,
        "fiat_code": "VES",
        "fiat_amount": 100,
        "payment_method": "face to face",
        "premium": 1,
        "created_at": 1698870173
      }
    }
  }
}
```

### The content array (v1 vs v2)

The first element above is the logical message. The array that actually
travels in the `content` differs by transport:

**Protocol v1** (gift wrap) — a **2-element** array:

```json
[
  { "order": { "version": 1, "...": "..." } },
  "<trade-key signature, or null>"
]
```

**Protocol v2** (NIP-44 direct) — a **3-element** array, the v1 pair plus an
identity proof, NIP-44 encrypted inside a signed kind-`14` event:

```json
[
  { "order": { "version": 2, "...": "..." } },
  "<trade-key signature, or null>",
  ["<identity pubkey>", "<identity signature>"]
]
```

| element | meaning |
|---|---|
| 1 | the logical message (the `version: 2` wrapper shown above) |
| 2 | the trade key's signature over the serialized first element, or `null` (Mostro's own messages, and full-privacy mode, are unsigned — as in v1) |
| 3 | the identity proof `["<identity pubkey>", "<identity signature>"]`, or `null` for full-privacy mode (where the identity is the trade key itself) |

In v1 the identity key is carried, authenticated, by the gift-wrap *seal*.
v2 has no seal, so the identity instead travels **inside the ciphertext**
as element 3 — never visible at the event level, exactly as private as
before. The full v2 envelope and identity-proof signing rule are documented
in [Keys management](./key_management.md#protocol-v2--nip-44-direct-messages).

## Payment Request Array Structure

The `payment_request` field in the payload can have different structures depending on the use case:

1. **Lightning invoice from buyer/seller to Mostro** (action: `add-invoice`):
   - With amount: `[null, "lnbc..."]`
   - Without amount (0-amount invoice): `[null, "lnbc...", amount_in_sats]`

2. **Lightning address**:
   - Taking a sell order with fixed amount: `[null, "user@domain.com"]`
   - Taking a sell Range order: `[null, "user@domain.com", fiat_amount]`

## Optional Fields

Some fields in order objects may be `null` or omitted depending on the context:

- `request_id`: Optional field that clients can include to correlate responses with requests. Mostro will echo this value back in responses.
- `trade_index`: Required for users maintaining reputation, omitted when using full privacy mode.
- `expires_at`: Unix timestamp when the order expires. Present in order confirmations and updates, but set to `0` or `null` when creating new orders.
- `created_at`: Unix timestamp when the order was created. Set to `0` or current time when creating orders; Mostro will set the actual timestamp.
- Signature (second array element): Required for reputation mode (signs the first element with trade key), set to `null` in full privacy mode or for Mostro responses.

## Payment Method Format

Payment methods can be specified like this:

```json
   ["pm", "face to face", "bank transfer", "mobile"]
```
