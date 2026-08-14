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
use std::collections::{BTreeSet, HashMap};

use before::meter::registry::{FamilyId, Shape};
use before::meter::Packed;
use before::{Clock, Party, Version};
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
