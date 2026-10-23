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
use super::judge::{evaluate, evaluate_acceptance, evaluate_small, CellResult};
use super::render::{render_results, Summary};
use super::shard::{merge_samples, Measured, ShardSpawner};
use super::worst::{check_with, WORST_RANKINGS};

#[cfg(test)]
mod tests;

/// The measurement ladder's sampling scales in sweep order: the small-input
/// pair's scale, then the growth ladder's base and top.
pub(super) const LADDER_SCALES: [f64; 3] = [SMALL_INPUT_SCALE, DEFAULT_SCALE, LADDER_TOP_SCALE];

/// The whole board measured at every sampling scale of the ladder.
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
    /// Sweeps the whole board at each of the ladder's sampling scales across
    /// `shards` child processes.
    ///
    /// `spawn` is invoked once per sampling scale, smallest first.
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
        let sweep = |scale| -> io::Result<(BuildStamp, Vec<Measured>)> {
            Ok(merge_samples(scale, shards, &spawn(scale)?))
        };
        let [small, base, top] = LADDER_SCALES;
        let (small_stamp, small) = sweep(small)?;
        let (base_stamp, base) = sweep(base)?;
        let (top_stamp, top) = sweep(top)?;
        assert!(
            small_stamp == base_stamp && top_stamp == base_stamp,
            "amp-board: the ladder's sweeps came from differently built children"
        );
        let ladder = Ladder {
            measured_on: base_stamp,
            small,
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
            grid(&ladder.small) == base_grid && grid(&ladder.top) == base_grid,
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
        for (_, cells) in self.by_scale() {
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

    /// Returns each sweep paired with its sampling scale, in [`LADDER_SCALES`]
    /// order.
    pub(super) fn by_scale(&self) -> [(f64, &[Measured]); 3] {
        let [small, base, top] = LADDER_SCALES;
        [(small, &self.small), (base, &self.base), (top, &self.top)]
    }

    /// Judges the sweep at `scale` as a single-scale run judges it
    /// ([`evaluate`]), for the ranking pin's fold.
    ///
    /// # Panics
    ///
    /// Panics unless `scale` is one of [`LADDER_SCALES`].
    fn judged_at(&self, scale: f64) -> Vec<CellResult> {
        let (_, cells) = self
            .by_scale()
            .into_iter()
            .find(|(at, _)| *at == scale)
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
    /// One matrix renders per sweep: the ladder's base, its top, and the small
    /// inputs. Each currency's exponent is fitted once, as one trend over the
    /// four growth points; every constant, declared-model band, and liveness
    /// floor is judged at every size, exactly as a single-scale run judges it.
    /// The small-input pair is checked against ceilings and liveness floors
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

    /// Renders the acceptance judgment's three matrices and returns its
    /// summary, as [`Ladder::judge`] states it.
    ///
    /// Each growth window is judged with one exponent fitted across the whole
    /// ladder, and the small inputs against ceilings and floors alone.
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
        for (((op, family, b1, b2), (_, _, t1, t2)), (_, _, s1, s2)) in
            self.base.iter().zip(&self.top).zip(&self.small)
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
            small_cells.push(evaluate_small(
                calibration,
                op,
                family,
                s1.clone(),
                s2.clone(),
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
        let red: BTreeSet<(&'static str, &'static str)> = base_cells
            .iter()
            .chain(&top_cells)
            .chain(&small_cells)
            .filter(|cell| !cell.red.is_empty())
            .map(|cell| (cell.op, cell.family))
            .collect();
        Ok(Summary {
            green: base_cells.len() - red.len(),
            red: red.len(),
        })
    }
}
