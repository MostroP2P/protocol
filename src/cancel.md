# Cancel Order

A user can cancel an order created by himself and with status `pending` sending action `cancel`, the rumor's content of the message will look like this:

```json
[
  {
    "order": {
      "version": 1,
      "id": "<Order Id>",
      "action": "cancel",
      "payload": null
    }
  },
  "<index N signature of the sha256 hash of the serialized first element of content>"
]
```

## Mostro response

Mostro will send a message with action `cancel` confirming the order was canceled, here an example of rumor's content of the message:

```json
[
  {
    "order": {
      "version": 1,
      "id": "<Order Id>",
      "action": "canceled",
      "payload": null
    }
  },
  null
]
```

Mostro updates the addressable event with `d` tag `<Order Id>` to change the status to `canceled`:

```json
[
  "EVENT",
  "RAND",
  {
    "id": "<Event id>",
    "pubkey": "<Mostro's pubkey>",
    "created_at": 1702549437,
    "kind": 38383,
    "tags": [
      ["d", "<Order Id>"],
      ["k", "sell"],
      ["f", "VES"],
      ["s", "canceled"],
      ["amt", "7851"],
      ["fa", "100"],
      ["pm", "face to face"],
      ["premium", "1"],
      ["network", "mainnet"],
      ["layer", "lightning"],
      ["expiration", "1719391096"],
      ["y", "mostro", "[Mostro instance name]"],
      ["z", "order"]
    ],
    "content": "",
    "sig": "<Mostro's signature>"
  }
]
```

## Bond race during take

Multiple takers may simultaneously attempt to take the same order: each creates their own bond hold invoice (a `Requested` bond row) and whichever HTLC is accepted first by LND wins. When a bond locks, all other concurrent `Requested` bonds on the same order are released and their takers each receive `Action::Canceled`.

A client that sent `take-buy` / `take-sell` and is waiting for [`pay-bond-invoice`](./pay_bond_invoice.md) may therefore receive `canceled` instead — meaning another taker's bond locked first. Surface this clearly to the user, e.g. *"Order was taken by another user before you locked the bond."* Do not silently retry the take — the order may no longer be available.

## Cancel during `waiting-taker-bond`

An order remains in the daemon-internal `waiting-taker-bond` state (NIP-33 `s` tag still `pending`) while one or more taker bonds are outstanding. Cancels during this window are routed differently from cancels on `pending` orders.

### Taker self-cancel

A taker who has a `Requested` (unpaid) bond on an order may cancel it by sending `cancel`. Mostro releases **only that taker's bond** (LND invoice cancelled, funds returned). Other concurrent takers' bonds are unaffected and continue racing.

- If the cancelling taker was the only prospective taker, the order drops back to `pending` and is visible on the order book again.
- If other takers' bonds are still `Requested`, the order remains in `waiting-taker-bond` with those bonds still racing.

The maker is not notified; the NIP-33 order event stays `pending` throughout.

### Maker cancel

The maker can cancel the order at any point before trade flow starts (i.e. while in `pending` or `waiting-taker-bond`). Sending `cancel` releases **all** concurrent taker bonds on the order, notifies each prospective taker with `canceled`, transitions the order to `canceled`, and publishes the updated NIP-33 event.

## Cancel cooperatively

A user can cancel an `active` order, but will need the counterparty to agree, let's look at an example where the seller initiates a cooperative cancellation:

```json
[
  {
    "order": {
      "version": 1,
      "id": "<Order Id>",
      "action": "cancel",
      "payload": null
    }
  },
  null
]
```

Mostro will send this message to the seller:

```json
[
  {
    "order": {
      "version": 1,
      "id": "<Order Id>",
      "action": "cooperative-cancel-initiated-by-you",
      "payload": null
    }
  },
  null
]
```

And this message to the buyer:

```json
[
  {
    "order": {
      "version": 1,
      "id": "<Order Id>",
      "action": "cooperative-cancel-initiated-by-peer",
      "payload": null
    }
  },
  null
]
```

The buyer can accept the cooperative cancellation sending this message:

```json
[
  {
    "order": {
      "version": 1,
      "id": "<Order Id>",
      "action": "cancel",
      "payload": null
    }
  },
  null
]
```

And Mostro will send this message to both parties:

```json
[
  {
    "order": {
      "version": 1,
      "id": "<Order Id>",
      "action": "cooperative-cancel-accepted",
      "payload": null
    }
  },
  null
]
```
Mostro updates the addressable event with `d` tag `<Order Id>` to change the status to `canceled`:

```json
[
  "EVENT",
  "RAND",
  {
    "id": "<Event id>",
    "pubkey": "<Mostro's pubkey>",
    "created_at": 1702549437,
    "kind": 38383,
    "tags": [
      ["d", "<Order Id>"],
      ["k", "sell"],
      ["f", "VES"],
      ["s", "canceled"],
      ["amt", "7851"],
      ["fa", "100"],
      ["pm", "face to face"],
      ["premium", "1"],
      ["network", "mainnet"],
      ["layer", "lightning"],
      ["expiration", "1719391096"],
      ["y", "mostro", "[Mostro instance name]"],
      ["z", "order"]
    ],
    "content": "",
    "sig": "<Mostro's signature>"
  }
]
```
