//! Generates `src/vectors/reputation_v1.json`, the test vectors of the
//! [reputation attestation](../../src/reputation_attestation.md).
//!
//! It depends on `nostr` alone, not on mostro-core, so the implementations
//! that test against the file are checked against an independent build of
//! the events. Keys come from fixed labels and signatures are BIP-340 with
//! no auxiliary randomness, so a rerun reproduces the file byte for byte:
//!
//! ```text
//! cargo run --manifest-path tools/reputation-vectors/Cargo.toml > src/vectors/reputation_v1.json
//! ```

use nostr::prelude::{Event, EventId, Keys, Kind, PublicKey, SecretKey, Signature, Tag, Timestamp, UnsignedEvent};
use secp256k1::{Keypair, SECP256K1};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const KIND: u16 = 38388;
const DAY: u64 = 86_400;
const CLOCK_SKEW: u64 = 300;
const MAX_LIFETIME: u64 = 7 * DAY;
const REBIND_MAX_LIFETIME: u64 = 3_600;

/// When the valid attestation is signed: a UTC day start.
const CREATED: u64 = 1_790_899_200;
/// The clock every check runs at.
const NOW: u64 = CREATED + 3_600;
/// First completed trade on the source: 2023-10-02, a UTC day start.
const SINCE: u64 = 1_696_204_800;
const SUBJECT: &str = "64f1c9f4e3a2b1c0d9e8f7a6";

fn keys(label: &str) -> Keys {
    let digest = Sha256::digest(format!("mostro/reputation/vectors/v1/{label}").as_bytes());
    Keys::new(SecretKey::from_slice(&digest).expect("a sha256 digest is a valid key"))
}

fn to_tags(raw: &[Vec<String>]) -> Vec<Tag> {
    raw.iter()
        .map(|t| Tag::parse(t.clone()).expect("non-empty tag"))
        .collect()
}

/// Signs with no auxiliary randomness so the output is reproducible.
fn sign(keys: &Keys, created_at: u64, kind: u16, raw: &[Vec<String>]) -> Event {
    let created_at = Timestamp::from(created_at);
    let kind = Kind::from(kind);
    let tags = to_tags(raw);
    let unsigned = UnsignedEvent::new(keys.public_key(), created_at, kind, tags.clone(), "");
    let id: EventId = unsigned.compute_id();
    let keypair = Keypair::from_secret_key(SECP256K1, keys.secret_key());
    let sig = SECP256K1.sign_schnorr_no_aux_rand(&id.to_bytes(), &keypair);
    Event::new(id, keys.public_key(), created_at, kind, tags, "", Signature::from_byte_array(sig.to_byte_array()))
}

/// The rounding of section 5.1: `clamp(round(avg × 100), 100, 500) / 100`,
/// half away from zero, written with exactly two decimals.
fn round_rating(avg: f64) -> String {
    let hundredths = (avg * 100.0).round().clamp(100.0, 500.0) as u64;
    format!("{}.{:02}", hundredths / 100, hundredths % 100)
}

fn s(v: &str) -> String {
    v.to_string()
}

/// The tags of an attestation, in the order issuers write them.
fn attestation_tags(destination: &PublicKey, reviews: &str, rating: &str, expiration: u64) -> Vec<Vec<String>> {
    vec![
        vec![s("p"), destination.to_hex()],
        vec![s("subject"), s(SUBJECT)],
        vec![s("reviews"), s(reviews)],
        vec![s("rating"), s(rating)],
        vec![s("since"), SINCE.to_string()],
        vec![s("expiration"), expiration.to_string()],
        vec![s("z"), s("reputation-attestation")],
    ]
}

fn set(raw: &mut [Vec<String>], name: &str, value: &str) {
    raw.iter_mut().find(|t| t[0] == name).expect("tag present")[1] = value.into();
}

fn event_json(event: &Event) -> Value {
    serde_json::from_str(&event.as_json()).expect("event serialises")
}

fn case(name: &str, why: &str, event: &Event, reason: &str) -> Value {
    json!({ "name": name, "description": why, "event": event_json(event), "reason": reason })
}

struct Fixture {
    issuer: Keys,
    other_issuer: Keys,
    own_issuer: Keys,
    identity: Keys,
    other_identity: Keys,
    new_identity: Keys,
}

