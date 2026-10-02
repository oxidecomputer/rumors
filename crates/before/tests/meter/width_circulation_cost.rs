//! Cost checks for repeated wide minimum reveals during ticking.
//!
//! Each family contains `k` sibling sites that share one `b`-bit minimum. The
//! walk repeatedly closes one site and reveals the shared minimum for the next.
//! Retaining that boundary makes the marginal work narrow; reconstructing or
//! folding the wide minimum at every site costs `Θ(kb)` on a `Θ(k + b)` input.
//! Jointly doubling `k` and `b` therefore separates the two designs: expected
//! work doubles, while the regression approaches four times the work.
//!
//! Every scenario asserts its closed-form tick result. A high-floor control
//! removes the wide gap while preserving the shape, and a pure comb isolates
//! the range-minimum tracker from tick pre-scanning.
//!
//! Derived liveness floors prove the accumulator remains observed. Measured
//! improvement tripwires instead flag a substantial optimization for deliberate
//! re-pinning; the two lower bounds must not be interpreted alike.

use before::testing::meter;
use before::testing::meter::registry::Shape;
use before::Party;
use suanpan::touch_meter;

/// One tick run over a family cross.
struct Run {
    input: u64,
    touches: u64,
}

/// Tick the event × id cross and read the touch counter over the
/// tick body alone.
///
/// Enforces a one-touch-per-eight-input-bytes liveness floor before
/// returning, derived from the walk's irreducible work: every
/// consumed code's magnitude folds into a live accumulator at least
/// once — one digit touch per 64-bit limb of the operand, zero limbs
/// included — and in every family here the folded payload (the
/// circulated wide minimum and the nonzero boundary codes) is at
/// least an eighth of the encoded input, the leanest committed shape
/// (the leveled control) holding roughly a limb of wide payload per
/// site's worth of structure. A reading below the floor means the
/// walk's accumulator work left the metered representation and any
/// ratio over it would hold vacuously.
fn tick_run(ev: meter::Encoding, id: meter::Encoding) -> Run {
    let mut v = ev.version();
    let p = Party::decode(&*id.bytes).expect("the generator's id is canonical");
    let input = (v.encode().len() + id.bytes.len()) as u64;
    touch_meter::reset();
    v.tick(&p);
    let run = Run {
        input,
        touches: touch_meter::touches(),
    };
    assert!(
        run.touches >= run.input / 8,
        "reveal family at {input} input bytes: {} digit touches under the \
         one-per-eight-bytes floor: the walk's accumulator work is not metered",
        run.touches,
    );
    run
}

/// The reveal comb's close-reveal cycle is gap-funded flat —
/// touches grow by at most ×2.5 across the joint (k, b) doubling
/// on a ×2 input, under an absolute band on the larger run.
///
/// The signature is the linear ×2.0 on a ×2.0 input: the
/// consumed width-b boundary difference parks in the latent
/// register at the site's close and the next consume's arm
/// recycles it by a narrow anchor-relative fold, so no hop
/// re-reads the width — a per-site width read reads ~×4 here. A
/// reading over the growth ceiling means a per-site width read is
/// back — re-pin only with a cure, never by deleting the family.
#[test]
fn reveal_comb_close_reveal_cycle_reads_width_quadratic() {
    let small = tick_run(
        Shape::RevealComb.build2(1_000, 1_024),
        Shape::RevealCombId.build1(1_000),
    );
    let large = tick_run(
        Shape::RevealComb.build2(2_000, 2_048),
        Shape::RevealCombId.build1(2_000),
    );
    eprintln!(
        "MEASURED reveal_comb: small={}/{}B large={}/{}B",
        small.touches, small.input, large.touches, large.input,
    );
    assert!(
        u128::from(large.touches) * 2 <= u128::from(small.touches) * 5,
        "reveal_comb: touch growth across the joint doubling exceeds x2.5 \
         ({} -> {}): a per-site width read is back in the close-reveal cycle",
        small.touches,
        large.touches,
    );
    assert!(
        large.touches <= REVEAL_COMB_TOUCH_CEILING,
        "reveal_comb: {} touches at (k, b) = (2,000, 2,048) exceed the pinned \
         ceiling {REVEAL_COMB_TOUCH_CEILING}",
        large.touches,
    );
    assert!(
        large.touches >= REVEAL_COMB_TOUCH_TRIPWIRE,
        "reveal_comb: {} touches read below the {REVEAL_COMB_TOUCH_TRIPWIRE} \
         improvement tripwire (measured x0.75): attribute the improvement \
         and re-pin",
        large.touches,
    );
}

