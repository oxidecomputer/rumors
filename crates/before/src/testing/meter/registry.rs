//! Adversarial input families shared by the resource instruments.
//!
//! Each [`FamilyId`] owns one [`FamilySpec`]. Instruments derive their family
//! axes from this roster, while [`Shape`] provides the registered constructors.
//! Types keep those declarations aligned; tests cover relationships that remain
//! data, such as a family's shape membership.
//!
//!   ```compile_fail,E0603
//!   // Raw constructors are private.
//!   let _ = before::testing::meter::cliff_comb(4, 4);
//!   ```
//!
//!   ```
//!   use before::testing::meter::registry::Shape;
//!   let comb = Shape::CliffComb.build2(4, 4);
//!   assert_eq!(comb.bits, 4 * (2 * 4 + 10) + 2);
//!   ```

#[cfg(test)]
mod tests;

use num_bigint::BigUint;

use super::Encoding;
use crate::Version;

// ─── registered shapes ───────────────────────────────────────────────────────

/// A registered adversarial shape constructor.
///
/// Each variant names its knobs and the accessor it builds through; the full
/// construction derivation (layout, normal-form argument, closed-form size,
/// panics) lives on the private constructor behind it, rendered by the internal
/// documentation build. Every variant is cited by at least one [`FamilyId`]
/// spec, as checked by this module's tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Shape {
    /// The dense event spine `S(d)`: [`Shape::build1`]`(d)`.
    Dense,
    /// The bigroot event `B(b, d)`, a `2^b − 1` root over `S(d)`:
    /// [`Shape::build2`]`(b, d)`.
    Bigroot,
    /// The hugeleaf event, one leaf of value `2^b − 1`:
    /// [`Shape::build1`]`(b)`.
    Hugeleaf,
    /// The boundary comb `C(k, n)`, `n` cliff teeth:
    /// [`Shape::build2`]`(k, n)`.
    CliffComb,
    /// The jump comb `J(k, n)`, one low tooth then `n − 1` cliff teeth:
    /// [`Shape::build2`]`(k, n)`.
    JumpComb,
    /// The wide-tooth comb `W(k, w, n)`, `n` teeth of width `2^w`:
    /// [`Shape::build3`]`(k, w, n)`.
    WideToothComb,
    /// The unpaid-crossing fan `F(k, n)`, `n` cheap teeth under one
    /// stored magnitude: [`Shape::build2`]`(k, n)`.
    CliffFan,
    /// The cancelling-prefix chain `P(k, n)`, `n` peak-to-1 drops:
    /// [`Shape::build2`]`(k, n)`.
    CancellingChain,
    /// The harmonic spine `H(d)`, a 1-leaf at every depth:
    /// [`Shape::build1`]`(d)`.
    Harmonic,
    /// The alternating-binary spine `A(d)`: [`Shape::build1`]`(d)`.
    AltSpine,
    /// The scattered id `Z(e)`, `e` owned fragments at alternating
    /// depths: [`Shape::build1`]`(e)`.
    ScatteredId,
    /// The id spine `I(d, divert)`, a unary chain of depth `d`:
    /// [`Shape::build_flagged`]`(d, divert)`.
    IdSpine,
    /// The nested-full-sibling id `N(d)`: [`Shape::build1`]`(d)`.
    NestedFullId,
    /// The mirror nested-full id `M(d)`: [`Shape::build1`]`(d)`.
    NestedLeftFullId,
    /// The wide-tail event, a zero-leaf spine with one `2^b − 1` tail:
    /// [`Shape::build2`]`(b, d)`.
    WideTail,
    /// The descending staircase `D(d)`: [`Shape::build1`]`(d)`.
    Staircase,
    /// The memo-chain event `Q(k, distinct)`:
    /// [`Shape::build_flagged`]`(k, distinct)`.
    MemoChain,
    /// The memo-chain id: [`Shape::build1`]`(k)`.
    MemoChainId,
    /// The memo-comb event `B(d)`: [`Shape::build1`]`(d)`.
    MemoComb,
    /// The memo-comb id: [`Shape::build1`]`(d)`.
    MemoCombId,
    /// The memo fan-out event `F(k, b)`: [`Shape::build2`]`(k, b)`.
    MemoFanout,
    /// The oscillating-siblings event `O(k, b)`:
    /// [`Shape::build2`]`(k, b)`.
    MemoOscillating,
    /// The memo-churn event `U(d)`: [`Shape::build1`]`(d)`.
    MemoChurn,
    /// The memo-churn id: [`Shape::build1`]`(d)`.
    MemoChurnId,
    /// The descending-raises event `W(d)`: [`Shape::build1`]`(d)`.
    DescendingRaises,
    /// The descending-raises id: [`Shape::build1`]`(d)`.
    DescendingRaisesId,
    /// The reveal-comb event `R(k, b)`: [`Shape::build2`]`(k, b)`.
    RevealComb,
    /// The reveal comb with its floor raised to `2^b − 2` (the gap
    /// control): [`Shape::build2`]`(k, b)`.
    RevealCombHifloor,
    /// The reveal-comb id: [`Shape::build1`]`(k)`.
    RevealCombId,
    /// The pure-comb event `L(k, b)`: [`Shape::build2`]`(k, b)`.
    PureComb,
    /// The pure-comb id: [`Shape::build1`]`(k)`.
    PureCombId,
    /// The ascending cliff `A(k, b)`: [`Shape::build2`]`(k, b)`.
    AscendCliff,
    /// The ascending cliff with every wide leaf leveled (the
    /// hop-schedule control): [`Shape::build2`]`(k, b)`.
    AscendCliffPlateau,
    /// The ascending-cliff id: [`Shape::build1`]`(k)`.
    AscendCliffId,
    /// The dominated-undercut spine `DU(k, b)`: [`Shape::build2`]`(k, b)`.
    DominatedUndercut,
    /// The dominated-undercut id: [`Shape::build1`]`(k)`.
    DominatedUndercutId,
    /// The seam-plunge spine `SP(k, r)`: [`Shape::build2`]`(k, r)`.
    SeamPlunge,
    /// The seam-plunge control (the ascent kept, the plunge leveled away):
    /// [`Shape::build2`]`(k, r)`.
    SeamPlungeControl,
    /// The seam-stop spine `SS(k)`: [`Shape::build1`]`(k)`.
    SeamStop,
    /// The seam-stop control (the descent kept, the stacked boundary
    /// removed): [`Shape::build1`]`(k)`.
    SeamStopControl,
    /// The latent-ladder comb `LL(w, k)`: [`Shape::build2`]`(w, k)`.
    LatentLadder,
    /// The freeze-position spine `FP(k)`: [`Shape::build1`]`(k)`.
    FreezePosition,
    /// The promotion re-arm spine `PR(p)`: [`Shape::build1`]`(p)`.
    PromotionRearm,
    /// The promotion re-arm mate `PRM(p)`, the small twin:
    /// [`Shape::build1`]`(p)`.
    PromotionRearmMate,
    /// The dense-suffix re-arm family `DS(p, d)`:
    /// [`Shape::build2`]`(p, d)`.
    DenseSuffix,
    /// The dense-suffix mate `DSM(p, d)`, the unit twin:
    /// [`Shape::build2`]`(p, d)`.
    DenseSuffixMate,
    /// The wide-arming family `WA(w, d)`: [`Shape::build2`]`(w, d)`.
    WideArming,
    /// The hoisted-window family `HW(w, d, t)`:
    /// [`Shape::build3`]`(w, d, t)`.
    HoistedWindow,
    /// The weight-comb family `WC(n)`: [`Shape::build1`]`(n)`.
    WeightComb,
    /// The freeze-parade family `FZ(k)`: [`Shape::build1`]`(k)`.
    FreezeParade,
    /// The lone-freeze spine `LF(pre, post)`:
    /// [`Shape::build2`]`(pre, post)`.
    LoneFreeze,
    /// The tooth-tail pair `TT(g, m)`: [`Shape::build_pair`]`(g, m)`.
    ToothTail,
    /// The puncture-product embedding `V(x, y)` over arbitrary factors:
    /// [`Shape::build_product`]`(&x, &y)`.
    PunctureProduct,
    /// The plateau-puncture family `PP(w, d)` over its committed
    /// factors: [`Shape::build2`]`(w, d)`.
    PlateauPuncture,
    /// The arming-train family `AT(n, w, g, alternate)`:
    /// [`Shape::build_train`]`(n, w, g, alternate)`.
    ArmingTrain,
    /// The two-operand jump comb `JP(k, m, d)`:
    /// [`Shape::build_pair3`]`(k, m, d)`.
    JumpPair,
    /// The concurrent pair `CP(n)`, two organically built versions:
    /// [`Shape::version_pair`]`(n)`.
    ConcurrentPair,
    /// The staggered-comb fold operand `SG(n, m, i)`:
    /// [`Shape::build3`]`(n, m, i)`.
    StaggerComb,
    /// The staggered id `SI(n, m, i)`: [`Shape::build3`]`(n, m, i)`.
    StaggerId,
    /// The staggered fold population, all `n` operands in bit-reversed
    /// feed order: [`Shape::population`]`(n, m)`.
    StaggerPopulation,
    /// The meet-shade population `MS(d, k)`: [`Shape::versions`]`(d, k)`.
    MeetShade,
    /// The masked-comparison correlated triple `MT(k, n)`:
    /// [`Shape::build_triple`]`(k, n)`.
    MaskDriftTriple,
    /// The masked-comparison correlated quadruple `MQ(k, n)`:
    /// [`Shape::build_quadruple`]`(k, n)`.
    MaskDriftQuadruple,
    /// The collapse-hole pair `CH(k, m)`: [`Shape::build_pair`]`(k, m)`.
    CollapseHole,
    /// The copy-hole pair `CO(k, m)`: [`Shape::build_pair`]`(k, m)`.
    CopyHole,
    /// The raise-hole pair `RH(k, m)`: [`Shape::build_pair`]`(k, m)`.
    RaiseHole,
    /// The site-hole pair `SH(k, m)`: [`Shape::build_pair`]`(k, m)`.
    SiteHole,
    /// The masked-hole triple `MH(d, h)`: [`Shape::build_triple`]`(d, h)`.
    MaskedHoleTriple,
}

