//! Measures a skeleton at every scale and repetition count, and judges the
//! fuel readings against one another.
//!
//! A skeleton holds a context, a value-neutral block called
//! [`BLOCK_REPETITIONS`] times, and an epilogue that clears every member.
//! [`judge`] runs these variants of it, each in a fresh guest instance:
//!
//! | Variant | Unit | Repetitions | Role |
//! |---|---|---|---|
//! | base | `u` | [`BASE_REPETITIONS`] | the control for the others |
//! | middle | `64 · u` | [`BASE_REPETITIONS`] | scaled once |
//! | top | `4096 · u` | 1, or [`BASE_REPETITIONS`] when the steady repetition has a fixed charge | scaled twice |
//! | long | `u` | [`LONG_REPETITIONS`] | repeated |
//!
//! Because of the epilogue, each repetition starts from zero values over the
//! storage the last one left, and repetitions after the first compute the
//! same values as the first. The readings are judged against each other,
//! never against a committed number.
//!
//! # The checks, and what each can see
//!
//! - **Scaled repetitions.** At the end of every repetition the units
//!   share, the fuel net of reallocation copies may be at most
//!   [`growth_bound`] times its value at the unit below, and the copies at
//!   most [`COPY_GROWTH`] times that; a prefix that is the same program at
//!   every unit must cost exactly the same. This sees work that grows faster
//!   than the sizes anywhere in a repetition: quadratic work, or a logarithm
//!   multiplying a length.
//! - **Long repetition.** The 4096-repetition run must match the base over
//!   their shared prefix and afterwards cost at most a seventh of the base's
//!   total per repetition after the first. This sees work that grows with
//!   the length of the history at a fixed size: positions never reclaimed,
//!   a scan that revisits what earlier repetitions wrote.
//! - **Steady repetition.** A skeleton whose repetitions charge no sized
//!   term once storage is grown may cost, per repetition over the last
//!   [`STEADY_REPETITIONS`] of [`BASE_REPETITIONS`], at most
//!   [`STEADY_GROWTH_BOUND`] times as much at the top unit as at the middle.
//!   This sees work linear in a size, paid again in every repetition: growth
//!   paid again after a clear, a gap crossed at a cost linear in its width.
//! - **Block calls.** A block whose calls charge no sized term may cost, per
//!   call once its cost repeats exactly, at most [`STEADY_GROWTH_BOUND`]
//!   times as much at the top unit as at the middle, however wide the
//!   members the context left. Equal costs over the last calls are taken as
//!   evidence that the state repeats; when it does, the potential returns to
//!   where it was over each call, so differencing removes whatever the
//!   context deferred and leaves the call's own cost. This sees a step of a
//!   kind a block holds (an update by a primitive or by fixed-count limbs, a
//!   sign query, a comparison with a primitive, a conversion that refuses, an
//!   owned negation) whose work is linear in the width of a member it
//!   touches, inside a program that charges sizes elsewhere: an update that
//!   pays its receiver's width, a sign query that rescans a wide value.
//! - **Floors and answers.** Every prefix spends at least its steps' floors,
//!   so the fuel is counting, and every step's answer agrees with the exact
//!   model's values.
//!
//! What remains unseen: work linear in a size the rules do not charge to a
//! step whose charge has a sized term of its own (an operand addition that
//! also pays its receiver's width), because that term scales by the same
//! factor; and work proportional to the context that is paid once, which
//! the block check differences away and no reading can tell from amortized
//! work.
//!
//! The premise of every comparison is that the guest's work depends on
//! sizes alone. The guest's allocator moves every block it reallocates (its
//! `moving` module): an allocator that grows blocks in place where the heap
//! allows it copies at one scale what it extends at another, and that copy,
//! which the third rule allows, would make readings grow faster than the
//! sizes.

use std::ops::AddAssign;

use suanpan_fuel_protocol::{observe_stable, Step};