/// Absolute touch ceiling on the reveal comb's larger run: the
/// measured record ×1.25, rounded up (the record and every
/// re-pin's movement live in the pin commits).
const REVEAL_COMB_TOUCH_CEILING: u64 = 77_120;

/// Improvement tripwire paired with [`REVEAL_COMB_TOUCH_CEILING`]:
/// the measured reading ×0.75, rounded down.
///
/// The module comment's tripwire: a trip means the reading
/// improved past the band, not that the meter died — attribute and
/// re-pin.
const REVEAL_COMB_TOUCH_TRIPWIRE: u64 = 46_272;

/// Touch ceiling on the pure comb's larger run: the measured dev-profile
/// reading with 25% headroom, rounded up.
///
/// The accumulator handles each narrow step in its inline state, so widening
/// the one plateau should add only the work needed to read that plateau.
const PURE_COMB_TOUCH_CEILING: u64 = 2_714;

/// Widening a pure comb's one plateau does not multiply the work at every site.
///
/// The site count stays fixed while the plateau width doubles. A correct walk
/// moves the wide boundary once and handles each zero offset in constant work;
/// rereading the plateau at every site would make touches per input byte grow.
/// The shared [`tick_run`] floor independently proves that the touch counter is
/// live.
#[test]
fn pure_comb_cost_is_independent_of_value_width() {
    let small = tick_run(
        Shape::PureComb.build2(1_000, 1_024),
        Shape::PureCombId.build1(1_000),
    );
    let large = tick_run(
        Shape::PureComb.build2(1_000, 2_048),
        Shape::PureCombId.build1(1_000),
    );
    eprintln!(
        "MEASURED pure_comb: small={}/{}B large={}/{}B",
        small.touches, small.input, large.touches, large.input,
    );
    assert!(
        u128::from(large.touches) * u128::from(small.input) * 100
            <= u128::from(small.touches) * u128::from(large.input) * 115,
        "pure_comb: per-byte touch growth across the width doubling exceeds \
         x1.15 ({}/{}B -> {}/{}B): the base stack's arm-move + close-pop cycle \
         has picked up a width term",
        small.touches,
        small.input,
        large.touches,
        large.input,
    );
    assert!(
        large.touches <= PURE_COMB_TOUCH_CEILING,
        "pure_comb: {} touches at (k, b) = (1,000, 2,048) exceed the pinned \
         ceiling {PURE_COMB_TOUCH_CEILING}",
        large.touches,
    );
}

/// Absolute touch ceiling on the high-floor control's larger run:
/// the measured record ×1.25, rounded up (the record and every
/// re-pin's movement live in the pin commits).
const HIFLOOR_TOUCH_CEILING: u64 = 42_292;

/// Improvement tripwire paired with [`HIFLOOR_TOUCH_CEILING`]: the
/// measured reading ×0.75, rounded down.
///
/// The module comment's tripwire: a trip means the reading
/// improved past the band, not that the meter died — attribute and
/// re-pin.
const HIFLOOR_TOUCH_TRIPWIRE: u64 = 25_374;

