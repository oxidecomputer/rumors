//! Pointwise differential checks across three implementations.
//!
//! Each descriptor defines the same deterministic operation for production,
//! the recursive oracle, and the function-space oracle. Shared drivers apply
//! every descriptor to two complementary populations: arbitrary canonical
//! values and values produced by realistic clock traces.
//!
//! Descriptors with the same operand types form a group. The declaration macro
//! registers every descriptor it defines, and `for_each_diff_group!` supplies
//! every group to both drivers. Adding a descriptor to an existing group needs
//! no test wiring. A new operand shape must be handled by both drivers before
//! it compiles; an unlisted private group is rejected as dead code.
//!
//! # Why a table
//!
//! Without the table, each population needs a separate test body for every
//! operation. Those copies can omit an operation or disagree about what it
//! means. Here the drivers own the populations and the descriptors own the
//! operations, so every registered operation meets every population.
//!
//! Centralizing an operation also centralizes any transcription mistake.
//! Focused tests therefore include deliberately wrong descriptors and prove
//! that the comparisons reject them.
//!
//! # The boundary
//!
//! The table covers deterministic, value-returning operations shared by all
//! three implementations. Stateful and under-determined operations such as
//! `fork` and `tick` belong to the trace differential, which compares their
//! contractual observations rather than demanding identical representations.
//! Each descriptor compares production with the recursive oracle, then the
//! recursive oracle with the function-space model. Agreement is therefore
//! checked across all three implementations.

use std::cmp::Ordering;

use crate::testing::oracles::{function, tree};
use crate::testing::{bridge, shape_rows};
use crate::{Party, Rank, Ticks, Version};
use num_bigint::BigUint;

/// One descriptor: its name and the check the drivers run.
///
/// The same shape as [`crate::testing::laws::Law`], and for the same reason: an
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

impl Matches<tree::Version> for Version {
    fn matches(&self, reference: &tree::Version) -> bool {
        *self == bridge::from_oracle_version(reference)
            && bridge::to_oracle_version(self) == *reference
    }
}

