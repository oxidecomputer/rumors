//! Measures fixed programs for `suanpan`'s focused cost claims, and states
//! each claim as a relation between fuel readings.
//!
//! The interleaving property draws its sequences at random and judges how
//! fuel grows with scale. Some claims name one sequence and a parameter
//! instead: that a negation costs the same at any width, that a scaled read
//! skips a prefix the unscaled read walks, that a sign flip across a gap
//! has no rounds-by-width product. The suite in `tests/claims.rs` states
//! each as a deterministic program, built by hand from public operations,
//! and [`measure`] runs it: an unmeasured setup, the measured steps, and a
//! signed read of every member, with every step's answer checked against
//! the exact [`Model`]. No fuel number is committed anywhere, so a toolchain
//! or Wasmtime change cannot move a claim's verdict unless it changes how
//! the measured operations compare.
//!
//! # Forms
//!
//! Every claim takes one of these forms, which its test's doc comment names
//! by the label in parentheses:
//!
//! - *Equal fuel* (F-eq): the readings are identical at every value of a
//!   parameter the operation's cost must not depend on ([`assert_equal`]).
//! - *Strict saving* (F-lt): one reading is strictly less than another
//!   ([`assert_less`]), and where the claim is that the saved work scales
//!   with a parameter, the saving grows with it ([`assert_saving_grows`]).
//! - *Flat per unit* (F-lin): fuel per unit of work at a larger size is at
//!   most 1.25 times fuel per unit at the smaller size ([`assert_flat`]).
//! - *No product term* (F-mix): over a two-by-two grid of two parameters,
//!   the mixed second difference is at most a tenth of the top cell, which
//!   any cost additive in the parameters meets and a product of them does
//!   not ([`assert_no_product`]).
//! - *Derived bound* (F-bound): readings across a parameter stay within a
//!   spread derived from the operation's mechanism, stated beside the test
//!   ([`assert_spread_below`]).
//!
//! Every checker prints its readings on a `MEASURED` line, so a failure and a
//! thin margin can be read from the test's output.

use std::fmt::Debug;

use suanpan_fuel_protocol::{observe_stable, Member, Program, Read, Step};

use crate::guest;
use crate::model::Model;

/// The flat-per-unit bound: fuel per unit may grow by at most this numerator
/// over [`FLAT_DEN`] from the smaller size to the larger.
const FLAT_NUM: u128 = 5;

/// The flat-per-unit bound's denominator; see [`FLAT_NUM`].
const FLAT_DEN: u128 = 4;

/// The unit, in bits, the checking model measures events against; it
/// detects none, so the unit changes nothing.
const MODEL_UNIT_BITS: u64 = 64;

/// The measured steps of one program run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Measured {
    /// The fuel each measured step spent, in step order.
    pub fuel: Vec<u64>,
    /// The observation word of each measured step, in step order.
    pub observations: Vec<u64>,
}

impl Measured {
    /// Returns the fuel the measured steps spent together.
    pub fn total(&self) -> u64 {
        self.fuel.iter().sum()
    }
}

/// Runs `setup` and then `measured` in one fresh guest instance, and returns
/// the readings of the measured steps.
///
/// A signed read of every member follows the measured steps, and the exact
/// model replays the whole program, so every answer the guest observes,
/// those final reads included, is checked against the model's values: each
/// read's sign, bit length, and lowest limb.
///
/// # Panics
///
/// Panics if the guest traps, or if any step's observation disagrees with the
/// model, naming the step.
pub fn measure(setup: &[Step], measured: &[Step]) -> Measured {
    let epilogue = Member::ALL.map(|member| Step::Read {
        member,
        read: Read::Signed,
    });
    let steps: Vec<Step> = setup
        .iter()
        .chain(measured)
        .chain(&epilogue)
        .cloned()
        .collect();
    let program = Program {
        repetitions: 1,
        steps,
    };
    let run = guest::run(&program);
    let mut model = Model::new(MODEL_UNIT_BITS, false);
    for (index, step) in program.steps.iter().enumerate() {
        let record = model.step(step);
        let observed = run.observations[index];
        let refused = record.may_refuse && observed == observe_stable(None);
        assert!(
            observed == record.observation || refused,
            "step {index} ({step:?}) observed {observed:#x}, but the model computes {:#x}",
            record.observation
        );
    }
    let start = setup.len();
    let range = start..start + measured.len();
    Measured {
        fuel: range
            .clone()
            .map(|index| run.fuel[index + 1] - run.fuel[index])
            .collect(),
        observations: run.observations[range].to_vec(),
    }
}