use crate::guest::{self, Run};
use crate::model::{Events, Model, Record};
use crate::skeleton::{Skeleton, BLOCK_REPETITIONS, BLOCK_WINDOW, UNIT_BITS};

/// The repetitions of the base, middle, and fixed-charge top variants.
pub const BASE_REPETITIONS: u32 = 8;

/// The repetitions of the long variant.
pub const LONG_REPETITIONS: u32 = 4096;

/// The factor between consecutive scales.
pub const SCALE_STEP: u64 = 64;

/// How far the scaled check lets fuel net of reallocation copies grow at a
/// repetition boundary: `k + (k − 1) · 2D / (unit − 2D)`.
///
/// Here every size grows by `k = scale` from a program at unit `unit` whose
/// structural offsets are at most `D = offset_bound` bits.
///
/// The rules bound work only by an unnamed constant times the charge, and
/// allow no smaller factor across scales: the constants differ from term to
/// term, so a correct program can raise its fuel per unit of charge as it
/// scales. The bound instead follows from the structure of `suanpan`'s work,
/// as its cost derivation lays it out. Every unit of work is one of these,
/// and each grows by at most the bound:
///
/// - a constant, such as a call's dispatch or an allocator call (whose
///   `dlmalloc` bin walk is bounded by its 32-bit size field), which grows
///   by a bounded factor far below `k`;
/// - an allocator's slow path, such as growing the heap or splitting or
///   coalescing a chunk, a bounded constant per call;
/// - work proportional to a length that runs between two positions of the
///   program's structure: such a length is `d · unit + δ` bits with
///   `|δ| ≤ 2D` ([`Skeleton::offset_bound`]), so it grows by at most
///   `(k · unit − 2D) / (unit − 2D)`, the bound itself;
/// - a climb through the written-position bitset, whose levels are base-64
///   digits of the distance climbed: a climb costs a constant plus a step per
///   level, a distance that scales spans at least one level already at the
///   base unit, whose positions lie past the first word ([`UNIT_BITS`]),
///   and growing it 64-fold adds at most one level, so a climb across a
///   distance that scales grows by at most three halves, and one across a
///   fixed distance that straddles a multiple of the unit, which spans no
///   level at the base unit and one at `64 · u`, by at most twice: a bounded
///   factor far below `k`;
/// - a walk of a bitset subtree, as a clear or a scan makes, which costs its
///   positions plus its words plus one word per level, each growing as the
///   kinds above do;
/// - a reallocation's copy, which the guest counts and the check judges
///   apart ([`COPY_GROWTH`]).
///
/// The comparison is made only where every member has just been cleared, at
/// the end of a repetition. Between such points a correct implementation can
/// defer work to a later step at one scale and do it at once at another,
/// as a clear that pays what earlier steps left can; single steps then grow
/// by more than any term above. Over a whole repetition every such deferral
/// has come due: the epilogue's clears pay what they owe, and what remains
/// (storage reserved but not yet covered by the bitset) is the same
/// structure at every unit.
///
/// # Panics
///
/// Panics unless `2 · offset_bound < unit`, which the skeleton budget keeps.
pub fn growth_bound(scale: u64, unit: u64, offset_bound: u64) -> f64 {
    let slop = 2 * offset_bound;
    assert!(slop < unit, "the offset budget keeps 2D below the unit");
    let k = scale as f64;
    k + (k - 1.0) * slop as f64 / (unit - slop) as f64
}

