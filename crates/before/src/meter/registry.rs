//! The family registry: the single source of truth for adversarial input
//! families, from which every instrument derives.
//!
//! # The invariant
//!
//! Every adversarial input family is a [`FamilyId`] variant carrying its
//! row of record ([`FamilySpec`]), and every instrument derives its
//! family axis from this roster, so a family existing outside an
//! instrument's coverage is structurally impossible — enforced by the
//! compiler wherever the compiler can reach:
//!
//! - **Construction.** The raw shape constructors are private to the
//!   [`meter`](crate::meter) module, and [`Shape`] is the one public door:
//!   its constructor table (`Shape::builder`, the exhaustive match) is
//!   the constructors' only caller outside the module's own unit tests.
//!   A constructor without a registry row is dead code the gate's
//!   warnings-denied builds reject; a [`Shape`] variant without a
//!   constructor does not compile; and no code outside the module —
//!   envelope band, bench, fuelscape panel, example, kernel unit test,
//!   or downstream crate — can mint an adversarial shape except through
//!   the registry:
//!
//!   ```compile_fail,E0603
//!   // The raw constructors are private: an unregistered shape cannot
//!   // compile outside the registry door.
//!   let _ = before::meter::cliff_comb(4, 4);
//!   ```
//!
//!   ```
//!   use before::meter::registry::Shape;
//!   // The registry door: the same comb, minted through its Shape row.
//!   let comb = Shape::CliffComb.packed2(4, 4);
//!   assert_eq!(comb.bits, 4 * (2 * 4 + 10) + 2);
//!   ```
//!
//! - **The board.** The amplification board's family axis is
//!   [`FamilyId::board`] — this roster filtered on each variant's
//!   committed [`Coverage`] answer — and its cells are the product of
//!   that axis with the operation and currency tables, so a variant
//!   answering [`Coverage::Board`] is priced on every operation its
//!   operand bundle supplies with no per-cell wiring, and a variant
//!   answering [`Coverage::EnvelopeOnly`] carries the dated reason it
//!   earns no column. The declared bundle reach (`cells`) is the
//!   committed expectation the board smoke suite holds the rendered
//!   matrix to.
//! - **The bands.** The envelope suite's flatness/adequacy bands build
//!   their operands through [`Shape`], so the band-to-family link is a
//!   compiler-checked construction site, never a name mapping held in a
//!   parallel table; each family's committed band roster is its spec's
//!   [`Bands`] answer, and a family without a band carries the dated
//!   reason instead. Bespoke instruments (adequacy tripwires with
//!   committed-bad kernels, gate pins outside the band convention) stay
//!   hand-written and reach their shapes through the same door.
//! - **The intermediate state.** A red board cell with a live task in
//!   [`BOARD_EXPECTED_REDS`](crate::meter::board::BOARD_EXPECTED_REDS) —
//!   asserted empty at acceptance — is the only legitimate intermediate
//!   state for a family under cure. Coverage by bands alone is not a
//!   state a family can occupy silently: the roster answer is a board
//!   column or a dated envelope-only ruling, nothing in between.
//!
//! # What the compiler cannot reach
//!
//! Two seams stay pinned by tests instead of types, each a deliberate,
//! named survivor:
//!
//! - **Band names.** The bands live in a separate test binary
//!   (`tests/meter.rs`), and test function names are not items the
//!   compiler can resolve across crates. The board smoke suite
//!   (`tests/amp_board_smoke.rs`) scans that suite's band-named tests
//!   and holds them equal, name for name, to the union of the specs'
//!   [`Bands`] rosters and [`AXIS_BANDS`].
//! - **Shape citation.** A [`Shape`]'s membership in some family's
//!   `shapes` row is data, not types; this module's tests hold every
//!   shape cited by at least one family, so a constructor cannot ride
//!   the registry door without a family answering for it.

#[cfg(test)]
mod tests;

use suanpan::UBig;

use super::Packed;
use crate::Version;

// ─── the construction door ───────────────────────────────────────────────────

/// One registered shape constructor: the only public door to the
/// [`meter`](crate::meter) module's adversarial generators.
///
/// Each variant names its knobs and the accessor it builds through; the
/// full construction derivation (layout, normal-form argument,
/// closed-form size, panics) lives on the private constructor behind it,
/// rendered by the internal documentation build. Every variant is cited
/// by at least one [`FamilyId`] spec (this module's tests hold it), so
/// building through this enum is building inside the registry's
/// coverage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Shape {
    /// The dense event spine `S(d)`: [`Shape::packed1`]`(d)`.
    Dense,
    /// The bigroot event `B(b, d)`, a `2^b − 1` root over `S(d)`:
    /// [`Shape::packed2`]`(b, d)`.
    Bigroot,
    /// The hugeleaf event, one leaf of value `2^b − 1`:
    /// [`Shape::packed1`]`(b)`.
    Hugeleaf,
    /// The boundary comb `C(k, n)`, `n` cliff teeth:
    /// [`Shape::packed2`]`(k, n)`.
    CliffComb,
    /// The jump comb `J(k, n)`, one low tooth then `n − 1` cliff teeth:
    /// [`Shape::packed2`]`(k, n)`.
    JumpComb,
    /// The wide-tooth comb `W(k, w, n)`, `n` teeth of width `2^w`:
    /// [`Shape::packed3`]`(k, w, n)`.
    WideToothComb,
    /// The unpaid-crossing fan `F(k, n)`, `n` cheap teeth under one
    /// stored magnitude: [`Shape::packed2`]`(k, n)`.
    CliffFan,
    /// The cancelling-prefix chain `P(k, n)`, `n` peak-to-1 drops:
    /// [`Shape::packed2`]`(k, n)`.
    CancellingChain,
    /// The harmonic spine `H(d)`, a 1-leaf at every depth:
    /// [`Shape::packed1`]`(d)`.
    Harmonic,
    /// The alternating-binary spine `A(d)`: [`Shape::packed1`]`(d)`.
    AltSpine,
    /// The scattered id `Z(e)`, `e` owned fragments at alternating
    /// depths: [`Shape::packed1`]`(e)`.
    ScatteredId,
    /// The id spine `I(d, divert)`, a unary chain of depth `d`:
    /// [`Shape::packed_flagged`]`(d, divert)`.
    IdSpine,
    /// The nested-full-sibling id `N(d)`: [`Shape::packed1`]`(d)`.
    NestedFullId,
    /// The mirror nested-full id `M(d)`: [`Shape::packed1`]`(d)`.
    NestedLeftFullId,
    /// The wide-tail event, a zero-leaf spine with one `2^b − 1` tail:
    /// [`Shape::packed2`]`(b, d)`.
    WideTail,
    /// The descending staircase `D(d)`: [`Shape::packed1`]`(d)`.
    Staircase,
    /// The memo-chain event `Q(k, distinct)`:
    /// [`Shape::packed_flagged`]`(k, distinct)`.
    MemoChain,
    /// The memo-chain id: [`Shape::packed1`]`(k)`.
    MemoChainId,
    /// The memo-comb event `B(d)`: [`Shape::packed1`]`(d)`.
    MemoComb,
    /// The memo-comb id: [`Shape::packed1`]`(d)`.
    MemoCombId,
    /// The memo fan-out event `F(k, b)`: [`Shape::packed2`]`(k, b)`.
    MemoFanout,
    /// The oscillating-siblings event `O(k, b)`:
    /// [`Shape::packed2`]`(k, b)`.
    MemoOscillating,
    /// The memo-churn event `U(d)`: [`Shape::packed1`]`(d)`.
    MemoChurn,
    /// The memo-churn id: [`Shape::packed1`]`(d)`.
    MemoChurnId,
    /// The descending-raises event `W(d)`: [`Shape::packed1`]`(d)`.
    DescendingRaises,
    /// The descending-raises id: [`Shape::packed1`]`(d)`.
    DescendingRaisesId,
    /// The reveal-comb event `R(k, b)`: [`Shape::packed2`]`(k, b)`.
    RevealComb,
    /// The reveal comb with its floor raised to `2^b − 2` (the gap
    /// control): [`Shape::packed2`]`(k, b)`.
    RevealCombHifloor,
    /// The reveal-comb id: [`Shape::packed1`]`(k)`.
    RevealCombId,
    /// The pure-comb event `L(k, b)`: [`Shape::packed2`]`(k, b)`.
    PureComb,
    /// The pure-comb id: [`Shape::packed1`]`(k)`.
    PureCombId,
    /// The ascending cliff `A(k, b)`: [`Shape::packed2`]`(k, b)`.
    AscendCliff,
    /// The ascending cliff with every wide leaf leveled (the
    /// hop-schedule control): [`Shape::packed2`]`(k, b)`.
    AscendCliffPlateau,
    /// The ascending-cliff id: [`Shape::packed1`]`(k)`.
    AscendCliffId,
    /// The freeze-position spine `FP(k)`: [`Shape::packed1`]`(k)`.
    FreezePosition,
    /// The promotion re-arm spine `PR(p)`: [`Shape::packed1`]`(p)`.
    PromotionRearm,
    /// The promotion re-arm mate `PRM(p)`, the small twin:
    /// [`Shape::packed1`]`(p)`.
    PromotionRearmMate,
    /// The dense-suffix re-arm family `DS(p, d)`:
    /// [`Shape::packed2`]`(p, d)`.
    DenseSuffix,
    /// The dense-suffix mate `DSM(p, d)`, the unit twin:
    /// [`Shape::packed2`]`(p, d)`.
    DenseSuffixMate,
    /// The wide-arming family `WA(w, d)`: [`Shape::packed2`]`(w, d)`.
    WideArming,
    /// The weight-comb family `WC(n)`: [`Shape::packed1`]`(n)`.
    WeightComb,
    /// The freeze-parade family `FZ(k)`: [`Shape::packed1`]`(k)`.
    FreezeParade,
    /// The lone-freeze spine `LF(pre, post)`:
    /// [`Shape::packed2`]`(pre, post)`.
    LoneFreeze,
    /// The tooth-tail pair `TT(g, m)`: [`Shape::packed_pair`]`(g, m)`.
    ToothTail,
    /// The puncture-product embedding `V(x, y)` over arbitrary factors:
    /// [`Shape::packed_product`]`(&x, &y)`.
    PunctureProduct,
    /// The plateau-puncture family `PP(w, d)` over its committed
    /// factors: [`Shape::packed2`]`(w, d)`.
    PlateauPuncture,
    /// The arming-train family `AT(n, w, g, alternate)`:
    /// [`Shape::packed_train`]`(n, w, g, alternate)`.
    ArmingTrain,
    /// The two-operand jump comb `JP(k, m, d)`:
    /// [`Shape::packed_pair3`]`(k, m, d)`.
    JumpPair,
    /// The concurrent pair `CP(n)`, two organically built versions:
    /// [`Shape::version_pair`]`(n)`.
    ConcurrentPair,
    /// The staggered-comb fold operand `SG(n, m, i)`:
    /// [`Shape::packed3`]`(n, m, i)`.
    StaggerComb,
    /// The staggered id `SI(n, m, i)`: [`Shape::packed3`]`(n, m, i)`.
    StaggerId,
    /// The staggered fold population, all `n` operands in bit-reversed
    /// feed order: [`Shape::population`]`(n, m)`.
    StaggerPopulation,
    /// The meet-shade population `MS(d, k)`: [`Shape::versions`]`(d, k)`.
    MeetShade,
    /// The masked-comparison correlated triple `MT(k, n)`:
    /// [`Shape::packed_triple`]`(k, n)`.
    MaskDriftTriple,
    /// The masked-comparison correlated quadruple `MQ(k, n)`:
    /// [`Shape::packed_quadruple`]`(k, n)`.
    MaskDriftQuadruple,
}

