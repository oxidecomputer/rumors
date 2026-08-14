//! The cross-kernel verdict matrix: every public answerer of the
//! causal-relation question, cross-checked against its siblings over one
//! roster-derived adversarial operand pool.
//!
//! # The goal (which wins over any mechanism below)
//!
//! A verdict bug on an adversarial path in any one of the five production
//! answerers of the causal-relation question must separate from its
//! siblings on one shared adversarial population, so that no kernel's
//! correctness rests solely on populations that never reach its worst-case
//! machinery. The five public surfaces, and the leg that binds each to the
//! others:
//!
//! - **The pair sweep** behind [`Version`]'s `PartialOrd`: antisymmetry in
//!   both orders, `Equal` exactly on `==`, and `concurrent` exactly on
//!   `None`. Every other axis transcribes its expectation from this
//!   verdict, so the sweep is the reference the siblings must separate
//!   from.
//! - **The masked co-walk** behind [`OwnVersion`](before::OwnVersion)
//!   comparison: a seed-party (unmasked) comparison must equal the sweep's
//!   verdict exactly, and every masked comparison — three-stream, its
//!   mirror orientation, four-stream, and the masked equality kernel —
//!   must agree with the sweep run over the materialized projections
//!   ([`OwnVersion::to_version`](before::OwnVersion::to_version)).
//! - **The placement walk** behind [`Span`]'s
//!   `place`/`dominance`/`precedence`/`contains`: each verdict must equal
//!   its composed-relation transcription from the probe's pairwise
//!   relations to the span's endpoints, on coincident and proper spans
//!   alike, and [`Span::new`]'s accept/reject must match the sweep.
//! - **The fused rank compare** behind [`Ranked`]'s `Ord`: strict causal
//!   order (the sweep's verdict) implies strict `Ranked` order and strict
//!   [`Rank`] order, and the whole comparison equals the rank-then-bytes
//!   transcription through the public [`Version::rank`] and
//!   [`Version::as_bytes`].
//! - **The coverage filter walks** behind `Query::coverage`/`contains`
//!   (the [`causally`] vocabulary): every atom's membership verdict, and
//!   coverage over hulls, points, and floor-and-ceiling conjunctions, must
//!   equal its relation-level transcription.
//!
//! # The operand pool: roster-derived, never hand-picked
//!
//! The pool derives from the family registry itself: [`matrix_operands`]
//! is an exhaustive match over [`FamilyId`], one committed matrix-coverage
//! answer per family in the registry's own coverage-pattern style, so a
//! new family fails to compile until it answers and no hand-enumerated
//! subset can silently exclude one. Each family contributes the first
//! yields of its spec's registered shapes, in spec order: at most two
//! versions and two mask parties, at the smallest committed-valid knobs
//! (each operand tens to hundreds of packed bytes). The pool then closes
//! consecutive seed pairs under `join` and `meet`, which populates the
//! ordered verdicts and the `lo <= hi` spans the matrix crosses.
//!
//! # The budget (derived from the roster count, never tuned by iteration)
//!
//! Seeds are capped at two versions per family, so the seed count is at
//! most `2 · |FamilyId::ALL|`; the closure adds at most two versions per
//! consecutive seed pair, so the pool is bounded by `6 · |FamilyId::ALL|`
//! (a committed assertion in [`build_pool`]) and the cross by its square:
//! on the order of a hundred thousand ordered pairs, each a handful of
//! short stream walks — seconds of work, with no timing asserted anywhere.
//!
//! # Instruments before cures
//!
//! - **Verdict-class liveness floors first**: the run must witness every
//!   sweep class (`Less`, `Equal`, `Greater`, and concurrent) within the
//!   pool, and every placement, dominance, precedence, coverage, and
//!   membership class within the grid, plus a distinct-version rank tie —
//!   red if any is missing, so an all-concurrent or all-equal pool cannot
//!   pass vacuously.
//! - **The adequacy tripwire second**: a committed polarity-flipped twin
//!   of the production verdict (strict orders reversed, `Equal` and
//!   concurrent untouched) runs through the matrix's own checker and is
//!   pinned failing on every cross-surface axis. A global flip is
//!   invisible to the sweep's own antisymmetry — flipping both orders
//!   stays antisymmetric — which is exactly why the matrix exists: only
//!   the sibling surfaces can see it, and the twin pins that each one
//!   does. The twin is rostered by name below, so deleting or renaming it
//!   is a reviewable diff.
//! - **Only then trust green**: the production matrix run asserts zero
//!   violations and a complete census.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use before::causally::{self, Coverage};
use before::meter::registry::{FamilyId, Shape};
use before::meter::Packed;
use before::{
    Clock, Dominance, Endpoint, Party, Placement, Precedence, Rank, Ranked, Span, Version,
};
use suanpan::UBig;

// ─── the roster-derived operand pool ─────────────────────────────────────────

/// One family's matrix-coverage answer: the operands it contributes to the
/// shared pool.
struct MatrixOperands {
    /// Version operands: the pool the cross runs over.
    versions: Vec<Version>,
    /// Mask parties: the masked axis's projection schedule.
    masks: Vec<Party>,
}

/// Decodes a registry-built id shape into its [`Party`].
fn decode_party(packed: &Packed) -> Party {
    Party::decode(&packed.bytes[..]).expect("registry-built id bytes are canonical")
}

/// The scatter population's smallest organic instance: two balanced-fork
/// halves of one universe, one tick each — concurrent, rank-tied, and
/// byte-distinct, the committed witness for the rank-tiebreak leg.
fn scatter_pair() -> (Version, Version) {
    let mut a = Clock::seed();
    let mut b = a.fork();
    let va = a.tick().clone();
    let vb = b.tick().clone();
    (va, vb)
}

/// The weave population's smallest organic instance: four fork-tree
/// parties of one universe, one tick each, joined round-robin so both
/// operands interleave across the shared upper skeleton.
fn weave_pair() -> (Version, Version) {
    let mut a = Clock::seed();
    let mut b = a.fork();
    let mut c = a.fork();
    let mut d = b.fork();
    a.tick();
    b.tick();
    c.tick();
    d.tick();
    (a.version().join(c.version()), b.version().join(d.version()))
}