/// How far the bytes reallocations copy may grow, at a repetition boundary,
/// relative to [`growth_bound`]: twice.
///
/// Every capacity a buffer requests is a length of the program, twice an
/// earlier capacity, or the allocator's fixed minimum, so it is
/// `d · unit + δ` with the same `δ` at every unit, and a buffer that
/// reallocates at one unit reallocates at the same step at the other. Its
/// copies, the capacities it leaves, then grow by at most the bound, except
/// for the fixed minimum a doubling starts from: a buffer that doubles from
/// minimum capacity `m` to `c` copies `c − m`, and `(64 · c − m) / (c − m)`
/// stays below 128 for every `c ≥ 2m`. A level of the written-position
/// bitset can reallocate at the upper unit alone, when two positions
/// straddle a multiple of the unit inside one 64-digit word at the base
/// unit; the height budget leaves room for such a straddle only at one or
/// two units, so it copies at most 1,280 bytes, and the bitset's creation
/// copies its level array, at least 12 bytes, at both units, which keeps
/// that ratio below 128 too.
pub const COPY_GROWTH: f64 = 2.0;

/// How far a steady repetition's or a block call's fuel may grow from unit
/// `64 · u` to `4096 · u` when no step's charge has a sized term: twice.
///
/// Such work is a constant per step plus climbs through the
/// written-position bitset, whose levels are base-64 digits of the distance
/// climbed (rule 4's logarithm). At `64 · u` every distance that scales
/// spans at least one level, since `64 · u / 32` positions exceed 64, and
/// multiplying a distance by 64 adds at most one level. So each climb at
/// most doubles, and every constant stays put.
pub const STEADY_GROWTH_BOUND: f64 = 2.0;

/// The repetitions, at the end of a [`BASE_REPETITIONS`] run, over which the
/// steady repetition's cost is averaged.
///
/// The repetitions before them can still pay one-time growth that the rules
/// amortize against the first repetition's storage growth. For example, after
/// the first clear, a one-limb write just below a far run converts the
/// written-position set to a bitset covering every position below it, one
/// bit per position, once. Such work lands in the second repetition, not in
/// every one, so it belongs to no steady repetition.
pub const STEADY_REPETITIONS: u32 = BASE_REPETITIONS / 2;

/// What one judged skeleton showed: how close each reading came to its
/// bound, as a fraction of the bound.
#[derive(Clone, Copy, Debug, Default)]
pub struct Margins {
    /// The largest middle-over-base ratio at a repetition boundary, over its
    /// bound.
    pub base_to_middle: f64,
    /// The largest top-over-middle ratio at a repetition boundary, over its
    /// bound.
    pub middle_to_top: f64,
    /// The largest long-variant prefix, over its bound.
    pub repetition: f64,
    /// The top steady repetition over the middle one, over its bound, when
    /// the steady repetition has a fixed charge.
    pub steady: Option<f64>,
    /// What became of the block check (see [`BlockOutcome`]).
    pub block: BlockOutcome,
    /// The smallest slack the scaled check left at the end of the first
    /// repetition, in fuel per digit of storage grown.
    ///
    /// The digits are those the repetition grows at the upper unit, so this
    /// is how much a superlinear term would have to spend per digit to fail.
    /// Infinite where nothing scales.
    pub scaled_slack_per_digit: f64,
    /// The steps in one repetition of the skeleton, epilogue included.
    pub steps: usize,
    /// Each variant's total fuel: base, middle, top, and long.
    pub fuel: [u64; 4],
}

/// One variant: its run, and the model's records of its first two
/// repetitions.
struct Variant {
    /// The guest's run.
    run: Run,
    /// The model's record of every step of the first repetition, then of the
    /// second, which every later repetition repeats.
    records: Vec<Record>,
}

/// Returns the model's records of the first two repetitions of `steps` at
/// `unit`.
fn model(steps: &[Step], unit: u64) -> Vec<Record> {
    let mut model = Model::new(unit, false);
    (0..2)
        .flat_map(|_| steps.iter())
        .map(|step| model.step(step))
        .collect()
}

/// Returns the record of step instance `index` (counting from zero) of a
/// run, from the records of its first two repetitions of `n` steps.
fn record(records: &[Record], n: usize, index: usize) -> &Record {
    if index < n {
        &records[index]
    } else {
        &records[n + index % n]
    }
}