/// A registered constructor's signature class, binding one [`Shape`] to
/// the private generator behind it.
// The signatures are the generators' own; an alias per tuple shape would
// be indirection to track, not documentation.
#[allow(clippy::type_complexity)]
enum Builder {
    /// One size knob to one packed shape.
    P1(fn(usize) -> Packed),
    /// Two size knobs to one packed shape.
    P2(fn(usize, usize) -> Packed),
    /// Three size knobs to one packed shape.
    P3(fn(usize, usize, usize) -> Packed),
    /// A size knob and a variant flag to one packed shape.
    Flag(fn(usize, bool) -> Packed),
    /// The arming-train signature: three knobs and a sign schedule.
    Train(fn(usize, usize, usize, bool) -> Packed),
    /// Two arbitrary factors to one packed shape.
    Product(fn(&UBig, &UBig) -> Packed),
    /// Two size knobs to a geometrically coupled packed pair.
    Pair2(fn(usize, usize) -> (Packed, Packed)),
    /// Three size knobs to a geometrically coupled packed pair.
    Pair3(fn(usize, usize, usize) -> (Packed, Packed)),
    /// One size knob to an organically built version pair.
    VersionPair(fn(usize) -> (Version, Version)),
    /// Two size knobs to a fold population of versions.
    Versions2(fn(usize, usize) -> Vec<Version>),
    /// Two size knobs to a fold population of (versions, ids).
    Population2(fn(usize, usize) -> (Vec<Packed>, Vec<Packed>)),
    /// Two size knobs to a correlated operand triple.
    Triple2(fn(usize, usize) -> (Packed, Packed, Packed)),
    /// Two size knobs to a correlated operand quadruple.
    Quad2(fn(usize, usize) -> ((Packed, Packed), (Packed, Packed))),
}

impl Shape {
    /// The constructor table: every registered shape's private generator.
    ///
    /// This exhaustive match is the compiler's half of the registry
    /// invariant — it is the generators' only caller outside the meter
    /// module's own unit tests, so a generator absent from it is dead
    /// code the warnings-denied builds reject, and a variant without a
    /// generator does not compile.
    fn builder(self) -> Builder {
        match self {
            Shape::Dense => Builder::P1(super::dense),
            Shape::Bigroot => Builder::P2(super::bigroot),
            Shape::Hugeleaf => Builder::P1(super::hugeleaf),
            Shape::CliffComb => Builder::P2(super::cliff_comb),
            Shape::JumpComb => Builder::P2(super::jump_comb),
            Shape::WideToothComb => Builder::P3(super::wide_tooth_comb),
            Shape::CliffFan => Builder::P2(super::cliff_fan),
            Shape::CancellingChain => Builder::P2(super::cancelling_chain),
            Shape::Harmonic => Builder::P1(super::harmonic),
            Shape::AltSpine => Builder::P1(super::alt_spine),
            Shape::ScatteredId => Builder::P1(super::scattered_id),
            Shape::IdSpine => Builder::Flag(super::id_spine),
            Shape::NestedFullId => Builder::P1(super::nested_full_id),
            Shape::NestedLeftFullId => Builder::P1(super::nested_left_full_id),
            Shape::WideTail => Builder::P2(super::wide_tail),
            Shape::Staircase => Builder::P1(super::staircase),
            Shape::MemoChain => Builder::Flag(super::memo_chain),
            Shape::MemoChainId => Builder::P1(super::memo_chain_id),
            Shape::MemoComb => Builder::P1(super::memo_comb),
            Shape::MemoCombId => Builder::P1(super::memo_comb_id),
            Shape::MemoFanout => Builder::P2(super::memo_fanout),
            Shape::MemoOscillating => Builder::P2(super::memo_oscillating),
            Shape::MemoChurn => Builder::P1(super::memo_churn),
            Shape::MemoChurnId => Builder::P1(super::memo_churn_id),
            Shape::DescendingRaises => Builder::P1(super::descending_raises),
            Shape::DescendingRaisesId => Builder::P1(super::descending_raises_id),
            Shape::RevealComb => Builder::P2(super::reveal_comb),
            Shape::RevealCombHifloor => Builder::P2(super::reveal_comb_hifloor),
            Shape::RevealCombId => Builder::P1(super::reveal_comb_id),
            Shape::PureComb => Builder::P2(super::pure_comb),
            Shape::PureCombId => Builder::P1(super::pure_comb_id),
            Shape::AscendCliff => Builder::P2(super::ascend_cliff),
            Shape::AscendCliffPlateau => Builder::P2(super::ascend_cliff_plateau),
            Shape::AscendCliffId => Builder::P1(super::ascend_cliff_id),
            Shape::FreezePosition => Builder::P1(super::freeze_position),
            Shape::PromotionRearm => Builder::P1(super::promotion_rearm),
            Shape::PromotionRearmMate => Builder::P1(super::promotion_rearm_mate),
            Shape::DenseSuffix => Builder::P2(super::dense_suffix),
            Shape::DenseSuffixMate => Builder::P2(super::dense_suffix_mate),
            Shape::WideArming => Builder::P2(super::wide_arming),
            Shape::WeightComb => Builder::P1(super::weight_comb),
            Shape::FreezeParade => Builder::P1(super::freeze_parade),
            Shape::LoneFreeze => Builder::P2(super::lone_freeze),
            Shape::ToothTail => Builder::Pair2(super::tooth_tail),
            Shape::PunctureProduct => Builder::Product(super::puncture_product),
            Shape::PlateauPuncture => Builder::P2(super::plateau_puncture),
            Shape::ArmingTrain => Builder::Train(super::arming_train),
            Shape::JumpPair => Builder::Pair3(super::jump_pair),
            Shape::ConcurrentPair => Builder::VersionPair(super::concurrent_pair),
            Shape::StaggerComb => Builder::P3(super::stagger_comb),
            Shape::StaggerId => Builder::P3(super::stagger_id),
            Shape::StaggerPopulation => Builder::Population2(super::stagger_population),
            Shape::MeetShade => Builder::Versions2(super::meet_shade),
            Shape::MaskDriftTriple => Builder::Triple2(super::mask_drift_triple),
            Shape::MaskDriftQuadruple => Builder::Quad2(super::mask_drift_quadruple),
        }
    }

    /// The accessor-mismatch failure: a shape asked to build through a
    /// signature its constructor does not have.
    fn wrong_door(self, called: &str) -> ! {
        panic!("{self:?} does not build through {called}: its variant doc names its accessor")
    }

    /// Build a one-knob packed shape.
    ///
    /// # Panics
    ///
    /// Panics if this shape's constructor takes a different signature,
    /// or on the constructor's own knob preconditions.
    pub fn packed1(self, a: usize) -> Packed {
        match self.builder() {
            Builder::P1(f) => f(a),
            _ => self.wrong_door("packed1"),
        }
    }

    /// Build a two-knob packed shape.
    ///
    /// # Panics
    ///
    /// Panics if this shape's constructor takes a different signature,
    /// or on the constructor's own knob preconditions.
    pub fn packed2(self, a: usize, b: usize) -> Packed {
        match self.builder() {
            Builder::P2(f) => f(a, b),
            _ => self.wrong_door("packed2"),
        }
    }

    /// Build a three-knob packed shape.
    ///
    /// # Panics
    ///
    /// Panics if this shape's constructor takes a different signature,
    /// or on the constructor's own knob preconditions.
    pub fn packed3(self, a: usize, b: usize, c: usize) -> Packed {
        match self.builder() {
            Builder::P3(f) => f(a, b, c),
            _ => self.wrong_door("packed3"),
        }
    }

