//! The pointwise differential table: one descriptor per public operation
//! whose prod↔tree leg is a pure function of version, id, and clock values.
//!
//! A *descriptor* names an operation once and spells it twice — once on
//! production, once on the recursive oracle — and the drivers in this
//! module's tests run every descriptor over shared populations. Registration
//! is execution: a descriptor cannot be written without being registered
//! under its own name, and every consumer expands the group roster
//! (`for_each_diff_group!`), so a descriptor added to a group with a known
//! signature is driven over every population with no further wiring, and a
//! group with a novel signature refuses to compile until each consumer says
//! how to feed it. The reverse door — a `pub(crate) static` group missing
//! from the roster, which nothing would execute — is closed by the totality
//! pin in this module's tests.
//!
//! # Why a table
//!
//! The operations here are pointwise pure functions over the same two
//! carriers, so a hand-written body per operation *per population* is one
//! semantics written three times. The table dissolves that product: the
//! population is the driver's, the operation is the descriptor's, and the
//! two never multiply in source. What is bought is totality by
//! construction — every registered descriptor meets every population — and
//! what is paid is transcription centralization: one descriptor is the only
//! spelling of the oracle side, where a body per population was three
//! independent spellings. The counterweight is the committed known-bad
//! descriptors this module's tests hold convicted; a table whose drivers
//! cannot reject a mis-transcribed descriptor is decoration.
//!
//! # The boundary
//!
//! The table covers what a value-returning descriptor states honestly. The
//! operations it does not cover keep their hand-written bodies and are
//! rostered in [`DIFF_BESPOKE`] under a [`BespokeGenre`], so "bespoke" is a
//! closed status a reviewer diffs rather than the default anything falls
//! into. The tiling pin in this module's tests holds every `Bound` citation
//! in [`crate::surface`] to exactly one side: derived from this table, or
//! bespoke with a declared genre — never both, never neither.

use std::cmp::Ordering;

use crate::testing::bridge;
use crate::{oracle, Party, Rank, Ticks, Version};