fn attestations(f: &Fixture) -> (Value, Vec<Value>) {
    let base = attestation_tags(&f.identity.public_key(), "214", "4.87", CREATED + MAX_LIFETIME);
    let valid = sign(&f.issuer, CREATED, KIND, &base);
    let valid_vector = json!({
        "description": "Signed by the trusted issuer for the proven identity, inside its lifetime.",
        "event": event_json(&valid),
        "json": valid.as_json(),
        "id": valid.id.to_hex(),
        "expect": {
            "issuer_name": "issuer-a",
            "issuer_key": f.issuer.public_key().to_hex(),
            "destination": f.identity.public_key().to_hex(),
            "subject": SUBJECT,
            "reviews": 214,
            "rating": "4.87",
            "since": SINCE,
            "created_at": CREATED,
            "expiration": CREATED + MAX_LIFETIME,
        }
    });

    let invalid_attestation = "invalid_reputation_attestation";
    let bad = |name: &str, why: &str, mutate: &dyn Fn(&mut Vec<Vec<String>>)| {
        let mut raw = base.clone();
        mutate(&mut raw);
        case(name, why, &sign(&f.issuer, CREATED, KIND, &raw), invalid_attestation)
    };

    let mut invalid = vec![
        case("wrong_kind", "Kind 38384 instead of 38388.", &sign(&f.issuer, CREATED, 38384, &base), invalid_attestation),
        bad("missing_z", "No `z` tag.", &|r| r.retain(|t| t[0] != "z")),
        bad(
            "rebind_z",
            "`z` is `reputation-rebind`: a rebind authorisation is never an attestation.",
            &|r| set(r, "z", "reputation-rebind"),
        ),
        bad("missing_p", "No `p` tag.", &|r| r.retain(|t| t[0] != "p")),
        bad("missing_since", "No `since` tag.", &|r| r.retain(|t| t[0] != "since")),
        bad("repeated_reviews", "`reviews` appears twice.", &|r| r.push(vec![s("reviews"), s("214")])),
        bad("uppercase_p", "`p` is not lowercase hex.", &|r| {
            let upper = r[0][1].to_uppercase();
            set(r, "p", &upper)
        }),
        bad("subject_empty", "`subject` is empty.", &|r| set(r, "subject", "")),
        bad("subject_bad_characters", "`subject` holds a character outside `[0-9A-Za-z_-]`.", &|r| {
            set(r, "subject", "user 42")
        }),
        bad("subject_too_long", "`subject` is 65 characters long.", &|r| set(r, "subject", &"a".repeat(65))),
        bad("rating_above_range", "`rating` is 5.01.", &|r| set(r, "rating", "5.01")),
        bad("rating_below_range", "`rating` is 0.99.", &|r| set(r, "rating", "0.99")),
        bad("rating_one_decimal", "`rating` has one decimal.", &|r| set(r, "rating", "4.8")),
        bad("rating_three_decimals", "`rating` has three decimals.", &|r| set(r, "rating", "4.870")),
        bad("rating_not_a_number", "`rating` is `4,87`.", &|r| set(r, "rating", "4,87")),
        bad("reviews_below_floor", "`reviews` is 4.", &|r| set(r, "reviews", "4")),
        bad("reviews_leading_zero", "`reviews` is written `0214`.", &|r| set(r, "reviews", "0214")),
        bad("reviews_signed", "`reviews` is written `+214`.", &|r| set(r, "reviews", "+214")),
        bad("reviews_overflow", "`reviews` is 4294967296.", &|r| set(r, "reviews", "4294967296")),
        bad("since_not_day_aligned", "`since` is not a multiple of 86400.", &|r| {
            set(r, "since", &(SINCE + 1).to_string())
        }),
        bad("since_before_2020", "`since` is 2019-12-31.", &|r| {
            set(r, "since", &(1_577_836_800 - DAY).to_string())
        }),
        bad("since_after_created_at", "`since` is the day after `created_at`.", &|r| {
            set(r, "since", &(CREATED + DAY).to_string())
        }),
        bad("expiration_not_after_created_at", "`expiration` equals `created_at`.", &|r| {
            set(r, "expiration", &CREATED.to_string())
        }),
        bad("lifetime_over_cap", "`expiration - created_at` is one second over the 7-day cap.", &|r| {
            set(r, "expiration", &(CREATED + MAX_LIFETIME + 1).to_string())
        }),
    ];

    // Clock checks: one second past the 300-second skew on either side.
    let future = NOW + CLOCK_SKEW + 1;
    let raw = attestation_tags(&f.identity.public_key(), "214", "4.87", future + DAY);
    invalid.push(case(
        "created_in_the_future",
        "`created_at` is 301 seconds after `now`.",
        &sign(&f.issuer, future, KIND, &raw),
        invalid_attestation,
    ));
    let expired_at = NOW - CLOCK_SKEW - 1;
    let raw = attestation_tags(&f.identity.public_key(), "214", "4.87", expired_at);
    invalid.push(case(
        "expired",
        "`expiration` is 301 seconds before `now`.",
        &sign(&f.issuer, expired_at - DAY, KIND, &raw),
        "expired_reputation_attestation",
    ));

    let mut mauled_sig = valid.clone();
    let mut sig = mauled_sig.sig.to_bytes();
    sig[63] ^= 0x01;
    mauled_sig.sig = Signature::from_byte_array(sig);
    invalid.push(case("mauled_signature", "Last byte of `sig` flipped.", &mauled_sig, invalid_attestation));
    let mut mauled_id = valid.clone();
    mauled_id.content = s("x");
    invalid.push(case(
        "id_mismatch",
        "`content` changed after signing, so `id` no longer matches.",
        &mauled_id,
        invalid_attestation,
    ));

    // Well-formed events the destination refuses from its own context.
    let for_other = attestation_tags(&f.other_identity.public_key(), "214", "4.87", CREATED + MAX_LIFETIME);
    invalid.push(case(
        "identity_mismatch",
        "Well formed, but names another identity than the one the request proves.",
        &sign(&f.issuer, CREATED, KIND, &for_other),
        "reputation_identity_mismatch",
    ));
    invalid.push(case(
        "untrusted_issuer",
        "Well formed, but signed by a key in no trust-list entry.",
        &sign(&f.other_issuer, CREATED, KIND, &base),
        "untrusted_reputation_issuer",
    ));
    invalid.push(case(
        "own_issuer_key",
        "Well formed, but signed by the destination's own issuer key, which its trust list also holds.",
        &sign(&f.own_issuer, CREATED, KIND, &base),
        "untrusted_reputation_issuer",
    ));

    (valid_vector, invalid)
}

