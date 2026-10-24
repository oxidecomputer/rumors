//! The interleaving-fuel property, deterministic cases at boundaries of the
//! harness's own model, and the census showing the skeletons reach the
//! regimes the checks rely on.
//!
//! The property runs [`GATE_CASES`] cases unless `PROPTEST_CASES` says
//! otherwise; each case runs one guest instance per variant `judge`
//! describes, one of them at values of a few megabytes and one over 4096
//! repetitions. The census runs the model
//! alone, from a fixed seed.

use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::{Config, RngAlgorithm, TestRng, TestRunner};
use suanpan_fuel_harness::guest;
use suanpan_fuel_harness::judge::{
    census, growth_bound, has_fixed_steady_charge, judge, regimes, BlockOutcome, Census,
    COPY_GROWTH, SCALE_STEP,
};
use suanpan_fuel_harness::skeleton::{arb_skeleton, Extent, Skeleton, UNIT_BITS};
use suanpan_fuel_protocol::{
    observe_stable, Direction, Form, LimbInput, Member, Pattern, Primitive, SignQuery, Step,
};

/// The property's case count when `PROPTEST_CASES` is unset.
///
/// A case takes about a tenth of a second on a shared host, so the default
/// run takes about half a minute, far inside nextest's time limit;
/// investigative runs raise `PROPTEST_CASES` into the thousands.
const GATE_CASES: u32 = 256;

/// Returns the property's case count: `PROPTEST_CASES` if set, otherwise
/// [`GATE_CASES`].
fn cases() -> u32 {
    std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|cases| cases.parse().ok())
        .unwrap_or(GATE_CASES)
}

proptest! {
    #![proptest_config(Config::with_cases(cases()))]

    /// Over random interleavings of accumulator operations, fuel grows only
    /// as the bounds in `judge` allow when sizes and repetitions grow.
    ///
    /// Each case's skeleton (a context, a value-neutral block run 12 times,
    /// and a clear of every member) runs at units `u`, `64 · u`, and
    /// `4096 · u`, and at `u` over 8 and 4096 repetitions. At the end of every
    /// repetition two units share, the fuel net of reallocation copies costs
    /// at most the growth bound times its cost at the unit below
    /// (`judge::growth_bound`), and the copies at most twice that; a prefix
    /// that is the same program at every unit costs exactly the same. Every
    /// prefix of the long run
    /// costs what the 8-repetition run costs over their shared prefix, and
    /// afterwards at most a seventh of that run's total for each repetition
    /// after the first. A skeleton whose repetitions charge no sized term once
    /// storage is grown costs, per repetition over the last four of eight, at
    /// most twice as much at `4096 · u` as at `64 · u`, and so does each
    /// call of a fixed-charge block once its cost repeats exactly. Every
    /// prefix also spends at least its steps' floors, and every step's
    /// answer agrees with the exact model's values.
    #[test]
    fn interleavings_cost_within_the_rules(skeleton in arb_skeleton()) {
        match judge(&skeleton) {
            Ok(margins) => eprintln!("margins: {margins:?}"),
            Err(violation) => return Err(TestCaseError::fail(violation)),
        }
    }
}

/// Returns a single-limb update of `member` by 1 at `shift`.
fn one(member: Member, direction: Direction, shift: Extent) -> Step<Extent> {
    Step::Limbs {
        member,
        direction,
        shift,
        input: LimbInput {
            pattern: Pattern::Sparse,
            count: Extent::fixed(1),
        },
    }
}

/// Returns `units · u + offset` bits.
fn at(units: u64, offset: i64) -> Extent {
    Extent { units, offset }
}

/// Judges `context` with no block, panicking with the violation if a bound
/// is exceeded.
fn judge_context(context: Vec<Step<Extent>>) {
    if let Err(violation) = judge(&Skeleton::new(context, Vec::new())) {
        panic!("{violation}");
    }
}

/// An owned addition of two zeros, then a gap round reaching up to `u`, in
/// every repetition, passes the judgment.
///
/// The addition returns its operand, a new accumulator, so the gap round
/// grows storage again in every repetition, and the model charges that
/// growth rather than counting the repetition as fixed-charge.
#[test]
fn growth_after_an_owned_addition_of_zeros_is_charged() {
    let (a, b) = (Member::ALL[0], Member::ALL[1]);
    judge_context(vec![
        Step::Operand {
            receiver: b,
            operand: a,
            direction: Direction::Add,
            form: Form::OwnedClone,
        },
        one(b, Direction::Add, Extent::fixed(0)),
        one(b, Direction::Add, at(1, 0)),
        one(b, Direction::Add, Extent::fixed(64)),
        one(b, Direction::Sub, at(1, 0)),
        one(b, Direction::Sub, Extent::fixed(64)),
        one(b, Direction::Sub, Extent::fixed(0)),
    ]);
}