/// Returns whether a skeleton's steady repetition charges the same sized
/// terms at the middle and top units.
///
/// The records are those of the first two repetitions at each unit. Equal
/// terms mean that, once storage is grown, no step reads, copies, produces,
/// or grows by a size that scales.
fn steady_charge_is_fixed(middle: &[Record], top: &[Record], n: usize) -> bool {
    (0..n).all(|i| middle[n + i].charge.sized_terms() == top[n + i].charge.sized_terms())
}

/// Which of the regimes that decide whether a check applies a skeleton
/// reaches, measured in the model alone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Regimes {
    /// The steady repetition charges no sized term, yet its storage widths
    /// differ between the middle and top units: the steady check compares
    /// work over storage that scales, not two identical programs.
    pub scaled_steady: bool,
    /// The block charges no sized term, and a member it touches is at least
    /// a unit wide when the block starts at the base unit: the block check
    /// compares a fixed-charge call against wide storage.
    pub block_in_sized_context: bool,
}

/// Returns the regimes `skeleton` reaches.
pub fn regimes(skeleton: &Skeleton) -> Regimes {
    let n = skeleton.steps().len();
    let [middle, top] = [SCALE_STEP * UNIT_BITS, SCALE_STEP * SCALE_STEP * UNIT_BITS]
        .map(|unit| model(&skeleton.steps_at(unit), unit));
    let scaled_steady = steady_charge_is_fixed(&middle, &top, n)
        && (0..n).any(|i| middle[n + i].charge.height_bits != top[n + i].charge.height_bits);

    let steps = skeleton.steps_at(UNIT_BITS);
    let mut base = Model::new(UNIT_BITS, false);
    for step in &steps[..skeleton.context_len()] {
        base.step(step);
    }
    let block = &steps[skeleton.context_len()..skeleton.context_len() + skeleton.block_len()];
    let touches_wide = block.iter().any(|step| {
        let member = match step {
            Step::Primitive { member, .. }
            | Step::Limbs { member, .. }
            | Step::Negate { member, .. }
            | Step::Sign { member, .. }
            | Step::ComparePrimitive { member, .. }
            | Step::Convert { member } => *member,
            _ => return false,
        };
        crate::model::width(&base.values()[member.index()]) >= UNIT_BITS
    });
    let block_in_sized_context =
        touches_wide && skeleton.block_len() > 0 && block_charge_is_fixed(skeleton, &middle, &top);
    Regimes {
        scaled_steady,
        block_in_sized_context,
    }
}

/// Returns whether `skeleton`'s steady repetition has a fixed charge, so
/// [`judge`] compares it at the middle and top units.
pub fn has_fixed_steady_charge(skeleton: &Skeleton) -> bool {
    let [middle, top] = [SCALE_STEP * UNIT_BITS, SCALE_STEP * SCALE_STEP * UNIT_BITS]
        .map(|unit| model(&skeleton.steps_at(unit), unit));
    steady_charge_is_fixed(&middle, &top, skeleton.steps().len())
}

