//! The algebraic and representational laws of the public API, as named
//! predicates: one collection, every consumer.
//!
//! Each law is a `(&str, fn(...) -> bool)` pair in a slice grouped by
//! predicate signature, so a harness iterates a slice, feeds every law the
//! same inputs, and reports the *name* of any law that fails. The crate's
//! law proptests drive these slices over generated inputs (arbitrary
//! normal-form trees and organic op-trace populations), and the law fuzz
//! target drives them over decoded hostile-but-canonical values; a law added
//! here reaches every consumer with no further wiring.
//!
//! The algebraic laws transcribe the ITC algebra (Almeida, Baquero & Fonte
//! 2008, §2–§4): versions form a distributive lattice under `|`/`&` whose
//! partial order is causality, ids form a partial commutative monoid under
//! disjoint join with `fork` as its splitting inverse, events inflate
//! strictly and only within the owned region, and `rank` is a strictly
//! monotone valuation. The representational laws pin the crate's own
//! contracts: the codec is a section of canonical bytes, `Eq`/`Hash` ride
//! byte equality, text round-trips, and [`Ranked`]'s total order linearly
//! extends causality. Every law holds unconditionally on the inputs its
//! group admits (below); conditional laws are stated as implications,
//! vacuously true when the antecedent fails, and where they can, they
//! *construct* a witness for the antecedent instead of waiting for one.
//!
//! # Group signatures and admissible inputs
//!
//! Groups are named by the borrowed inputs their predicates take:
//! [`Version`]s are any canonical versions, [`Party`]s are any *live*
//! (non-anonymous) parties — exactly what `decode` accepts and the crate
//! can construct — [`Rank`]s are any ranks, and [`Clock`]s are any
//! canonical party/version pairings.
//!
//! # Linearity
//!
//! `Party` and `Clock` are `!Clone`, and the operations under law (`fork`,
//! `join`, `tick`, `without`, `sync`) consume or mutate their operands — a
//! shared borrow alone cannot exercise them. Every predicate therefore
//! takes shared borrows and materializes its own working copies with
//! [`Party::dangerously_alias`] / [`Clock::dangerously_alias`]: the aliases
//! live and die inside the predicate, which owns no clock universe, so the
//! linearity hazard the method documents (two live holders of one region)
//! never escapes a call. The laws quantify over a value's geometry, which
//! aliasing preserves exactly.
//!
//! # Fallible operations
//!
//! Laws over fallible operations (`Party::join` and friends return
//! `Result`) quantify over the *outcome*: both sides of an equation must
//! agree in arm (`Ok`/`Err`) **and** payload. "Join is commutative" means
//! both orders accept the same pairs and produce equal unions — and hand
//! back equal values when they refuse.

// The group statics are slices of (name, fn pointer) tuples: the fn-pointer
// signature IS the group's identity, so naming each one would only add
// indirection between a group and its shape.
#![allow(clippy::type_complexity)]

use core::cmp::Ordering;
use core::hash::{Hash, Hasher};

use crate::{Clock, Party, Rank, Ranked, Ticks, Version};