    /// Build a knob-and-flag packed shape.
    ///
    /// # Panics
    ///
    /// Panics if this shape's constructor takes a different signature,
    /// or on the constructor's own knob preconditions.
    pub fn packed_flagged(self, a: usize, flag: bool) -> Packed {
        match self.builder() {
            Builder::Flag(f) => f(a, flag),
            _ => self.wrong_door("packed_flagged"),
        }
    }

    /// Build the arming-train signature: three knobs and a sign schedule.
    ///
    /// # Panics
    ///
    /// Panics if this shape's constructor takes a different signature,
    /// or on the constructor's own knob preconditions.
    pub fn packed_train(self, n: usize, w: usize, g: usize, alternate: bool) -> Packed {
        match self.builder() {
            Builder::Train(f) => f(n, w, g, alternate),
            _ => self.wrong_door("packed_train"),
        }
    }

    /// Build a packed shape over two arbitrary factors.
    ///
    /// # Panics
    ///
    /// Panics if this shape's constructor takes a different signature,
    /// or on the constructor's own factor preconditions.
    pub fn packed_product(self, x: &UBig, y: &UBig) -> Packed {
        match self.builder() {
            Builder::Product(f) => f(x, y),
            _ => self.wrong_door("packed_product"),
        }
    }

    /// Build a geometrically coupled two-knob packed pair.
    ///
    /// # Panics
    ///
    /// Panics if this shape's constructor takes a different signature,
    /// or on the constructor's own knob preconditions.
    pub fn packed_pair(self, a: usize, b: usize) -> (Packed, Packed) {
        match self.builder() {
            Builder::Pair2(f) => f(a, b),
            _ => self.wrong_door("packed_pair"),
        }
    }

    /// Build a geometrically coupled three-knob packed pair.
    ///
    /// # Panics
    ///
    /// Panics if this shape's constructor takes a different signature,
    /// or on the constructor's own knob preconditions.
    pub fn packed_pair3(self, a: usize, b: usize, c: usize) -> (Packed, Packed) {
        match self.builder() {
            Builder::Pair3(f) => f(a, b, c),
            _ => self.wrong_door("packed_pair3"),
        }
    }

    /// Build an organically constructed version pair.
    ///
    /// # Panics
    ///
    /// Panics if this shape's constructor takes a different signature,
    /// or on the constructor's own knob preconditions.
    pub fn version_pair(self, n: usize) -> (Version, Version) {
        match self.builder() {
            Builder::VersionPair(f) => f(n),
            _ => self.wrong_door("version_pair"),
        }
    }

    /// Build a fold population of versions.
    ///
    /// # Panics
    ///
    /// Panics if this shape's constructor takes a different signature,
    /// or on the constructor's own knob preconditions.
    pub fn versions(self, a: usize, b: usize) -> Vec<Version> {
        match self.builder() {
            Builder::Versions2(f) => f(a, b),
            _ => self.wrong_door("versions"),
        }
    }

    /// Build a fold population of (versions, ids), in feed order.
    ///
    /// # Panics
    ///
    /// Panics if this shape's constructor takes a different signature,
    /// or on the constructor's own knob preconditions.
    pub fn population(self, a: usize, b: usize) -> (Vec<Packed>, Vec<Packed>) {
        match self.builder() {
            Builder::Population2(f) => f(a, b),
            _ => self.wrong_door("population"),
        }
    }

    /// Build a correlated operand triple.
    ///
    /// # Panics
    ///
    /// Panics if this shape's constructor takes a different signature,
    /// or on the constructor's own knob preconditions.
    pub fn packed_triple(self, a: usize, b: usize) -> (Packed, Packed, Packed) {
        match self.builder() {
            Builder::Triple2(f) => f(a, b),
            _ => self.wrong_door("packed_triple"),
        }
    }

    /// Build a correlated operand quadruple.
    ///
    /// # Panics
    ///
    /// Panics if this shape's constructor takes a different signature,
    /// or on the constructor's own knob preconditions.
    pub fn packed_quadruple(self, a: usize, b: usize) -> ((Packed, Packed), (Packed, Packed)) {
        match self.builder() {
            Builder::Quad2(f) => f(a, b),
            _ => self.wrong_door("packed_quadruple"),
        }
    }
}

// ─── the family roster ───────────────────────────────────────────────────────