/// A registered constructor's signature class, binding one [`Shape`] to
/// the private generator behind it.
// The signatures are the generators' own; an alias per tuple shape would
// be indirection to track, not documentation.
#[allow(clippy::type_complexity)]
enum Builder {
    /// One size knob to one encoded shape.
    P1(fn(usize) -> Encoding),
    /// Two size knobs to one encoded shape.
    P2(fn(usize, usize) -> Encoding),
    /// Three size knobs to one encoded shape.
    P3(fn(usize, usize, usize) -> Encoding),
    /// A size knob and a variant flag to one encoded shape.
    Flag(fn(usize, bool) -> Encoding),
    /// The arming-train signature: three knobs and a sign schedule.
    Train(fn(usize, usize, usize, bool) -> Encoding),
    /// Two arbitrary factors to one encoded shape.
    Product(fn(&BigUint, &BigUint) -> Encoding),
    /// Two size knobs to a geometrically coupled encoded pair.
    Pair2(fn(usize, usize) -> (Encoding, Encoding)),
    /// Three size knobs to a geometrically coupled encoded pair.
    Pair3(fn(usize, usize, usize) -> (Encoding, Encoding)),
    /// One size knob to an organically built version pair.
    VersionPair(fn(usize) -> (Version, Version)),
    /// Two size knobs to a fold population of versions.
    Versions2(fn(usize, usize) -> Vec<Version>),
    /// Two size knobs to a fold population of (versions, ids).
    Population2(fn(usize, usize) -> (Vec<Encoding>, Vec<Encoding>)),
    /// Two size knobs to a correlated operand triple.
    Triple2(fn(usize, usize) -> (Encoding, Encoding, Encoding)),
    /// Two size knobs to a correlated operand quadruple.
    Quad2(fn(usize, usize) -> ((Encoding, Encoding), (Encoding, Encoding))),
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
            Shape::DominatedUndercut => Builder::P2(super::dominated_undercut),
            Shape::DominatedUndercutId => Builder::P1(super::dominated_undercut_id),
            Shape::SeamPlunge => Builder::P2(super::seam_plunge),
            Shape::SeamPlungeControl => Builder::P2(super::seam_plunge_control),
            Shape::SeamStop => Builder::P1(super::seam_stop),
            Shape::SeamStopControl => Builder::P1(super::seam_stop_control),
            Shape::LatentLadder => Builder::P2(super::latent_ladder),
            Shape::FreezePosition => Builder::P1(super::freeze_position),
            Shape::PromotionRearm => Builder::P1(super::promotion_rearm),
            Shape::PromotionRearmMate => Builder::P1(super::promotion_rearm_mate),
            Shape::DenseSuffix => Builder::P2(super::dense_suffix),
            Shape::DenseSuffixMate => Builder::P2(super::dense_suffix_mate),
            Shape::WideArming => Builder::P2(super::wide_arming),
            Shape::HoistedWindow => Builder::P3(super::hoisted_window),
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
            Shape::CollapseHole => Builder::Pair2(super::collapse_hole),
            Shape::CopyHole => Builder::Pair2(super::copy_hole),
            Shape::RaiseHole => Builder::Pair2(super::raise_hole),
            Shape::SiteHole => Builder::Pair2(super::site_hole),
            Shape::MaskedHoleTriple => Builder::Triple2(super::masked_hole),
        }
    }

    /// The accessor-mismatch failure: a shape asked to build through a
    /// signature its constructor does not have.
    fn wrong_builder(self, called: &str) -> ! {
        panic!("{self:?} does not build through {called}: its variant doc names its accessor")
    }

    /// Build a one-knob encoded shape.
    ///
    /// # Panics
    ///
    /// Panics if this shape's constructor takes a different signature,
    /// or on the constructor's own knob preconditions.
    pub fn build1(self, a: usize) -> Encoding {
        match self.builder() {
            Builder::P1(f) => f(a),
            _ => self.wrong_builder("build1"),
        }
    }

    /// Build a two-knob encoded shape.
    ///
    /// # Panics
    ///
    /// Panics if this shape's constructor takes a different signature,
    /// or on the constructor's own knob preconditions.
    pub fn build2(self, a: usize, b: usize) -> Encoding {
        match self.builder() {
            Builder::P2(f) => f(a, b),
            _ => self.wrong_builder("build2"),
        }
    }

    /// Build a three-knob encoded shape.
    ///
    /// # Panics
    ///
    /// Panics if this shape's constructor takes a different signature,
    /// or on the constructor's own knob preconditions.
    pub fn build3(self, a: usize, b: usize, c: usize) -> Encoding {
        match self.builder() {
            Builder::P3(f) => f(a, b, c),
            _ => self.wrong_builder("build3"),
        }
    }

    /// Build a knob-and-flag encoded shape.
    ///
    /// # Panics
    ///
    /// Panics if this shape's constructor takes a different signature,
    /// or on the constructor's own knob preconditions.
    pub fn build_flagged(self, a: usize, flag: bool) -> Encoding {
        match self.builder() {
            Builder::Flag(f) => f(a, flag),
            _ => self.wrong_builder("build_flagged"),
        }
    }

    /// Build the arming-train signature: three knobs and a sign schedule.
    ///
    /// # Panics
    ///
    /// Panics if this shape's constructor takes a different signature,
    /// or on the constructor's own knob preconditions.
    pub fn build_train(self, n: usize, w: usize, g: usize, alternate: bool) -> Encoding {
        match self.builder() {
            Builder::Train(f) => f(n, w, g, alternate),
            _ => self.wrong_builder("build_train"),
        }
    }

    /// Build a shape from two arbitrary factors.
    ///
    /// # Panics
    ///
    /// Panics if this shape's constructor takes a different signature,
    /// or on the constructor's own factor preconditions.
    pub fn build_product(self, x: &BigUint, y: &BigUint) -> Encoding {
        match self.builder() {
            Builder::Product(f) => f(x, y),
            _ => self.wrong_builder("build_product"),
        }
    }

    /// Build a geometrically coupled two-knob encoded pair.
    ///
    /// # Panics
    ///
    /// Panics if this shape's constructor takes a different signature,
    /// or on the constructor's own knob preconditions.
    pub fn build_pair(self, a: usize, b: usize) -> (Encoding, Encoding) {
        match self.builder() {
            Builder::Pair2(f) => f(a, b),
            _ => self.wrong_builder("build_pair"),
        }
    }

    /// Build a geometrically coupled three-knob encoded pair.
    ///
    /// # Panics
    ///
    /// Panics if this shape's constructor takes a different signature,
    /// or on the constructor's own knob preconditions.
    pub fn build_pair3(self, a: usize, b: usize, c: usize) -> (Encoding, Encoding) {
        match self.builder() {
            Builder::Pair3(f) => f(a, b, c),
            _ => self.wrong_builder("build_pair3"),
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
            _ => self.wrong_builder("version_pair"),
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
            _ => self.wrong_builder("versions"),
        }
    }

    /// Build a fold population of (versions, ids), in feed order.
    ///
    /// # Panics
    ///
    /// Panics if this shape's constructor takes a different signature,
    /// or on the constructor's own knob preconditions.
    pub fn population(self, a: usize, b: usize) -> (Vec<Encoding>, Vec<Encoding>) {
        match self.builder() {
            Builder::Population2(f) => f(a, b),
            _ => self.wrong_builder("population"),
        }
    }

    /// Build a correlated operand triple.
    ///
    /// # Panics
    ///
    /// Panics if this shape's constructor takes a different signature,
    /// or on the constructor's own knob preconditions.
    pub fn build_triple(self, a: usize, b: usize) -> (Encoding, Encoding, Encoding) {
        match self.builder() {
            Builder::Triple2(f) => f(a, b),
            _ => self.wrong_builder("build_triple"),
        }
    }

    /// Build a correlated operand quadruple.
    ///
    /// # Panics
    ///
    /// Panics if this shape's constructor takes a different signature,
    /// or on the constructor's own knob preconditions.
    pub fn build_quadruple(
        self,
        a: usize,
        b: usize,
    ) -> ((Encoding, Encoding), (Encoding, Encoding)) {
        match self.builder() {
            Builder::Quad2(f) => f(a, b),
            _ => self.wrong_builder("build_quadruple"),
        }
    }
}

