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
//! partial order is causality, parties join when disjoint and `fork` splits
//! ownership, events inflate strictly and only within the owned region, and
//! `rank` is a strictly monotone
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

use crate::causally::{self, Coverage, Query};
use crate::error::Crossed;
use crate::span::{Dominance, Endpoint, Placement, Precedence, Span};
use crate::{Clock, Party, Rank, Ranked, Ticks, Version};

/// A named law: the name a failure reports, and the predicate that must
/// hold on every admissible input.
pub type Law<F> = (&'static str, F);

/// Apply a function to each algebraic law asserted about this crate.
///
/// This is the single registration point for law groups. The macro passes the
/// roster to a caller-supplied macro, with an optional argument clause
/// (`consumer(args)`) forwarded as `args: (...)`. Consumers select inputs by
/// signature: a new known signature is driven automatically, while a new
/// signature fails to compile until the consumer defines how to supply it.
/// A test independently checks that every declared group appears here.
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
        /// declarations in the focused law modules.
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
/// module's tests closes it by comparing the roster against the `pub static`
/// declarations across the focused law modules. The header keeps that literal
/// spelling so the scan stays exact and simple.
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

/// Returns whether `a` causally precedes or equals `b`.
fn le(a: &Version, b: &Version) -> bool {
    a.partial_cmp(b).is_some_and(|o| o != Ordering::Greater)
}

/// Returns causal `<=` for any pair of comparable version views.
fn le_by<L: PartialOrd<R>, R>(a: &L, b: &R) -> bool {
    a.partial_cmp(b).is_some_and(|o| o != Ordering::Greater)
}

/// Hashes one value for laws that check `Eq` and `Hash` coherence.
fn hash_of<T: Hash>(value: &T) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

mod clock;
mod party;
mod rank;
mod version_list;
mod version_pair;
mod version_party;
mod version_solo;
mod version_triple;

pub use clock::{CLOCK_AND_LIST, CLOCK_PAIR, CLOCK_SOLO, CLOCK_VERSION};
pub use party::{PARTY_AND_LIST, PARTY_PAIR, PARTY_SOLO, PARTY_TRIPLE};
pub use rank::RANK_TRIPLE;
pub use version_list::{VERSION_AND_LIST, VERSION_LIST};
pub use version_pair::VERSION_PAIR;
pub use version_party::{
    VERSION_PAIR_PARTY, VERSION_PAIR_PARTY_PAIR, VERSION_PARTY, VERSION_PARTY_PAIR,
};
pub use version_solo::VERSION_SOLO;
pub use version_triple::VERSION_TRIPLE;

#[cfg(test)]
mod tests;
