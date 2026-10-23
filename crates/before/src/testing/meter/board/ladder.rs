//! Sweeps the board's measurement ladder once and judges the verdict of record
//! over that one sweep.
//!
//! The board's module docs say why one sweep serves every consumer. Each
//! consumer reads only the merged cells [`merge_samples`] returned, so judging
//! or capturing a ladder reads exactly what a separate sweep of the same tree
//! would.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Write};

use super::ceilings::{DEFAULT_SCALE, LADDER_TOP_SCALE, SMALL_INPUT_SCALE};
use super::cell_ceilings::{calibration, Calibration};
use super::compare::{measured, BuildStamp};
use super::currency::Currency;
use super::judge::{evaluate, evaluate_acceptance, evaluate_without_fit, CellResult};
use super::render::{render_results, Summary};
use super::sampling::{Sampling, PAST_POWER_OFFSET};
use super::shard::{merge_samples, Measured, ShardSpawner};
use super::worst::{check_with, WORST_RANKINGS};

#[cfg(test)]
mod tests;

/// The measurement ladder's samplings in sweep order: the two pairs judged
/// against ceilings and floors alone (the small inputs, then the knobs just
/// past a power of two), then the growth ladder's base and top.
pub(super) const LADDER_SAMPLINGS: [Sampling; 4] = [
    Sampling::Scaled(SMALL_INPUT_SCALE),
    Sampling::PastPowerOfTwo,
    Sampling::Scaled(DEFAULT_SCALE),
    Sampling::Scaled(LADDER_TOP_SCALE),
];

/// The whole board measured at every sampling of the ladder.
///
/// Every sweep holds the same cells in board row order (operation outer,
/// family inner); [`Ladder::sweep`] establishes that, and every consumer
/// relies on it to pair a cell's samples across scales by position.
pub struct Ladder {
    /// The build stamp every shard child reported: the target it was
    /// compiled for and whether debug assertions were compiled in.
    pub(super) measured_on: BuildStamp,
    /// The small-input sweep, at [`SMALL_INPUT_SCALE`].
    pub(super) small: Vec<Measured>,
    /// The sweep whose knobs sit just past a power of two
    /// ([`Sampling::PastPowerOfTwo`]).
    pub(super) past_power: Vec<Measured>,
    /// The growth ladder's base, at [`DEFAULT_SCALE`].
    pub(super) base: Vec<Measured>,
    /// The growth ladder's top, at [`LADDER_TOP_SCALE`].
    pub(super) top: Vec<Measured>,
}

/// The board's verdict of record over one ladder: the acceptance summary and
/// the worst-case ranking pin's result.
#[derive(Debug, Clone, Copy)]
pub struct Verdict {
    /// The acceptance judgment's cell counts; a cell red at any size counts
    /// once.
    pub summary: Summary,
    /// Whether every pinned worst-case ranking matched the live fold.
    pub pin_clean: bool,
}

impl Verdict {
    /// Returns whether the board passes: no red cell and no ranking drift.
    pub fn passed(&self) -> bool {
        self.summary.red == 0 && self.pin_clean
    }
}

impl Ladder {
    /// Sweeps the whole board at each of the ladder's samplings across
    /// `shards` child processes.
    ///
    /// `spawn` is invoked once per sampling: the small inputs, the knobs just
    /// past a power of two, then the growth ladder's base and top.
    ///
    /// # Errors
    ///
    /// Returns any error `spawn` returns.
    ///
    /// # Panics
    ///
    /// Panics on any protocol violation in a child capture (the `shard`
    /// module's refusal list), and if the sweeps measured different cell
    /// grids or came from differently built children.
    pub fn sweep(shards: usize, spawn: ShardSpawner<'_>) -> io::Result<Ladder> {
        let sweep = |sampling| -> io::Result<(BuildStamp, Vec<Measured>)> {
            Ok(merge_samples(sampling, shards, &spawn(sampling)?))
        };
        let [small, past_power, base, top] = LADDER_SAMPLINGS;
        let (small_stamp, small) = sweep(small)?;
        let (past_power_stamp, past_power) = sweep(past_power)?;
        let (base_stamp, base) = sweep(base)?;
        let (top_stamp, top) = sweep(top)?;
        assert!(
            [&small_stamp, &past_power_stamp, &top_stamp]
                .iter()
                .all(|stamp| **stamp == base_stamp),
            "amp-board: the ladder's sweeps came from differently built children"
        );
        let ladder = Ladder {
            measured_on: base_stamp,
            small,
            past_power,
            base,
            top,
        };
        let grid = |cells: &[Measured]| -> Vec<(&'static str, &'static str)> {
            cells
                .iter()
                .map(|(op, family, ..)| (*op, *family))
                .collect()
        };
        let base_grid = grid(&ladder.base);
        assert!(
            [&ladder.small, &ladder.past_power, &ladder.top]
                .iter()
                .all(|cells| grid(cells) == base_grid),
            "amp-board: the ladder's sweeps measured different cell grids"
        );
        Ok(ladder)
    }