// ─── the family roster ───────────────────────────────────────────────────────

/// One adversarial input family on the amplification board.
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
    /// The harmonic spine `H(d)`: the rank fold's wide-numerator adversary,
    /// designed against the linear-functional rows and the rank pair.
    Harmonic,
    /// The scatter-ordered population: balanced-forked single-tick operands
    /// whose join accumulator never coalesces.
    Scatter,
    /// The weave fold population: the leaves of one balanced fork tree dealt
    /// round-robin among 16 group parties (the board's weave-group constant),
    /// one tick each.
    ///
    /// Every operand is individually benign — an organic region set any
    /// retire/reunite call site could hold — while every internal node of the
    /// shared upper skeleton is both-present in every operand pair, so the
    /// fold's per-node costs that scale with the *other* operand (the overlap
    /// test against the accumulator, the join merges over interleaved trees)
    /// dominate. Scatter cannot reach this genre (its operands are single
    /// leaves) and benign reaches it only diluted; the arity is fixed so the
    /// scaling axis is both-present richness alone.
    Weave,
    /// The staggered fold population ([`Shape::StaggerPopulation`]): `n`
    /// operands of `m` unit teeth each, every operand's teeth landing in the
    /// gaps of every other's, fed in bit-reversed order.
    ///
    /// The correlated-population loading of the balanced reduction itself: the
    /// feed order pairs operands whose slot addresses diverge at the top bit,
    /// so every internal merge — at every level — joins region sets that
    /// interleave maximally and swell to near the sum of their sizes, the
    /// intermediate-swell worst case of the declared `O(D log k)` fold model,
    /// held until the last level (the full union collapses to the constant-1
    /// skyline on the version side, the whole seed region on the id side).
    /// Scatter scales arity at single-leaf operands and weave scales operand
    /// size at fixed arity; this population scales both, and its bit-reversed
    /// feed forecloses the adjacent-slot coalescing luck index order would hand
    /// the counter.
    Stagger,
    /// The staggered population with growing operand count and fixed operand
    /// size, isolating the balanced fold's arity axis.
    StaggerArity,
    /// The staggered population with fixed operand count and growing operand
    /// size, isolating the balanced fold's per-operand size axis.
    StaggerSize,
    /// The nested-full-sibling cross `N(d)` × the dense spine `S(d)`.
    ///
    /// Every level a right-full shortcut site, the deepest stacking of the
    /// walk's deferred right-full decisions and raise bookkeeping on narrow
    /// values — the designated cross of the two tick rows.
    NestedFull,
    /// The wide right-full cross: `bigroot(b, d)` × `N(d)`.
    ///
    /// The stream's first payload is coded absolute, so the deepest subtree's
    /// net movement carries the root's full magnitude and every level's
    /// bookkeeping meets it — width × depth through the right-full arm. The
    /// designated cross of the two tick rows.
    NestedWide,
    /// The wide left-full (memo) cross: `wide_tail(b, d)` × `M(d)`.
    ///
    /// Every proper subtree nets the tail's full magnitude while every level is
    /// a memoized pre-scan site — width × depth through the left-full arm and
    /// the pre-scan's own chains. The designated cross of the two tick rows.
    MirrorWide,
    /// The narrow left-full (memo) cross: `wide_tail(1, d)` × `M(d)`.
    ///
    /// The memoized pre-scan machinery itself, all values word-scale. The
    /// designated cross of the two tick rows.
    MirrorNarrow,
    /// The descending staircase `D(d)` × the unary id spine `I(d)`.
    ///
    /// Every consumed leaf undercuts every open range's minimum —
    /// full-penetration minimum updates at every level, all values word-scale.
    /// The designated cross of the two tick rows.
    Staircase,
    /// The reveal-comb cross: `reveal_comb(s, s)` × its own id.
    ///
    /// `s` sibling left-full sites share one `2^s`-wide minimum over a zero
    /// floor, and the left-leaning spine closes each site's frame back into the
    /// floor frame between consecutive consumes: the width-`s` boundary
    /// difference is created at every consume and popped at every close. The
    /// gate pins its touch cost in `tests/meter.rs`. The designated cross of
    /// the two tick rows.
    RevealComb,
    /// The reveal-comb control: `reveal_comb_hifloor(s, s)` × the
    /// reveal-comb id.
    ///
    /// Identical forest and close-reveal cycle with the floor raised to `2^s −
    /// 2`, so the circulated boundary difference is O(1) wide: the gap control.
    /// The designated cross of the two tick rows.
    RevealHifloor,
    /// The pure-comb cross: `pure_comb(s, s)` × its own id.
    ///
    /// The reveal comb's cycle with no left-full site anywhere — no memo, no
    /// pre-scan, no site consume: the range-minimum stack's own arm-move +
    /// close-pop width circulation, isolated from the frame ledger. The
    /// designated cross of the two tick rows.
    PureComb,
    /// The ascending-cliff cross: `ascend_cliff(s, s)` × its own id.
    ///
    /// `s` ascending wide leaves stack `s − 1` nonzero unit boundary
    /// differences and a terminal 0-cliff drives one width-`s` undercut residue
    /// through all of them — the cascade whose per-hop fold direction the gate
    /// pins in `tests/meter.rs` price in the touch currency these columns do
    /// not carry. The designated cross of the two tick rows.
    AscendCliff,
    /// The ascending-cliff control: `ascend_cliff_plateau(s, s)` × the
    /// ascending-cliff id.
    ///
    /// Identical spine, arming schedule, and cliff undercut with every leaf
    /// leveled, so the difference stack is one compressed zero run the residue
    /// passes whole in O(1): the hop-schedule control. The designated cross of
    /// the two tick rows.
    AscendPlateau,
    /// The dominated-undercut cross: `dominated_undercut(s, s)` × its own id.
    ///
    /// Each of `s` raise sites re-arms the range-minimum stack at the top of a
    /// `5·2^s`-scale climb and then emits its copied region's block minimum
    /// from one word above it — a no-latent, word-scale-offset emission
    /// against a wide-negative anchor gap. The board charges the resulting
    /// arithmetic and traversal work to the family's input. The designated
    /// cross of the two tick rows.
    DominatedUndercut,
    /// The two-operand jump comb `jump_pair(k, m, d)`: wide height-difference
    /// crests over a dense-position spine.
    ///
    /// The overlay interleaves one operand's wide teeth with the other's cheap
    /// codes, so the pair rows park wide drift at the other operand's
    /// boundaries `2m` times while every absolute position stays `d` digits
    /// dense — the shape that separates segment-anchored freeze accounting
    /// (flat) from absolute-position accounting (superlinear), with each
    /// operand certified-linear alone (the generator doc carries the
    /// mechanism).
    JumpPair,
    /// The freeze-position spine `freeze_position(s)`: the many-freezes
    /// sentinel.
    ///
    /// `2s` descending wide leaves alternate a ten-digit drop and a unit drop
    /// down a right spine, so a numeric fold freezes `Θ(s)` times at ever-deeper
    /// stream positions — every comb fires O(1) freezes, which was exactly the
    /// coverage hole — and any freeze accounting that reads an absolute
    /// position (or any whole-history state) per freeze goes quadratic here
    /// while the family's positions compact to O(1) digits. The
    /// freeze-position band requires the anchored-segment implementation to
    /// remain flat. Designed against the numeric-measure rows.
    FreezePos,
    /// The promotion re-arm spine `promotion_rearm(s)`: the many-armings
    /// sentinel.
    ///
    /// `32s` span-building levels grow the consumed mass's written span, then
    /// `s` four-node blocks each park a wide drift and promote it at a narrow
    /// one — `Θ(s)` numeric-fold promotions at O(1) stored codes each, where
    /// every comb promotes never and the freeze-position spine's parked drift
    /// is monotone. Any promotion accounting that re-reads whole-history state
    /// per arming goes quadratic here while the family's suffix masses compact
    /// to O(1) balanced terms. The promotion re-arm bands require the ledger's
    /// cost to remain flat. Designed against the numeric-measure rows.
    PromoRearm,
    /// The weight-comb spine `weight_comb(n)`: the many-jumps sentinel.
    ///
    /// A depth-`32n` parked-unit spine, then `2n` shallow leaves oscillating
    /// heights 0 and 2: the rank integral deposits the oscillation at one digit
    /// position `Θ(n)` digits above the parked unit for O(1) stored bits per
    /// event — the position weight is topology, so no code funds the gap — and
    /// every cancellation makes the accumulator's top settle back across the
    /// never-written run. A settlement scan that steps the gap digit by digit
    /// goes quadratic here (demonstrated by a probe build with certificate
    /// consumption disabled); consuming one
    /// zero-run certificate per jumped run reads flat (the `skyline_flatness`
    /// weight-comb band). Designed against the numeric-measure rows.
    WeightComb,
    /// The freeze-parade spine `freeze_parade(k)`: the deep-segment freeze
    /// sentinel.
    ///
    /// The parked-unit spine at depth `64k`, then `k` shallow freeze blocks
    /// whose wide in-pair drops each fire one numeric-fold freeze at the block's
    /// position weight, `Θ(k)` digits above digit 0, so every freeze's scaled
    /// segment read starts `Θ(k)` digits up. The accumulator's write watermark
    /// prices each read at the segment's written span; a scaled read that
    /// starts at digit 0 walks the never-written prefix per freeze and goes
    /// quadratic in accumulator touches; the watermark stays flat in the
    /// `skyline_flatness` freeze-parade band. The freeze-position spine prices the numeric fold's
    /// per-freeze accounting; this family prices the accumulator's read side
    /// under the same schedule. Designed against the numeric-measure rows.
    FreezeParade,
    /// The dense-suffix pair `dense_suffix(p, p)` against its unit mate
    /// `dense_suffix_mate(p, p)`: the many-armings × dense-trailing-mass
    /// sentinel.
    ///
    /// A gap spine holds the trailing interval mass at `Θ(p)` balanced digits,
    /// then `p` re-arm blocks each park a wide drift and promote it at O(1)
    /// stored codes — `Θ(p)` ledger armings all owing their debt across the
    /// same `Θ(p)`-dense trailing mass, so a settle that walks the suffix once
    /// per arming (or re-reads a promoted prefix once per window) goes
    /// quadratic here while the mass-balanced product tree charges every
    /// arming-window cross term inside one aggregate product and reads flat.
    /// The mate is the same topology at unit bases, and the wide operand dominates
    /// it pointwise, so the pair rows run the co-sweep whose freezes and
    /// promotions fire on drift only the wide operand deposited (the
    /// `skyline_flatness` dense-suffix rank and distance bands carry the
    /// enforcement). Designed against the numeric-measure rows.
    DenseSuffix,
    /// Combines one wide counter change with a dense suffix.
    ///
    /// Both grow linearly with `s`, exposing implementations that repeatedly
    /// process the wide value for every suffix element.
    WideArming,
    /// The plateau-puncture family `plateau_puncture(s, s)`: the
    /// answer-embedded-product sentinel, and the floor under every settle.
    ///
    /// Every turn leaf sits on one incompressible pseudorandom plateau `x` of
    /// `Θ(s)` digits and the turn positions spell a jittered punctured mass `y`
    /// of `Θ(s)` isolated digits, so the exact rank embeds the integer product
    /// `2·x·y + 1` — bought with `Θ(s)` input bits, both factors' content
    /// beyond the settle's own balanced-digit compaction. No promotion ever
    /// fires; the cost is the close-time settle, one wide × dense
    /// multiplication run inside the backend at its bound `M(|v|)` — and
    /// because the same constructor embeds the product of arbitrary factors,
    /// any fold that answers exactly multiplies arbitrary input-funded
    /// integers, so `Ω(M(|v|))` floors every settle.
    /// Designed against the numeric-measure rows.
    PlateauPuncture,
    /// The lone-freeze spine `lone_freeze(s, s)`: the first-freeze gate
    /// straddle, both sides on one knob.
    ///
    /// `s` unit-oscillation pairs ride a wide plateau strictly before the
    /// sweep's one freeze-firing drop, and `s` more run behind it with the gate
    /// open and a ten-digit drift parked — so any per-interval deposit toward
    /// the settle machinery made before drift exists to settle scales with the
    /// prefix, and a segment feed or close read that is not amortized O(1) per
    /// interval scales with the tail, while the family's funded wide codes stay
    /// O(1). Exactly one freeze and no promotion ever fires, so the column also
    /// prices the settle's smallest nonempty configuration. The
    /// `skyline_flatness` lone-freeze bands isolate each axis at the generator
    /// minimum and carry the enforcement; the column scales both together.
    /// Designed against the numeric-measure rows.
    LoneFreeze,
    /// The concurrent pair `concurrent_pair(n)`: the emit side-switch density
    /// population.
    ///
    /// Organically forked and ticked so the sweep's side switch fires at every
    /// one of the `n − 1` overlay boundaries, join and meet alike — the pairing
    /// the ticked counterpart cannot reach.
    ConcurrentPair,
    /// The tooth-tail pair `tooth_tail(g, m)`: the boundary-aligned exact-`top`
    /// population.
    ///
    /// Two same-shape unit chains whose second leaves spike `2^(32g)` in both
    /// operands, `b` one tick above `a` everywhere except the shared terminal:
    /// the pair sweep folds both spikes into one cancelling difference at the
    /// same boundary, then reads `sign(D)` once per remaining boundary with no
    /// intervening write. Exact-`top` maintenance prices each read at the
    /// settled value's own width; a high-water bound re-walks the spike's `g`
    /// dead digits per read — `Θ(m·g)` on `Θ(m + g)` input (the
    /// `skyline_flatness` tooth-tail band carries both readings). Every overlay
    /// boundary is shared by both operands and almost every stored delta is
    /// zero, so the pair is also the touch floor's honest-less-work witness
    /// (the board floor module's `touch_pair_fold`): a conforming sweep is
    /// forced to fold only the three nonzero deltas per operand, and the
    /// measured per-boundary sign-read traffic sits far above that floor as
    /// implementation, never mandate.
    ToothTail,
    /// The fixed-seed organic control population.
    Benign,
    /// The wide-tooth comb `W(k, w, n)`: bounded wide oscillation — height
    /// state that must *stay* live across a fixed-width window.
    WideToothComb,
    /// The jump comb `J(k, n)`: the stale-drift eviction probe — height state
    /// that must *leave* the cheap-delta path exactly once.
    JumpComb,
    /// The unpaid-crossing fan `F(k, n)`: `n` sibling carry excursions funded
    /// by one stored magnitude.
    CliffFan,
    /// The cancelling-prefix chain `P(k, n)`: deep sign scans funded by the
    /// wide writes that immediately precede them.
    CancellingChain,
    /// The alternating-binary spine `A(d)`: the frame-count adversary for
    /// iterative walks that keep per-level records.
    AltSpine,
    /// The memo-chain pair `Q(k, distinct)` × its party: `k` sibling sites
    /// whose precomputed minima are either distinct or all equal.
    MemoChain,
    /// The memo-comb pair `B(d)` × its party: nested sites that finish their
    /// minima in a different order from the tick walk's consumption order.
    MemoComb,
    /// The memo fan-out `F(k, b)`: `k` sites share one `b`-bit minimum that the
    /// input stores once, exposing any implementation that copies it per site.
    MemoFanout,
    /// The oscillating siblings `O(k, b)`: every precomputed difference is
    /// `b` bits wide and each one is represented in the input.
    MemoOscillating,
    /// The memo-churn pair `U(d)` × its party: a descending run repeatedly
    /// lowers the minimum while `d` outer sites await their results.
    MemoChurn,
    /// The descending raises `W(d)` × its party: each site's requested raise
    /// falls below the minimum established by its sibling range.
    DescendingRaises,
    /// The masked-comparison correlated tuples `MT(k, n)` / `MQ(k, n)`: operand
    /// pairings built for the fused three- and four-stream comparison walks
    /// alone.
    MaskDrift,
    /// The meet-shade population `MS(d, k)`: one deep carrier under `k − 1`
    /// dominating plateau shades, the meet fold's wedge.
    MeetShade,
    /// The arming-train family `AT(n, w, g, alternate)`: the product tree's
    /// level-ratio probe, three fixed-width points in two sign schedules.
    ArmingTrain,
    /// A deep collapsing range crossed by one tick traversal.
    CollapseHole,
    /// A deep unchanged range copied by one tick traversal.
    CopyHole,
    /// A deep range whose accumulated value is raised by one tick traversal.
    RaiseHole,
    /// A deep party site crossed by one tick traversal.
    SiteHole,
    /// The masked-hole triple `MH(d, h)`: a deep dense spine under a
    /// shallow diverted mask against a dominating plateau.
    ///
    /// The fused three-stream comparison's depth-independence adversary:
    /// the mask leaves the spine's whole continuation below depth `h` as
    /// one unowned run whose boundaries no other cursor crosses, so the
    /// masked walk's block skip must consume it whole and the accumulator
    /// work reads flat across a spine-depth doubling — the shape a
    /// per-boundary walk cannot survive.
    MaskedHole,
    /// The hoisted-window family `HW(w, d, t)`: the wide-arming close with a
    /// dense tail hoisting the settle clusters' absolute positions.
    ///
    /// The settle's densify seam routes each balanced-digit cluster through
    /// two zero-filled byte images sized by the cluster's *span*; the worst
    /// artifact green on every width and touch counter is an image sized by
    /// the cluster's absolute digit *position* — zero fill that scales with
    /// depth, invisible because a zeroed byte no digit lands on enters no
    /// operand width and touches no accumulator digit. The tail knob moves
    /// exactly that axis: it hoists the trailing window's cluster positions
    /// by `~t/32` digits while every span, width, freeze, and window density
    /// stays put, so span-priced densification reads flat across a tail
    /// doubling and position-priced densification scales with the knob (the
    /// `hoisted_window` band in `tests/meter.rs` prices the family). Designed
    /// against the pair-integral settle's densified cluster allocation.
    HoistedWindow,
    /// The propagate-seam family: the range-minimum tracker's wide-hop domination
    /// guards at their clearance line, both arms.
    ///
    /// An undercut's residue penetrates the difference stack by top-index
    /// domination: a hop with two digits of clearance is decided before any
    /// fold, and the dying side — whichever it is — folds once at its own
    /// width. Every other committed cascade sits far from that line: the
    /// ascending cliff's dying differences are word-scale (one digit) under
    /// a residue dozens wide, and the dominated-undercut sites annihilate
    /// exactly. Four shapes: the plunge (`SP(k, r)`, `k` three-digit
    /// boundaries dying into one `r`-digit residue at exactly `r = 5`'s
    /// minimal clearance, the `r` knob moving the clearance alone) and its
    /// leveled control, the stop (`SS(k)`, `k` three-digit residues dying
    /// against one five-digit surviving boundary) and its unstacked control
    /// — each control isolating the hops by wire-near-identical run
    /// difference. What each side enforces differs, deliberately: a
    /// clearance regression reroutes the *plunge*'s hops onto the
    /// comparable-scale fold, whose per-hop cost is the residue's width
    /// instead of the dying boundary's — the plunge bands trip — while the
    /// *stop*'s rerouted hop is touch-identical to its guarded arm (the
    /// survivor's top digit decides its sign fold at one touch either way),
    /// so the stop band's charge is width conservation alone: the surviving
    /// boundary is never read across its width while it survives, and the
    /// arm's value flow rides the closed form. The descending- and
    /// stopping-boundary bands price the family.
    PropagateSeam,
    /// The latent-ladder family `LL(w, k)`: the parked-latent undercut
    /// decision's O(1) claim, on the axis that would falsify it.
    ///
    /// A drop below a stale anchor while a latent boundary is parked is
    /// decided against the true minimum by top-index domination
    /// (`decide_undercut_through_latent`), documented O(1) for
    /// scale-disparate operands. The shape parks one `w`-digit boundary and
    /// then presents `k` word-scale drops just under the anchor — `k`
    /// dominating-latent reads whose only wide operand is the one the
    /// decision must not read across — so the per-decision marginal cost
    /// (the `k`-marginal at fixed `w`) is the claim's meter, and a decision
    /// that reads the latent's width doubles that marginal when `w`
    /// doubles. The `latent_ladder` band in `tests/meter.rs` prices it.
    LatentLadder,
}