/// A named law: the name a failure reports, and the predicate that must
/// hold on every admissible input.
pub type Law<F> = (&'static str, F);

/// `a <= b` under the causal order (concurrency is not-`<=`).
fn le(a: &Version, b: &Version) -> bool {
    a.partial_cmp(b).is_some_and(|o| o != Ordering::Greater)
}

/// `a <= b` for any pair the view comparison matrix admits (view vs
/// version, view vs view), under the same not-`<=` reading of
/// concurrency.
fn le_by<L: PartialOrd<R>, R>(a: &L, b: &R) -> bool {
    a.partial_cmp(b).is_some_and(|o| o != Ordering::Greater)
}

/// One value's `Hash` output under the std hasher, for the `Eq`/`Hash`
/// coherence laws.
fn hash_of<T: Hash>(value: &T) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

// ───────────────────────────── Version: one value ─────────────────────────────

/// Laws over one version.
///
/// The lattice point laws at a single value (idempotence, the bottom
/// element), observer coherence (`is_empty`, `concurrent`, `distance`,
/// `rank`, `min_ticks`, [`Ranked`]), and the representational round-trips
/// (codec, text, byte views).
pub static VERSION_SOLO: &[Law<fn(&Version) -> bool>] = &[
    ("merge_idempotent", merge_idempotent),
    ("meet_idempotent", meet_idempotent),
    ("order_reflexive", order_reflexive),
    ("new_is_the_bottom", new_is_the_bottom),
    ("merge_new_is_identity", merge_new_is_identity),
    ("meet_new_is_absorbing", meet_new_is_absorbing),
    ("is_empty_iff_new", is_empty_iff_new),
    ("never_concurrent_with_self", never_concurrent_with_self),
    ("distance_to_self_is_zero", distance_to_self_is_zero),
    ("rank_zero_iff_empty", rank_zero_iff_empty),
    ("min_ticks_zero_iff_empty", min_ticks_zero_iff_empty),
    ("seed_projection_is_identity", seed_projection_is_identity),
    ("ranked_carries_own_rank", ranked_carries_own_rank),
    ("version_codec_roundtrip", version_codec_roundtrip),
    ("version_text_roundtrip", version_text_roundtrip),
    (
        "version_as_bytes_matches_encode",
        version_as_bytes_matches_encode,
    ),
    (
        "version_encoded_bits_matches_encode_len",
        version_encoded_bits_matches_encode_len,
    ),
];

/// Idempotence: `a | a == a` (the LUB of a value and itself is that value).
fn merge_idempotent(a: &Version) -> bool {
    (a.clone() | a.clone()) == *a
}

/// Idempotence: `a & a == a` (the GLB of a value and itself is that value).
fn meet_idempotent(a: &Version) -> bool {
    (a.clone() & a.clone()) == *a
}

/// Reflexivity: `a` compares `Equal` to itself (the canonical-bit
/// short-circuit never reports an inequality).
fn order_reflexive(a: &Version) -> bool {
    a.partial_cmp(a) == Some(Ordering::Equal)
}

/// `Version::new()` is the lattice bottom: below every version.
fn new_is_the_bottom(a: &Version) -> bool {
    le(&Version::new(), a)
}

/// The bottom is the join identity: `new | a == a`.
fn merge_new_is_identity(a: &Version) -> bool {
    (Version::new() | a.clone()) == *a
}

/// The bottom absorbs the meet: `new & a == new`.
fn meet_new_is_absorbing(a: &Version) -> bool {
    (Version::new() & a.clone()) == Version::new()
}

/// `is_empty` recognizes exactly the bottom: `a.is_empty() ⟺ a == new`.
fn is_empty_iff_new(a: &Version) -> bool {
    a.is_empty() == (*a == Version::new())
}

/// Concurrency is irreflexive: a version is never concurrent with itself.
fn never_concurrent_with_self(a: &Version) -> bool {
    !a.concurrent(a)
}

/// The metric point law at the diagonal: `d(a, a) == 0`.
fn distance_to_self_is_zero(a: &Version) -> bool {
    a.distance(a) == Rank::ZERO
}

/// `rank` separates the bottom: zero area exactly for the empty version
/// (rank is a strictly monotone valuation, so only the zero function has
/// zero area).
fn rank_zero_iff_empty(a: &Version) -> bool {
    (a.rank() == Rank::ZERO) == a.is_empty()
}

/// `min_ticks` separates the bottom: a zero tick floor exactly for the
/// empty version (the floor is a sum of nonnegative bases, zero only
/// when every base is).
fn min_ticks_zero_iff_empty(a: &Version) -> bool {
    (a.min_ticks() == Ticks::ZERO) == a.is_empty()
}

/// The whole-interval party is the projection identity: `a / seed == a`.
fn seed_projection_is_identity(a: &Version) -> bool {
    (a / &Party::seed()) == *a
}

/// [`Ranked`] carries exactly its version's rank, and `into_parts` returns
/// the version with that rank.
fn ranked_carries_own_rank(a: &Version) -> bool {
    let ranked = Ranked::from(a.clone());
    let carried = ranked.rank() == &a.rank() && ranked.version() == a;
    let (version, rank) = ranked.into_parts();
    carried && version == *a && rank == a.rank()
}

/// `decode ∘ encode == id`, and the round-tripped value re-encodes to the
/// same bytes (the codec is a section of canonical bytes — what byte-level
/// `Eq`/`Hash` rest on).
fn version_codec_roundtrip(a: &Version) -> bool {
    let bytes = a.encode();
    Version::decode(&bytes[..]).is_ok_and(|decoded| decoded == *a && decoded.encode() == bytes)
}

/// `FromStr ∘ Display == id`: the paper notation round-trips.
fn version_text_roundtrip(a: &Version) -> bool {
    a.to_string()
        .parse::<Version>()
        .is_ok_and(|parsed| parsed == *a)
}

/// The borrowed byte view is the encoding: `as_bytes == encode`.
fn version_as_bytes_matches_encode(a: &Version) -> bool {
    a.as_bytes() == &a.encode()[..]
}

/// `encoded_bits` is the pre-pad bit length of `encode`: the byte length is
/// the bit length rounded up to whole bytes.
fn version_encoded_bits_matches_encode_len(a: &Version) -> bool {
    a.encode().len() == a.encoded_bits().div_ceil(8)
}

// ───────────────────────────── Version: pairs ─────────────────────────────

/// Laws over a pair of versions.
///
/// Commutativity and the bound laws of the lattice operations, absorption,
/// the partial order's pair laws and their coherence with `Eq`/`Hash` and
/// `concurrent`, the valuation identity tying `rank` to the lattice, the
/// `distance`/`lag` metric laws, and [`Ranked`]'s linear extension.
pub static VERSION_PAIR: &[Law<fn(&Version, &Version) -> bool>] = &[
    ("merge_commutative", merge_commutative),
    ("meet_commutative", meet_commutative),
    ("merge_is_upper_bound", merge_is_upper_bound),
    ("meet_is_lower_bound", meet_is_lower_bound),
    ("meet_join_absorption", meet_join_absorption),
    ("order_antisymmetric", order_antisymmetric),
    ("order_absorbing", order_absorbing),
    ("eq_iff_cmp_equal", eq_iff_cmp_equal),
    ("partial_cmp_is_dual", partial_cmp_is_dual),
    ("concurrent_iff_incomparable", concurrent_iff_incomparable),
    ("rank_is_a_valuation", rank_is_a_valuation),
    ("rank_strictly_monotone", rank_strictly_monotone),
    ("distance_symmetric", distance_symmetric),
    ("distance_separates", distance_separates),
    ("distance_is_the_rank_gap", distance_is_the_rank_gap),
    ("lag_halves_sum_to_distance", lag_halves_sum_to_distance),
    ("lag_zero_iff_dominated", lag_zero_iff_dominated),
    ("lag_is_the_rank_gap", lag_is_the_rank_gap),
    ("version_eq_iff_bytes_eq", version_eq_iff_bytes_eq),
    ("version_eq_implies_hash_eq", version_eq_implies_hash_eq),
    (
        "ranked_linearly_extends_causality",
        ranked_linearly_extends_causality,
    ),
];

/// Commutativity: `a | b == b | a` (the LUB does not depend on operand
/// order).
fn merge_commutative(a: &Version, b: &Version) -> bool {
    (a | b) == (b | a)
}

/// Commutativity: `a & b == b & a` (the GLB does not depend on operand
/// order).
fn meet_commutative(a: &Version, b: &Version) -> bool {
    (a & b) == (b & a)
}

/// The join is an upper bound: `a <= a | b` and `b <= a | b` — what ties
/// `|` to the causal order.
fn merge_is_upper_bound(a: &Version, b: &Version) -> bool {
    let ab = a | b;
    le(a, &ab) && le(b, &ab)
}

/// The meet is a lower bound: `a & b <= a` and `a & b <= b`, the dual of
/// [`merge_is_upper_bound`].
fn meet_is_lower_bound(a: &Version, b: &Version) -> bool {
    let ab = a & b;
    le(&ab, a) && le(&ab, b)
}

/// Absorption ties `&` and `|` into a lattice: `a & (a | b) == a` and
/// `a | (a & b) == a`.
fn meet_join_absorption(a: &Version, b: &Version) -> bool {
    (a & &(a | b)) == *a && (a | &(a & b)) == *a
}

/// Antisymmetry: `a <= b && b <= a ⟹ a == b` (mutually dominating versions
/// denote the same history, so their canonical bytes coincide).
fn order_antisymmetric(a: &Version, b: &Version) -> bool {
    !(le(a, b) && le(b, a)) || a == b
}

/// Domination absorbs: `a <= b ⟹ a | b == b && a & b == a`.
fn order_absorbing(a: &Version, b: &Version) -> bool {
    !le(a, b) || ((a | b) == *b && (a & b) == *a)
}

/// `Eq` and the order agree: `a == b ⟺ partial_cmp == Some(Equal)`.
fn eq_iff_cmp_equal(a: &Version, b: &Version) -> bool {
    (a == b) == (a.partial_cmp(b) == Some(Ordering::Equal))
}

/// The order is its own dual: `cmp(a, b)` is `cmp(b, a)` reversed
/// (including the concurrent `None`).
fn partial_cmp_is_dual(a: &Version, b: &Version) -> bool {
    a.partial_cmp(b) == b.partial_cmp(a).map(Ordering::reverse)
}

/// `concurrent` is exactly incomparability, and symmetric.
fn concurrent_iff_incomparable(a: &Version, b: &Version) -> bool {
    a.concurrent(b) == a.partial_cmp(b).is_none() && a.concurrent(b) == b.concurrent(a)
}

/// The valuation law: `rank(a|b) + rank(a&b) == rank(a) + rank(b)` (area
/// is a lattice valuation because `max + min == sum` holds pointwise) —
/// the identity that makes [`Version::distance`] a metric.
fn rank_is_a_valuation(a: &Version, b: &Version) -> bool {
    (a | b).rank() + (a & b).rank() == a.rank() + b.rank()
}

/// `rank` is strictly monotone on the causal order: `a <= b ⟹ rank(a) <=
/// rank(b)`, strictly when `a != b`.
fn rank_strictly_monotone(a: &Version, b: &Version) -> bool {
    !le(a, b) || (a.rank() <= b.rank() && (a == b || a.rank() < b.rank()))
}

/// The metric symmetry law: `d(a, b) == d(b, a)`.
fn distance_symmetric(a: &Version, b: &Version) -> bool {
    a.distance(b) == b.distance(a)
}

/// The metric separates points: `d(a, b) == 0 ⟺ a == b`.
fn distance_separates(a: &Version, b: &Version) -> bool {
    (a.distance(b) == Rank::ZERO) == (a == b)
}

/// `distance` is the valuation gap across the lattice interval:
/// `d(a, b) == rank(a|b) - rank(a&b)` (the join dominates the meet, so the
/// subtraction is defined).
fn distance_is_the_rank_gap(a: &Version, b: &Version) -> bool {
    (a | b).rank().checked_sub(&(a & b).rank()) == Some(a.distance(b))
}

/// `lag` is the directed half of `distance`: the two directions sum to it.
fn lag_halves_sum_to_distance(a: &Version, b: &Version) -> bool {
    a.lag(b) + b.lag(a) == a.distance(b)
}

/// `lag` vanishes exactly when there is nothing left to learn:
/// `a.lag(b) == 0 ⟺ b <= a`.
fn lag_zero_iff_dominated(a: &Version, b: &Version) -> bool {
    (a.lag(b) == Rank::ZERO) == le(b, a)
}

/// `lag` is the valuation gap up to the join: `a.lag(b) == rank(a|b) -
/// rank(a)`.
fn lag_is_the_rank_gap(a: &Version, b: &Version) -> bool {
    (a | b).rank().checked_sub(&a.rank()) == Some(a.lag(b))
}

/// `Eq` is canonical-byte equality: `a == b ⟺ encode(a) == encode(b)`.
fn version_eq_iff_bytes_eq(a: &Version, b: &Version) -> bool {
    (a == b) == (a.encode() == b.encode())
}

/// `Eq`/`Hash` coherence: equal versions hash equally.
fn version_eq_implies_hash_eq(a: &Version, b: &Version) -> bool {
    a != b || hash_of(a) == hash_of(b)
}

/// [`Ranked`]'s total order linearly extends causality: causally ordered
/// versions compare the same way, concurrent versions are tiebroken (never
/// `Equal`), and `Ranked` equality is version equality.
fn ranked_linearly_extends_causality(a: &Version, b: &Version) -> bool {
    let (ra, rb) = (Ranked::from(a.clone()), Ranked::from(b.clone()));
    let extends = match a.partial_cmp(b) {
        Some(ord) => ra.cmp(&rb) == ord,
        None => ra.cmp(&rb) != Ordering::Equal,
    };
    extends && (ra == rb) == (a == b)
}

// ───────────────────────────── Version: triples ─────────────────────────────

/// Laws over a triple of versions.
///
/// Associativity, the least/greatest bound laws, both distributive laws,
/// transitivity (constructed and incidental), and the triangle inequality.
pub static VERSION_TRIPLE: &[Law<fn(&Version, &Version, &Version) -> bool>] = &[
    ("merge_associative", merge_associative),
    ("meet_associative", meet_associative),
    ("merge_is_least_upper_bound", merge_is_least_upper_bound),
    ("meet_is_greatest_lower_bound", meet_is_greatest_lower_bound),
    ("meet_distributes_over_join", meet_distributes_over_join),
    ("join_distributes_over_meet", join_distributes_over_meet),
    ("order_transitive_constructed", order_transitive_constructed),
    ("order_transitive_incidental", order_transitive_incidental),
    ("distance_triangle_inequality", distance_triangle_inequality),
];

/// Associativity: `(a | b) | c == a | (b | c)` — with commutativity and
/// idempotence, `|` is a join-semilattice operation.
fn merge_associative(a: &Version, b: &Version, c: &Version) -> bool {
    (&(a | b) | c) == (a | &(b | c))
}

/// Associativity: `(a & b) & c == a & (b & c)`, the meet dual.
fn meet_associative(a: &Version, b: &Version, c: &Version) -> bool {
    (&(a & b) & c) == (a & &(b & c))
}

/// The join is the *least* upper bound: the constructed common upper bound
/// `a | b | c` dominates `a | b` (any common upper bound of `a` and `b`
/// dominates their join).
fn merge_is_least_upper_bound(a: &Version, b: &Version, c: &Version) -> bool {
    let ab = a | b;
    let upper = &ab | c;
    le(a, &upper) && le(b, &upper) && le(&ab, &upper)
}

/// The meet is the *greatest* lower bound: the constructed common lower
/// bound `a & b & c` is dominated by `a & b`, the dual of
/// [`merge_is_least_upper_bound`].
fn meet_is_greatest_lower_bound(a: &Version, b: &Version, c: &Version) -> bool {
    let ab = a & b;
    let lower = &ab & c;
    le(&lower, a) && le(&lower, b) && le(&lower, &ab)
}

/// Meet distributes over join: `a & (b | c) == (a & b) | (a & c)`. The
/// version lattice embeds in a function space into the chain of naturals
/// (pointwise min/max), so it is distributive.
fn meet_distributes_over_join(a: &Version, b: &Version, c: &Version) -> bool {
    (a & &(b | c)) == (&(a & b) | &(a & c))
}

/// Join distributes over meet: `a | (b & c) == (a | b) & (a | c)`, the dual
/// law (in a lattice each distributive law implies the other; asserting
/// both guards an impl that realized only one direction).
fn join_distributes_over_meet(a: &Version, b: &Version, c: &Version) -> bool {
    (a | &(b & c)) == (&(a | b) & &(a | c))
}

/// Transitivity on a constructed chain: `a <= a|b <= a|b|c` holds by the
/// upper-bound law, so the endpoints must compare — arbitrary inputs rarely
/// chain by chance, so the chain is built rather than awaited.
fn order_transitive_constructed(a: &Version, b: &Version, c: &Version) -> bool {
    let mid = a | b;
    let hi = &mid | c;
    le(a, &mid) && le(&mid, &hi) && le(a, &hi)
}

/// Transitivity, incidental: whenever three arbitrary versions happen to
/// chain (`a <= b` and `b <= c`), the endpoints must too.
fn order_transitive_incidental(a: &Version, b: &Version, c: &Version) -> bool {
    !(le(a, b) && le(b, c)) || le(a, c)
}

/// The triangle inequality: `d(a, c) <= d(a, b) + d(b, c)` — the defining
/// metric law, which holds because the strictly monotone valuation `rank`
/// lives on a *distributive* lattice.
fn distance_triangle_inequality(a: &Version, b: &Version, c: &Version) -> bool {
    a.distance(c) <= a.distance(b) + b.distance(c)
}

// ───────────────────────────── Party: one value ─────────────────────────────

/// Laws over one live party.
///
/// The fork/join round-trip and its disjointness geometry, the balanced
/// n-way fork's two forms, `join_all`'s fold laws, the covering order's
/// point laws (with a constructed transitivity chain), `without` at the
/// reflexive corner, aliasing, and the representational round-trips.
pub static PARTY_SOLO: &[Law<fn(&Party) -> bool>] = &[
    ("fork_join_roundtrip", fork_join_roundtrip),
    ("fork_halves_disjoint", fork_halves_disjoint),
    (
        "fork_halves_covered_by_parent",
        fork_halves_covered_by_parent,
    ),
    ("forks_matches_from_array", forks_matches_from_array),
    (
        "forks_partial_drop_folds_back",
        forks_partial_drop_folds_back,
    ),
    (
        "party_join_all_reunites_a_fork",
        party_join_all_reunites_a_fork,
    ),
    (
        "party_join_all_empty_is_identity",
        party_join_all_empty_is_identity,
    ),
    ("party_join_all_best_effort", party_join_all_best_effort),
    ("join_overlap_hands_back", join_overlap_hands_back),
    ("covers_reflexive", covers_reflexive),
    (
        "covers_transitive_constructed",
        covers_transitive_constructed,
    ),
    ("seed_covers_every_party", seed_covers_every_party),
    ("never_disjoint_from_self", never_disjoint_from_self),
    ("without_self_is_none", without_self_is_none),
    ("without_inverts_fork", without_inverts_fork),
    (
        "alias_is_byte_identical_overlap",
        alias_is_byte_identical_overlap,
    ),
    ("is_seed_iff_equals_seed", is_seed_iff_equals_seed),
    ("party_codec_roundtrip", party_codec_roundtrip),
    ("party_text_roundtrip", party_text_roundtrip),
    (
        "party_as_bytes_matches_encode",
        party_as_bytes_matches_encode,
    ),
    (
        "party_encoded_bits_matches_encode_len",
        party_encoded_bits_matches_encode_len,
    ),
];

/// `fork` then `join` round-trips: the two halves reconstruct the original
/// region exactly.
fn fork_join_roundtrip(p: &Party) -> bool {
    let mut kept = p.dangerously_alias();
    let given = kept.fork();
    kept.join(given).is_ok() && kept == *p
}

/// The two halves a `fork` produces are disjoint, the relation is
/// symmetric, and neither half is anonymous (both re-encode and decode as
/// nonzero shares) — the invariant that keeps a forked population pairwise
/// `join`-able.
fn fork_halves_disjoint(p: &Party) -> bool {
    let mut kept = p.dangerously_alias();
    let given = kept.fork();
    kept.is_disjoint(&given)
        && given.is_disjoint(&kept)
        && Party::decode(&kept.encode()[..]).is_ok()
        && Party::decode(&given.encode()[..]).is_ok()
}

/// A fork's parent covers both halves, and the halves cover neither other
/// (they are disjoint proper subregions).
fn fork_halves_covered_by_parent(p: &Party) -> bool {
    let mut kept = p.dangerously_alias();
    let given = kept.fork();
    p.covers(&kept) && p.covers(&given) && !kept.covers(&given) && !given.covers(&kept)
}

/// The two balanced-fork forms agree: `From<Party>` for `[Party; N]` equals
/// the residual the borrowing `forks(N - 1)` keeps, followed by the shares
/// it yields (`[residual] ++ forks`).
fn forks_matches_from_array(p: &Party) -> bool {
    const N: usize = 4;
    let array: [Party; N] = p.dangerously_alias().into();
    let mut keeper = p.dangerously_alias();
    let yielded: Vec<Party> = keeper.forks(N - 1).collect();
    let reconstructed: Vec<Party> = std::iter::once(keeper).chain(yielded).collect();
    array.iter().eq(reconstructed.iter())
}

/// Dropping `forks` early folds the untaken shares back: after pulling 2 of
/// 5, rejoining the 2 taken shares recovers the original region — the
/// drop-time reabsorption the iterator promises.
fn forks_partial_drop_folds_back(p: &Party) -> bool {
    let mut keeper = p.dangerously_alias();
    let taken: Vec<Party> = keeper.forks(5).take(2).collect(); // iterator dropped after 2
    keeper.join_all(taken).is_ok() && keeper == *p
}

/// `join_all` reunites a fork: folding a balanced fork's shares back
/// recovers the original region (`self` seeds the fold; balanced-fork
/// shares are pairwise disjoint, so the fold is defined the whole way).
fn party_join_all_reunites_a_fork(p: &Party) -> bool {
    let mut keeper = p.dangerously_alias();
    let shares: Vec<Party> = keeper.forks(3).collect();
    keeper.join_all(shares).is_ok() && keeper == *p
}

/// `join_all` of the empty iterator leaves the party unchanged (`self`
/// seeds the fold; the partial monoid has no identity element of its own).
fn party_join_all_empty_is_identity(p: &Party) -> bool {
    let mut q = p.dangerously_alias();
    q.join_all(std::iter::empty::<Party>()).is_ok() && q == *p
}

/// `join_all` is best-effort and lossless: given a clashing alias *before*
/// a genuine disjoint share, the share is still absorbed and only the alias
/// comes back (fail-fast would abandon the share after the clash).
fn party_join_all_best_effort(p: &Party) -> bool {
    let mut keeper = p.dangerously_alias();
    let share = keeper.fork(); // keeper holds one half...
    let clash = keeper.dangerously_alias(); // ...and this aliases it (overlaps)
    match keeper.join_all([clash, share]) {
        Err(returned) => returned.len() == 1 && keeper == *p,
        Ok(()) => false,
    }
}

/// Joining an overlapping party errors and hands it back unchanged: a
/// proper subregion refuses to absorb the region containing it.
fn join_overlap_hands_back(p: &Party) -> bool {
    let mut sub = p.dangerously_alias();
    let _ = sub.fork(); // sub is now a proper subregion of p
    match sub.join(p.dangerously_alias()) {
        Err(handed_back) => handed_back == *p,
        Ok(()) => false,
    }
}

/// Covering is reflexive: a party covers its own region.
fn covers_reflexive(p: &Party) -> bool {
    p.covers(&p.dangerously_alias())
}

/// Covering chains down a constructed fork tower: the whole covers its
/// half, the half its quarter, and — transitively — the whole covers the
/// quarter.
fn covers_transitive_constructed(p: &Party) -> bool {
    let mut quarter = p.dangerously_alias();
    let _ = quarter.fork(); // quarter: a half of p
    let half = quarter.dangerously_alias();
    let _ = quarter.fork(); // quarter: a quarter of p
    p.covers(&half) && half.covers(&quarter) && p.covers(&quarter)
}

/// The whole-interval seed covers every live party — and, owning
/// everything, is disjoint from none.
fn seed_covers_every_party(p: &Party) -> bool {
    Party::seed().covers(p) && !Party::seed().is_disjoint(p) && !p.is_disjoint(&Party::seed())
}

/// Disjointness is irreflexive on live parties: a nonzero region overlaps
/// itself.
fn never_disjoint_from_self(p: &Party) -> bool {
    !p.is_disjoint(&p.dangerously_alias())
}

/// A party covers itself, so removing itself leaves nothing:
/// `p \ p == None`.
fn without_self_is_none(p: &Party) -> bool {
    p.dangerously_alias().without(p).is_none()
}

/// `without` is the partial inverse of `join` on the fork lattice: carving
/// a forked-off share back out of the parent recovers the kept half, and
/// removing a disjoint share is a no-op.
fn without_inverts_fork(p: &Party) -> bool {
    let mut keep = p.dangerously_alias();
    let give = keep.fork();
    let carved = p.dangerously_alias().without(&give);
    let noop = keep.dangerously_alias().without(&give);
    carved.is_some_and(|c| c == keep) && noop.is_some_and(|n| n == keep)
}

/// `dangerously_alias` yields a byte-identical, `Eq` copy aliasing the
/// entire region: the two are *not* disjoint — the deliberate linearity
/// violation the method documents.
fn alias_is_byte_identical_overlap(p: &Party) -> bool {
    let dup = p.dangerously_alias();
    dup == *p && dup.as_bytes() == p.as_bytes() && !p.is_disjoint(&dup)
}

/// `is_seed` recognizes exactly the whole-interval party: `p.is_seed() ⟺
/// p == seed`.
fn is_seed_iff_equals_seed(p: &Party) -> bool {
    p.is_seed() == (*p == Party::seed())
}

/// `decode ∘ encode == id`, and the round-tripped party re-encodes to the
/// same bytes.
fn party_codec_roundtrip(p: &Party) -> bool {
    let bytes = p.encode();
    Party::decode(&bytes[..]).is_ok_and(|decoded| decoded == *p && decoded.encode() == bytes)
}

/// `FromStr ∘ Display == id`: the paper notation round-trips.
fn party_text_roundtrip(p: &Party) -> bool {
    p.to_string()
        .parse::<Party>()
        .is_ok_and(|parsed| parsed == *p)
}

/// The borrowed byte view is the encoding: `as_bytes == encode`.
fn party_as_bytes_matches_encode(p: &Party) -> bool {
    p.as_bytes() == &p.encode()[..]
}

/// `encoded_bits` is the pre-pad bit length of `encode`.
fn party_encoded_bits_matches_encode_len(p: &Party) -> bool {
    p.encode().len() == p.encoded_bits().div_ceil(8)
}

// ───────────────────────────── Party: pairs ─────────────────────────────

/// Laws over a pair of live parties.
///
/// The covering order's antisymmetry and its exclusion by disjointness,
/// disjointness symmetry, `join`'s outcome-quantified commutativity and its
/// coherence with `is_disjoint`, `without`'s two characterizations, and
/// `Eq`/`Hash` coherence.
pub static PARTY_PAIR: &[Law<fn(&Party, &Party) -> bool>] = &[
    ("covers_antisymmetric", covers_antisymmetric),
    ("disjoint_symmetric", disjoint_symmetric),
    ("disjoint_excludes_covering", disjoint_excludes_covering),
    ("join_defined_iff_disjoint", join_defined_iff_disjoint),
    ("join_commutative_outcomes", join_commutative_outcomes),
    (
        "join_covers_both_and_without_undoes",
        join_covers_both_and_without_undoes,
    ),
    ("without_characterization", without_characterization),
    ("without_disjoint_is_noop", without_disjoint_is_noop),
    ("party_eq_iff_bytes_eq", party_eq_iff_bytes_eq),
    ("party_eq_implies_hash_eq", party_eq_implies_hash_eq),
];

/// Covering is antisymmetric: two regions cover each other exactly when
/// they are equal.
fn covers_antisymmetric(a: &Party, b: &Party) -> bool {
    (a.covers(b) && b.covers(a)) == (a == b)
}

/// Disjointness is symmetric.
fn disjoint_symmetric(a: &Party, b: &Party) -> bool {
    a.is_disjoint(b) == b.is_disjoint(a)
}

/// Disjoint live regions cover neither other (covering needs overlap, and
/// a live party is nonempty).
fn disjoint_excludes_covering(a: &Party, b: &Party) -> bool {
    !a.is_disjoint(b) || (!a.covers(b) && !b.covers(a))
}

/// `join` accepts exactly the disjoint pairs: `a.join(b) is Ok ⟺
/// a.is_disjoint(b)`.
fn join_defined_iff_disjoint(a: &Party, b: &Party) -> bool {
    a.dangerously_alias().join(b.dangerously_alias()).is_ok() == a.is_disjoint(b)
}

/// `join` is commutative over outcomes: both orders agree in arm, produce
/// equal unions on `Ok`, and hand back the argument unchanged (leaving
/// `self` unchanged) on `Err`.
fn join_commutative_outcomes(a: &Party, b: &Party) -> bool {
    let mut ab = a.dangerously_alias();
    let ab_result = ab.join(b.dangerously_alias());
    let mut ba = b.dangerously_alias();
    let ba_result = ba.join(a.dangerously_alias());
    match (ab_result, ba_result) {
        (Ok(()), Ok(())) => ab == ba,
        (Err(back_b), Err(back_a)) => back_b == *b && back_a == *a && ab == *a && ba == *b,
        _ => false,
    }
}

/// A disjoint join absorbs both operands, and `without` undoes it:
/// `(a + b).covers(a)`, `(a + b).covers(b)`, and `(a + b) \ a == b`.
fn join_covers_both_and_without_undoes(a: &Party, b: &Party) -> bool {
    if !a.is_disjoint(b) {
        return true;
    }
    let mut joined = a.dangerously_alias();
    if joined.join(b.dangerously_alias()).is_err() {
        return false;
    }
    joined.covers(a) && joined.covers(b) && joined.without(a).is_some_and(|rest| rest == *b)
}

/// `without`'s two characterizations: the result is `None` exactly when
/// `other` covers `self`, and a surviving remainder is a subregion of
/// `self` disjoint from `other`.
fn without_characterization(a: &Party, b: &Party) -> bool {
    match a.dangerously_alias().without(b) {
        None => b.covers(a),
        Some(remainder) => !b.covers(a) && a.covers(&remainder) && remainder.is_disjoint(b),
    }
}

/// Removing a disjoint share is a no-op: `a \ b == a` when the regions
/// share nothing.
fn without_disjoint_is_noop(a: &Party, b: &Party) -> bool {
    !a.is_disjoint(b) || a.dangerously_alias().without(b).is_some_and(|r| r == *a)
}

/// `Eq` is canonical-byte equality: `a == b ⟺ encode(a) == encode(b)`.
fn party_eq_iff_bytes_eq(a: &Party, b: &Party) -> bool {
    (a == b) == (a.encode() == b.encode())
}

/// `Eq`/`Hash` coherence: equal parties hash equally.
fn party_eq_implies_hash_eq(a: &Party, b: &Party) -> bool {
    a != b || hash_of(a) == hash_of(b)
}

// ───────────────────────────── Party: triples ─────────────────────────────

/// Laws over a triple of live parties.
///
/// The covering order's incidental transitivity and the partial monoid's
/// associativity — outcome-quantified, with `join_all`'s acceptance tied to
/// pairwise disjointness.
pub static PARTY_TRIPLE: &[Law<fn(&Party, &Party, &Party) -> bool>] = &[
    ("covers_transitive_incidental", covers_transitive_incidental),
    ("join_associative_outcomes", join_associative_outcomes),
    (
        "join_all_defined_iff_pairwise_disjoint",
        join_all_defined_iff_pairwise_disjoint,
    ),
];

/// Covering is transitive: whenever three arbitrary parties happen to chain
/// (`a ⊇ b ⊇ c`), the endpoints must too.
fn covers_transitive_incidental(a: &Party, b: &Party, c: &Party) -> bool {
    !(a.covers(b) && b.covers(c)) || a.covers(c)
}

/// Fold `second` then `third` into an alias of `first`, `None` at the
/// first overlap — one association order of the partial monoid's ternary
/// sum.
fn join3(first: &Party, second: &Party, third: &Party) -> Option<Party> {
    let mut acc = first.dangerously_alias();
    acc.join(second.dangerously_alias()).ok()?;
    acc.join(third.dangerously_alias()).ok()?;
    Some(acc)
}

/// The partial monoid is associative over outcomes: `(a + b) + c` and
/// `a + (b + c)` agree in definedness and, where defined, in value (both
/// are defined exactly on pairwise-disjoint triples).
fn join_associative_outcomes(a: &Party, b: &Party, c: &Party) -> bool {
    let left = join3(a, b, c);
    let right = {
        let mut bc = b.dangerously_alias();
        match bc.join(c.dangerously_alias()) {
            Ok(()) => {
                let mut acc = a.dangerously_alias();
                acc.join(bc).ok().map(|()| acc)
            }
            Err(_) => None,
        }
    };
    match (left, right) {
        (Some(l), Some(r)) => l == r,
        (None, None) => true,
        _ => false,
    }
}

/// `join_all` accepts exactly the pairwise-disjoint families: folding `b`
/// and `c` into `a` succeeds if and only if all three regions are pairwise
/// disjoint.
fn join_all_defined_iff_pairwise_disjoint(a: &Party, b: &Party, c: &Party) -> bool {
    let mut acc = a.dangerously_alias();
    let accepted = acc
        .join_all([b.dangerously_alias(), c.dangerously_alias()])
        .is_ok();
    accepted == (a.is_disjoint(b) && a.is_disjoint(c) && b.is_disjoint(c))
}

// ───────────────────────────── Version × Party ─────────────────────────────

/// Laws over a version and a live party.
///
/// The event laws (`tick` strictly advances, and only within the party's
/// region — §4's `e' = e + f·i`), the entry points' agreement (`tick` and
/// `ticks`, each across its two spellings), the fused multi-tick's point
/// laws (`ticks(0)` the identity, `ticks(1)` the tick, small counts
/// against the iterated ground truth, a fresh line realizing the tick
/// floor at any width), and the projection (`/`) point laws.
pub static VERSION_PARTY: &[Law<fn(&Version, &Party) -> bool>] = &[
    ("tick_strictly_advances", tick_strictly_advances),
    (
        "tick_only_inflates_the_region",
        tick_only_inflates_the_region,
    ),
    (
        "tick_advances_within_the_region",
        tick_advances_within_the_region,
    ),
    (
        "party_tick_matches_version_tick",
        party_tick_matches_version_tick,
    ),
    ("ticks_zero_is_identity", ticks_zero_is_identity),
    ("ticks_one_is_tick", ticks_one_is_tick),
    (
        "ticks_agrees_with_iterated_ticks",
        ticks_agrees_with_iterated_ticks,
    ),
    (
        "party_ticks_matches_version_ticks",
        party_ticks_matches_version_ticks,
    ),
    (
        "ticks_line_realizes_min_ticks",
        ticks_line_realizes_min_ticks,
    ),
    ("projection_is_sub_version", projection_is_sub_version),
    ("projection_idempotent", projection_idempotent),
    (
        "projection_additive_over_fork",
        projection_additive_over_fork,
    ),
];

/// `tick` strictly advances the causal order: `a < a.tick(p)`.
fn tick_strictly_advances(a: &Version, p: &Party) -> bool {
    let mut ticked = a.clone();
    ticked.tick(p);
    le(a, &ticked) && !le(&ticked, a) && *a != ticked
}

/// `tick` inflates only within the party's region (§4: `e' = e + f·i`, zero
/// outside `i`): projected onto the region's complement, the ticked version
/// is unchanged. Vacuous only for the seed party, which has no complement.
fn tick_only_inflates_the_region(a: &Version, p: &Party) -> bool {
    let mut ticked = a.clone();
    ticked.tick(p);
    match Party::seed().without(p) {
        None => true, // p owns the whole interval: nothing lies outside it
        Some(rest) => (&ticked / &rest) == (a / &rest),
    }
}

/// `tick`'s inflation is real *within* the region (§4: `f · i ⊐ 0`): the
/// projection onto the ticking party strictly advances.
fn tick_advances_within_the_region(a: &Version, p: &Party) -> bool {
    let mut ticked = a.clone();
    ticked.tick(p);
    (a / p).partial_cmp(&(&ticked / p)) == Some(Ordering::Less)
}

/// The two `tick` entry points agree: `version.tick(&party)` and
/// `party.tick(&mut version)` produce the same advance.
fn party_tick_matches_version_tick(a: &Version, p: &Party) -> bool {
    let mut via_version = a.clone();
    via_version.tick(p);
    let mut via_party = a.clone();
    p.tick(&mut via_party);
    via_version == via_party
}

/// `ticks(0)` is the identity: the empty run records nothing.
fn ticks_zero_is_identity(a: &Version, p: &Party) -> bool {
    let mut run = a.clone();
    run.ticks(p, 0u64);
    run == *a
}

/// `ticks(1)` is exactly `tick`: the fused multi-tick degenerates to the
/// single event.
fn ticks_one_is_tick(a: &Version, p: &Party) -> bool {
    let mut fused = a.clone();
    fused.ticks(p, 1u64);
    let mut ticked = a.clone();
    ticked.tick(p);
    fused == ticked
}

/// `ticks(n)` equals `n` sequential `tick`s, checked at every count a
/// short iterated run reaches (0..=3) — the ground-truth seam the wide
/// counts compose over ([`ticks_composes`] in the pair-party group).
fn ticks_agrees_with_iterated_ticks(a: &Version, p: &Party) -> bool {
    let mut iterated = a.clone();
    (0u64..=3).all(|n| {
        let mut fused = a.clone();
        fused.ticks(p, n);
        let agrees = fused == iterated;
        iterated.tick(p);
        agrees
    })
}

/// The two `ticks` entry points agree: `version.ticks(&party, n)` and
/// `party.ticks(&mut version, n)` produce the same advance.
fn party_ticks_matches_version_ticks(a: &Version, p: &Party) -> bool {
    let n = a.min_ticks();
    let mut via_version = a.clone();
    via_version.ticks(p, n.clone());
    let mut via_party = a.clone();
    p.ticks(&mut via_party, n);
    via_version == via_party
}

/// A fresh line realizes the tick floor exactly: `n` ticks on the empty
/// version at any one party cost floor `n`.
///
/// Quantified over the wide counts the version operand's own floor
/// supplies, all fused (no iteration at any width).
fn ticks_line_realizes_min_ticks(a: &Version, p: &Party) -> bool {
    let n = a.min_ticks();
    let mut line = Version::new();
    line.ticks(p, n.clone());
    line.min_ticks() == n
}

/// Projection keeps at most the history it is given: `a / p <= a`.
fn projection_is_sub_version(a: &Version, p: &Party) -> bool {
    le_by(&(a / p), a)
}

/// Projection is idempotent: `(a / p) / p == a / p`.
///
/// The inner projection is materialized — idempotence quantifies over the
/// projected *object* — and the outer one stays a view: the equality is
/// the fused view-vs-version comparison.
fn projection_idempotent(a: &Version, p: &Party) -> bool {
    let projected = (a / p).to_version();
    (&projected / p) == projected
}

/// Projection is additive across a fork: a party's contribution equals the
/// join of its two halves' contributions — the homomorphism the join/meet
/// distribution rests on.
///
/// The halves' contributions are materialized — the join needs its
/// operands as objects — and the whole-party side stays a view: the
/// equality is the fused version-vs-view comparison.
fn projection_additive_over_fork(a: &Version, p: &Party) -> bool {
    let mut keeper = p.dangerously_alias();
    let child = keeper.fork();
    ((a / &keeper).to_version() | (a / &child).to_version()) == (a / p)
}

// ───────────────────────────── Version × Version × Party ─────────────────────────────

/// Laws over two versions and a live party: projection as a lattice
/// homomorphism, its monotonicity in the version, and `ticks` as a
/// monoid action at the wide counts the operands' tick floors supply.
pub static VERSION_PAIR_PARTY: &[Law<fn(&Version, &Version, &Party) -> bool>] = &[
    ("projection_join_homomorphism", projection_join_homomorphism),
    ("projection_meet_homomorphism", projection_meet_homomorphism),
    (
        "projection_monotone_in_version",
        projection_monotone_in_version,
    ),
    ("ticks_composes", ticks_composes),
    (
        "own_version_cmp_matches_materialized",
        own_version_cmp_matches_materialized,
    ),
    (
        "own_version_seed_mask_coherence",
        own_version_seed_mask_coherence,
    ),
];

/// Projection is a homomorphism of the join: `(a | b) / p == (a/p) | (b/p)`
/// (the pointwise gate commutes with pointwise max).
///
/// The right-hand side's join needs its operands as objects, so the
/// per-operand projections materialize; the left-hand side stays a view.
fn projection_join_homomorphism(a: &Version, b: &Version, p: &Party) -> bool {
    let joined = a | b;
    (&joined / p) == ((a / p).to_version() | (b / p).to_version())
}

/// Projection is a homomorphism of the meet: `(a & b) / p == (a/p) & (b/p)`.
fn projection_meet_homomorphism(a: &Version, b: &Version, p: &Party) -> bool {
    let met = a & b;
    (&met / p) == ((a / p).to_version() & (b / p).to_version())
}

/// `ticks` is a monoid action of the naturals: `ticks(n)` then
/// `ticks(m)` equals `ticks(n + m)`.
///
/// Quantified over the wide counts the two version operands' tick
/// floors supply, all fused, so the law exercises counts no iterated
/// reference could reach.
fn ticks_composes(a: &Version, b: &Version, p: &Party) -> bool {
    let (n, m) = (a.min_ticks(), b.min_ticks());
    let mut stepwise = a.clone();
    stepwise.ticks(p, n.clone());
    stepwise.ticks(p, m.clone());
    let mut joint = a.clone();
    joint.ticks(p, n + m);
    stepwise == joint
}

/// Projection is monotone in the version: on the constructed comparable
/// pair `a <= a | b`, the projections compare the same way — and whenever
/// the inputs happen to compare directly, so do their projections.
fn projection_monotone_in_version(a: &Version, b: &Version, p: &Party) -> bool {
    let ab = a | b;
    let constructed = le_by(&(a / p), &(&ab / p));
    let incidental = !le(a, b) || le_by(&(a / p), &(b / p));
    constructed && incidental
}

/// The view's heterogeneous comparisons are the materialized projection's,
/// exactly: `(a/p) ⋚ b ≡ (a/p).to_version() ⋚ b`, in both operand orders
/// and under `==` — the three-stream differential law the fused co-walk
/// is pinned by.
fn own_version_cmp_matches_materialized(a: &Version, b: &Version, p: &Party) -> bool {
    let view = a / p;
    let materialized = view.to_version();
    let cmp_agrees = view.partial_cmp(b) == materialized.partial_cmp(b);
    let cmp_reversed_agrees = b.partial_cmp(&view) == b.partial_cmp(&materialized);
    let eq_agrees = (view == *b) == (materialized == *b);
    let eq_reversed_agrees = (*b == view) == (*b == materialized);
    cmp_agrees && cmp_reversed_agrees && eq_agrees && eq_reversed_agrees
}

/// A heterogeneous comparison is the homogeneous comparison against the
/// seed-masked view: `(a/p) ⋚ b ≡ (a/p) ⋚ (b/seed)` — sound because
/// projection by the seed party is the identity
/// ([`seed_projection_is_identity`]), and the coherence that makes the
/// three-stream walk a special case of the four-stream one.
fn own_version_seed_mask_coherence(a: &Version, b: &Version, p: &Party) -> bool {
    let view = a / p;
    let seed = Party::seed();
    let seeded = b / &seed;
    view.partial_cmp(&seeded) == view.partial_cmp(b) && (view == seeded) == (view == *b)
}

// ───────────────────────────── Version × Party × Party ─────────────────────────────

/// Laws over a version and two live parties: projection's interaction with
/// the region geometry.
pub static VERSION_PARTY_PAIR: &[Law<fn(&Version, &Party, &Party) -> bool>] = &[
    ("projection_commutes", projection_commutes),
    (
        "projection_monotone_in_region",
        projection_monotone_in_region,
    ),
    (
        "disjoint_projections_share_nothing",
        disjoint_projections_share_nothing,
    ),
];

/// Successive projections commute: `(v / p) / q == (v / q) / p` (both keep
/// exactly the history on the regions' intersection).
///
/// The inner projections materialize (the outer projection needs a
/// version to gate); the outer comparison is the fused view-vs-view walk.
fn projection_commutes(v: &Version, p: &Party, q: &Party) -> bool {
    let vp = (v / p).to_version();
    let vq = (v / q).to_version();
    (&vp / q) == (&vq / p)
}

/// Projection is monotone in the region: a constructed subregion (a fork
/// half of `p`) keeps no more than `p` does — and whenever `p` happens to
/// cover `q`, `v / q <= v / p`.
fn projection_monotone_in_region(v: &Version, p: &Party, q: &Party) -> bool {
    let mut keeper = p.dangerously_alias();
    let child = keeper.fork();
    let constructed = le_by(&(v / &child), &(v / p));
    let incidental = !p.covers(q) || le_by(&(v / q), &(v / p));
    constructed && incidental
}

/// Disjoint regions carve disjoint histories: `p · q = 0 ⟹ (v/p) & (v/q)`
/// is empty (the projections' supports cannot overlap).
fn disjoint_projections_share_nothing(v: &Version, p: &Party, q: &Party) -> bool {
    !p.is_disjoint(q) || ((v / p).to_version() & (v / q).to_version()).is_empty()
}

// ──────────────────── Version × Version × Party × Party ────────────────────

/// Laws over two versions and two live parties: the homogeneous view
/// comparison against its materialized oracle.
pub static VERSION_PAIR_PARTY_PAIR: &[Law<fn(&Version, &Version, &Party, &Party) -> bool>] = &[(
    "own_version_pair_cmp_matches_materialized",
    own_version_pair_cmp_matches_materialized,
)];

/// The view's homogeneous comparisons are the materialized projections',
/// exactly: `(a/p) ⋚ (b/q) ≡ (a/p).to_version() ⋚ (b/q).to_version()`,
/// under `==` too — the four-stream differential law the fused co-walk is
/// pinned by.
fn own_version_pair_cmp_matches_materialized(
    a: &Version,
    b: &Version,
    p: &Party,
    q: &Party,
) -> bool {
    let (va, vb) = (a / p, b / q);
    let (ma, mb) = (va.to_version(), vb.to_version());
    va.partial_cmp(&vb) == ma.partial_cmp(&mb) && (va == vb) == (ma == mb)
}

// ───────────────────────────── Rank: triples ─────────────────────────────

/// Laws over a triple of ranks.
///
/// `Rank` is a totally ordered commutative monoid: commutativity,
/// associativity, the `ZERO` identity and bottom, add-monotonicity,
/// `checked_sub` as the partial inverse defined exactly on domination, the
/// order's duality, and cross-path normalization (value-equal ranks built
/// along different operation paths are one structural value, equal under
/// `Eq` and `Hash`).
pub static RANK_TRIPLE: &[Law<fn(&Rank, &Rank, &Rank) -> bool>] = &[
    ("rank_add_commutative", rank_add_commutative),
    ("rank_add_associative", rank_add_associative),
    ("rank_zero_is_identity", rank_zero_is_identity),
    ("rank_zero_is_bottom", rank_zero_is_bottom),
    ("rank_add_monotone", rank_add_monotone),
    ("rank_sub_inverts_add", rank_sub_inverts_add),
    (
        "rank_checked_sub_iff_dominated",
        rank_checked_sub_iff_dominated,
    ),
    ("rank_sub_then_add_restores", rank_sub_then_add_restores),
    ("rank_cmp_antisymmetric", rank_cmp_antisymmetric),
    (
        "rank_cross_path_normalization",
        rank_cross_path_normalization,
    ),
];

/// Addition is commutative: `a + b == b + a`.
fn rank_add_commutative(a: &Rank, b: &Rank, _c: &Rank) -> bool {
    a + b == b + a
}

/// Addition is associative: `(a + b) + c == a + (b + c)`.
fn rank_add_associative(a: &Rank, b: &Rank, c: &Rank) -> bool {
    &(a + b) + c == a + &(b + c)
}

/// `ZERO` is the additive identity.
fn rank_zero_is_identity(a: &Rank, _b: &Rank, _c: &Rank) -> bool {
    a + &Rank::ZERO == a.clone()
}

/// `ZERO` is the order's bottom: no rank sits below it.
fn rank_zero_is_bottom(a: &Rank, _b: &Rank, _c: &Rank) -> bool {
    Rank::ZERO <= *a
}

/// Addition never shrinks a rank: `a + b >= a`.
fn rank_add_monotone(a: &Rank, b: &Rank, _c: &Rank) -> bool {
    &(a + b) >= a
}

/// `checked_sub` inverts addition: `(a + b) - b == a`.
fn rank_sub_inverts_add(a: &Rank, b: &Rank, _c: &Rank) -> bool {
    (a + b).checked_sub(b) == Some(a.clone())
}

/// `checked_sub` is defined exactly on domination: `a - b` is `Some` iff
/// `b <= a`.
fn rank_checked_sub_iff_dominated(a: &Rank, b: &Rank, _c: &Rank) -> bool {
    a.checked_sub(b).is_some() == (b <= a)
}

/// Where defined, subtraction restores: `(a - b) + b == a`.
fn rank_sub_then_add_restores(a: &Rank, b: &Rank, _c: &Rank) -> bool {
    match a.checked_sub(b) {
        Some(difference) => &difference + b == a.clone(),
        None => true,
    }
}

/// The total order is its own dual: `cmp(a, b)` is `cmp(b, a)` reversed.
fn rank_cmp_antisymmetric(a: &Rank, b: &Rank, _c: &Rank) -> bool {
    a.cmp(b) == b.cmp(a).reverse()
}

/// Value-equal ranks built along different operation paths — pairwise
/// addition, `Sum`, and add-then-subtract — are one structural value.
///
/// Equal under `Eq` and under `Hash`: the normalization invariant `Ord`'s
/// class-first fast path and every container key rest on.
fn rank_cross_path_normalization(a: &Rank, b: &Rank, c: &Rank) -> bool {
    let via_add = a + b;
    let via_sum = [a.clone(), b.clone()].into_iter().sum::<Rank>();
    let via_sub = (&(a + b) + c).checked_sub(c);
    via_add == via_sum && via_sub == Some(via_add.clone()) && hash_of(&via_add) == hash_of(&via_sum)
}

// ───────────────────────────── Clock: one value ─────────────────────────────

/// Laws over one clock.
///
/// The fork-event-join model's composite operations on a whole stamp:
/// `fork` preserves the event component and splits the id, `tick`/`send`
/// advance strictly and fix the party, `ticks` agrees with the version
/// entry point, peeks are stable, an own-message receive is a bare tick,
/// `sync` reconciles a fork, `own_version` is the projection, and the
/// parts/codec/text round-trips.
pub static CLOCK_SOLO: &[Law<fn(&Clock) -> bool>] = &[
    ("fork_preserves_version", fork_preserves_version),
    ("fork_splits_the_party", fork_splits_the_party),
    ("fork_join_restores_the_clock", fork_join_restores_the_clock),
    ("peek_is_stable", peek_is_stable),
    (
        "clock_tick_advances_and_fixes_party",
        clock_tick_advances_and_fixes_party,
    ),
    ("own_receive_is_tick", own_receive_is_tick),
    (
        "clock_ticks_matches_version_ticks",
        clock_ticks_matches_version_ticks,
    ),
    (
        "send_advances_and_returns_the_version",
        send_advances_and_returns_the_version,
    ),
    ("sync_reconciles_a_fork", sync_reconciles_a_fork),
    (
        "own_version_is_the_projection",
        own_version_is_the_projection,
    ),
    ("parts_roundtrip", parts_roundtrip),
    ("clock_codec_roundtrip", clock_codec_roundtrip),
    ("clock_text_roundtrip", clock_text_roundtrip),
    (
        "encode_frames_party_then_version",
        encode_frames_party_then_version,
    ),
    (
        "clock_encoded_bits_matches_encode_len",
        clock_encoded_bits_matches_encode_len,
    ),
];

/// `fork` preserves the version on both halves (§3: fork clones the causal
/// past).
fn fork_preserves_version(c: &Clock) -> bool {
    let mut keeper = c.dangerously_alias();
    let child = keeper.fork();
    keeper.version() == c.version() && child.version() == c.version()
}

/// `fork` splits the id: the two halves' parties are disjoint, and each is
/// covered by the original.
fn fork_splits_the_party(c: &Clock) -> bool {
    let mut keeper = c.dangerously_alias();
    let child = keeper.fork();
    keeper.party().is_disjoint(child.party())
        && c.party().covers(keeper.party())
        && c.party().covers(child.party())
}

/// `fork` then `join` restores the clock exactly: the party halves rejoin
/// and the version join is idempotent (`e ⊔ e == e`).
fn fork_join_restores_the_clock(c: &Clock) -> bool {
    let mut keeper = c.dangerously_alias();
    let child = keeper.fork();
    keeper.join(child).is_ok() && keeper == *c
}

/// `version()` (peek) does not advance the clock: repeated peeks are equal
/// and the clock's bytes are unchanged.
fn peek_is_stable(c: &Clock) -> bool {
    let before = c.encode();
    let first = c.version().clone();
    first == *c.version() && c.encode() == before
}

/// `tick` strictly advances the version and leaves the party untouched.
fn clock_tick_advances_and_fixes_party(c: &Clock) -> bool {
    let mut ticked = c.dangerously_alias();
    ticked.tick();
    le(c.version(), ticked.version())
        && c.version() != ticked.version()
        && ticked.party() == c.party()
}

/// The clock's `ticks` agrees with the version-level `ticks` on its own
/// parts, and returns the freshly advanced version.
fn clock_ticks_matches_version_ticks(c: &Clock) -> bool {
    let n = Ticks::from(3u64);
    let mut via_clock = c.dangerously_alias();
    let returned = via_clock.ticks(n.clone()).clone();
    let mut expected = c.version().clone();
    expected.ticks(c.party(), n);
    returned == expected && *via_clock.version() == expected
}

/// `receive` of a dominated message (here the clock's own version) equals a
/// bare `tick`: an own-message receive is benign.
fn own_receive_is_tick(c: &Clock) -> bool {
    let mut received = c.dangerously_alias();
    let mut ticked = c.dangerously_alias();
    let own = received.version().clone();
    received.recv(&own);
    ticked.tick();
    received == ticked
}

/// `send` (event then peek) returns the freshly advanced version: the
/// returned message equals the clock's version and strictly dominates the
/// pre-send version.
fn send_advances_and_returns_the_version(c: &Clock) -> bool {
    let mut sender = c.dangerously_alias();
    let sent = sender.send().clone();
    sent == *sender.version() && le(c.version(), &sent) && *c.version() != sent
}

/// `sync` (join then fork) reconciles a fork: after two concurrent ticks,
/// both sides end at the ticks' join, with disjoint parties whose rejoin
/// recovers the original region.
fn sync_reconciles_a_fork(c: &Clock) -> bool {
    let mut left = c.dangerously_alias();
    let mut right = left.fork();
    left.tick();
    right.tick();
    let want = left.version() | right.version();
    if left.sync(&mut right).is_err() {
        return false;
    }
    let reconciled = *left.version() == want
        && *right.version() == want
        && left.party().is_disjoint(right.party());
    let (left_party, _) = left.into_parts();
    let (right_party, _) = right.into_parts();
    let mut rejoined = left_party;
    reconciled && rejoined.join(right_party).is_ok() && &rejoined == c.party()
}

/// `own_version` is the projection of the version onto the party.
fn own_version_is_the_projection(c: &Clock) -> bool {
    c.own_version() == (c.version() / c.party())
}

/// `from_parts ∘ into_parts == id`.
fn parts_roundtrip(c: &Clock) -> bool {
    let (party, version) = c.dangerously_alias().into_parts();
    Clock::from_parts(party, version) == *c
}

/// `decode ∘ encode == id`, and the round-tripped clock re-encodes to the
/// same bytes.
fn clock_codec_roundtrip(c: &Clock) -> bool {
    let bytes = c.encode();
    Clock::decode(&bytes[..]).is_ok_and(|decoded| decoded == *c && decoded.encode() == bytes)
}

/// `FromStr ∘ Display == id`: the paper notation round-trips.
fn clock_text_roundtrip(c: &Clock) -> bool {
    c.to_string()
        .parse::<Clock>()
        .is_ok_and(|parsed| parsed == *c)
}

/// The clock's encoding is its party's bytes then its version's, exactly
/// (each part byte-aligned and independently canonical).
fn encode_frames_party_then_version(c: &Clock) -> bool {
    c.encode() == [c.party().encode(), c.version().encode()].concat()
}

/// `encoded_bits` is the pre-pad bit length of `encode`, at the clock level
/// too.
fn clock_encoded_bits_matches_encode_len(c: &Clock) -> bool {
    c.encode().len() == c.encoded_bits().div_ceil(8)
}

// ───────────────────────────── Clock × Version ─────────────────────────────

/// Laws over a clock and a message version.
///
/// The receive laws (join-then-event: the result dominates both the old
/// version and the message, strictly past their join, with the party
/// untouched) and the anonymous-join operators.
pub static CLOCK_VERSION: &[Law<fn(&Clock, &Version) -> bool>] = &[
    ("recv_learns_and_advances", recv_learns_and_advances),
    ("recv_fixes_party", recv_fixes_party),
    (
        "anonymous_join_merges_versions",
        anonymous_join_merges_versions,
    ),
];

/// `recv` (join then event) learns the message and advances past it: the
/// result dominates `old | msg` strictly, and the returned reference is the
/// clock's new version.
fn recv_learns_and_advances(c: &Clock, msg: &Version) -> bool {
    let mut receiver = c.dangerously_alias();
    let old = receiver.version().clone();
    let returned = receiver.recv(msg).clone();
    let now = receiver.version().clone();
    let lub = &old | msg;
    returned == now && le(&lub, &now) && lub != now
}

/// `recv` never changes the id: message reception is an anonymous join.
fn recv_fixes_party(c: &Clock, msg: &Version) -> bool {
    let mut receiver = c.dangerously_alias();
    receiver.recv(msg);
    receiver.party() == c.party()
}

/// The anonymous joins `Clock | Version` and `Version | Clock` merge the
/// versions and keep the clock's party, and agree with each other.
fn anonymous_join_merges_versions(c: &Clock, msg: &Version) -> bool {
    let clock_version = c.dangerously_alias() | msg.clone();
    let version_clock = msg.clone() | c.dangerously_alias();
    clock_version.party() == c.party()
        && *clock_version.version() == (c.version() | msg)
        && version_clock == clock_version
}

#[cfg(test)]
mod tests;