/// One adversarial input family: the roster every instrument's family
/// axis derives from.
///
/// The first thirty-two variants are the amplification board's columns,
/// in render order (each variant's doc carries its genre note); the rest
/// are the envelope suite's kernel-seam probe families, each answering
/// [`Coverage::EnvelopeOnly`] with the dated reason it earns no column.
/// Every variant's row of record is its [`FamilyId::spec`] answer.
///
/// Adding a family: the [`FamilyId::spec`] and [`FamilyId::index`] arms
/// and — for a board column — the board family module's bundle-build and
/// designed-diagonal match arms are compiler-forced from the variant.
/// What the compiler cannot force, in the order it is otherwise found by
/// luck: the roster entry in [`FamilyId::ALL`], the shape's base-size
/// constant (the board family module, with its derivation doc), that
/// module's family prose and any cardinality it carries, the declared
/// bundle reach on the variant's [`Coverage::Board`] answer, the
/// envelope rows in `tests/meter.rs` (the enforced record), the
/// ceiling-calibration witnesses (the board `ceilings` module's header
/// comment), and — only if a cell needs a declared model or turns up
/// red — the declaration site (the `ceilings` module's declared-models
/// section), the red-triage buffer
/// ([`BOARD_EXPECTED_REDS`](crate::meter::board::BOARD_EXPECTED_REDS),
/// with a live task), the rider list
/// ([`BOARD_DECLARED_BENCH_RIDERS`](crate::meter::board::BOARD_DECLARED_BENCH_RIDERS)),
/// and the judge roster with its membership pin
/// (`tools/benchjudge-expected.json`, `tests/bench_judge_roster.rs`).
/// And not every family belongs on the board: a whole-surface adversary
/// earns a column, while a kernel-seam shape answers
/// [`Coverage::EnvelopeOnly`] with the dated reason, as wide-tooth-comb,
/// alt-spine, and the memo probes do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FamilyId {
    /// The dense event spine `S(d)`: node count and depth maximizer.
    Dense,
    /// `bigroot(B, d)`: a huge root magnitude over a long spine.
    Bigroot,
    /// `hugeleaf(B)`: one node, maximal bits per node.
    Hugeleaf,
    /// The boundary comb `C(k, n)` at `k = n`: leaf values oscillating
    /// across a `2^k` carry cliff, every crossing paid by a stored code.
    Cliff,
    /// The diverted id-spine pair `I(d, ·)`: full-lockstep two-party walks.
    IdPair,
    /// The output-domination cross: boundary comb × scattered party.
    CombScatter,
    /// The harmonic spine `H(d)`: the rank fold's wide-numerator
    /// adversary, designed against the linear-functional rows and the
    /// rank pair.
    Harmonic,
    /// The scatter-ordered fold population: balanced-forked single-tick
    /// operands whose join accumulator never coalesces; its bundle
    /// carries fold operands alone, so only the fold rows apply.
    Scatter,
    /// The weave fold population: the leaves of one balanced fork tree
    /// dealt round-robin among 16 group parties (the board's weave-group
    /// constant), one tick each.
    ///
    /// Every operand is individually benign — an organic region set any
    /// retire/reunite call site could hold — while every internal node
    /// of the shared upper skeleton is both-present in every operand
    /// pair, so the fold's per-node costs that scale with the *other*
    /// operand (the overlap test against the accumulator, the join
    /// merges over interleaved trees) dominate. Scatter cannot reach
    /// this genre (its operands are single leaves) and benign reaches
    /// it only diluted; the arity is fixed so the scaling axis is
    /// both-present richness alone. Its bundle carries fold operands
    /// alone, so only the fold rows apply.
    Weave,
    /// The staggered fold population ([`Shape::StaggerPopulation`]): `n`
    /// operands of `m` unit teeth each, every operand's teeth landing
    /// in the gaps of every other's, fed in bit-reversed order.
    ///
    /// The correlated-population loading of the balanced reduction
    /// itself: the feed order pairs operands whose slot addresses
    /// diverge at the top bit, so every internal merge — at every
    /// level — joins region sets that interleave maximally and swell
    /// to near the sum of their sizes, the intermediate-swell worst
    /// case of the declared `O(D log k)` fold model, held until the
    /// last level (the full union collapses to the constant-1 skyline
    /// on the version side, the whole seed region on the id side).
    /// Scatter scales arity at single-leaf operands and weave scales
    /// operand size at fixed arity; this population scales both, and
    /// its bit-reversed feed forecloses the adjacent-slot coalescing
    /// luck index order would hand the counter. Its bundle carries
    /// fold operands alone, so only the fold rows apply.
    Stagger,
    /// The nested-full-sibling cross `N(d)` × the dense spine `S(d)`.
    ///
    /// Every level a right-full shortcut site, the deepest stacking of
    /// the walk's deferred right-full decisions and raise bookkeeping
    /// on narrow values — the designated cross of the two tick rows.
    NestedFull,
    /// The wide right-full cross: `bigroot(b, d)` × `N(d)`.
    ///
    /// The stream's first payload is coded absolute, so the deepest
    /// subtree's net movement carries the root's full magnitude and
    /// every level's bookkeeping meets it — width × depth through the
    /// right-full arm. The designated cross of the two tick rows.
    NestedWide,
    /// The wide left-full (memo) cross: `wide_tail(b, d)` × `M(d)`.
    ///
    /// Every proper subtree nets the tail's full magnitude while every
    /// level is a memoized pre-scan site — width × depth through the
    /// left-full arm and the pre-scan's own chains. The designated cross
    /// of the two tick rows.
    MirrorWide,
    /// The narrow left-full (memo) cross: `wide_tail(1, d)` × `M(d)`.
    ///
    /// The memoized pre-scan machinery itself, all values word-scale.
    /// The designated cross of the two tick rows.
    MirrorNarrow,
    /// The descending staircase `D(d)` × the unary id spine `I(d)`.
    ///
    /// Every consumed leaf undercuts every open range's minimum —
    /// full-penetration minimum updates at every level, all values
    /// word-scale. The designated cross of the two tick rows.
    Staircase,
    /// The reveal-comb cross: `reveal_comb(s, s)` × its own id.
    ///
    /// `s` sibling left-full sites share one `2^s`-wide minimum over a
    /// zero floor, and the left-leaning spine closes each site's frame
    /// back into the floor frame between consecutive consumes: the
    /// width-`s` boundary difference is minted at every consume and
    /// popped at every close — the unfunded width circulation, in the
    /// touch currency these columns do not carry (the gate pins in
    /// `tests/meter.rs` enforce it; the bench mirror's time leg sees
    /// it). The designated cross of the two tick rows.
    RevealComb,
    /// The reveal-comb control: `reveal_comb_hifloor(s, s)` × the
    /// reveal-comb id.
    ///
    /// Identical forest and close-reveal cycle with the floor raised
    /// to `2^s − 2`, so the circulated boundary difference is O(1)
    /// wide: the gap control. The designated cross of the two tick rows.
    RevealHifloor,
    /// The pure-comb cross: `pure_comb(s, s)` × its own id.
    ///
    /// The reveal comb's cycle with no left-full site anywhere — no
    /// memo, no pre-scan, no site consume: the base watermark stack's
    /// own arm-move + close-pop width circulation, isolated from the
    /// frame ledger. The designated cross of the two tick rows.
    PureComb,
    /// The ascending-cliff cross: `ascend_cliff(s, s)` × its own id.
    ///
    /// `s` ascending wide leaves stack `s − 1` nonzero unit boundary
    /// differences and a terminal 0-cliff drives one width-`s` undercut
    /// residue through all of them — the cascade whose per-hop fold
    /// direction the gate pins in `tests/meter.rs` price in the touch
    /// currency these columns do not carry. The designated cross of the
    /// two tick rows.
    AscendCliff,
    /// The ascending-cliff control: `ascend_cliff_plateau(s, s)` × the
    /// ascending-cliff id.
    ///
    /// Identical spine, arming schedule, and cliff undercut with every
    /// leaf leveled, so the difference stack is one compressed zero run
    /// the residue passes whole in O(1): the hop-schedule control.
    /// The designated cross of the two tick rows.
    AscendPlateau,
    /// The two-operand jump comb `jump_pair(k, m, d)`: wide
    /// height-difference crests over a dense-position spine.
    ///
    /// The overlay interleaves one operand's wide teeth with the
    /// other's cheap codes, so the pair rows park wide drift at the
    /// other operand's boundaries `2m` times while every absolute
    /// position stays `d` digits dense — the shape that separates
    /// segment-anchored freeze accounting (flat) from absolute-position
    /// accounting (superlinear), with each operand certified-linear
    /// alone (the generator doc carries the mechanism).
    JumpPair,
    /// The freeze-position spine `freeze_position(s)`: the
    /// many-freezes sentinel.
    ///
    /// `2s` descending wide leaves alternate a ten-digit drop and a
    /// unit drop down a right spine, so a query fold freezes `Θ(s)`
    /// times at ever-deeper stream positions — every comb fires O(1)
    /// freezes, which was exactly the coverage hole — and any freeze
    /// accounting that reads an absolute position (or any
    /// whole-history state) per freeze goes quadratic here while the
    /// family's positions compact to O(1) digits. The committed
    /// known-bad kernel reads ×1.50 per byte across the doubling on
    /// this shape (the query fold's adequacy tripwire); the
    /// anchored-segment discipline reads flat (the `skyline_flatness`
    /// freeze-position band). Designed against the linear-functional
    /// query rows.
    FreezePos,
    /// The promotion re-arm spine `promotion_rearm(s)`: the
    /// many-armings sentinel.
    ///
    /// `32s` span-building levels grow the consumed mass's written
    /// span, then `s` four-node blocks each park a wide drift and
    /// promote it at a narrow one — `Θ(s)` query-fold promotions at
    /// O(1) stored codes each, where every comb promotes never and the
    /// freeze-position spine's parked drift is monotone. Any promotion
    /// accounting that re-reads whole-history state per arming goes
    /// quadratic here while the family's suffix masses compact to O(1)
    /// balanced terms. The committed known-bad kernel reads ×1.74 per
    /// byte across the doubling on this shape (the query fold's
    /// span-promotion tripwire); the promotion ledger reads flat (the
    /// `skyline_flatness` promotion re-arm bands). Designed against
    /// the linear-functional query rows.
    PromoRearm,
    /// The weight-comb spine `weight_comb(n)`: the many-jumps
    /// sentinel.
    ///
    /// A depth-`32n` parked-unit spine, then `2n` shallow leaves
    /// oscillating heights 0 and 2: the rank integral deposits the
    /// oscillation at one digit position `Θ(n)` digits above the
    /// parked unit for O(1) stored bits per event — the position
    /// weight is topology, so no code funds the gap — and every
    /// cancellation makes the accumulator's top settle back across the
    /// never-written run. A settlement scan that steps the gap digit
    /// by digit goes quadratic here (×1.93 per byte across the
    /// doubling, measured under a probe build with certificate
    /// consumption disabled); consuming one zero-run certificate per
    /// jumped run reads flat (the `skyline_flatness` weight-comb
    /// band). Designed against the linear-functional query rows.
    WeightComb,
    /// The freeze-parade spine `freeze_parade(k)`: the deep-segment
    /// freeze sentinel.
    ///
    /// The parked-unit spine at depth `64k`, then `k` shallow freeze
    /// blocks whose wide in-pair drops each fire one query-fold freeze
    /// at the block's position weight, `Θ(k)` digits above digit 0, so
    /// every freeze's scaled segment read starts `Θ(k)` digits up. The
    /// accumulator's write watermark prices each read at the segment's
    /// written span; a scaled read that starts at digit 0 walks the
    /// never-written prefix per freeze and goes quadratic in the touch
    /// and limb currencies together (×1.91 per byte across the
    /// doubling, measured under a probe build whose scaled reads start
    /// at digit 0); the watermark reads flat (the `skyline_flatness`
    /// freeze-parade band). The freeze-position spine prices the query
    /// layer's per-freeze accounting; this family prices the
    /// accumulator's read side under the same schedule. Designed
    /// against the linear-functional query rows.
    FreezeParade,
    /// The dense-suffix pair `dense_suffix(p, p)` against its unit
    /// mate `dense_suffix_mate(p, p)`: the many-armings ×
    /// dense-trailing-mass sentinel.
    ///
    /// A gap spine holds the trailing interval mass at `Θ(p)` balanced
    /// digits, then `p` re-arm blocks each park a wide drift and
    /// promote it at O(1) stored codes — `Θ(p)` ledger armings all
    /// owing their debt across the same `Θ(p)`-dense trailing mass, so
    /// a settle that walks the suffix once per arming (or re-reads a
    /// promoted prefix once per window) goes quadratic here while the
    /// mass-balanced product tree charges every arming-window cross
    /// term inside one aggregate product and reads flat. The committed
    /// tripwire beside the kernel
    /// (`suffix_walk_settle_reads_superlinear_on_dense_suffix`, the
    /// query fold's test suite) keeps the per-arming walk failing on
    /// this family. The mate is the same topology at unit bases, and
    /// the wide operand dominates it pointwise, so the pair rows run
    /// the co-sweep whose freezes and promotions fire on drift only
    /// the wide operand deposited (the `skyline_flatness` dense-suffix
    /// rank and distance bands carry the enforcement). Designed
    /// against the linear-functional query rows.
    DenseSuffix,
    /// The wide-arming family `wide_arming(s, s)`: the single-arming
    /// wide × dense sentinel, both factors on one knob.
    ///
    /// The gap spine holds the trailing interval mass at `Θ(s)`
    /// isolated digits and the one re-arm block parks a `2^(32s)`
    /// drift and promotes it — one ledger arming as wide as the input
    /// owing its debt across a trailing mass as dense as the input,
    /// so the settle's one aggregate product is the wide × dense
    /// cross term at its purest, undodgeable by seam cancellation
    /// (the `ledger_wide_arming` band in `tests/meter.rs` carries the
    /// enforcement; the committed schoolbook settle kernel keeps the
    /// per-digit charge failing on this family). Its rendered text is
    /// the same shape at the parse seam: one wide swing ahead of
    /// `Θ(s)` trailing zero-delta leaves, where a per-leaf delta
    /// extraction that pays a stale high-water span instead of the
    /// settled top reads `Θ(w·d)` touches on `Θ(w + d)` text (the
    /// `parse_wide_arming` band and the committed schoolbook parse
    /// kernel carry both readings), so the column's five text-parse
    /// cells are the standing watch on the exact-`top` genre at the
    /// text seam. Designed against the linear-functional query rows.
    WideArming,
    /// The plateau-puncture family `plateau_puncture(s, s)`: the
    /// answer-embedded-product sentinel, and the floor under every
    /// settle.
    ///
    /// Every turn leaf sits on one incompressible pseudorandom plateau
    /// `x` of `Θ(s)` digits and the turn positions spell a jittered
    /// punctured mass `y` of `Θ(s)` isolated digits, so the exact rank
    /// embeds the integer product `2·x·y + 1` — bought with `Θ(s)`
    /// input bits, both factors' content beyond the settle's own
    /// balanced-digit compaction. No promotion ever fires; the cost is
    /// the close-time settle, one wide × dense multiplication run
    /// inside the backend at its bound `M(|v|)` — and because the same
    /// constructor embeds the product of arbitrary factors, any fold
    /// that answers exactly multiplies arbitrary input-funded
    /// integers, so `Ω(M(|v|))` floors every settle. The committed
    /// kernel
    /// (`schoolbook_settle_reads_superlinear_on_plateau_puncture`, the
    /// query fold's test suite) keeps the per-digit charge failing on
    /// this family (the `skyline_flatness` plateau-puncture band
    /// carries the enforcement). Designed against the
    /// linear-functional query rows.
    PlateauPuncture,
    /// The lone-freeze spine `lone_freeze(s, s)`: the first-freeze
    /// gate straddle, both sides on one knob.
    ///
    /// `s` unit-oscillation pairs ride a wide plateau strictly before
    /// the sweep's one freeze-firing drop, and `s` more run behind it
    /// with the gate open and a ten-digit drift parked — so any
    /// per-interval deposit toward the settle machinery made before
    /// drift exists to settle scales with the prefix, and a segment
    /// feed or close read that is not amortized O(1) per interval
    /// scales with the tail, while the family's funded wide codes stay
    /// O(1). Exactly one freeze and no promotion ever fires, so the
    /// column also prices the settle's smallest nonempty
    /// configuration. The `skyline_flatness` lone-freeze bands isolate
    /// each axis at the generator minimum and carry the enforcement;
    /// the column scales both together. Designed against the
    /// linear-functional query rows.
    LoneFreeze,
    /// The concurrent pair `concurrent_pair(n)`: the emit side-switch
    /// density population.
    ///
    /// Organically forked and ticked so the sweep's side switch fires at
    /// every one of the `n − 1` overlay boundaries, join and meet alike
    /// — the pairing the ticked counterpart cannot reach.
    ConcurrentPair,
    /// The tooth-tail pair `tooth_tail(g, m)`: the boundary-aligned
    /// exact-`top` population.
    ///
    /// Two same-shape unit chains whose second leaves spike `2^(32g)`
    /// in both operands, `b` one tick above `a` everywhere except the
    /// shared terminal: the pair sweep folds both spikes into one
    /// cancelling difference at the same boundary, then reads
    /// `sign(D)` once per remaining boundary with no intervening
    /// write. Exact-`top` maintenance prices each read at the settled
    /// value's own width; a high-water bound re-walks the spike's `g`
    /// dead digits per read — `Θ(m·g)` on `Θ(m + g)` input (the
    /// `skyline_flatness` tooth-tail band carries both readings).
    /// Every overlay boundary is shared by both operands and almost
    /// every stored delta is zero, so the pair is also the touch
    /// floor's honest-less-work witness (the board floor module's
    /// `touch_pair_fold`): a conforming sweep is forced to fold only
    /// the three nonzero deltas per operand, and the measured
    /// per-boundary sign-read traffic sits far above that floor as
    /// implementation, never mandate.
    ToothTail,
    /// The fixed-seed organic control population.
    Benign,
    /// The wide-tooth comb `W(k, w, n)`: bounded wide oscillation —
    /// height state that must *stay* live across a fixed-width window.
    WideToothComb,
    /// The jump comb `J(k, n)`: the stale-drift eviction probe — height
    /// state that must *leave* the cheap-delta path exactly once.
    JumpComb,
    /// The unpaid-crossing fan `F(k, n)`: `n` sibling carry excursions
    /// funded by one stored magnitude.
    CliffFan,
    /// The cancelling-prefix chain `P(k, n)`: deep sign scans funded by
    /// the wide writes that immediately precede them.
    CancellingChain,
    /// The alternating-binary spine `A(d)`: the frame-count adversary
    /// for iterative walks that keep per-level records.
    AltSpine,
    /// The memo-chain pair `Q(k, distinct)` × its id: `k`
    /// consumption-sibling memo records in one fresh scan, with the
    /// shared twin (all differences zero) as the unstored control.
    MemoChain,
    /// The memo-comb pair `B(d)` × its id: consecutively consumed sites
    /// Θ(d) apart in recording order, the resolution-order adversary.
    MemoComb,
    /// The memo fan-out `F(k, b)`: one wide minimum shared by `k` sites,
    /// paid by the input exactly once — the funding argument's red side.
    MemoFanout,
    /// The oscillating siblings `O(k, b)`: every ledger link wide but
    /// funded one-for-one by the input — the funding argument's control.
    MemoOscillating,
    /// The memo-churn pair `U(d)` × its id: a descending run
    /// undercutting `d` live records, the live-anchored followers'
    /// tombstone.
    MemoChurn,
    /// The descending raises `W(d)` × its id: a floor realized high with
    /// every site's raise landing below it — the decide-then-emit
    /// ordering's one exerciser.
    DescendingRaises,
    /// The masked-comparison correlated tuples `MT(k, n)` / `MQ(k, n)`:
    /// operand pairings built for the fused three- and four-stream
    /// comparison walks alone.
    MaskDrift,
    /// The meet-shade population `MS(d, k)`: one deep carrier under
    /// `k − 1` dominating plateau shades, the meet fold's wedge.
    MeetShade,
    /// The arming-train family `AT(n, w, g, alternate)`: the product
    /// tree's level-ratio probe, three fixed-width points in two sign
    /// schedules.
    ArmingTrain,
}

