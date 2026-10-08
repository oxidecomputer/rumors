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
use crate::{Clock, Count, Party, Rank, Ranked, Version};

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
        /// names (currently the uniqueness pin) resolves against what actually
        /// runs, never against a text scan that a stray same-named `fn` could
        /// satisfy.
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

/// Hashes one value for laws that check `Eq` and `Hash` coherence.
///
/// `DefaultHasher::new()` is deterministic within one build, so values that
/// feed the hasher the same data hash equally. Accepting unsized values
/// (`[u8]`) lets a law compare a value's hash with its bytes' hash.
fn hash_of<T: Hash + ?Sized>(value: &T) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

/// Whether `iter`'s size hint is exactly `(remaining, Some(remaining))` before
/// every step and after exhaustion, for an iterator that yields `len` items.
fn size_hints_are_exact<I: Iterator>(mut iter: I, len: usize) -> bool {
    for remaining in (1..=len).rev() {
        if iter.size_hint() != (remaining, Some(remaining)) || iter.next().is_none() {
            return false;
        }
    }
    iter.size_hint() == (0, Some(0)) && iter.next().is_none()
}

/// Tests whether two families of parties own every point equally often.
///
/// A point's *multiplicity* in a family is the number of the family's parties
/// that own it. The families agree when every point of `[0, 1)` has the same
/// multiplicity in `lhs` as in `rhs`. Comparing unions cannot see a region that
/// one family holds twice; this comparison can.
///
/// The tree oracle's test-only `same_multiplicity` states the same comparison
/// over oracle trees. This one uses only the public API, so laws can apply it
/// wherever they run, the fuzz targets included; a test in this module's suite
/// checks that the two agree.
fn same_multiplicity(lhs: &[&Party], rhs: &[&Party]) -> bool {
    match (owner_layers(lhs), owner_layers(rhs)) {
        (Some(lhs), Some(rhs)) => lhs == rhs,
        _ => false,
    }
}

/// Sorts a family's ownership into nested layers: layer `k`, counting from
/// zero, holds exactly the points that more than `k` of the family's parties
/// own.
///
/// A point's multiplicity is the number of layers that hold it, so two families
/// have equal multiplicities everywhere exactly when their layers are equal.
/// Adding a party raises the multiplicity of each of its points by one, so it
/// visits the layers from the bottom: its points missing from a layer join that
/// layer, and its points already there go on to the next. Points that pass the
/// top layer start a new one.
///
/// Returns `None` if joining a remainder into its layer fails. A correct
/// `without` makes that impossible, because the remainder is disjoint from the
/// layer by construction.
fn owner_layers(family: &[&Party]) -> Option<Vec<Party>> {
    let mut layers: Vec<Party> = Vec::new();
    for party in family {
        let mut carry = Some(party.dangerously_alias());
        for layer in &mut layers {
            let Some(incoming) = carry.take() else { break };
            let fresh = incoming.dangerously_alias().without(layer);
            carry = match &fresh {
                Some(fresh) => incoming.without(fresh),
                None => Some(incoming),
            };
            if let Some(fresh) = fresh {
                layer.join(fresh).ok()?;
            }
        }
        layers.extend(carry);
    }
    Some(layers)
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
