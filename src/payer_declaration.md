# Payer declaration and payment-account history

A buyer can commit, before reporting fiat as sent, to the fiat account it will pay from. Mostro receives only a hash of that account's details and answers the seller with aggregate counters: how many successful trades this buyer has already completed from this same account, with how many distinct counterparties, and over what period. The seller's client shows those counters before release.

The feature is opt-in per node and produces a **risk signal, never a block**. Mostro never releases, refuses or cancels a trade because of it; the seller always keeps the final decision.

## Why: the triangulation scam

Three participants:

- **Seller**: a legitimate Mostro user selling sats.
- **Attacker**: a malicious Mostro buyer.
- **Victim**: an unrelated third party, outside Mostro.

The attacker advertises something for sale outside Mostro (a phone, tickets, furniture) and the victim agrees to buy it. The attacker then takes a Mostro sell order and, instead of paying the seller, gives the **seller's** fiat payment details to the victim as "where to pay for the item". The victim pays the seller. The seller sees the right amount at the right time, releases the sats to the attacker, and is left holding a payment from somebody who never agreed to buy bitcoin, and who will sooner or later dispute it with their bank or report the seller for fraud.

### Why checking the sender is not enough

| Mitigation | Why it fails |
|---|---|
| A per-trade payment reference (`MOSTRO-8F21`) | The attacker tells the victim to use that reference. Knowing the reference proves nothing about *who* is paying. |
| The buyer declares the payer account and the seller checks the incoming payment matches it | The attacker collects the victim's account details *before* taking the order and declares those. The incoming payment matches perfectly. |

A declared account plus a matching sender therefore **cannot** prove that the sender is the Mostro buyer. It only proves that the buyer knew in advance which account the money would come from.

### Why history helps

A legitimate buyer reuses one or a handful of fiat accounts across many trades. A triangulation attacker must burn a *fresh* third-party account on almost every trade; that is the business model. So the useful question is not "does this bank account belong to this person?" but:

> Has **this Mostro user** already completed successful trades from **these same payment details**?

A positive answer (many successful trades, over months, against many different counterparties) is expensive to fake. A negative answer is exactly what a triangulation attack looks like, and it is also what every honest first-time user looks like. That is why the answer is a risk signal and never an automatic block.

The seller's client ends up with two independent checks before release:

1. **Sender match**: does the sender shown by the seller's bank match the details the buyer declared? Checked by the seller, by hand.
2. **Payment-account history**: how much successful history does *this buyer* have with *this payment identity*, and how experienced were the counterparties behind it? Computed privately by Mostro and returned as aggregate counters.

A sophisticated attacker can satisfy the first check. They cannot cheaply satisfy the second.

## Node support

