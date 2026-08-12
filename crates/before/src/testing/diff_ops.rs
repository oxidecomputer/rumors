//! The pointwise differential table: one descriptor per public operation
//! whose reference legs are pure functions of version, id, and clock values.
//!
//! A *descriptor* names an operation once and spells it per reference —
//! once on production, once on the recursive oracle, and (where the
//! operation is deterministic in the function space) once as a
//! function-space combinator — and the drivers in this
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
//! prod and tree spellings are mandatory; the fs spelling is per-descriptor,
//! because it exists only where the function-space realization is
//! deterministic — the under-determined operations (`fork`, `event`) draw
//! random §4-valid policies there, so equality against them is unassertable
//! and their fs legs stay bespoke as the model's policy-soundness suite. A
//! descriptor's fs spelling binds the tree↔fs leg directly (the oracle
//! spelling sits beside it); the prod↔fs leg then rides transitively
//! through the same descriptor's prod↔tree comparison. The
//! operations the table does not cover keep their hand-written bodies and are
//! rostered in [`DIFF_BESPOKE`] under a [`BespokeGenre`], so "bespoke" is a
//! closed status a reviewer diffs rather than the default anything falls
//! into. The tiling pin in this module's tests holds every `Bound` citation
//! in [`crate::surface`] to exactly one side: derived from this table, or
//! bespoke with a declared genre — never both, never neither.

use std::cmp::Ordering;

use crate::testing::{bridge, semantic_oracle};
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

/// How a function-space result is compared with the recursive oracle's, at
/// a comparison grid.
///
/// The fs-column counterpart of [`Matches`], implemented once per result
/// type an fs spelling can produce, so a descriptor never spells its own
/// comparison here either. Function-shaped results ([`semantic_oracle::Event`],
/// [`semantic_oracle::Id`]) compare by scanning both functions over the
/// grid; the reference tree is lifted for the scan, and its own depth is
/// folded into the grid so a reference deeper than the operands (which no
/// pointwise combinator produces, but the comparison does not assume that)
/// is still resolved exactly. Scalar and verdict results compare directly,
/// the grid unread.
pub(crate) trait FsMatches<Reference> {
    /// Whether this function-space result agrees with the oracle's, scanned
    /// at (at least) `grid`.
    fn fs_matches(&self, reference: &Reference, grid: u32) -> bool;
}

impl FsMatches<oracle::Version> for semantic_oracle::Event {
    fn fs_matches(&self, reference: &oracle::Version, grid: u32) -> bool {
        let g = grid.max(semantic_oracle::fs_grid(&[semantic_oracle::ev_depth(
            reference,
        )]));
        semantic_oracle::ev_order(self, &semantic_oracle::lift_ev(reference.clone()), g)
            == Some(Ordering::Equal)
    }
}

impl FsMatches<oracle::Party> for semantic_oracle::Id {
    fn fs_matches(&self, reference: &oracle::Party, grid: u32) -> bool {
        let g = grid.max(semantic_oracle::fs_grid(&[semantic_oracle::id_depth(
            reference,
        )]));
        semantic_oracle::id_order(self, &semantic_oracle::lift_id(reference.clone()), g)
            == Some(Ordering::Equal)
    }
}

impl FsMatches<bool> for bool {
    fn fs_matches(&self, reference: &bool, _grid: u32) -> bool {
        self == reference
    }
}

impl FsMatches<Ticks> for Ticks {
    fn fs_matches(&self, reference: &Ticks, _grid: u32) -> bool {
        self == reference
    }
}

impl FsMatches<Rank> for Rank {
    fn fs_matches(&self, reference: &Rank, _grid: u32) -> bool {
        self == reference
    }
}