impl Matches<tree::Party> for Option<Party> {
    /// The oracle carries the empty region as a value where production
    /// carries it as `None`, so the arms are matched before the trees.
    fn matches(&self, reference: &tree::Party) -> bool {
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

impl Matches<tree::Party> for Party {
    /// A production party where the population guarantees nonempty ownership.
    fn matches(&self, reference: &tree::Party) -> bool {
        !reference.is_empty()
            && *self == bridge::from_oracle_party(reference)
            && bridge::to_oracle_party(self) == *reference
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

/// Shape rows — plateau, region, overlay, and refinement-cell listings —
/// compare directly: both sides are already folded into the same
/// absolute vocabulary.
impl Matches<Vec<(BigUint, u64)>> for Vec<(BigUint, u64)> {
    fn matches(&self, reference: &Vec<(BigUint, u64)>) -> bool {
        self == reference
    }
}

impl Matches<Vec<(bool, u64)>> for Vec<(bool, u64)> {
    fn matches(&self, reference: &Vec<(bool, u64)>) -> bool {
        self == reference
    }
}

impl Matches<Vec<(u64, BigUint, bool)>> for Vec<(u64, BigUint, bool)> {
    fn matches(&self, reference: &Vec<(u64, BigUint, bool)>) -> bool {
        self == reference
    }
}

impl Matches<Vec<(u64, Vec<BigUint>)>> for Vec<(u64, Vec<BigUint>)> {
    fn matches(&self, reference: &Vec<(u64, Vec<BigUint>)>) -> bool {
        self == reference
    }
}

/// How a function-space result is compared with the recursive oracle's, at
/// a comparison grid.
///
/// The function-space counterpart of [`Matches`], implemented once per result
/// type a function-space spelling can produce. Function-shaped results
/// ([`function::Event`],
/// [`function::Id`]) compare by scanning both functions over the
/// grid; the reference tree is lifted for the scan, and its own depth is
/// folded into the grid so a reference deeper than the operands (which no
/// pointwise combinator produces, but the comparison does not assume that)
/// is still resolved exactly. Scalar and verdict results compare directly,
/// the grid unread.
pub(crate) trait FunctionMatches<Reference> {
    /// Whether this function-space result agrees with the oracle's, scanned
    /// at (at least) `grid`.
    fn function_matches(&self, reference: &Reference, grid: u32) -> bool;
}

impl FunctionMatches<tree::Version> for function::Event {
    fn function_matches(&self, reference: &tree::Version, grid: u32) -> bool {
        let g = grid.max(function::fs_grid(&[
            self.res_ceiling(),
            function::ev_depth(reference),
        ]));
        function::ev_order(self, &function::lift_ev(reference.clone()), g) == Some(Ordering::Equal)
    }
}

impl FunctionMatches<tree::Party> for function::Id {
    fn function_matches(&self, reference: &tree::Party, grid: u32) -> bool {
        let g = grid.max(function::fs_grid(&[
            self.res_ceiling(),
            function::id_depth(reference),
        ]));
        function::id_order(self, &function::lift_id(reference.clone()), g) == Some(Ordering::Equal)
    }
}

impl FunctionMatches<bool> for bool {
    fn function_matches(&self, reference: &bool, _grid: u32) -> bool {
        self == reference
    }
}

impl FunctionMatches<Option<Ordering>> for Option<Ordering> {
    fn function_matches(&self, reference: &Option<Ordering>, _grid: u32) -> bool {
        self == reference
    }
}

impl FunctionMatches<Ticks> for Ticks {
    fn function_matches(&self, reference: &Ticks, _grid: u32) -> bool {
        self == reference
    }
}

impl FunctionMatches<Rank> for Rank {
    fn function_matches(&self, reference: &Rank, _grid: u32) -> bool {
        self == reference
    }
}

/// Shape rows against the function space.
///
/// The rows rebuild into normal oracle trees (the reconstruction
/// collapses any refinement fragments), and the existing
/// recursive-versus-function scans decide. The rows come from the
/// descriptor's recursive spelling, whose depths the grid already covers.
impl FunctionMatches<Vec<(BigUint, u64)>> for function::Event {
    fn function_matches(&self, reference: &Vec<(BigUint, u64)>, grid: u32) -> bool {
        let tree = shape_rows::version_from_rows(reference);
        <function::Event as FunctionMatches<tree::Version>>::function_matches(self, &tree, grid)
    }
}

impl FunctionMatches<Vec<(bool, u64)>> for function::Id {
    fn function_matches(&self, reference: &Vec<(bool, u64)>, grid: u32) -> bool {
        let tree = shape_rows::party_from_rows(reference);
        <function::Id as FunctionMatches<tree::Party>>::function_matches(self, &tree, grid)
    }
}

impl FunctionMatches<Vec<(u64, BigUint, bool)>> for function::FunctionClock {
    fn function_matches(&self, reference: &Vec<(u64, BigUint, bool)>, grid: u32) -> bool {
        let heights: Vec<(BigUint, u64)> = reference
            .iter()
            .map(|(depth, height, _)| (height.clone(), *depth))
            .collect();
        let owned: Vec<(bool, u64)> = reference
            .iter()
            .map(|(depth, _, owned)| (*owned, *depth))
            .collect();
        self.ev.function_matches(&heights, grid) && self.id.function_matches(&owned, grid)
    }
}

impl FunctionMatches<Vec<(u64, Vec<BigUint>)>> for (function::Event, function::Event) {
    fn function_matches(&self, reference: &Vec<(u64, Vec<BigUint>)>, grid: u32) -> bool {
        let column = |index: usize| -> Vec<(BigUint, u64)> {
            reference
                .iter()
                .map(|(depth, heights)| (heights[index].clone(), *depth))
                .collect()
        };
        self.0.function_matches(&column(0), grid) && self.1.function_matches(&column(1), grid)
    }
}

/// Declares a group of operations with the same operand types.
///
/// The header tags each operand as a `version`, `party`, `disjoint_party`, or
/// `clock`. For every operation, the body gives three adjacent spellings:
/// `production`, `recursive`, and `function`. The macro converts the recursive-oracle
/// inputs into the other two representations, evaluates all three spellings,
/// and compares their results through [`Matches`] and [`FunctionMatches`].
/// Each spelling receives fresh values, so it may consume or mutate them.
///
/// `function(g)` also receives a grid fine enough to resolve every operand.
/// Spellings that do not scan the grid name it `_g`. The macro registers each
/// generated check in the group's private slice under the operation's Rust
/// name. The group must then appear in `for_each_diff_group!`; otherwise the
/// dead-code lint rejects it.
macro_rules! diff_ops {
    (
        $(#[$group_meta:meta])* static $group:ident: ($($param:ident: $kind:tt),+ $(,)?);
        $(
            $(#[$op_meta:meta])*
            fn $op:ident { $($body:tt)* }
        )+
    ) => {
        $(#[$group_meta])* static $group: &[$crate::testing::diff_ops::DiffOp<
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
        fn $op:ident {
            production: $production:expr,
            recursive: $recursive:expr,
            function($grid:ident): $function:expr $(,)?
        }
    ) => {
        $(#[$op_meta])*
        // The bindings are uniformly mutable so a descriptor may spell an
        // operation that mutates its receiver; most do not.
        #[allow(unused_mut)]
        fn $op($($param: &diff_ops!(@oracle $kind)),+) -> bool {
            let production = {
                $( let mut $param = diff_ops!(@lower $kind, $param); )+
                $production
            };
            let recursive = {
                $( let mut $param = ::core::clone::Clone::clone($param); )+
                $recursive
            };
            // The grid derives from the oracle carriers before the function
            // scope shadows them, so a spelling can never scan a lifted
            // value coarser than the value's own boundaries.
            let $grid = $crate::testing::oracles::function::fs_grid(&[
                $(diff_ops!(@depth $kind, $param)),+
            ]);
            let function_result = {
                $( let mut $param = diff_ops!(@fslift $kind, $param); )+
                $function
            };
            $crate::testing::diff_ops::Matches::matches(&production, &recursive)
                && $crate::testing::diff_ops::FunctionMatches::function_matches(
                    &function_result,
                    &recursive,
                    $grid,
                )
        }
    };

    // The carrier tags: each names the oracle-side type a driver supplies,
    // the production value raised from it, the function-space value lifted
    // from it, and the structural depth its comparison grid folds in.
    (@oracle version) => { $crate::testing::oracles::tree::Version };
    (@oracle party) => { $crate::testing::oracles::tree::Party };
    (@oracle disjoint_party) => { $crate::testing::oracles::tree::Party };
    (@oracle clock) => { $crate::testing::oracles::tree::Clock };
    (@lower version, $carrier:expr) => { $crate::testing::bridge::from_oracle_version($carrier) };
    (@lower party, $carrier:expr) => { $crate::testing::bridge::from_oracle_party($carrier) };
    (@lower disjoint_party, $carrier:expr) => { $crate::testing::bridge::from_oracle_party($carrier) };
    (@lower clock, $carrier:expr) => { $crate::testing::bridge::from_oracle_clock($carrier) };
    (@fslift version, $carrier:expr) => {
        $crate::testing::oracles::function::lift_ev(::core::clone::Clone::clone($carrier))
    };
    (@fslift party, $carrier:expr) => {
        $crate::testing::oracles::function::lift_id(::core::clone::Clone::clone($carrier))
    };
    (@fslift disjoint_party, $carrier:expr) => {
        $crate::testing::oracles::function::lift_id(::core::clone::Clone::clone($carrier))
    };
    (@fslift clock, $carrier:expr) => {
        $crate::testing::oracles::function::FunctionClock {
            id: $crate::testing::oracles::function::lift_id(::core::clone::Clone::clone(
                $carrier.party(),
            )),
            ev: $crate::testing::oracles::function::lift_ev($carrier.version()),
        }
    };
    (@depth version, $carrier:expr) => { $crate::testing::oracles::function::ev_depth($carrier) };
    (@depth party, $carrier:expr) => { $crate::testing::oracles::function::id_depth($carrier) };
    (@depth disjoint_party, $carrier:expr) => {
        $crate::testing::oracles::function::id_depth($carrier)
    };
    (@depth clock, $carrier:expr) => {
        $crate::testing::oracles::function::id_depth($carrier.party())
            .max($crate::testing::oracles::function::ev_depth(&$carrier.version()))
    };
}

diff_ops! {
    /// Operations that view a version through a party.
    ///
    /// Feeding a party whose shape is unrelated to the version reaches
    /// projection cases that a clock's own parts rarely do.
    static VERSION_PARTY: (a: version, p: party);

    /// The projection (`/`): the version restricted to the party's region.
    ///
    /// Production answers lazily and materializes on demand, so the
    /// production leg spells the materialization the oracle's projection
    /// returns directly. The function space realizes it as the pointwise
    /// mask — keep the value where the party owns the region, zero it
    /// everywhere else — the shares-no-recursion witness that projection
    /// masks exactly the owned region.
    fn version_projection_matches_the_oracle {
        production: (&a / &p).to_version(),
        recursive: a / &p,
        function(_g): function::project(a, p),
    }
}

diff_ops! {
    /// A projected version compared with a complete version.
    ///
    /// Production walks the projection and the comparison together, never
    /// materializing; the oracle materializes and then compares. The
    /// descriptor is that boundary.
    static VERSION_PARTY_VERSION: (a: version, p: party, b: version);

    /// `(a / p) ⋚ b`: the fused walk against materialize-then-compare.
    fn own_version_cmp_matches_the_oracle {
        production: (&a / &p).partial_cmp(&b),
        recursive: (a / &p).partial_cmp(&b),
        function(g): function::ev_order(&function::project(a, p), &b, g),
    }
}

diff_ops! {
    /// The fused four-stream comparison: two projected views, each through
    /// its own region.
    static VERSION_PARTY_VERSION_PARTY: (a: version, p: party, b: version, q: party);

    /// `(a / p) ⋚ (b / q)`: the four-stream co-walk against
    /// materialize-then-compare.
    fn own_version_pair_cmp_matches_the_oracle {
        production: (&a / &p).partial_cmp(&(&b / &q)),
        recursive: (a / &p).partial_cmp(&(b / &q)),
        function(g): function::ev_order(
            &function::project(a, p),
            &function::project(b, q),
            g,
        ),
    }
}

diff_ops! {
    /// Operations over a clock by itself.
    static CLOCK_SOLO: (c: clock);

    /// `own_version`: the clock's version restricted to its own party.
    ///
    /// Production answers lazily, the oracle answers by projection, and
    /// the function space answers as the pointwise mask of the clock's
    /// step function by its own characteristic function.
    fn clock_own_version_matches_the_oracle {
        production: c.own_version().to_version(),
        recursive: c.own_version(),
        function(_g): function::project(c.ev, c.id),
    }

    /// `shape`: the clock's overlay walk.
    ///
    /// The clock's version plateaus overlaid with its party's ownership
    /// are the coarsest common refinement of the two tilings.
    ///
    /// Production folds the public overlay walk (the fold asserts
    /// nonzero rises, a nonnegative running height, and exact tiling in
    /// passing); the oracle refines its version against its party read
    /// as a 0/1 step function; the function space scans both rebuilt
    /// component functions against the geometric lifts.
    fn clock_shape_matches_the_oracle {
        production: shape_rows::fold_overlay(c.shape()),
        recursive: shape_rows::oracle_cells(vec![
            (BigUint::ZERO, c.version()),
            (BigUint::ZERO, shape_rows::party_as_steps(c.party())),
        ])
        .into_iter()
        .map(|(depth, heights)| {
            let [height, owned] = <[BigUint; 2]>::try_from(heights)
                .expect("two inputs, two heights");
            (depth, height, owned == BigUint::from(1u8))
        })
        .collect::<Vec<_>>(),
        function(_g): c,
    }
}

diff_ops! {
    /// Operations over one party.
    static PARTY_SOLO: (a: party);

    /// `is_seed`: production's constant-time test against the oracle's
    /// notion of the full region, which in normal form is exactly the
    /// seed.
    fn party_is_seed_matches_the_oracle {
        production: a.is_seed(),
        recursive: a == tree::Party::seed(),
        function(g): function::id_order(&a, &function::seed_id(), g) == Some(Ordering::Equal),
    }

    /// `shape`: the party's membership function as region items.
    ///
    /// Production folds the public walk (the fold asserts exact tiling
    /// in passing); the oracle enumerates its leaves directly; the
    /// function space scans the rows' own membership function, rebuilt
    /// as a tree, against the geometric lift. The anonymous party is in the
    /// population: its walk is the single unowned whole-interval region.
    fn party_shape_matches_the_oracle {
        production: shape_rows::fold_regions(a.shape()),
        recursive: shape_rows::oracle_regions(&a),
        function(_g): a,
    }
}

diff_ops! {
    /// Region algebra over a pair of parties.
    ///
    /// The arbitrary population admits the anonymous party and pairs that
    /// genuinely overlap, so the partial-overlap arms — neither region
    /// covering the other, a difference that empties — are reachable at
    /// all; a seed-derived pipeline produces only disjoint siblings. The
    /// organic pairings run in both operand orders, which is what the
    /// asymmetric operations need.
    static PARTY_PAIR: (a: party, b: party);

    /// `covers`: one region contains the other.
    ///
    /// Geometrically, every point `b` owns is owned by `a` too, which the
    /// function space's containment order reports as `Less`/`Equal` (an
    /// ancestor reads as `Less`), the partial-overlap `None` arm
    /// included.
    fn party_covers_matches_the_oracle {
        production: a.covers(&b),
        recursive: a.covers(&b),
        function(g): matches!(
            function::id_order(&a, &b, g),
            Some(Ordering::Less | Ordering::Equal)
        ),
    }

    /// `is_disjoint`: the two regions share nothing — geometrically, no
    /// grid point owned by both.
    ///
    /// This population is where the function-space comparison's `false` arm lives: the
    /// replay's single-seed populations keep every live pair disjoint, so
    /// only the arbitrary overlapping pairs here drive the geometric scan
    /// to a shared point.
    fn party_disjointness_matches_the_oracle {
        production: a.is_disjoint(&b),
        recursive: a.is_disjoint(&b),
        function(g): function::disjoint(&a, &b, g),
    }

    /// `without`: the region difference, which production answers as
    /// `None` where the oracle answers with the empty region — and the
    /// function space as the pointwise mask `a ∧ ¬b`, the all-`false`
    /// function when `b` covers `a`.
    fn party_without_matches_the_oracle {
        production: a.without(&b),
        recursive: a.without(&b),
        function(_g): function::diff(a, b),
    }
}

diff_ops! {
    /// The region algebra on fork-derived disjoint pairs.
    ///
    /// The arbitrary pair population overlaps too often to drive the
    /// success arm of the fallible region operations, so this population
    /// is its complement: every pair is two nonempty halves of one
    /// region, the disjoint-success arm runs on every case, and the
    /// driver asserts the premise per case rather than assuming it. The
    /// hand-back arm stays with the fallible operations' own bespoke
    /// differentials, whose contract a verdict descriptor cannot carry.
    static PARTY_DISJOINT_PAIR: (a: disjoint_party, b: disjoint_party);

    /// `join` on disjoint parties: the union of the two regions, which every
    /// reference realizes without a hand-back — the function space as the
    /// pointwise `or`.
    fn party_disjoint_join_matches_the_oracle {
        production: {
            a.join(b).expect("disjoint parties join");
            a
        },
        recursive: {
            a.join(b).expect("disjoint parties join");
            a
        },
        function(_g): function::sum(a, b),
    }
}

diff_ops! {
    /// Scalar quantities derived from one version.
    ///
    /// Both are folds over the whole tree, so the regime that matters is
    /// depth and base magnitude rather than the relationship between two
    /// values.
    static VERSION_SOLO: (a: version);

    /// `min_ticks`: the events every region of the version has seen.
    ///
    /// The function space recovers it by pulling per-node floors up the
    /// dyadic subdivision — the geometric mirror of tree normalization,
    /// sharing no code with either fold.
    fn version_min_ticks_matches_the_oracle {
        production: a.min_ticks(),
        recursive: a.min_ticks(),
        function(g): crate::Ticks(function::min_ticks(&a, g)),
    }

    /// `shape`: the version's step function as plateau items.
    ///
    /// Production folds the public walk's rises into absolute rows (the
    /// fold asserts nonzero rises, a nonnegative running height, and
    /// exact tiling in passing); the oracle enumerates its leaves
    /// directly; the function space scans the rows' own step function,
    /// rebuilt as a tree, against the geometric lift.
    fn version_shape_matches_the_oracle {
        production: shape_rows::fold_heights(a.shape()),
        recursive: shape_rows::oracle_plateaus(&a),
        function(_g): a,
    }

    /// `rank`: the area the version covers, against the oracle's fold and
    /// the function space's plain Riemann sum over the resolving grid.
    ///
    /// The Riemann sum has no recursion, no per-node bases, and no
    /// normalization sink, so a formula bug the two tree folds shared
    /// could not hide here.
    fn version_rank_matches_the_oracle {
        production: a.rank(),
        recursive: a.rank(),
        function(g): function::rank(&a, g),
    }
}

diff_ops! {
    /// Lattice, order, and distance operations over two versions.
    ///
    /// The regime the arbitrary population reaches is the *unrelated*
    /// pair — independent shapes and independent base magnitudes, where
    /// arm selection and the normalization corners live; the organic
    /// populations supply the causally related pairs, where domination and
    /// equality actually occur.
    static VERSION_PAIR: (a: version, b: version);

    /// The join (`|`): the least upper bound of two versions.
    fn version_join_matches_the_oracle {
        production: a | b,
        recursive: a | b,
        function(_g): function::join(a, b),
    }

    /// `shape::combine`: two shapes walked as the coarsest common
    /// refinement of their plateau intervals.
    ///
    /// Production folds the public combined walk's per-input rises into
    /// per-cell absolute rows (the fold asserts nonzero rises,
    /// nonnegative running heights, and exact tiling in passing); the
    /// oracle computes the refinement directly by recursion — splitting
    /// only where some input splits, which also pins the tiling as the
    /// coarsest one; the function space scans each rebuilt column
    /// against its own geometric lift.
    fn shape_combine_matches_the_oracle {
        production: shape_rows::fold_cells(crate::shape::combine([&a, &b])),
        recursive: shape_rows::oracle_cells(vec![
            (BigUint::ZERO, a),
            (BigUint::ZERO, b),
        ]),
        function(_g): (a, b),
    }

    /// The meet (`&`): the greatest lower bound of two versions, realized
    /// in the function space as the pointwise minimum.
    ///
    /// The function-space leg is the non-recursive witness that the
    /// tree recursion computes the true GLB — the dual of the pointwise
    /// maximum the replay differential exercises through `join`/`send`.
    fn version_meet_matches_the_oracle {
        production: a & b,
        recursive: a & b,
        function(_g): function::meet(a, b),
    }

    /// The causal order's verdict, the concurrent `None` arm included.
    fn version_order_matches_the_oracle {
        production: a.partial_cmp(&b),
        recursive: a.partial_cmp(&b),
        function(g): function::ev_order(&a, &b, g),
    }

    /// Concurrency: production's predicate against incomparability under
    /// the oracle's order, which is what concurrency means in the model.
    fn version_concurrency_matches_the_oracle {
        production: a.concurrent(&b),
        recursive: a.partial_cmp(&b).is_none(),
        function(g): function::ev_order(&a, &b, g).is_none(),
    }

    /// `distance`: the causal area between two versions, the rank of
    /// their join less the rank of their meet.
    ///
    /// Production computes it in one fused sweep; the tree leg pins the
    /// arithmetic with rank differences over the oracle's own join and
    /// meet, and the function-space leg pins the meaning with Riemann sums over the
    /// function space's — three computations sharing no walk, no
    /// accumulator, and no normalization sink.
    fn version_distance_matches_the_oracle {
        production: a.distance(&b),
        recursive: {
            let met = (a.clone() & b.clone()).rank();
            (a | b)
                .rank()
                .checked_sub(&met)
                .expect("the join dominates the meet")
        },
        function(g): {
            let met = function::rank(&function::meet(a.clone(), b.clone()), g);
            function::rank(&function::join(a, b), g)
                .checked_sub(&met)
                .expect("the join dominates the meet")
        },
    }

    /// `lag`: how far `a` lags behind `b` — the rank of the version `b`
    /// records that `a` does not, the join's rank less `a`'s own.
    ///
    /// The directed half of `distance`, pinned the same two independent
    /// ways.
    fn version_lag_matches_the_oracle {
        production: a.lag(&b),
        recursive: {
            let own = a.rank();
            (a | b)
                .rank()
                .checked_sub(&own)
                .expect("the join dominates its operand")
        },
        function(g): {
            let own = function::rank(&a, g);
            function::rank(&function::join(a, b), g)
                .checked_sub(&own)
                .expect("the join dominates its operand")
        },
    }
}

/// Expands to every registered descriptor group: its static, the driver
/// name the arbitrary-population consumer gives it, and its input
/// signature.
///
/// Consumers take an optional argument clause (`consumer(args)`) ahead of
/// the list as `args: (...)`, exactly as the law-group roster does. The
/// signature kinds name the carrier each input borrows: `version`, `party`,
/// or `clock`.
macro_rules! for_each_diff_group {
    ($callback:ident) => { for_each_diff_group!($callback()); };
    ($callback:ident($($args:tt)*)) => {
        $callback! {
            args: ($($args)*);
            (VERSION_SOLO, version_solo_ops, (version)),
            (VERSION_PAIR, version_pair_ops, (version, version)),
            (VERSION_PARTY, version_party_ops, (version, party)),
            (PARTY_SOLO, party_solo_ops, (party)),
            (PARTY_PAIR, party_pair_ops, (party, party)),
            (
                PARTY_DISJOINT_PAIR,
                party_disjoint_pair_ops,
                (disjoint_party, disjoint_party)
            ),
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

#[cfg(test)]
mod tests;
