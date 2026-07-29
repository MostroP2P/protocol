# Last Trade Index

Defines the `last-trade-index` action used to retrieve the user's last `trade_index`.

## Request

Client sends a NIP-44 direct message (kind `14`) to Mostro with the following decrypted content. The request sends a `null` payload to indicate that the client is querying for the last trade index.

```json
[
  {
    "restore": {
      "version": 2,
      "action": "last-trade-index",
      "payload": null
    }
  },
  null,
  null
]
```

## Response

Mostro responds with the user's last trade index as a u32 directly in the `trade_index` field. If the user has never created a trade, the value SHOULD be `1`.

```json
{
  "restore": {
    "version": 2,
    "action": "last-trade-index",
    "trade_index": 42,
    "payload": null
  }
}
```

### Fields

* `restore.version`: Protocol version — `1` on the gift-wrap transport (DEPRECATED), `2` on the NIP-44 direct transport. Current is `2`.
* `restore.action`: Must be `last-trade-index`.
* `restore.trade_index` (response): u32 representing the last `trade_index` for the user. `1` if none.
* `restore.payload` (response): Must be `null`.

## Example

Client requests the last trade index and receives `7`, meaning the next trade the client creates SHOULD use `trade_index = 8`.

```json
{
  "restore": {
    "version": 2,
    "action": "last-trade-index",
    "trade_index": 7,
    "payload": null
  }
}
```
