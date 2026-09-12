import hashlib, hmac, json

P  = 2**256 - 2**32 - 977
N  = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141
Gx = 0x79BE667EF9DCBBAC55A06295CE870B07029BFCDB2DCE28D959F2815B16F81798
Gy = 0x483ADA7726A3C4655DA4FBFC0E1108A8FD17B448A68554199C47D08FFB10D4B8
G  = (Gx, Gy)

def inv(a, m=P): return pow(a, m - 2, m)

def add(p1, p2):
    if p1 is None: return p2
    if p2 is None: return p1
    (x1, y1), (x2, y2) = p1, p2
    if x1 == x2 and (y1 + y2) % P == 0: return None
    if p1 == p2: lam = (3 * x1 * x1) * inv(2 * y1) % P
    else:        lam = (y2 - y1) * inv(x2 - x1) % P
    x3 = (lam * lam - x1 - x2) % P
    return (x3, (lam * (x1 - x3) - y1) % P)

def mul(k, p=G):
    k %= N; r = None
    while k:
        if k & 1: r = add(r, p)
        p = add(p, p); k >>= 1
    return r

def ser(p):  # 33-byte compressed SEC1
    x, y = p
    return bytes([2 + (y & 1)]) + x.to_bytes(32, 'big')

def xonly(p): return p[0].to_bytes(32, 'big')

def tagged(tag, msg):
    t = hashlib.sha256(tag.encode()).digest()
    return hashlib.sha256(t + t + msg).digest()

CHALLENGE_TAG = "mostro/reputation/challenge/v1"
TOKEN_TAG     = "mostro/reputation/token/v1"

def challenge(R, m): return int.from_bytes(tagged(CHALLENGE_TAG, ser(R) + m), 'big') % N
def token_id(m):     return tagged(TOKEN_TAG, m).hex()

# ── HKDF-SHA256 per §4.1 ─────────────────────────────────────────────────────
def hkdf(ikm, salt, info, L=32):
    prk = hmac.new(salt, ikm, hashlib.sha256).digest()
    okm, t, i = b"", b"", 1
    while len(okm) < L:
        t = hmac.new(prk, t + info + bytes([i]), hashlib.sha256).digest()
        okm += t; i += 1
    return okm[:L]

KEYSET_SALT = b"mostro/reputation/keyset/v1"

def derive_cell_key(issuer_sk_int, epoch, cell_id):
    ikm = issuer_sk_int.to_bytes(32, 'big')
    for counter in range(256):
        info = epoch.encode() + b"\x00" + cell_id.encode() + bytes([counter])
        x = int.from_bytes(hkdf(ikm, KEYSET_SALT, info), 'big')
        if 1 <= x < N: return x, counter
    raise AssertionError("unreachable")

# ── fixtures (fixed, never random) ───────────────────────────────────────────
ISSUER_SK = 0x0000000000000000000000000000000000000000000000000000000000000003
EPOCH     = "2026"
CELL      = "reviews:200+|rating:4.5+|age:24m+"
CELL_2    = "reviews:50-200|rating:4.0-4.5|age:6m-24m"
DEST_SK   = 0x00000000000000000000000000000000000000000000000000000000000000b0
NONCE     = bytes.fromhex("a1" * 32)
K0        = 0x0000000000000000000000000000000000000000000000000000000000000011
K1        = 0x0000000000000000000000000000000000000000000000000000000000000012
A0        = 0x0000000000000000000000000000000000000000000000000000000000000021
B0        = 0x0000000000000000000000000000000000000000000000000000000000000022
A1        = 0x0000000000000000000000000000000000000000000000000000000000000031
B1        = 0x0000000000000000000000000000000000000000000000000000000000000032
CLAUSE    = 1

issuer_pub = mul(ISSUER_SK)
dest_pub   = mul(DEST_SK)
m          = b"repv1:" + xonly(dest_pub) + NONCE
assert len(m) == 70

x_cell, counter = derive_cell_key(ISSUER_SK, EPOCH, CELL)
P_cell = mul(x_cell)
x_cell2, counter2 = derive_cell_key(ISSUER_SK, EPOCH, CELL_2)
P_cell2 = mul(x_cell2)
assert P_cell != P_cell2

# ── transcript ───────────────────────────────────────────────────────────────
Rp = [mul(K0), mul(K1)]
alpha, beta = [A0, A1], [B0, B1]
R, cp = [], []
for i in (0, 1):
    Ri = add(add(Rp[i], mul(alpha[i])), mul(beta[i], P_cell))
    R.append(Ri)
    cp.append((challenge(Ri, m) + beta[i]) % N)

b = CLAUSE
s_blind = (([K0, K1][b]) + cp[b] * x_cell) % N
s = (s_blind + alpha[b]) % N

# the check a destination runs
assert mul(s) == add(R[b], mul(challenge(R[b], m), P_cell)), "transcript does not verify"
# and it must fail against the other cell's key
assert mul(s) != add(R[b], mul(challenge(R[b], m), P_cell2))

h = lambda x: x.hex() if isinstance(x, bytes) else format(x, '064x')

token = {
    "issuer": xonly(issuer_pub).hex(),
    "epoch": EPOCH,
    "cell": CELL,
    "m": m.hex(),
    "r_point": ser(R[b]).hex(),
    "s": format(s, '064x'),
}