/// Attestations that pass: the skew and cap boundaries, the floor values
/// and an unknown tag.
fn accepted_edges(f: &Fixture) -> Vec<Value> {
    let edge = |name: &str, why: &str, created: u64, expiration: u64, extra: Option<Vec<String>>| {
        let mut raw = attestation_tags(&f.identity.public_key(), "5", "1.00", expiration);
        raw.extend(extra);
        json!({ "name": name, "description": why, "event": event_json(&sign(&f.issuer, created, KIND, &raw)) })
    };
    vec![
        edge(
            "created_at_skew_limit",
            "`created_at` is exactly 300 seconds after `now`.",
            NOW + CLOCK_SKEW,
            NOW + CLOCK_SKEW + DAY,
            None,
        ),
        edge(
            "expiration_skew_limit",
            "`expiration` is exactly 300 seconds before `now`.",
            NOW - CLOCK_SKEW - DAY,
            NOW - CLOCK_SKEW,
            None,
        ),
        edge(
            "lifetime_at_cap",
            "`expiration - created_at` is exactly the 7-day cap; `reviews` and `rating` sit at their floors.",
            CREATED,
            CREATED + MAX_LIFETIME,
            None,
        ),
        edge(
            "unknown_tag_ignored",
            "Carries a tag the rules do not name, which a receiver ignores.",
            CREATED,
            CREATED + MAX_LIFETIME,
            Some(vec![s("client"), s("lnp2pbot")]),
        ),
    ]
}