/// The high-floor control is flat and width-independent
/// — identical forest, identical deferral and close-reveal cycle,
/// consume-time gap 2.
///
/// Per-byte touches stay flat (×1.25) across the width QUADRUPLING
/// the wide family scales with, under an absolute
/// band on the larger run. The wide GAP is the cycle's cost driver
/// — not the site forest, not the deferral, not the close-reveal
/// schedule, all of which this family shares with the wide one.
#[test]
fn reveal_comb_hifloor_control_is_flat_per_unit() {
    let small = tick_run(
        Shape::RevealCombHifloor.build2(1_000, 512),
        Shape::RevealCombId.build1(1_000),
    );
    let large = tick_run(
        Shape::RevealCombHifloor.build2(1_000, 2_048),
        Shape::RevealCombId.build1(1_000),
    );
    eprintln!(
        "MEASURED reveal_comb_hifloor: small={}/{}B large={}/{}B",
        small.touches, small.input, large.touches, large.input,
    );
    assert!(
        u128::from(large.touches) * u128::from(small.input) * 4
            <= u128::from(small.touches) * u128::from(large.input) * 5,
        "reveal_comb_hifloor: per-byte touch cost grew more than x1.25 across \
         the width quadrupling: {}/{}B -> {}/{}B — the narrow-gap cycle has \
         picked up a width term",
        small.touches,
        small.input,
        large.touches,
        large.input,
    );
    assert!(
        large.touches <= HIFLOOR_TOUCH_CEILING,
        "reveal_comb_hifloor: {} touches exceed the pinned ceiling \
         {HIFLOOR_TOUCH_CEILING}",
        large.touches,
    );
    assert!(
        large.touches >= HIFLOOR_TOUCH_TRIPWIRE,
        "reveal_comb_hifloor: {} touches read below the \
         {HIFLOOR_TOUCH_TRIPWIRE} improvement tripwire (measured x0.75): \
         attribute the improvement and re-pin",
        large.touches,
    );
}

/// Absolute touch ceiling on the undercut cascade's larger run:
/// the measured record ×1.25, rounded up (the record and every
/// re-pin's movement live in the pin commits).
///
/// The record's regime is the at-height arm's no-fold move.
const ASCEND_CLIFF_TOUCH_CEILING: u64 = 18_560;

/// Touch liveness floor on the undercut cascade's larger run,
/// derived from the cascade's irreducible work — never from a
/// measured basis.
///
/// Four per-boundary charges the cascade mechanism cannot avoid —
/// two per boundary *created* (one fold of each of the k − 1
/// consumed nonzero unit codes into the running height, one sign
/// read per arm deciding each pushed boundary's trichotomy) and two
/// per boundary *penetrated* (one domination read plus one dying
/// fold per boundary the cascade consumes, the difference dying
/// into the residue at its own width, at least one digit each).
/// The two counts coincide at k − 1 here because the one cascade
/// penetrates every boundary. The wide
/// cliff code folds into the running height once, at one touch per
/// 64-bit limb. At (k, b) = (2,000, 4,096):
/// 4·(k − 1) + b/64 = 7,996 + 64. A design that validly does less
/// is a floor-premise finding — re-derive the premise before
/// trusting the trip.
const ASCEND_CLIFF_TOUCH_FLOOR: u64 = 8_060;

