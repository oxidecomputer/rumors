//! Algebraic-law proptests, asserted directly on the impl.
//!
//! Each test states one law in its doc comment and checks it on impl `Version`
//! / `Party` values built from the arbitrary-normal-form generators. No
//! assertion's right-hand side mentions the oracle: the laws hold by the ITC
//! algebra, so they catch a defect the impl and the recursive oracle would
//! share.

use std::cmp::Ordering;

use proptest::prelude::*;

use crate::oracle;
use crate::testing::generators::{arb_oracle_party, arb_oracle_party_nonempty, arb_oracle_version};
use crate::{Party, Version};

/// `a <= b` under the impl event causal order (concurrency is not-`<=`).
fn le(a: &Version, b: &Version) -> bool {
    a.partial_cmp(b).is_some_and(|o| o != Ordering::Greater)
}

/// Build a fresh impl `Version` from an oracle source tree. The oracle tree is
/// only a carrier of canonical bits here.
fn ver(o: &oracle::Version) -> Version {
    crate::testing::bridge::from_oracle_version(o)
}

/// Build a fresh impl `Party` from an oracle source tree. `Party` is `!Clone`,
/// so every use that consumes or borrows a party rebuilds one from its (cheap,
/// `Clone`) oracle source.
fn party(o: &oracle::Party) -> Party {
    crate::testing::bridge::from_oracle_party(o)
}

// ───────────────────────────── merge: join-semilattice ─────────────────────────────

proptest! {
    /// Idempotence: `a | a == a`. Merging a version with itself is the identity
    /// (the LUB of a value and itself is that value).
    #[test]
    fn merge_idempotent(a in arb_oracle_version()) {
        let merged = ver(&a) | ver(&a);
        prop_assert!(merged == ver(&a), "a | a != a");
    }
}

proptest! {
    /// Commutativity: `a | b == b | a`. The LUB does not depend on operand
    /// order.
    #[test]
    fn merge_commutative(a in arb_oracle_version(), b in arb_oracle_version()) {
        let ab = ver(&a) | ver(&b);
        let ba = ver(&b) | ver(&a);
        prop_assert!(ab == ba, "a | b != b | a");
    }
}

proptest! {
    /// Associativity: `(a | b) | c == a | (b | c)`. With commutativity and
    /// idempotence, this makes `|` a join-semilattice operation.
    #[test]
    fn merge_associative(
        a in arb_oracle_version(),
        b in arb_oracle_version(),
        c in arb_oracle_version(),
    ) {
        let left = (ver(&a) | ver(&b)) | ver(&c);
        let right = ver(&a) | (ver(&b) | ver(&c));
        prop_assert!(left == right, "(a | b) | c != a | (b | c)");
    }
}

proptest! {
    /// The join is an upper bound: `a <= a | b` and `b <= a | b`. This is what
    /// ties `|` to the causal order — the merge dominates both inputs.
    #[test]
    fn merge_is_upper_bound(a in arb_oracle_version(), b in arb_oracle_version()) {
        let ab = ver(&a) | ver(&b);
        prop_assert!(le(&ver(&a), &ab), "a is not <= a | b");
        prop_assert!(le(&ver(&b), &ab), "b is not <= a | b");
    }
}

// ───────────────────────────── causal order: partial order ─────────────────────────────

proptest! {
    /// Reflexivity: `a <= a` and `a == a` under the causal order. (Equality is
    /// the canonical-bit short-circuit; this guards it against ever reporting
    /// an inequality.)
    #[test]
    fn order_reflexive(a in arb_oracle_version()) {
        prop_assert_eq!(ver(&a).partial_cmp(&ver(&a)), Some(Ordering::Equal));
    }
}

proptest! {
    /// Antisymmetry: `a <= b && b <= a ⇒ a == b`. Two mutually-dominating
    /// versions denote the same history, so their canonical forms (and bytes)
    /// coincide.
    #[test]
    fn order_antisymmetric(a in arb_oracle_version(), b in arb_oracle_version()) {
        let (va, vb) = (ver(&a), ver(&b));
        if le(&va, &vb) && le(&vb, &va) {
            prop_assert!(ver(&a) == ver(&b), "a <= b and b <= a but a != b");
        }
    }
}

proptest! {
    /// Transitivity: `a <= b && b <= c ⇒ a <= c`. The arbitrary generators
    /// rarely produce three causally-chained versions by chance, so we
    /// *construct* a chain that always satisfies the antecedent — `a <= a|b <=
    /// a|b|c` — by the upper-bound law, then assert the conclusion holds across
    /// the impl's order directly.
    #[test]
    fn order_transitive(
        a in arb_oracle_version(),
        b in arb_oracle_version(),
        c in arb_oracle_version(),
    ) {
        let lo = ver(&a);
        let mid = ver(&a) | ver(&b);
        let hi = (ver(&a) | ver(&b)) | ver(&c);
        // The chain is real (the antecedent holds by the upper-bound law)...
        prop_assert!(le(&lo, &mid), "constructed chain broken: lo not <= mid");
        prop_assert!(le(&mid, &hi), "constructed chain broken: mid not <= hi");
        // ...so transitivity demands the endpoints compare.
        prop_assert!(le(&lo, &hi), "a <= b <= c but a not <= c");
    }
}