/// The benign control's smallest organic instance: a shared history
/// prefix, then divergent ticks — the ordinary overlapping-past pair every
/// call site holds.
fn benign_pair() -> (Version, Version) {
    let mut a = Clock::seed();
    a.tick();
    let mut b = a.fork();
    b.tick();
    a.tick();
    (a.version().clone(), b.version().clone())
}

/// The committed matrix-coverage answer for one adversarial family.
///
/// The match is exhaustive over [`FamilyId`], mirroring the registry's
/// coverage pattern: a new family fails to compile until it answers here,
/// and its answer is held nonempty by the floor test below. Each arm
/// yields the first operands of the family's registered shapes in spec
/// order — capped at two versions and two masks, the pool-budget rule the
/// module doc derives — at the smallest committed-valid knobs, so every
/// operand stays tens to hundreds of packed bytes while keeping its
/// family's adversarial structure.
fn matrix_operands(family: FamilyId) -> MatrixOperands {
    let (versions, masks): (Vec<Version>, Vec<Party>) = match family {
        // The dense event spine: node-count and depth maximizer.
        FamilyId::Dense => (vec![Shape::Dense.packed1(8).version()], vec![]),
        // A huge root magnitude over a spine.
        FamilyId::Bigroot => (vec![Shape::Bigroot.packed2(7, 3).version()], vec![]),
        // One node, maximal bits per node.
        FamilyId::Hugeleaf => (vec![Shape::Hugeleaf.packed1(9).version()], vec![]),
        // Leaf values oscillating across a carry cliff.
        FamilyId::Cliff => (vec![Shape::CliffComb.packed2(4, 4).version()], vec![]),
        // The diverted id-spine pair: mask parties for the masked axis.
        FamilyId::IdPair => (
            vec![],
            vec![
                decode_party(&Shape::IdSpine.packed_flagged(4, false)),
                decode_party(&Shape::IdSpine.packed_flagged(4, true)),
            ],
        ),
        // The output-domination cross: boundary comb × scattered party.
        FamilyId::CombScatter => (
            vec![Shape::CliffComb.packed2(4, 8).version()],
            vec![decode_party(&Shape::ScatteredId.packed1(4))],
        ),
        // The harmonic spine: a 1-leaf at every depth.
        FamilyId::Harmonic => (vec![Shape::Harmonic.packed1(16).version()], vec![]),
        // Balanced-forked single-tick operands (organic, one universe).
        FamilyId::Scatter => {
            let (a, b) = scatter_pair();
            (vec![a, b], vec![])
        }
        // Round-robin interleaved fork-tree leaves (organic, one universe).
        FamilyId::Weave => {
            let (a, b) = weave_pair();
            (vec![a, b], vec![])
        }
        // The staggered fold population, teeth in every other operand's gaps.
        FamilyId::Stagger => {
            let (combs, ids) = Shape::StaggerPopulation.population(4, 4);
            (
                vec![combs[0].version(), combs[1].version()],
                vec![decode_party(&ids[0])],
            )
        }
        // The nested-full-sibling cross: dense spine × shortcut-stacking id.
        FamilyId::NestedFull => (
            vec![Shape::Dense.packed1(6).version()],
            vec![decode_party(&Shape::NestedFullId.packed1(6))],
        ),
        // The wide right-full cross: big root × nested-full id.
        FamilyId::NestedWide => (
            vec![Shape::Bigroot.packed2(64, 4).version()],
            vec![decode_party(&Shape::NestedFullId.packed1(4))],
        ),
        // The wide left-full (memo) cross: wide tail × mirrored id.
        FamilyId::MirrorWide => (
            vec![Shape::WideTail.packed2(7, 3).version()],
            vec![decode_party(&Shape::NestedLeftFullId.packed1(8))],
        ),
        // The narrow left-full (memo) cross: unit tail × mirrored id.
        FamilyId::MirrorNarrow => (
            vec![Shape::WideTail.packed2(1, 8).version()],
            vec![decode_party(&Shape::NestedLeftFullId.packed1(4))],
        ),
        // The descending staircase × the unary id spine.
        FamilyId::Staircase => (
            vec![Shape::Staircase.packed1(16).version()],
            vec![decode_party(&Shape::IdSpine.packed_flagged(8, false))],
        ),
        // The reveal comb: sibling left-full sites over a zero floor.
        FamilyId::RevealComb => (
            vec![Shape::RevealComb.packed2(6, 5).version()],
            vec![decode_party(&Shape::RevealCombId.packed1(6))],
        ),
        // The reveal comb's raised-floor gap control.
        FamilyId::RevealHifloor => (
            vec![Shape::RevealCombHifloor.packed2(6, 5).version()],
            vec![decode_party(&Shape::RevealCombId.packed1(6))],
        ),
        // The pure comb: the watermark web's own width circulation.
        FamilyId::PureComb => (
            vec![Shape::PureComb.packed2(6, 5).version()],
            vec![decode_party(&Shape::PureCombId.packed1(6))],
        ),
        // The ascending cliff: stacked boundary differences under an undercut.
        FamilyId::AscendCliff => (
            vec![Shape::AscendCliff.packed2(6, 5).version()],
            vec![decode_party(&Shape::AscendCliffId.packed1(6))],
        ),
        // The ascending cliff's leveled hop-schedule control.
        FamilyId::AscendPlateau => (
            vec![Shape::AscendCliffPlateau.packed2(6, 5).version()],
            vec![decode_party(&Shape::AscendCliffId.packed1(6))],
        ),
        // The dominated-undercut spine (wide leaves decide domination).
        FamilyId::DominatedUndercut => (
            vec![Shape::DominatedUndercut.packed2(4, 128).version()],
            vec![decode_party(&Shape::DominatedUndercutId.packed1(4))],
        ),
        // The two-operand jump comb pair.
        FamilyId::JumpPair => {
            let (a, b) = Shape::JumpPair.packed_pair3(4, 3, 3);
            (vec![a.version(), b.version()], vec![])
        }
        // The freeze-position spine.
        FamilyId::FreezePos => (vec![Shape::FreezePosition.packed1(3).version()], vec![]),
        // The promotion re-arm spine and its small twin.
        FamilyId::PromoRearm => (
            vec![
                Shape::PromotionRearm.packed1(3).version(),
                Shape::PromotionRearmMate.packed1(3).version(),
            ],
            vec![],
        ),
        // The weight comb (one complete power-of-two subtree).
        FamilyId::WeightComb => (vec![Shape::WeightComb.packed1(8).version()], vec![]),
        // The freeze parade (one complete power-of-two subtree).
        FamilyId::FreezeParade => (vec![Shape::FreezeParade.packed1(8).version()], vec![]),
        // The dense-suffix re-arm family and its unit twin.
        FamilyId::DenseSuffix => (
            vec![
                Shape::DenseSuffix.packed2(3, 2).version(),
                Shape::DenseSuffixMate.packed2(3, 2).version(),
            ],
            vec![],
        ),
        // The wide-arming family (width at its precondition floor).
        FamilyId::WideArming => (vec![Shape::WideArming.packed2(10, 2).version()], vec![]),
        // The plateau puncture and its arbitrary-factor product embedding.
        FamilyId::PlateauPuncture => (
            vec![
                Shape::PlateauPuncture.packed2(10, 3).version(),
                Shape::PunctureProduct
                    .packed_product(&UBig::from(3u8), &UBig::from(5u8))
                    .version(),
            ],
            vec![],
        ),
        // The lone freeze: a whole-pair plateau prefix and low tail.
        FamilyId::LoneFreeze => (vec![Shape::LoneFreeze.packed2(2, 2).version()], vec![]),
        // The organically built concurrent pair.
        FamilyId::ConcurrentPair => {
            let (a, b) = Shape::ConcurrentPair.version_pair(8);
            (vec![a, b], vec![])
        }
        // The tooth-tail pair: a spike riding the second leaf.
        FamilyId::ToothTail => {
            let (a, b) = Shape::ToothTail.packed_pair(2, 8);
            (vec![a.version(), b.version()], vec![])
        }
        // The organic overlapping-history control (one universe).
        FamilyId::Benign => {
            let (a, b) = benign_pair();
            (vec![a, b], vec![])
        }
        // The wide-tooth comb: teeth of width below the cliff.
        FamilyId::WideToothComb => (
            vec![Shape::WideToothComb.packed3(16, 8, 8).version()],
            vec![],
        ),
        // The jump comb: one low tooth, then cliff teeth.
        FamilyId::JumpComb => (vec![Shape::JumpComb.packed2(4, 4).version()], vec![]),
        // The unpaid-crossing fan under one stored magnitude.
        FamilyId::CliffFan => (vec![Shape::CliffFan.packed2(4, 4).version()], vec![]),
        // The cancelling-prefix chain of peak-to-1 drops.
        FamilyId::CancellingChain => (vec![Shape::CancellingChain.packed2(4, 4).version()], vec![]),
        // The alternating-binary spine.
        FamilyId::AltSpine => (vec![Shape::AltSpine.packed1(8).version()], vec![]),
        // The memo chain, both flag variants, with its id as a mask.
        FamilyId::MemoChain => (
            vec![
                Shape::MemoChain.packed_flagged(8, true).version(),
                Shape::MemoChain.packed_flagged(8, false).version(),
            ],
            vec![decode_party(&Shape::MemoChainId.packed1(8))],
        ),
        // The memo comb with its id as a mask.
        FamilyId::MemoComb => (
            vec![Shape::MemoComb.packed1(4).version()],
            vec![decode_party(&Shape::MemoCombId.packed1(4))],
        ),
        // The memo fan-out event.
        FamilyId::MemoFanout => (vec![Shape::MemoFanout.packed2(4, 8).version()], vec![]),
        // The oscillating-siblings event.
        FamilyId::MemoOscillating => (vec![Shape::MemoOscillating.packed2(4, 8).version()], vec![]),
        // The memo-churn event with its id as a mask.
        FamilyId::MemoChurn => (
            vec![Shape::MemoChurn.packed1(5).version()],
            vec![decode_party(&Shape::MemoChurnId.packed1(5))],
        ),
        // The descending-raises event with its id as a mask.
        FamilyId::DescendingRaises => (
            vec![Shape::DescendingRaises.packed1(6).version()],
            vec![decode_party(&Shape::DescendingRaisesId.packed1(6))],
        ),
        // The masked-comparison drift bundles: correlated versions plus
        // the registry's mask-carrying parties for the masked axis.
        FamilyId::MaskDrift => {
            let (comb, mask, plateau) = Shape::MaskDriftTriple.packed_triple(8, 8);
            let ((_, even_mask), _) = Shape::MaskDriftQuadruple.packed_quadruple(8, 8);
            (
                vec![comb.version(), plateau.version()],
                vec![decode_party(&mask), decode_party(&even_mask)],
            )
        }
        // The meet-shade population: a carrier under shading operands.
        FamilyId::MeetShade => {
            let population = Shape::MeetShade.versions(4, 4);
            (vec![population[0].clone(), population[1].clone()], vec![])
        }
        // The arming train, both sign schedules (width at its floor).
        FamilyId::ArmingTrain => (
            vec![
                Shape::ArmingTrain.packed_train(4, 19, 2, true).version(),
                Shape::ArmingTrain.packed_train(3, 19, 1, false).version(),
            ],
            vec![],
        ),
        // The scan-hole cross: the collapse hole's coupled (event, id) pair
        // (the first yields of the family's registered shapes under the cap).
        FamilyId::ScanHole => {
            let (ev, id) = Shape::CollapseHole.packed_pair(4, 4);
            (vec![ev.version()], vec![decode_party(&id)])
        }
        // The masked-hole triple: a deep spine under a shallow diverted mask.
        FamilyId::MaskedHole => {
            let (spine, mask, plateau) = Shape::MaskedHoleTriple.packed_triple(6, 2);
            (
                vec![spine.version(), plateau.version()],
                vec![decode_party(&mask)],
            )
        }
        // The hoisted window (tail at its hoist precondition floor).
        FamilyId::HoistedWindow => (
            vec![Shape::HoistedWindow.packed3(10, 2, 384).version()],
            vec![],
        ),
        // The seam-plunge spine and its leveled control.
        FamilyId::PropagateSeam => (
            vec![
                Shape::SeamPlunge.packed2(4, 5).version(),
                Shape::SeamPlungeControl.packed2(4, 5).version(),
            ],
            vec![],
        ),
        // The latent-ladder comb.
        FamilyId::LatentLadder => (vec![Shape::LatentLadder.packed2(4, 4).version()], vec![]),
    };
    MatrixOperands { versions, masks }
}