/// Two owned additions between values whose top digits coincide, the
/// receiver one bit narrower, pass the judgment.
///
/// The implementation keeps the receiver's storage on a tie of stored digit
/// counts, so the storage two units high regrows in every repetition, and
/// the model, unable to tell which storage survives, charges the regrowth.
#[test]
fn growth_after_an_owned_addition_of_close_widths_is_charged() {
    let (a, b) = (Member::ALL[0], Member::ALL[1]);
    let prim = |member, value| Step::Primitive {
        member,
        direction: Direction::Add,
        value: Primitive::U128(value),
    };
    let take = |receiver, operand| Step::Operand {
        receiver,
        operand,
        direction: Direction::Add,
        form: Form::OwnedTake,
    };
    judge_context(vec![
        one(a, Direction::Add, at(2, 0)),
        one(a, Direction::Sub, at(2, 0)),
        prim(a, 1 << 100),
        prim(b, 1 << 99),
        take(b, a),
        one(b, Direction::Add, at(2, 0)),
        one(b, Direction::Sub, at(2, 0)),
        prim(a, 1 << 99),
        take(a, b),
    ]);
}

/// A stability query on a value the query may decline to order passes: the
/// guest's refusal is a correct answer whatever the value, so the comparison
/// with the model accepts it beside the model's order.
#[test]
fn a_declined_stability_query_agrees_with_the_model() {
    let a = Member::ALL[0];
    let park = |word| Step::Limbs {
        member: a,
        direction: Direction::Sub,
        shift: Extent::fixed(64),
        input: LimbInput {
            pattern: Pattern::Word(word),
            count: Extent::fixed(1),
        },
    };
    let context = vec![
        park(1 << 32),
        park((1 << 32) - 1),
        Step::Sign {
            member: a,
            query: SignQuery::StableUnder(Extent::fixed(33)),
        },
    ];
    let run = guest::run(&Skeleton::new(context.clone(), Vec::new()).program(UNIT_BITS, 1));
    assert_eq!(
        run.observations[2],
        observe_stable(None),
        "the guest declines to order the value, so the case exercises the refusal"
    );
    judge_context(context);
}

/// A fixed-charge block that updates and questions a member the context
/// left a unit wide is checked, and passes.
///
/// Its calls cost the same at the end of its repetitions, and at the top
/// unit at most twice what they cost at the middle.
#[test]
fn a_fixed_charge_block_over_a_wide_member_is_checked() {
    let a = Member::ALL[0];
    let context = vec![Step::Limbs {
        member: a,
        direction: Direction::Add,
        shift: at(1, 0),
        input: LimbInput {
            pattern: Pattern::Mixed(7),
            count: at(1, 0),
        },
    }];
    let block = vec![
        Step::Primitive {
            member: a,
            direction: Direction::Add,
            value: Primitive::U64(3),
        },
        Step::Sign {
            member: a,
            query: SignQuery::Sign,
        },
        Step::Primitive {
            member: a,
            direction: Direction::Sub,
            value: Primitive::U64(3),
        },
    ];
    let skeleton = Skeleton::new(context, block);
    assert!(
        regimes(&skeleton).block_in_sized_context,
        "the block touches a member at least a unit wide"
    );
    match judge(&skeleton) {
        Ok(margins) => assert!(
            matches!(margins.block, BlockOutcome::Checked(_)),
            "the block check applies: {:?}",
            margins.block
        ),
        Err(violation) => panic!("{violation}"),
    }
}

/// A bitset level that reallocates at `64 · u` but not at `u` stays within
/// the copy allowance, and the case passes.
///
/// One member takes a top-only value by `clone_from` each repetition, so its
/// digits never reallocate and its written set starts fresh; then two
/// one-limb writes in the gap below its run straddle `2 · u`, which lies
/// inside one 64-digit bitset word at `u` and on a word boundary at
/// `64 · u`. The second write reallocates a bitset level at the upper unit
/// alone, the case `COPY_GROWTH`'s derivation bounds.
#[test]
fn a_bitset_level_reallocating_at_the_upper_unit_alone_stays_within_the_copy_allowance() {
    let (a, b) = (Member::ALL[0], Member::ALL[1]);
    let write = |member, pattern, shift| Step::Limbs {
        member,
        direction: Direction::Add,
        shift,
        input: LimbInput {
            pattern,
            count: Extent::fixed(1),
        },
    };
    let context = vec![
        write(a, Pattern::Ones, at(3, -64)),
        Step::CloneFrom {
            receiver: b,
            source: a,
        },
        write(b, Pattern::Sparse, at(2, -32)),
        write(b, Pattern::Sparse, at(2, 0)),
    ];
    let skeleton = Skeleton::new(context.clone(), Vec::new());
    let n = skeleton.steps().len();
    let (lower, upper) = (UNIT_BITS, SCALE_STEP * UNIT_BITS);
    let [low, high] = [lower, upper].map(|unit| guest::run(&skeleton.program(unit, 1)));
    let straddle = 3;
    let copied = |run: &guest::Run| run.copied[straddle + 1] - run.copied[straddle];
    assert_eq!(copied(&low), 0, "the straddling write copies nothing at u");
    assert!(
        copied(&high) > 0,
        "the straddling write reallocates a bitset level at 64u"
    );
    let bound = growth_bound(SCALE_STEP, lower, skeleton.offset_bound());
    let (low_copies, high_copies) = (low.copied_prefix(n), high.copied_prefix(n));
    assert!(
        high_copies as f64 <= COPY_GROWTH * bound * low_copies as f64,
        "the repetition's copies, {high_copies} bytes at 64u against {low_copies} at u, \
         grow by at most {} times",
        COPY_GROWTH * bound
    );
    judge_context(context);
}