proptest! {
    /// Transitivity, unconstructed: whenever three *arbitrary* generated
    /// versions happen to chain (`a <= b` and `b <= c`), the endpoints must
    /// too. Complements [`order_transitive`] by exercising chains the
    /// generators stumble into rather than ones we built.
    #[test]
    fn order_transitive_incidental(
        a in arb_oracle_version(),
        b in arb_oracle_version(),
        c in arb_oracle_version(),
    ) {
        let (va, vb, vc) = (ver(&a), ver(&b), ver(&c));
        if le(&va, &vb) && le(&vb, &vc) {
            prop_assert!(le(&va, &vc), "a <= b <= c but a not <= c");
        }
    }
}

// ───────────────────────────── id: fork / join / split / sum ─────────────────────────────

proptest! {
    /// `fork` then `join` round-trips: forking a non-empty share yields two
    /// halves whose `join` reconstructs the original id exactly. (`fork` keeps
    /// one half in place and returns the other; `join` sums them back.)
    #[test]
    fn fork_join_roundtrip(p in arb_oracle_party_nonempty()) {
        let original = party(&p);
        let mut kept = party(&p);
        let given = kept.fork();
        kept.join(given).expect("the two fork halves are disjoint, so join succeeds");
        prop_assert!(kept == original, "fork then join did not recover the original id");
    }
}

proptest! {
    /// `split` ⊕ `sum` disjointness: the two halves a `fork` produces are
    /// disjoint, the relation is symmetric, and neither half is the anonymous
    /// id (a `fork` always hands out a real share). This is the invariant that
    /// keeps a forked population pairwise `join`-able.
    #[test]
    fn fork_halves_disjoint(p in arb_oracle_party_nonempty()) {
        let mut kept = party(&p);
        let given = kept.fork();
        prop_assert!(kept.is_disjoint(&given), "fork halves overlap");
        prop_assert!(given.is_disjoint(&kept), "is_disjoint not symmetric on fork halves");
        // Neither half decodes-rejects as anonymous: re-encode and decode round-trips,
        // which only succeeds for a nonzero share.
        prop_assert!(Party::decode(&kept.encode()).is_ok(), "kept half is anonymous");
        prop_assert!(Party::decode(&given.encode()).is_ok(), "given half is anonymous");
    }
}

// ───────────────────────────── codec: section of canonical bytes ─────────────────────────────

proptest! {
    /// `decode ∘ encode == id` on `Party`: encoding a value and decoding the
    /// bytes recovers an equal value. The codec must be a section of the
    /// canonical byte form, or byte-equality `Eq`/`Hash` would be unsound.
    #[test]
    fn party_codec_roundtrip(p in arb_oracle_party_nonempty()) {
        let original = party(&p);
        let bytes = original.encode();
        let decoded = Party::decode(&bytes).expect("a fresh encoding decodes");
        prop_assert!(decoded == party(&p), "Party decode∘encode is not the identity");
    }
}

proptest! {
    /// `decode ∘ encode == id` on `Version`, including the large-base events
    /// (path sums that would overflow `u64`) the [`arb_oracle_version`]
    /// generator draws near and beyond `u64::MAX`: the widened Elias-gamma code
    /// must round-trip arbitrary-width bases as a canonical prefix code.
    #[test]
    fn version_codec_roundtrip(v in arb_oracle_version()) {
        let original = ver(&v);
        let bytes = original.encode();
        let decoded = Version::decode(&bytes).expect("a fresh encoding decodes");
        prop_assert!(decoded == ver(&v), "Version decode∘encode is not the identity");
    }
}

proptest! {
    /// An arbitrary (possibly anonymous) id still round-trips through the codec
    /// *as a sub-tree*: wrapping it under a `seed` sibling makes a nonzero
    /// share that `decode` accepts, so the anonymous leaf is exercised on the
    /// codec path too (a standalone anonymous `Party` is rejected by design and
    /// cannot be encoded as a top-level value).
    #[test]
    fn party_codec_roundtrip_with_anonymous_subtree(p in arb_oracle_party()) {
        let wrapped = oracle::Party::node(oracle::Party::seed(), p);
        let original = party(&wrapped);
        let bytes = original.encode();
        let decoded = Party::decode(&bytes).expect("a nonzero share decodes");
        prop_assert!(decoded == party(&wrapped), "Party decode∘encode is not the identity");
    }
}