/// The shared operand pool: the deduplicated roster seeds, their
/// consecutive-pair join/meet closure, and the mask schedule.
struct Pool {
    /// Every pool version; seeds first (roster order), closure after.
    versions: Vec<Version>,
    /// The mask parties the masked axis cycles through.
    masks: Vec<Party>,
    /// One record per consecutive seed pair: `[a, b, meet, join]` as pool
    /// indices — the span roster and conjunction grid derive from these.
    adjacent: Vec<[usize; 4]>,
}

/// Interns a version into the pool, returning its index.
fn intern(versions: &mut Vec<Version>, index: &mut HashMap<Version, usize>, v: Version) -> usize {
    if let Some(&i) = index.get(&v) {
        return i;
    }
    let i = versions.len();
    index.insert(v.clone(), i);
    versions.push(v);
    i
}

/// Builds the pool from the roster: every family's committed answer, then
/// the consecutive-pair closure under join and meet.
///
/// The committed budget ceiling lives here: at most two seed versions per
/// family and at most two closure versions per consecutive seed pair
/// bounds the pool by six versions per roster entry.
fn build_pool() -> Pool {
    let mut versions = Vec::new();
    let mut index = HashMap::new();
    let mut masks: Vec<Party> = Vec::new();
    let mut seeds = Vec::new();
    for family in FamilyId::ALL {
        let answer = matrix_operands(family);
        for v in answer.versions {
            let i = intern(&mut versions, &mut index, v);
            if !seeds.contains(&i) {
                seeds.push(i);
            }
        }
        for p in answer.masks {
            if !masks.contains(&p) {
                masks.push(p);
            }
        }
    }
    let mut adjacent = Vec::new();
    for w in seeds.windows(2) {
        let (a, b) = (w[0], w[1]);
        let meet = versions[a].meet(&versions[b]);
        let join = versions[a].join(&versions[b]);
        let m = intern(&mut versions, &mut index, meet);
        let j = intern(&mut versions, &mut index, join);
        adjacent.push([a, b, m, j]);
    }
    assert!(
        versions.len() <= 6 * FamilyId::ALL.len(),
        "the pool outgrew its committed budget: {} versions against a ceiling of \
         six per roster family",
        versions.len()
    );
    Pool {
        versions,
        masks,
        adjacent,
    }
}

