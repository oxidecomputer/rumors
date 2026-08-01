//! The bench export: the board's cells exposed for wall-clock
//! benchmarking.
//!
//! The wall-time mirror rides the same axes: the bench suite's
//! criterion IDs are exactly the board's op × family cell names
//! ([`bench_cells`] is the board's own table), so board coverage is
//! bench coverage cell for cell, with no second enumeration. Wall
//! benching pays criterion's warmup and sampling per cell, so the judge
//! cadence times the rule-derived pinned subset (each shape's
//! designed-stress pairings, the organic control, and the
//! declared-model riders) while `BOARD_BENCH_MODE=full` times the whole
//! product for final verdicts — the subset is a rule over the product,
//! never a hand-maintained cell list.

use std::any::Any;
use std::rc::Rc;

use super::cell::{assert_honest_text, Cell, Denom};
use super::family::FamilyData;
use super::ops::{designed, ops};
use crate::meter::registry::FamilyId;

/// One board cell exposed for wall-clock benchmarking.
///
/// The bench suite (`benches/board.rs`) is the board's wall-time shadow: its
/// criterion group and function IDs are exactly [`BenchCell::op`] and
/// [`BenchCell::family`], so a board cell names the bench that times it and
/// a criterion filter selects a cell. The bench judge (`tools/benchjudge`)
/// reads those same IDs out of criterion's saved estimates to run the time
/// leg the board module doc describes.
pub struct BenchCell {
    /// The board row's operation name: the bench group ID.
    pub op: &'static str,
    /// The input family's name: the bench function ID within the group.
    pub family: &'static str,
    /// The family operands the row's prepare reads, shared across cells.
    data: Rc<FamilyData>,
    /// The board row's prepare, re-run per measured body.
    prepare: fn(&FamilyData) -> Option<Cell>,
}

impl BenchCell {
    /// Build one fresh run of the cell's measured body.
    ///
    /// Operands are decoded anew on every call — the board's prepare
    /// discipline — so a bench harness rebuilds destructive operands in its
    /// untimed setup and times the returned closure alone. The closure is
    /// exactly what the board meters: same operands, same operation, same
    /// kept-alive result.
    pub fn body(&self) -> Box<dyn FnOnce() -> Box<dyn Any>> {
        (self.prepare)(&self.data)
            .expect("cell applicability was settled at construction")
            .body
    }

    /// The cell's denominator bytes at its scale: packed input, or total
    /// I/O on the I/O-denominated rows, or the bundle's value content on
    /// the flat-denominator shape's input-denominated rows.
    ///
    /// The content denominator is the same one the board fits those
    /// cells' exponents against: the judge's fitted time exponents must
    /// not re-manufacture what the board's re-denomination corrected.
    ///
    /// Runs one untimed body to read the output side back from the actual
    /// result, exactly as the board's measurement does (a prediction never
    /// substitutes for the result, and a text output is checked against the
    /// honesty ceiling on the way). The bench judge denominates its fitted
    /// time exponents against these bytes — the board's own convention — so
    /// a family whose packed bytes grow faster than the scale knob (the
    /// cliff comb's value content is quadratic in its parameter) is judged
    /// against what the operation actually reads and writes, never against
    /// the knob.
    pub fn denominator_bytes(&self) -> usize {
        let cell =
            (self.prepare)(&self.data).expect("cell applicability was settled at construction");
        let result = (cell.body)();
        match cell.denom {
            Denom::Input => self.data.content_bytes.unwrap_or(cell.input_bytes),
            Denom::Io(spec) => {
                let output_bytes = (spec.output_bytes)(result.as_ref());
                if let Some(text) = spec.text {
                    if text.output_is_text {
                        assert_honest_text(self.op, output_bytes, text.radix_units);
                    }
                }
                cell.input_bytes + output_bytes
            }
        }
    }
}

/// Which slice of the shape × operation product a bench run times.
///
/// The deterministic board always runs the whole product (its cells are
/// cheap counters); wall-clock benching is not, so the mirror has two
/// modes, both derived from the same axis declarations — the subset is a
/// rule over the product, never a second hand-maintained cell list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BenchMode {
    /// The pinned rule-derived subset: every operation on the benign
    /// control, each shape's designed-stress pairings (declared per
    /// shape on the shape axis), and the declared-model riders
    /// ([`BOARD_DECLARED_BENCH_RIDERS`]).
    Pinned,
    /// The whole product: the mode for final verdicts.
    Full,
}

/// Declared-model board cells outside the designed pairings whose time
/// leg the pinned bench subset must still cover.
///
/// A cell judged green under an owner-declared counter model keeps a
/// wall-clock witness even where no designed pairing times its shape.
///
/// Membership is by `(operation, family)` cell name, expectations live
/// in the judge's roster as ever; a cell whose declared model dissolves
/// (a cure landing) leaves this list in the same change. The current
/// membership: the `version_min_ticks` reign-state cell and the display
/// pair's render-merge cells. The tick trio's ascend-cliff cells need
/// no rider — the tick group is those crosses' designed diagonal — and
/// the tooth-tail parse cell rides its designed pairing likewise.
pub const BOARD_DECLARED_BENCH_RIDERS: &[(&str, &str)] = &[
    ("version_min_ticks", "ascend-cliff"),
    ("version_display", "mirror-wide"),
    ("clock_display", "mirror-wide"),
];

/// Every board cell of the chosen [`BenchMode`] at `scale`, in board row
/// order.
///
/// `scale` multiplies the family base sizes exactly as [`run`](crate::meter::board::run)'s does; the
/// cells are op × family pairings applicable at that scale, at one
/// measurement level (a bench varies repetition, not size).
///
/// # Panics
///
/// Panics if `scale` is not a strictly positive finite number.
pub fn bench_cells(scale: f64, mode: BenchMode) -> Vec<BenchCell> {
    assert!(
        scale > 0.0 && scale.is_finite(),
        "bench cells: scale must be a positive finite number"
    );
    let families: Vec<Rc<FamilyData>> = FamilyId::board()
        .map(|kind| Rc::new(FamilyData::build(kind, scale, 0)))
        .collect();
    let mut cells = Vec::new();
    for op in ops() {
        for family in &families {
            let include = match mode {
                BenchMode::Full => true,
                BenchMode::Pinned => {
                    designed(family.kind, op.group)
                        || BOARD_DECLARED_BENCH_RIDERS.contains(&(op.name, family.name))
                }
            };
            if include && (op.prepare)(family).is_some() {
                cells.push(BenchCell {
                    op: op.name,
                    family: family.name,
                    data: Rc::clone(family),
                    prepare: op.prepare,
                });
            }
        }
    }
    cells
}