/// Measures every variant of `skeleton` and judges the readings.
///
/// # Errors
///
/// Returns a description of the first bound a reading exceeded, with the
/// skeleton, the step, the readings, and their charges.
pub fn judge(skeleton: &Skeleton) -> Result<Margins, String> {
    let n = skeleton.steps().len();
    let units = [
        UNIT_BITS,
        SCALE_STEP * UNIT_BITS,
        SCALE_STEP * SCALE_STEP * UNIT_BITS,
    ];
    let steps = units.map(|unit| skeleton.steps_at(unit));
    let records = [0, 1, 2].map(|i| model(&steps[i], units[i]));
    let fixed_steady = steady_charge_is_fixed(&records[1], &records[2], n);
    let top_repetitions = if fixed_steady { BASE_REPETITIONS } else { 1 };
    let [base_records, middle_records, top_records] = records;

    let base = Variant {
        run: guest::run(&skeleton.program(units[0], BASE_REPETITIONS)),
        records: base_records.clone(),
    };
    let middle = Variant {
        run: guest::run(&skeleton.program(units[1], BASE_REPETITIONS)),
        records: middle_records,
    };
    let top = Variant {
        run: guest::run(&skeleton.program(units[2], top_repetitions)),
        records: top_records,
    };
    let long = Variant {
        run: guest::run(&skeleton.program(units[0], LONG_REPETITIONS)),
        records: base_records,
    };

    let context = |what: &str| format!("{what}\nskeleton: {skeleton:#?}");
    for (name, variant) in [
        ("base", &base),
        ("middle", &middle),
        ("top", &top),
        ("long", &long),
    ] {
        check_observations(name, variant, n).map_err(|e| context(&e))?;
        check_floor(name, variant, n).map_err(|e| context(&e))?;
    }

    let mut margins = Margins {
        steps: n,
        fuel: [&base, &middle, &top, &long].map(|variant| variant.run.prefix(variant.run.len())),
        scaled_slack_per_digit: f64::INFINITY,
        ..Margins::default()
    };
    let identical = (0..n)
        .take_while(|&i| steps[0][i] == steps[1][i] && steps[1][i] == steps[2][i])
        .count();
    for m in 1..=identical {
        let (a, b, c) = (base.run.prefix(m), middle.run.prefix(m), top.run.prefix(m));
        if a != b || b != c {
            return Err(context(&format!(
                "the first {m} steps are identical at every unit, yet their fuel differs: base \
                 {a}, middle {b}, top {c}"
            )));
        }
    }
    if identical < n {
        let pairs = [
            (&base, &middle, units[0], "base", "middle"),
            (&middle, &top, units[1], "middle", "top"),
        ];
        let mut slack = f64::INFINITY;
        for (index, (lower, upper, unit, lower_name, upper_name)) in pairs.into_iter().enumerate() {
            let bound = growth_bound(SCALE_STEP, unit, skeleton.offset_bound());
            let (margin, per_digit) =
                scaled(lower, upper, n, bound, lower_name, upper_name).map_err(|e| context(&e))?;
            if index == 0 {
                margins.base_to_middle = margin;
            } else {
                margins.middle_to_top = margin;
            }
            slack = slack.min(per_digit);
        }
        margins.scaled_slack_per_digit = slack;
    }

    margins.repetition = repeated(&base, &long, n).map_err(|e| context(&e))?;

    if fixed_steady {
        let steady = |variant: &Variant| {
            let total = BASE_REPETITIONS as usize * n;
            let settled = (BASE_REPETITIONS - STEADY_REPETITIONS) as usize * n;
            (variant.run.prefix(total) - variant.run.prefix(settled)) as f64
                / f64::from(STEADY_REPETITIONS)
        };
        let (s_middle, s_top) = (steady(&middle), steady(&top));
        let margin = s_top / (STEADY_GROWTH_BOUND * s_middle);
        if margin > 1.0 {
            return Err(context(&format!(
                "a repetition whose steps charge no sized term once storage is grown costs \
                 {s_top} fuel at the top unit, more than {STEADY_GROWTH_BOUND} times its \
                 {s_middle} at the middle unit: work grew with a size the rules charge only \
                 once"
            )));
        }
        margins.steady = Some(margin);
    }

    margins.block = check_block(skeleton, &middle, &top).map_err(|e| context(&e))?;
    Ok(margins)
}

/// What the block check found for one skeleton.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum BlockOutcome {
    /// The skeleton has no block: the budgets dropped it, or none was drawn.
    #[default]
    Absent,
    /// A call of the block charges a sized term that differs between the
    /// middle and top units, so the comparison does not apply.
    Sized,
    /// The calls at the end of the block's repetitions do not all cost the
    /// same at both units, so the state is not known to be periodic and the
    /// comparison does not apply.
    Aperiodic,
    /// The last call at the top unit cost this fraction of its bound.
    Checked(f64),
}