fn rebinds(f: &Fixture) -> Value {
    let raw = |new: &PublicKey, issuer: &PublicKey, expiration: u64, z: &str| -> Vec<Vec<String>> {
        vec![
            vec![s("p"), new.to_hex()],
            vec![s("issuer"), issuer.to_hex()],
            vec![s("expiration"), expiration.to_string()],
            vec![s("z"), s(z)],
        ]
    };
    let created = NOW - 600;
    let (issuer, new) = (f.issuer.public_key(), f.new_identity.public_key());
    let rebind = "reputation-rebind";
    let valid = sign(&f.identity, created, KIND, &raw(&new, &issuer, created + REBIND_MAX_LIFETIME, rebind));
    let reason = "invalid_reputation_rebind";
    let mut no_issuer = raw(&new, &issuer, created + 600, rebind);
    no_issuer.retain(|t| t[0] != "issuer");
    let mut mauled = valid.clone();
    mauled.created_at = Timestamp::from(created + 1);
    let invalid = vec![
        case(
            "signed_by_other_identity",
            "Valid signature, but not by the identity the account is bound to.",
            &sign(&f.other_identity, created, KIND, &raw(&new, &issuer, created + 600, rebind)),
            reason,
        ),
        case(
            "other_issuer",
            "`issuer` names another issuer, so it cannot be replayed here.",
            &sign(&f.identity, created, KIND, &raw(&new, &f.other_issuer.public_key(), created + 600, rebind)),
            reason,
        ),
        case(
            "lifetime_over_an_hour",
            "`expiration - created_at` is 3601 seconds.",
            &sign(&f.identity, created, KIND, &raw(&new, &issuer, created + REBIND_MAX_LIFETIME + 1, rebind)),
            reason,
        ),
        case(
            "expired",
            "`expiration` is 301 seconds before `now`.",
            &sign(&f.identity, NOW - 1_200, KIND, &raw(&new, &issuer, NOW - CLOCK_SKEW - 1, rebind)),
            reason,
        ),
        case(
            "future_created_at",
            "`created_at` is 301 seconds after `now`, beyond the clock skew.",
            &sign(&f.identity, NOW + CLOCK_SKEW + 1, KIND, &raw(&new, &issuer, NOW + CLOCK_SKEW + 601, rebind)),
            reason,
        ),
        case(
            "other_destination",
            "`p` names another identity than the `destination` of the request.",
            &sign(&f.identity, created, KIND, &raw(&f.other_identity.public_key(), &issuer, created + 600, rebind)),
            reason,
        ),
        case(
            "attestation_z",
            "`z` is `reputation-attestation`.",
            &sign(&f.identity, created, KIND, &raw(&new, &issuer, created + 600, "reputation-attestation")),
            reason,
        ),
        case(
            "missing_issuer",
            "No `issuer` tag.",
            &sign(&f.identity, created, KIND, &no_issuer),
            reason,
        ),
        case("id_mismatch", "`created_at` changed after signing.", &mauled, reason),
    ];

    json!({
        "context": {
            "now": NOW,
            "clock_skew": CLOCK_SKEW,
            "bound_identity": f.identity.public_key().to_hex(),
            "issuer_key": issuer.to_hex(),
            "destination": new.to_hex(),
        },
        "valid": {
            "description": "Signed by the bound identity, for this issuer, inside its hour.",
            "event": event_json(&valid),
            "json": valid.as_json(),
            "expect": {
                "bound_identity": f.identity.public_key().to_hex(),
                "new_identity": new.to_hex(),
                "issuer": issuer.to_hex(),
                "created_at": created,
                "expiration": created + REBIND_MAX_LIFETIME,
            }
        },
        "invalid": invalid,
    })
}

/// A `users` row as the destination stores it.
#[derive(Clone)]
struct Row {
    total_reviews: i64,
    total_rating: f64,
    created_at: i64,
    native_created_at: i64,
    min_rating: i64,
    max_rating: i64,
    last_rating: i64,
    seeded_reviews: i64,
    seeded_rating_sum: f64,
    native_rating_sum: f64,
}

impl Row {
    fn json(&self) -> Value {
        json!({
            "total_reviews": self.total_reviews,
            "total_rating": self.total_rating,
            "created_at": self.created_at,
            "native_created_at": self.native_created_at,
            "min_rating": self.min_rating,
            "max_rating": self.max_rating,
            "last_rating": self.last_rating,
            "seeded_reviews": self.seeded_reviews,
            "seeded_rating_sum": self.seeded_rating_sum,
            "native_rating_sum": self.native_rating_sum,
        })
    }

    /// The merge of the plan's section 6.
    fn import(&self, reviews: i64, rating: f64, since: i64) -> Row {
        let rounded = rating.round() as i64;
        let or_rounded = |v: i64| if v == 0 { rounded } else { v };
        Row {
            total_reviews: self.total_reviews + reviews,
            total_rating: (self.total_rating * self.total_reviews as f64 + rating * reviews as f64)
                / (self.total_reviews + reviews) as f64,
            created_at: self.created_at.min(since),
            min_rating: or_rounded(self.min_rating),
            max_rating: or_rounded(self.max_rating),
            last_rating: or_rounded(self.last_rating),
            seeded_reviews: self.seeded_reviews + reviews,
            seeded_rating_sum: self.seeded_rating_sum + rating * reviews as f64,
            ..self.clone()
        }
    }

