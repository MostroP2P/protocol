# Bitcoin on other networks

Mostro can swap Lightning sats for bitcoin on another network. The asset is the order's
currency, in the `f` tag where a fiat order has its ISO 4217 code, and each asset is pegged
1:1 to bitcoin itself:

| Code | Asset | Network |
| --- | --- | --- |
| `BTC` | bitcoin | Bitcoin, on-chain |
| `LBTC` | L-BTC | Liquid |
| `RBTC` | RBTC | Rootstock |

Codes are upper-case.

## Amounts are in sats

For these codes `fa` counts **sats of the asset**, not whole units: `fa` = `100000` is
0.001 RBTC. A range order's two `fa` values are sats too.

```json
["k", "sell"],
["f", "RBTC"],
["amt", "0"],
["fa", "100000"],
["pm", "RBTC (Rootstock)"],
["premium", "1"]
```

- Clients MUST read and write `fa` in sats for these codes. A client that shows `fa` with the
  code as if it were a whole unit would show "100000 RBTC" for 0.001 RBTC.
- Clients SHOULD let users enter and see these amounts in sats, or convert them to the asset
  with 8 decimals.
- RBTC has 18 decimals on Rootstock. A client rounds an RBTC amount to the sat (10⁻⁸ RBTC)
  before it builds the order.

## Price

A node publishes each of these codes that it quotes at `100000000` in its [exchange rates
event](./other_events.md#exchange-rates): 10⁸ units per BTC, a unit being a sat. An operator
may quote any subset of the three. A market-priced order (`amt` = `0`) is therefore `fa`
sats, less the premium:

```text
sats = fa × (1 − premium / 100), rounded to the nearest sat
```

The order above is 99,000 sats: the seller locks 99,000 Lightning sats and receives 100,000
sats of RBTC. The premium is the price of the swap. A fixed-price order (`amt` > `0`) states
both amounts, as for fiat.

Pricing these codes is the operator's choice, as for [stablecoins](./stablecoins.md#price):

- A node that quotes a code publishes it in its exchange rates event.
- A node that does not quote it refuses market-priced orders in it. It may still accept
  fixed-price ones, if [`fiat_currencies_accepted`](./other_events.md#mostro-instance-status)
  allows the code.
- A node with [MostroP2P/mostro#1095](https://github.com/MostroP2P/mostro/pull/1095) checks
  a market order against its sats limits (`min_order_amount`, `max_order_amount`) with the
  premium applied. An earlier node checks them without it, so an order close to a limit can
  pass creation and be taken outside it.

Clients SHOULD offer market price for these codes only when the node's exchange rates event
carries the code.

The exchange rates content keeps its outer `"BTC"` key, so on-chain bitcoin appears as an
inner `"BTC"` entry. Only `100000000` there means the node quotes it. A value of `1` is a
source's bitcoin-to-bitcoin self-rate, not a quote, and clients SHOULD ignore it.

## Network

The code names the network, so an order needs no network choice. Clients SHOULD still write
the payment method as `<CODE> (<Network>)`, as for stablecoins, so that the order book reads
the same way for every pegged code:

| Code | Payment method |
| --- | --- |
| `BTC` | `BTC (On-chain)` |
| `LBTC` | `LBTC (Liquid)` |
| `RBTC` | `RBTC (Rootstock)` |

The daemon does not check `pm`; it publishes the methods as written.

## Settlement and disputes

The trade runs as with fiat: the buyer sends the asset, then sends
[`fiat-sent`](./fiatsent.md), and the seller releases once it has arrived.

- **The seller releases only after the transfer confirms**, never on zero confirmations.
- **The window is the node's escrow deadline, not the hold invoice's full life.** Near the
  hold invoice's expiry the node cancels an `active` trade, or opens a dispute on a
  `fiat-sent` one. With the daemon's default settings that is about 120 blocks after the
  seller's payment is accepted. Liquid and Rootstock confirm in minutes. On-chain BTC sent
  with a low fee can stay unconfirmed past the deadline, so the sender SHOULD pay a fee that
  confirms within a few hours.
- **The sender pays the network fee.** The receiver must get the full `fa` sats.
- **The buyer SHOULD paste the transaction id in the [peer chat](./chat.md).**
  - BTC and RBTC transfers are public, so a solver checks them in an explorer.
  - Liquid transactions are confidential, with amounts and assets blinded. The sender SHOULD
    share the unblinded link, which carries the blinding data, so that a solver can see the
    amount and the asset.

## Compatibility

`f` is [NIP-69](https://github.com/nostr-protocol/nips/blob/master/69.md)'s tag, defined
there as an ISO 4217 code. A NIP-69 client that checks for one will likely skip these orders.

A client that accepts any code but does not know these would read `fa` as whole units. Any
client can already publish a fixed-price order in these codes, so the sats rule applies
whether or not a node quotes them.
