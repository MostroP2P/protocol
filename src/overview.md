## Overview

In order to have a shared order's book, Mostro daemon send [Addressable Events](https://github.com/nostr-protocol/nips/blob/master/01.md#kinds) with `38383` as event `kind`, you can find more details about that specific event [here](./order_event.md)

## The Message

All **_messages_** from/to Mostro should be [Gift wrap Nostr events](https://github.com/nostr-protocol/nips/blob/master/59.md), the `content` of the `rumor` event is a JSON-serialized `array` as a string (with no white space or line breaks), the first element is the message, the second element is the `signature` of the sha256 hash of the serialized first element, here the structure of the first element:

- [Wrapper](https://docs.rs/mostro-core/latest/mostro_core/message/enum.Message.html): Wrapper of the **_Message_**
  - <`version` integer>: Version of the protocol, currently `1`
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
