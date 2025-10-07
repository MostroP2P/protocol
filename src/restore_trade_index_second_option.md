# Sync Trade Index

Defines the `synch-trade-index` action used to retrieve the user's last `trade_index`.

## Request

Client sends a Gift wrap Nostr event to Mostro with the following rumor's content. The request uses a `null` payload to indicate that the client is querying for the last trade index.

```json
{
  "restore": {
    "version": 1,
    "action": "synch-trade-index",
    "payload": null
  }
}
```

## Response

Mostro responds with the user's last trade index as a u32 directly in the `payload` field. If the user has never created a trade, the value SHOULD be `1`.

```json
{
  "restore": {
    "version": 1,
    "action": "synch-trade-index",
    "payload": 42
  }
}
```

### Fields

* `restore.version`: Protocol version. Current is `1`.
* `restore.action`: Must be `synch-trade-index`.
* `restore.payload` (request): Must be `null` when querying.
* `restore.payload` (response): u32 representing the last `trade_index` for the user. `1` if none.

## Example

Client requests the last trade index and receives `7`, meaning the next trade the client creates SHOULD use `trade_index = 8`.

```json
{
  "restore": {
    "version": 1,
    "action": "synch-trade-index",
    "payload": 7
  }
}
```