vectors = {
    "version": 1,
    "note": "Generated deterministically; every value is reproducible from the fixtures block.",
    "curve": "secp256k1",
    "tags": {"challenge": CHALLENGE_TAG, "token_id": TOKEN_TAG},
    "tagged_hash": [
        {"tag": CHALLENGE_TAG, "input": "", "output": tagged(CHALLENGE_TAG, b"").hex()},
        {"tag": CHALLENGE_TAG, "input": "00", "output": tagged(CHALLENGE_TAG, bytes([0])).hex()},
        {"tag": TOKEN_TAG, "input": "", "output": tagged(TOKEN_TAG, b"").hex()},
    ],
    "encoding": {
        "destination_identity_seckey": format(DEST_SK, '064x'),
        "destination_identity_xonly": xonly(dest_pub).hex(),
        "nonce": NONCE.hex(),
        "m": m.hex(),
        "m_len": len(m),
        "token_id": token_id(m),
        "cell_id": CELL,
        "point_compressed_example": ser(P_cell).hex(),
    },
    "cell_key_derivation": {
        "scheme": "HKDF-SHA256",
        "salt_ascii": KEYSET_SALT.decode(),
        "info": "epoch ‖ 0x00 ‖ cell_id ‖ counter",
        "issuer_seckey": format(ISSUER_SK, '064x'),
        "epoch": EPOCH,
        "cases": [
            {"cell": CELL, "counter": counter, "x_cell": format(x_cell, '064x'), "p_cell": ser(P_cell).hex()},
            {"cell": CELL_2, "counter": counter2, "x_cell": format(x_cell2, '064x'), "p_cell": ser(P_cell2).hex()},
        ],
    },
    "keyset": {
        "kind": 38388,
        "d": f"reputation-keyset:{EPOCH}",
        "epoch": EPOCH,
        "issuer": xonly(issuer_pub).hex(),
        "content": {
            "cells": {CELL: ser(P_cell).hex(), CELL_2: ser(P_cell2).hex()},
            "merges": {"reviews:200+|rating:4.0-4.5|age:6m-24m": CELL_2},
        },
    },
    "transcript": {
        "x_cell": format(x_cell, '064x'),
        "p_cell": ser(P_cell).hex(),
        "k": [format(K0, '064x'), format(K1, '064x')],
        "r_prime": [ser(Rp[0]).hex(), ser(Rp[1]).hex()],
        "alpha": [format(A0, '064x'), format(A1, '064x')],
        "beta": [format(B0, '064x'), format(B1, '064x')],
        "r_blinded": [ser(R[0]).hex(), ser(R[1]).hex()],
        "c_blinded": [format(cp[0], '064x'), format(cp[1], '064x')],
        "clause": b,
        "s_blinded": format(s_blind, '064x'),
        "s": format(s, '064x'),
    },
    "valid_token": token,
    "invalid_tokens": [],
}

def bad(name, why, **over):
    t = dict(token); t.update(over)
    vectors["invalid_tokens"].append({"name": name, "reason": why, "token": t})

bad("wrong_cell_key", "invalid-reputation-token: signed for a different cell's key", cell=CELL_2)
bad("wrong_identity", "reputation-identity-mismatch: `m` carries another identity",
    m=(b"repv1:" + xonly(mul(DEST_SK + 1)) + NONCE).hex())
bad("expired_epoch", "expired-reputation-keyset: epoch no longer accepted", epoch="2019")
bad("mauled_s", "invalid-reputation-token: s altered", s=format((s + 1) % N, '064x'))
bad("mauled_r", "invalid-reputation-token: R altered", r_point=ser(R[1 - b]).hex())
bad("zero_s", "invalid-reputation-token: s = 0 must be rejected, never accepted", s="00" * 32)
bad("s_overflow", "invalid-reputation-token: s = n + s must be rejected, never reduced mod n",
    s=format(N + s, 'x').rjust(64, '0'))

# every invalid token must actually fail the destination's check
def verify(t):
    try:
        if t["cell"] not in vectors["keyset"]["content"]["cells"]: return False
        if t["epoch"] != EPOCH: return False
        sv = int(t["s"], 16)
        if not (1 <= sv < N): return False
        rb = bytes.fromhex(t["r_point"]); mm = bytes.fromhex(t["m"])
        x = int.from_bytes(rb[1:], 'big')
        y2 = (pow(x, 3, P) + 7) % P; y = pow(y2, (P + 1) // 4, P)
        if y % 2 != rb[0] - 2: y = P - y
        Rpt = (x, y)
        pc = bytes.fromhex(vectors["keyset"]["content"]["cells"][t["cell"]])
        px = int.from_bytes(pc[1:], 'big')
        py2 = (pow(px, 3, P) + 7) % P; py = pow(py2, (P + 1) // 4, P)
        if py % 2 != pc[0] - 2: py = P - py
        return mul(sv) == add(Rpt, mul(challenge(Rpt, mm), (px, py)))
    except Exception:
        return False

assert verify(token), "valid token must verify"
for iv in vectors["invalid_tokens"]:
    assert not verify(iv["token"]), f"{iv['name']} must NOT verify"

import os
os.makedirs("src/vectors", exist_ok=True)
with open("src/vectors/reputation_v1.json", "w") as f:
    json.dump(vectors, f, indent=2, ensure_ascii=False)
    f.write("\n")
print("valid token verifies:", verify(token))
print("invalid tokens rejected:", all(not verify(i["token"]) for i in vectors["invalid_tokens"]))
print("cases:", len(vectors["invalid_tokens"]))
print("token_id:", vectors["encoding"]["token_id"])