The feature is disabled by default. A node that enables it advertises its policy on the instance status event (kind `38385`) with four tags, emitted **only when the feature is enabled** (see [Payer history policy tags](./other_events.md#payer-history-policy-tags)):

| Tag | Value |
|---|---|
| `payer_history_enabled` | `"true"` |
| `payer_declaration_required` | `"true"` or `"false"` |
| `payer_history_experienced_min_trades` | decimal string, e.g. `"5"` |
| `payer_history_experienced_min_days` | decimal string, e.g. `"30"` |

Clients treat missing tags as "feature disabled". On a node where the feature is disabled, `declare-payer` and `payment-history` answer `cant-do` with reason `invalid_action`, `fiat-sent` behaves exactly as described in [Fiat sent](./fiatsent.md), and no `payer-declared` or `payment-history` message is ever sent.

When `payer_declaration_required` is `"true"`, the node rejects `fiat-sent` with `payer_not_declared` until the buyer has declared a payer for the order. Even then the node only requires the declaration; it never decides anything from the history.

## Flow

The flow is the same whether the buyer is the maker or the taker; what matters is only which side is the buyer.

```text
 Buyer                          Mostro                             Seller
   |                               |                                  |
   |  canonicalise payer details,  |                                  |
   |  h = payment_hash             |                                  |
   |                               |                                  |
   |  declare-payer {h} ---------->|  validate, store (last wins)     |
   |<-- payer-declared {h} --------|--- payer-declared {h} ---------->|  seller learns h
   |                               |                                  |
   |  ===== plaintext payer details over the peer chat ============>  |  Mostro never sees it
   |                               |                                  |
   |  fiat-sent ------------------>|  optional: require a declaration |
   |<-- fiat-sent-ok --------------|--- fiat-sent-ok ---------------->|
   |                               |--- payment-history {stats} ----->|  push
   |                               |                                  |
   |                               |<-- payment-history (query) ------|  optional pull
   |                               |--- payment-history {stats} ----->|
   |                               |                                  |
   |                               |  seller checks the sender match  |
   |                               |  and reads the history           |
   |                               |<-- release (or dispute) ---------|
   |                               |                                  |
   |<-- purchase-completed --------|  success: history updated        |
```

1. The buyer's client canonicalises the payer details and computes `payment_hash` (see [Canonicalisation and hash](#canonicalisation-and-hash)).
2. The buyer sends `declare-payer` with the hash. Mostro stores it, acks it to the buyer and forwards it to the seller, both with the action `payer-declared`.
3. The buyer sends the **plaintext** payer details to the seller over the [peer-to-peer chat](./chat.md). The plaintext never goes to Mostro.
4. The seller's client recomputes the hash from the plaintext and compares it with the forwarded one.
5. The buyer sends `fiat-sent`. Right after `fiat-sent-ok`, Mostro pushes `payment-history` to the seller.
6. The seller may query `payment-history` again later (for example after restoring a session).
7. When the trade reaches `success` without a dispute, Mostro adds it to the buyer's history for that hash.

## Declaring the payer

The buyer sends `declare-payer` from its trade key, with the order id and a `payer_declaration` payload:

```json
[
  {
    "order": {
      "version": 2,
      "request_id": 981231,
      "trade_index": null,
      "id": "<Order Id>",
      "action": "declare-payer",
      "payload": {
        "payer_declaration": {
          "payment_hash": "ee06af92c95429e7cb0cf8428636199a71a01e32bab7a8526d226161f0de9903"
        }
      }
    }
  },
  "<trade-key signature of the sha256 hash of the serialized first element of content>",
  ["<identity pubkey>", "<identity proof signature>"]
]
```

`trade_index` is not used by this action and may be `null`. In full-privacy mode the signature and the identity proof are `null`, as for any other message.

The declaration window is `waiting-payment`, `waiting-buyer-invoice` and `active`. `active` is the normal moment; the two waiting statuses let a client collect the payer details on the same screen where it collects the invoice. A buyer may declare again inside the window: the last declaration wins and replaces the previous one. Once the order leaves the window (from `fiat-sent` on) the declaration is frozen, and a new `declare-payer` answers `not_allowed_by_status`. The freeze is atomic: a declaration that races a concurrent `fiat-sent` is either stored before the transition or refused.

### Mostro response

Mostro acks the declaration to the buyer, echoing the `request_id`:

```json
[
  {
    "order": {
      "version": 2,
      "request_id": 981231,
      "trade_index": null,
      "id": "<Order Id>",
      "action": "payer-declared",
      "payload": {
        "payer_declaration": {
          "payment_hash": "ee06af92c95429e7cb0cf8428636199a71a01e32bab7a8526d226161f0de9903"
        }
      }
    }
  },
  null,
  null
]
```

And forwards the same hash to the seller's trade key. The forward is unsolicited, so its `request_id` is `null`:

```json
[
  {
    "order": {
      "version": 2,
      "request_id": null,
      "trade_index": null,
      "id": "<Order Id>",
      "action": "payer-declared",
      "payload": {
        "payer_declaration": {
          "payment_hash": "ee06af92c95429e7cb0cf8428636199a71a01e32bab7a8526d226161f0de9903"
        }
      }
    }
  },
  null,
  null
]
```

The seller receives every re-declaration and keeps the last one. If the order has no seller yet when the buyer declares, only the ack is sent; the seller then learns the hash from the `payment-history` push at `fiat-sent` time, which echoes it.

`payer-declared` is a Mostro → user action. Mostro ignores it when a client sends it.

## Sending the plaintext to the seller

The buyer sends the payer details themselves to the seller over the [peer-to-peer chat](./chat.md), never to Mostro. Mostro cannot leak, log or be compelled to hand over what it never receives.

The seller's client needs the details in a form it can canonicalise exactly as the buyer's client did, so the buyer's client SHOULD include the canonical string (for example `EU|SEPA|DE89370400440532013000|ALICE SMITH`) alongside any human-readable rendering. The seller's client hashes the canonical string and compares the result with the `payment_hash` it received in `payer-declared` or `payment-history`. A mismatch means the buyer committed to one account and disclosed another; treat it like a sender mismatch.

## Reporting fiat sent

`fiat-sent` is unchanged on the wire (see [Fiat sent](./fiatsent.md)). Two things are added on a node with the feature enabled.

### Required declaration

When the node advertises `payer_declaration_required = "true"` and the buyer has not declared a payer for the order, Mostro answers `fiat-sent` with a `cant-do` and leaves the order in `active`:

```json
[
  {
    "order": {
      "version": 2,
      "request_id": 5521,
      "trade_index": null,
      "id": "<Order Id>",
      "action": "cant-do",
      "payload": {
        "cant_do": "payer_not_declared"
      }
    }
  },
  null,
  null
]
```

The `request_id` of the refused `fiat-sent` is echoed (`5521` here). The check runs after the usual `fiat-sent` status and buyer checks. The buyer's client sends `declare-payer` and then retries `fiat-sent`.

### History push

Right after `fiat-sent-ok`, Mostro pushes the history to the seller's trade key. The push is unsolicited, so its `request_id` is `null`:

```json
[
  {
    "order": {
      "version": 2,
      "request_id": null,
      "trade_index": null,
      "id": "<Order Id>",
      "action": "payment-history",
      "payload": {
        "payment_history": {
          "payment_hash": "ee06af92c95429e7cb0cf8428636199a71a01e32bab7a8526d226161f0de9903",
          "buyer_mode": "reputation",
          "successful_trades": 47,
          "distinct_counterparties": 29,
          "experienced_counterparties": 11,
          "first_success_at": 1762128000,
          "last_success_at": 1787654321
        }
      }
    }
  },
  null,
  null
]
```

The push is sent only when the buyer declared a payer. When there is no declaration, nothing is pushed; the seller's client treats "no `payment-history` by the time fiat is reported sent" as its own warning (see [Client contract](#client-contract)).

The push is queued only after the `fiat-sent` transition is stored. If Mostro cannot build the history at that point, it logs the failure and still completes `fiat-sent`; the seller can pull the history with a query.

## Querying the history

The seller can ask for the history again with `payment-history` and a `null` payload, signed by the seller's trade key:

```json
[
  {
    "order": {
      "version": 2,
      "request_id": 4412,
      "trade_index": null,
      "id": "<Order Id>",
      "action": "payment-history",
      "payload": null
    }
  },
  "<trade-key signature of the sha256 hash of the serialized first element of content>",
  ["<identity pubkey>", "<identity proof signature>"]
]
```

Mostro answers with the same `payment-history` message as the push, with the `request_id` echoed (`4412` here). The query takes no parameter beyond the order id: Mostro resolves the buyer and the hash from the order itself.

The query is accepted in `fiat-sent`, `dispute` and `settled-hold-invoice`. Each reply is a live snapshot, not a value frozen at `fiat-sent`: while the order stays queryable, another order of the same buyer from the same account may reach `success`, and a node restart with new experience thresholds re-evaluates `experienced_counterparties`, so a repeated query can return different numbers. Once the order reaches `success` the declaration has been consumed and the query answers `not_found`: the seller already received the push at `fiat-sent` time, and a post-success value would include the trade just completed.

## Status windows

| Action | Sender | Order status | Effect |
|---|---|---|---|
| `declare-payer` | buyer trade key | `waiting-payment`, `waiting-buyer-invoice`, `active` | declaration stored (last write wins); `payer-declared` to buyer and seller |
| `declare-payer` | buyer trade key | any other status, including `fiat-sent` and later | `cant-do not_allowed_by_status` (declaration frozen) |
| `payment-history` (push) | Mostro → seller | right after `fiat-sent-ok` | sent only when a declaration exists |
| `payment-history` (query) | seller trade key | `fiat-sent`, `dispute`, `settled-hold-invoice` | same object as the push |
| `payment-history` (query) | seller trade key | `success` | `cant-do not_found` (declaration consumed) |
| `payment-history` (query) | seller trade key | any other status | `cant-do not_allowed_by_status` |
| success | Mostro | `settled-hold-invoice` → `success` | history updated, declaration consumed |
| cleanup | Mostro | any other terminal status | declaration deleted |

## Refusal reasons

Each refusal is a `cant-do` whose payload carries the reason (see [Cant Do Reasons](./message_suggestions_for_actions.md#cant-do-reasons)). Checks run in the order listed.

**`declare-payer`**

| Reason | When |
|---|---|
| `invalid_action` | The feature is disabled on this node. |
| `not_found` | The order does not exist. |
| `invalid_pubkey` | The sender is not the buyer's trade key for this order. |
| `not_allowed_by_status` | The order is outside the declaration window, or left it while the declaration was being stored. |
| `invalid_payment_hash` | `payment_hash` is not exactly 64 lowercase hexadecimal characters. Uppercase hex is rejected. |

**`fiat-sent`**

| Reason | When |
|---|---|
| `payer_not_declared` | The node requires a declaration and the buyer has not sent `declare-payer` for this order. |

**`payment-history` (query)**

| Reason | When |
|---|---|
| `invalid_action` | The feature is disabled on this node. |
| `not_found` | The order does not exist. |
| `invalid_peer` | The sender is not the seller's trade key for this order. |
| `not_found` | The order is in `success`: the declaration has been consumed. |
| `not_allowed_by_status` | The order is in any other status than `fiat-sent`, `dispute` or `settled-hold-invoice`. |
| `not_found` | The buyer never declared a payer for this order. |

A message that fails the basic shape check (no order `id`, or for `declare-payer` a payload other than `payer_declaration`, or for `payment-history` a payload other than `null` or `payment_history`) is dropped without a reply, like any malformed message.

## Payloads

### `payer_declaration`

Carried by `declare-payer` (buyer → Mostro) and `payer-declared` (Mostro → buyer and seller).

| Field | Type | Meaning |
|---|---|---|
| `payment_hash` | string | `sha256("mostro-payer-v1\|" + canonical)` as 64 lowercase hex characters. See [Canonicalisation and hash](#canonicalisation-and-hash). |

### `payment_history`

Carried by `payment-history` from Mostro to the seller. Counters only ever include trades that reached `success` **without a dispute**, and never include the current order.

| Field | Type | Meaning |
|---|---|---|
| `payment_hash` | string | Echo of the hash the buyer committed to this order. |
| `buyer_mode` | string | `"reputation"`: the counters are meaningful. `"full_privacy"`: the buyer trades in full-privacy mode, the counters are always zero and the history is *unavailable*. |
| `successful_trades` | integer | Orders of this buyer, with this hash, that reached `success`. |
| `distinct_counterparties` | integer | Distinct sellers among those orders. |
| `experienced_counterparties` | integer | How many of those distinct sellers were already *experienced* (below) when their trade with this buyer and hash succeeded. |
| `first_success_at` | integer or `null` | Unix seconds of the first successful trade; `null` when `successful_trades` is `0`. |
| `last_success_at` | integer or `null` | Unix seconds of the last successful trade; `null` when `successful_trades` is `0`. |

The counters satisfy `experienced_counterparties ≤ distinct_counterparties ≤ successful_trades`.

A full-privacy buyer always gets this shape:

```json
{
  "payment_history": {
    "payment_hash": "ee06af92c95429e7cb0cf8428636199a71a01e32bab7a8526d226161f0de9903",
    "buyer_mode": "full_privacy",
    "successful_trades": 0,
    "distinct_counterparties": 0,
    "experienced_counterparties": 0,
    "first_success_at": null,
    "last_success_at": null
  }
}
```

Mostro has no cross-trade continuity for a full-privacy buyer, so it stores nothing for one and the counters can never grow. Clients MUST render this as "history unavailable", never as "new account".

**Forward compatibility.** Clients ignore unknown keys in `payment_history`. A `buyer_mode` value the client does not know (`mostro-core` reads it as `unknown`) is treated like `full_privacy`: history unavailable. A daemon never sends `unknown` on purpose.

### Experienced counterparty

A distinct seller counts as *experienced* for a buyer's history when, at the moment its trade with that buyer and hash reached `success`, the seller already had, counting only earlier trades:

- at least `payer_history_experienced_min_trades` successful, undisputed trades **with other buyers**; and
- at least `payer_history_experienced_min_days` days since the first such trade.

Trades with the same buyer never count, so a seller key that only ever trades with one buyer cannot make that buyer look established. A full-privacy seller never qualifies.

The thresholds are node policy, not protocol constants: read them from the info-event tags and do not hard-code them. When an operator changes them, the node re-evaluates every stored snapshot under the new thresholds at its next restart, so the advertised tags and the stored counters always describe the same policy. Within one policy the flag never flips back. It is a flat, one-hop signal: a seller's own history counters play no part in its qualification.

## Canonicalisation and hash

Mostro never sees the plaintext, so canonicalisation is a **client contract**. Every client MUST produce the same canonical string from the same account, or the same account yields different hashes and its history silently fragments. This section is normative.

### Canonical string

The canonical string is

```text
<COUNTRY>|<METHOD>|<field 1>|<field 2>|...
```

- `<COUNTRY>|<METHOD>` is the method prefix from the [registry](#method-registry). It keeps identical account numbers under different rails from colliding. Country codes are ISO 3166-1 alpha-2; the registry also uses `EU` (an ISO 3166-1 exceptionally reserved code) for rails that span the European Union. Currency codes, where a method needs one, are ISO 4217.
- The fields follow, in the fixed order the registry defines for the method, joined by `|` with no surrounding spaces.

Each field is normalised as follows:

1. Apply Unicode normalisation form NFKC.
2. Convert to uppercase with the Unicode default, locale-independent case mapping.
3. Then, by field kind:
   - **Identifier** fields (IBAN, CBU/CVU, PIX key, account number, tax id): remove every whitespace character, hyphen (`-`), dot (`.`) and slash (`/`).
   - **Name** fields: replace every run of whitespace with a single space (U+0020) and trim leading and trailing whitespace.
   - **E-mail** fields: remove every whitespace character only. Dots, hyphens and other punctuation are part of an e-mail address, so `A.B@EXAMPLE.COM` and `AB@EXAMPLE.COM` stay two different accounts.

Diacritics are kept: NFKC does not remove them, so `José` becomes `JOSÉ`, not `JOSE`. A field that is empty after normalisation, or that contains `|`, has no canonical form; the client MUST NOT declare it.

The canonical string MUST NOT include the order id, a trade key, a timestamp, a salt or anything else specific to one trade: that would make the hash unique per trade and defeat the history.

### Hash

```text
payment_hash = lowercase_hex( sha256( UTF-8("mostro-payer-v1|" + canonical) ) )
```

The fixed `mostro-payer-v1|` prefix is domain separation: it makes the value useless as an identifier outside this protocol. It is part of the input, not optional. The input has no trailing newline. The result is always 64 lowercase hexadecimal characters; Mostro rejects any other encoding with `invalid_payment_hash`, so two spellings of the same digest can never become two histories. `mostro-core` exposes this construction as `payment_hash()` and the prefix as `PAYMENT_HASH_DOMAIN`.

From a shell, for a canonical string you already have:

```bash
printf '%s' 'mostro-payer-v1|EU|SEPA|DE89370400440532013000|ALICE SMITH' | sha256sum
```

### Method registry

| Prefix | Fields, in order | Kind | Notes |
|---|---|---|---|
| `AR\|CVU` | CBU or CVU number; holder's CUIT/CUIL | identifier; identifier | The 22-digit account number, never an alias. The tax id is the 11-digit CUIT/CUIL. |
| `EU\|SEPA` | IBAN; account holder name | identifier; name | Covers SEPA credit transfers in any SEPA country. |
| `BR\|PIX` | PIX key | identifier, or e-mail when the key contains `@` | Any key type (CPF, CNPJ, phone, e-mail, random key) as registered. Phone keys keep the leading `+` and the country code. An e-mail key keeps its dots and hyphens (e-mail rule); every other key type follows the identifier rule. |

New methods are added to this table by a pull request to this book. An entry fixes the prefix, the fields, their order and their kind; once published, an entry never changes, because changing it would split every history built under it.

Methods that cannot show the seller who sent the money (cash, gift cards, vouchers) have no canonical form. Clients MUST NOT declare a payer for them and SHOULD tell the seller that sender verification is unavailable for the method.

A node that advertises `payer_declaration_required = "true"` refuses `fiat-sent` without a declaration, so a buyer paying with a method that has no canonical form could never report fiat as sent. Operators MUST NOT require declarations while accepting such methods; the setting is meant for markets that trade only over sender-verifiable rails listed in this registry. Clients SHOULD warn a buyer before taking such an order on a node that requires declarations.

### Test vectors

Normalisation:

| Input | Canonical string |
|---|---|
| `AR`, `CVU`, `0000003100012345678901`, `27-12345678-9` | `AR\|CVU\|0000003100012345678901\|27123456789` |
| `EU`, `SEPA`, `de89 3704 0044 0532 0130 00`, `"  Alice   Smith "` (quotes added to show the spaces) | `EU\|SEPA\|DE89370400440532013000\|ALICE SMITH` |
| `BR`, `PIX`, `+55 11 99999-8888` | `BR\|PIX\|+5511999998888` |
| `BR`, `PIX`, `Alice.Smith@Example.com ` | `BR\|PIX\|ALICE.SMITH@EXAMPLE.COM` |
| `EU`, `SEPA`, `ES91 2100 0418 4502 0005 1332`, `José  García` | `EU\|SEPA\|ES9121000418450200051332\|JOSÉ GARCÍA` |

`DE89 3704 0044 0532 0130 00` and `DE89370400440532013000` canonicalise to the same string.

Hashes, `sha256("mostro-payer-v1|" + canonical)` as lowercase hex:

```text
AR|CVU|0000003100012345678901|27123456789
df82c620ee9df8f7ad068e3bd771a707d9c0dfcfdef2c34a3fb8e6cdda3c9a2f

EU|SEPA|DE89370400440532013000|ALICE SMITH
ee06af92c95429e7cb0cf8428636199a71a01e32bab7a8526d226161f0de9903

BR|PIX|+5511999998888
77801d9713f5a93e133c8b507429b69ce89ae392e5c37c4777730e2153f08b78

BR|PIX|ALICE.SMITH@EXAMPLE.COM
bc10fa5b6d8914c550e3db8e5b3e461720646992e046d2fa5af097e769c323a2

EU|SEPA|ES9121000418450200051332|JOSÉ GARCÍA
91863709cf207cf042cece0cc4673241e4e0f321a39a327c16f05a9d0d231ebd
```

The last vector checks the UTF-8 and NFKC handling: `É` is the single code point U+00C9 (bytes `c3 89`). A name typed in decomposed form (`E` followed by U+0301) has different bytes, so hashing it without normalisation would give a different hash. NFKC maps both forms to U+00C9, which is why it comes first: a conforming client gets the same canonical string, and the same hash, from either form.

A client that forgets the prefix gets `7838e67266dea11dbca22c155c52ceb344f36658a7c7206f45af189c4ea2a99a` for the SEPA string instead of `ee06af92…9903`, and builds a history nobody else can match.

### Reference normalisation

A non-normative Python sketch of the rules above, which reproduces the vectors:

```python
import hashlib
import re
import unicodedata

DOMAIN = "mostro-payer-v1|"


def _base(value: str) -> str:
    return unicodedata.normalize("NFKC", value).upper()


def identifier(value: str) -> str:
    return re.sub(r"[\s\-./]", "", _base(value))


def name(value: str) -> str:
    return re.sub(r"\s+", " ", _base(value)).strip()


def email(value: str) -> str:
    return re.sub(r"\s", "", _base(value))


def canonical(country: str, method: str, *fields: str) -> str:
    parts = [country, method, *fields]
    if any(p == "" or "|" in p for p in parts):
        raise ValueError("this payer has no canonical form")
    return "|".join(parts)


def payment_hash(canonical_string: str) -> str:
    return hashlib.sha256((DOMAIN + canonical_string).encode("utf-8")).hexdigest()


c = canonical("EU", "SEPA", identifier("de89 3704 0044 0532 0130 00"), name("  Alice   Smith "))
assert payment_hash(c) == "ee06af92c95429e7cb0cf8428636199a71a01e32bab7a8526d226161f0de9903"
```

### The hash is not a secret

An account number plus a name is guessable by anyone who already knows the account, so the hash is brute-forceable from a candidate list. It is an identifier that must never be published, not a secret. That is why it only ever travels inside encrypted messages between the parties and Mostro. The node database is its only server-side copy. The two clients of the trade keep it as well, the buyer's for the declaration and the seller's to compare with the plaintext; they SHOULD keep it only as long as they keep the order, and never send it anywhere else.

## Client contract

Normative for clients that support the feature, which they detect through the info-event tags.

**Buyer side**

1. When the order is taken and the node advertises `payer_history_enabled = "true"`, show a "payment sender" form for the payment method in use, and explain why it is asked for.
2. Canonicalise and hash the details, and send `declare-payer`. Keep the plaintext locally.
3. Send the plaintext to the seller over the peer chat.
4. Before sending `fiat-sent`, ask the user to confirm: *"Did you send the payment from the account declared for this trade?"*
5. If `fiat-sent` answers `payer_not_declared`, go back to step 1.

**Seller side**

1. On `payer-declared`, store the hash for the order. When the plaintext arrives from the buyer, recompute the hash; if it differs, show a hard warning.
2. On `payment-history` (push or reply), show two independent blocks: *Sender match* (a manual confirmation by the seller) and *Payment-account history*.
3. In the history block, show `experienced_counterparties` next to the raw counters, for example *"`N` of the buyer's past counterparties were already experienced on this node when they traded with them"*, with the thresholds read from the info-event tags.
4. Never auto-release and never auto-refuse. The release screen shows both blocks above the release and dispute buttons.
5. The `payment-history` push is a separate message and may arrive late, or not at all (a relay can drop it, and Mostro skips it if it cannot build the history). If it has not arrived once fiat is reported sent, send the `payment-history` query. Show *"Buyer did not declare a payment sender"* as its own warning only when that query answers `not_found` while the order is still `fiat-sent`. Until then, show the history as pending.

### Suggested tiers

Client policy, not protocol. Clients may use other thresholds, but should weight all four signals.

| Tier | Condition (all of) | Wording |
|---|---|---|
| Established | `successful_trades ≥ 5`, `distinct_counterparties ≥ 3`, `experienced_counterparties ≥ 1`, and `first_success_at ≤ now − 30 days` (an age check on the timestamp, not a check on its raw value) | "Established payment account" |
| Limited | `successful_trades ≥ 1` and not Established | "Limited payment history" |
| New | `buyer_mode = "reputation"` and `successful_trades = 0` | "No previous successful trades with this account" |
| Unavailable | `buyer_mode = "full_privacy"` (or a mode the client does not know) | "History unavailable (buyer trades in full-privacy mode)" |

Requiring at least one experienced counterparty keeps a fresh cluster of colluding keys from reaching Established by trading only among themselves: at least one counterparty must already have had real history with third parties at trade time.

**Forbidden wording.** Clients MUST NOT describe the result as "verified owner", "verified account" or "trusted account". Mostro does not verify ownership of anything; the history says how an account has been used on this node, not who owns it.

## Privacy

### Who learns what

| Party | Learns | Never learns |
|---|---|---|
| Seller (through Mostro) | the hash the buyer committed to **this order**; the aggregate counters for **this** buyer and hash, including how many past counterparties met the node's experience policy, as a bare count; whether the buyer trades in full-privacy mode (already visible today through `Peer.reputation == null`) | the buyer's identity key, other trade keys, other order ids, which sellers were the counterparties, any single counterparty's qualification, any hash other than the one the buyer chose to commit to this order |
| Buyer | nothing new about the seller | |
| Mostro node | the association between the buyer's identity key and the hash, with the counters; keyed hashes of the counterparties; per-counterparty qualification snapshots derived from orders the node already holds | the plaintext payer details |
| Public relays | nothing | everything in this feature |

Payer details, hashes and history are never published on Nostr. The four info-event tags only describe node policy.

### Why there is no oracle

- The only query, `payment-history`, takes no parameter beyond the order id. The buyer and the hash are resolved by Mostro from the order. A seller cannot ask about a hash the buyer did not commit to this order, nor about a user who is not its counterparty in this order.
- Repeating the query only returns a fresher snapshot of the same buyer and hash; it leaks nothing about anyone else.
- The seller already has the plaintext, because the buyer sent it. Learning its hash is not new information.
- The seller cannot tell "this buyer used account X before" from "somebody used account X before" across users: the counters are for the buyer it is trading with now, keyed by that buyer's identity, so a victim's own history on the same account is never attributed to an attacker.
- There is no "are these two keys the same user?" primitive.
- Counterparties are stored as keyed hashes; even an exported database does not list which sellers a buyer dealt with without the node's secret key.
- The experienced count is returned only as an aggregate, never per counterparty, so a seller cannot use it to probe whether some third-party seller is experienced.

### Limits

- History is per node. It does not travel between Mostro instances.
- A buyer in full-privacy mode never builds history. This is deliberate: storing rows that can never be matched again would only link a hash to a trade key.
- A trade that went through a dispute never counts, whatever the outcome.
- A compromised node database reveals which identity keys used which hashes, and the hashes can be brute-forced by anyone who already knows a candidate account. This is the same trust boundary as the rest of the data a node holds about its trades.
