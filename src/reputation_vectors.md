# Reputation test vectors

Fixed vectors for the [portability wire format](./reputation_portability.md).
Every implementation — the daemon, both apps and the bot — MUST reproduce
these byte for byte before it is considered interoperable. Two independent
implementations that each look correct in isolation will still fail to talk to
each other if they disagree on one hash input, and these vectors are where
that disagreement surfaces.

The machine-readable file is
[`vectors/reputation_v1.json`](./vectors/reputation_v1.json).

Nothing here is random. Every value is derived from the fixtures below, so a
re-run reproduces the file exactly.

## Fixtures

| Input | Value |
|---|---|
| Issuer secret key | `0000000000000000000000000000000000000000000000000000000000000003` |
| Destination identity secret key | `00000000000000000000000000000000000000000000000000000000000000b0` |
| Destination identity (x-only) | `78a891aa2234a498896a193ed088a2b68fcae82788f506a0f3287432beb31db2` |
| Nonce | `a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1` |
| Epoch | `2026` |
| Cell | `reviews:200+|rating:4.5+|age:24m+` |
| Issuer nonce scalars `k_0`, `k_1` | `0000000000000000000000000000000000000000000000000000000000000011`, `0000000000000000000000000000000000000000000000000000000000000012` |
| Blinding factors `α_0`, `α_1` | `0000000000000000000000000000000000000000000000000000000000000021`, `0000000000000000000000000000000000000000000000000000000000000031` |
| Blinding factors `β_0`, `β_1` | `0000000000000000000000000000000000000000000000000000000000000022`, `0000000000000000000000000000000000000000000000000000000000000032` |
| Clause answered | `1` |

Low-valued scalars are used on purpose: they are trivially checkable by hand
and they make an implementation that accidentally reduces or pads differently
fail loudly.

## Tagged hashes

`H_tag(x) = SHA256(SHA256(tag) ‖ SHA256(tag) ‖ x)`.

| Tag | Input | Output |
|---|---|---|
| `mostro/reputation/challenge/v1` | `(empty)` | `ee81d9919f920195820f26e9bb4d885cf4cf1f81b55a589b8df8fe8add7ee790` |
| `mostro/reputation/challenge/v1` | `00` | `7c2d028606a6912f9a89ff3753f8d76252bebc45297e3ae57ce38137b4ca41c2` |
| `mostro/reputation/token/v1` | `(empty)` | `222632a20a442feacacec257eafc5fdd4333bf4ee5cea39645c059001ca87bb7` |

## The message and its token id

`m = "repv1:" ‖ destination_identity_xonly ‖ nonce`, **70 bytes**:

```text
m        = 72657076313a78a891aa2234a498896a193ed088a2b68fcae82788f506a0f3287432beb31db2a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1
token_id = 68d6322d10e4fbf3f362b416654c8623fc1bc9ef37d4e923b7ee4004ce1f955b
```

`token_id = H_"mostro/reputation/token/v1"(m)`, and it is the primary key of a
destination's redeemed set.

## Per-cell key derivation

HKDF-SHA256 with `salt = "mostro/reputation/keyset/v1"` and
`info = epoch ‖ 0x00 ‖ cell_id ‖ counter`, `L = 32`. The counter starts at
`0x00` and is incremented only if the output is `0` or `≥ n`.

Each issuer derives from its own secret, so these two cases exist to prove that
two implementations agree on the *scheme*, not to share a key:

| Cell | Counter | `x_cell` | `P_cell` |
|---|---|---|---|
| `reviews:200+|rating:4.5+|age:24m+` | `0` | `7d3a941fd26dc5b8d626178e7baf4eca0e01ecdbab5da8d8cf070edfd4cdc2e8` | `024e7403fcee10f849b635e334449a21e1b04b695db3e7e93651837631fd83c727` |
| `reviews:50-200|rating:4.0-4.5|age:6m-24m` | `0` | `c8ba3f8d9e093cd9f36fb8d5745da3a423a61e81f4769e85243658cb5df15050` | `02d0874c428b0810d8a8b4be42c88e4ac37081a1f52ede9d2a182d68f74c98e7bf` |

