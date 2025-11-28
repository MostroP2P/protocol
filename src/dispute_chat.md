# Dispute Chat

Once an admin takes a dispute, both the admin and the involved parties (buyer and seller) can communicate through an encrypted chat using Gift Wrap messages (NIP-59).

## Sending a message

Users and admins send messages using action `send-dm`. The message content is wrapped in a Gift Wrap event (kind 1059) to ensure privacy and encryption.

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

## Receiving messages

Clients must subscribe to kind 1059 events with a `p` tag matching their pubkey. The Gift Wrap protocol (NIP-59) ensures that only the intended recipient can decrypt and read the messages, while preserving the sender's identity through the SEAL layer