/// Asserts the *equal fuel* form: every reading in `rows`, each labeled by
/// the parameter it was taken at, is identical, and returns that reading.
///
/// # Panics
///
/// Panics if two readings differ, naming every label and its reading, or if
/// `rows` is empty.
pub fn assert_equal<P: Debug, R: Debug + PartialEq + Clone>(name: &str, rows: &[(P, R)]) -> R {
    eprintln!("MEASURED {name}: (parameter, fuel) {rows:?}");
    assert!(
        rows.windows(2).all(|pair| pair[0].1 == pair[1].1),
        "{name} must cost the same at every parameter, but read (parameter, fuel) {rows:?}"
    );
    rows.first().expect("a claim compares readings").1.clone()
}

/// Asserts the *strict saving* form: `smaller` is strictly less than
/// `larger`, with `name` describing the comparison.
///
/// # Panics
///
/// Panics unless `smaller < larger`.
pub fn assert_less(name: &str, smaller: u64, larger: u64) {
    eprintln!(
        "MEASURED {name}: {smaller} < {larger} (ratio {:.3})",
        smaller as f64 / larger as f64
    );
    assert!(
        smaller < larger,
        "{name}: {smaller} must be strictly less than {larger}"
    );
}

/// Asserts the *strict saving* form across a parameter: in every row the
/// first reading is strictly less than the second, and the saving, the
/// second minus the first, grows strictly from each row to the next.
///
/// Each row is `(parameter, smaller, larger)`, in increasing parameter. A
/// growing saving shows that the work saved scales with the parameter rather
/// than being a fixed cost.
///
/// # Panics
///
/// Panics if either bound fails.
pub fn assert_saving_grows(name: &str, rows: &[(u64, u64, u64)]) {
    let mut previous: Option<(u64, u64)> = None;
    for &(parameter, smaller, larger) in rows {
        assert_less(&format!("{name} at {parameter}"), smaller, larger);
        let saving = larger - smaller;
        if let Some((earlier, earlier_saving)) = previous {
            assert!(
                earlier_saving < saving,
                "{name}: the saving must grow with the parameter, but it read {earlier_saving} \
                 at {earlier} and {saving} at {parameter}"
            );
        }
        previous = Some((parameter, saving));
    }
}

/// Asserts the *flat per unit* form: `large`'s fuel per unit is at most 1.25
/// times `small`'s, where each argument is `(fuel, units)`.
///
/// # Panics
///
/// Panics if the bound fails or either unit count is zero.
pub fn assert_flat(name: &str, small: (u64, u64), large: (u64, u64)) {
    let ((fuel1, units1), (fuel2, units2)) = (small, large);
    assert!(units1 > 0 && units2 > 0, "{name}: unit counts are positive");
    eprintln!(
        "MEASURED {name}: small={fuel1}/{units1} large={fuel2}/{units2} growth per unit {:.3} \
         (band {:.3})",
        (fuel2 as f64 / units2 as f64) / (fuel1 as f64 / units1 as f64),
        FLAT_NUM as f64 / FLAT_DEN as f64,
    );
    assert!(
        u128::from(fuel2) * u128::from(units1) * FLAT_DEN
            <= u128::from(fuel1) * u128::from(units2) * FLAT_NUM,
        "{name}: fuel per unit grew past the band: {fuel1}/{units1} -> {fuel2}/{units2}"
    );
}

/// Asserts the *no product term* form over `grid`, the readings at
/// `(x, y)`, `(2x, y)`, `(x, 2y)`, and `(2x, 2y)` in that order: the mixed
/// second difference is at most a tenth of the top cell.
///
/// A cost `p·x + q·y + r` has a mixed second difference of zero, and a cost
/// `s·x·y` one of a quarter of the top cell, so the bound tells them apart
/// with room for amortized costs that wobble between cells.
///
/// # Panics
///
/// Panics if the bound fails.
pub fn assert_no_product(name: &str, grid: [u64; 4]) {
    let [base, x2, y2, both] = grid.map(i128::from);
    let mixed = both - x2 - y2 + base;
    eprintln!("MEASURED {name}: grid {grid:?} mixed {mixed}");
    assert!(
        mixed.abs() * 10 <= both,
        "{name}: the mixed second difference {mixed} exceeds a tenth of the top cell {both}: \
         the cost has a product term"
    );
}

/// Asserts the *derived bound* form: the readings in `rows`, each labeled by
/// the parameter it was taken at, differ by less than `bound` between the
/// largest and the smallest.
///
/// # Panics
///
/// Panics if the spread reaches `bound`, or if `rows` is empty.
pub fn assert_spread_below<P: Debug>(name: &str, rows: &[(P, u64)], bound: u64) {
    let readings = rows.iter().map(|(_, reading)| *reading);
    let spread = readings.clone().max().expect("a claim compares readings")
        - readings.min().expect("a claim compares readings");
    eprintln!("MEASURED {name}: (parameter, fuel) {rows:?} spread {spread} bound {bound}");
    assert!(
        spread < bound,
        "{name}: the readings spread by {spread}, reaching the derived bound {bound}"
    );
}