/// Declares one descriptor group: the group's `pub(crate) static` slice and
/// every descriptor in it, from a single spelling.
///
/// The header names the group and the input signature every descriptor in
/// the block shares, each input tagged with the carrier it borrows and the
/// population regime the drivers owe it (`version`, `party`,
/// `disjoint_party`, `clock`, `ticks`). Each `fn` that follows is one
/// descriptor: it names the operation and spells it per reference —
/// `prod:` against the production types, `tree:` against the recursive
/// oracle's, and optionally `fs(g):` against the function-space
/// combinators — and the macro registers it in the slice under its own
/// name (`stringify!`ed): a descriptor cannot be written without being
/// registered, nor registered under a name that is not its own.
///
/// The spellings are written in one place, side by side, over identically
/// named bindings: what the transcription centralizes, it also makes
/// diffable. Each side gets its own scope holding its own values —
/// production values raised from the oracle carriers through the bridge,
/// oracle values cloned, function-space values lifted through the
/// embedding — so every side may consume or mutate freely, and `!Clone`
/// production types are simply rebuilt per descriptor. Whether the results
/// agree is the result types' business, through [`Matches`] for the tree
/// comparison and [`FsMatches`] for the fs one; the descriptor states only
/// the spellings.
///
/// The fs spelling's header names one extra binding, `fs(g):` — the
/// comparison grid ([`semantic_oracle::fs_grid`] over the operands'
/// structural depths), in scope for spellings whose combinator scans
/// (`id_order`, `disjoint`, `min_ticks`, `rank`); a spelling that never
/// scans names it with an underscore. The fs result is compared against
/// the *tree* spelling's result at that grid, so the fs column binds the
/// tree↔fs leg, and the prod↔fs leg rides the same descriptor
/// transitively.
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
            fn $op:ident { $($body:tt)* }
        )+
    ) => {
        $(#[$group_meta])* pub(crate) static $group: &[$crate::testing::diff_ops::DiffOp<
            fn($(&diff_ops!(@oracle $kind)),+) -> bool,
        >] = &[$((stringify!($op), $op)),+];
        diff_ops! {
            @ops ($($param: $kind),+);
            $(
                $(#[$op_meta])*
                fn $op { $($body)* }
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
        fn $op:ident { $($body:tt)* }
        $($rest:tt)*
    ) => {
        diff_ops! { @op ($($signature)+); $(#[$op_meta])* fn $op { $($body)* } }
        diff_ops! { @ops ($($signature)+); $($rest)* }
    };
    (
        @op ($($param:ident: $kind:tt),+ $(,)?);
        $(#[$op_meta:meta])*
        fn $op:ident { prod: $prod:expr, tree: $tree:expr $(,)? }
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
    (
        @op ($($param:ident: $kind:tt),+ $(,)?);
        $(#[$op_meta:meta])*
        fn $op:ident { prod: $prod:expr, tree: $tree:expr, fs($grid:ident): $fs:expr $(,)? }
    ) => {
        $(#[$op_meta])*
        // The bindings are uniformly mutable so a descriptor may spell an
        // operation that mutates its receiver; most do not.
        #[allow(unused_mut)]
        fn $op($($param: &diff_ops!(@oracle $kind)),+) -> bool {
            let prod = { $( let mut $param = diff_ops!(@lower $kind, $param); )+ $prod };
            let tree = { $( let mut $param = ::core::clone::Clone::clone($param); )+ $tree };
            // The grid derives from the oracle carriers before the fs
            // scope shadows them, so a spelling can never scan a lifted
            // value coarser than the value's own boundaries.
            let $grid = $crate::testing::semantic_oracle::fs_grid(&[
                $(diff_ops!(@depth $kind, $param)),+
            ]);
            let fs = { $( let mut $param = diff_ops!(@fslift $kind, $param); )+ $fs };
            $crate::testing::diff_ops::Matches::matches(&prod, &tree)
                && $crate::testing::diff_ops::FsMatches::fs_matches(&fs, &tree, $grid)
        }
    };

    // The carrier tags: each names the oracle-side type a driver supplies,
    // the production value raised from it, the function-space value lifted
    // from it, and the structural depth its fs comparison grid folds in.
    (@oracle version) => { $crate::oracle::Version };
    (@oracle party) => { $crate::oracle::Party };
    (@oracle disjoint_party) => { $crate::oracle::Party };
    (@oracle clock) => { $crate::oracle::Clock };
    (@oracle ticks) => { $crate::Ticks };
    (@lower version, $carrier:expr) => { $crate::testing::bridge::from_oracle_version($carrier) };
    (@lower party, $carrier:expr) => { $crate::testing::bridge::from_oracle_party($carrier) };
    (@lower disjoint_party, $carrier:expr) => { $crate::testing::bridge::from_oracle_party($carrier) };
    (@lower clock, $carrier:expr) => { $crate::testing::bridge::from_oracle_clock($carrier) };
    (@lower ticks, $carrier:expr) => { ::core::clone::Clone::clone($carrier) };
    (@fslift version, $carrier:expr) => {
        $crate::testing::semantic_oracle::lift_ev(::core::clone::Clone::clone($carrier))
    };
    (@fslift party, $carrier:expr) => {
        $crate::testing::semantic_oracle::lift_id(::core::clone::Clone::clone($carrier))
    };
    (@fslift disjoint_party, $carrier:expr) => {
        $crate::testing::semantic_oracle::lift_id(::core::clone::Clone::clone($carrier))
    };
    (@fslift clock, $carrier:expr) => {
        $crate::testing::semantic_oracle::FunctionClock {
            id: $crate::testing::semantic_oracle::lift_id(::core::clone::Clone::clone(
                $carrier.party(),
            )),
            ev: $crate::testing::semantic_oracle::lift_ev($carrier.version()),
        }
    };
    (@fslift ticks, $carrier:expr) => { ::core::clone::Clone::clone($carrier) };
    (@depth version, $carrier:expr) => { $crate::testing::semantic_oracle::ev_depth($carrier) };
    (@depth party, $carrier:expr) => { $crate::testing::semantic_oracle::id_depth($carrier) };
    (@depth disjoint_party, $carrier:expr) => {
        $crate::testing::semantic_oracle::id_depth($carrier)
    };
    (@depth clock, $carrier:expr) => {
        $crate::testing::semantic_oracle::id_depth($carrier.party())
            .max($crate::testing::semantic_oracle::ev_depth(&$carrier.version()))
    };
    (@depth ticks, $carrier:expr) => { 0u32 };
}

diff_ops! {
    /// The operations that act on a history *through* a region.
    ///
    /// Both read the id as a mask over the history: one inflates inside
    /// it, the other keeps only what lies inside it. Feeding an id whose
    /// shape is unrelated to the history's is what drives the full-subtree
    /// arms and the multi-region cost comparison that a clock's own pair
    /// under-hits.
    pub(crate) static VERSION_PARTY: (a: version, p: party);

    /// `tick`: register one event, inflating the region the id owns.
    fn version_tick_matches_the_oracle {
        prod: { a.tick(&p); a },
        tree: { a.tick(&p); a },
    }

    /// The projection (`/`): the history masked to the id's region.
    ///
    /// Production answers lazily and materializes on demand, so the
    /// production leg spells the materialization the oracle's projection
    /// returns directly. The function space realizes it as the pointwise
    /// mask — keep the value where the id owns the region, zero it
    /// everywhere else — the shares-no-recursion witness that projection
    /// masks exactly the owned region.
    fn version_projection_matches_the_oracle {
        prod: (&a / &p).to_version(),
        tree: a / &p,
        fs(_g): semantic_oracle::project(a, p),
    }
}

diff_ops! {
    /// The fused repeated tick.
    pub(crate) static VERSION_PARTY_TICKS: (a: version, p: party, n: ticks);

    /// `ticks(n)`: production's fused advance against the oracle's
    /// literally iterated one.
    fn version_ticks_matches_the_oracle {
        prod: { a.ticks(&p, n); a },
        tree: { a.ticks(&p, n); a },
    }
}

diff_ops! {
    /// The fused three-stream comparison: a projected view against a whole
    /// history.
    ///
    /// Production walks the projection and the comparison together, never
    /// materializing; the oracle materializes and then compares. The
    /// descriptor is that seam.
    pub(crate) static VERSION_PARTY_VERSION: (a: version, p: party, b: version);

    /// `(a / p) ⋚ b`: the fused walk against materialize-then-compare.
    fn own_version_cmp_matches_the_oracle {
        prod: (&a / &p).partial_cmp(&b),
        tree: (a / &p).partial_cmp(&b),
    }
}

diff_ops! {
    /// The fused four-stream comparison: two projected views, each through
    /// its own region.
    pub(crate) static VERSION_PARTY_VERSION_PARTY: (a: version, p: party, b: version, q: party);

    /// `(a / p) ⋚ (b / q)`: the four-stream co-walk against
    /// materialize-then-compare.
    fn own_version_pair_cmp_matches_the_oracle {
        prod: (&a / &p).partial_cmp(&(&b / &q)),
        tree: (a / &p).partial_cmp(&(b / &q)),
    }
}

diff_ops! {
    /// The clock's view of its own history.
    pub(crate) static CLOCK_SOLO: (c: clock);

    /// `own_version`: the clock's history inside the region its own id
    /// holds.
    ///
    /// Production answers lazily, the oracle answers by projection, and
    /// the function space answers as the pointwise mask of the clock's
    /// step function by its own characteristic function.
    fn clock_own_version_matches_the_oracle {
        prod: c.own_version().to_version(),
        tree: c.own_version(),
        fs(_g): semantic_oracle::project(c.ev, c.id),
    }
}

diff_ops! {
    /// The fullness predicate over one id.
    pub(crate) static PARTY_SOLO: (a: party);

    /// `is_seed`: production's constant-time test against the oracle's
    /// notion of the full region, which in normal form is exactly the
    /// seed.
    fn party_is_seed_matches_the_oracle {
        prod: a.is_seed(),
        tree: a == oracle::Party::seed(),
    }
}

diff_ops! {
    /// The region algebra over a pair of ids.
    ///
    /// The arbitrary population admits the anonymous id and pairs that
    /// genuinely overlap, so the partial-overlap arms — neither region
    /// covering the other, a difference that empties — are reachable at
    /// all; a seed-derived pipeline produces only disjoint siblings. The
    /// organic pairings run in both operand orders, which is what the
    /// asymmetric operations need.
    pub(crate) static PARTY_PAIR: (a: party, b: party);

    /// `covers`: one region contains the other.
    ///
    /// Geometrically, every point `b` owns is owned by `a` too, which the
    /// function space's containment order reports as `Less`/`Equal` (an
    /// ancestor reads as `Less`), the partial-overlap `None` arm
    /// included.
    fn party_covers_matches_the_oracle {
        prod: a.covers(&b),
        tree: a.covers(&b),
        fs(g): matches!(
            semantic_oracle::id_order(&a, &b, g),
            Some(Ordering::Less | Ordering::Equal)
        ),
    }

    /// `is_disjoint`: the two regions share nothing — geometrically, no
    /// grid point owned by both.
    ///
    /// This population is where the fs leg's `false` arm lives: the
    /// replay's single-seed populations keep every live pair disjoint, so
    /// only the arbitrary overlapping pairs here drive the geometric scan
    /// to a shared point.
    fn party_disjointness_matches_the_oracle {
        prod: a.is_disjoint(&b),
        tree: a.is_disjoint(&b),
        fs(g): semantic_oracle::disjoint(&a, &b, g),
    }

    /// `without`: the region difference, which production answers as
    /// `None` where the oracle answers with the empty region — and the
    /// function space as the pointwise mask `a ∧ ¬b`, the all-`false`
    /// function when `b` covers `a`.
    fn party_without_matches_the_oracle {
        prod: a.without(&b),
        tree: a.without(&b),
        fs(_g): semantic_oracle::diff(a, b),
    }
}

diff_ops! {
    /// The scalar quantities one history carries.
    ///
    /// Both are folds over the whole tree, so the regime that matters is
    /// depth and base magnitude rather than the relationship between two
    /// values.
    pub(crate) static VERSION_SOLO: (a: version);

    /// `min_ticks`: the events every region of the history has seen.
    ///
    /// The function space recovers it by pulling per-node floors up the
    /// dyadic subdivision — the geometric mirror of tree normalization,
    /// sharing no code with either fold.
    fn version_min_ticks_matches_the_oracle {
        prod: a.min_ticks(),
        tree: a.min_ticks(),
        fs(g): crate::Ticks(semantic_oracle::min_ticks(&a, g)),
    }

    /// `rank`: the area the history covers, against the oracle's fold and
    /// the function space's plain Riemann sum over the resolving grid.
    ///
    /// The Riemann sum has no recursion, no per-node bases, and no
    /// normalization sink, so a formula bug the two tree folds shared
    /// could not hide here.
    fn version_rank_matches_the_oracle {
        prod: a.rank(),
        tree: a.rank(),
        fs(g): semantic_oracle::rank(&a, g),
    }
}

diff_ops! {
    /// The lattice and the order over a pair of histories.
    ///
    /// The regime the arbitrary population reaches is the *unrelated*
    /// pair — independent shapes and independent base magnitudes, where
    /// arm selection and the normalization corners live; the organic
    /// populations supply the causally related pairs, where domination and
    /// equality actually occur.
    pub(crate) static VERSION_PAIR: (a: version, b: version);

    /// The join (`|`): the least upper bound of two histories.
    fn version_join_matches_the_oracle {
        prod: a | b,
        tree: a | b,
    }

    /// The meet (`&`): the greatest lower bound of two histories, realized
    /// in the function space as the pointwise minimum.
    ///
    /// The fs leg is the explicit, shares-no-recursion witness that the
    /// tree recursion computes the true GLB — the dual of the pointwise
    /// maximum the keystone replay exercises through `join`/`send`.
    fn version_meet_matches_the_oracle {
        prod: a & b,
        tree: a & b,
        fs(_g): semantic_oracle::meet(a, b),
    }

    /// The causal order's verdict, the concurrent `None` arm included.
    fn version_order_matches_the_oracle {
        prod: a.partial_cmp(&b),
        tree: a.partial_cmp(&b),
    }

    /// Concurrency: production's predicate against incomparability under
    /// the oracle's order, which is what concurrency means in the model.
    fn version_concurrency_matches_the_oracle {
        prod: a.concurrent(&b),
        tree: a.partial_cmp(&b).is_none(),
    }

    /// `distance`: the causal area between two histories, the rank of
    /// their join less the rank of their meet.
    ///
    /// Production computes it in one fused sweep; the tree leg pins the
    /// arithmetic with rank differences over the oracle's own join and
    /// meet, and the fs leg pins the meaning with Riemann sums over the
    /// function space's — three computations sharing no walk, no
    /// accumulator, and no normalization sink.
    fn version_distance_matches_the_oracle {
        prod: a.distance(&b),
        tree: {
            let met = (a.clone() & b.clone()).rank();
            (a | b)
                .rank()
                .checked_sub(&met)
                .expect("the join dominates the meet")
        },
        fs(g): {
            let met = semantic_oracle::rank(&semantic_oracle::meet(a.clone(), b.clone()), g);
            semantic_oracle::rank(&semantic_oracle::join(a, b), g)
                .checked_sub(&met)
                .expect("the join dominates the meet")
        },
    }

    /// `lag`: how far `a` lags behind `b` — the rank of the history `b`
    /// records that `a` does not, the join's rank less `a`'s own.
    ///
    /// The directed half of `distance`, pinned the same two independent
    /// ways.
    fn version_lag_matches_the_oracle {
        prod: a.lag(&b),
        tree: {
            let own = a.rank();
            (a | b)
                .rank()
                .checked_sub(&own)
                .expect("the join dominates its operand")
        },
        fs(g): {
            let own = semantic_oracle::rank(&a, g);
            semantic_oracle::rank(&semantic_oracle::join(a, b), g)
                .checked_sub(&own)
                .expect("the join dominates its operand")
        },
    }
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
    ];

    /// The genre's name, as [`GENRES`](BespokeGenre::GENRES) spells it.
    pub(crate) fn name(self) -> &'static str {
        match self {
            BespokeGenre::TraceLockstep => "TraceLockstep",
            BespokeGenre::FallibleHandBack => "FallibleHandBack",
            BespokeGenre::OperandFormMatrix => "OperandFormMatrix",
            BespokeGenre::FunctionSpaceRealization => "FunctionSpaceRealization",
            BespokeGenre::NAryFold => "NAryFold",
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
    ("d_fork_join_roundtrip", BespokeGenre::FallibleHandBack),
    (
        "event_dominates_local_and_advances",
        BespokeGenre::FunctionSpaceRealization,
    ),
    ("fork_partitions", BespokeGenre::FunctionSpaceRealization),
    ("heterogeneous_joins", BespokeGenre::OperandFormMatrix),
    (
        "join_all_matches_the_recursive_oracle",
        BespokeGenre::FallibleHandBack,
    ),
    ("master_differential", BespokeGenre::TraceLockstep),
    ("meet_all_matches_oracle", BespokeGenre::NAryFold),
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
            (VERSION_SOLO, version_solo_ops, (version)),
            (VERSION_PAIR, version_pair_ops, (version, version)),
            (VERSION_PARTY, version_party_ops, (version, party)),
            (VERSION_PARTY_TICKS, version_party_ticks_ops, (version, party, ticks)),
            (PARTY_SOLO, party_solo_ops, (party)),
            (PARTY_PAIR, party_pair_ops, (party, party)),
            (CLOCK_SOLO, clock_solo_ops, (clock)),
            (
                VERSION_PARTY_VERSION,
                version_party_version_ops,
                (version, party, version)
            ),
            (
                VERSION_PARTY_VERSION_PARTY,
                version_party_version_party_ops,
                (version, party, version, party)
            ),
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