/// One family's row of record: the answers every instrument derives
/// from.
#[derive(Debug, Clone, Copy)]
pub struct FamilySpec {
    /// The family name of record: the board column header, the bench
    /// cell key, and the prose name the bands and pins use.
    pub name: &'static str,
    /// The registered constructors that build this family's operands.
    ///
    /// Empty exactly for the populations built organically from the
    /// public API (scatter, weave, benign), whose construction lives in
    /// the board's family module.
    pub shapes: &'static [Shape],
    /// The board answer: a column with its declared bundle reach, or
    /// the dated reason this family earns no column.
    pub coverage: Coverage,
    /// The envelope-band answer: the committed band roster in
    /// `tests/meter.rs`, or the dated reason no band exists.
    pub bands: Bands,
    /// The denominator of record: what this family's priced readings
    /// are charged against.
    pub denominator: &'static str,
    /// The closed-form hook, where one exists: the quantity computable
    /// two ways and the pin that compares them.
    pub closed_form: Option<&'static str>,
}

/// A family's board answer.
#[derive(Debug, Clone, Copy)]
pub enum Coverage {
    /// A board column: the whole-surface product prices this family on
    /// every operation row its operand bundle supplies.
    ///
    /// `cells` is the declared bundle reach — how many operation rows
    /// the bundle feeds, scale-independent — which the board smoke
    /// suite holds the rendered matrix to, so a bundle slot gained or
    /// lost without a deliberate re-declaration fails there.
    Board {
        /// The declared operation-row reach of this family's bundle.
        ///
        /// The version-only shapes (a version, its derived pairings,
        /// and its rejection rows) supply 49 rows; the id pair
        /// (parties only) 38; the cross shapes (version, mounted
        /// party pair, clock, and the id-side rejections) 70; the
        /// fold-only populations exactly the 3 fold rows; and the
        /// benign control supplies every row.
        cells: usize,
    },
    /// No board column: a kernel-seam probe (or an operand-tuple
    /// pairing) whose enforcement home is the envelope suite alone,
    /// with the dated ruling.
    EnvelopeOnly {
        /// Why this family earns no column (the board-roster criterion
        /// answer).
        reason: &'static str,
        /// The date of the ruling of record.
        decided: &'static str,
    },
}

/// A family's envelope-band answer.
#[derive(Debug, Clone, Copy)]
pub enum Bands {
    /// The committed flatness/adequacy bands in `tests/meter.rs` that
    /// price this family, by test name.
    Priced(&'static [&'static str]),
    /// No band, with the dated reason (which instrument prices the
    /// family instead).
    Unbanded {
        /// Why no two-point band exists for this family.
        reason: &'static str,
        /// The date of the ruling of record.
        decided: &'static str,
    },
}

/// The default denominator: packed input bytes.
const PACKED: &str = "packed input bytes";

/// The date the registry's rulings were ratified as the rows of record.
const REGISTRY_RATIFIED: &str = "2026-07-29";

/// The fold populations' shared denominator note.
const FOLD_DENOM: &str = "packed operand bytes, judged under the declared O(D log k) fold model";

/// The tick crosses' shared no-band reason.
const TICK_CROSS_UNBANDED: &str = "tick cross: the tick gate pins in tests/meter.rs price its walk";