/// One family's row of record: the answers every instrument derives
/// from.
#[derive(Debug, Clone, Copy)]
pub struct FamilySpec {
    /// The name used in board output.
    pub name: &'static str,
    /// Constructors that belong to this family.
    pub shapes: &'static [Shape],
    /// Number of operation rows reached by this family's operand bundle.
    pub cells: usize,
}

/// Optional deserialization rows reached by each available encoded value.
const DESERIALIZATION_FORMATS: usize =
    cfg!(feature = "serde") as usize + cfg!(feature = "borsh") as usize;

/// Board rows reached by a family with version operands.
const VERSION_BUNDLE_CELLS: usize = 76 + 5 * DESERIALIZATION_FORMATS;

/// Board rows reached by a family with party operands.
const PARTY_BUNDLE_CELLS: usize = 37 + 2 * DESERIALIZATION_FORMATS;

/// Board rows reached by a family with version, party, and clock operands.
const CROSS_BUNDLE_CELLS: usize = 95 + 6 * DESERIALIZATION_FORMATS;

/// Board rows reached by a family with every operand bundle.
const FULL_BUNDLE_CELLS: usize = 112 + 6 * DESERIALIZATION_FORMATS;

/// Board rows reached by a population family.
const POPULATION_BUNDLE_CELLS: usize = 25;

