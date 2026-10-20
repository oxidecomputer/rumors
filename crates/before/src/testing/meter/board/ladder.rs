//! Sweeps the board's measurement ladder once and judges the verdict of record
//! over that one sweep.
//!
//! The board's module docs say why one sweep serves every consumer. Each
//! consumer reads only the merged cells [`merge_samples`] returned, so judging
//! or capturing a ladder reads exactly what a separate sweep of the same tree
//! would.

use std::collections::BTreeSet;
use std::io::{self, Write};

use super::ceilings::{DEFAULT_SCALE, LADDER_TOP_SCALE, SMALL_INPUT_SCALE};
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
    /// grids.
    pub fn sweep(shards: usize, spawn: ShardSpawner<'_>) -> io::Result<Ladder> {
        let sweep = |scale| -> io::Result<Vec<Measured>> {
            Ok(merge_samples(scale, shards, &spawn(scale)?))
        };
        let [small, base, top] = LADDER_SCALES;
        let ladder = Ladder {
            small: sweep(small)?,
            base: sweep(base)?,
            top: sweep(top)?,
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
        cells
            .iter()
            .cloned()
            .map(|(op, family, s1, s2)| evaluate(op, family, s1, s2))
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
    /// Returns any error writing to `out`.
    ///
    /// # Panics
    ///
    /// Panics if a mapped counter is not compiled into this run: the pin is
    /// stated over heap, scan, and touch, so the check requires the
    /// `touch-meter` and `scan-meter` features.
    pub fn judge(&self, out: &mut dyn Write) -> io::Result<Verdict> {
        self.judge_with(WORST_RANKINGS, out)
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
        pins: &[(&str, &str, [&str; 3])],
        out: &mut dyn Write,
    ) -> io::Result<Verdict> {
        let summary = self.accept(out)?;
        let pin_clean = check_with(pins, &|scale| self.judged_at(scale), out)?;
        Ok(Verdict { summary, pin_clean })
    }

    /// Renders the acceptance judgment's three matrices and returns its
    /// summary, as [`Ladder::judge`] states it: the growth windows through
    /// [`evaluate_acceptance`], the small inputs through [`evaluate_small`].
    fn accept(&self, out: &mut dyn Write) -> io::Result<Summary> {
        let mut base_cells = Vec::with_capacity(self.base.len());
        let mut top_cells = Vec::with_capacity(self.top.len());
        let mut small_cells = Vec::with_capacity(self.small.len());
        for (((op, family, b1, b2), (_, _, t1, t2)), (_, _, s1, s2)) in
            self.base.iter().zip(&self.top).zip(&self.small)
        {
            let (base, top) = evaluate_acceptance(
                op,
                family,
                (b1.clone(), b2.clone()),
                (t1.clone(), t2.clone()),
            );
            base_cells.push(base);
            top_cells.push(top);
            small_cells.push(evaluate_small(op, family, s1.clone(), s2.clone()));
        }
        writeln!(out, "=== ladder base: sampling scale {DEFAULT_SCALE} ===")?;
        render_results(&base_cells, out)?;
        writeln!(out, "=== ladder top: sampling scale {LADDER_TOP_SCALE} ===")?;
        render_results(&top_cells, out)?;
        writeln!(
            out,
            "=== small inputs: sampling scale {SMALL_INPUT_SCALE}, ceilings and floors ==="
        )?;
        render_results(&small_cells, out)?;
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