## Keyset

A minimal keyset event content, with one `merges` entry so an implementation
can exercise the transitive fold:

```json
{
  "cells": {
    "reviews:200+|rating:4.5+|age:24m+": "024e7403fcee10f849b635e334449a21e1b04b695db3e7e93651837631fd83c727",
    "reviews:50-200|rating:4.0-4.5|age:6m-24m": "02d0874c428b0810d8a8b4be42c88e4ac37081a1f52ede9d2a182d68f74c98e7bf"
  },
  "merges": {
    "reviews:200+|rating:4.0-4.5|age:6m-24m": "reviews:50-200|rating:4.0-4.5|age:6m-24m"
  }
}
```

## Blind Schnorr transcript

The full clause-variant exchange for the fixtures above.

| Step | Value |
|---|---|
| `R'_0` | `03defdea4cdb677750a420fee807eacf21eb9898ae79b9768766e4faa04a2d4a34` |
| `R'_1` | `025601570cb47f238d2b0286db4a990fa0f3ba28d1a319f5e7cf55c2a2444da7cc` |
| `R_0` (blinded) | `03c8aac7997194030a5db25e498f2292dbae02e47613424316133424b04db8c7a4` |
| `R_1` (blinded) | `02492787a0628d2874562487bdb7500c310bf4f6e0940c40ee2eeb54969f3509e0` |
| `c'_0` | `9202edaf3e1358c174f8d240bf545b45ecc2a1c07e048b8f75d8c082267db7bc` |
| `c'_1` | `8a341d86445060c8876c16a6e9e235f3a685131820bde4c26c9fd8ca149b516a` |
| clause `b` | `1` |
| `s'_b` | `89b6194d77ae5f905579767e11acff73dc8c72aec2bd2b8582e1f0572d516703` |
| `s` (unblinded) | `89b6194d77ae5f905579767e11acff73dc8c72aec2bd2b8582e1f0572d516734` |

An implementation that reaches this `s` from these inputs has the blinding,
the challenge and the unblinding all right.

## Valid token

```json
{
  "issuer": "f9308a019258c31049344f85f89d5229b531c845836f99b08601f113bce036f9",
  "epoch": "2026",
  "cell": "reviews:200+|rating:4.5+|age:24m+",
  "m": "72657076313a78a891aa2234a498896a193ed088a2b68fcae82788f506a0f3287432beb31db2a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1",
  "r_point": "02492787a0628d2874562487bdb7500c310bf4f6e0940c40ee2eeb54969f3509e0",
  "s": "89b6194d77ae5f905579767e11acff73dc8c72aec2bd2b8582e1f0572d516734"
}
```

It satisfies `s·G == R + H(R ‖ m)·P_cell` with the `P_cell` published above,
and MUST be accepted by a destination that trusts this issuer and accepts this
epoch.

## Invalid tokens

Every one of these MUST be rejected. The last two matter most: a parser that
reduces a scalar mod `n` instead of rejecting it turns the final row into a
valid token, which is a forgery.

| Case | Why it must be rejected |
|---|---|
| `wrong_cell_key` | invalid-reputation-token: signed for a different cell's key |
| `wrong_identity` | reputation-identity-mismatch: `m` carries another identity |
| `expired_epoch` | expired-reputation-keyset: epoch no longer accepted |
| `mauled_s` | invalid-reputation-token: s altered |
| `mauled_r` | invalid-reputation-token: R altered |
| `zero_s` | invalid-reputation-token: s = 0 must be rejected, never accepted |
| `s_overflow` | invalid-reputation-token: s = n + s must be rejected, never reduced mod n |

Each case in the JSON file is a complete token, so a test can feed it to the
same code path as the valid one and assert the rejection rather than
constructing the malformation itself.