// ─── the sweep verdict classes ───────────────────────────────────────────────

/// The sweep-class label of one pairwise verdict.
fn class(rel: Option<Ordering>) -> &'static str {
    match rel {
        Some(Ordering::Less) => "less",
        Some(Ordering::Equal) => "equal",
        Some(Ordering::Greater) => "greater",
        None => "concurrent",
    }
}

/// The sweep verdict classes a version list witnesses under the
/// production comparison, for the liveness floors.
fn verdict_classes(versions: &[Version]) -> BTreeSet<&'static str> {
    let mut classes = BTreeSet::new();
    for a in versions {
        for b in versions {
            classes.insert(class(a.partial_cmp(b)));
        }
    }
    classes
}

/// Every sweep verdict class, the floors' completeness reference.
const ALL_CLASSES: [&str; 4] = ["concurrent", "equal", "greater", "less"];

// ─── the checker ─────────────────────────────────────────────────────────────

/// The five verdict surfaces, as violation labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Axis {
    /// The pair sweep's own coherence (antisymmetry, equality, concurrency).
    Sweep,
    /// The masked co-walk against the sweep and the materialized projections.
    Masked,
    /// The placement walk against its composed-relation transcription.
    Placement,
    /// The fused rank compare against strict causal order and the tiebreak.
    Ranked,
    /// The coverage filter walks against their relation-level transcription.
    Query,
}

/// What one checker run witnessed: the liveness census the floors assert.
#[derive(Default)]
struct Census {
    /// Sweep verdict classes seen in the pool cross.
    classes: BTreeSet<&'static str>,
    /// Top-level placement verdicts seen across the span grid.
    placements: BTreeSet<&'static str>,
    /// Dominance verdicts seen across the span grid.
    dominances: BTreeSet<&'static str>,
    /// Precedence verdicts seen across the span grid.
    precedences: BTreeSet<&'static str>,
    /// Coverage verdicts seen across the query grid.
    coverages: BTreeSet<&'static str>,
    /// Membership verdicts seen across the query grid.
    memberships: BTreeSet<bool>,
    /// Distinct-version rank ties seen in the pool: the tiebreak witness.
    rank_ties: usize,
}

/// One checker run's result: per-axis violation counts, a bounded sample
/// of violation details, and the liveness census.
struct Outcome {
    /// Violations per axis (absent means zero).
    counts: BTreeMap<Axis, usize>,
    /// The first few violation details, for diagnosis.
    samples: Vec<String>,
    /// The classes this run witnessed.
    census: Census,
}

impl Outcome {
    /// Violations recorded against `axis`.
    fn count(&self, axis: Axis) -> usize {
        self.counts.get(&axis).copied().unwrap_or(0)
    }
}

/// `probe >= bound` under a pairwise verdict.
fn ge(rel: Option<Ordering>) -> bool {
    matches!(rel, Some(Ordering::Greater | Ordering::Equal))
}

/// `probe <= bound` under a pairwise verdict.
fn le(rel: Option<Ordering>) -> bool {
    matches!(rel, Some(Ordering::Less | Ordering::Equal))
}

/// The composed-relation transcription of the nine-way placement verdict
/// from the probe's relations to the span's endpoints.
fn expected_place(lo_rel: Option<Ordering>, hi_rel: Option<Ordering>) -> Placement {
    match lo_rel {
        Some(Ordering::Less) => Placement::Before,
        Some(Ordering::Equal) => match hi_rel {
            Some(Ordering::Equal) => Placement::At(Endpoint::Both),
            _ => Placement::At(Endpoint::Start),
        },
        Some(Ordering::Greater) => match hi_rel {
            Some(Ordering::Less) => Placement::Between,
            Some(Ordering::Equal) => Placement::At(Endpoint::End),
            Some(Ordering::Greater) => Placement::After,
            None => Placement::Concurrent(Endpoint::End),
        },
        None => match hi_rel {
            None => Placement::Concurrent(Endpoint::Both),
            _ => Placement::Concurrent(Endpoint::Start),
        },
    }
}

