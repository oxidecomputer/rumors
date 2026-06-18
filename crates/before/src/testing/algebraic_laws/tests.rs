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
use crate::{Party, Rank, Version};

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

// ───────────────────────────── meet: meet-semilattice ─────────────────────────────

proptest! {
    /// Idempotence: `a & a == a`. The GLB of a value and itself is that value.
    #[test]
    fn meet_idempotent(a in arb_oracle_version()) {
        let met = ver(&a) & ver(&a);
        prop_assert!(met == ver(&a), "a & a != a");
    }
}

proptest! {
    /// Commutativity: `a & b == b & a`. The GLB does not depend on operand order.
    #[test]
    fn meet_commutative(a in arb_oracle_version(), b in arb_oracle_version()) {
        let ab = ver(&a) & ver(&b);
        let ba = ver(&b) & ver(&a);
        prop_assert!(ab == ba, "a & b != b & a");
    }
}

proptest! {
    /// Associativity: `(a & b) & c == a & (b & c)`. With commutativity and
    /// idempotence, this makes `&` a meet-semilattice operation.
    #[test]
    fn meet_associative(
        a in arb_oracle_version(),
        b in arb_oracle_version(),
        c in arb_oracle_version(),
    ) {
        let left = (ver(&a) & ver(&b)) & ver(&c);
        let right = ver(&a) & (ver(&b) & ver(&c));
        prop_assert!(left == right, "(a & b) & c != a & (b & c)");
    }
}

proptest! {
    /// The meet is a lower bound: `a & b <= a` and `a & b <= b`. The dual of
    /// [`merge_is_upper_bound`] — what ties `&` to the causal order.
    #[test]
    fn meet_is_lower_bound(a in arb_oracle_version(), b in arb_oracle_version()) {
        let ab = ver(&a) & ver(&b);
        prop_assert!(le(&ab, &ver(&a)), "a & b is not <= a");
        prop_assert!(le(&ab, &ver(&b)), "a & b is not <= b");
    }
}

proptest! {
    /// Absorption ties `&` and `|` into a lattice: `a & (a | b) == a` and
    /// `a | (a & b) == a`. Holds by the ITC algebra, independent of the oracle.
    #[test]
    fn meet_join_absorption(a in arb_oracle_version(), b in arb_oracle_version()) {
        prop_assert!((ver(&a) & (ver(&a) | ver(&b))) == ver(&a), "a & (a | b) != a");
        prop_assert!((ver(&a) | (ver(&a) & ver(&b))) == ver(&a), "a | (a & b) != a");
    }
}

// ───────────────────────────── lattice: distributivity ─────────────────────────────

proptest! {
    /// Meet distributes over join: `a & (b | c) == (a & b) | (a & c)`.
    ///
    /// The version lattice is a sublattice of a function space into the chain
    /// of naturals (pointwise min/max), so it is distributive; this pins that
    /// the impl's `&`/`|` realize it, beyond the lattice laws absorption
    /// already fixes. Holds by the ITC algebra, independent of the oracle.
    #[test]
    fn meet_distributes_over_join(
        a in arb_oracle_version(),
        b in arb_oracle_version(),
        c in arb_oracle_version(),
    ) {
        let (va, vb, vc) = (ver(&a), ver(&b), ver(&c));
        let left = &va & (&vb | &vc);
        let right = (&va & &vb) | (&va & &vc);
        prop_assert!(left == right, "a & (b | c) != (a & b) | (a & c)");
    }
}

