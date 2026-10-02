# Reputation bands

Reputation earned on one venue can be carried to a Mostro instance, but it
never travels as an exact figure. This page defines the grid it travels as,
which is a **protocol constant**: every issuer uses the same cuts, so a token
means the same thing wherever it is redeemed. Operators choose whom to trust,
not what a band means.

This page defines only the grid. The event that publishes an issuer's keys and
the messages that carry a band between instances are specified separately.

## Why bands and not numbers

"347 trades, 4.87 rating, 12.3 BTC" is unique in the source database. Any
design that publishes exact figures on the destination lets the source
operator — or a past counterparty who saw those numbers — recognise the
destination pubkey and link the two identities. Quantising to a band destroys
that fingerprint: a band is only useful if many people share it, which is also
why sparse cells are merged (see [K-anonymity](#k-anonymity-merge)).

## The grid

Reputation is quantised into a cell of a three-dimensional grid.

| Dimension (cell-id key) | Bands | Cell-id labels | Floor |
|---|---|---|---|
| Ratings received (`reviews`) | `[1,10)`, `[10,50)`, `[50,200)`, `[200,∞)` | `1-10`, `10-50`, `50-200`, `200+` | 1 / 10 / 50 / 200 |
| Average rating (`rating`) | `[1.0,4.0)`, `[4.0,4.5)`, `[4.5,5.0]` | `1.0-4.0`, `4.0-4.5`, `4.5+` | 1.0 / 4.0 / 4.5 |
| Account age (`age`) | `[0,6m)`, `[6m,24m)`, `[24m,∞)` | `0-6m`, `6m-24m`, `24m+` | 0d / 180d / 720d |

The grid therefore has 4 × 3 × 3 = **36 raw cells**.

All intervals are **half-open**, `[low, high)`, so every value falls in exactly
one band and two implementations cannot disagree on a boundary. The top rating
band is closed at `5.0` because that is the maximum a rating can take; every
other band is half-open. The rating dimension starts at `1.0` rather than `0`
because every rating is a number from 1 to 5 (see [User rating](./user_rating.md))
and eligibility requires at least one, so an eligible user's average is never
below 1; a floor of `0.0` would seed an average no set of valid reviews can
produce.

A **month** is exactly 30 days of 86400 seconds, so `6m` is 180 days and `24m`
is 720 days. Ages are computed from the user's **`since`**, the day-truncated
date of their first trade published in the [rating tag](./order_event.md) and
the [rating event](./user_rating.md), so a band cannot flip in the middle of a
day and an issuer cannot place a user in a different age band from the one
their public rating implies. On a Mostro issuer that date is the user's own,
never a date moved back by a previous import: age is computed from native
history only.

### The first dimension counts ratings received, not completed trades

On Mostro `total_reviews` only increments when a counterparty actually submits
a rating, and it is also the weight of the running average. Seeding a
destination from a trade count would publish a review count the user never
earned and give a single later rating the weight of hundreds. Completed trades
decide [eligibility](#eligibility) and nothing else.

### Floors

A band is carried to the destination as the **floor** of its interval. Nobody
receives more than they earned, and the destination shows a conservative lower
bound: a user with 214 ratings lands in `200+` and is seeded with 200.

## Cell id

A cell is named by a single UTF-8 string, with no spaces:

```text
reviews:<label>|rating:<label>|age:<label>
```

The three keys appear in exactly this order, each label is taken verbatim from
the table above, and the separator is `|` (U+007C). A label always names its
band's low bound and, except for the open-ended top band, its high bound.

```text
reviews:200+|rating:4.5+|age:24m+
reviews:10-50|rating:1.0-4.0|age:0-6m
```

This string is the key of the `cells` map in the keyset event and the value of
the `cell` field of a token. A parser MUST reject a cell id whose keys are
missing, repeated, out of order, or whose labels are not in the table.

## What a band means on the destination

Each dimension has its own merge rule, because they measure different things.
Seeds are applied to the user's existing values; nothing is overwritten.

| Dimension | Rule on the destination |
|---|---|
| Ratings received | **Added.** A rating on A and a rating on B are distinct reviews by distinct counterparties. |
| Average rating | **Weighted average**, weighted by review count per origin. Never summed, never maxed. |
| Account age | **Maximum**, never a sum: `since = min(since_local, now - age_floor)`. |

Age is a maximum because "since when does this person trade?" is a single
date. Time passes in parallel on every venue, so adding day counts would count
the same month twice: a user who opened orders on A and B on the same day has
30 days on both a month later, and importing A into B must leave B at 30, not
60.

### Worked example

A user with 10 days and 2 ratings at 4.5 on the destination imports a token for
`reviews:200+|rating:4.5+|age:24m+`. The seed is 200 ratings at 4.5 and 720
days:

| Field | Before | After |
|---|---|---|
| `total_reviews` | 2 | 202 |
| `total_rating` | 4.5 | 4.5 |
| `since` | 10 days ago | 720 days ago |

Counterparties see **4.5 · 202 reviews · trading for 2 years**. The average
does not move, because both sides average 4.5; the review count acts as an
anchor, so later ratings move it slowly, exactly as they would for a
long-standing user.

A destination MUST NOT publish any marker distinguishing seeded reputation from
native reputation. A counterparty sees one rating, one review count and one
date, and never needs to know where they came from.

## K-anonymity merge

A cell that almost nobody occupies is a fingerprint, not a crowd. An issuer
MUST therefore merge any cell holding fewer than **K = 50** users into a lower
cell before publishing its keyset, and MUST publish the resulting map so a
client can reproduce the decision instead of trusting it.

The population of a cell counts only users who meet the
[eligibility](#eligibility) rules. They are the only ones who can ever hold a
token, so they are the crowd a token hides in: a raw cell with 50 accounts of
which one is eligible would otherwise hand that one user a band nobody else
can show.

The merge is deterministic. Given the population of every cell:

```text
effective[c] = c            for every one of the 36 cells
population[c] = number of eligible users whose raw cell is c

loop:
    sparse = { c : effective[c] == c and c != lowest and population[c] < K }
    if sparse is empty: stop
    c = the smallest member of sparse in cell order
    t = effective[step_down(c)]
    population[t] += population[c]
    population[c] = 0
    for every raw cell r with effective[r] == c: effective[r] = t
```

with:

- **cell order** — the total order on cells given by the tuple of band indices
  `(reviews, age, rating)`, each counted from 0 at the lowest band, compared
  lexicographically. It exists only to make the loop deterministic.
- **`lowest`** — the cell that is the lowest band in all three dimensions,
  `reviews:1-10|rating:1.0-4.0|age:0-6m`.
- **`step_down(c)`** — lower `c` by one band in the first dimension that has a
  lower band, trying the dimensions in the fixed order `reviews`, `age`,
  `rating`. It is defined for every cell except `lowest`, which never enters
  `sparse`.

Two properties follow and make the result easy to check. `effective[r]` is
always a cell that has not been folded, so the target `t` is resolved in one
lookup and population is never added to a cell that no longer has a key. And
because the loop always takes the smallest sparse cell, every cell below `c`
in cell order that has not been folded already holds at least `K` users or is
`lowest`, so `t` never
needs to fold again on `c`'s account.

`lowest` keeps its key whatever it ends up holding, even fewer than `K` users
or none: it has nowhere lower to go, and otherwise nobody could ever migrate
from a young issuer. Every other cell either holds at least `K` users at the
end or has been folded. The cells left with a key are the keys of `cells`, the
folded ones are the keys of `merges` with `effective[r]` as their value, and
every one of the 36 raw cells is a key of exactly one of the two maps, so a
client always finds its own cell.

The map an issuer computes this way is already flat: every value of `merges`
is a key of `cells`. A client SHOULD still fold its own raw cell through
`merges` **transitively**, and a conforming implementation MUST terminate
rather than loop if an issuer publishes a cyclic map.

## Eligibility

An issuer MUST refuse to issue a token for a user who does not meet all of:

- at least **10 completed trades**;
- at least **1 rating received**;
- not banned;
- disputes lost below **10%** of completed trades.

Below this there is nothing worth migrating, and the floor of the lowest cell
is what an unseeded account already has.

## Values fixed by this page

The band cuts, the floors, `K`, and the eligibility minimums are protocol
constants. An issuer that changes any of them issues tokens that mean something
different from every other issuer's, which defeats the point of a shared grid.
They are versioned with the protocol, not configured per instance; the only
per-instance choice is which issuers a destination trusts.