/// The transcription of the three-way dominance verdict: after on
/// dominating both endpoints, between on dominating only the start.
fn expected_dominance(lo_rel: Option<Ordering>, hi_rel: Option<Ordering>) -> Dominance {
    if ge(lo_rel) && ge(hi_rel) {
        Dominance::After
    } else if ge(lo_rel) {
        Dominance::Between
    } else {
        Dominance::Before
    }
}

/// The transcription of the three-way precedence verdict, dominance
/// mirrored.
fn expected_precedence(lo_rel: Option<Ordering>, hi_rel: Option<Ordering>) -> Precedence {
    if le(lo_rel) && le(hi_rel) {
        Precedence::Before
    } else if le(hi_rel) {
        Precedence::Between
    } else {
        Precedence::After
    }
}

/// The top-level placement label, for the census.
fn placement_label(p: Placement) -> &'static str {
    match p {
        Placement::Before => "before",
        Placement::At(_) => "at",
        Placement::Between => "between",
        Placement::After => "after",
        Placement::Concurrent(_) => "concurrent",
    }
}

/// The coverage label, for the census.
fn coverage_label(c: Coverage) -> &'static str {
    match c {
        Coverage::Full => "full",
        Coverage::Partial => "partial",
        Coverage::Empty => "empty",
    }
}

/// How many violation details to retain verbatim for diagnosis.
const SAMPLE_CAP: usize = 40;