    /// The reversal of the plan's section 5.3, with no other import left.
    fn revert(&self, reviews: i64, rating: f64) -> Row {
        let remaining = self.total_reviews - reviews;
        let none_left = remaining == 0;
        let zero_if_none = |v: i64| if none_left { 0 } else { v };
        Row {
            total_reviews: remaining,
            total_rating: if none_left {
                0.0
            } else {
                (self.total_rating * self.total_reviews as f64 - rating * reviews as f64) / remaining as f64
            },
            created_at: self.native_created_at,
            min_rating: zero_if_none(self.min_rating),
            max_rating: zero_if_none(self.max_rating),
            last_rating: zero_if_none(self.last_rating),
            seeded_reviews: self.seeded_reviews - reviews,
            seeded_rating_sum: self.seeded_rating_sum - rating * reviews as f64,
            ..self.clone()
        }
    }
}

fn merges() -> Value {
    let joined = (CREATED - 10 * DAY + 12_345) as i64;
    let existing = Row {
        total_reviews: 2,
        total_rating: 4.5,
        created_at: joined,
        native_created_at: joined,
        min_rating: 4,
        max_rating: 5,
        last_rating: 5,
        seeded_reviews: 0,
        seeded_rating_sum: 0.0,
        native_rating_sum: 9.0,
    };
    let empty = Row {
        total_reviews: 0,
        total_rating: 0.0,
        min_rating: 0,
        max_rating: 0,
        last_rating: 0,
        native_rating_sum: 0.0,
        ..existing.clone()
    };
    let (reviews, rating) = (214, 4.87_f64);
    let cases: Vec<Value> = [
        (
            "existing_reputation",
            "The plan's worked example: 2 native ratings averaging 4.5, 10 days on the instance.",
            existing,
        ),
        (
            "no_reputation",
            "No rating at all: the import also sets the rating extrema, and reverting clears them.",
            empty,
        ),
    ]
    .into_iter()
    .map(|(name, why, before)| {
        let after = before.import(reviews, rating, SINCE as i64);
        let reverted = after.revert(reviews, rating);
        json!({
            "name": name,
            "description": why,
            "before": before.json(),
            "import": { "reviews": reviews, "rating": "4.87", "since": SINCE },
            "after": after.json(),
            "reverted": reverted.json(),
        })
    })
    .collect();
    json!({
        "tolerance": 1e-9,
        "cases": cases,
    })
}

fn main() {
    let f = Fixture {
        issuer: keys("issuer-a"),
        other_issuer: keys("issuer-b"),
        own_issuer: keys("destination-issuer"),
        identity: keys("identity"),
        other_identity: keys("other-identity"),
        new_identity: keys("new-identity"),
    };
    let (valid, invalid) = attestations(&f);
    let rounding: Vec<Value> = [4.866_574_074_074_074, 4.125, 4.895, 4.875, 4.005, 0.9, 5.0, 5.2]
        .iter()
        .map(|avg| json!({ "average": avg, "rating": round_rating(*avg) }))
        .collect();

    let vectors = json!({
        "version": 1,
        "description": "Test vectors for the reputation attestation (kind 38388). See reputation_vectors.md.",
        "context": {
            "now": NOW,
            "clock_skew": CLOCK_SKEW,
            "max_lifetime": MAX_LIFETIME,
            "proven_identity": f.identity.public_key().to_hex(),
            "trust_list": [
                { "name": "issuer-a", "keys": [f.issuer.public_key().to_hex()] },
                { "name": "self", "keys": [f.own_issuer.public_key().to_hex()] },
            ],
            "own_issuer_key": f.own_issuer.public_key().to_hex(),
        },
        "secret_keys": {
            "issuer-a": f.issuer.secret_key().to_secret_hex(),
            "issuer-b": f.other_issuer.secret_key().to_secret_hex(),
            "destination-issuer": f.own_issuer.secret_key().to_secret_hex(),
            "identity": f.identity.secret_key().to_secret_hex(),
            "other-identity": f.other_identity.secret_key().to_secret_hex(),
            "new-identity": f.new_identity.secret_key().to_secret_hex(),
        },
        "attestation": {
            "valid": valid,
            "accepted": accepted_edges(&f),
            "invalid": invalid,
        },
        "rating_rounding": rounding,
        "rebind": rebinds(&f),
        "merge": merges(),
    });
    println!("{}", serde_json::to_string_pretty(&vectors).expect("vectors serialise"));
}