/// Returns whether a skeleton's block charges the same sized terms at the
/// middle and top units in its last call, from the variants' records.
fn block_charge_is_fixed(skeleton: &Skeleton, middle: &[Record], top: &[Record]) -> bool {
    let start = skeleton.context_len() + (BLOCK_REPETITIONS - 1) * skeleton.block_len();
    (start..start + skeleton.block_len())
        .all(|i| middle[i].charge.sized_terms() == top[i].charge.sized_terms())
}

/// Checks the block's cost per call at the top unit against its cost at the
/// middle unit.
///
/// The block runs [`BLOCK_REPETITIONS`] times after the context, each call
/// leaving every value as it found it. When the last [`BLOCK_WINDOW`] calls
/// each cost the same fuel, at both units, we take the state they run in to
/// repeat. Equal fuel does not prove that, since a correct implementation
/// could in principle pay deferred work in equal installments across the
/// window; `suanpan`'s potential has no such installments, because each unit
/// of it pays one event that happens once per insertion, and a call
/// reinserts only the block's own positions. When the state repeats, the
/// potential returns to where it was over each call, and the call's cost is
/// its amortized cost. A call whose steps charge no sized term may then
/// cost at most [`STEADY_GROWTH_BOUND`] times as much at the top unit, for
/// the reason that bound states, however wide the members the context left:
/// the context's work is spent before the window, and differencing removes
/// it.
fn check_block(
    skeleton: &Skeleton,
    middle: &Variant,
    top: &Variant,
) -> Result<BlockOutcome, String> {
    let (start, length) = (skeleton.context_len(), skeleton.block_len());
    if length == 0 {
        return Ok(BlockOutcome::Absent);
    }
    if !block_charge_is_fixed(skeleton, &middle.records, &top.records) {
        return Ok(BlockOutcome::Sized);
    }
    let calls = |variant: &Variant| -> Vec<u64> {
        (BLOCK_REPETITIONS - BLOCK_WINDOW..BLOCK_REPETITIONS)
            .map(|call| {
                variant.run.prefix(start + (call + 1) * length)
                    - variant.run.prefix(start + call * length)
            })
            .collect()
    };
    let (at_middle, at_top) = (calls(middle), calls(top));
    let periodic = |costs: &[u64]| costs.windows(2).all(|pair| pair[0] == pair[1]);
    if !periodic(&at_middle) || !periodic(&at_top) {
        return Ok(BlockOutcome::Aperiodic);
    }
    let (per_middle, per_top) = (at_middle[0] as f64, at_top[0] as f64);
    let margin = per_top / (STEADY_GROWTH_BOUND * per_middle);
    if margin > 1.0 {
        return Err(format!(
            "a call of the block, whose steps charge no sized term, costs {per_top} fuel at the \
             top unit, more than {STEADY_GROWTH_BOUND} times its {per_middle} at the middle \
             unit: a step's work grew with a size the rules do not charge it"
        ));
    }
    Ok(BlockOutcome::Checked(margin))
}

/// Checks that the guest observed, at every step instance, the answer the
/// model computes.
fn check_observations(name: &str, variant: &Variant, n: usize) -> Result<(), String> {
    for (index, &observed) in variant.run.observations.iter().enumerate() {
        let record = record(&variant.records, n, index);
        let expected = record.observation;
        let refused = record.may_refuse && observed == observe_stable(None);
        if observed != expected && !refused {
            return Err(format!(
                "the {name} variant's step instance {index} (step {}) observed {observed:#x}, \
                 but the model computes {expected:#x}: the guest's answer disagrees with the \
                 model's values",
                index % n
            ));
        }
    }
    Ok(())
}