/// Runs the full consistency matrix over the pool with `sweep` as the
/// pairwise verdict of record, returning per-axis violations and the
/// liveness census.
///
/// Under the production sweep (`Version::partial_cmp`) every leg must
/// agree; under the polarity-flipped twin every cross-surface axis must
/// dissent, which is the committed proof that each leg can fail.
fn check(pool: &Pool, sweep: &dyn Fn(&Version, &Version) -> Option<Ordering>) -> Outcome {
    let seed = Party::seed();
    let mut outcome = Outcome {
        counts: BTreeMap::new(),
        samples: Vec::new(),
        census: Census::default(),
    };

    // The pairwise verdict matrix of record: every transcription below
    // reads from here, so a dissenting kernel separates from `sweep`.
    let rel: Vec<Vec<Option<Ordering>>> = pool
        .versions
        .iter()
        .map(|a| pool.versions.iter().map(|b| sweep(a, b)).collect())
        .collect();
    for row in &rel {
        for &r in row {
            outcome.census.classes.insert(class(r));
        }
    }

    // Materialized projections: proj[i][m] is pool version i projected by
    // mask m, through the public materialization door. The masked axis
    // holds every fused verdict to the sweep over these.
    let proj: Vec<Vec<Version>> = pool
        .versions
        .iter()
        .map(|v| pool.masks.iter().map(|p| (v / p).to_version()).collect())
        .collect();

    // Rank folds, once per pool version, for the ranked axis.
    let ranks: Vec<Rank> = pool.versions.iter().map(Version::rank).collect();
    for (i, ri) in ranks.iter().enumerate() {
        for (j, rj) in ranks.iter().enumerate().skip(i + 1) {
            if ri == rj && pool.versions[i] != pool.versions[j] {
                outcome.census.rank_ties += 1;
            }
        }
    }

    let flag = |outcome: &mut Outcome, axis: Axis, detail: String| {
        *outcome.counts.entry(axis).or_insert(0) += 1;
        if outcome.samples.len() < SAMPLE_CAP {
            outcome.samples.push(format!("{axis:?}: {detail}"));
        }
    };

    // ── the ordered-pair cross: sweep, masked, ranked, degenerate spans,
    //    and the per-pair query legs ──
    for (j, vj) in pool.versions.iter().enumerate() {
        // Queries bound at vj, built once per bound and probed by every i.
        // Expected membership is the relation-level transcription of each
        // form's documented meaning, from rel[i][j].
        #[allow(clippy::type_complexity)]
        let membership_forms: Vec<(
            &'static str,
            Box<dyn Fn(&Version) -> bool + '_>,
            Box<dyn Fn(Option<Ordering>) -> bool>,
        )> = {
            let q_floor = causally::after(vj) & causally::all();
            let q_ceiling = causally::before(vj) & causally::all();
            let q_gt = causally::strictly_after(vj);
            let q_lt = causally::strictly_before(vj);
            let q_since = causally::since(vj);
            let q_until = causally::until(vj);
            let q_floor_wide = causally::after(vj).or_concurrent();
            let q_ceiling_wide = causally::before(vj).or_concurrent();
            let q_point = causally::after(vj) & causally::before(vj);
            vec![
                (
                    "after",
                    Box::new(move |v: &Version| q_floor.contains(v)),
                    Box::new(ge),
                ),
                (
                    "before",
                    Box::new(move |v: &Version| q_ceiling.contains(v)),
                    Box::new(le),
                ),
                (
                    "strictly_after",
                    Box::new(move |v: &Version| q_gt.contains(v)),
                    Box::new(|r| r == Some(Ordering::Greater)),
                ),
                (
                    "strictly_before",
                    Box::new(move |v: &Version| q_lt.contains(v)),
                    Box::new(|r| r == Some(Ordering::Less)),
                ),
                (
                    "since",
                    Box::new(move |v: &Version| q_since.contains(v)),
                    Box::new(|r| matches!(r, Some(Ordering::Greater) | None)),
                ),
                (
                    "until",
                    Box::new(move |v: &Version| q_until.contains(v)),
                    Box::new(|r| matches!(r, Some(Ordering::Less) | None)),
                ),
                (
                    "after.or_concurrent",
                    Box::new(move |v: &Version| q_floor_wide.contains(v)),
                    Box::new(|r| r != Some(Ordering::Less)),
                ),
                (
                    "before.or_concurrent",
                    Box::new(move |v: &Version| q_ceiling_wide.contains(v)),
                    Box::new(|r| r != Some(Ordering::Greater)),
                ),
                (
                    "after & before (point)",
                    Box::new(move |v: &Version| q_point.contains(v)),
                    Box::new(|r| r == Some(Ordering::Equal)),
                ),
            ]
        };
        let q_floor_cov = causally::after(vj) & causally::all();
        let q_ceiling_cov = causally::before(vj) & causally::all();
        let q_gt_cov = causally::strictly_after(vj);
        let at_j = Span::at(vj);

        for (i, vi) in pool.versions.iter().enumerate() {
            let r = rel[i][j];

            // Sweep axis: antisymmetry, equality, and concurrency cohere
            // with the verdict of record.
            if r != rel[j][i].map(Ordering::reverse) {
                flag(
                    &mut outcome,
                    Axis::Sweep,
                    format!("antisymmetry broke at pool pair ({i}, {j})"),
                );
            }
            if (vi == vj) != (r == Some(Ordering::Equal)) {
                flag(
                    &mut outcome,
                    Axis::Sweep,
                    format!("equality disagrees with the verdict at ({i}, {j})"),
                );
            }
            if vi.concurrent(vj) != r.is_none() {
                flag(
                    &mut outcome,
                    Axis::Sweep,
                    format!("concurrent() disagrees with the verdict at ({i}, {j})"),
                );
            }

            // Masked axis. The unmasked (seed-party) comparison must equal
            // the verdict of record exactly; every masked cell must equal
            // the verdict of record over the materialized projections.
            if (vi / &seed).partial_cmp(vj) != r {
                flag(
                    &mut outcome,
                    Axis::Masked,
                    format!("seed-projected compare dissents from the sweep at ({i}, {j})"),
                );
            }
            if !pool.masks.is_empty() {
                let m = (i + j) % pool.masks.len();
                let mask = &pool.masks[m];
                let fused3 = (vi / mask).partial_cmp(vj);
                if fused3 != sweep(&proj[i][m], vj) {
                    flag(
                        &mut outcome,
                        Axis::Masked,
                        format!("three-stream verdict dissents at ({i}, {j}) under mask {m}"),
                    );
                }
                if vj.partial_cmp(&(vi / mask)) != fused3.map(Ordering::reverse) {
                    flag(
                        &mut outcome,
                        Axis::Masked,
                        format!("mirror orientation dissents at ({i}, {j}) under mask {m}"),
                    );
                }
                if ((vi / mask) == *vj) != (sweep(&proj[i][m], vj) == Some(Ordering::Equal)) {
                    flag(
                        &mut outcome,
                        Axis::Masked,
                        format!("masked equality dissents at ({i}, {j}) under mask {m}"),
                    );
                }
                let fused4 = (vi / mask).partial_cmp(&(vj / mask));
                if fused4 != sweep(&proj[i][m], &proj[j][m]) {
                    flag(
                        &mut outcome,
                        Axis::Masked,
                        format!("four-stream verdict dissents at ({i}, {j}) under mask {m}"),
                    );
                }
                if ((vi / mask) == (vj / mask))
                    != (sweep(&proj[i][m], &proj[j][m]) == Some(Ordering::Equal))
                {
                    flag(
                        &mut outcome,
                        Axis::Masked,
                        format!("four-stream equality dissents at ({i}, {j}) under mask {m}"),
                    );
                }
            }

            // Ranked axis: strict causal order implies strict Ranked and
            // Rank order, and the whole comparison is the rank-then-bytes
            // transcription.
            let ranked = Ranked::from(vi).cmp(&Ranked::from(vj));
            match r {
                Some(strict @ (Ordering::Less | Ordering::Greater)) => {
                    if ranked != strict {
                        flag(
                            &mut outcome,
                            Axis::Ranked,
                            format!("Ranked order dissents from strict causal order at ({i}, {j})"),
                        );
                    }
                    if ranks[i].cmp(&ranks[j]) != strict {
                        flag(
                            &mut outcome,
                            Axis::Ranked,
                            format!("Rank is not strictly monotone at ({i}, {j})"),
                        );
                    }
                }
                Some(Ordering::Equal) | None => {}
            }
            let transcribed = ranks[i]
                .cmp(&ranks[j])
                .then_with(|| vi.as_bytes().cmp(vj.as_bytes()));
            if ranked != transcribed {
                flag(
                    &mut outcome,
                    Axis::Ranked,
                    format!("Ranked order is not rank-then-bytes at ({i}, {j})"),
                );
            }

            // Placement axis, degenerate spans: a coincident span's four
            // verdicts collapse to the pairwise relation, and Span::new
            // accepts exactly the ordered pairs.
            let place = at_j.place(vi);
            outcome.census.placements.insert(placement_label(place));
            let place_expected = match r {
                Some(Ordering::Less) => Placement::Before,
                Some(Ordering::Equal) => Placement::At(Endpoint::Both),
                Some(Ordering::Greater) => Placement::After,
                None => Placement::Concurrent(Endpoint::Both),
            };
            if place != place_expected {
                flag(
                    &mut outcome,
                    Axis::Placement,
                    format!("coincident placement dissents at ({i}, {j})"),
                );
            }
            if at_j.contains(vi) != (r == Some(Ordering::Equal)) {
                flag(
                    &mut outcome,
                    Axis::Placement,
                    format!("coincident containment dissents at ({i}, {j})"),
                );
            }
            let dom = at_j.dominance(vi);
            let dom_expected = if ge(r) {
                Dominance::After
            } else {
                Dominance::Before
            };
            if dom != dom_expected {
                flag(
                    &mut outcome,
                    Axis::Placement,
                    format!("coincident dominance dissents at ({i}, {j})"),
                );
            }
            let prec = at_j.precedence(vi);
            let prec_expected = if le(r) {
                Precedence::Before
            } else {
                Precedence::After
            };
            if prec != prec_expected {
                flag(
                    &mut outcome,
                    Axis::Placement,
                    format!("coincident precedence dissents at ({i}, {j})"),
                );
            }
            if Span::new(vi, vj).is_ok() != le(r) {
                flag(
                    &mut outcome,
                    Axis::Placement,
                    format!("span construction disagrees with the verdict at ({i}, {j})"),
                );
            }

            // Query axis, membership: every form's verdict equals its
            // relation-level transcription.
            for (name, admits, expected) in &membership_forms {
                let got = admits(vi);
                outcome.census.memberships.insert(got);
                if got != expected(r) {
                    flag(
                        &mut outcome,
                        Axis::Query,
                        format!("membership under `{name}` dissents at ({i}, {j})"),
                    );
                }
            }

            // Query axis, coverage over the pair's hull [meet, join]: by
            // lattice absorption a floor-only query is full exactly when
            // the bound sits below the probe, a ceiling-only one exactly
            // when it sits above, and neither can empty the hull.
            let hull = vi.span(vj);
            let floor_cov = q_floor_cov.coverage(hull.reborrow());
            outcome.census.coverages.insert(coverage_label(floor_cov));
            let floor_expected = if ge(r) {
                Coverage::Full
            } else {
                Coverage::Partial
            };
            if floor_cov != floor_expected {
                flag(
                    &mut outcome,
                    Axis::Query,
                    format!("floor coverage over the hull dissents at ({i}, {j})"),
                );
            }
            let ceiling_cov = q_ceiling_cov.coverage(hull.reborrow());
            outcome.census.coverages.insert(coverage_label(ceiling_cov));
            let ceiling_expected = if le(r) {
                Coverage::Full
            } else {
                Coverage::Partial
            };
            if ceiling_cov != ceiling_expected {
                flag(
                    &mut outcome,
                    Axis::Query,
                    format!("ceiling coverage over the hull dissents at ({i}, {j})"),
                );
            }
            // Coverage of a point span collapses to membership.
            let point_cov = q_gt_cov.coverage(vi);
            outcome.census.coverages.insert(coverage_label(point_cov));
            let point_expected = if r == Some(Ordering::Greater) {
                Coverage::Full
            } else {
                Coverage::Empty
            };
            if point_cov != point_expected {
                flag(
                    &mut outcome,
                    Axis::Query,
                    format!("point coverage dissents at ({i}, {j})"),
                );
            }
        }
    }

    // ── the span grid: proper spans from the closure records, every pool
    //    version as a probe, all four verdicts transcribed ──
    let mut spans: Vec<(usize, usize)> = Vec::new();
    for &[a, _, m, j] in &pool.adjacent {
        spans.push((m, j));
        spans.push((a, j));
        spans.push((m, a));
    }
    for &(lo, hi) in &spans {
        let span = Span::new(&pool.versions[lo], &pool.versions[hi])
            .expect("closure endpoints are ordered by construction");
        for (i, vi) in pool.versions.iter().enumerate() {
            let (lo_rel, hi_rel) = (rel[i][lo], rel[i][hi]);
            let place = span.place(vi);
            outcome.census.placements.insert(placement_label(place));
            if place != expected_place(lo_rel, hi_rel) {
                flag(
                    &mut outcome,
                    Axis::Placement,
                    format!("placement dissents at probe {i} against span ({lo}, {hi})"),
                );
            }
            let dom = span.dominance(vi);
            outcome.census.dominances.insert(match dom {
                Dominance::Before => "before",
                Dominance::Between => "between",
                Dominance::After => "after",
            });
            if dom != expected_dominance(lo_rel, hi_rel) {
                flag(
                    &mut outcome,
                    Axis::Placement,
                    format!("dominance dissents at probe {i} against span ({lo}, {hi})"),
                );
            }
            let prec = span.precedence(vi);
            outcome.census.precedences.insert(match prec {
                Precedence::Before => "before",
                Precedence::Between => "between",
                Precedence::After => "after",
            });
            if prec != expected_precedence(lo_rel, hi_rel) {
                flag(
                    &mut outcome,
                    Axis::Placement,
                    format!("precedence dissents at probe {i} against span ({lo}, {hi})"),
                );
            }
            if span.contains(vi) != (ge(lo_rel) && le(hi_rel)) {
                flag(
                    &mut outcome,
                    Axis::Placement,
                    format!("containment dissents at probe {i} against span ({lo}, {hi})"),
                );
            }
        }
    }

    // ── the conjunction-coverage grid: floor-and-ceiling queries from the
    //    closure records against the closure spans, with the exact
    //    full/partial/empty transcription ──
    for &[_, _, qm, qj] in &pool.adjacent {
        let q = causally::after(&pool.versions[qm]) & causally::before(&pool.versions[qj]);
        for &[_, _, sm, sj] in &pool.adjacent {
            let span = Span::new(&pool.versions[sm], &pool.versions[sj])
                .expect("closure endpoints are ordered by construction");
            let got = q.coverage(span);
            outcome.census.coverages.insert(coverage_label(got));
            // Full exactly when the segment sits inside the bounds; empty
            // exactly when the clamped segment crosses; partial otherwise.
            let expected = if ge(rel[sm][qm]) && le(rel[sj][qj]) {
                Coverage::Full
            } else {
                let clamped_lo = pool.versions[sm].join(&pool.versions[qm]);
                let clamped_hi = pool.versions[sj].meet(&pool.versions[qj]);
                if le(sweep(&clamped_lo, &clamped_hi)) {
                    Coverage::Partial
                } else {
                    Coverage::Empty
                }
            };
            if got != expected {
                flag(
                    &mut outcome,
                    Axis::Query,
                    format!(
                        "conjunction coverage dissents for bounds ({qm}, {qj}) over span \
                         ({sm}, {sj})"
                    ),
                );
            }
        }
    }

    outcome
}

