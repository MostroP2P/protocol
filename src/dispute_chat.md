# Dispute Chat

Once an admin takes a dispute, both the admin and the involved parties (buyer and seller) can communicate through an encrypted chat using Gift Wrap messages (NIP-59).

## Sending a message

Users and admins send messages using action `send-dm`. The message must be wrapped in a Gift Wrap event (kind 1059) to ensure privacy and encryption.

### User sending a message

Here is an example of a user sending a message to the admin:

```json
[
  {
    "dm": {
      "version": 1,
      "action": "send-dm",
      "payload": {
        "text_message": "Hello, I need help with this order"
      }
    }
  },
  null
]
```

This content is wrapped in a RUMOR (kind 1, unsigned), then encrypted in a SEAL (kind 13), and finally wrapped in a Gift Wrap event (kind 1059) before being published to the relays.

### Admin sending a message

Admins use the same format to send messages to users:

```json
[
  {
    "dm": {
      "version": 1,
      "action": "send-dm",
      "payload": {
        "text_message": "I'm reviewing the evidence, please wait"
      }
    }
  },
  null
]
```

## Gift Wrap structure

The Gift Wrap protocol (NIP-59) provides three layers of encryption:

### Layer 1: RUMOR (kind 1)

The original unsigned message containing the actual content:

```json
{
  "kind": 1,
  "content": "[{\"dm\": {\"version\": 1, \"action\": \"send-dm\", \"payload\": {\"text_message\": \"Hello\"}}}, null]",
  "pubkey": "<Sender's trade pubkey>",
  "created_at": 1234567890,
  "tags": []
}
```

### Layer 2: SEAL (kind 13)

The RUMOR encrypted with NIP-44:

```json
{
  "id": "<Seal event id>",
  "kind": 13,
  "content": "<Encrypted RUMOR with NIP-44>",
  "pubkey": "<Sender's trade pubkey>",
  "created_at": 1234567890,
  "tags": [],
  "sig": "<Signature>"
}
```

### Layer 3: GIFT WRAP (kind 1059)

The final layer using an ephemeral key:

```json
{
  "id": "<Gift wrap event id>",
  "kind": 1059,
  "content": "<Encrypted SEAL with NIP-44>",
  "pubkey": "<Ephemeral pubkey>",
  "created_at": 1234567890,
  "tags": [
    ["p", "<Receiver's pubkey>"]
  ],
  "sig": "<Signature>"
}
```

## Receiving messages

To receive messages, clients must:

1. Subscribe to kind 1059 events with a `p` tag matching their pubkey
2. Decrypt the Gift Wrap using their private key and the ephemeral pubkey
3. Decrypt the SEAL using their private key and the sender's pubkey from the SEAL
4. Extract the RUMOR content and parse the message

The sender's identity is preserved in the SEAL's pubkey field, allowing the receiver to identify who sent the message (admin or user) while maintaining privacy through the ephemeral key in the Gift Wrap layer.

## Message flow

```
User writes message
    ↓
Create RUMOR (kind 1)
    ↓
Encrypt into SEAL (kind 13) with sender's key
    ↓
Encrypt into Gift Wrap (kind 1059) with ephemeral key
    ↓
Publish to relays
    ↓
Receiver gets kind 1059 event
    ↓
Decrypt Gift Wrap → SEAL
    ↓
Decrypt SEAL → RUMOR
    ↓
Parse and display message
```

## Privacy features

The Gift Wrap protocol provides:

- **Double encryption**: SEAL + Gift Wrap layers
- **Sender anonymity**: Ephemeral keys hide the real sender from relay observers
- **Metadata obfuscation**: Randomized timestamps (±2 days)
- **End-to-end encryption**: Only sender and receiver can read messages