/// The seed of the census's skeleton stream.
const CENSUS_SEED: [u8; 32] = *b"suanpan-fuel interleaving census";

/// The number of skeletons the census draws.
const CENSUS_SKELETONS: usize = 64;

/// The skeleton strategy realizes every regime the property relies on, at
/// the base unit and at the top unit alike.
///
/// The step regimes are steps of opposite signs, cancellations of half a
/// unit or more, borrows across runs half a unit long, storage growth,
/// sparse high digits, and shifted updates, counted at both units. The
/// skeleton regimes are those in which the two fixed-charge checks compare
/// work over storage that scales: a steady repetition whose storage widths
/// differ between units, and a block that touches a member at least a unit
/// wide.
///
/// The census replays a fixed stream of skeletons in the model alone. Each
/// floor asserts that its regime is alive, not how often it occurs: the
/// floors sit at a quarter of the counts the stream realized when they were
/// set, so they need re-measuring only when the strategy or proptest's draws
/// deliberately change.
#[test]
fn skeletons_reach_every_regime_at_both_scales() {
    let mut runner = TestRunner::new_with_rng(
        Config::default(),
        TestRng::from_seed(RngAlgorithm::ChaCha, &CENSUS_SEED),
    );
    let strategy = arb_skeleton();
    let top = SCALE_STEP * SCALE_STEP * UNIT_BITS;
    let (mut base, mut scaled) = (Census::default(), Census::default());
    let (mut fixed_steady, mut scaled_steady, mut wide_blocks) = (0, 0, 0);
    for _ in 0..CENSUS_SKELETONS {
        let skeleton = strategy
            .new_tree(&mut runner)
            .expect("the skeleton strategy generates")
            .current();
        base += census(&skeleton, UNIT_BITS);
        scaled += census(&skeleton, top);
        fixed_steady += usize::from(has_fixed_steady_charge(&skeleton));
        let reached = regimes(&skeleton);
        scaled_steady += usize::from(reached.scaled_steady);
        wide_blocks += usize::from(reached.block_in_sized_context);
    }
    eprintln!(
        "census over {CENSUS_SKELETONS} skeletons: base {base:?}, top {scaled:?}, fixed-charge \
         steady repetitions {fixed_steady}, of them over scaled storage {scaled_steady}, \
         fixed-charge blocks touching wide members {wide_blocks}"
    );
    // Each floor is a quarter of the stream's count when it was set.
    let regimes = [
        (
            "steps of opposite signs",
            291,
            base.mixed_signs,
            scaled.mixed_signs,
        ),
        (
            "cancellations",
            114,
            base.cancellations,
            scaled.cancellations,
        ),
        ("borrow runs", 27, base.borrow_runs, scaled.borrow_runs),
        ("storage growths", 29, base.growths, scaled.growths),
        (
            "sparse high digits",
            26,
            base.sparse_values,
            scaled.sparse_values,
        ),
        (
            "shifted updates",
            469,
            base.shifted_updates,
            scaled.shifted_updates,
        ),
    ];
    for (regime, floor, at_base, at_top) in regimes {
        assert!(
            at_base >= floor && at_top >= floor,
            "the census realizes at least {floor} {regime} at each unit, found {at_base} at the \
             base unit and {at_top} at the top unit"
        );
    }
    for (regime, floor, count) in [
        ("steady repetitions over scaled storage", 2, scaled_steady),
        ("fixed-charge blocks touching wide members", 2, wide_blocks),
    ] {
        assert!(
            count >= floor,
            "the census realizes at least {floor} skeletons with {regime}, found {count}"
        );
    }
}