proptest! {
    /// Join distributes over meet: `a | (b & c) == (a | b) & (a | c)`, the
    /// dual law.
    ///
    /// In any lattice each distributive law implies the other; asserting both
    /// guards against an impl that realized one direction but not its dual.
    #[test]
    fn join_distributes_over_meet(
        a in arb_oracle_version(),
        b in arb_oracle_version(),
        c in arb_oracle_version(),
    ) {
        let (va, vb, vc) = (ver(&a), ver(&b), ver(&c));
        let left = &va | (&vb & &vc);
        let right = (&va | &vb) & (&va | &vc);
        prop_assert!(left == right, "a | (b & c) != (a | b) & (a | c)");
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
    /// Transitivity: `a <= b && b <= c ⇒ a <= c`.
    ///
    /// The arbitrary generators rarely produce three causally-chained versions
    /// by chance, so we *construct* a chain that always satisfies the
    /// antecedent — `a <= a|b <= a|b|c` — by the upper-bound law, then assert
    /// the conclusion holds across the impl's order directly.
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
    /// versions happen to chain (`a <= b` and `b <= c`), the endpoints must too.
    ///
    /// Complements [`order_transitive`] by exercising chains the generators
    /// stumble into rather than ones we built.
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
    /// `fork` then `join` round-trips.
    ///
    /// Forking a non-empty share yields two halves whose `join` reconstructs
    /// the original id exactly. (`fork` keeps one half in place and returns the
    /// other; `join` sums them back.)
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
    /// id (a `fork` always hands out a real share).
    ///
    /// This is the invariant that keeps a forked population pairwise
    /// `join`-able.
    #[test]
    fn fork_halves_disjoint(p in arb_oracle_party_nonempty()) {
        let mut kept = party(&p);
        let given = kept.fork();
        prop_assert!(kept.is_disjoint(&given), "fork halves overlap");
        prop_assert!(given.is_disjoint(&kept), "is_disjoint not symmetric on fork halves");
        // Neither half decodes-rejects as anonymous: re-encode and decode round-trips,
        // which only succeeds for a nonzero share.
        prop_assert!(Party::decode(&kept.encode()[..]).is_ok(), "kept half is anonymous");
        prop_assert!(Party::decode(&given.encode()[..]).is_ok(), "given half is anonymous");
    }
}

// ───────────────────────────── id: balanced n-way fork ─────────────────────────────

proptest! {
    /// The two balanced-fork forms agree: `From<Party>` for `[Party; N]` equals
    /// the residual the borrowing `forks(N - 1)` keeps, followed by the shares
    /// it yields.
    ///
    /// Both are one balanced split of the same region in the same preorder, so
    /// the consuming array and the borrowing iterator hand out identical
    /// shares — the array is just `[residual] ++ forks`.
    #[test]
    fn forks_matches_from_array(p in arb_oracle_party_nonempty()) {
        const N: usize = 4;
        let array: [Party; N] = party(&p).into();

        let mut keeper = party(&p);
        let yielded: Vec<Party> = keeper.forks(N - 1).collect();
        // [residual] ++ yielded, compared element-wise (`Party` is `!Clone`, so
        // the array cannot be cloned into a `Vec` to compare).
        let reconstructed: Vec<Party> = std::iter::once(keeper).chain(yielded).collect();

        prop_assert!(
            array.iter().eq(reconstructed.iter()),
            "From<Party> array != [residual] ++ forks",
        );
    }
}

proptest! {
    /// Dropping `forks` early folds the untaken shares back: after pulling 2 of
    /// 5, the borrowed party holds everything it did not hand out, so rejoining
    /// the 2 taken shares recovers the original region.
    ///
    /// This is the drop-time reabsorption the iterator promises.
    #[test]
    fn forks_partial_drop_folds_back(p in arb_oracle_party_nonempty()) {
        let original = party(&p);
        let mut keeper = party(&p);
        let taken: Vec<Party> = keeper.forks(5).take(2).collect(); // iterator dropped after 2
        keeper
            .join_all(taken)
            .expect("the taken shares are disjoint from the residual");
        prop_assert!(keeper == original, "early-drop did not fold the remainder back");
    }
}

// ───────────────────────────── id: join_all, the partial-monoid fold ─────────────────────────────

proptest! {
    /// `join_all` reunites a fork: splitting a party with `forks` and folding
    /// the shares back with `join_all` recovers the original region.
    ///
    /// `self` seeds the fold, and balanced-fork shares are pairwise disjoint, so
    /// the fold is defined the whole way.
    #[test]
    fn party_join_all_reunites_a_fork(p in arb_oracle_party_nonempty()) {
        let original = party(&p);
        let mut keeper = party(&p);
        let shares: Vec<Party> = keeper.forks(3).collect();
        keeper
            .join_all(shares)
            .expect("balanced-fork shares are pairwise disjoint");
        prop_assert!(keeper == original, "join_all of a fork did not recover the original");
    }
}

proptest! {
    /// `join_all` is total because `self` seeds the fold: an empty iterator
    /// leaves the party unchanged (the partial monoid has no identity element of
    /// its own to stand in).
    #[test]
    fn party_join_all_empty_is_identity(p in arb_oracle_party_nonempty()) {
        let original = party(&p);
        let mut q = party(&p);
        q.join_all(std::iter::empty::<Party>())
            .expect("the empty join cannot overlap");
        prop_assert!(q == original, "empty join_all changed the party");
    }
}

proptest! {
    /// `join_all` is best-effort and lossless: it folds in every disjoint share
    /// and hands back only those that overlapped, dropping nothing.
    ///
    /// Given a clashing alias of `self` *followed by* a genuine disjoint share,
    /// the share is still absorbed — only the alias comes back. (Fail-fast would
    /// instead abandon the share after the clash; this is what distinguishes the
    /// two.)
    #[test]
    fn party_join_all_best_effort(p in arb_oracle_party_nonempty()) {
        let original = party(&p);
        let mut keeper = party(&p);
        let share = keeper.fork(); // keeper now holds the left half...
        let clash = keeper.dangerously_alias(); // ...and this aliases it (overlaps)
        let returned = keeper
            .join_all([clash, share])
            .expect_err("the alias of keeper overlaps the running union");
        prop_assert_eq!(returned.len(), 1, "only the overlapping alias should be handed back");
        // The disjoint `share` was folded in despite the earlier clash, so the
        // fork is reunited and `keeper` is whole again.
        prop_assert!(keeper == original, "best-effort did not absorb the disjoint share");
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
        let decoded = Party::decode(&bytes[..]).expect("a fresh encoding decodes");
        prop_assert!(decoded == party(&p), "Party decode∘encode is not the identity");
    }
}

proptest! {
    /// `decode ∘ encode == id` on `Version`, including the large-base events
    /// (path sums that would overflow `u64`) the [`arb_oracle_version`]
    /// generator draws near and beyond `u64::MAX`.
    ///
    /// The widened Elias-gamma code must round-trip arbitrary-width bases as a
    /// canonical prefix code.
    #[test]
    fn version_codec_roundtrip(v in arb_oracle_version()) {
        let original = ver(&v);
        let bytes = original.encode();
        let decoded = Version::decode(&bytes[..]).expect("a fresh encoding decodes");
        prop_assert!(decoded == ver(&v), "Version decode∘encode is not the identity");
    }
}

proptest! {
    /// An arbitrary (possibly anonymous) id still round-trips through the codec
    /// *as a sub-tree*: wrapping it under a `seed` sibling makes a nonzero
    /// share that `decode` accepts.
    ///
    /// So the anonymous leaf is exercised on the codec path too (a standalone
    /// anonymous `Party` is rejected by design and cannot be encoded as a
    /// top-level value).
    #[test]
    fn party_codec_roundtrip_with_anonymous_subtree(p in arb_oracle_party()) {
        let wrapped = oracle::Party::node(oracle::Party::seed(), p);
        let original = party(&wrapped);
        let bytes = original.encode();
        let decoded = Party::decode(&bytes[..]).expect("a nonzero share decodes");
        prop_assert!(decoded == party(&wrapped), "Party decode∘encode is not the identity");
    }
}

// ───────────────────────────── rank: valuation & the causal metric ─────────────────────────────

proptest! {
    /// The valuation law: `rank(a | b) + rank(a & b) == rank(a) + rank(b)`.
    ///
    /// Rank is the area under the event tree — a linear functional — and
    /// `max + min == sum` holds pointwise, so area is a lattice valuation. This
    /// is the identity that makes [`Version::distance`] a metric, and it
    /// exercises `Rank` addition against the lattice ops. Holds by the ITC
    /// algebra, independent of the oracle.
    #[test]
    fn rank_is_a_valuation(a in arb_oracle_version(), b in arb_oracle_version()) {
        let (va, vb) = (ver(&a), ver(&b));
        let lhs = (&va | &vb).rank() + (&va & &vb).rank();
        let rhs = va.rank() + vb.rank();
        prop_assert!(lhs == rhs, "rank(a|b) + rank(a&b) != rank(a) + rank(b)");
    }
}

proptest! {
    /// `distance` is symmetric and separating: `d(a, b) == d(b, a)`, `d(a, a)
    /// == 0`, and `d(a, b) > 0` for distinct versions. These are the metric
    /// point laws, following from `rank` being a *strictly* monotone valuation.
    #[test]
    fn distance_is_symmetric_and_separating(
        a in arb_oracle_version(),
        b in arb_oracle_version(),
    ) {
        let (va, vb) = (ver(&a), ver(&b));
        prop_assert!(va.distance(&vb) == vb.distance(&va), "distance is not symmetric");
        prop_assert_eq!(va.distance(&va), Rank::ZERO, "distance(a, a) != 0");
        if va != vb {
            prop_assert!(va.distance(&vb) > Rank::ZERO, "distinct versions at distance 0");
        }
    }
}

proptest! {
    /// The triangle inequality: `d(a, c) <= d(a, b) + d(b, c)`. The defining
    /// metric law; it holds because the strictly monotone valuation `rank`
    /// lives on a *distributive* lattice (see the distributivity laws above).
    #[test]
    fn distance_triangle_inequality(
        a in arb_oracle_version(),
        b in arb_oracle_version(),
        c in arb_oracle_version(),
    ) {
        let (va, vb, vc) = (ver(&a), ver(&b), ver(&c));
        let direct = va.distance(&vc);
        let detour = va.distance(&vb) + vb.distance(&vc);
        prop_assert!(direct <= detour, "triangle inequality violated");
    }
}

proptest! {
    /// `lag` is the directed half of `distance`: the two directions sum to it,
    /// `a.lag(b) + b.lag(a) == a.distance(b)`, and `lag` vanishes exactly when
    /// `self` already dominates `other` (nothing left to learn).
    #[test]
    fn lag_halves_sum_to_distance(a in arb_oracle_version(), b in arb_oracle_version()) {
        let (va, vb) = (ver(&a), ver(&b));
        prop_assert!(
            va.lag(&vb) + vb.lag(&va) == va.distance(&vb),
            "lag halves do not sum to distance",
        );
        if le(&vb, &va) {
            prop_assert_eq!(va.lag(&vb), Rank::ZERO, "lag is nonzero though other <= self");
        }
    }
}