/// Checks that every prefix of the run spent at least the fuel its steps'
/// floors add up to, so fuel is counting.
fn check_floor(name: &str, variant: &Variant, n: usize) -> Result<(), String> {
    let mut floor = 0u64;
    for index in 0..variant.run.len() {
        floor += record(&variant.records, n, index).floor;
        let spent = variant.run.prefix(index + 1);
        if spent < floor {
            return Err(format!(
                "the {name} variant's first {} step instances spent {spent} fuel, below their \
                 floor of {floor}: the fuel reading is not counting the steps' work",
                index + 1
            ));
        }
    }
    Ok(())
}

/// Returns the charge, in words, of the first `m` step instances.
fn charge(variant: &Variant, n: usize, m: usize) -> f64 {
    (0..m)
        .map(|index| record(&variant.records, n, index).charge.words())
        .sum()
}

/// Checks `upper` against `lower`, a variant at a unit `SCALE_STEP` times
/// smaller, at every repetition boundary they share.
///
/// The bound is `bound` (see [`growth_bound`]). Returns the largest reading
/// as a fraction of its bound, and the slack, in fuel per digit of the
/// storage the first repetition grows at `upper`'s unit, that the first
/// boundary leaves.
///
/// At each boundary, the fuel net of reallocation copies may be at most
/// `bound` times `lower`'s, and the copies at most twice that. The slack per
/// digit is how much extra fuel a superlinear term would have to spend per
/// digit of grown storage to fail the first boundary: the instrument's
/// sensitivity for that case.
fn scaled(
    lower: &Variant,
    upper: &Variant,
    n: usize,
    bound: f64,
    lower_name: &str,
    upper_name: &str,
) -> Result<(f64, f64), String> {
    let repetitions = lower.run.len().min(upper.run.len()) / n;
    let mut margin = 0f64;
    let mut slack = f64::INFINITY;
    for repetition in 1..=repetitions {
        let m = repetition * n;
        let (low, high) = (lower.run.prefix(m) as f64, upper.run.prefix(m) as f64);
        let (low_copied, high_copied) = (
            lower.run.copied_prefix(m) as f64,
            upper.run.copied_prefix(m) as f64,
        );
        let (low_net, high_net) = (low - low_copied, high - high_copied);
        let net_margin = high_net / (bound * low_net);
        // Copies at the lower unit are never zero where a buffer scales (a
        // digit buffer starts at a capacity of one digit, so its first write
        // past position zero reallocates at every unit); where they are, no
        // buffer reallocated, and the comparison has nothing to judge.
        let copy_margin = if low_copied > 0.0 {
            high_copied / (COPY_GROWTH * bound * low_copied)
        } else {
            0.0
        };
        if net_margin > 1.0 || copy_margin > 1.0 {
            let (low_charge, high_charge) = (charge(lower, n, m), charge(upper, n, m));
            return Err(format!(
                "the first {repetition} repetitions cost {high} fuel at the {upper_name} unit \
                 and {low} at the {lower_name} unit ({:.3} and {:.3} per word of charge); net \
                 of reallocation copies, {high_net} against {low_net}, a ratio of {:.3} where \
                 {bound:.3} is allowed; the copies, {high_copied} bytes against {low_copied}, \
                 may grow by {:.3}",
                high / high_charge,
                low / low_charge,
                high_net / low_net,
                COPY_GROWTH * bound,
            ));
        }
        margin = margin.max(net_margin).max(copy_margin);
        if repetition == 1 {
            let grown: u64 = (0..n).map(|i| upper.records[i].charge.growth_bits).sum();
            if grown > 0 {
                let allowed = bound * low_net + COPY_GROWTH * bound * low_copied;
                slack = (allowed - high) / grown.div_ceil(32) as f64;
            }
        }
    }
    Ok((margin, slack))
}

