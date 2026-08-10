//! The algebraic and representational laws of the public API, as named
//! predicates.
//!
//! Public under the `laws` feature so the fuzz workspace can drive the same
//! collection the in-tree proptests assert.
//!
//! Each law is a `(&str, fn(...) -> bool)` pair in a slice grouped by predicate
//! signature, so a harness iterates a slice, feeds every law the same inputs,
//! and reports the *name* of any law that fails. The crate's law proptests
//! drive these slices over generated inputs (arbitrary normal-form trees and
//! organic op-trace populations), and the law fuzz target drives them over
//! decoded hostile-but-canonical values; a law added here reaches every
//! consumer with no further wiring.
//!
//! The algebraic laws transcribe the ITC algebra (Almeida, Baquero & Fonte
//! 2008, §2–§4): versions form a distributive lattice under `|`/`&` whose
//! partial order is causality, ids form a partial commutative monoid under
//! disjoint join with `fork` as its splitting inverse, events inflate strictly
//! and only within the owned region, and `rank` is a strictly monotone
//! valuation. The representational laws pin the crate's own contracts: the
//! codec is a section of canonical bytes, `Eq`/`Hash` ride byte equality, text
//! round-trips, and [`Ranked`]'s total order linearly extends causality. Every
//! law holds unconditionally on the inputs its group admits (below);
//! conditional laws are stated as implications, vacuously true when the
//! antecedent fails, and where they can, they *construct* a witness for the
//! antecedent instead of waiting for one.
//!
//! # Group signatures and admissible inputs
//!
//! Groups are named by the borrowed inputs their predicates take: [`Version`]s
//! are any canonical versions, [`Party`]s are any *live* (non-anonymous)
//! parties — exactly what `decode` accepts and the crate can construct —
//! [`Rank`]s are any ranks, and [`Clock`]s are any canonical party/version
//! pairings. The list groups take a slice of the same inputs at *any* arity —
//! the length is a quantified variable, and every driver sweeps it across the
//! balanced fold's structural boundaries (the drivers' strategies document the
//! derivation) — and the receiver-and-items groups additionally distinguish the
//! element the operation's receiver supplies.
//!
//! # Linearity
//!
//! `Party` and `Clock` are `!Clone`, and the operations under law (`fork`,
//! `join`, `tick`, `without`, `sync`) consume or mutate their operands — a
//! shared borrow alone cannot exercise them. Every predicate therefore takes
//! shared borrows and materializes its own working copies with
//! [`Party::dangerously_alias`] / [`Clock::dangerously_alias`]: the aliases
//! live and die inside the predicate, which owns no clock universe, so the
//! linearity hazard the method documents (two live holders of one region) never
//! escapes a call. The laws quantify over a value's geometry, which aliasing
//! preserves exactly.
//!
//! # Fallible operations
//!
//! Laws over fallible operations (`Party::join` and friends return `Result`)
//! quantify over the *outcome*: both sides of an equation must agree in arm
//! (`Ok`/`Err`) **and** payload. "Join is commutative" means both orders accept
//! the same pairs and produce equal unions — and hand back equal values when
//! they refuse.

// The group statics are slices of (name, fn pointer) tuples: the fn-pointer
// signature IS the group's identity, so naming each one would only add
// indirection between a group and its shape.
#![allow(clippy::type_complexity)]

use core::cmp::Ordering;
use core::hash::{Hash, Hasher};

use crate::causally::{self, Coverage, Dominance, Endpoint, Placement, Precedence, Query, Span};
use crate::error::Crossed;
use crate::{Clock, Party, Rank, Ranked, Ticks, Version};

/// A named law: the name a failure reports, and the predicate that must
/// hold on every admissible input.
pub type Law<F> = (&'static str, F);

/// Apply a function to each algebraic law asserted about this crate.
///
/// Every consumer derives from this one list by callback: the macro
/// hands the entries to a consumer-supplied macro, forwarding an
/// optional argument clause (`consumer(args)`) ahead of them as
/// `args: (...)`. The derived consumers:
///
/// - `registered_names` and `REGISTERED_GROUPS` (this module's tests'
///   registration surface) chain it, so every name-facing check — the
///   uniqueness pin, the coverage roster's citation haystack — resolves
///   against exactly the entries the drivers run;
/// - the algebraic-laws suite expands it twice: into the per-group
///   proptest drivers (by the driver names carried here) and into the
///   organic-populations drive list;
/// - the law fuzz target expands it into its drive loop over decoded
///   hostile-but-canonical values.
///
/// A group added here is therefore *executed by construction* in every
/// consumer: each consumer keys its expansion arms on the input
/// signature, so a new group with a known signature is driven with no
/// further wiring, and one with a novel signature refuses to compile
/// until every consumer says how to feed it. The reverse door — a
/// `pub static` law group missing from this roster, which nothing would
/// ever execute — is closed by the totality pin in this module's tests,
/// which holds the roster equal to a source scan of the `pub static`
/// declarations in this file.
///
/// The signature kinds name what each predicate borrows: `version`,
/// `party`, `rank`, `clock` for single values, and `versions`,
/// `parties`, `clocks` for the variadic item lists.
#[cfg(any(test, feature = "laws"))]
#[macro_export]
macro_rules! for_each_law_group {
    ($callback:ident) => { $crate::for_each_law_group!($callback()); };
    ($callback:ident($($args:tt)*)) => {
        $callback! {
            args: ($($args)*);
            (VERSION_SOLO, version_solo_laws, (version)),
            (VERSION_PAIR, version_pair_laws, (version, version)),
            (VERSION_TRIPLE, version_triple_laws, (version, version, version)),
            (VERSION_LIST, version_list_laws, (versions)),
            (VERSION_AND_LIST, version_and_list_laws, (version, versions)),
            (PARTY_SOLO, party_solo_laws, (party)),
            (PARTY_PAIR, party_pair_laws, (party, party)),
            (PARTY_TRIPLE, party_triple_laws, (party, party, party)),
            (PARTY_AND_LIST, party_and_list_laws, (party, parties)),
            (VERSION_PARTY, version_party_laws, (version, party)),
            (VERSION_PAIR_PARTY, version_pair_party_laws, (version, version, party)),
            (VERSION_PARTY_PAIR, version_party_pair_laws, (version, party, party)),
            (VERSION_PAIR_PARTY_PAIR, version_pair_party_pair_laws, (version, version, party, party)),
            (RANK_TRIPLE, rank_triple_laws, (rank, rank, rank)),
            (CLOCK_SOLO, clock_solo_laws, (clock)),
            (CLOCK_PAIR, clock_pair_laws, (clock, clock)),
            (CLOCK_VERSION, clock_version_laws, (clock, version)),
            (CLOCK_AND_LIST, clock_and_list_laws, (clock, clocks)),
        }
    };
}

/// Emits the registration surface from the roster.
///
/// The name chain (`registered_names`) and the group list (`REGISTERED_GROUPS`)
/// — both test-only, so code spans rather than links here — expand from
/// `for_each_law_group!`'s single spelling, so they cannot drift from each
/// other or from what the derived drivers execute.
#[cfg(test)]
macro_rules! emit_registration {
    (args: (); $(($group:ident, $driver:ident, $shape:tt)),* $(,)?) => {
        /// Every registered law name, across all groups.
        ///
        /// The collection read from the tables themselves — the same entries
        /// the roster-derived drivers execute — so anything that consumes law
        /// names (the uniqueness pin, the coverage roster's citation check)
        /// resolves against what actually runs, never against a text scan that
        /// a stray same-named `fn` could satisfy.
        pub(crate) fn registered_names() -> Vec<&'static str> {
            std::iter::empty()
                $(.chain($group.iter().map(|(name, _)| *name)))*
                .collect()
        }

        /// Every group static the roster carries, by name — the same single
        /// list, stringified, for the totality pin against the `pub static`
        /// declarations in this file.
        pub(crate) const REGISTERED_GROUPS: &[&str] = &[$(stringify!($group)),*];
    };
}

#[cfg(test)]
crate::for_each_law_group!(emit_registration);

