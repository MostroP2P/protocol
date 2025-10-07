# Restore Trade Index

Use this message to request the last trade index associated with the user's mnemonic, while keeping the `restore-session` action. This allows clients to resume indexing from the correct `trade_index` value without changing the action name.

## Request

Client sends a Gift wrap Nostr event to Mostro with the following rumor's content. The client indicates the request by sending a `payload` object with a `last_trade_index` key set to `null`:

```json
{
  "restore": {
    "version": 1,
    "action": "restore-session",
    "payload": {
      "last_trade_index": null
    }
  }
}
```

## Response

Mostro responds with the user's last trade index as a u32 under the same `payload.last_trade_index` key. If the user has never created a trade, the value SHOULD be `1`.

```json
{
  "restore": {
    "version": 1,
    "action": "restore-session",
    "payload": {
      "last_trade_index": 42
    }
  }
}
```

### Fields

* `restore.version`: Protocol version. Current is `1`.
* `restore.action`: Must be `restore-session`.
* `restore.payload.last_trade_index` (request): Must be `null` to indicate a query.
* `restore.payload.last_trade_index` (response): u32 representing the last `trade_index` for the user. `1` if none.

## Example

Client requests the last trade index and receives `7`, meaning the next trade the client creates SHOULD use `trade_index = 8`.

```json
{
  "restore": {
    "version": 1,
    "action": "restore-session",
    "payload": {
      "last_trade_index": 7
    }
  }
}
```