    /// Returns the architecture the measuring build targeted.
    pub(super) fn arch(&self) -> &str {
        &self.measured_on.arch
    }

    /// Returns whether the measuring build compiled in debug assertions.
    pub(super) fn debug_assertions(&self) -> bool {
        self.measured_on.debug_assertions
    }

    /// Returns the per-cell ceilings that judge this ladder's readings.
    fn calibration(&self) -> Calibration {
        calibration(self.arch())
    }

    /// Asserts that every resource model an operation declares changes a unit
    /// axis or the ceiling for at least one sample of one of its cells.
    ///
    /// A model that matches the defaults everywhere declares nothing, and the
    /// rendered row would disclose an override that does not exist. The check
    /// spans the operation's whole ladder rather than one sample, because a
    /// model whose units are a function of the operands, such as `d²` for a
    /// count of `d` digits, coincides with the defaults on a family whose
    /// count stays one digit long.
    ///
    /// # Panics
    ///
    /// Panics naming the operation and currency of a model that changes
    /// nothing.
    pub(super) fn assert_models_override(&self) {
        let mut overrides: BTreeMap<(&str, Currency), bool> = BTreeMap::new();
        for (_, cells) in self.by_sampling() {
            for (op, _, s1, s2) in cells {
                for sample in [s1, s2] {
                    for (currency, model) in sample.models.each() {
                        let Some(model) = model else {
                            continue;
                        };
                        let changes = model.trend_units != sample.exp_denom_bytes as u64
                            || model.constant_units != sample.denom_bytes as u64
                            || model.ceiling.is_some();
                        *overrides.entry((op, currency)).or_default() |= changes;
                    }
                }
            }
        }
        for ((op, currency), changes) in overrides {
            assert!(
                changes,
                "amp-board: {op}'s {} model must change a unit axis or the ceiling for some \
                 sample",
                currency.label()
            );
        }
    }

    /// Returns each sweep paired with its sampling, in [`LADDER_SAMPLINGS`]
    /// order.
    pub(super) fn by_sampling(&self) -> [(Sampling, &[Measured]); 4] {
        let [small, past_power, base, top] = LADDER_SAMPLINGS;
        [
            (small, &self.small),
            (past_power, &self.past_power),
            (base, &self.base),
            (top, &self.top),
        ]
    }

    /// Judges the sweep at `scale` as a single-scale run judges it
    /// ([`evaluate`]), for the ranking pin's fold.
    ///
    /// # Panics
    ///
    /// Panics unless `scale` is one of [`LADDER_SAMPLINGS`]' scales.
    fn judged_at(&self, scale: f64) -> Vec<CellResult> {
        let (_, cells) = self
            .by_sampling()
            .into_iter()
            .find(|(at, _)| *at == Sampling::Scaled(scale))
            .unwrap_or_else(|| panic!("amp-board: the ladder holds no sweep at scale {scale}"));
        let calibration = self.calibration();
        cells
            .iter()
            .cloned()
            .map(|(op, family, s1, s2)| evaluate(&calibration, op, family, s1, s2))
            .collect()
    }

    /// Judges the ladder and renders the verdict of record to `out`: the
    /// acceptance matrices, then the worst-case ranking pin's check.
    ///
    /// Both judgments always run, so a red cell never hides ranking drift
    /// from the same change.
    ///
    /// # The acceptance judgment
    ///
    /// One matrix renders per sweep: the ladder's base, its top, the small
    /// inputs, and the knobs just past a power of two. Each currency's
    /// exponent is fitted once, as one trend over the four growth points;
    /// every constant, declared-model band, and liveness floor is judged at
    /// every size, exactly as a single-scale run judges it. The small-input
    /// and past-power pairs are checked against ceilings and liveness floors
    /// alone. Each matrix prints its own summary line; the returned summary
    /// counts distinct cells, so a cell red at several sizes counts once.
    ///
    /// # The ranking pin
    ///
    /// The worst-case map is folded from each growth sweep at its sampling
    /// scale ([`WORST_MAP_SCALES`](super::WORST_MAP_SCALES)) and entry-compared
    /// against the committed ranking table; each disagreement prints one drift
    /// line, and a clean check prints one summary line.
    ///
    /// # Errors
    ///
    /// Returns [`io::ErrorKind::InvalidInput`], judging nothing, if the run
    /// did not measure every currency in `required` (a counter feature left
    /// out of the build, or a meter the runner did not supply), and any error
    /// writing to `out`.
    ///
    /// # Panics
    ///
    /// Panics if a mapped counter is not compiled into this run: the pin is
    /// stated over heap, scan, and touch, so the check requires the
    /// `touch-meter` and `scan-meter` features.
    pub fn judge(&self, required: &[Currency], out: &mut dyn Write) -> io::Result<Verdict> {
        self.judge_with(required, WORST_RANKINGS, out)
    }