/// Declares one law group: the group's `pub static` slice and every predicate
/// in it, from a single spelling.
///
/// The header names the group and the parameters every law in the block shares;
/// each `fn` that follows is one law. The macro gives the predicate the
/// header's parameters and its `-> bool` return, and registers it in the slice
/// under its own name (`stringify!`ed) — a law cannot be written without being
/// registered, nor registered under a name that is not its own. A law that
/// deliberately ignores an input restates the parameter list with its own names
/// (`fn law(a, b, _c) { ... }`), arity-checked against the header; the types
/// are always the header's.
///
/// Group membership is the block: helper `fn`s live outside `laws!`, so nothing
/// a block contains can escape registration. Registration in the roster
/// (`for_each_law_group!`) stays a separate step, and the totality pin in this
/// module's tests closes it by comparing the roster against a source scan of
/// lines declaring a `pub static` — which is why the header keeps that literal
/// spelling.
macro_rules! laws {
    // In both the matcher and the transcriber, attributes and the declaration
    // share a line: the totality pin's source scan reads any line starting `pub
    // static` as a group declaration, and must see the invocations' headers
    // only, never this definition.
    (
        $(#[$group_meta:meta])* pub static $group:ident: ($($param:ident: $ty:ty),+ $(,)?);
        $(
            $(#[$law_meta:meta])*
            fn $law:ident $(($($rename:ident),+ $(,)?))? $body:block
        )+
    ) => {
        $(#[$group_meta])* pub static $group: &[Law<fn($($ty),+) -> bool>] = &[$((stringify!($law), $law)),+];
        laws! {
            @laws ($($param: $ty),+);
            $(
                $(#[$law_meta])*
                fn $law $(($($rename),+))? $body
            )+
        }
    };
    // Peel one law at a time: the header parameters are re-carried to every law
    // as a plain token list, which sidesteps the transcriber depth rule (a
    // header-level repetition cannot be re-expanded inside the per-law
    // repetition above).
    (@laws ($($params:tt)+);) => {};
    (
        @laws ($($params:tt)+);
        $(#[$law_meta:meta])*
        fn $law:ident $body:block
        $($rest:tt)*
    ) => {
        laws! { @law ($($params)+); $(#[$law_meta])* fn $law $body }
        laws! { @laws ($($params)+); $($rest)* }
    };
    (
        @laws ($($params:tt)+);
        $(#[$law_meta:meta])*
        fn $law:ident ($($rename:ident),+ $(,)?) $body:block
        $($rest:tt)*
    ) => {
        laws! { @law ($($params)+); $(#[$law_meta])* fn $law ($($rename),+) $body }
        laws! { @laws ($($params)+); $($rest)* }
    };
    (
        @law ($($param:ident: $ty:ty),+ $(,)?);
        $(#[$law_meta:meta])*
        fn $law:ident $body:block
    ) => {
        $(#[$law_meta])*
        fn $law($($param: $ty),+) -> bool $body
    };
    (
        @law ($($param:ident: $ty:ty),+ $(,)?);
        $(#[$law_meta:meta])*
        fn $law:ident ($($rename:ident),+) $body:block
    ) => {
        $(#[$law_meta])*
        fn $law($($rename: $ty),+) -> bool $body
    };
}

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

laws! {
    /// Laws over one version.
    ///
    /// The lattice point laws at a single value (idempotence, the bottom
    /// element), observer coherence (`is_empty`, `concurrent`, `distance`,
    /// `rank`, `min_ticks`, [`Ranked`]), and the representational round-trips
    /// (codec, text, byte views).
    pub static VERSION_SOLO: (a: &Version);

    /// Idempotence: `a | a == a` (the LUB of a value and itself is that value).
    fn merge_idempotent {
        (a.clone() | a.clone()) == *a
    }

    /// Idempotence: `a & a == a` (the GLB of a value and itself is that value).
    fn meet_idempotent {
        (a.clone() & a.clone()) == *a
    }

    /// Reflexivity: `a` compares `Equal` to itself (the canonical-bit
    /// short-circuit never reports an inequality).
    fn order_reflexive {
        a.partial_cmp(a) == Some(Ordering::Equal)
    }

    /// `Version::new()` is the lattice bottom: below every version.
    fn new_is_the_bottom {
        le(&Version::new(), a)
    }

    /// The bottom is the join identity: `new | a == a`.
    fn merge_new_is_identity {
        (Version::new() | a.clone()) == *a
    }

    /// The bottom absorbs the meet: `new & a == new`.
    fn meet_new_is_absorbing {
        (Version::new() & a.clone()) == Version::new()
    }

    /// `is_empty` recognizes exactly the bottom: `a.is_empty() ⟺ a == new`.
    fn is_empty_iff_new {
        a.is_empty() == (*a == Version::new())
    }

    /// Concurrency is irreflexive: a version is never concurrent with itself.
    fn never_concurrent_with_self {
        !a.concurrent(a)
    }

    /// The metric point law at the diagonal: `d(a, a) == 0`.
    fn distance_to_self_is_zero {
        a.distance(a) == Rank::ZERO
    }

    /// The directed metric's point law at the diagonal: `a.lag(a) == 0` —
    /// a version lags itself by nothing (`rank(a | a) == rank(a)` by
    /// idempotence).
    fn lag_to_self_is_zero {
        a.lag(a) == Rank::ZERO
    }

    /// The hull at the diagonal: a version's span with itself is the
    /// coincident span `[a, a]` — both endpoints equal `a`, and the empty
    /// n-ary hull (`span_all` of nothing) agrees with it.
    fn span_with_self_is_coincident {
        let span = a.span(a);
        span.lo() == a && span.hi() == a && a.span_all(core::iter::empty::<Version>()) == span
    }

    /// The coincident constructors are the singleton hull: `Span::at`,
    /// the consuming `From<Version>` door, and the lending
    /// `From<&Version>` door all build exactly the pair hull `a.span(&a)`.
    fn at_is_the_coincident_hull {
        let hull = a.span(a);
        Span::at(a.clone()) == hull && Span::from(a.clone()) == hull && Span::from(a) == hull
    }

    /// `rank` separates the bottom: zero area exactly for the empty version
    /// (rank is a strictly monotone valuation, so only the zero function has
    /// zero area).
    fn rank_zero_iff_empty {
        (a.rank() == Rank::ZERO) == a.is_empty()
    }

    /// `min_ticks` separates the bottom: a zero tick floor exactly for the
    /// empty version (the floor is a sum of nonnegative bases, zero only
    /// when every base is).
    fn min_ticks_zero_iff_empty {
        (a.min_ticks() == Ticks::ZERO) == a.is_empty()
    }

    /// The whole-interval party is the projection identity: `a / seed == a`.
    fn seed_projection_is_identity {
        (a / &Party::seed()) == *a
    }

    /// [`Ranked`] carries exactly its version's rank, and its key encoding
    /// carries exactly the view.
    ///
    /// Every entry views the same version (both `From` constructors and
    /// the `Version::ranked` method spelling), `rank` (and the `From`
    /// materialization, and the fused `encode_rank`) realize exactly
    /// `Version::rank`'s value, the composite `encode` is the rank
    /// encoding followed by the version's canonical bytes,
    /// `decode ∘ encode` is the identity exactly (same version, equal
    /// view), `Hash` is the viewed version's hash (the delegation `Eq`
    /// coherence rides on), and settling the borrow (`into_owned`)
    /// changes nothing.
    // The owned-instance comparison is the point: the law exercises the
    // `From<Ranked> for Rank` materialization itself.
    #[allow(clippy::cmp_owned)]
    fn ranked_carries_own_rank {
        let ranked = Ranked::from(a);
        ranked.version() == a
            && ranked.rank() == a.rank()
            && Rank::from(ranked.clone()) == a.rank()
            && ranked.encode_rank() == a.rank().encode()
            && ranked.encode() == [a.rank().encode(), a.as_bytes().to_vec()].concat()
            && Ranked::decode(&ranked.encode()[..])
                .is_ok_and(|decoded| decoded.version() == a && decoded == ranked)
            && hash_of(&ranked) == hash_of(a)
            && {
                let owned = Ranked::from(a.clone()).into_owned();
                owned.version() == a && owned.rank() == a.rank()
            }
            && {
                let method = a.ranked();
                method.version() == a && method.rank() == a.rank()
            }
    }

    /// `decode ∘ encode == id`, and the round-tripped value re-encodes to the
    /// same bytes (the codec is a section of canonical bytes — what byte-level
    /// `Eq`/`Hash` rest on).
    fn version_codec_roundtrip {
        let bytes = a.encode();
        Version::decode(&bytes[..]).is_ok_and(|decoded| decoded == *a && decoded.encode() == bytes)
    }

    /// `FromStr ∘ Display == id`: the paper notation round-trips.
    fn version_text_roundtrip {
        a.to_string()
            .parse::<Version>()
            .is_ok_and(|parsed| parsed == *a)
    }

    /// The borrowed byte view is the encoding: `as_bytes == encode`.
    fn version_as_bytes_matches_encode {
        a.as_bytes() == &a.encode()[..]
    }

    /// `encoded_bits` is the pre-padding bit length of `encode`: the byte length
    /// is the bit length plus the marker, rounded up to whole bytes.
    fn version_encoded_bits_matches_encode_len {
        a.encode().len() == (a.encoded_bits() + 1).div_ceil(8)
    }
}

// ───────────────────────────── Version: pairs ─────────────────────────────

laws! {
    /// Laws over a pair of versions.
    ///
    /// Commutativity and the bound laws of the lattice operations, absorption,
    /// the partial order's pair laws and their coherence with `Eq`/`Hash` and
    /// `concurrent`, the valuation identity tying `rank` to the lattice, the
    /// `distance`/`lag` metric laws, [`Ranked`]'s total order and its
    /// lexicographic key encoding, the degenerate-span identity tying span
    /// placement back to pairwise comparison, the pair span's definitional pin,
    /// the span wire form's round-trip, and the version encoding's
    /// prefix-freedom.
    pub static VERSION_PAIR: (a: &Version, b: &Version);

    /// Commutativity: `a | b == b | a` (the LUB does not depend on operand
    /// order).
    fn merge_commutative {
        (a | b) == (b | a)
    }

    /// Commutativity: `a & b == b & a` (the GLB does not depend on operand
    /// order).
    fn meet_commutative {
        (a & b) == (b & a)
    }

    /// The join is an upper bound: `a <= a | b` and `b <= a | b` — what ties
    /// `|` to the causal order.
    fn merge_is_upper_bound {
        let ab = a | b;
        le(a, &ab) && le(b, &ab)
    }

    /// The meet is a lower bound: `a & b <= a` and `a & b <= b`, the dual of
    /// [`merge_is_upper_bound`].
    fn meet_is_lower_bound {
        let ab = a & b;
        le(&ab, a) && le(&ab, b)
    }

    /// Absorption ties `&` and `|` into a lattice: `a & (a | b) == a` and
    /// `a | (a & b) == a`.
    fn meet_join_absorption {
        (a & &(a | b)) == *a && (a | &(a & b)) == *a
    }

    /// [`Version::join`] is the spelled form of `|`: equal to the operator on
    /// every pair (equality on `Version` is canonical byte equality), so the
    /// named method inherits the operator's differential and law coverage.
    fn join_method_is_the_operator {
        a.join(b) == (a | b)
    }

    /// [`Version::meet`] is the spelled form of `&`, dual to
    /// [`join_method_is_the_operator`]: equal to the operator on every pair,
    /// so the named method inherits the operator's differential and law
    /// coverage.
    fn meet_method_is_the_operator {
        a.meet(b) == (a & b)
    }

    /// The full `^` (BitXor) matrix over owned and borrowed operands equals
    /// [`Version::span`]: every cell is the same pair hull, endpoints and all.
    ///
    /// The hull itself is pinned by [`span_is_the_pair_hull`]; this law pins
    /// each operator cell's delegation to it.
    fn span_operator_matrix_is_the_method {
        let expected = a.span(b);
        (a.clone() ^ b.clone()) == expected
            && (a ^ b.clone()) == expected
            && (a.clone() ^ b) == expected
            && (a ^ b) == expected
    }

    /// Antisymmetry: `a <= b && b <= a ⟹ a == b` (mutually dominating versions
    /// denote the same history, so their canonical bytes coincide).
    fn order_antisymmetric {
        !(le(a, b) && le(b, a)) || a == b
    }

    /// Domination absorbs: `a <= b ⟹ a | b == b && a & b == a`.
    fn order_absorbing {
        !le(a, b) || ((a | b) == *b && (a & b) == *a)
    }

    /// `Eq` and the order agree: `a == b ⟺ partial_cmp == Some(Equal)`.
    fn eq_iff_cmp_equal {
        (a == b) == (a.partial_cmp(b) == Some(Ordering::Equal))
    }

    /// The order is its own dual: `cmp(a, b)` is `cmp(b, a)` reversed
    /// (including the concurrent `None`).
    fn partial_cmp_is_dual {
        a.partial_cmp(b) == b.partial_cmp(a).map(Ordering::reverse)
    }

    /// `concurrent` is exactly incomparability, and symmetric.
    fn concurrent_iff_incomparable {
        a.concurrent(b) == a.partial_cmp(b).is_none() && a.concurrent(b) == b.concurrent(a)
    }

    /// The valuation law: `rank(a|b) + rank(a&b) == rank(a) + rank(b)` (area
    /// is a lattice valuation because `max + min == sum` holds pointwise) —
    /// the identity that makes [`Version::distance`] a metric.
    fn rank_is_a_valuation {
        (a | b).rank() + (a & b).rank() == a.rank() + b.rank()
    }

    /// `rank` is strictly monotone on the causal order: `a <= b ⟹ rank(a) <=
    /// rank(b)`, strictly when `a != b`.
    fn rank_strictly_monotone {
        !le(a, b) || (a.rank() <= b.rank() && (a == b || a.rank() < b.rank()))
    }

    /// The metric symmetry law: `d(a, b) == d(b, a)`.
    fn distance_symmetric {
        a.distance(b) == b.distance(a)
    }

    /// The metric separates points: `d(a, b) == 0 ⟺ a == b`.
    fn distance_separates {
        (a.distance(b) == Rank::ZERO) == (a == b)
    }

    /// `distance` is the valuation gap across the lattice interval:
    /// `d(a, b) == rank(a|b) - rank(a&b)` (the join dominates the meet, so the
    /// subtraction is defined).
    fn distance_is_the_rank_gap {
        (a | b).rank().checked_sub(&(a & b).rank()) == Some(a.distance(b))
    }

    /// `lag` is the directed half of `distance`: the two directions sum to it.
    fn lag_halves_sum_to_distance {
        a.lag(b) + b.lag(a) == a.distance(b)
    }

    /// `lag` vanishes exactly when there is nothing left to learn:
    /// `a.lag(b) == 0 ⟺ b <= a`.
    fn lag_zero_iff_dominated {
        (a.lag(b) == Rank::ZERO) == le(b, a)
    }

    /// `lag` is the valuation gap up to the join: `a.lag(b) == rank(a|b) -
    /// rank(a)`.
    fn lag_is_the_rank_gap {
        (a | b).rank().checked_sub(&a.rank()) == Some(a.lag(b))
    }

    /// `Eq` is canonical-byte equality: `a == b ⟺ encode(a) == encode(b)`.
    fn version_eq_iff_bytes_eq {
        (a == b) == (a.encode() == b.encode())
    }

    /// Version canonical byte encodings are prefix-free: distinct versions'
    /// `as_bytes` are never byte prefixes of one another.
    ///
    /// The property the composite [`Ranked`] key's suffix safety rests on for
    /// its version component, pinned directly rather than inferred from the
    /// stream being bit-self-delimiting: a strict bit-prefix that were itself
    /// canonical would make the longer stream carry live bits past a complete
    /// tree, which the strict decoder rejects — this pins that argument's
    /// conclusion.
    fn version_encoding_is_prefix_free {
        a == b
            || (!a.as_bytes().starts_with(b.as_bytes())
                && !b.as_bytes().starts_with(a.as_bytes()))
    }

    /// `Eq`/`Hash` coherence: equal versions hash equally.
    fn version_eq_implies_hash_eq {
        a != b || hash_of(a) == hash_of(b)
    }

    /// [`Ranked`]'s total order is rank order completed by the version-byte
    /// tiebreak, exactly.
    ///
    /// The fused co-walk equals the materialized `Rank` order wherever the
    /// ranks differ, rank ties resolve by the versions' canonical bytes,
    /// equality is version identity, the explicit spelling of the rank question
    /// (`rank`, then [`Rank`]'s own comparison) answers exactly the
    /// materialized rank order — and the order therefore extends causality
    /// (causally ordered versions compare the same way, by rank strict
    /// monotonicity; only ties fall to the causally-free tiebreak).
    fn ranked_orders_by_rank_then_bytes {
        let (ra, rb) = (Ranked::from(a), Ranked::from(b));
        let rank_want = a.rank().cmp(&b.rank());
        let want = rank_want.then_with(|| a.as_bytes().cmp(b.as_bytes()));
        let fused = ra.cmp(&rb) == want && rb.cmp(&ra) == want.reverse();
        let eq = (ra == rb) == (a == b);
        let explicit = ra.rank().cmp(&rb.rank()) == rank_want
            && (ra.rank() == b.rank()) == (rank_want == Ordering::Equal);
        let extends = match a.partial_cmp(b) {
            Some(ord) => want == ord,
            None => true, // concurrent: rank or tiebreak orders them
        };
        fused && eq && explicit && extends
    }

    /// The composite key encoding is lexicographic, totally: byte order on
    /// [`Ranked::encode`] equals [`Ord`] on the views — ties included, so byte
    /// equality on keys is exactly `Eq` (version identity).
    fn ranked_encoding_orders_like_ord {
        let (ra, rb) = (Ranked::from(a), Ranked::from(b));
        let (ea, eb) = (ra.encode(), rb.encode());
        ea.cmp(&eb) == ra.cmp(&rb) && (ea == eb) == (ra == rb)
    }

    /// [`Span::new`] admits exactly the ordered pairs, and builds exactly the
    /// pair it was given.
    ///
    /// The gate is the causal order itself: `Span::new(a, b)` is `Ok` ⟺
    /// `a <= b` (concurrent and strictly reversed pairs alike are refused,
    /// with the payload-free [`Crossed`] as the whole verdict), and an
    /// admitted span's endpoints are byte-identical to the arguments — the
    /// validating door adds nothing and reorders nothing.
    fn span_gate_admits_exactly_the_ordered {
        match Span::new(a, b) {
            Ok(span) => le(a, b) && span.lo() == a && span.hi() == b,
            Err(Crossed) => !le(a, b),
        }
    }

    /// `place` against the degenerate span `[v, v]` is pairwise comparison
    /// itself.
    ///
    /// The four verdicts reachable with coincident endpoints transcribe
    /// `partial_cmp`'s four outcomes, and the five endpoint-splitting verdicts
    /// are unreachable.
    fn degenerate_span_place_is_partial_cmp {
        let Ok(span) = Span::new(b, b) else {
            // A version is always ordered with itself.
            return false;
        };
        for probe in [a, b] {
            let expect = match probe.partial_cmp(b) {
                Some(Ordering::Less) => Placement::Before,
                Some(Ordering::Equal) => Placement::At(Endpoint::Both),
                Some(Ordering::Greater) => Placement::After,
                None => Placement::Concurrent(Endpoint::Both),
            };
            if span.place(probe) != expect {
                return false;
            }
        }
        true
    }

    /// The pair span: endpoints the pair's meet and join, commutative,
    /// subsuming the flip repair on comparable pairs, coherent with the n-ary
    /// form at its edges, and preserved exactly by the accessors and the borrow
    /// mechanics.
    ///
    /// The n-ary edges: the empty iterator is the coincident `[self, self]`,
    /// one item is the binary span. The accessors: `meet`/`join` borrow the
    /// endpoints; `into_parts` hands them out owned, in `(meet, join)` order.
    /// The borrow mechanics: `reborrow` reads the same endpoints back
    /// byte-equal (`Version` equality *is* byte equality, pinned by
    /// `version_eq_iff_bytes_eq`), and `into_owned` settles the borrows with
    /// the endpoints byte-equal too — neither moves a value, so `lo <= hi`
    /// rides through both.
    ///
    /// On a comparable pair the hull *is* the reordered pair (either
    /// orientation yields the validated span); on a concurrent pair the
    /// meet/join bracket is the only span containing both.
    fn span_is_the_pair_hull {
        let hull = a.span(b);
        let (meet, join) = (a & b, a | b);
        let definitional = hull == Span::new(&meet, &join).unwrap();
        let accessors = *hull.lo() == meet && *hull.hi() == join && {
            let (lo, hi) = a.span(b).into_parts();
            lo == meet && hi == join
        };
        let reborrowed = {
            let view = hull.reborrow();
            view == hull && *view.lo() == meet && *view.hi() == join
        };
        let settled = {
            // The settling copy (borrowed endpoints) and the free passthrough
            // (already-owned endpoints, the derived hull's state) both preserve
            // the endpoints exactly.
            let copied: Span<'static> = Span::new(&meet, &join)
                .expect("a meet/join pair is ordered")
                .into_owned();
            let passed: Span<'static> = hull.clone().into_owned();
            copied == hull
                && *copied.lo() == meet
                && *copied.hi() == join
                && passed == hull
                && *passed.lo() == meet
                && *passed.hi() == join
        };
        let commutative = hull == b.span(a);
        let flip_subsumed = match a.partial_cmp(b) {
            Some(Ordering::Less | Ordering::Equal) => hull == Span::new(a, b).unwrap(),
            Some(Ordering::Greater) => hull == Span::new(b, a).unwrap(),
            None => true, // no reordering exists; the bracket is definitional
        };
        let empty_edge = a.span_all(core::iter::empty::<&Version>()) == Span::at(a);
        let unary_edge = a.span_all([b]) == hull;
        definitional
            && accessors
            && reborrowed
            && settled
            && commutative
            && flip_subsumed
            && empty_edge
            && unary_edge
    }

    /// The span wire form: `encode` is the meet's encoding followed by the
    /// join's, and `decode ∘ encode` is the identity exactly.
    ///
    /// Quantified over the pair's hull (every valid span is some pair's hull)
    /// and the coincident span `[a, a]`: each round-trips to an equal span, and
    /// the round-tripped span re-encodes to the same bytes — the composite is a
    /// section of canonical bytes, so byte equality on encodings is span
    /// equality.
    fn span_codec_roundtrip {
        let hull = a.span(b);
        let bytes = hull.encode();
        let framed = bytes == [hull.lo().encode(), hull.hi().encode()].concat();
        let round = Span::decode(&bytes[..])
            .is_ok_and(|decoded| decoded == hull && decoded.encode() == bytes);
        let coincident = {
            let span = a.span(a);
            Span::decode(&span.encode()[..]).is_ok_and(|decoded| decoded == span)
        };
        framed && round && coincident
    }

    /// Every causal atom's membership is exactly its order relation, and every
    /// negated atom keeps exactly the complement.
    ///
    /// The eight atomic bounds are transcribed row by row from `partial_cmp`:
    /// the four elementary atoms are the four order relations against the bound
    /// version (concurrency failing all four), and `!` on each keeps precisely
    /// the probes the atom drops — which is where "or concurrent" enters the
    /// query language. Checked in both probe/bound orientations, so the
    /// self-dual corner (`a == b`) and both strict sides are reached on every
    /// call.
    fn atom_membership_matches_relations {
        for (p, q) in [(a, b), (b, a)] {
            let rel = p.partial_cmp(q);
            let le = matches!(rel, Some(Ordering::Less | Ordering::Equal));
            let lt = rel == Some(Ordering::Less);
            let ge = matches!(rel, Some(Ordering::Greater | Ordering::Equal));
            let gt = rel == Some(Ordering::Greater);
            let atoms = causally::after(q).contains(p) == ge
                && causally::strictly_after(q).contains(p) == gt
                && causally::before(q).contains(p) == le
                && causally::strictly_before(q).contains(p) == lt;
            let complements = (!causally::before(q)).contains(p) == !le
                && (!causally::after(q)).contains(p) == !ge;
            // `or_concurrent` widens an atom's relation by incomparability,
            // which is exactly the complement of the opposite side's strict
            // relation.
            let concurrent = rel.is_none();
            let widened = causally::after(q).or_concurrent().contains(p) == (ge || concurrent)
                && causally::after(q).or_concurrent().contains(p) == !lt
                && causally::before(q).or_concurrent().contains(p) == (le || concurrent)
                && causally::before(q).or_concurrent().contains(p) == !gt;
            if !(atoms && complements && widened) {
                return false;
            }
        }
        true
    }

    /// The named query shorthands and conversions equal the expressions they
    /// abbreviate, behaviorally.
    ///
    /// `since` is `!before`, `until` is `!after`, `delta` is `since & before`,
    /// `toward` is `after & until`, `all` admits everything; a [`Span`]
    /// converts to its segment's query (`after(meet) & before(join)`, consuming
    /// and borrowing spellings agreeing) and a [`Version`] to the singleton
    /// query admitting exactly itself. Behavioral equations only: a query's
    /// observation surface is membership, deliberately not identity
    /// (`causally`'s module docs carry the no-`Eq` decision).
    fn query_shorthands_are_their_expressions {
        let span = a.span(b);
        for p in [a, b] {
            let since = causally::since(a).contains(p) == (!causally::before(a)).contains(p);
            let until = causally::until(a).contains(p) == (!causally::after(a)).contains(p);
            let delta = causally::delta(a, b).contains(p)
                == (causally::since(a) & causally::before(b)).contains(p);
            let toward = causally::toward(a, b).contains(p)
                == (causally::after(a) & causally::until(b)).contains(p);
            let all = causally::all().contains(p);
            let segment = Query::from(&span).contains(p)
                == (causally::after(span.lo()) & causally::before(span.hi())).contains(p)
                && Query::from(span.clone()).contains(p) == Query::from(&span).contains(p);
            let singleton = Query::from(a).contains(p) == (p == a)
                && Query::from(a.clone()).contains(p) == (p == a);
            if !(since && until && delta && toward && all && segment && singleton) {
                return false;
            }
        }
        true
    }
}

// ───────────────────────────── Version: triples ─────────────────────────────

laws! {
    /// Laws over a triple of versions.
    ///
    /// Associativity, the least/greatest bound laws, both distributive laws,
    /// transitivity (constructed and incidental), the metric and quasi-metric
    /// triangle inequalities with `lag`'s monotonicities, and
    /// the [`causally`] query and placement laws: conjunction as pointwise
    /// intersection across every atomic operand pairing and every typed `&`
    /// form, coverage's sound arms and its point degeneracy to membership, the
    /// segment-query/span-placement tie, the nine-way [`Span::place`] verdict
    /// as a pure transcription of the two endpoint comparisons, its
    /// coarsening to `dominance`, and the span operators' associativity in
    /// both of the span algebra's lattices.
    pub static VERSION_TRIPLE: (a: &Version, b: &Version, c: &Version);

    /// Associativity: `(a | b) | c == a | (b | c)` — with commutativity and
    /// idempotence, `|` is a join-semilattice operation.
    fn merge_associative {
        (&(a | b) | c) == (a | &(b | c))
    }

    /// Associativity: `(a & b) & c == a & (b & c)`, the meet dual.
    fn meet_associative {
        (&(a & b) & c) == (a & &(b & c))
    }

    /// The join is the *least* upper bound: the constructed common upper bound
    /// `a | b | c` dominates `a | b` (any common upper bound of `a` and `b`
    /// dominates their join).
    fn merge_is_least_upper_bound {
        let ab = a | b;
        let upper = &ab | c;
        le(a, &upper) && le(b, &upper) && le(&ab, &upper)
    }

    /// The meet is the *greatest* lower bound: the constructed common lower
    /// bound `a & b & c` is dominated by `a & b`, the dual of
    /// [`merge_is_least_upper_bound`].
    fn meet_is_greatest_lower_bound {
        let ab = a & b;
        let lower = &ab & c;
        le(&lower, a) && le(&lower, b) && le(&lower, &ab)
    }

    /// Meet distributes over join: `a & (b | c) == (a & b) | (a & c)`. The
    /// version lattice embeds in a function space into the chain of naturals
    /// (pointwise min/max), so it is distributive.
    fn meet_distributes_over_join {
        (a & &(b | c)) == (&(a & b) | &(a & c))
    }

    /// Join distributes over meet: `a | (b & c) == (a | b) & (a | c)`, the dual
    /// law (in a lattice each distributive law implies the other; asserting
    /// both guards an impl that realized only one direction).
    fn join_distributes_over_meet {
        (a | &(b & c)) == (&(a | b) & &(a | c))
    }

    /// Transitivity on a constructed chain: `a <= a|b <= a|b|c` holds by the
    /// upper-bound law, so the endpoints must compare — arbitrary inputs rarely
    /// chain by chance, so the chain is built rather than awaited.
    fn order_transitive_constructed {
        let mid = a | b;
        let hi = &mid | c;
        le(a, &mid) && le(&mid, &hi) && le(a, &hi)
    }

    /// Transitivity, incidental: whenever three arbitrary versions happen to
    /// chain (`a <= b` and `b <= c`), the endpoints must too.
    fn order_transitive_incidental {
        !(le(a, b) && le(b, c)) || le(a, c)
    }

    /// The triangle inequality: `d(a, c) <= d(a, b) + d(b, c)` — the defining
    /// metric law, which holds because the strictly monotone valuation `rank`
    /// lives on a *distributive* lattice.
    fn distance_triangle_inequality {
        a.distance(c) <= a.distance(b) + b.distance(c)
    }

    /// The directed triangle inequality: `a.lag(c) <= a.lag(b) + b.lag(c)` —
    /// the quasi-metric law for the directed half of `distance`.
    ///
    /// Holds by rank's modularity ([`rank_is_a_valuation`]) plus
    /// monotonicity: `rank(a|b) + rank(b|c) == rank(a|b|c) + rank((a|b) &
    /// (b|c)) >= rank(a|c) + rank(b)`, and subtracting `rank(a) + rank(b)`
    /// from both sides leaves the lags.
    fn lag_triangle_inequality {
        a.lag(c) <= a.lag(b) + b.lag(c)
    }

    /// `lag` is monotone in the message: a larger message leaves at least as
    /// much to learn.
    ///
    /// Constructed on `b <= b | c` (so the comparable pair exists on every
    /// call), and incidentally whenever the operands happen to compare —
    /// `b <= c ⟹ a.lag(b) <= a.lag(c)`, by the join's monotonicity under
    /// the rank valuation.
    fn lag_monotone_in_the_message {
        let constructed = a.lag(b) <= a.lag(&(b | c));
        let incidental = !le(b, c) || a.lag(b) <= a.lag(c);
        constructed && incidental
    }

    /// `lag` is antitone in the receiver: learning more leaves less to
    /// learn.
    ///
    /// Constructed on `a <= a | c`, and incidentally whenever the operands
    /// happen to compare — `a <= a' ⟹ a'.lag(b) <= a.lag(b)`, by rank's
    /// modularity applied to `u = a | b`, `v = a'` (the meet `(a|b) & a'`
    /// dominates `a`, so the gap can only shrink).
    fn lag_antitone_in_the_receiver {
        let constructed = (a | c).lag(b) <= a.lag(b);
        let incidental = !le(a, c) || c.lag(b) <= a.lag(b);
        constructed && incidental
    }

    /// Conjunction is pointwise intersection, commutatively, across every
    /// atomic operand pairing.
    ///
    /// `(x & y).contains(p)` equals `x.contains(p) && y.contains(p)` for every
    /// pair drawn from the atomic queries at two versions, in both operand
    /// orders. This is the behavioral pin on the whole merge kernel: floor
    /// joins, ceiling meets, hole absorption, vacuity pruning, and the
    /// strictness normalization all sit between `&` and `contains`, so any of
    /// them changing what a query admits diverges here. Probes include the
    /// operands' meet and join, reaching the at-bound corners on every call.
    fn conjunction_is_intersection {
        let (meet, join) = (b & c, b | c);
        let probes = [a, b, c, &meet, &join];
        /// One polarity-homogeneous double loop of the pointwise check.
        macro_rules! check {
            ($xs:expr, $ys:expr) => {
                for x in &$xs {
                    for y in &$ys {
                        let xy = x.clone() & y.clone();
                        let yx = y.clone() & x.clone();
                        for p in probes {
                            let want = x.contains(p) && y.contains(p);
                            if xy.contains(p) != want || yx.contains(p) != want {
                                return false;
                            }
                        }
                    }
                }
            };
        }
        check!(neutral_queries(b, c), neutral_queries(c, b));
        check!(down_queries(b, c), down_queries(c, b));
        check!(up_queries(b, c), up_queries(c, b));
        check!(neutral_queries(b, c), down_queries(c, b));
        check!(neutral_queries(b, c), up_queries(c, b));
        true
    }

    /// The typed `&` matrix lands in one predicate: wherever a conjunction
    /// lands in the type census (atom, bound, or query), it admits exactly the
    /// intersection of its operands.
    ///
    /// One equation per distinct merge path: the two elementary same-side
    /// collapses (which stay atoms, exercising the strictness-survival rule on
    /// comparable bounds and its dissolution on concurrent ones), the two side
    /// merges, and the cross-side pairings that land in a [`Query`] — the paths
    /// every macro-generated impl delegates to.
    fn conjunction_operand_forms_agree {
        use causally::{after, before, since, strictly_after, strictly_before};
        let (meet, join) = (b & c, b | c);
        let probes = [a, b, c, &meet, &join];
        for p in probes {
            let atoms = (after(b) & after(c)).contains(p)
                == (after(b).contains(p) && after(c).contains(p))
                && (before(b) & before(c)).contains(p)
                    == (before(b).contains(p) && before(c).contains(p))
                && (after(b) & before(c)).contains(p)
                    == (after(b).contains(p) && before(c).contains(p));
            let down = (since(b) & since(c)).contains(p)
                == (since(b).contains(p) && since(c).contains(p))
                && (after(b) & since(c)).contains(p)
                    == (after(b).contains(p) && since(c).contains(p))
                && (strictly_after(b) & strictly_after(c)).contains(p)
                    == (strictly_after(b).contains(p) && strictly_after(c).contains(p));
            let up = ((!after(b)) & (!after(c))).contains(p)
                == ((!after(b)).contains(p) && (!after(c)).contains(p))
                && (before(b) & (!after(c))).contains(p)
                    == (before(b).contains(p) && (!after(c)).contains(p))
                && (strictly_before(b) & strictly_before(c)).contains(p)
                    == (strictly_before(b).contains(p) && strictly_before(c).contains(p));
            if !(atoms && down && up) {
                return false;
            }
        }
        true
    }

    /// Coverage's verdicts are sound over the segment: `Full` admits the
    /// constructed in-segment probes, `Empty` rejects them.
    ///
    /// Quantified over conjunctions of an atomic query at one version with a
    /// representative bound at another, against the constructed ordered,
    /// coincident, and incidental spans; probes are the endpoints and `(lo | x)
    /// & hi` — a version within the segment by construction — so both sound
    /// arms are exercised against genuinely interior points. `Partial` promises
    /// nothing pointwise: the [`Coverage`] docs carry the precision contract,
    /// including why `Empty` cannot be complete.
    fn coverage_bounds_membership {
        for (lo, hi) in &span_candidates(b, c) {
            let Ok(span) = Span::new(lo, hi) else {
                // Every candidate is ordered by construction or admission.
                return false;
            };
            let mid = &(lo | a) & hi;
            let probes = [lo, hi, &mid];
            /// One family's soundness check over the span.
            macro_rules! check {
                ($qs:expr) => {
                    for q in &$qs {
                        match q.coverage(span.reborrow()) {
                            Coverage::Full => {
                                if probes.iter().any(|p| !q.contains(p)) {
                                    return false;
                                }
                            }
                            Coverage::Empty => {
                                if probes.iter().any(|p| q.contains(p)) {
                                    return false;
                                }
                            }
                            Coverage::Partial => {}
                        }
                    }
                };
            }
            check!(neutral_queries(a, b));
            check!(down_queries(a, b));
            check!(up_queries(a, b));
        }
        true
    }

    /// Coverage of a coincident span is membership: `Full` for a member,
    /// `Empty` otherwise — `Partial` is unreachable when the segment is one
    /// version — through both the span door and the version door.
    fn coverage_matches_membership_on_points {
        /// One family's point-degeneracy check.
        macro_rules! check {
            ($qs:expr, $probes:expr) => {
                for q in &$qs {
                    for p in $probes {
                        let want = if q.contains(p) {
                            Coverage::Full
                        } else {
                            Coverage::Empty
                        };
                        if q.coverage(p) != want || q.coverage(Span::at(p)) != want {
                            return false;
                        }
                    }
                }
            };
        }
        check!(neutral_queries(b, c), [a, b, c]);
        check!(down_queries(b, c), [a, b, c]);
        check!(up_queries(b, c), [a, b, c]);
        true
    }

    /// `Span::place` is exactly the two causal comparisons against the
    /// endpoints: the nine-state verdict is a pure transcription of `(probe vs
    /// lo, probe vs hi)`.
    ///
    /// Checked over the constructed ordered, coincident, and incidental span
    /// pairs, probing each operand and the pair's meet (which reaches the
    /// at-endpoint and `lo == hi` corners on every call).
    fn span_place_matches_relations {
        let meet = b & c;
        for (lo, hi) in &span_candidates(b, c) {
            let Ok(span) = Span::new(lo, hi) else {
                // Every candidate is ordered by construction or admission.
                return false;
            };
            for probe in [a, b, c, &meet] {
                if span.place(probe) != place_from_relations(lo, hi, probe) {
                    return false;
                }
            }
        }
        true
    }

    /// `dominance` is `place` coarsened to the dominance question, on every
    /// span.
    ///
    /// `Dominance::After` collects the verdicts with `hi <= p` (`At(End)`,
    /// `At(Both)`, `After`), `Dominance::Between` those with `lo <= p` but not
    /// `hi <= p` (`At(Start)`, `Between`, `Concurrent(End)`), and
    /// `Dominance::Before` the rest (`Before`, `Concurrent(Start)`,
    /// `Concurrent(Both)`).
    fn span_dominance_coarsens_place {
        let meet = b & c;
        for (lo, hi) in &span_candidates(b, c) {
            let Ok(span) = Span::new(lo, hi) else {
                return false;
            };
            for probe in [a, b, c, &meet] {
                let coarse = match span.place(probe) {
                    Placement::At(Endpoint::End | Endpoint::Both) | Placement::After => {
                        Dominance::After
                    }
                    Placement::At(Endpoint::Start)
                    | Placement::Between
                    | Placement::Concurrent(Endpoint::End) => Dominance::Between,
                    Placement::Before | Placement::Concurrent(Endpoint::Start | Endpoint::Both) => {
                        Dominance::Before
                    }
                };
                if span.dominance(probe) != coarse {
                    return false;
                }
            }
        }
        true
    }

    /// `precedence` is `place` coarsened to the precedence question — the
    /// dominance coarsening's dual, mirrored bucket by bucket — on every span.
    ///
    /// `Precedence::Before` collects the verdicts with `p <= lo` (`Before`,
    /// `At(Start)`, `At(Both)`), `Precedence::Between` those with `p <= hi` but
    /// not `p <= lo` (`At(End)`, `Between`, `Concurrent(Start)`), and
    /// `Precedence::After` the rest (`After`, `Concurrent(End)`,
    /// `Concurrent(Both)`).
    fn span_precedence_coarsens_place {
        let meet = b & c;
        for (lo, hi) in &span_candidates(b, c) {
            let Ok(span) = Span::new(lo, hi) else {
                return false;
            };
            for probe in [a, b, c, &meet] {
                let coarse = match span.place(probe) {
                    Placement::At(Endpoint::Start | Endpoint::Both) | Placement::Before => {
                        Precedence::Before
                    }
                    Placement::At(Endpoint::End)
                    | Placement::Between
                    | Placement::Concurrent(Endpoint::Start) => Precedence::Between,
                    Placement::After | Placement::Concurrent(Endpoint::End | Endpoint::Both) => {
                        Precedence::After
                    }
                };
                if span.precedence(probe) != coarse {
                    return false;
                }
            }
        }
        true
    }

    /// `contains` is segment membership: `lo <= p <= hi`, exactly the
    /// placements at an endpoint or between them — a `Concurrent` placement is
    /// beside the segment, never within it.
    fn span_contains_matches_place {
        let meet = b & c;
        for (lo, hi) in &span_candidates(b, c) {
            let Ok(span) = Span::new(lo, hi) else {
                return false;
            };
            for probe in [a, b, c, &meet] {
                let inside = matches!(span.place(probe), Placement::At(_) | Placement::Between);
                if span.contains(probe) != inside {
                    return false;
                }
            }
        }
        true
    }

    /// A span's segment, as a query, is exactly span placement's contained
    /// region: `Query::from(&span).contains(p)` iff [`Span::place`] puts `p` at
    /// an endpoint or between them.
    ///
    /// The tie between the two constructions: a span is the concrete pair, and
    /// its segment-as-predicate is the `after(lo) & before(hi)` cell of the
    /// query language — every other placement verdict (outside either endpoint,
    /// or concurrent to one) is exactly non-membership.
    fn segment_query_matches_span_place {
        let meet = b & c;
        for (lo, hi) in &span_candidates(b, c) {
            let Ok(span) = Span::new(lo, hi) else {
                // Every candidate is ordered by construction or admission.
                return false;
            };
            let q = Query::from(&span);
            for probe in [a, b, c, &meet] {
                let inside = matches!(span.place(probe), Placement::At(_) | Placement::Between);
                if q.contains(probe) != inside {
                    return false;
                }
            }
        }
        true
    }

    /// Span composite encodings are prefix-free: distinct spans' encodings
    /// are never byte prefixes of one another.
    ///
    /// Pinned directly on the composite (it rides the components'
    /// prefix-freedom, but the pin is on the composite itself, never
    /// inferred). Prefix-freedom is what lets one composite self-delimit
    /// inside a larger stream: the borsh leg reads exactly one span and
    /// leaves the next field's bytes unread. The quantified spans share
    /// endpoints across the two operand families, so byte-prefix-adjacent
    /// composites (equal meets under differing joins) arise on every call.
    fn span_encoding_is_prefix_free {
        let mut spans = operand_spans(a, b);
        spans.extend(operand_spans(b, c));
        for (i, x) in spans.iter().enumerate() {
            for y in &spans[i + 1..] {
                if x == y {
                    continue;
                }
                let (ex, ey) = (x.encode(), y.encode());
                if ex.starts_with(&ey) || ey.starts_with(&ex) {
                    return false;
                }
            }
        }
        true
    }

    /// `+` is the containment join: endpoints definitionally the meet of the
    /// meets and the join of the joins.
    ///
    /// The same span from either operand order, from every owned/borrowed
    /// cell, and from the `union` method spelling; idempotent, and covering
    /// both operands' whole segments.
    // The idempotence probes repeat an operand on purpose: `s + s == s` is
    // the law itself, not a typo the lint should flag.
    #[allow(clippy::eq_op)]
    fn span_union_is_the_containment_join {
        for s in &operand_spans(a, b) {
            for t in &operand_spans(b, c) {
                let union = s + t;
                let definitional =
                    union == Span::new(&(s.lo() & t.lo()), &(s.hi() | t.hi())).unwrap();
                let commutative = union == t + s;
                let idempotent = (s + s) == *s;
                let cells = (s.clone() + t.clone()) == union
                    && (s.clone() + t) == union
                    && (s + t.clone()) == union;
                let method = s.union(t) == union;
                let covering = [s.lo(), s.hi(), t.lo(), t.hi()]
                    .into_iter()
                    .all(|v| within(&union, v));
                if !(definitional && commutative && idempotent && cells && method && covering) {
                    return false;
                }
            }
        }
        true
    }

    /// `*` is the containment meet: the joined meets under the met joins
    /// when that pair orders, [`None`] exactly otherwise.
    ///
    /// Commutative, idempotent, absorbing with `+`, the `intersect` method
    /// spelling exactly, and containing every version both operands contain
    /// (so two overlapping operands always intersect).
    // The idempotence probe repeats an operand on purpose: `s * s` is the law.
    #[allow(clippy::eq_op)]
    fn span_intersect_is_the_shared_segment {
        for s in &operand_spans(a, b) {
            for t in &operand_spans(b, c) {
                let inter = s * t;
                let definitional = inter
                    == Span::new(&(s.lo() | t.lo()), &(s.hi() & t.hi()))
                        .ok()
                        .map(Span::into_owned);
                let commutative = inter == (t * s);
                let idempotent = (s * s) == Some(s.clone());
                let cells = (s.clone() * t.clone()) == inter
                    && (s.clone() * t) == inter
                    && (s * t.clone()) == inter;
                let method = s.intersect(t) == inter;
                let absorbing = {
                    let u = s + t;
                    (s * &u) == Some(s.clone())
                };
                let membership = [a, b, c].into_iter().all(|probe| {
                    !(within(s, probe) && within(t, probe))
                        || inter.as_ref().is_some_and(|i| within(i, probe))
                });
                if !(definitional
                    && commutative
                    && idempotent
                    && cells
                    && method
                    && absorbing
                    && membership)
                {
                    return false;
                }
            }
        }
        true
    }

    /// `|` is the pointwise join: endpoints definitionally the joins of the
    /// corresponding endpoints.
    ///
    /// Commutative, idempotent, with the coincident empty span as identity,
    /// the `join` method spelling exactly — and on coincident operands it
    /// restricts to the version join exactly (the lifting is a lattice
    /// homomorphism on points, where `+` yields the hull instead).
    // The idempotence probe repeats an operand on purpose: `s | s == s` is
    // the law itself.
    #[allow(clippy::eq_op)]
    fn span_join_is_the_pointwise_join {
        let empty = Version::new();
        let identity = empty.span(&empty);
        for s in &operand_spans(a, b) {
            for t in &operand_spans(b, c) {
                let join = s | t;
                let definitional =
                    join == Span::new(&(s.lo() | t.lo()), &(s.hi() | t.hi())).unwrap();
                let commutative = join == (t | s);
                let idempotent = (s | s) == *s;
                let cells = (s.clone() | t.clone()) == join
                    && (s.clone() | t) == join
                    && (s | t.clone()) == join;
                let method = s.join(t) == join;
                let identity_holds = (s | &identity) == *s;
                if !(definitional && commutative && idempotent && cells && method && identity_holds)
                {
                    return false;
                }
            }
        }
        // The point identity: two coincident spans join to the coincident
        // span at their versions' join.
        let bc = b | c;
        (b.span(b) | c.span(c)) == bc.span(&bc)
    }

    /// `&` is the pointwise meet: endpoints definitionally the meets of the
    /// corresponding endpoints.
    ///
    /// Commutative, idempotent, absorbing with `|` in the pointwise lattice,
    /// the `meet` method spelling exactly — and on coincident operands it
    /// restricts to the version meet exactly.
    // The idempotence probe repeats an operand on purpose: `s & s == s` is
    // the law itself.
    #[allow(clippy::eq_op)]
    fn span_meet_is_the_pointwise_meet {
        for s in &operand_spans(a, b) {
            for t in &operand_spans(b, c) {
                let meet = s & t;
                let definitional =
                    meet == Span::new(&(s.lo() & t.lo()), &(s.hi() & t.hi())).unwrap();
                let commutative = meet == (t & s);
                let idempotent = (s & s) == *s;
                let cells = (s.clone() & t.clone()) == meet
                    && (s.clone() & t) == meet
                    && (s & t.clone()) == meet;
                let method = s.meet(t) == meet;
                let absorbing = {
                    let u = s | t;
                    (&u & s) == *s
                };
                if !(definitional && commutative && idempotent && cells && method && absorbing) {
                    return false;
                }
            }
        }
        // The point identity: two coincident spans meet to the
        // coincident span at their versions' meet.
        let bc = b & c;
        (b.span(b) & c.span(c)) == bc.span(&bc)
    }

    /// `+` (the containment join) is associative: `(s + t) + u == s + (t + u)`.
    ///
    /// Componentwise the version lattice's meet on the lows and join on the
    /// his, each associative, so both association orders build the same span
    /// — the associativity half of the containment lattice the span docs
    /// claim.
    fn span_union_associative {
        for s in &operand_spans(a, b) {
            for t in &operand_spans(b, c) {
                for u in &operand_spans(a, c) {
                    if (&(s + t) + u) != (s + &(t + u)) {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// `*` (the containment meet) is associative over outcomes: both
    /// association orders agree in definedness and, where defined, in value.
    ///
    /// Both orders are defined exactly when the joint condition
    /// `lo_s | lo_t | lo_u <= hi_s & hi_t & hi_u` holds — the joint condition
    /// implies every pairwise one (the pairwise join is below the joint join,
    /// the pairwise meet above the joint meet), so neither order can fail an
    /// inner intersection where the other survives; the same shape as
    /// [`join_associative_outcomes`] on the partial monoid.
    fn span_intersect_associative_outcomes {
        for s in &operand_spans(a, b) {
            for t in &operand_spans(b, c) {
                for u in &operand_spans(a, c) {
                    let left = (s * t).and_then(|st| &st * u);
                    let right = (t * u).and_then(|tu| s * &tu);
                    if left != right {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// `|` (the pointwise join) is associative: `(s | t) | u == s | (t | u)`.
    ///
    /// Componentwise the version join on both endpoint pairs, so span `|`
    /// inherits its associativity — the associativity half of the pointwise
    /// lattice the span docs claim.
    fn span_join_associative {
        for s in &operand_spans(a, b) {
            for t in &operand_spans(b, c) {
                for u in &operand_spans(a, c) {
                    if (&(s | t) | u) != (s | &(t | u)) {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// `&` (the pointwise meet) is associative: `(s & t) & u == s & (t & u)`,
    /// the dual of [`span_join_associative`].
    fn span_meet_associative {
        for s in &operand_spans(a, b) {
            for t in &operand_spans(b, c) {
                for u in &operand_spans(a, c) {
                    if (&(s & t) & u) != (s & &(t & u)) {
                        return false;
                    }
                }
            }
        }
        true
    }
}

/// The hole-free queries over a version pair: the unbounded query, each atom
/// alone, and the interval — the [`causally::Neutral`] fragment's shapes.
fn neutral_queries<'a>(b: &'a Version, c: &'a Version) -> Vec<Query<'a>> {
    vec![
        causally::all(),
        causally::after(b).into(),
        causally::before(c).into(),
        causally::after(b) & causally::before(c),
    ]
}

/// Down-polar queries over a version pair: every down-hole spelling — negation,
/// widening, strict floor — alone and conjoined, so the merge kernel's
/// absorption and pruning arms are all reached.
fn down_queries<'a>(b: &'a Version, c: &'a Version) -> Vec<Query<'a, causally::Down>> {
    vec![
        causally::since(b),
        causally::since(c),
        causally::strictly_after(b),
        causally::after(b).or_concurrent(),
        causally::delta(b, c),
        causally::since(b) & causally::since(c),
        causally::after(b) & causally::since(c),
    ]
}

/// Up-polar queries over a version pair, dually to [`down_queries`].
fn up_queries<'a>(b: &'a Version, c: &'a Version) -> Vec<Query<'a, causally::Up>> {
    vec![
        !causally::after(b),
        !causally::after(c),
        causally::strictly_before(b),
        causally::before(b).or_concurrent(),
        (!causally::after(b)) & (!causally::after(c)),
        causally::before(b) & (!causally::after(c)),
        causally::after(b) & (!causally::after(c)),
    ]
}

/// The span pairs the placement laws quantify over, from a version pair.
///
/// The constructed always-ordered pair (`meet <= join`), the coincident pair
/// (reaching `lo == hi` on every call), and the raw pair whenever it happens to
/// order.
fn span_candidates(b: &Version, c: &Version) -> Vec<(Version, Version)> {
    let (meet, join) = (b & c, b | c);
    let mut out = vec![(meet.clone(), join), (meet.clone(), meet)];
    if le(b, c) {
        out.push((b.clone(), c.clone()));
    }
    out
}

/// [`Placement`], transcribed from the two raw causal comparisons against the
/// endpoints — the nine-state table stated relation by relation, with the start
/// relation examined first.
fn place_from_relations(lo: &Version, hi: &Version, p: &Version) -> Placement {
    match p.partial_cmp(lo) {
        Some(Ordering::Less) => Placement::Before,
        Some(Ordering::Equal) => match p.partial_cmp(hi) {
            Some(Ordering::Equal) => Placement::At(Endpoint::Both),
            _ => Placement::At(Endpoint::Start),
        },
        Some(Ordering::Greater) => match p.partial_cmp(hi) {
            Some(Ordering::Less) => Placement::Between,
            Some(Ordering::Equal) => Placement::At(Endpoint::End),
            Some(Ordering::Greater) => Placement::After,
            None => Placement::Concurrent(Endpoint::End),
        },
        None => match p.partial_cmp(hi) {
            None => Placement::Concurrent(Endpoint::Both),
            _ => Placement::Concurrent(Endpoint::Start),
        },
    }
}

// ─────────────────────────── the span algebra ───────────────────────────

/// The valid spans the operator laws quantify over, from a version pair: the
/// pair's hull, the coincident span at the meet, and the raw pair whenever it
/// happens to order — settled owned so the operand cells can consume them.
fn operand_spans(b: &Version, c: &Version) -> Vec<Span<'static>> {
    span_candidates(b, c)
        .into_iter()
        .map(|(lo, hi)| {
            Span::new(&lo, &hi)
                .expect("every candidate is ordered")
                .into_owned()
        })
        .collect()
}

/// Whether `probe` lies on the span's chain segment (`lo <= p <= hi`): the
/// order-sense membership the containment operators speak about. A `Concurrent`
/// placement is *beside* the span, never within it.
fn within(span: &Span<'_>, probe: &Version) -> bool {
    matches!(span.place(probe), Placement::At(_) | Placement::Between)
}

// ───────────────────────────── Version: lists ─────────────────────────────

laws! {
    /// Laws over a list of versions, at any arity.
    ///
    /// The seedless iterator join doors (`Sum` and `FromIterator`, owned and
    /// borrowed) against their sequential pair-operator oracle, and their
    /// order-independence. The list length is the quantified variable no
    /// fixed-arity group can reach: the drivers sweep it across every
    /// structural boundary of the balanced counter the folds run on — the
    /// identity and lone-input short-circuits, the first leaf combine, the
    /// closing drain, and the merged–merged carries that first fire at arity
    /// four — so no combine arm sits beyond the suite's reach under any future
    /// reshaping of the fold.
    pub static VERSION_LIST: (xs: &[Version]);

    /// `sum`/`collect` are the sequential pair fold: at every arity, every
    /// seedless iterator door equals `|` folded left-to-right from the
    /// identity ([`Version::new`]).
    ///
    /// The right-hand side is the bound pair operator, never a fold door, so
    /// the two sides cannot share a broken combine arm; the balanced
    /// regrouping inside the doors is exactly what the equation quantifies
    /// away. At arity zero the equation *is* the empty edge (the empty sum is
    /// the empty version), at one the lone input, at two the pair operator
    /// itself.
    fn version_sum_is_the_sequential_pair_fold {
        let sequential = xs.iter().fold(Version::new(), |acc, x| &acc | x);
        xs.iter().sum::<Version>() == sequential
            && xs.iter().cloned().sum::<Version>() == sequential
            && xs.iter().collect::<Version>() == sequential
            && xs.iter().cloned().collect::<Version>() == sequential
    }

    /// The seedless join doors are order-independent at every arity: every
    /// rotation and the reversal of the list fold to the same join.
    ///
    /// Each rotation hands the balanced counter a different grouping of the
    /// same population (which elements coalesce at which weights depends only
    /// on arrival order), so an arm that favors one grouping diverges from the
    /// rest of the orbit. Together with the sequential-fold law and the pair
    /// operator's commutativity and associativity, the full permutation orbit
    /// is pinned.
    fn version_sum_is_order_invariant {
        let join: Version = xs.iter().sum();
        xs.iter().rev().sum::<Version>() == join
            && (1..xs.len()).all(|r| xs[r..].iter().chain(&xs[..r]).sum::<Version>() == join)
    }
}

// ──────────────────── Version: a receiver and items ────────────────────

laws! {
    /// Laws over a version (the receiver) and a list of versions (the items),
    /// at any arity.
    ///
    /// The shape the receiver-seeded folds ([`Version::join_all`],
    /// [`Version::meet_all`], [`Version::span_all`]) quantify over: the
    /// receiver is the guaranteed first element that keeps each fold total, so
    /// the family under law is `{receiver} ∪ items` — never empty. The item
    /// count is swept across the same fold boundaries as the [`VERSION_LIST`]
    /// laws', the reach no fixed-arity group has.
    pub static VERSION_AND_LIST: (receiver: &Version, items: &[Version]);

    /// `join_all` is the sequential pair fold: at every arity, the n-ary door
    /// equals `|` folded left-to-right from the receiver.
    ///
    /// The right-hand side is the bound pair operator, never the n-ary door,
    /// so the two sides cannot share a broken combine arm; the balanced
    /// regrouping inside the door is exactly what the equation quantifies
    /// away. At zero items the equation *is* the lone-input edge (the join of
    /// the receiver alone is the receiver), at one item the pair operator
    /// itself.
    fn join_all_is_the_sequential_pair_fold {
        receiver.join_all(items) == items.iter().fold(receiver.clone(), |acc, x| &acc | x)
    }

    /// `meet_all` is the sequential pair fold: at every arity, the n-ary door
    /// equals `&` folded left-to-right from the receiver — total at every
    /// arity, because the receiver seeds the identityless meet.
    fn meet_all_is_the_sequential_pair_fold {
        receiver.meet_all(items) == items.iter().fold(receiver.clone(), |acc, x| &acc & x)
    }

    /// The n-ary lattice folds are rotation-independent at every arity: which
    /// element rides as the receiver is irrelevant, and so is item order.
    ///
    /// Every rotation of the family — each element taking one turn as the
    /// receiver, the rest following in rotated order — and the reversal fold
    /// to the same join and the same meet. Each rotation hands the balanced
    /// counter a different grouping of the same population (which elements
    /// coalesce at which weights depends only on arrival order), so an arm
    /// that favors one grouping diverges from the orbit. Together with the
    /// sequential-fold laws and the pair operators' commutativity and
    /// associativity, the full permutation orbit is pinned.
    fn fold_all_is_rotation_invariant {
        let family: Vec<&Version> = core::iter::once(receiver).chain(items).collect();
        let join = receiver.join_all(items);
        let meet = receiver.meet_all(items);
        let rotations = (1..family.len()).all(|r| {
            let rotated = || family[r + 1..].iter().chain(&family[..r]).copied();
            family[r].join_all(rotated()) == join && family[r].meet_all(rotated()) == meet
        });
        let reversed = {
            let (last, front) = family.split_last().expect("the receiver is always present");
            last.join_all(front.iter().rev().copied()) == join
                && last.meet_all(front.iter().rev().copied()) == meet
        };
        rotations && reversed
    }

    /// The n-ary span at every arity: endpoints definitionally the n-ary meet
    /// and join over `{receiver} ∪ items`, every input within.
    ///
    /// The endpoints are [`Version::meet_all`] and [`Version::join_all`] over
    /// the same family — the accessors read exactly them back — and every input
    /// places within the hull: never [`Before`](Placement::Before) or
    /// [`After`](Placement::After), since the meet bounds each input from below
    /// and the join from above. At zero items the family is the receiver alone
    /// and the hull is the coincident `[receiver, receiver]`; at one item it is
    /// the pair hull ([`span_is_the_pair_hull`] pins those same edges from the
    /// binary door's side). The hull fold carries both lattice directions
    /// through one balanced counter, so a combine arm that reads the wrong
    /// endpoint of a merged group breaks exactly one side of this equation at
    /// exactly the arities that reach the arm.
    fn span_all_is_the_family_hull {
        let hull = receiver.span_all(items);
        let family = || core::iter::once(receiver).chain(items);
        let meet = receiver.meet_all(items);
        let join = receiver.join_all(items);
        let definitional = hull == Span::new(&meet, &join).unwrap();
        let accessors = *hull.lo() == meet && *hull.hi() == join;
        let contained =
            family().all(|v| !matches!(hull.place(v), Placement::Before | Placement::After));
        definitional && accessors && contained
    }

    /// The n-ary span is rotation-independent at every arity: which element
    /// rides as the receiver is irrelevant, and so is item order.
    ///
    /// Every rotation of the family — each element taking one turn as the
    /// receiver, the rest following in rotated order — and the reversal build
    /// the same hull. Each rotation regroups the hull fold's balanced counter
    /// differently, so an arm wrong under one grouping diverges from the orbit.
    fn span_all_is_rotation_invariant {
        let family: Vec<&Version> = core::iter::once(receiver).chain(items).collect();
        let hull = receiver.span_all(items);
        let rotations = (1..family.len()).all(|r| {
            let items = family[r + 1..].iter().chain(&family[..r]).copied();
            family[r].span_all(items) == hull
        });
        let reversed = {
            let (last, front) = family.split_last().expect("the receiver is always present");
            last.span_all(front.iter().rev().copied()) == hull
        };
        rotations && reversed
    }

    /// The n-ary span doors are their binary operators folded left-to-right
    /// over `{seed} ∪ items`, at every arity.
    ///
    /// The balanced regrouping inside each door is exactly what the equation
    /// quantifies away, and the right-hand sides are the bound binary
    /// operators, never the doors.
    ///
    /// The containment doors run from the receiver's coincident span (union)
    /// and from the family hull (intersection — a wide seed keeps the nonempty
    /// path exercised deep into the fold, while disjoint item spans still reach
    /// [`None`]); the pointwise doors run from the coincident seed.
    fn span_folds_match_the_sequential_operators {
        let seed = receiver.span(receiver);
        let hull = receiver.span_all(items);
        let spans = item_spans(items);
        let union = seed.union_all(&spans) == spans.iter().fold(seed.clone(), |acc, s| &acc + s);
        // The sequential reference folds *through* `Option` with no early exit,
        // deliberately: the door defers its verdict to the end, and the
        // equation quantifies over the same completed fold (`try_fold` would
        // exit at the first `None` — a different reference).
        #[allow(clippy::manual_try_fold)]
        let intersect = hull.intersect_all(&spans)
            == spans
                .iter()
                .fold(Some(hull.clone()), |acc, s| acc.and_then(|a| &a * s));
        let join = seed.join_all(&spans) == spans.iter().fold(seed.clone(), |acc, s| &acc | s);
        let meet = seed.meet_all(&spans) == spans.iter().fold(seed.clone(), |acc, s| &acc & s);
        union && intersect && join && meet
    }

    /// The n-ary span doors are item-order-independent at every arity: every
    /// rotation and the reversal of the item list fold to the same span (or the
    /// same [`None`]).
    ///
    /// Each rotation regroups every door's balanced counter differently, so a
    /// combine arm wrong under one grouping diverges from the orbit — the
    /// span-door instance of `fold_all_is_rotation_invariant`.
    fn span_folds_are_rotation_invariant {
        let seed = receiver.span(receiver);
        let hull = receiver.span_all(items);
        let spans = item_spans(items);
        let union = seed.union_all(&spans);
        let intersect = hull.intersect_all(&spans);
        let join = seed.join_all(&spans);
        let meet = seed.meet_all(&spans);
        let agrees = |ordered: &mut dyn Iterator<Item = &Span<'static>>| {
            let ordered: Vec<&Span<'static>> = ordered.collect();
            seed.union_all(ordered.iter().copied()) == union
                && hull.intersect_all(ordered.iter().copied()) == intersect
                && seed.join_all(ordered.iter().copied()) == join
                && seed.meet_all(ordered.iter().copied()) == meet
        };
        agrees(&mut spans.iter().rev())
            && (1..spans.len()).all(|r| agrees(&mut spans[r..].iter().chain(&spans[..r])))
    }

    /// A union of coincident spans is the version hull: on points the
    /// containment door restricts to [`Version::span_all`] exactly, so the two
    /// doors can never drift apart on the shapes both serve.
    fn span_union_of_points_is_span_all {
        let points: Vec<Span<'static>> = items.iter().map(|v| v.span(v)).collect();
        receiver.span(receiver).union_all(&points) == receiver.span_all(items)
    }

    /// Summing or collecting an iterator of spans is the union fold.
    ///
    /// Both collection doors equal the receiver-seeded n-ary union over the
    /// same inputs, owned and borrowed alike, and the empty iterator yields
    /// [`None`] (union has no identity span).
    fn span_sum_and_collect_are_the_union_fold {
        let seed = receiver.span(receiver);
        let spans = item_spans(items);
        let expected = Some(seed.union_all(&spans));
        let inputs = || core::iter::once(&seed).chain(&spans);
        let sum: Option<Span> = inputs().sum();
        let collected: Option<Span> = inputs().collect();
        let owned: Option<Span> = inputs().map(Span::clone).sum();
        let empty: Option<Span> = core::iter::empty::<&Span>().sum();
        sum == expected && collected == expected && owned == expected && empty.is_none()
    }

    /// Multiplying out an iterator of spans is the intersection fold.
    ///
    /// The `Product` door equals the receiver-seeded n-ary intersection over
    /// the same inputs, owned and borrowed alike, and the empty iterator
    /// yields [`None`] (intersection has no identity span).
    fn span_product_is_the_intersect_fold {
        let hull = receiver.span_all(items);
        let spans = item_spans(items);
        let expected = hull.intersect_all(&spans);
        let inputs = || core::iter::once(&hull).chain(&spans);
        let product: Option<Span> = inputs().product();
        let owned: Option<Span> = inputs().map(Span::clone).product();
        let empty: Option<Span> = core::iter::empty::<&Span>().product();
        product == expected && owned == expected && empty.is_none()
    }
}

/// The item spans the fold-door laws quantify over: a deterministic mix of
/// coincident and wide spans over the items.
///
/// The mix drives the doors' point and wide combine arms alike, at every
/// counter boundary the list sweep reaches.
fn item_spans(items: &[Version]) -> Vec<Span<'static>> {
    items
        .iter()
        .enumerate()
        .map(|(i, v)| {
            if i % 2 == 0 {
                v.span(v)
            } else {
                items[i - 1].span(v)
            }
        })
        .collect()
}

// ───────────────────────────── Party: one value ─────────────────────────────

laws! {
    /// Laws over one live party.
    ///
    /// The fork/join round-trip and its disjointness geometry, the balanced
    /// n-way fork's two forms, the covering order's point laws (with a
    /// constructed transitivity chain), `without` at the reflexive corner,
    /// aliasing, and the representational round-trips. `join_all`'s fold laws
    /// are [`PARTY_AND_LIST`]'s: the width-quantified family (reunion,
    /// acceptance, best-effort at every arity) subsumes any fixed-width point
    /// instance.
    pub static PARTY_SOLO: (p: &Party);

    /// `fork` then `join` round-trips: the two halves reconstruct the original
    /// region exactly.
    fn fork_join_roundtrip {
        let mut kept = p.dangerously_alias();
        let given = kept.fork();
        kept.join(given).is_ok() && kept == *p
    }

    /// The two halves a `fork` produces are disjoint, the relation is
    /// symmetric, and neither half is anonymous (both re-encode and decode as
    /// nonzero shares) — the invariant that keeps a forked population pairwise
    /// `join`-able.
    fn fork_halves_disjoint {
        let mut kept = p.dangerously_alias();
        let given = kept.fork();
        kept.is_disjoint(&given)
            && given.is_disjoint(&kept)
            && Party::decode(&kept.encode()[..]).is_ok()
            && Party::decode(&given.encode()[..]).is_ok()
    }

    /// A fork's parent covers both halves, and the halves cover neither other
    /// (they are disjoint proper subregions).
    fn fork_halves_covered_by_parent {
        let mut kept = p.dangerously_alias();
        let given = kept.fork();
        p.covers(&kept) && p.covers(&given) && !kept.covers(&given) && !given.covers(&kept)
    }

    /// The two balanced-fork forms agree: `From<Party>` for `[Party; N]` equals
    /// the residual the borrowing `forks(N - 1)` keeps, followed by the shares
    /// it yields (`[residual] ++ forks`).
    fn forks_matches_from_array {
        const N: usize = 4;
        let array: [Party; N] = p.dangerously_alias().into();
        let mut keeper = p.dangerously_alias();
        let yielded: Vec<Party> = keeper.forks(N as u64 - 1).collect();
        let reconstructed: Vec<Party> = std::iter::once(keeper).chain(yielded).collect();
        array.iter().eq(reconstructed.iter())
    }

    /// Dropping `forks` early folds the untaken shares back: after pulling 2 of
    /// 5, rejoining the 2 taken shares recovers the original region — the
    /// drop-time reabsorption the iterator promises.
    fn forks_partial_drop_folds_back {
        let mut keeper = p.dangerously_alias();
        let taken: Vec<Party> = keeper.forks(5).take(2).collect(); // iterator dropped after 2
        keeper.join_all(taken).is_ok() && keeper == *p
    }

    /// Joining an overlapping party errors and hands it back unchanged: a
    /// proper subregion refuses to absorb the region containing it.
    fn join_overlap_hands_back {
        let mut sub = p.dangerously_alias();
        let _ = sub.fork(); // sub is now a proper subregion of p
        match sub.join(p.dangerously_alias()) {
            Err(handed_back) => handed_back == *p,
            Ok(()) => false,
        }
    }

    /// Covering is reflexive: a party covers its own region.
    fn covers_reflexive {
        p.covers(&p.dangerously_alias())
    }

    /// Covering chains down a constructed fork tower: the whole covers its
    /// half, the half its quarter, and — transitively — the whole covers the
    /// quarter.
    fn covers_transitive_constructed {
        let mut quarter = p.dangerously_alias();
        let _ = quarter.fork(); // quarter: a half of p
        let half = quarter.dangerously_alias();
        let _ = quarter.fork(); // quarter: a quarter of p
        p.covers(&half) && half.covers(&quarter) && p.covers(&quarter)
    }

    /// The whole-interval seed covers every live party — and, owning
    /// everything, is disjoint from none.
    fn seed_covers_every_party {
        Party::seed().covers(p) && !Party::seed().is_disjoint(p) && !p.is_disjoint(&Party::seed())
    }

    /// Disjointness is irreflexive on live parties: a nonzero region overlaps
    /// itself.
    fn never_disjoint_from_self {
        !p.is_disjoint(&p.dangerously_alias())
    }

    /// A party covers itself, so removing itself leaves nothing:
    /// `p \ p == None`.
    fn without_self_is_none {
        p.dangerously_alias().without(p).is_none()
    }

    /// `without` is the partial inverse of `join` on the fork lattice: carving
    /// a forked-off share back out of the parent recovers the kept half, and
    /// removing a disjoint share is a no-op.
    fn without_inverts_fork {
        let mut keep = p.dangerously_alias();
        let give = keep.fork();
        let carved = p.dangerously_alias().without(&give);
        let noop = keep.dangerously_alias().without(&give);
        carved.is_some_and(|c| c == keep) && noop.is_some_and(|n| n == keep)
    }

    /// `dangerously_alias` yields a byte-identical, `Eq` copy aliasing the
    /// entire region: the two are *not* disjoint — the deliberate linearity
    /// violation the method documents.
    fn alias_is_byte_identical_overlap {
        let dup = p.dangerously_alias();
        dup == *p && dup.as_bytes() == p.as_bytes() && !p.is_disjoint(&dup)
    }

    /// `is_seed` recognizes exactly the whole-interval party: `p.is_seed() ⟺
    /// p == seed`.
    fn is_seed_iff_equals_seed {
        p.is_seed() == (*p == Party::seed())
    }

    /// `decode ∘ encode == id`, and the round-tripped party re-encodes to the
    /// same bytes.
    fn party_codec_roundtrip {
        let bytes = p.encode();
        Party::decode(&bytes[..]).is_ok_and(|decoded| decoded == *p && decoded.encode() == bytes)
    }

    /// `FromStr ∘ Display == id`: the paper notation round-trips.
    fn party_text_roundtrip {
        p.to_string()
            .parse::<Party>()
            .is_ok_and(|parsed| parsed == *p)
    }

    /// The borrowed byte view is the encoding: `as_bytes == encode`.
    fn party_as_bytes_matches_encode {
        p.as_bytes() == &p.encode()[..]
    }

    /// `encoded_bits` is the pre-pad bit length of `encode`.
    fn party_encoded_bits_matches_encode_len {
        p.encode().len() == (p.encoded_bits() + 1).div_ceil(8)
    }
}

// ───────────────────────────── Party: pairs ─────────────────────────────

laws! {
    /// Laws over a pair of live parties.
    ///
    /// The covering order's antisymmetry and its exclusion by disjointness,
    /// disjointness symmetry, `join`'s outcome-quantified commutativity and its
    /// coherence with `is_disjoint`, `without`'s two characterizations, and
    /// `Eq`/`Hash` coherence.
    pub static PARTY_PAIR: (a: &Party, b: &Party);

    /// Covering is antisymmetric: two regions cover each other exactly when
    /// they are equal.
    fn covers_antisymmetric {
        (a.covers(b) && b.covers(a)) == (a == b)
    }

    /// Disjointness is symmetric.
    fn disjoint_symmetric {
        a.is_disjoint(b) == b.is_disjoint(a)
    }

    /// Disjoint live regions cover neither other (covering needs overlap, and
    /// a live party is nonempty).
    fn disjoint_excludes_covering {
        !a.is_disjoint(b) || (!a.covers(b) && !b.covers(a))
    }

    /// `join` accepts exactly the disjoint pairs: `a.join(b) is Ok ⟺
    /// a.is_disjoint(b)`.
    fn join_defined_iff_disjoint {
        a.dangerously_alias().join(b.dangerously_alias()).is_ok() == a.is_disjoint(b)
    }

    /// `join` is commutative over outcomes: both orders agree in arm, produce
    /// equal unions on `Ok`, and hand back the argument unchanged (leaving
    /// `self` unchanged) on `Err`.
    fn join_commutative_outcomes {
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
    fn join_covers_both_and_without_undoes {
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
    fn without_characterization {
        match a.dangerously_alias().without(b) {
            None => b.covers(a),
            Some(remainder) => !b.covers(a) && a.covers(&remainder) && remainder.is_disjoint(b),
        }
    }

    /// Removing a disjoint share is a no-op: `a \ b == a` when the regions
    /// share nothing.
    fn without_disjoint_is_noop {
        !a.is_disjoint(b) || a.dangerously_alias().without(b).is_some_and(|r| r == *a)
    }

    /// `Eq` is canonical-byte equality: `a == b ⟺ encode(a) == encode(b)`.
    fn party_eq_iff_bytes_eq {
        (a == b) == (a.encode() == b.encode())
    }

    /// `Eq`/`Hash` coherence: equal parties hash equally.
    fn party_eq_implies_hash_eq {
        a != b || hash_of(a) == hash_of(b)
    }
}

// ───────────────────────────── Party: triples ─────────────────────────────

laws! {
    /// Laws over a triple of live parties.
    ///
    /// The covering order's incidental transitivity and the partial monoid's
    /// associativity, outcome-quantified.
    pub static PARTY_TRIPLE: (a: &Party, b: &Party, c: &Party);

    /// Covering is transitive: whenever three arbitrary parties happen to chain
    /// (`a ⊇ b ⊇ c`), the endpoints must too.
    fn covers_transitive_incidental {
        !(a.covers(b) && b.covers(c)) || a.covers(c)
    }

    /// The partial monoid is associative over outcomes: `(a + b) + c` and
    /// `a + (b + c)` agree in definedness and, where defined, in value (both
    /// are defined exactly on pairwise-disjoint triples).
    fn join_associative_outcomes {
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

// ───────────────────── Party: a receiver and items ─────────────────────

laws! {
    /// Laws over a live party (the receiver) and a list of live parties
    /// (the items), at any arity.
    ///
    /// [`Party::join_all`]'s faces, each quantified over the item count
    /// no fixed-width point law can sweep: acceptance is pairwise
    /// disjointness of the whole family, an accepted fold is the
    /// sequential pair joins, and rejection is per-input, lossless
    /// (best-effort), and region-conserving. The constructed laws draw
    /// their families from the receiver's own fork tree, so the accepted
    /// arm is exercised at every width even though arbitrary parties
    /// rarely happen to be disjoint — while the arbitrary items
    /// (frequently aliased, since the drivers index small pools) keep
    /// the refusal arm under mass, with two and more distinct
    /// overlapping groups arising from repeated pool picks.
    pub static PARTY_AND_LIST: (p: &Party, items: &[Party]);

    /// `join_all` accepts exactly the pairwise-disjoint families —
    /// receiver included — at every arity.
    ///
    /// An accepted fold equals the sequential pair joins (the bound pair
    /// operation, never the n-ary door, so the two sides cannot share a
    /// broken arm); a refused one still absorbed every region it could —
    /// the accumulator covers its original region — and handed at least
    /// one input back.
    fn party_join_all_accepts_iff_family_pairwise_disjoint {
        let family: Vec<&Party> = core::iter::once(p).chain(items).collect();
        let pairwise_disjoint = family
            .iter()
            .enumerate()
            .all(|(i, a)| family[i + 1..].iter().all(|b| a.is_disjoint(b)));
        let mut acc = p.dangerously_alias();
        match acc.join_all(items.iter().map(Party::dangerously_alias)) {
            Ok(()) => {
                let mut seq = p.dangerously_alias();
                let sequential = items
                    .iter()
                    .all(|item| seq.join(item.dangerously_alias()).is_ok());
                pairwise_disjoint && sequential && acc == seq
            }
            Err(returned) => !pairwise_disjoint && !returned.is_empty() && acc.covers(p),
        }
    }

    /// `join_all` reunites a balanced fork at every width: the shares of
    /// `forks(k)` fold back to the original region exactly.
    ///
    /// Along the way, half the shares folded through the n-ary door must
    /// equal the same half folded through sequential pair joins — a value
    /// comparison on a genuinely proper subregion, so a fold that
    /// misplaces a group cannot hide behind the full reunion's fixed
    /// endpoint. Width zero is the empty-fold identity (`join_all(∅)`
    /// leaves the receiver unchanged).
    fn party_join_all_reunites_forks_at_any_width {
        let width = items.len();
        let mut keeper = p.dangerously_alias();
        let shares: Vec<Party> = keeper.forks(width as u64).collect();
        let half = &shares[..width / 2];
        let mut seq = keeper.dangerously_alias();
        if !half
            .iter()
            .all(|share| seq.join(share.dangerously_alias()).is_ok())
        {
            return false;
        }
        let mut balanced = keeper.dangerously_alias();
        if balanced
            .join_all(half.iter().map(Party::dangerously_alias))
            .is_err()
            || balanced != seq
        {
            return false;
        }
        keeper.join_all(shares).is_ok() && keeper == *p
    }

    /// `join_all`'s rejection is per-input and lossless at every width:
    /// one aliased input planted among `k` genuine shares costs exactly
    /// itself.
    ///
    /// The clash — an alias of the accumulator's own region — rides
    /// mid-stream, so shares both before and after it must be absorbed
    /// around the rejection (fail-fast would abandon the tail): the fold
    /// reunites the region exactly, and the alias alone comes back,
    /// unchanged.
    fn party_join_all_is_best_effort_at_any_width {
        let width = items.len();
        let mut keeper = p.dangerously_alias();
        let mut fed: Vec<Party> = keeper.forks(width as u64).collect();
        let residual = keeper.dangerously_alias();
        fed.insert(width / 2, keeper.dangerously_alias());
        match keeper.join_all(fed) {
            Err(returned) => returned.len() == 1 && returned[0] == residual && keeper == *p,
            Ok(()) => false,
        }
    }

    /// `join_all`'s rejection conserves regions at every arity: the
    /// accumulator joined with the returned regions covers the
    /// receiver's original region unioned with every input's.
    ///
    /// Stated over region unions (`covers` against a `join`-built
    /// union), never byte identity: the closing drain legitimately
    /// hands back *coalesced* groups, byte-distinct from every input,
    /// so an element-wise identity clause would reject correct
    /// behavior. What the union statement convicts is a fold that
    /// *drops* a group instead of handing it back — a loss invisible to
    /// the acceptance law's `Err` clauses (hand-back nonempty,
    /// accumulator covers its origin) whenever another input already
    /// sits in the rejection channel. The accepted arm's conservation
    /// is the acceptance law's: an accepted fold equals the sequential
    /// pair joins, which drop nothing.
    fn party_join_all_err_conserves_the_region_union {
        let mut acc = p.dangerously_alias();
        match acc.join_all(items.iter().map(Party::dangerously_alias)) {
            Ok(()) => true,
            Err(returned) => {
                // The union of the accumulator and every returned
                // region: each hand-back may overlap the accumulator
                // and its fellows (aliases are why it came back), so
                // the union grows by each one's uncovered remainder.
                let mut union = acc;
                for back in returned {
                    if let Some(missing) = back.without(&union) {
                        if union.join(missing).is_err() {
                            return false; // the remainder is disjoint by construction
                        }
                    }
                }
                union.covers(p) && items.iter().all(|item| union.covers(item))
            }
        }
    }
}

// ───────────────────────────── Version × Party ─────────────────────────────

laws! {
    /// Laws over a version and a live party.
    ///
    /// The event laws (`tick` strictly advances, and only within the party's
    /// region — §4's `e' = e + f·i`), the entry points' agreement (`tick` and
    /// `ticks`, each across its two spellings), the fused multi-tick's point
    /// laws (`ticks(0)` the identity, `ticks(1)` the tick, small counts
    /// against the iterated ground truth, a fresh line realizing the tick
    /// floor at any width), and the projection (`/`) point laws.
    pub static VERSION_PARTY: (a: &Version, p: &Party);

    /// `tick` strictly advances the causal order: `a < a.tick(p)`.
    fn tick_strictly_advances {
        let mut ticked = a.clone();
        ticked.tick(p);
        le(a, &ticked) && !le(&ticked, a) && *a != ticked
    }

    /// `tick` inflates only within the party's region (§4: `e' = e + f·i`, zero
    /// outside `i`): projected onto the region's complement, the ticked version
    /// is unchanged. Vacuous only for the seed party, which has no complement.
    fn tick_only_inflates_the_region {
        let mut ticked = a.clone();
        ticked.tick(p);
        match Party::seed().without(p) {
            None => true, // p owns the whole interval: nothing lies outside it
            Some(rest) => (&ticked / &rest) == (a / &rest),
        }
    }

    /// `tick`'s inflation is real *within* the region (§4: `f · i ⊐ 0`): the
    /// projection onto the ticking party strictly advances.
    fn tick_advances_within_the_region {
        let mut ticked = a.clone();
        ticked.tick(p);
        (a / p).partial_cmp(&(&ticked / p)) == Some(Ordering::Less)
    }

    /// The two `tick` entry points agree: `version.tick(&party)` and
    /// `party.tick(&mut version)` produce the same advance.
    fn party_tick_matches_version_tick {
        let mut via_version = a.clone();
        via_version.tick(p);
        let mut via_party = a.clone();
        p.tick(&mut via_party);
        via_version == via_party
    }

    /// `ticks(0)` is the identity: the empty run records nothing.
    fn ticks_zero_is_identity {
        let mut run = a.clone();
        run.ticks(p, 0u64);
        run == *a
    }

    /// `ticks(1)` is exactly `tick`: the fused multi-tick degenerates to the
    /// single event.
    fn ticks_one_is_tick {
        let mut fused = a.clone();
        fused.ticks(p, 1u64);
        let mut ticked = a.clone();
        ticked.tick(p);
        fused == ticked
    }

    /// `ticks(n)` equals `n` sequential `tick`s, checked at every count a
    /// short iterated run reaches (0..=3) — the ground-truth seam the wide
    /// counts compose over ([`ticks_composes`] in the pair-party group).
    fn ticks_agrees_with_iterated_ticks {
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
    fn party_ticks_matches_version_ticks {
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
    fn ticks_line_realizes_min_ticks {
        let n = a.min_ticks();
        let mut line = Version::new();
        line.ticks(p, n.clone());
        line.min_ticks() == n
    }

    /// Projection keeps at most the history it is given: `a / p <= a`.
    fn projection_is_sub_version {
        le_by(&(a / p), a)
    }

    /// Projection is idempotent: `(a / p) / p == a / p`.
    ///
    /// The inner projection is materialized — idempotence quantifies over the
    /// projected *object* — and the outer one stays a view: the equality is
    /// the fused view-vs-version comparison.
    fn projection_idempotent {
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
    fn projection_additive_over_fork {
        let mut keeper = p.dangerously_alias();
        let child = keeper.fork();
        ((a / &keeper).to_version() | (a / &child).to_version()) == (a / p)
    }

    /// The named spelling is the operator's, exactly: `a.project(p)` is
    /// `a / p` — the same view, the same materialization.
    fn project_is_the_operator_spelling {
        a.project(p) == (a / p) && a.project(p).to_version() == (a / p).to_version()
    }
}

// ───────────────────────────── Version × Version × Party ─────────────────────────────

laws! {
    /// Laws over two versions and a live party: projection as a lattice
    /// homomorphism, its monotonicity in the version, its metric short-map
    /// property, and `ticks` as a monoid action at the wide counts the
    /// operands' tick floors supply.
    pub static VERSION_PAIR_PARTY: (a: &Version, b: &Version, p: &Party);

    /// Projection is a homomorphism of the join: `(a | b) / p == (a/p) | (b/p)`
    /// (the pointwise gate commutes with pointwise max).
    ///
    /// The right-hand side's join needs its operands as objects, so the
    /// per-operand projections materialize; the left-hand side stays a view.
    fn projection_join_homomorphism {
        let joined = a | b;
        (&joined / p) == ((a / p).to_version() | (b / p).to_version())
    }

    /// Projection is a homomorphism of the meet: `(a & b) / p == (a/p) & (b/p)`.
    fn projection_meet_homomorphism {
        let met = a & b;
        (&met / p) == ((a / p).to_version() & (b / p).to_version())
    }

    /// Projection is a short map (1-Lipschitz) for the metric quantities:
    /// masking both operands to one region can only shrink `distance` and
    /// `lag` — `d(a/p, b/p) <= d(a, b)` and `lag(a/p, b/p) <= lag(a, b)`.
    ///
    /// The projection homomorphism family's metric member: a region carries
    /// `rank(v) == rank(v/p) + rank(v/p̄)` (disjoint regions carve disjoint
    /// histories, and projection is additive over a region split), so the
    /// whole metric splits into the in-region part plus the complement's,
    /// each nonnegative — subtracting across a mask never inflates. At the
    /// seed party both sides are equal.
    fn projection_is_a_short_map {
        let (pa, pb) = ((a / p).to_version(), (b / p).to_version());
        pa.distance(&pb) <= a.distance(b) && pa.lag(&pb) <= a.lag(b)
    }

    /// Projection is monotone in the version: on the constructed comparable
    /// pair `a <= a | b`, the projections compare the same way — and whenever
    /// the inputs happen to compare directly, so do their projections.
    fn projection_monotone_in_version {
        let ab = a | b;
        let constructed = le_by(&(a / p), &(&ab / p));
        let incidental = !le(a, b) || le_by(&(a / p), &(b / p));
        constructed && incidental
    }

    /// `ticks` is a monoid action of the naturals: `ticks(n)` then
    /// `ticks(m)` equals `ticks(n + m)`.
    ///
    /// Quantified over the wide counts the two version operands' tick
    /// floors supply, all fused, so the law exercises counts no iterated
    /// reference could reach.
    fn ticks_composes {
        let (n, m) = (a.min_ticks(), b.min_ticks());
        let mut stepwise = a.clone();
        stepwise.ticks(p, n.clone());
        stepwise.ticks(p, m.clone());
        let mut joint = a.clone();
        joint.ticks(p, n + m);
        stepwise == joint
    }

    /// The view's heterogeneous comparisons are the materialized
    /// projection's, exactly: `(a/p) ⋚ b ≡ (a/p).to_version() ⋚ b`.
    ///
    /// Checked in both operand orders and under `==` — the three-stream
    /// differential law the fused co-walk is pinned by.
    fn own_version_cmp_matches_materialized {
        let view = a / p;
        let materialized = view.to_version();
        let cmp_agrees = view.partial_cmp(b) == materialized.partial_cmp(b);
        let cmp_reversed_agrees = b.partial_cmp(&view) == b.partial_cmp(&materialized);
        let eq_agrees = (view == *b) == (materialized == *b);
        let eq_reversed_agrees = (*b == view) == (*b == materialized);
        cmp_agrees && cmp_reversed_agrees && eq_agrees && eq_reversed_agrees
    }

    /// A heterogeneous comparison is the homogeneous comparison against the
    /// seed-masked view: `(a/p) ⋚ b ≡ (a/p) ⋚ (b/seed)`.
    ///
    /// Sound because projection by the seed party is the identity
    /// ([`seed_projection_is_identity`]), and the coherence that makes the
    /// three-stream walk a special case of the four-stream one.
    fn own_version_seed_mask_coherence {
        let view = a / p;
        let seed = Party::seed();
        let seeded = b / &seed;
        view.partial_cmp(&seeded) == view.partial_cmp(b) && (view == seeded) == (view == *b)
    }

    /// The quotient view of a span answers every verdict, and
    /// materializes, exactly as the eagerly projected span.
    ///
    /// Endpoints, the placement verdicts, the dominance and precedence
    /// coarsenings, membership, and both materialization doors —
    /// quantified over the pair's hull and the coincident span, with
    /// probes at the operands and the projected endpoints (reaching the
    /// at-endpoint corners). Every probe built
    /// from the operands dominates the projected start (projection only
    /// shrinks a version), so the concurrent-to-start placements are this
    /// law's negative space: the committed
    /// `own_span_place_reaches_every_concurrent_corner` witness beside
    /// the span tests constructs them.
    ///
    /// The eager side exists at all because projection is monotone
    /// (`projection_monotone_in_version`), which this law re-witnesses by
    /// validating the projected pair through [`Span::new`].
    fn own_span_matches_the_projected_span(a, b, party) {
        let hull = a.span(b);
        let coincident = a.span(a);
        for span in [&hull, &coincident] {
            let view = span / party;
            let lo = (span.lo() / party).to_version();
            let hi = (span.hi() / party).to_version();
            let Ok(eager) = Span::new(&lo, &hi) else {
                return false; // monotone masking never crosses a valid pair
            };
            let endpoints = view.lo() == lo && view.hi() == hi;
            let materialized = view.to_span() == eager && Span::from(view) == eager;
            if !(endpoints && materialized) {
                return false;
            }
            for probe in [a, b, &lo, &hi] {
                if view.place(probe) != eager.place(probe)
                    || view.dominance(probe) != eager.dominance(probe)
                    || view.precedence(probe) != eager.precedence(probe)
                    || view.contains(probe) != eager.contains(probe)
                {
                    return false;
                }
            }
        }
        true
    }

    /// The named spelling is the operator's, exactly: `span.project(p)` is
    /// `span / p` — the same endpoint views, the same materialization —
    /// quantified over the pair's hull and the coincident span.
    fn span_project_is_the_operator_spelling(a, b, party) {
        let hull = a.span(b);
        let coincident = a.span(a);
        for span in [&hull, &coincident] {
            let named = span.project(party);
            let operator = span / party;
            if named.lo() != operator.lo()
                || named.hi() != operator.hi()
                || named.to_span() != operator.to_span()
            {
                return false;
            }
        }
        true
    }
}

// ───────────────────────────── Version × Party × Party ─────────────────────────────

laws! {
    /// Laws over a version and two live parties: projection's interaction with
    /// the region geometry.
    pub static VERSION_PARTY_PAIR: (v: &Version, p: &Party, q: &Party);

    /// Successive projections commute: `(v / p) / q == (v / q) / p` (both keep
    /// exactly the history on the regions' intersection).
    ///
    /// The inner projections materialize (the outer projection needs a
    /// version to gate); the outer comparison is the fused view-vs-view walk.
    fn projection_commutes {
        let vp = (v / p).to_version();
        let vq = (v / q).to_version();
        (&vp / q) == (&vq / p)
    }

    /// Projection is monotone in the region: a constructed subregion (a fork
    /// half of `p`) keeps no more than `p` does — and whenever `p` happens to
    /// cover `q`, `v / q <= v / p`.
    fn projection_monotone_in_region {
        let mut keeper = p.dangerously_alias();
        let child = keeper.fork();
        let constructed = le_by(&(v / &child), &(v / p));
        let incidental = !p.covers(q) || le_by(&(v / q), &(v / p));
        constructed && incidental
    }

    /// Disjoint regions carve disjoint histories: `p · q = 0 ⟹ (v/p) & (v/q)`
    /// is empty (the projections' supports cannot overlap).
    fn disjoint_projections_share_nothing {
        !p.is_disjoint(q) || ((v / p).to_version() & (v / q).to_version()).is_empty()
    }

    /// Projection is additive over any without-carved decomposition of a
    /// region: when `r = p \ q` and `inner = p \ r` both survive, they
    /// partition `p`'s region, and `(v/r) | (v/inner) == v / p`.
    ///
    /// [`projection_additive_over_fork`] states additivity for the balanced
    /// fork geometry; the ragged region pairs `without` carves are that
    /// law's negative space, and overlapping arbitrary parties keep both
    /// remainders inhabited under mass. Vacuous only when `q` covers `p`
    /// (no outer remainder) or is disjoint from it (no inner part).
    fn projection_additive_over_carved_regions {
        let Some(r) = p.dangerously_alias().without(q) else {
            return true; // q covers p: no outer remainder to carve
        };
        let Some(inner) = p.dangerously_alias().without(&r) else {
            return true; // r == p (q disjoint from p): no inner part
        };
        ((v / &r).to_version() | (v / &inner).to_version()) == (v / p)
    }
}

// ──────────────────── Version × Version × Party × Party ────────────────────

laws! {
    /// Laws over two versions and two live parties: the homogeneous view
    /// comparison against its materialized oracle.
    pub static VERSION_PAIR_PARTY_PAIR: (a: &Version, b: &Version, p: &Party, q: &Party);

    /// The view's homogeneous comparisons are the materialized projections',
    /// exactly: `(a/p) ⋚ (b/q) ≡ (a/p).to_version() ⋚ (b/q).to_version()`,
    /// under `==` too — the four-stream differential law the fused co-walk is
    /// pinned by.
    fn own_version_pair_cmp_matches_materialized {
        let (va, vb) = (a / p, b / q);
        let (ma, mb) = (va.to_version(), vb.to_version());
        va.partial_cmp(&vb) == ma.partial_cmp(&mb) && (va == vb) == (ma == mb)
    }
}

// ───────────────────────────── Rank: triples ─────────────────────────────

laws! {
    /// Laws over a triple of ranks.
    ///
    /// `Rank` is a totally ordered commutative monoid: commutativity,
    /// associativity, the `ZERO` identity and bottom, add-monotonicity,
    /// `checked_sub` as the partial inverse defined exactly on domination, the
    /// order's duality, and cross-path normalization (value-equal ranks built
    /// along different operation paths are one structural value, equal under
    /// `Eq` and `Hash`). The wire form's laws ride the same group: the codec
    /// round-trip, byte order equal to `Ord`, and prefix-freedom.
    pub static RANK_TRIPLE: (a: &Rank, b: &Rank, c: &Rank);

    /// Addition is commutative: `a + b == b + a`.
    fn rank_add_commutative(a, b, _c) {
        a + b == b + a
    }

    /// Addition is associative: `(a + b) + c == a + (b + c)`.
    fn rank_add_associative {
        &(a + b) + c == a + &(b + c)
    }

    /// `ZERO` is the additive identity.
    fn rank_zero_is_identity(a, _b, _c) {
        a + &Rank::ZERO == a.clone()
    }

    /// `ZERO` is the order's bottom: no rank sits below it.
    fn rank_zero_is_bottom(a, _b, _c) {
        Rank::ZERO <= *a
    }

    /// Addition never shrinks a rank: `a + b >= a`.
    fn rank_add_monotone(a, b, _c) {
        &(a + b) >= a
    }

    /// `checked_sub` inverts addition: `(a + b) - b == a`.
    fn rank_sub_inverts_add(a, b, _c) {
        (a + b).checked_sub(b) == Some(a.clone())
    }

    /// `checked_sub` is defined exactly on domination: `a - b` is `Some` iff
    /// `b <= a`.
    fn rank_checked_sub_iff_dominated(a, b, _c) {
        a.checked_sub(b).is_some() == (b <= a)
    }

    /// Where defined, subtraction restores: `(a - b) + b == a`.
    fn rank_sub_then_add_restores(a, b, _c) {
        match a.checked_sub(b) {
            Some(difference) => &difference + b == a.clone(),
            None => true,
        }
    }

    /// `saturating_sub` is `checked_sub` with the nonexistent difference
    /// floored: equal where the difference exists, `ZERO` exactly
    /// otherwise.
    fn rank_saturating_sub_is_checked_sub_floored(a, b, _c) {
        a.saturating_sub(b) == a.checked_sub(b).unwrap_or(Rank::ZERO)
    }

    /// Saturation reaches the floor from every deficit: `a - (a + b)` is
    /// `ZERO` (with `b == ZERO` the degenerate equal-operands arm).
    fn rank_saturating_sub_saturates_at_zero(a, b, _c) {
        a.saturating_sub(&(a + b)) == Rank::ZERO
    }

    /// The total order is its own dual: `cmp(a, b)` is `cmp(b, a)` reversed.
    fn rank_cmp_antisymmetric(a, b, _c) {
        a.cmp(b) == b.cmp(a).reverse()
    }

    /// `decode ∘ encode == id`, and the round-tripped rank re-encodes to the
    /// same bytes (the wire form is a section of canonical bytes).
    fn rank_codec_roundtrip(a, _b, _c) {
        let bytes = a.encode();
        Rank::decode(&bytes[..]).is_ok_and(|decoded| decoded == *a && decoded.encode() == bytes)
    }

    /// THE LAW of the rank wire form: byte-wise lexicographic order on
    /// canonical encodings equals `Ord` on the ranks — ties included, so byte
    /// equality on encodings is exactly `Eq`.
    fn rank_lex_order(a, b, _c) {
        a.encode().cmp(&b.encode()) == a.cmp(b)
    }

    /// Rank encodings are prefix-free: distinct ranks' encodings are never
    /// byte prefixes of one another — what lets one encoding self-delimit
    /// inside a composite key, and byte order stay rank order under any
    /// appended tiebreak.
    fn rank_encoding_prefix_free(a, b, _c) {
        a == b || {
            let (ea, eb) = (a.encode(), b.encode());
            !ea.starts_with(&eb) && !eb.starts_with(&ea)
        }
    }

    /// Value-equal ranks built along different operation paths — pairwise
    /// addition, `Sum`, and add-then-subtract — are one structural value.
    ///
    /// Equal under `Eq` and under `Hash`: the normalization invariant `Ord`'s
    /// class-first fast path and every container key rest on.
    fn rank_cross_path_normalization {
        let via_add = a + b;
        let via_sum = [a.clone(), b.clone()].into_iter().sum::<Rank>();
        let via_sub = (&(a + b) + c).checked_sub(c);
        via_add == via_sum
            && via_sub == Some(via_add.clone())
            && hash_of(&via_add) == hash_of(&via_sum)
    }
}

// ───────────────────────────── Clock: one value ─────────────────────────────

laws! {
    /// Laws over one clock.
    ///
    /// The fork-event-join model's composite operations on a whole stamp:
    /// `fork` preserves the event component and splits the id, the balanced
    /// n-way fork's two forms agree, `tick`/`send` advance strictly and fix
    /// the party, `ticks` agrees with the version entry point, peeks are
    /// stable, an own-message receive is a bare tick, an absorb is the
    /// anonymous join with no event minted, `sync` reconciles a fork,
    /// `own_version` is the projection, and the parts/codec/text
    /// round-trips.
    pub static CLOCK_SOLO: (c: &Clock);

    /// `fork` preserves the version on both halves (§3: fork clones the causal
    /// past).
    fn fork_preserves_version {
        let mut keeper = c.dangerously_alias();
        let child = keeper.fork();
        keeper.version() == c.version() && child.version() == c.version()
    }

    /// `fork` splits the id: the two halves' parties are disjoint, and each is
    /// covered by the original.
    fn fork_splits_the_party {
        let mut keeper = c.dangerously_alias();
        let child = keeper.fork();
        keeper.party().is_disjoint(child.party())
            && c.party().covers(keeper.party())
            && c.party().covers(child.party())
    }

    /// `fork` then `join` restores the clock exactly: the party halves rejoin
    /// and the version join is idempotent (`e ⊔ e == e`).
    fn fork_join_restores_the_clock {
        let mut keeper = c.dangerously_alias();
        let child = keeper.fork();
        keeper.join(child).is_ok() && keeper == *c
    }

    /// The two balanced-fork forms agree at the clock level: `From<Clock>`
    /// for `[Clock; N]` equals the residual the borrowing `forks(N - 1)`
    /// keeps, followed by the shares it yields (`[residual] ++ forks`).
    ///
    /// Every child pairs its party share with a clone of the parent
    /// version.
    fn clock_forks_matches_from_array {
        const N: usize = 4;
        let array: [Clock; N] = c.dangerously_alias().into();
        let mut keeper = c.dangerously_alias();
        let yielded: Vec<Clock> = keeper.forks(N as u64 - 1).collect();
        let reconstructed: Vec<Clock> = std::iter::once(keeper).chain(yielded).collect();
        array.iter().eq(reconstructed.iter())
    }

    /// `version()` (peek) does not advance the clock: repeated peeks are equal
    /// and the clock's bytes are unchanged.
    fn peek_is_stable {
        let before = c.encode();
        let first = c.version().clone();
        first == *c.version() && c.encode() == before
    }

    /// `tick` strictly advances the version and leaves the party untouched.
    fn clock_tick_advances_and_fixes_party {
        let mut ticked = c.dangerously_alias();
        ticked.tick();
        le(c.version(), ticked.version())
            && c.version() != ticked.version()
            && ticked.party() == c.party()
    }

    /// `receive` of a dominated message (here the clock's own version) equals a
    /// bare `tick`: an own-message receive is benign.
    fn own_receive_is_tick {
        let mut received = c.dangerously_alias();
        let mut ticked = c.dangerously_alias();
        let own = received.version().clone();
        received.recv(&own);
        ticked.tick();
        received == ticked
    }

    /// The clock's `ticks` agrees with the version-level `ticks` on its own
    /// parts, and returns the freshly advanced version.
    fn clock_ticks_matches_version_ticks {
        let n = Ticks::from(3u64);
        let mut via_clock = c.dangerously_alias();
        let returned = via_clock.ticks(n.clone()).clone();
        let mut expected = c.version().clone();
        expected.ticks(c.party(), n);
        returned == expected && *via_clock.version() == expected
    }

    /// `send` (event then peek) returns the freshly advanced version: the
    /// returned message equals the clock's version and strictly dominates the
    /// pre-send version.
    fn send_advances_and_returns_the_version {
        let mut sender = c.dangerously_alias();
        let sent = sender.send().clone();
        sent == *sender.version() && le(c.version(), &sent) && *c.version() != sent
    }

    /// `sync` (join then fork) reconciles a fork: after two concurrent ticks,
    /// both sides end at the ticks' join, with disjoint parties whose rejoin
    /// recovers the original region.
    fn sync_reconciles_a_fork {
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
    fn own_version_is_the_projection {
        c.own_version() == (c.version() / c.party())
    }

    /// `from_parts ∘ into_parts == id`.
    fn parts_roundtrip {
        let (party, version) = c.dangerously_alias().into_parts();
        Clock::from_parts(party, version) == *c
    }

    /// `decode ∘ encode == id`, and the round-tripped clock re-encodes to the
    /// same bytes.
    fn clock_codec_roundtrip {
        let bytes = c.encode();
        Clock::decode(&bytes[..]).is_ok_and(|decoded| decoded == *c && decoded.encode() == bytes)
    }

    /// `FromStr ∘ Display == id`: the paper notation round-trips.
    fn clock_text_roundtrip {
        c.to_string()
            .parse::<Clock>()
            .is_ok_and(|parsed| parsed == *c)
    }

    /// The clock's encoding is its party's bytes then its version's, exactly
    /// (each part byte-aligned and independently canonical).
    fn encode_frames_party_then_version {
        c.encode() == [c.party().encode(), c.version().encode()].concat()
    }

    /// `encoded_bits` is the pre-pad bit length of `encode`, at the clock level
    /// too.
    fn clock_encoded_bits_matches_encode_len {
        c.encode().len() == (c.encoded_bits() + 1).div_ceil(8)
    }
}

// ───────────────────────────── Clock: pairs ─────────────────────────────

laws! {
    /// Laws over a pair of clocks.
    ///
    /// The representational pair laws [`Version`] and [`Party`] each carry,
    /// closed over the whole stamp: `Eq` rides the canonical byte encoding,
    /// and equal clocks hash equally — what container keys on stamps rest
    /// on.
    pub static CLOCK_PAIR: (a: &Clock, b: &Clock);

    /// `Eq` is canonical-byte equality on whole stamps: `a == b ⟺
    /// encode(a) == encode(b)`.
    ///
    /// Both directions matter: equal clocks must encode identically (the
    /// canonical encoding is a function of the value), and distinct clocks
    /// must encode distinctly (injectivity — what byte-level `Eq`/`Hash`
    /// uses rest on). With the frame pinned as the party's bytes then the
    /// version's ([`encode_frames_party_then_version`]), the biconditional
    /// also pins the id/event boundary unambiguous: a difference in either
    /// component alone changes the composite bytes.
    fn clock_eq_iff_bytes_eq {
        (a == b) == (a.encode() == b.encode())
    }

    /// `Eq`/`Hash` coherence: equal clocks hash equally.
    fn clock_eq_implies_hash_eq {
        a != b || hash_of(a) == hash_of(b)
    }
}

// ───────────────────────────── Clock × Version ─────────────────────────────

laws! {
    /// Laws over a clock and a message version.
    ///
    /// The receive laws (join-then-event: the result dominates both the old
    /// version and the message, strictly past their join, with the party
    /// untouched), the composition laws (`recv` and `sync` equal the
    /// compositions of the public operations they fuse, value for value),
    /// and the anonymous-join operators.
    pub static CLOCK_VERSION: (c: &Clock, msg: &Version);

    /// `recv` (join then event) learns the message and advances past it: the
    /// result dominates `old | msg` strictly, and the returned reference is the
    /// clock's new version.
    fn recv_learns_and_advances {
        let mut receiver = c.dangerously_alias();
        let old = receiver.version().clone();
        let returned = receiver.recv(msg).clone();
        let now = receiver.version().clone();
        let lub = &old | msg;
        returned == now && le(&lub, &now) && lub != now
    }

    /// `recv` never changes the id: message reception is an anonymous join.
    fn recv_fixes_party {
        let mut receiver = c.dangerously_alias();
        receiver.recv(msg);
        receiver.party() == c.party()
    }

    /// `recv` equals its stated composition — join the message into the
    /// version, then [`Clock::tick`] — value for value, returned reference
    /// included: reception is exactly the two public operations, however it
    /// is computed.
    fn recv_is_join_then_tick {
        let mut fused = c.dangerously_alias();
        let returned = fused.recv(msg).clone();
        let (party, version) = c.dangerously_alias().into_parts();
        let mut composed = Clock::from_parts(party, version | msg);
        composed.tick();
        returned == *composed.version() && fused == composed
    }

    /// `sync` equals its stated composition — [`Clock::join`] then
    /// [`Clock::fork`] — outcome for outcome.
    ///
    /// The disjoint arm is constructed by forking the clock and letting the
    /// sides diverge (one ticking, one receiving the message): it must
    /// reconcile to exactly the joined-then-reforked pair. The overlap arm
    /// is the clock against its own alias: it must be refused with neither
    /// side moved, exactly where `join` refuses.
    fn sync_is_join_then_fork {
        // The disjoint arm.
        let mut a = c.dangerously_alias();
        let mut b = a.fork();
        a.tick();
        b.recv(msg);
        let mut fused_a = a.dangerously_alias();
        let mut fused_b = b.dangerously_alias();
        let Ok(returned) = fused_a.sync(&mut fused_b).cloned() else {
            return false; // forked halves are disjoint: sync must accept
        };
        let mut composed_a = a.dangerously_alias();
        if composed_a.join(b.dangerously_alias()).is_err() {
            return false; // forked halves are disjoint: join must accept
        }
        let composed_b = composed_a.fork();
        if fused_a != composed_a || fused_b != composed_b || returned != *composed_a.version() {
            return false;
        }
        // The overlap arm: a clock shares its whole region with its alias.
        let mut x = c.dangerously_alias();
        let mut y = c.dangerously_alias();
        x.sync(&mut y).is_err()
            && c.dangerously_alias().join(c.dangerously_alias()).is_err()
            && x == *c
            && y == *c
    }

    /// The anonymous joins `Clock | Version` and `Version | Clock` merge the
    /// versions and keep the clock's party, and agree with each other.
    fn anonymous_join_merges_versions {
        let clock_version = c.dangerously_alias() | msg.clone();
        let version_clock = msg.clone() | c.dangerously_alias();
        clock_version.party() == c.party()
            && *clock_version.version() == (c.version() | msg)
            && version_clock == clock_version
    }

    /// `absorb` is the anonymous join with no event minted: the version
    /// becomes exactly `old | msg`, the party never moves, and the returned
    /// reference is the clock's new version.
    ///
    /// Absorbing the same message a second time changes nothing.
    fn absorb_is_the_anonymous_join {
        let mut fused = c.dangerously_alias();
        let returned = fused.absorb(msg).clone();
        let (party, version) = c.dangerously_alias().into_parts();
        let composed = Clock::from_parts(party, version | msg);
        let mut again = fused.dangerously_alias();
        again.absorb(msg);
        returned == *fused.version() && fused == composed && again == fused
    }
}

// ───────────────────── Clock: a receiver and items ─────────────────────

laws! {
    /// Laws over a clock (the receiver) and a list of clocks (the items),
    /// at any arity.
    ///
    /// [`Clock::join_all`]'s faces at swept widths: acceptance is
    /// pairwise disjointness of the parties (an accepted fold equals the
    /// sequential pair joins on both components, and the returned
    /// reference is the freshly folded version), a constructed fork
    /// family — every child line ticked apart — reunites to the original
    /// region carrying the join of every line's history, and rejection
    /// conserves the family's regions and histories. Beside them, the
    /// n-ary doors are pinned to their composed spellings:
    /// [`Clock::sync_all`] byte-identical to `join_all` then the balanced
    /// re-share — with every overlap refused and no participant moved —
    /// [`Clock::recv_all`] to the sequential binary joins followed by one
    /// tick, and [`Clock::absorb_all`] to the same joins with no tick at
    /// all.
    pub static CLOCK_AND_LIST: (c: &Clock, items: &[Clock]);

    /// `join_all` accepts exactly the families whose parties — the
    /// receiver's included — are pairwise disjoint, at every arity.
    ///
    /// An accepted fold equals the sequential pair joins (party union and
    /// version join alike, through the bound pair operation, never the
    /// n-ary door) and returns the folded version; a refused one still
    /// absorbed every clock it could — the party covers its original
    /// region and the version dominates its original — and handed at
    /// least one input back.
    fn clock_join_all_accepts_iff_parties_pairwise_disjoint {
        let pairwise_disjoint = {
            let family: Vec<&Party> = core::iter::once(c.party())
                .chain(items.iter().map(Clock::party))
                .collect();
            family
                .iter()
                .enumerate()
                .all(|(i, a)| family[i + 1..].iter().all(|b| a.is_disjoint(b)))
        };
        let mut acc = c.dangerously_alias();
        match acc.join_all(items.iter().map(Clock::dangerously_alias)) {
            Ok(returned) => {
                let returned = returned.clone();
                let mut seq = c.dangerously_alias();
                let sequential = items
                    .iter()
                    .all(|item| seq.join(item.dangerously_alias()).is_ok());
                pairwise_disjoint && sequential && acc == seq && returned == *acc.version()
            }
            Err(returned) => {
                !pairwise_disjoint
                    && !returned.is_empty()
                    && acc.party().covers(c.party())
                    && le(c.version(), acc.version())
            }
        }
    }

    /// `join_all` reunites a fork family at every width: fork `k`
    /// children, tick each so every line carries history its siblings
    /// lack, and the fold restores the original region with the join of
    /// every line's version.
    ///
    /// The expected version is the sequential pair fold of the lines'
    /// histories (the bound `|`, never the n-ary door); the returned
    /// reference and the folded clock's version must both realize it,
    /// and the party must come back exactly the receiver's. Width zero
    /// is the empty-fold identity.
    fn clock_join_all_reunites_forks_at_any_width {
        let width = items.len();
        let mut keeper = c.dangerously_alias();
        let mut children: Vec<Clock> = keeper.forks(width as u64).collect();
        for child in &mut children {
            child.tick();
        }
        let expected = children
            .iter()
            .fold(keeper.version().clone(), |acc, child| {
                &acc | child.version()
            });
        match keeper.join_all(children) {
            Ok(returned) => {
                let returned = returned.clone();
                returned == expected && *keeper.version() == expected && keeper.party() == c.party()
            }
            Err(_) => false,
        }
    }

    /// `join_all`'s rejection conserves the family at every arity:
    /// joined with the returned clocks, the accumulator covers the
    /// receiver's original region and history and every input's.
    ///
    /// Spelled out: the accumulator's party joined with the returned
    /// parties covers the receiver's original region unioned with every
    /// input's, and the accumulator's version joined with the returned
    /// versions dominates the receiver's original version and every
    /// input's.
    ///
    /// The clock face of the party group's conservation law, stated
    /// over unions/joins on both components, never byte identity: the
    /// closing drain legitimately hands back *coalesced* groups
    /// (parties unioned, versions joined), so an element-wise identity
    /// clause would reject correct behavior. What the union statement
    /// convicts is a fold that *drops* a group — region and history
    /// alike — instead of handing it back, a loss invisible to the
    /// acceptance law's `Err` clauses whenever another input already
    /// sits in the rejection channel.
    fn clock_join_all_err_conserves_the_region_union {
        let mut acc = c.dangerously_alias();
        match acc.join_all(items.iter().map(Clock::dangerously_alias)) {
            Ok(_) => true, // an accepted fold equals the sequential pair joins
            Err(returned) => {
                // The union of the accumulator's region with every
                // returned one (each hand-back may overlap the union —
                // aliases are why it came back — so it contributes its
                // uncovered remainder), and the join of the
                // accumulator's history with every returned one.
                let (mut union, mut history) = acc.into_parts();
                for back in returned {
                    let (party, version) = back.into_parts();
                    if let Some(missing) = party.without(&union) {
                        if union.join(missing).is_err() {
                            return false; // the remainder is disjoint by construction
                        }
                    }
                    history = &history | &version;
                }
                union.covers(c.party())
                    && le(c.version(), &history)
                    && items
                        .iter()
                        .all(|item| union.covers(item.party()) && le(item.version(), &history))
            }
        }
    }

    /// `sync_all` equals its stated composition — [`Clock::join_all`] then
    /// [`Clock::forks`] — outcome for outcome, at every arity.
    ///
    /// The disjoint arm forks the receiver into one child per item and lets
    /// every line diverge (each child absorbs one item's version, then
    /// ticks): the fused reconcile must leave every participant — the
    /// receiver, each child in order, and the returned version —
    /// byte-identical to folding the children in with `join_all` and
    /// re-sharing with `forks`. The overlap arms must be refused with no
    /// participant moved: the receiver against its own alias, and a family
    /// carrying a duplicated child.
    fn sync_all_is_join_all_then_forks {
        // The disjoint arm: a diverged fork family of the receiver.
        let mut parent = c.dangerously_alias();
        let mut children: Vec<Clock> = parent.forks(items.len() as u64).collect();
        for (child, item) in children.iter_mut().zip(items) {
            *child |= item.version();
            child.tick();
        }
        let mut fused = parent.dangerously_alias();
        let mut fused_children: Vec<Clock> =
            children.iter().map(Clock::dangerously_alias).collect();
        let Ok(returned) = fused.sync_all(fused_children.iter_mut()).cloned() else {
            return false; // fork shares are disjoint: sync_all must accept
        };
        let mut composed = parent.dangerously_alias();
        if composed
            .join_all(children.iter().map(Clock::dangerously_alias))
            .is_err()
        {
            return false; // fork shares are disjoint: join_all must accept
        }
        let composed_children: Vec<Clock> = composed.forks(children.len() as u64).collect();
        if fused != composed
            || returned != *composed.version()
            || fused_children != composed_children
        {
            return false;
        }
        // The overlap arms: refusal with nothing moved, both for a receiver
        // overlapping an item and for two items overlapping each other.
        let mut x = c.dangerously_alias();
        let mut y = c.dangerously_alias();
        let receiver_overlap = x.sync_all([&mut y]).is_err() && x == *c && y == *c;
        let mut p = c.dangerously_alias();
        let mut child = p.fork();
        let mut dup = child.dangerously_alias();
        let (p0, child0, dup0) = (
            p.dangerously_alias(),
            child.dangerously_alias(),
            dup.dangerously_alias(),
        );
        let item_overlap = p.sync_all([&mut child, &mut dup]).is_err()
            && p == p0
            && child == child0
            && dup == dup0;
        receiver_overlap && item_overlap
    }

    /// `recv_all` equals its stated composition — join every message into
    /// the version through the bound binary join, then one [`Clock::tick`]
    /// — value for value at every arity.
    ///
    /// Returned reference included: the n-ary door adds no observable
    /// behavior of its own.
    ///
    /// The items' versions serve as the message list; the party never
    /// moving and the empty list being a bare tick both ride the whole-clock
    /// comparison.
    fn recv_all_is_joins_then_tick {
        let mut fused = c.dangerously_alias();
        let returned = fused
            .recv_all(items.iter().map(|item| item.version()))
            .clone();
        let mut composed = c.dangerously_alias();
        for item in items {
            composed |= item.version();
        }
        composed.tick();
        returned == *composed.version() && fused == composed
    }

    /// `absorb_all` equals its stated composition — every message joined in
    /// through the bound binary join, no event minted — value for value at
    /// every arity, returned reference included.
    ///
    /// The items' versions serve as the message list; the party never
    /// moving and the empty list changing nothing both ride the whole-clock
    /// comparison.
    fn absorb_all_is_the_sequential_joins {
        let mut fused = c.dangerously_alias();
        let returned = fused
            .absorb_all(items.iter().map(|item| item.version()))
            .clone();
        let mut composed = c.dangerously_alias();
        for item in items {
            composed |= item.version();
        }
        returned == *composed.version() && fused == composed
    }
}

#[cfg(test)]
mod tests;