impl FamilyId {
    /// Every registered family, in the roster order of record: the
    /// board columns first, in render order, then the envelope-only
    /// probe families.
    pub const ALL: [FamilyId; 46] = [
        FamilyId::Dense,
        FamilyId::Bigroot,
        FamilyId::Hugeleaf,
        FamilyId::Cliff,
        FamilyId::IdPair,
        FamilyId::CombScatter,
        FamilyId::Harmonic,
        FamilyId::Scatter,
        FamilyId::Weave,
        FamilyId::Stagger,
        FamilyId::NestedFull,
        FamilyId::NestedWide,
        FamilyId::MirrorWide,
        FamilyId::MirrorNarrow,
        FamilyId::Staircase,
        FamilyId::RevealComb,
        FamilyId::RevealHifloor,
        FamilyId::PureComb,
        FamilyId::AscendCliff,
        FamilyId::AscendPlateau,
        FamilyId::JumpPair,
        FamilyId::FreezePos,
        FamilyId::PromoRearm,
        FamilyId::WeightComb,
        FamilyId::FreezeParade,
        FamilyId::DenseSuffix,
        FamilyId::WideArming,
        FamilyId::PlateauPuncture,
        FamilyId::LoneFreeze,
        FamilyId::ConcurrentPair,
        FamilyId::ToothTail,
        FamilyId::Benign,
        FamilyId::WideToothComb,
        FamilyId::JumpComb,
        FamilyId::CliffFan,
        FamilyId::CancellingChain,
        FamilyId::AltSpine,
        FamilyId::MemoChain,
        FamilyId::MemoComb,
        FamilyId::MemoFanout,
        FamilyId::MemoOscillating,
        FamilyId::MemoChurn,
        FamilyId::DescendingRaises,
        FamilyId::MaskDrift,
        FamilyId::MeetShade,
        FamilyId::ArmingTrain,
    ];

    /// This family's position in [`FamilyId::ALL`] — the roster-order
    /// tie the registry tests hold against the array, so a variant
    /// cannot be declared without joining the roster at a committed
    /// position.
    pub const fn index(self) -> usize {
        match self {
            FamilyId::Dense => 0,
            FamilyId::Bigroot => 1,
            FamilyId::Hugeleaf => 2,
            FamilyId::Cliff => 3,
            FamilyId::IdPair => 4,
            FamilyId::CombScatter => 5,
            FamilyId::Harmonic => 6,
            FamilyId::Scatter => 7,
            FamilyId::Weave => 8,
            FamilyId::Stagger => 9,
            FamilyId::NestedFull => 10,
            FamilyId::NestedWide => 11,
            FamilyId::MirrorWide => 12,
            FamilyId::MirrorNarrow => 13,
            FamilyId::Staircase => 14,
            FamilyId::RevealComb => 15,
            FamilyId::RevealHifloor => 16,
            FamilyId::PureComb => 17,
            FamilyId::AscendCliff => 18,
            FamilyId::AscendPlateau => 19,
            FamilyId::JumpPair => 20,
            FamilyId::FreezePos => 21,
            FamilyId::PromoRearm => 22,
            FamilyId::WeightComb => 23,
            FamilyId::FreezeParade => 24,
            FamilyId::DenseSuffix => 25,
            FamilyId::WideArming => 26,
            FamilyId::PlateauPuncture => 27,
            FamilyId::LoneFreeze => 28,
            FamilyId::ConcurrentPair => 29,
            FamilyId::ToothTail => 30,
            FamilyId::Benign => 31,
            FamilyId::WideToothComb => 32,
            FamilyId::JumpComb => 33,
            FamilyId::CliffFan => 34,
            FamilyId::CancellingChain => 35,
            FamilyId::AltSpine => 36,
            FamilyId::MemoChain => 37,
            FamilyId::MemoComb => 38,
            FamilyId::MemoFanout => 39,
            FamilyId::MemoOscillating => 40,
            FamilyId::MemoChurn => 41,
            FamilyId::DescendingRaises => 42,
            FamilyId::MaskDrift => 43,
            FamilyId::MeetShade => 44,
            FamilyId::ArmingTrain => 45,
        }
    }

    /// The amplification board's family axis, in render order: the
    /// roster filtered on each variant's committed coverage answer.
    pub fn board() -> impl Iterator<Item = FamilyId> {
        FamilyId::ALL
            .into_iter()
            .filter(|f| matches!(f.spec().coverage, Coverage::Board { .. }))
    }

