//! Measures a skeleton at every scale and repetition count, and judges the
//! fuel readings against bounds derived from the cost rules.
//!
//! [`judge`] runs four variants of one skeleton, `P`, each in a fresh guest
//! instance:
//!
//! | Variant | Unit | Repetitions | Role |
//! |---|---|---|---|
//! | base | `u` | [`BASE_REPETITIONS`] | the control for the others |
//! | middle | `64 · u` | [`BASE_REPETITIONS`] | scaled once |
//! | top | `4096 · u` | 1, or [`BASE_REPETITIONS`] when the steady repetition has a fixed charge | scaled twice |
//! | long | `u` | [`LONG_REPETITIONS`] | repeated |
//!
//! Every repetition ends by clearing every member, so each one starts from
//! zero values over the storage the last one left, and repetitions after the
//! first compute the same values as the first. The readings are judged
//! against each other, never against a committed number; the bounds below
//! say how far each reading may exceed the reading it is compared with.
//!
//! # Why fuel may grow by a factor, and no less
//!
//! The rules bound a program's work by a constant times its charge, without
//! naming the constant, and each rule's term is linear in the sizes it
//! names. So the work of a correct implementation is a sum of terms, each a
//! constant, a size the rules charge, or a logarithm of one, times a constant
//! of its own. Multiplying every size by `k` multiplies each term by at most
//! `k`, and so the whole reading by at most `k`, plus the slack the offsets
//! allow ([`growth_bound`]). In units of charge, that lets fuel per unit of
//! charge grow by `growth_bound · charge(P) / charge(k P)`.
//!
//! The premise is that the guest's work depends on sizes alone. The guest's
//! allocator moves every block it reallocates (its `moving` module): an
//! allocator that grows blocks in place where the heap allows it copies at
//! one scale what it extends at another, and that copy, which the third
//! rule allows, would make the readings grow faster than the sizes.
//!
//! No smaller allowance follows from the rules. The constants differ from
//! term to term, and scaling shifts weight from the fixed terms to the sized
//! ones, so a correct program whose sized terms cost more per unit of charge
//! than its fixed ones legitimately raises its fuel per unit of charge as it
//! scales. A consequence is what this judgment cannot see: a defect whose
//! extra work is linear in a size the rules do not charge to that step
//! (an update that pays its receiver's width, growth paid again on every
//! repetition) multiplies by `k` exactly as charged work does, and passes.
//! It sees work that grows faster than the sizes do (quadratic work, a
//! logarithm multiplying a size, cost that grows with the length of the
//! history) and, through the steady repetition at two scales, work that
//! grows with a size the rules charge only once.

use std::ops::AddAssign;

use suanpan_fuel_protocol::{observe_stable, Step};

use crate::guest::{self, Run};
use crate::model::{Events, Model, Record};
use crate::skeleton::{Skeleton, UNIT_BITS};

/// The repetitions of the base, middle, and fixed-charge top variants.
pub const BASE_REPETITIONS: u32 = 8;

/// The repetitions of the long variant.
pub const LONG_REPETITIONS: u32 = 4096;

/// The factor between consecutive scales.
pub const SCALE_STEP: u64 = 64;

/// How far fuel may grow when every size grows by `scale`, from a program
/// at unit `unit` whose structural offsets are at most `offset_bound` bits.
///
/// Every length the implementation traverses is `d · unit + δ` bits with
/// `d` a whole number and `|δ| ≤ 2 · offset_bound`
/// ([`Skeleton::offset_bound`]). A length with `d = 0` does not grow. One
/// with `d ≥ 1` grows by at most `(k · unit − 2D) / (unit − 2D)`, which is
/// `k + (k − 1) · 2D / (unit − 2D)`: the most its fixed part can make the
/// ratio exceed `k`.
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

/// How far a steady repetition's fuel may grow from unit `64 · u` to
/// `4096 · u` when no step's charge has a sized term: twice.
///
/// Such a repetition's work is a constant per step plus climbs through the
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
    /// The largest middle-over-base prefix ratio, over its bound.
    pub base_to_middle: f64,
    /// The largest top-over-middle prefix ratio, over its bound.
    pub middle_to_top: f64,
    /// The largest long-variant prefix, over its bound.
    pub repetition: f64,
    /// The top steady repetition over the middle one, over its bound, when
    /// the steady repetition has a fixed charge.
    pub steady: Option<f64>,
    /// The steps in one repetition of the skeleton, epilogue included.
    pub steps: usize,
    /// Each variant's total fuel: base, middle, top, and long.
    pub fuel: [u64; 4],
}

/// One variant: its runs, and the model's records of its first two
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
        ..Margins::default()
    };
    let identical = (0..n)
        .take_while(|&i| steps[0][i] == steps[1][i] && steps[1][i] == steps[2][i])
        .count();
    let base_to_middle = growth_bound(SCALE_STEP, units[0], skeleton.offset_bound());
    let middle_to_top = growth_bound(SCALE_STEP, units[1], skeleton.offset_bound());
    for m in 1..=n {
        let (a, b, c) = (base.run.prefix(m), middle.run.prefix(m), top.run.prefix(m));
        if m <= identical {
            if a != b || b != c {
                return Err(context(&format!(
                    "the first {m} steps are identical at every unit, yet their fuel differs: \
                     base {a}, middle {b}, top {c}"
                )));
            }
            continue;
        }
        margins.base_to_middle = margins.base_to_middle.max(
            scaled(&base, &middle, m, base_to_middle, "base", "middle").map_err(|e| context(&e))?,
        );
        margins.middle_to_top = margins.middle_to_top.max(
            scaled(&middle, &top, m, middle_to_top, "middle", "top").map_err(|e| context(&e))?,
        );
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
    Ok(margins)
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
                 but the model computes {expected:#x}: the guest ran another program",
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

/// Checks prefix `m` of `upper`, a variant at a unit `SCALE_STEP` times
/// `lower`'s, against `bound` times `lower`'s, returning the reading as a
/// fraction of its bound.
fn scaled(
    lower: &Variant,
    upper: &Variant,
    m: usize,
    bound: f64,
    lower_name: &str,
    upper_name: &str,
) -> Result<f64, String> {
    let n = lower.records.len() / 2;
    let (low, high) = (lower.run.prefix(m), upper.run.prefix(m));
    let margin = high as f64 / (bound * low as f64);
    if margin > 1.0 {
        let (low_charge, high_charge) = (charge(lower, n, m), charge(upper, n, m));
        return Err(format!(
            "the first {m} steps cost {high} fuel at the {upper_name} unit and {low} at the \
             {lower_name} unit, a ratio of {:.2}, above the {bound:.2} that {SCALE_STEP}-fold \
             sizes allow. Per unit of charge: {:.3} at the {lower_name} unit ({low_charge:.0} \
             words), {:.3} at the {upper_name} unit ({high_charge:.0} words), allowed \
             {:.3}.",
            high as f64 / low as f64,
            low as f64 / low_charge,
            high as f64 / high_charge,
            bound * low as f64 / high_charge,
        ));
    }
    Ok(margin)
}

/// Checks the long variant against the base: identical fuel over the
/// repetitions they share, and after them, fuel within the base's average
/// cost per repetition.
///
/// Every repetition after the first computes the same values over the same
/// storage, so its charge is the same, and the rules allow it the same work.
/// If repetitions 2 through `r` each cost what one of the base's
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