/// Checks the long variant against the base: identical fuel over the
/// repetitions they share, and after them, at most a seventh of the base's
/// total for each repetition after the first.
///
/// This rests on a premise like the block check's: that the repetitions
/// after the first settle into a state that repeats, so that each costs what
/// the base's later repetitions cost on average. The values repeat, since
/// every repetition starts from zeros. Storage settles too: which
/// accumulator keeps which buffer depends on stored digit counts, which each
/// repetition reproduces from the same zeroed stored forms, so the buffers
/// follow the same exchange every repetition, and each meets every role its
/// cycle gives it, so stops growing, within the first four repetitions,
/// inside the base's eight. The allocator's free lists are assumed to settle
/// as quickly. If repetitions 2 through `r` each cost what one of the base's
/// repetitions 2 through [`BASE_REPETITIONS`] costs on average, the first
/// `r` repetitions cost at most `(r − 1) / (BASE_REPETITIONS − 1)` times the
/// base's whole run, whatever the first repetition costs. Returns the
/// largest reading as a fraction of its bound.
fn repeated(base: &Variant, long: &Variant, n: usize) -> Result<f64, String> {
    let shared = BASE_REPETITIONS as usize * n;
    for index in 1..=shared {
        let (a, d) = (base.run.prefix(index), long.run.prefix(index));
        if a != d {
            return Err(format!(
                "the long and base variants run the same first {index} step instances, yet \
                 spent {d} and {a} fuel"
            ));
        }
    }
    let whole = base.run.prefix(shared) as f64;
    let mut margin = 0f64;
    for index in shared + 1..=long.run.len() {
        let repetition = index.div_ceil(n);
        let bound = (repetition - 1) as f64 / f64::from(BASE_REPETITIONS - 1) * whole;
        let spent = long.run.prefix(index);
        margin = margin.max(spent as f64 / bound);
        if spent as f64 > bound {
            return Err(format!(
                "the first {index} step instances (into repetition {repetition}) spent {spent} \
                 fuel, more than the {bound:.0} that the base's {BASE_REPETITIONS} repetitions \
                 ({whole:.0} fuel) allow: a repetition's cost grew with the length of the \
                 history"
            ));
        }
    }
    Ok(margin)
}

/// The hard regimes one skeleton realized at one unit, counted over its
/// first repetition.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Census {
    /// Steps with a receiver and contribution of opposite signs.
    pub mixed_signs: usize,
    /// Steps that cancelled at least half a unit of width.
    pub cancellations: usize,
    /// Steps that left a run of ones half a unit longer than any in their
    /// inputs.
    pub borrow_runs: usize,
    /// Steps that grew their storage by at least half a unit.
    pub growths: usize,
    /// Steps that left a gap of a unit of zero bits inside a value.
    pub sparse_values: usize,
    /// Updates at a nonzero shift.
    pub shifted_updates: usize,
}

impl AddAssign for Census {
    fn add_assign(&mut self, other: Census) {
        self.mixed_signs += other.mixed_signs;
        self.cancellations += other.cancellations;
        self.borrow_runs += other.borrow_runs;
        self.growths += other.growths;
        self.sparse_values += other.sparse_values;
        self.shifted_updates += other.shifted_updates;
    }
}

impl Census {
    /// Adds one step's events.
    fn add(&mut self, events: &Events) {
        self.mixed_signs += usize::from(events.mixed_sign);
        self.cancellations += usize::from(events.cancellation);
        self.borrow_runs += usize::from(events.borrow_run);
        self.growths += usize::from(events.growth);
        self.sparse_values += usize::from(events.sparse);
        self.shifted_updates += usize::from(events.shifted);
    }
}

/// Counts the hard regimes `skeleton` realizes in one repetition at unit
/// `unit`, from the model alone.
pub fn census(skeleton: &Skeleton, unit: u64) -> Census {
    let mut model = Model::new(unit, true);
    let mut census = Census::default();
    for step in skeleton.steps_at(unit) {
        census.add(&model.step(&step).events);
    }
    census
}