    /// The family name of record (the spec's `name`).
    pub fn name(self) -> &'static str {
        self.spec().name
    }

    /// This family's row of record.
    pub const fn spec(self) -> FamilySpec {
        match self {
            FamilyId::Dense => FamilySpec {
                name: "dense",
                shapes: &[Shape::Dense],
                coverage: Coverage::Board { cells: 49 },
                bands: Bands::Unbanded {
                    reason: "depth/node maximizer; absolute envelope rows carry it, no \
                             committed two-point flatness claim",
                    decided: REGISTRY_RATIFIED,
                },
                denominator: PACKED,
                closed_form: Some(
                    "4d + 4 bits for 2d + 1 nodes at depth d; the meter suite pins the \
                     size closed form",
                ),
            },
            FamilyId::Bigroot => FamilySpec {
                name: "bigroot",
                shapes: &[Shape::Bigroot],
                coverage: Coverage::Board { cells: 49 },
                bands: Bands::Unbanded {
                    reason: "magnitude-over-depth shape; absolute envelope rows carry it",
                    decided: REGISTRY_RATIFIED,
                },
                denominator: PACKED,
                closed_form: None,
            },
            FamilyId::Hugeleaf => FamilySpec {
                name: "hugeleaf",
                shapes: &[Shape::Hugeleaf],
                coverage: Coverage::Board { cells: 49 },
                bands: Bands::Unbanded {
                    reason: "single-node magnitude maximizer; absolute envelope rows carry it",
                    decided: REGISTRY_RATIFIED,
                },
                denominator: PACKED,
                closed_form: None,
            },
            FamilyId::Cliff => FamilySpec {
                name: "cliff",
                shapes: &[Shape::CliffComb],
                coverage: Coverage::Board { cells: 49 },
                bands: Bands::Priced(&[
                    "skyline_validate_cliff_cost_is_flat_per_unit",
                    "skyline_cmp_cliff_cost_is_flat_per_unit",
                    "skyline_join_cliff_cost_is_flat_per_unit",
                    "skyline_parse_cliff_touch_cost_is_flat_per_unit",
                ]),
                denominator: PACKED,
                closed_form: None,
            },
            FamilyId::IdPair => FamilySpec {
                name: "id-pair",
                shapes: &[Shape::IdSpine],
                coverage: Coverage::Board { cells: 38 },
                bands: Bands::Unbanded {
                    reason: "party-only bundle; the flatness bands price version query and \
                             comparison kernels",
                    decided: REGISTRY_RATIFIED,
                },
                denominator: PACKED,
                closed_form: None,
            },
            FamilyId::CombScatter => FamilySpec {
                name: "comb-scatter",
                shapes: &[Shape::CliffComb, Shape::ScatteredId],
                coverage: Coverage::Board { cells: 70 },
                bands: Bands::Unbanded {
                    reason: "the output-domination cross; its projection rows are \
                             I/O-denominated",
                    decided: REGISTRY_RATIFIED,
                },
                denominator: "value content bytes for exponents (the flat-denominator \
                              shape); packed I/O on the projection rows",
                closed_form: None,
            },
            FamilyId::Harmonic => FamilySpec {
                name: "harmonic",
                shapes: &[Shape::Harmonic],
                coverage: Coverage::Board { cells: 49 },
                bands: Bands::Unbanded {
                    reason: "the rank fold's wide-numerator adversary; the board's harmonic \
                             tripwire column and its envelope rows carry it",
                    decided: REGISTRY_RATIFIED,
                },
                denominator: PACKED,
                closed_form: Some(
                    "rank telescopes to (2^d − 1)/2^d; the meter suite pins the closed \
                     form against the fold",
                ),
            },
            FamilyId::Scatter => FamilySpec {
                name: "scatter",
                shapes: &[],
                coverage: Coverage::Board { cells: 3 },
                bands: Bands::Unbanded {
                    reason: "fold-only bundle; the fold rows are judged by the declared \
                             fold model",
                    decided: REGISTRY_RATIFIED,
                },
                denominator: FOLD_DENOM,
                closed_form: None,
            },
            FamilyId::Weave => FamilySpec {
                name: "weave",
                shapes: &[],
                coverage: Coverage::Board { cells: 3 },
                bands: Bands::Unbanded {
                    reason: "fold-only bundle; the fold rows are judged by the declared \
                             fold model",
                    decided: REGISTRY_RATIFIED,
                },
                denominator: FOLD_DENOM,
                closed_form: None,
            },
            FamilyId::Stagger => FamilySpec {
                name: "stagger",
                shapes: &[
                    Shape::StaggerPopulation,
                    Shape::StaggerComb,
                    Shape::StaggerId,
                ],
                coverage: Coverage::Board { cells: 3 },
                bands: Bands::Priced(&[
                    "fold_version_stagger_arity_axis_is_flat_per_unit",
                    "fold_version_stagger_size_axis_is_flat_per_unit",
                    "fold_party_stagger_arity_axis_is_flat_per_unit",
                    "fold_party_stagger_size_axis_is_flat_per_unit",
                ]),
                denominator: FOLD_DENOM,
                closed_form: None,
            },
            FamilyId::NestedFull => FamilySpec {
                name: "nested-full",
                shapes: &[Shape::Dense, Shape::NestedFullId],
                coverage: Coverage::Board { cells: 70 },
                bands: Bands::Unbanded {
                    reason: TICK_CROSS_UNBANDED,
                    decided: REGISTRY_RATIFIED,
                },
                denominator: PACKED,
                closed_form: None,
            },
            FamilyId::NestedWide => FamilySpec {
                name: "nested-wide",
                shapes: &[Shape::Bigroot, Shape::NestedFullId],
                coverage: Coverage::Board { cells: 70 },
                bands: Bands::Unbanded {
                    reason: TICK_CROSS_UNBANDED,
                    decided: REGISTRY_RATIFIED,
                },
                denominator: PACKED,
                closed_form: None,
            },
            FamilyId::MirrorWide => FamilySpec {
                name: "mirror-wide",
                shapes: &[Shape::WideTail, Shape::NestedLeftFullId],
                coverage: Coverage::Board { cells: 70 },
                bands: Bands::Unbanded {
                    reason: TICK_CROSS_UNBANDED,
                    decided: REGISTRY_RATIFIED,
                },
                denominator: PACKED,
                closed_form: None,
            },
            FamilyId::MirrorNarrow => FamilySpec {
                name: "mirror-narrow",
                shapes: &[Shape::WideTail, Shape::NestedLeftFullId],
                coverage: Coverage::Board { cells: 70 },
                bands: Bands::Unbanded {
                    reason: TICK_CROSS_UNBANDED,
                    decided: REGISTRY_RATIFIED,
                },
                denominator: PACKED,
                closed_form: None,
            },
            FamilyId::Staircase => FamilySpec {
                name: "staircase",
                shapes: &[Shape::Staircase, Shape::IdSpine],
                coverage: Coverage::Board { cells: 70 },
                bands: Bands::Unbanded {
                    reason: TICK_CROSS_UNBANDED,
                    decided: REGISTRY_RATIFIED,
                },
                denominator: PACKED,
                closed_form: None,
            },
            FamilyId::RevealComb => FamilySpec {
                name: "reveal-comb",
                shapes: &[Shape::RevealComb, Shape::RevealCombId],
                coverage: Coverage::Board { cells: 70 },
                bands: Bands::Priced(&["skyline_min_ticks_reveal_comb_is_flat_per_unit"]),
                denominator: "packed input bytes; packed I/O on the output-dominated \
                              projection rows",
                closed_form: None,
            },
            FamilyId::RevealHifloor => FamilySpec {
                name: "reveal-hifloor",
                shapes: &[Shape::RevealCombHifloor, Shape::RevealCombId],
                coverage: Coverage::Board { cells: 70 },
                bands: Bands::Priced(&["reveal_comb_hifloor_control_is_flat_per_unit"]),
                denominator: "packed input bytes; packed I/O on the output-dominated \
                              projection rows",
                closed_form: None,
            },
            FamilyId::PureComb => FamilySpec {
                name: "pure-comb",
                shapes: &[Shape::PureComb, Shape::PureCombId],
                coverage: Coverage::Board { cells: 70 },
                bands: Bands::Priced(&["skyline_min_ticks_pure_comb_is_flat_per_unit"]),
                denominator: "packed input bytes; packed I/O on the output-dominated \
                              projection rows",
                closed_form: None,
            },
            FamilyId::AscendCliff => FamilySpec {
                name: "ascend-cliff",
                shapes: &[Shape::AscendCliff, Shape::AscendCliffId],
                coverage: Coverage::Board { cells: 70 },
                bands: Bands::Unbanded {
                    reason: "the cascade's red-direction driver; its leveled control \
                             (ascend-plateau) carries the committed flatness band",
                    decided: REGISTRY_RATIFIED,
                },
                denominator: PACKED,
                closed_form: None,
            },
            FamilyId::AscendPlateau => FamilySpec {
                name: "ascend-plateau",
                shapes: &[Shape::AscendCliffPlateau, Shape::AscendCliffId],
                coverage: Coverage::Board { cells: 70 },
                bands: Bands::Priced(&["ascend_cliff_plateau_control_is_flat_per_unit"]),
                denominator: PACKED,
                closed_form: None,
            },
            FamilyId::JumpPair => FamilySpec {
                name: "jump-pair",
                shapes: &[Shape::JumpPair],
                coverage: Coverage::Board { cells: 49 },
                bands: Bands::Priced(&["skyline_distance_jump_pair_is_flat_per_unit"]),
                denominator: PACKED,
                closed_form: None,
            },
            FamilyId::FreezePos => FamilySpec {
                name: "freeze-pos",
                shapes: &[Shape::FreezePosition],
                coverage: Coverage::Board { cells: 49 },
                bands: Bands::Priced(&[
                    "skyline_rank_freeze_position_is_flat_per_unit",
                    "skyline_min_ticks_freeze_position_is_flat_per_unit",
                    "skyline_distance_freeze_position_is_flat_per_unit",
                ]),
                denominator: PACKED,
                closed_form: Some(
                    "rank exponent 2s − 1 (one trailing zero strips): the remainder-\
                     alignment derivation on the family's board base constant",
                ),
            },
            FamilyId::PromoRearm => FamilySpec {
                name: "promo-rearm",
                shapes: &[Shape::PromotionRearm, Shape::PromotionRearmMate],
                coverage: Coverage::Board { cells: 49 },
                bands: Bands::Priced(&[
                    "skyline_rank_promotion_rearm_is_flat_per_unit",
                    "skyline_min_ticks_promotion_rearm_is_flat_per_unit",
                    "skyline_distance_promotion_rearm_is_flat_per_unit",
                ]),
                denominator: PACKED,
                closed_form: Some(
                    "rank exponent 36s: the remainder-alignment derivation on the \
                     family's board base constant",
                ),
            },
            FamilyId::WeightComb => FamilySpec {
                name: "weight-comb",
                shapes: &[Shape::WeightComb],
                coverage: Coverage::Board { cells: 49 },
                bands: Bands::Priced(&["skyline_rank_weight_comb_is_flat_per_unit"]),
                denominator: PACKED,
                closed_form: None,
            },
            FamilyId::FreezeParade => FamilySpec {
                name: "freeze-parade",
                shapes: &[Shape::FreezeParade],
                coverage: Coverage::Board { cells: 49 },
                bands: Bands::Priced(&["skyline_rank_freeze_parade_is_flat_per_unit"]),
                denominator: PACKED,
                closed_form: None,
            },
            FamilyId::DenseSuffix => FamilySpec {
                name: "dense-suffix",
                shapes: &[Shape::DenseSuffix, Shape::DenseSuffixMate],
                coverage: Coverage::Board { cells: 49 },
                bands: Bands::Priced(&[
                    "skyline_rank_dense_suffix_is_flat_per_unit",
                    "skyline_distance_dense_suffix_is_flat_per_unit",
                ]),
                denominator: PACKED,
                closed_form: None,
            },
            FamilyId::WideArming => FamilySpec {
                name: "wide-arming",
                shapes: &[Shape::WideArming],
                coverage: Coverage::Board { cells: 49 },
                bands: Bands::Priced(&[
                    "rank_wide_arming_is_flat_per_unit",
                    "parse_wide_arming_touch_cost_is_flat_per_unit",
                ]),
                denominator: PACKED,
                closed_form: Some(
                    "rank exponent 32s (remainder 0 at every knob): the derivation on \
                     the family's board base constant",
                ),
            },
            FamilyId::PlateauPuncture => FamilySpec {
                name: "plateau-puncture",
                shapes: &[Shape::PlateauPuncture, Shape::PunctureProduct],
                coverage: Coverage::Board { cells: 49 },
                bands: Bands::Priced(&["rank_plateau_puncture_is_flat_per_unit"]),
                denominator: PACKED,
                closed_form: Some(
                    "the exact rank embeds the integer product 2·x·y + 1 of the committed \
                     factors (plateau_puncture_factors); the query fold's suite pins the \
                     embedding against the backend's own multiply",
                ),
            },
            FamilyId::LoneFreeze => FamilySpec {
                name: "lone-freeze",
                shapes: &[Shape::LoneFreeze],
                coverage: Coverage::Board { cells: 49 },
                bands: Bands::Priced(&[
                    "skyline_rank_lone_freeze_late_is_flat_per_unit",
                    "skyline_rank_lone_freeze_tail_is_flat_per_unit",
                ]),
                denominator: PACKED,
                closed_form: None,
            },
            FamilyId::ConcurrentPair => FamilySpec {
                name: "concurrent-pair",
                shapes: &[Shape::ConcurrentPair],
                coverage: Coverage::Board { cells: 49 },
                bands: Bands::Unbanded {
                    reason: "the switch-density pair; absolute envelope pair-query rows \
                             carry it",
                    decided: REGISTRY_RATIFIED,
                },
                denominator: PACKED,
                closed_form: None,
            },
            FamilyId::ToothTail => FamilySpec {
                name: "tooth-tail",
                shapes: &[Shape::ToothTail],
                coverage: Coverage::Board { cells: 49 },
                bands: Bands::Priced(&["skyline_cmp_tooth_tail_is_flat_per_unit"]),
                denominator: PACKED,
                closed_form: None,
            },
            FamilyId::Benign => FamilySpec {
                name: "benign",
                shapes: &[],
                coverage: Coverage::Board { cells: 73 },
                bands: Bands::Unbanded {
                    reason: "the organic control population; flatness bands price \
                             adversarial constructions",
                    decided: REGISTRY_RATIFIED,
                },
                denominator: "packed input bytes (the organic control)",
                closed_form: None,
            },
            FamilyId::WideToothComb => FamilySpec {
                name: "wide-tooth-comb",
                shapes: &[Shape::WideToothComb],
                coverage: Coverage::EnvelopeOnly {
                    reason: "kernel-seam probe measured through the internal skyline \
                             entries, which the board's public-operation rows cannot host \
                             — a deliberate, documented internal-entry decision at the \
                             band's citation site",
                    decided: REGISTRY_RATIFIED,
                },
                bands: Bands::Priced(&["skyline_rank_wide_tooth_freeze_band"]),
                denominator: "packed input bytes, through the internal skyline rank entry",
                closed_form: None,
            },
            FamilyId::JumpComb => FamilySpec {
                name: "jump-comb",
                shapes: &[Shape::JumpComb],
                coverage: Coverage::EnvelopeOnly {
                    reason: "kernel-seam probe measured through the internal skyline \
                             entries — a deliberate, documented internal-entry decision at \
                             the band's citation site; its whole-surface lift is the \
                             jump-pair family",
                    decided: REGISTRY_RATIFIED,
                },
                bands: Bands::Priced(&["skyline_rank_jump_eviction_is_flat_per_unit"]),
                denominator: "packed input bytes, through the internal skyline rank entry",
                closed_form: None,
            },
            FamilyId::CliffFan => FamilySpec {
                name: "cliff-fan",
                shapes: &[Shape::CliffFan],
                coverage: Coverage::EnvelopeOnly {
                    reason: "kernel-seam probe: sibling carry excursions funded by one \
                             stored magnitude, priced by the in-crate skyline and tier2 \
                             suites' pinned envelopes",
                    decided: REGISTRY_RATIFIED,
                },
                bands: Bands::Unbanded {
                    reason: "its pins are absolute envelopes in the in-crate skyline and \
                             tier2 suites, not two-point bands in tests/meter.rs",
                    decided: REGISTRY_RATIFIED,
                },
                denominator: PACKED,
                closed_form: None,
            },
            FamilyId::CancellingChain => FamilySpec {
                name: "cancelling-chain",
                shapes: &[Shape::CancellingChain],
                coverage: Coverage::EnvelopeOnly {
                    reason: "kernel-seam probe: deep sign scans funded by adjacent wide \
                             writes, priced by the in-crate skyline suites' pinned \
                             envelopes",
                    decided: REGISTRY_RATIFIED,
                },
                bands: Bands::Unbanded {
                    reason: "its pins are absolute envelopes in the in-crate skyline and \
                             tier2 suites, not two-point bands in tests/meter.rs",
                    decided: REGISTRY_RATIFIED,
                },
                denominator: PACKED,
                closed_form: None,
            },
            FamilyId::AltSpine => FamilySpec {
                name: "alt-spine",
                shapes: &[Shape::AltSpine],
                coverage: Coverage::EnvelopeOnly {
                    reason: "kernel-seam probe: the frame-count adversary for iterative \
                             walks; its envelope rows in tests/meter.rs and the in-crate \
                             skyline suites price it",
                    decided: REGISTRY_RATIFIED,
                },
                bands: Bands::Unbanded {
                    reason: "its pins are absolute envelope rows, not two-point bands",
                    decided: REGISTRY_RATIFIED,
                },
                denominator: PACKED,
                closed_form: None,
            },
            FamilyId::MemoChain => FamilySpec {
                name: "memo-chain",
                shapes: &[Shape::MemoChain, Shape::MemoChainId],
                coverage: Coverage::EnvelopeOnly {
                    reason: "the memo_* shapes are kernel-seam probes by the board-roster \
                             criterion",
                    decided: REGISTRY_RATIFIED,
                },
                bands: Bands::Priced(&["memo_chain_shared_control_is_flat_per_unit"]),
                denominator: "packed cross bytes (the touch currency)",
                closed_form: None,
            },
            FamilyId::MemoComb => FamilySpec {
                name: "memo-comb",
                shapes: &[Shape::MemoComb, Shape::MemoCombId],
                coverage: Coverage::EnvelopeOnly {
                    reason: "the memo_* shapes are kernel-seam probes by the board-roster \
                             criterion",
                    decided: REGISTRY_RATIFIED,
                },
                bands: Bands::Unbanded {
                    reason: "priced by the memo-resolution gate pins in tests/meter.rs, \
                             absolute pins outside the band convention",
                    decided: REGISTRY_RATIFIED,
                },
                denominator: "packed cross bytes (the touch currency)",
                closed_form: None,
            },
            FamilyId::MemoFanout => FamilySpec {
                name: "memo-fanout",
                shapes: &[Shape::MemoFanout],
                coverage: Coverage::EnvelopeOnly {
                    reason: "the memo_* shapes are kernel-seam probes by the board-roster \
                             criterion",
                    decided: REGISTRY_RATIFIED,
                },
                bands: Bands::Unbanded {
                    reason: "priced by the memo-resolution gate pins in tests/meter.rs \
                             (the absolute touch ceiling is what an unfunded fan-out \
                             blows), outside the band convention",
                    decided: REGISTRY_RATIFIED,
                },
                denominator: "packed cross bytes (the touch currency)",
                closed_form: None,
            },
            FamilyId::MemoOscillating => FamilySpec {
                name: "memo-oscillating",
                shapes: &[Shape::MemoOscillating],
                coverage: Coverage::EnvelopeOnly {
                    reason: "the memo_* shapes are kernel-seam probes by the board-roster \
                             criterion",
                    decided: REGISTRY_RATIFIED,
                },
                bands: Bands::Unbanded {
                    reason: "the funding argument's control, priced beside the fan-out by \
                             the memo-resolution gate pins in tests/meter.rs",
                    decided: REGISTRY_RATIFIED,
                },
                denominator: "packed cross bytes (the touch currency)",
                closed_form: None,
            },
            FamilyId::MemoChurn => FamilySpec {
                name: "memo-churn",
                shapes: &[Shape::MemoChurn, Shape::MemoChurnId],
                coverage: Coverage::EnvelopeOnly {
                    reason: "the memo_* shapes are kernel-seam probes by the board-roster \
                             criterion",
                    decided: REGISTRY_RATIFIED,
                },
                bands: Bands::Unbanded {
                    reason: "priced by the memo-resolution gate pins in tests/meter.rs, \
                             absolute pins outside the band convention",
                    decided: REGISTRY_RATIFIED,
                },
                denominator: "packed cross bytes (the touch currency)",
                closed_form: None,
            },
            FamilyId::DescendingRaises => FamilySpec {
                name: "descending-raises",
                shapes: &[Shape::DescendingRaises, Shape::DescendingRaisesId],
                coverage: Coverage::EnvelopeOnly {
                    reason: "kernel-seam probe: the decide-then-emit ordering's one \
                             exerciser, priced by the memo-resolution gate pins and the \
                             oracle differential",
                    decided: REGISTRY_RATIFIED,
                },
                bands: Bands::Unbanded {
                    reason: "priced by the memo-resolution gate pins in tests/meter.rs, \
                             absolute pins outside the band convention",
                    decided: REGISTRY_RATIFIED,
                },
                denominator: "packed cross bytes (the touch currency)",
                closed_form: None,
            },
            FamilyId::MaskDrift => FamilySpec {
                name: "mask-drift",
                shapes: &[Shape::MaskDriftTriple, Shape::MaskDriftQuadruple],
                coverage: Coverage::EnvelopeOnly {
                    reason: "operand tuples correlated for the fused three- and \
                             four-stream comparisons alone, which the own_version_cmp rows \
                             run on every board family; a tuple built for one row \
                             signature is a pairing probe, not a shape",
                    decided: REGISTRY_RATIFIED,
                },
                bands: Bands::Priced(&[
                    "masked_cmp_drift_cost_is_flat_per_unit",
                    "masked_pair_cmp_drift_cost_is_flat_per_unit",
                ]),
                denominator: "combined packed tuple bytes",
                closed_form: None,
            },
            FamilyId::MeetShade => FamilySpec {
                name: "meet-shade",
                shapes: &[Shape::MeetShade],
                coverage: Coverage::EnvelopeOnly {
                    reason: "an envelope-suite fold wedge by the board-roster criterion; \
                             the version_meet_all row prices the fold on the rostered \
                             fold populations",
                    decided: REGISTRY_RATIFIED,
                },
                bands: Bands::Priced(&["meet_all_shade_is_flat_per_unit"]),
                denominator: "packed operand bytes of the population",
                closed_form: None,
            },
            FamilyId::ArmingTrain => FamilySpec {
                name: "arming-train",
                shapes: &[Shape::ArmingTrain],
                coverage: Coverage::EnvelopeOnly {
                    reason: "the trains isolate the product tree's level ratio: a \
                             three-point fixed-width design in two sign schedules the \
                             board's single-knob two-scale fit cannot express; the \
                             multi-arming ledger settle itself is board-priced on the \
                             promo-rearm and dense-suffix columns",
                    decided: REGISTRY_RATIFIED,
                },
                bands: Bands::Priced(&[
                    "arming_trains_is_flat_per_unit",
                    "pair_plateau_train_is_flat_per_unit",
                ]),
                denominator: "packed input bytes; three fixed-width points (level ratio, \
                              not a two-scale fit)",
                closed_form: None,
            },
        }
    }
}