/// One descriptor: its name and the check the drivers run.
///
/// The same shape as [`crate::laws::Law`], and for the same reason: an
/// assertion that fails names the entry it came from.
pub(crate) type DiffOp<F> = (&'static str, F);

/// How a production result is compared with the recursive oracle's.
///
/// Implemented once per result type a descriptor can produce, so a
/// descriptor never spells its own comparison and the drivers never carry
/// an assert-kind switch: the result types decide. Verdicts and carrier
/// quantities compare directly; tree-shaped results compare *both ways*
/// across the bridge — the production value equals the oracle value raised
/// through it, and lowering the production value returns the oracle value —
/// so a result that is right in meaning but not in normal form is a
/// failure, not a pass.
pub(crate) trait Matches<Reference> {
    /// Whether this production result agrees with the oracle's.
    fn matches(&self, reference: &Reference) -> bool;
}

impl Matches<oracle::Version> for Version {
    fn matches(&self, reference: &oracle::Version) -> bool {
        *self == bridge::from_oracle_version(reference)
            && bridge::to_oracle_version(self) == *reference
    }
}

impl Matches<oracle::Party> for Option<Party> {
    /// The oracle carries the empty region as a value where production
    /// carries it as `None`, so the arms are matched before the trees.
    fn matches(&self, reference: &oracle::Party) -> bool {
        match self {
            None => reference.is_empty(),
            Some(party) => {
                !reference.is_empty()
                    && *party == bridge::from_oracle_party(reference)
                    && bridge::to_oracle_party(party) == *reference
            }
        }
    }
}

impl Matches<bool> for bool {
    fn matches(&self, reference: &bool) -> bool {
        self == reference
    }
}

impl Matches<Option<Ordering>> for Option<Ordering> {
    fn matches(&self, reference: &Option<Ordering>) -> bool {
        self == reference
    }
}

impl Matches<Ticks> for Ticks {
    fn matches(&self, reference: &Ticks) -> bool {
        self == reference
    }
}

impl Matches<Rank> for Rank {
    fn matches(&self, reference: &Rank) -> bool {
        self == reference
    }
}

/// Declares one descriptor group: the group's `pub(crate) static` slice and
/// every descriptor in it, from a single spelling.
///
/// The header names the group and the input signature every descriptor in
/// the block shares, each input tagged with the carrier it borrows
/// (`version`, `party`, `clock`, `ticks`). Each `fn` that follows is one
/// descriptor: it names the operation and spells it twice, `prod:` against
/// the production types and `tree:` against the recursive oracle's, and the
/// macro registers it in the slice under its own name (`stringify!`ed) — a
/// descriptor cannot be written without being registered, nor registered
/// under a name that is not its own.
///
/// The two spellings are written in one place, side by side, over
/// identically named bindings: what the transcription centralizes, it also
/// makes diffable. Each side gets its own scope holding its own values —
/// production values raised from the oracle carriers through the bridge,
/// oracle values cloned — so both sides may consume or mutate freely, and
/// `!Clone` production types are simply rebuilt per descriptor. Whether the
/// two results agree is the result types' business, through [`Matches`];
/// the descriptor states only the two spellings.
///
/// Group membership is the block, and registration in the roster
/// (`for_each_diff_group!`) stays a separate step that the totality pin in
/// this module's tests closes — which is why the header keeps the literal
/// `pub(crate) static` spelling that pin scans for.
macro_rules! diff_ops {
    // In both the matcher and the transcriber, the attributes and the
    // declaration share a line: the registration totality pin's source scan
    // reads any line starting `pub(crate) static` as a group declaration,
    // and must see the invocations' headers only, never this definition.
    (
        $(#[$group_meta:meta])* pub(crate) static $group:ident: ($($param:ident: $kind:tt),+ $(,)?);
        $(
            $(#[$op_meta:meta])*
            fn $op:ident {
                prod: $prod:expr,
                tree: $tree:expr $(,)?
            }
        )+
    ) => {
        $(#[$group_meta])* pub(crate) static $group: &[$crate::testing::diff_ops::DiffOp<
            fn($(&diff_ops!(@oracle $kind)),+) -> bool,
        >] = &[$((stringify!($op), $op)),+];
        diff_ops! {
            @ops ($($param: $kind),+);
            $(
                $(#[$op_meta])*
                fn $op { prod: $prod, tree: $tree }
            )+
        }
    };

    // Peel one descriptor at a time: the header signature is re-carried to
    // every descriptor as a plain token list, which sidesteps the
    // transcriber depth rule (a header-level repetition cannot be
    // re-expanded inside the per-descriptor repetition above).
    (@ops ($($signature:tt)+);) => {};
    (
        @ops ($($signature:tt)+);
        $(#[$op_meta:meta])*
        fn $op:ident { prod: $prod:expr, tree: $tree:expr }
        $($rest:tt)*
    ) => {
        diff_ops! { @op ($($signature)+); $(#[$op_meta])* fn $op { prod: $prod, tree: $tree } }
        diff_ops! { @ops ($($signature)+); $($rest)* }
    };
    (
        @op ($($param:ident: $kind:tt),+ $(,)?);
        $(#[$op_meta:meta])*
        fn $op:ident { prod: $prod:expr, tree: $tree:expr }
    ) => {
        $(#[$op_meta])*
        // The bindings are uniformly mutable so a descriptor may spell an
        // operation that mutates its receiver; most do not.
        #[allow(unused_mut)]
        fn $op($($param: &diff_ops!(@oracle $kind)),+) -> bool {
            let prod = { $( let mut $param = diff_ops!(@lower $kind, $param); )+ $prod };
            let tree = { $( let mut $param = ::core::clone::Clone::clone($param); )+ $tree };
            $crate::testing::diff_ops::Matches::matches(&prod, &tree)
        }
    };

    // The carrier tags: each names the oracle-side type a driver supplies
    // and the production value raised from it.
    (@oracle version) => { $crate::oracle::Version };
    (@oracle party) => { $crate::oracle::Party };
    (@oracle clock) => { $crate::oracle::Clock };
    (@oracle ticks) => { $crate::Ticks };
    (@lower version, $carrier:expr) => { $crate::testing::bridge::from_oracle_version($carrier) };
    (@lower party, $carrier:expr) => { $crate::testing::bridge::from_oracle_party($carrier) };
    (@lower clock, $carrier:expr) => { $crate::testing::bridge::from_oracle_clock($carrier) };
    (@lower ticks, $carrier:expr) => { ::core::clone::Clone::clone($carrier) };
}

/// Why an operation's `Bound` differential resists a descriptor.
///
/// A descriptor states one thing: that a pure function of the carriers
/// agrees with the oracle's spelling of it, over whatever populations the
/// drivers supply. Each genre below names a contract that statement leaves
/// unbound, so a body in that genre would lose assert strength by
/// migrating. Every genre is inhabited — an empty genre is a dead category,
/// dissolved rather than carried — and a citation classified into none of
/// them fails the tiling pin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BespokeGenre {
    /// Whole populations co-evolved step by step, asserted after each step.
    ///
    /// The contract is over the *trace*, not over any one call: every live
    /// value agrees after every operation, which no per-call descriptor
    /// quantifies. These bodies are already the unified drivers for the
    /// stateful vocabulary.
    TraceLockstep,
    /// A fallible operation whose contract covers the outcome arm, the
    /// post-state of every operand, and the identity of what is handed
    /// back.
    ///
    /// Hand-back is value identity over the *inputs* — the accumulator and
    /// the refused items, unchanged and in order — which is a statement
    /// about operands, not about the geometry a descriptor's result
    /// carries. A verdict-only binding would read as coverage while the
    /// contract stayed unbound.
    FallibleHandBack,
    /// Every owned and borrowed operand form of one operator or comparison,
    /// swept cell by cell.
    ///
    /// The property is about Rust's impl selection and delegation, not
    /// about semantics: each cell must resolve to its own impl and agree
    /// with the one source of truth. A descriptor spells one cell.
    OperandFormMatrix,
    /// A function-space realization: the recursive oracle's operation
    /// lifted into the function space and compared with the combinator
    /// there.
    ///
    /// These bodies are the function-space model's own soundness suite. A
    /// body that binds the function-space leg and the recursive-oracle leg
    /// in one walk is bespoke on both, since splitting the walk would
    /// re-derive one leg from the other.
    FunctionSpaceRealization,
    /// An n-ary fold against the reference's fold over the same family.
    ///
    /// The contract is quantified over arity and feed order, so the
    /// population is a family rather than a fixed tuple of carriers.
    NAryFold,
    /// A pointwise pure operation still bound by a hand-written body per
    /// population.
    ///
    /// This is the shape the descriptor table's drivers derive, so a
    /// citation here records coverage the table has not absorbed. The
    /// inhabitation pin dissolves this genre when the last entry leaves it.
    HandWrittenPointwise,
}

impl BespokeGenre {
    /// Every genre name, for the inhabitation census; the exhaustive match
    /// in [`name`](BespokeGenre::name) beside this list keeps the two in
    /// one diff when the vocabulary changes.
    pub(crate) const GENRES: &'static [&'static str] = &[
        "TraceLockstep",
        "FallibleHandBack",
        "OperandFormMatrix",
        "FunctionSpaceRealization",
        "NAryFold",
        "HandWrittenPointwise",
    ];

    /// The genre's name, as [`GENRES`](BespokeGenre::GENRES) spells it.
    pub(crate) fn name(self) -> &'static str {
        match self {
            BespokeGenre::TraceLockstep => "TraceLockstep",
            BespokeGenre::FallibleHandBack => "FallibleHandBack",
            BespokeGenre::OperandFormMatrix => "OperandFormMatrix",
            BespokeGenre::FunctionSpaceRealization => "FunctionSpaceRealization",
            BespokeGenre::NAryFold => "NAryFold",
            BespokeGenre::HandWrittenPointwise => "HandWrittenPointwise",
        }
    }
}

/// The bespoke half of the tiling: every `Bound` citation in
/// [`crate::surface`] this table does not derive, with the genre excusing
/// it.
///
/// Held equal, both directions, to the roster's `Bound` citations minus the
/// derived ones: a new hand-written differential cited by a row fails the
/// tiling pin until it is classified, and an entry naming a citation no row
/// makes is a phantom that fails the same pin.
pub(crate) const DIFF_BESPOKE: &[(&str, BespokeGenre)] = &[
    (
        "clock_observers_match_oracle",
        BespokeGenre::HandWrittenPointwise,
    ),
    (
        "compare_matrix_matches_oracle",
        BespokeGenre::OperandFormMatrix,
    ),
    ("covers_arbitrary", BespokeGenre::HandWrittenPointwise),
    (
        "covers_realizes_containment",
        BespokeGenre::FunctionSpaceRealization,
    ),
    ("d_fork_join_roundtrip", BespokeGenre::FallibleHandBack),
    ("disjoint_arbitrary", BespokeGenre::HandWrittenPointwise),
    (
        "distance_and_lag_realize_both_oracles",
        BespokeGenre::FunctionSpaceRealization,
    ),
    ("div_matches_oracle", BespokeGenre::HandWrittenPointwise),
    (
        "event_dominates_local_and_advances",
        BespokeGenre::FunctionSpaceRealization,
    ),
    ("fork_partitions", BespokeGenre::FunctionSpaceRealization),
    ("heterogeneous_joins", BespokeGenre::OperandFormMatrix),
    (
        "is_seed_matches_the_oracle",
        BespokeGenre::HandWrittenPointwise,
    ),
    (
        "join_all_matches_the_recursive_oracle",
        BespokeGenre::FallibleHandBack,
    ),
    ("master_differential", BespokeGenre::TraceLockstep),
    ("meet_all_matches_oracle", BespokeGenre::NAryFold),
    ("meet_arbitrary", BespokeGenre::HandWrittenPointwise),
    (
        "meet_realizes_pointwise_min",
        BespokeGenre::FunctionSpaceRealization,
    ),
    ("merge_arbitrary", BespokeGenre::HandWrittenPointwise),
    (
        "min_ticks_matches_oracle",
        BespokeGenre::HandWrittenPointwise,
    ),
    (
        "min_ticks_realizes_base_sum",
        BespokeGenre::FunctionSpaceRealization,
    ),
    (
        "own_version_matches_oracle",
        BespokeGenre::HandWrittenPointwise,
    ),
    (
        "quotient_realizes_region_mask",
        BespokeGenre::FunctionSpaceRealization,
    ),
    ("rank_matches_oracle", BespokeGenre::HandWrittenPointwise),
    (
        "rank_realizes_riemann_sum",
        BespokeGenre::FunctionSpaceRealization,
    ),
    (
        "replay_matches_across_references",
        BespokeGenre::TraceLockstep,
    ),
    ("sum_arbitrary", BespokeGenre::FallibleHandBack),
    (
        "sum_of_disjoint_is_union",
        BespokeGenre::FunctionSpaceRealization,
    ),
    ("sync", BespokeGenre::FallibleHandBack),
    ("tick_arbitrary", BespokeGenre::HandWrittenPointwise),
    ("ticks_matches_oracle", BespokeGenre::HandWrittenPointwise),
    (
        "view_cmp_matches_oracle_composed",
        BespokeGenre::HandWrittenPointwise,
    ),
    (
        "view_pair_cmp_matches_oracle_composed",
        BespokeGenre::HandWrittenPointwise,
    ),
    ("without_arbitrary", BespokeGenre::HandWrittenPointwise),
    (
        "without_realizes_region_difference",
        BespokeGenre::FunctionSpaceRealization,
    ),
];

/// Expands to every registered descriptor group: its static, the driver
/// name the arbitrary-population consumer gives it, and its input
/// signature.
///
/// Consumers take an optional argument clause (`consumer(args)`) ahead of
/// the list as `args: (...)`, exactly as the law-group roster does. The
/// signature kinds name the carrier each input borrows: `version`, `party`,
/// `clock`, and `ticks` for the tick count.
macro_rules! for_each_diff_group {
    ($callback:ident) => { for_each_diff_group!($callback()); };
    ($callback:ident($($args:tt)*)) => {
        $callback! {
            args: ($($args)*);
        }
    };
}

/// Emits the registration surface from the group roster.
///
/// The name chain and the group list expand from `for_each_diff_group!`'s
/// single spelling, so neither can drift from the other or from what the
/// derived drivers execute.
macro_rules! emit_diff_registration {
    (args: (); $(($group:ident, $driver:ident, $shape:tt)),* $(,)?) => {
        /// Every registered descriptor name, across all groups.
        ///
        /// Read from the tables the drivers run, so anything that resolves
        /// descriptor names — the tiling pin, the coverage roster's
        /// citation check — resolves against what actually executes rather
        /// than against a text scan a stray same-named item could satisfy.
        pub(crate) fn registered_names() -> Vec<&'static str> {
            std::iter::empty()
                $(.chain($group.iter().map(|(name, _)| *name)))*
                .collect()
        }

        /// Every group static the roster carries, by name — the same list,
        /// stringified, for the totality pin against the `pub(crate)
        /// static` declarations in this file.
        #[cfg(test)]
        pub(crate) const REGISTERED_GROUPS: &[&str] = &[$(stringify!($group)),*];
    };
}

for_each_diff_group!(emit_diff_registration);

#[cfg(test)]
mod tests;
