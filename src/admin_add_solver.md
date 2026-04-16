# Admin add solver

Solvers are users appointed by the Mostro administrator and are responsible for helping resolve disputes.

The administrator can add or remove them at any time.

The administrator can also solve disputes.

To add a solver the admin sends an `order` message to Mostro with action `admin-add-solver`.

## Payload format

The payload uses `text_message`.

Supported formats:

- `npub1...` -> register solver with `read-write` permission (default)
- `npub1...:read` -> register solver with `read` permission only
- `npub1...:read-write` -> register solver with `read-write` permission
- `npub1...:write` -> alias for `read-write`

Permission meanings:

- `read`: solver can take disputes, receive dispute context, and communicate with users, but cannot execute `admin-settle` or `admin-cancel`
- `read-write`: solver can do everything above and can also execute `admin-settle` and `admin-cancel`

The default remains `read-write` for backward compatibility.

## Example: default read-write solver

```json
[
  {
    "order": {
      "version": 1,
      "action": "admin-add-solver",
      "payload": {
        "text_message": "npub1qqq884wtp2jn96lqhqlnarl4kk3rmvrc9z2nmrvqujx3m4l2ea5qd5d0fq"
      }
    }
  },
  null
]
```

## Example: read-only solver

```json
[
  {
    "order": {
      "version": 1,
      "action": "admin-add-solver",
      "payload": {
        "text_message": "npub1qqq884wtp2jn96lqhqlnarl4kk3rmvrc9z2nmrvqujx3m4l2ea5qd5d0fq:read"
      }
    }
  },
  null
]
```

## Mostro response

Mostro sends this message to the admin:

```json
[
  {
    "order": {
      "version": 1,
      "action": "admin-add-solver",
      "payload": null
    }
  },
  null
]
```