// ─── the committed tripwire roster ───────────────────────────────────────────

/// Every `_reads_inverted` adequacy tripwire, as `(file relative to the
/// crate root, test fn name)`.
///
/// A polarity-flipped twin binds nowhere mechanically unless something
/// names it: this roster is the jaw that makes deleting or renaming one a
/// reviewable diff, in the committed style of the crate's superlinear
/// tripwire roster.
const TRIPWIRE_ROSTER: &[(&str, &str)] = &[(
    "tests/verdict_matrix.rs",
    "polarity_flipped_sweep_reads_inverted_through_the_matrix",
)];

/// Collect every `fn` whose name carries `_reads_inverted` under `dir`,
/// keyed by the path relative to the crate root.
fn scan(dir: &Path, root: &Path, found: &mut BTreeSet<(String, String)>) {
    for entry in std::fs::read_dir(dir).expect("source directory is readable") {
        let path = entry.expect("directory entry is readable").path();
        if path.is_dir() {
            scan(&path, root, found);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let text = std::fs::read_to_string(&path).expect("source file is readable");
            for line in text.lines() {
                let Some(rest) = line.trim_start().strip_prefix("fn ") else {
                    continue;
                };
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if name.contains("_reads_inverted") {
                    let rel = path
                        .strip_prefix(root)
                        .expect("scanned file lives under the crate root")
                        .to_string_lossy()
                        .into_owned();
                    found.insert((rel, name));
                }
            }
        }
    }
}