/// Board rows reached by scatter, including correlated query witnesses.
const SCATTER_BUNDLE_CELLS: usize = POPULATION_BUNDLE_CELLS + 4;

impl FamilyId {
    /// Every registered family in board render order.
    pub const ALL: [FamilyId; 57] = [
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
        FamilyId::StaggerArity,
        FamilyId::StaggerSize,
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
        FamilyId::DominatedUndercut,
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
        FamilyId::CollapseHole,
        FamilyId::CopyHole,
        FamilyId::RaiseHole,
        FamilyId::SiteHole,
        FamilyId::MaskedHole,
        FamilyId::HoistedWindow,
        FamilyId::PropagateSeam,
        FamilyId::LatentLadder,
    ];

    /// This family's position in [`FamilyId::ALL`] — the roster-order tie the
    /// registry tests hold against the array, so a variant cannot be declared
    /// without joining the roster at a committed position.
    pub const fn index(self) -> usize {
        self as usize
    }

    /// The amplification board's family axis, in render order.
    pub fn board() -> impl Iterator<Item = FamilyId> {
        FamilyId::ALL.into_iter()
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
                cells: VERSION_BUNDLE_CELLS,
            },
            FamilyId::Bigroot => FamilySpec {
                name: "bigroot",
                shapes: &[Shape::Bigroot],
                cells: VERSION_BUNDLE_CELLS,
            },
            FamilyId::Hugeleaf => FamilySpec {
                name: "hugeleaf",
                shapes: &[Shape::Hugeleaf],
                cells: VERSION_BUNDLE_CELLS,
            },
            FamilyId::Cliff => FamilySpec {
                name: "cliff",
                shapes: &[Shape::CliffComb],
                cells: VERSION_BUNDLE_CELLS,
            },
            FamilyId::IdPair => FamilySpec {
                name: "id-pair",
                shapes: &[Shape::IdSpine],
                cells: PARTY_BUNDLE_CELLS,
            },
            FamilyId::CombScatter => FamilySpec {
                name: "comb-scatter",
                shapes: &[Shape::CliffComb, Shape::ScatteredId],
                cells: CROSS_BUNDLE_CELLS,
            },
            FamilyId::Harmonic => FamilySpec {
                name: "harmonic",
                shapes: &[Shape::Harmonic],
                cells: VERSION_BUNDLE_CELLS,
            },
            FamilyId::Scatter => FamilySpec {
                name: "scatter",
                shapes: &[],
                cells: SCATTER_BUNDLE_CELLS,
            },
            FamilyId::Weave => FamilySpec {
                name: "weave",
                shapes: &[],
                cells: POPULATION_BUNDLE_CELLS,
            },
            FamilyId::Stagger => FamilySpec {
                name: "stagger",
                shapes: &[
                    Shape::StaggerPopulation,
                    Shape::StaggerComb,
                    Shape::StaggerId,
                ],
                cells: POPULATION_BUNDLE_CELLS,
            },
            FamilyId::StaggerArity => FamilySpec {
                name: "stagger-arity",
                shapes: &[Shape::StaggerPopulation],
                cells: POPULATION_BUNDLE_CELLS,
            },
            FamilyId::StaggerSize => FamilySpec {
                name: "stagger-size",
                shapes: &[Shape::StaggerPopulation],
                cells: POPULATION_BUNDLE_CELLS,
            },
            FamilyId::NestedFull => FamilySpec {
                name: "nested-full",
                shapes: &[Shape::Dense, Shape::NestedFullId],
                cells: CROSS_BUNDLE_CELLS,
            },
            FamilyId::NestedWide => FamilySpec {
                name: "nested-wide",
                shapes: &[Shape::Bigroot, Shape::NestedFullId],
                cells: CROSS_BUNDLE_CELLS,
            },
            FamilyId::MirrorWide => FamilySpec {
                name: "mirror-wide",
                shapes: &[Shape::WideTail, Shape::NestedLeftFullId],
                cells: CROSS_BUNDLE_CELLS,
            },
            FamilyId::MirrorNarrow => FamilySpec {
                name: "mirror-narrow",
                shapes: &[Shape::WideTail, Shape::NestedLeftFullId],
                cells: CROSS_BUNDLE_CELLS,
            },
            FamilyId::Staircase => FamilySpec {
                name: "staircase",
                shapes: &[Shape::Staircase, Shape::IdSpine],
                cells: CROSS_BUNDLE_CELLS,
            },
            FamilyId::RevealComb => FamilySpec {
                name: "reveal-comb",
                shapes: &[Shape::RevealComb, Shape::RevealCombId],
                cells: CROSS_BUNDLE_CELLS,
            },
            FamilyId::RevealHifloor => FamilySpec {
                name: "reveal-hifloor",
                shapes: &[Shape::RevealCombHifloor, Shape::RevealCombId],
                cells: CROSS_BUNDLE_CELLS,
            },
            FamilyId::PureComb => FamilySpec {
                name: "pure-comb",
                shapes: &[Shape::PureComb, Shape::PureCombId],
                cells: CROSS_BUNDLE_CELLS,
            },
            FamilyId::AscendCliff => FamilySpec {
                name: "ascend-cliff",
                shapes: &[Shape::AscendCliff, Shape::AscendCliffId],
                cells: CROSS_BUNDLE_CELLS,
            },
            FamilyId::AscendPlateau => FamilySpec {
                name: "ascend-plateau",
                shapes: &[Shape::AscendCliffPlateau, Shape::AscendCliffId],
                cells: CROSS_BUNDLE_CELLS,
            },
            FamilyId::DominatedUndercut => FamilySpec {
                name: "dominated-undercut",
                shapes: &[Shape::DominatedUndercut, Shape::DominatedUndercutId],
                cells: CROSS_BUNDLE_CELLS,
            },
            FamilyId::JumpPair => FamilySpec {
                name: "jump-pair",
                shapes: &[Shape::JumpPair],
                cells: VERSION_BUNDLE_CELLS,
            },
            FamilyId::FreezePos => FamilySpec {
                name: "freeze-pos",
                shapes: &[Shape::FreezePosition],
                cells: VERSION_BUNDLE_CELLS,
            },
            FamilyId::PromoRearm => FamilySpec {
                name: "promo-rearm",
                shapes: &[Shape::PromotionRearm, Shape::PromotionRearmMate],
                cells: VERSION_BUNDLE_CELLS,
            },
            FamilyId::WeightComb => FamilySpec {
                name: "weight-comb",
                shapes: &[Shape::WeightComb],
                cells: VERSION_BUNDLE_CELLS,
            },
            FamilyId::FreezeParade => FamilySpec {
                name: "freeze-parade",
                shapes: &[Shape::FreezeParade],
                cells: VERSION_BUNDLE_CELLS,
            },
            FamilyId::DenseSuffix => FamilySpec {
                name: "dense-suffix",
                shapes: &[Shape::DenseSuffix, Shape::DenseSuffixMate],
                cells: VERSION_BUNDLE_CELLS,
            },
            FamilyId::WideArming => FamilySpec {
                name: "wide-arming",
                shapes: &[Shape::WideArming],
                cells: VERSION_BUNDLE_CELLS,
            },
            FamilyId::PlateauPuncture => FamilySpec {
                name: "plateau-puncture",
                shapes: &[Shape::PlateauPuncture, Shape::PunctureProduct],
                cells: VERSION_BUNDLE_CELLS,
            },
            FamilyId::LoneFreeze => FamilySpec {
                name: "lone-freeze",
                shapes: &[Shape::LoneFreeze],
                cells: VERSION_BUNDLE_CELLS,
            },
            FamilyId::ConcurrentPair => FamilySpec {
                name: "concurrent-pair",
                shapes: &[Shape::ConcurrentPair],
                cells: VERSION_BUNDLE_CELLS,
            },
            FamilyId::ToothTail => FamilySpec {
                name: "tooth-tail",
                shapes: &[Shape::ToothTail],
                cells: VERSION_BUNDLE_CELLS,
            },
            FamilyId::Benign => FamilySpec {
                name: "benign",
                shapes: &[],
                cells: FULL_BUNDLE_CELLS,
            },
            FamilyId::WideToothComb => FamilySpec {
                name: "wide-tooth-comb",
                shapes: &[Shape::WideToothComb],
                cells: VERSION_BUNDLE_CELLS,
            },
            FamilyId::JumpComb => FamilySpec {
                name: "jump-comb",
                shapes: &[Shape::JumpComb],
                cells: VERSION_BUNDLE_CELLS,
            },
            FamilyId::CliffFan => FamilySpec {
                name: "cliff-fan",
                shapes: &[Shape::CliffFan],
                cells: VERSION_BUNDLE_CELLS,
            },
            FamilyId::CancellingChain => FamilySpec {
                name: "cancelling-chain",
                shapes: &[Shape::CancellingChain],
                cells: VERSION_BUNDLE_CELLS,
            },
            FamilyId::AltSpine => FamilySpec {
                name: "alt-spine",
                shapes: &[Shape::AltSpine],
                cells: VERSION_BUNDLE_CELLS,
            },
            FamilyId::MemoChain => FamilySpec {
                name: "memo-chain",
                shapes: &[Shape::MemoChain, Shape::MemoChainId],
                cells: CROSS_BUNDLE_CELLS,
            },
            FamilyId::MemoComb => FamilySpec {
                name: "memo-comb",
                shapes: &[Shape::MemoComb, Shape::MemoCombId],
                cells: CROSS_BUNDLE_CELLS,
            },
            FamilyId::MemoFanout => FamilySpec {
                name: "memo-fanout",
                shapes: &[Shape::MemoFanout],
                cells: CROSS_BUNDLE_CELLS,
            },
            FamilyId::MemoOscillating => FamilySpec {
                name: "memo-oscillating",
                shapes: &[Shape::MemoOscillating],
                cells: CROSS_BUNDLE_CELLS,
            },
            FamilyId::MemoChurn => FamilySpec {
                name: "memo-churn",
                shapes: &[Shape::MemoChurn, Shape::MemoChurnId],
                cells: CROSS_BUNDLE_CELLS,
            },
            FamilyId::DescendingRaises => FamilySpec {
                name: "descending-raises",
                shapes: &[Shape::DescendingRaises, Shape::DescendingRaisesId],
                cells: CROSS_BUNDLE_CELLS,
            },
            FamilyId::MaskDrift => FamilySpec {
                name: "mask-drift",
                shapes: &[Shape::MaskDriftTriple, Shape::MaskDriftQuadruple],
                cells: VERSION_BUNDLE_CELLS,
            },
            FamilyId::MeetShade => FamilySpec {
                name: "meet-shade",
                shapes: &[Shape::MeetShade],
                cells: POPULATION_BUNDLE_CELLS,
            },
            FamilyId::ArmingTrain => FamilySpec {
                name: "arming-train",
                shapes: &[Shape::ArmingTrain],
                cells: VERSION_BUNDLE_CELLS,
            },
            FamilyId::CollapseHole => FamilySpec {
                name: "collapse-hole",
                shapes: &[Shape::CollapseHole],
                cells: CROSS_BUNDLE_CELLS,
            },
            FamilyId::CopyHole => FamilySpec {
                name: "copy-hole",
                shapes: &[Shape::CopyHole],
                cells: CROSS_BUNDLE_CELLS,
            },
            FamilyId::RaiseHole => FamilySpec {
                name: "raise-hole",
                shapes: &[Shape::RaiseHole],
                cells: CROSS_BUNDLE_CELLS,
            },
            FamilyId::SiteHole => FamilySpec {
                name: "site-hole",
                shapes: &[Shape::SiteHole],
                cells: CROSS_BUNDLE_CELLS,
            },
            FamilyId::MaskedHole => FamilySpec {
                name: "masked-hole",
                shapes: &[Shape::MaskedHoleTriple],
                cells: VERSION_BUNDLE_CELLS,
            },
            FamilyId::HoistedWindow => FamilySpec {
                name: "hoisted-window",
                shapes: &[Shape::HoistedWindow],
                cells: VERSION_BUNDLE_CELLS,
            },
            FamilyId::PropagateSeam => FamilySpec {
                name: "propagate-seam",
                shapes: &[
                    Shape::SeamPlunge,
                    Shape::SeamPlungeControl,
                    Shape::SeamStop,
                    Shape::SeamStopControl,
                ],
                cells: VERSION_BUNDLE_CELLS,
            },
            FamilyId::LatentLadder => FamilySpec {
                name: "latent-ladder",
                shapes: &[Shape::LatentLadder],
                cells: VERSION_BUNDLE_CELLS,
            },
        }
    }
}