/// The undercut cascade is dying-digit-funded flat — touches grow
/// by at most ×2.5 across the joint (k, b) doubling on a ×2 input,
/// under an absolute band on the larger run.
///
/// The cliff's single wide undercut
/// penetrates k − 1 nonzero unit boundary differences, each dying
/// by one fold into the surviving residue at the difference's own
/// width, top-index domination deciding every hop in O(1) — a
/// per-hop residue-width read reads ~×4 here. A
/// reading over the growth ceiling means a per-hop residue-width
/// read is back — re-pin only with a cure, never by deleting the
/// family.
#[test]
fn ascend_cliff_undercut_cascade_reads_residue_width() {
    let small = tick_run(
        Shape::AscendCliff.build2(1_000, 2_048),
        Shape::AscendCliffId.build1(1_000),
    );
    let large = tick_run(
        Shape::AscendCliff.build2(2_000, 4_096),
        Shape::AscendCliffId.build1(2_000),
    );
    eprintln!(
        "MEASURED ascend_cliff: small={}/{}B large={}/{}B",
        small.touches, small.input, large.touches, large.input,
    );
    assert!(
        u128::from(large.touches) * 2 <= u128::from(small.touches) * 5,
        "ascend_cliff: touch growth across the joint doubling exceeds x2.5 \
         ({} -> {}): a per-hop residue-width read is back in the undercut \
         cascade",
        small.touches,
        large.touches,
    );
    assert!(
        large.touches <= ASCEND_CLIFF_TOUCH_CEILING,
        "ascend_cliff: {} touches at (k, b) = (2,000, 4,096) exceed the pinned \
         ceiling {ASCEND_CLIFF_TOUCH_CEILING}",
        large.touches,
    );
    assert!(
        large.touches >= ASCEND_CLIFF_TOUCH_FLOOR,
        "ascend_cliff: {} touches read below the {ASCEND_CLIFF_TOUCH_FLOOR} \
         liveness floor (the cascade's derived irreducible work): the \
         cascade's work left the metered representation",
        large.touches,
    );
}

/// Absolute touch ceiling on the leveled control's larger run: the
/// measured record ×1.25, rounded up (the record and every
/// re-pin's movement live in the pin commits).
const PLATEAU_TOUCH_CEILING: u64 = 3_568;

/// Touch liveness floor paired with [`PLATEAU_TOUCH_CEILING`],
/// derived from the walk's irreducible work on this family — never
/// from a measured basis.
///
/// Every arm reads its pushed boundary offset's sign — one touch,
/// even for this family's all-zero boundaries — and the wide first
/// raise folds into the running height once, at one touch per
/// 64-bit limb; the all-zero difference stack passes the final
/// undercut whole, so the cascade owes nothing further. At
/// (k, b) = (2,000, 4,096): (k − 1) + b/64 = 1,999 + 64.
const PLATEAU_TOUCH_FLOOR: u64 = 2_063;

/// The leveled control is flat: identical spine,
/// identical arming schedule, identical cliff undercut, all
/// boundary differences zero.
///
/// Per-byte touches stay flat (×1.25) across the joint (k, b)
/// doubling the ascending family scales with,
/// under an absolute band on the larger run. The
/// nonzero differences are the cascade's cost driver — with the
/// stack one compressed zero run, the same wide undercut passes it
/// whole in O(1) — so the hop schedule, not the undercut or the
/// spine, carries the red family's growth.
#[test]
fn ascend_cliff_plateau_control_is_flat_per_unit() {
    let small = tick_run(
        Shape::AscendCliffPlateau.build2(1_000, 2_048),
        Shape::AscendCliffId.build1(1_000),
    );
    let large = tick_run(
        Shape::AscendCliffPlateau.build2(2_000, 4_096),
        Shape::AscendCliffId.build1(2_000),
    );
    eprintln!(
        "MEASURED ascend_cliff_plateau: small={}/{}B large={}/{}B",
        small.touches, small.input, large.touches, large.input,
    );
    assert!(
        u128::from(large.touches) * u128::from(small.input) * 4
            <= u128::from(small.touches) * u128::from(large.input) * 5,
        "ascend_cliff_plateau: per-byte touch cost grew more than x1.25 across \
         the joint doubling: {}/{}B -> {}/{}B — the zero-run cascade has \
         picked up a width term",
        small.touches,
        small.input,
        large.touches,
        large.input,
    );
    assert!(
        large.touches <= PLATEAU_TOUCH_CEILING,
        "ascend_cliff_plateau: {} touches exceed the pinned ceiling \
         {PLATEAU_TOUCH_CEILING}",
        large.touches,
    );
    assert!(
        large.touches >= PLATEAU_TOUCH_FLOOR,
        "ascend_cliff_plateau: {} touches read below the {PLATEAU_TOUCH_FLOOR} \
         liveness floor (the walk's derived irreducible work): the cascade's \
         work left the metered representation",
        large.touches,
    );
}