/// Bands that price an operation-argument axis or an API seam rather
/// than a registered shape, each with its dated disposition: the
/// registry's answer for band names no family row can carry.
///
/// The board smoke suite's band-name scan (the named parity survivor in
/// the module doc) accepts a scanned band exactly when some family's
/// [`Bands`] roster or this table cites it.
pub const AXIS_BANDS: &[(&str, &str)] = &[
    (
        "ticks_flatness_holds_the_log_band",
        "prices the ticks count axis across three rostered families: an \
         operation-argument axis, not a shape of its own — registry answer of record, \
         2026-07-29",
    ),
    (
        "ticks_wide_count_flatness_holds_the_width_band",
        "prices the ticks count-width axis across the same three rostered families: an \
         operation-argument axis, not a shape of its own — registry answer of record, \
         2026-07-29",
    ),
    (
        "party_fold_alias_rejection_count_is_flat_per_unit",
        "the aliased population probes the fold's hand-back seam through \
         dangerously_alias, not a packed shape (aliases arrive only through decode or \
         dangerously_alias); the board prices the rejection fold on the \
         party_join_all_overlap row — registry answer of record, 2026-07-29",
    ),
    (
        "party_fold_alias_rejection_depth_is_flat_per_unit",
        "as the alias-count band's — registry answer of record, 2026-07-29",
    ),
];