    /// Judges the ladder as [`Ladder::judge`] does, against the ranking pin
    /// `pins` in place of the committed table.
    ///
    /// # Errors
    ///
    /// As [`Ladder::judge`].
    ///
    /// # Panics
    ///
    /// As [`Ladder::judge`], and if `pins` is malformed (duplicate or unknown
    /// scale and operation keys).
    pub(super) fn judge_with(
        &self,
        required: &[Currency],
        pins: &[(&str, &str, [&str; 3])],
        out: &mut dyn Write,
    ) -> io::Result<Verdict> {
        let summary = self.accept(required, out)?;
        let pin_clean = check_with(pins, &|scale| self.judged_at(scale), out)?;
        Ok(Verdict { summary, pin_clean })
    }

    /// Renders the acceptance judgment's matrices, one per sweep, and returns its
    /// summary, as [`Ladder::judge`] states it.
    ///
    /// Each growth window is judged with one exponent fitted across the whole
    /// ladder, and the small inputs and past-power knobs against ceilings and
    /// floors alone.
    ///
    /// This is the whole verdict of a fuel run. The ranking pin is stated over
    /// heap, scan, and touch, which a fuel run does not measure, so only
    /// [`Ladder::judge`] checks it.
    ///
    /// # Errors
    ///
    /// Returns [`io::ErrorKind::InvalidInput`], judging nothing, if the run
    /// did not measure every currency in `required`, and any error writing to
    /// `out`.
    ///
    /// # Panics
    ///
    /// Panics if a cell has no ceiling at all for a measured currency (fuel,
    /// with no table for the measured architecture). A cell that a table
    /// omits is judged red instead (the `cell_ceilings` module doc).
    pub fn accept(&self, required: &[Currency], out: &mut dyn Write) -> io::Result<Summary> {
        for &currency in required {
            if !measured(self, currency) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "amp-board: the run measured no {} readings, which its verdict requires",
                        currency.label()
                    ),
                ));
            }
        }
        self.accept_with(&self.calibration(), out)
    }

    /// Renders the acceptance judgment as [`Ladder::accept`] does, under
    /// `calibration` in place of the committed per-cell tables, and without
    /// requiring any currency.
    ///
    /// # Panics
    ///
    /// As [`Ladder::accept`], and if an operation declares a resource model
    /// that changes nothing ([`Ladder::assert_models_override`]).
    pub(super) fn accept_with(
        &self,
        calibration: &Calibration,
        out: &mut dyn Write,
    ) -> io::Result<Summary> {
        self.assert_models_override();
        let mut base_cells = Vec::with_capacity(self.base.len());
        let mut top_cells = Vec::with_capacity(self.top.len());
        let mut small_cells = Vec::with_capacity(self.small.len());
        let mut past_power_cells = Vec::with_capacity(self.past_power.len());
        for ((((op, family, b1, b2), (_, _, t1, t2)), (_, _, s1, s2)), (_, _, p1, p2)) in self
            .base
            .iter()
            .zip(&self.top)
            .zip(&self.small)
            .zip(&self.past_power)
        {
            let (base, top) = evaluate_acceptance(
                calibration,
                op,
                family,
                (b1.clone(), b2.clone()),
                (t1.clone(), t2.clone()),
            );
            base_cells.push(base);
            top_cells.push(top);
            small_cells.push(evaluate_without_fit(
                calibration,
                op,
                family,
                s1.clone(),
                s2.clone(),
            ));
            past_power_cells.push(evaluate_without_fit(
                calibration,
                op,
                family,
                p1.clone(),
                p2.clone(),
            ));
        }
        writeln!(out, "=== ladder base: sampling scale {DEFAULT_SCALE} ===")?;
        render_results(calibration, &base_cells, out)?;
        writeln!(out, "=== ladder top: sampling scale {LADDER_TOP_SCALE} ===")?;
        render_results(calibration, &top_cells, out)?;
        writeln!(
            out,
            "=== small inputs: sampling scale {SMALL_INPUT_SCALE}, ceilings and floors ==="
        )?;
        render_results(calibration, &small_cells, out)?;
        writeln!(
            out,
            "=== past a power of two: every knob {PAST_POWER_OFFSET} past a power of two, \
             ceilings and floors ==="
        )?;
        render_results(calibration, &past_power_cells, out)?;
        let red: BTreeSet<(&'static str, &'static str)> = base_cells
            .iter()
            .chain(&top_cells)
            .chain(&small_cells)
            .chain(&past_power_cells)
            .filter(|cell| !cell.red.is_empty())
            .map(|cell| (cell.op, cell.family))
            .collect();
        Ok(Summary {
            green: base_cells.len() - red.len(),
            red: red.len(),
        })
    }
}