// ─── the tests ───────────────────────────────────────────────────────────────

/// Every roster family answers the matrix-coverage question with at least
/// one operand, and the assembled pool stays within its derived budget
/// with a nonempty mask schedule.
#[test]
fn every_family_answers_the_matrix_coverage_question() {
    for family in FamilyId::ALL {
        let answer = matrix_operands(family);
        assert!(
            !answer.versions.is_empty() || !answer.masks.is_empty(),
            "{family:?} contributes no matrix operands: an empty answer silently \
             excludes the family from every axis"
        );
        assert!(
            answer.versions.len() <= 2 && answer.masks.len() <= 2,
            "{family:?} outgrew the per-family operand cap the budget derives from"
        );
    }
    let pool = build_pool();
    assert!(
        !pool.masks.is_empty(),
        "the mask schedule is empty: the masked axis would run unmasked only"
    );
    assert!(
        pool.adjacent.len() >= 2,
        "the closure produced fewer than two records: the span and conjunction \
         grids need at least a pair"
    );
}

/// The pool witnesses every sweep verdict class — less, equal, greater,
/// and concurrent — so no axis of the matrix can pass on a vacuous
/// population.
#[test]
fn pool_witnesses_every_verdict_class() {
    let pool = build_pool();
    let classes = verdict_classes(&pool.versions);
    assert_eq!(
        classes,
        BTreeSet::from(ALL_CLASSES),
        "the pool fails its verdict-class liveness floor"
    );
}

/// The verdict-class floor reads red on degenerate pools: an all-equal
/// pool and an all-concurrent pool each fail the census, proving the floor
/// is alive.
#[test]
fn verdict_class_floors_read_red_on_degenerate_pools() {
    let v = Shape::Dense.packed1(4).version();
    let equal_only = verdict_classes(&[v.clone(), v]);
    assert_ne!(
        equal_only,
        BTreeSet::from(ALL_CLASSES),
        "an all-equal pool passed the class floor"
    );
    let (a, b) = Shape::ConcurrentPair.version_pair(8);
    let concurrent_only = verdict_classes(&[a, b]);
    assert_ne!(
        concurrent_only,
        BTreeSet::from(ALL_CLASSES),
        "an all-concurrent pool passed the class floor"
    );
}

/// The committed adequacy twin: the production verdict with strict
/// orders reversed (equal and concurrent untouched) dissents on every
/// cross-surface axis of the matrix.
///
/// The sweep's own coherence legs stay quiet — a global flip is
/// antisymmetric, so only the sibling surfaces can catch a verdict
/// inversion, and this pins that every one of them does. This is the
/// failure the oracle differentials do not commit: a kernel-local
/// verdict inversion on adversarial shapes the law populations
/// under-hit.
#[test]
fn polarity_flipped_sweep_reads_inverted_through_the_matrix() {
    let pool = build_pool();
    let outcome = check(&pool, &|a, b| a.partial_cmp(b).map(Ordering::reverse));
    for axis in [Axis::Masked, Axis::Placement, Axis::Ranked, Axis::Query] {
        assert!(
            outcome.count(axis) > 0,
            "the flipped twin passed the {axis:?} axis: that leg of the matrix \
             cannot catch a verdict inversion"
        );
    }
    assert_eq!(
        outcome.count(Axis::Sweep),
        0,
        "the flipped twin tripped the sweep's own coherence legs: a global flip \
         is antisymmetric, so those legs firing means they read something other \
         than the verdict of record"
    );
}

/// The committed-failing inverted-verdict twins match the roster exactly,
/// in both directions, so deleting or renaming one is a reviewable diff.
#[test]
fn inverted_verdict_tripwires_match_the_committed_roster() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut found = BTreeSet::new();
    scan(&root.join("src"), &root, &mut found);
    scan(&root.join("tests"), &root, &mut found);
    let expected: BTreeSet<(String, String)> = TRIPWIRE_ROSTER
        .iter()
        .map(|&(file, name)| (file.to_owned(), name.to_owned()))
        .collect();
    assert_eq!(
        found, expected,
        "the _reads_inverted twin set drifted from the committed roster: an \
         adequacy twin that binds nowhere is silently deletable, and the matrix \
         green as decoration"
    );
}

/// The production verdict matrix: all five public surfaces agree on
/// every ordered pool pair, with every liveness floor holding.
///
/// The floors: every verdict class, placement, dominance, precedence,
/// coverage, and membership witnessed, and at least one distinct-version
/// rank tie exercising the byte tiebreak.
#[test]
fn production_verdicts_agree_across_the_five_surfaces() {
    let pool = build_pool();
    assert_eq!(
        verdict_classes(&pool.versions),
        BTreeSet::from(ALL_CLASSES),
        "the pool fails its verdict-class liveness floor"
    );
    let outcome = check(&pool, &|a, b| a.partial_cmp(b));
    assert!(
        outcome.counts.is_empty(),
        "the verdict matrix dissents: {:?}\nfirst samples:\n{}",
        outcome.counts,
        outcome.samples.join("\n")
    );
    let census = &outcome.census;
    assert_eq!(
        census.classes,
        BTreeSet::from(ALL_CLASSES),
        "the checker's own class census disagrees with the floor"
    );
    assert_eq!(
        census.placements,
        BTreeSet::from(["after", "at", "before", "between", "concurrent"]),
        "the span grid fails its placement liveness floor"
    );
    assert_eq!(
        census.dominances,
        BTreeSet::from(["after", "before", "between"]),
        "the span grid fails its dominance liveness floor"
    );
    assert_eq!(
        census.precedences,
        BTreeSet::from(["after", "before", "between"]),
        "the span grid fails its precedence liveness floor"
    );
    assert_eq!(
        census.coverages,
        BTreeSet::from(["empty", "full", "partial"]),
        "the query grid fails its coverage liveness floor"
    );
    assert_eq!(
        census.memberships,
        BTreeSet::from([false, true]),
        "the query grid fails its membership liveness floor"
    );
    assert!(
        census.rank_ties > 0,
        "no distinct-version rank tie in the pool: the byte-tiebreak leg ran \
         vacuously"
    );
}
