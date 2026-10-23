# Stablecoins

Mostro can trade bitcoin for stablecoins. The coin is the order's currency, in the `f`
tag where a fiat order has its ISO 4217 code, and each coin is pegged 1:1 to a fiat:

| Code | Coin | Pegged to |
| --- | --- | --- |
| `DOC` | Dollar on Chain (Money on Chain, Rootstock) | `USD` |
| `USDC` | USD Coin (Circle) | `USD` |
| `USDT` | Tether USD | `USD` |

Codes are upper-case. The list grows one row per coin; a node prices only the coins it
knows.

## Price

A node prices a market order in a stablecoin at exactly the price of the fiat it is pegged
to. One USDT is one USD, so the BTC price in USDT is the node's BTC price in USD, and the
premium applies as usual.

Pricing a stablecoin is the operator's choice:

- A node that quotes a coin publishes it in its [exchange rates
  event](./other_events.md#exchange-rates), next to the fiat currencies.
- A node that does not quote it refuses market-priced orders in it (`amt` = `0`). It may
  still accept fixed-price ones, if [`fiat_currencies_accepted`](./other_events.md#mostro-instance-status)
  allows the code.

Clients SHOULD offer market price for a stablecoin only when the node's exchange rates event
carries its code.

Amounts are whole units, as for fiat: `fa` is `100`, not `100.50`.

## Network

The same coin exists on several networks, and a transfer sent on the wrong one can be lost.
The order names the network in its payment method:

```json
["f", "USDT"],
["fa", "100"],
["pm", "USDT (Tron)", "USDT (Liquid)"]
```

Clients SHOULD write each method as `<CODE> (<Network>)`, with the network names below, so
that the same network is spelled the same way in every order and the order book can be
filtered by it. An order may list several networks; the buyer says in the chat which one
they will use.

| Code | Networks |
| --- | --- |
| `USDT` | Tron, Ethereum, Polygon, Solana, Liquid |
| `USDC` | Ethereum, Base, Solana, Polygon |
| `DOC` | Rootstock |

A network not listed here is written with its common name. The daemon does not check `pm`;
it publishes the methods as written.

## Settlement and disputes

The trade runs as with fiat: the buyer sends the coin, then sends [`fiat-sent`](./fiatsent.md),
and the seller releases once it has arrived.

- **The sender pays the network fee.** The seller must receive the full `fa` amount.
- **The buyer SHOULD paste the transaction id in the [peer chat](./chat.md).** On a public
  chain it is what a solver checks in a dispute: the network, the recipient, the amount and
  the confirmations.

## Compatibility

`f` is [NIP-69](https://github.com/nostr-protocol/nips/blob/master/69.md)'s tag, defined
there as an ISO 4217 code. A NIP-69 client that checks for one will likely skip orders in a
stablecoin.
