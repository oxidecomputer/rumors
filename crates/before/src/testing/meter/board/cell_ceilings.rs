//! Ceilings calibrated for each board cell, so that each cell is bounded by
//! its own worst reading rather than only by a ceiling the costliest cells
//! set.
//!
//! A board-wide ceiling binds only the cells that read near it. Fuel per byte
//! differs between cells by orders of magnitude, so no board-wide fuel
//! ceiling could bind more than the costliest cells. A *per-cell ceiling* is
//! derived from the cell's own readings instead.
//!
//! # The rule
//!
//! Each ceiling follows the measured-ceiling rule of the `ceilings` module,
//! applied to one cell's six samples (both sizes at each of the ladder's
//! three sampling scales), with two differences: [`CELL_CEILING_HEADROOM`]
//! replaces the board-wide headroom, and the ceiling is rounded up to a
//! thousandth ([`CEILING_RESOLUTION`]) rather than to a whole number. That
//! is, `C = max(0, ceil(1000 * 1.10 * max((reading - intercept) / units)) /
//! 1000)`, where `units` is the cell's constant units for the currency (input
//! bytes unless the cell declares a model) and `intercept` is the currency's
//! additive allowance.
//!
//! What this holds: the cell's worst sample, the one with the largest reading
//! per unit after the intercept, may grow by at most 10% (plus the rounding)
//! before the cell turns red. What it leaves looser: every other sample is
//! held to the same per-unit ceiling, so a sample that reads less per unit
//! than the worst one may grow by more than 10%, up to the worst one's level
//! plus 10%. The growth exponent, judged separately, bounds how differently
//! the samples can move.
//!
//! Whole-number rounding suits a board-wide ceiling, which bounds the
//! costliest cells, but many cells read only a few units of their currency
//! per unit of input: rounded to a whole number, a cell reading 0.1 would be
//! allowed ten times its own reading. Rounded to a thousandth, every cell
//! whose worst reading is at least 0.1 per unit keeps its headroom within a
//! point of 10%. The clamp at zero keeps a cell whose every reading lies
//! within the intercept judged against the intercept alone.
//!
//! The judge checks every sample against the tighter of the cell's ceiling
//! and its outer bound: the row's declared model ceiling if it has one,
//! otherwise the currency's board-wide ceiling. Fuel has no board-wide
//! ceiling, so its per-cell ceiling is its only proportional bound.
//!
//! # Tables
//!
//! A table holds one currency's ceilings and names the architectures whose
//! readings it was calibrated from, because a reading is deterministic only
//! for a given target. Fuel is measured only in a wasm32 guest
//! ([`FUEL_ARCHS`]). A table calibrated from several architectures takes the
//! larger of their readings at every sample, so it holds on each. Each table
//! lives in a generated child module named for its currency, and [`TABLES`]
//! lists them.
//!
//! A run on an architecture its currency's table does not name is judged
//! without the table: a currency with an outer bound falls back to it, and
//! the rendered board names the missing table. Fuel has no outer bound to
//! fall back to, so a fuel run without its table is refused rather than
//! judged. A currency with no table at all is judged by its outer bound
//! alone.
//!
//! # Recalibrating
//!
//! `just amp-board-calibrate-fuel` rewrites the fuel table from a fresh fuel
//! sweep, printing each cell whose ceiling moved. A calibration refuses a
//! capture of another source tree, and refuses to write a table whose
//! architectures would differ from its currency's architectures of record.
//! Review the diff like a snapshot (one line per cell), and commit it with the
//! movement explained.

mod fuel;

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::Path;

use super::compare::{measured, readings, Readings};
use super::files::write_atomically;
use super::judge::{constant_units, currency_intercept, trend_units};
use super::ladder::Ladder;
use super::{ByCurrency, Currency};

#[cfg(test)]
mod tests;

/// The headroom a per-cell ceiling allows above the cell's largest
/// normalized reading: the `1.10` in the module doc's rule.
///
/// The readings are exact counters, so the headroom absorbs no measurement
/// noise. It sets how far a deliberate change may move a cell before the
/// cell turns red and its ceiling is re-derived on purpose.
pub const CELL_CEILING_HEADROOM: f64 = 1.10;

/// The number of steps per unit of reading in which a table stores its
/// ceilings: entries hold thousandths.
pub const CEILING_RESOLUTION: u64 = 1_000;

/// One currency's per-cell ceilings, and the architectures they hold for.
#[derive(Debug)]
pub struct CellCeilings {
    /// The currency the ceilings bound.
    pub currency: Currency,
    /// The architectures whose readings the ceilings were derived from, as
    /// [`std::env::consts::ARCH`] names them; the table judges runs measured
    /// on these alone.
    pub archs: &'static [&'static str],
    /// What the readings depend on besides the code under test (the
    /// toolchain, runtime, and pricing), as the calibrating runner recorded
    /// it; `None` where the runner records nothing beyond the architectures.
    pub provenance: Option<&'static str>,
    /// Every cell's ceiling, in thousandths ([`CEILING_RESOLUTION`]) of a
    /// reading per constant unit, sorted by operation and then family.
    entries: &'static [(&'static str, &'static str, u64)],
}

/// The architectures whose readings the fuel table must be calibrated from:
/// the guest's alone, since fuel is measured nowhere else.
pub const FUEL_ARCHS: [&str; 1] = ["wasm32"];

/// The architectures `currency`'s table must be calibrated from; empty for a
/// currency with no table.
fn archs_of_record(currency: Currency) -> &'static [&'static str] {
    match currency {
        Currency::Fuel => &FUEL_ARCHS,
        Currency::Heap | Currency::Scan | Currency::Touch => &[],
    }
}

/// Every per-cell table, at most one for each currency.
const TABLES: [&CellCeilings; 1] = [&fuel::CEILINGS];

impl CellCeilings {
    /// Returns cell `op` × `family`'s ceiling, in reading per constant unit.
    ///
    /// # Panics
    ///
    /// Panics if the table holds no ceiling for the cell: the cell is new, or
    /// renamed, since the table was calibrated.
    pub(super) fn ceiling(&self, op: &str, family: &str) -> f64 {
        let index = self
            .entries
            .binary_search_by(|&(o, f, _)| (o, f).cmp(&(op, family)))
            .unwrap_or_else(|_| {
                panic!(
                    "amp-board: no calibrated {} ceiling for {op} x {family}; recalibrate \
                     as the `cell_ceilings` module doc says",
                    self.currency.label(),
                )
            });
        self.entries[index].2 as f64 / CEILING_RESOLUTION as f64
    }

    /// The generated module's file name, relative to this module's directory.
    fn file(&self) -> String {
        format!("{}.rs", self.currency.label())
    }
}

/// Returns the table for `currency` that holds for `arch`, if one exists.
pub fn cell_ceilings(currency: Currency, arch: &str) -> Option<&'static CellCeilings> {
    TABLES
        .into_iter()
        .find(|table| table.currency == currency && table.archs.contains(&arch))
}

/// The per-cell tables that judge a run measured by a build for one
/// architecture.
pub(super) struct Calibration {
    /// The architecture the run's build targets.
    pub(super) arch: String,
    /// Each currency's table for [`Self::arch`]; `None` where that currency
    /// has no table, or has one that does not hold for the architecture.
    pub(super) tables: ByCurrency<Option<&'static CellCeilings>>,
}

impl Calibration {
    /// Returns `currency`'s table for the run's architecture, if one holds.
    pub(super) fn table(&self, currency: Currency) -> Option<&'static CellCeilings> {
        *self.tables.get(currency)
    }

    /// Returns the architectures `currency`'s table is calibrated from, if
    /// the currency has a table and it does not hold for the run's
    /// architecture.
    pub(super) fn missing(&self, currency: Currency) -> Option<&'static [&'static str]> {
        let archs = archs_of_record(currency);
        (self.table(currency).is_none() && !archs.is_empty()).then_some(archs)
    }
}

/// Returns the per-cell tables for readings measured on `arch`.
pub(super) fn calibration(arch: &str) -> Calibration {
    Calibration {
        arch: arch.to_owned(),
        tables: ByCurrency::from_fn(|currency| cell_ceilings(currency, arch)),
    }
}

/// One regenerated table, ready to replace its module's source file.
#[derive(Debug)]
pub struct Recalibration {
    /// The table's module file, relative to the `before` crate's root.
    pub path: String,
    /// The module's new source text.
    pub source: String,
    /// One line for each cell whose ceiling moved, appeared, or disappeared.
    pub movement: Vec<String>,
}

/// The directory of the generated table modules, relative to the `before`
/// crate's root.
const TABLE_DIR: &str = "src/testing/meter/board/cell_ceilings";

impl Recalibration {
    /// Replaces the table's module file under `crate_root`, the `before`
    /// crate's root, in one rename, or leaves it untouched.
    ///
    /// # Errors
    ///
    /// Returns any error writing the new text beside the file or renaming
    /// it into place; either leaves the committed table intact.
    pub fn write(&self, crate_root: &Path) -> io::Result<()> {
        write_atomically(&crate_root.join(&self.path), self.source.as_bytes())
    }
}

/// Writes every regenerated table over its module file under `crate_root`,
/// the `before` crate's root, reporting to `out` each cell whose ceiling
/// moved and one summary line per table.
///
/// # Errors
///
/// Returns [`io::ErrorKind::InvalidInput`], writing nothing, if there is no
/// table to write (the run measured no currency that has one), and any error
/// writing a table or the report.
pub fn write_recalibrations(
    recalibrations: &[Recalibration],
    crate_root: &Path,
    out: &mut dyn std::io::Write,
) -> io::Result<()> {
    if recalibrations.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "amp-board calibrate: the run measured no currency that has a per-cell table",
        ));
    }
    for recalibration in recalibrations {
        recalibration.write(crate_root)?;
        for line in &recalibration.movement {
            writeln!(out, "{line}")?;
        }
        writeln!(
            out,
            "amp-board: wrote {path}: {moved} cells moved",
            path = recalibration.path,
            moved = recalibration.movement.len()
        )?;
    }
    Ok(())
}

/// Recalibrates the table of every currency the ladder measured, from the
/// larger of the ladder's reading and each capture's reading at every
/// sample, recording `provenance` in each.
///
/// `source_tree` names the source the ladder's build was compiled from. Each
/// capture is `(name, text)`, as [`capture`](super::capture) wrote it, and
/// must come from a release build of the same tree; `name` labels it in
/// errors. Together the ladder and the captures must cover exactly the
/// currency's architectures of record ([`FUEL_ARCHS`] for fuel), so a
/// table never holds for an architecture whose readings it never saw, nor
/// stops holding for one of record.
///
/// # Errors
///
/// Returns [`io::ErrorKind::InvalidData`] if a capture is malformed, came
/// from a debug build or another tree, lacks a cell or currency the ladder
/// measured, or measured a sample at a different size than the ladder did,
/// or if the architectures differ from the currency's architectures of
/// record.
///
/// # Panics
///
/// Panics if the ladder was measured with debug assertions compiled in (the
/// tables judge the release board of record), or if it measured a currency
/// in some samples but not others.
pub fn recalibrate(
    ladder: &Ladder,
    source_tree: &str,
    captures: &[(&str, &str)],
    provenance: Option<&str>,
) -> io::Result<Vec<Recalibration>> {
    assert!(
        !ladder.debug_assertions(),
        "amp-board: per-cell ceilings are calibrated from a release build's readings"
    );
    let others: Vec<(&str, Readings)> = captures
        .iter()
        .map(|&(name, text)| Ok((name, readings(name, text)?)))
        .collect::<io::Result<_>>()?;
    for (name, other) in &others {
        if other.measured_on.debug_assertions {
            return Err(invalid(name, "comes from a build with debug assertions"));
        }
        if other.source_tree != source_tree {
            return Err(invalid(
                name,
                &format!(
                    "measured source tree {}, not {source_tree}",
                    other.source_tree
                ),
            ));
        }
    }
    TABLES
        .into_iter()
        .filter(|table| measured(ladder, table.currency))
        .map(|table| {
            let archs: BTreeSet<&str> = std::iter::once(ladder.arch())
                .chain(
                    others
                        .iter()
                        .map(|(_, other)| other.measured_on.arch.as_str()),
                )
                .collect();
            let of_record: BTreeSet<&str> =
                archs_of_record(table.currency).iter().copied().collect();
            if archs != of_record {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "amp-board calibrate: the {} table needs readings from {:?}, but the \
                         run and its captures measured {:?}",
                        table.currency.label(),
                        of_record,
                        archs
                    ),
                ));
            }
            let entries = derive_ceilings(ladder, &others, table.currency)?;
            Ok(Recalibration {
                path: format!("{TABLE_DIR}/{}", table.file()),
                source: render_table(table, &archs, provenance, &entries),
                movement: movement(table, &entries),
            })
        })
        .collect()
}

/// An invalid capture's error, naming the capture.
fn invalid(name: &str, detail: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("amp-board calibrate: capture {name} {detail}"),
    )
}

/// Derives every cell's `currency` ceiling by the module doc's rule from the
/// larger of the ladder's and the other captures' reading at every sample,
/// sorted by operation and then family, in thousandths.
///
/// # Errors
///
/// Returns [`io::ErrorKind::InvalidData`] if a capture lacks a cell's row or
/// measured one of its samples at a different size than the ladder did.
fn derive_ceilings<'a>(
    ladder: &'a Ladder,
    others: &[(&str, Readings)],
    currency: Currency,
) -> io::Result<Vec<(&'a str, &'a str, u64)>> {
    let intercept = currency_intercept(currency);
    let mut entries = Vec::with_capacity(ladder.base.len());
    for cell in 0..ladder.base.len() {
        let (op, family, ..) = ladder.base[cell];
        let samples = ladder.by_scale().into_iter().flat_map(|(_, cells)| {
            let (_, _, s1, s2) = &cells[cell];
            [s1, s2]
        });
        let mut worst = f64::NEG_INFINITY;
        for (index, sample) in samples.enumerate() {
            let mut reading = sample
                .readings
                .get(currency)
                .expect("a measured currency reads in every sample");
            for (name, other) in others {
                let key = (op.to_owned(), family.to_owned(), currency);
                let points = other.rows.get(&key).ok_or_else(|| {
                    invalid(
                        name,
                        &format!("holds no {} row for {op} x {family}", currency.label()),
                    )
                })?;
                let (axis, other_reading) = points[index];
                if axis != trend_units(sample, currency) {
                    return Err(invalid(
                        name,
                        &format!("measured {op} x {family} at a different size"),
                    ));
                }
                reading = reading.max(other_reading);
            }
            let per_unit = (reading as f64 - intercept) / constant_units(sample, currency) as f64;
            worst = worst.max(per_unit);
        }
        entries.push((op, family, ceiling_thousandths(worst)));
    }
    entries.sort_unstable();
    Ok(entries)
}

/// The ceiling, in thousandths, that the module doc's rule derives from a
/// cell's worst normalized reading: the headroom applied, rounded up to a
/// thousandth, and clamped at zero.
fn ceiling_thousandths(worst: f64) -> u64 {
    (CEILING_RESOLUTION as f64 * CELL_CEILING_HEADROOM * worst)
        .ceil()
        .max(0.0) as u64
}

/// Renders a table module in the form the child modules hold.
fn render_table(
    table: &CellCeilings,
    archs: &BTreeSet<&str>,
    provenance: Option<&str>,
    entries: &[(&str, &str, u64)],
) -> String {
    let rows: String = entries
        .iter()
        .map(|(op, family, ceiling)| format!("        ({op:?}, {family:?}, {ceiling}),\n"))
        .collect();
    let label = table.currency.label();
    let variant = format!("{:?}", table.currency);
    let arch_list = archs
        .iter()
        .map(|arch| format!("{arch:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    let arch_prose = archs.iter().copied().collect::<Vec<_>>().join(" and ");
    let provenance = match provenance {
        Some(provenance) => format!("Some({provenance:?})"),
        None => "None".to_string(),
    };
    format!(
        "//! Per-cell {label} ceilings, calibrated from {arch_prose} readings.
//!
//! Generated by a calibration; the `cell_ceilings` module doc states the rule
//! and the recipe. Review a regenerated table like a snapshot.

use super::{{CellCeilings, Currency}};

/// The calibrated table.
pub(super) const CEILINGS: CellCeilings = CellCeilings {{
    currency: Currency::{variant},
    archs: &[{arch_list}],
    provenance: {provenance},
    entries: &[
{rows}    ],
}};
"
    )
}

/// Lists every cell whose ceiling differs between `table` and `entries`, both
/// sorted by operation and then family, with the relative change where the
/// cell has both ceilings.
fn movement(table: &CellCeilings, entries: &[(&str, &str, u64)]) -> Vec<String> {
    let old: BTreeMap<(&str, &str), u64> = table
        .entries
        .iter()
        .map(|&(op, family, ceiling)| ((op, family), ceiling))
        .collect();
    let new: BTreeMap<(&str, &str), u64> = entries
        .iter()
        .map(|&(op, family, ceiling)| ((op, family), ceiling))
        .collect();
    let keys: BTreeSet<&(&str, &str)> = old.keys().chain(new.keys()).collect();
    let show = |ceiling: Option<&u64>| {
        ceiling.map_or_else(
            || "-".to_string(),
            |&ceiling| format!("{:.3}", ceiling as f64 / CEILING_RESOLUTION as f64),
        )
    };
    keys.into_iter()
        .filter(|key| old.get(key) != new.get(key))
        .map(|key @ (op, family)| {
            let change = match (old.get(key), new.get(key)) {
                (Some(&before), Some(&after)) if before > 0 => {
                    format!(" ({:+.1}%)", 100.0 * (after as f64 / before as f64 - 1.0))
                }
                _ => String::new(),
            };
            format!(
                "{label}: {op} x {family}: {old} -> {new}{change}",
                label = table.currency.label(),
                old = show(old.get(key)),
                new = show(new.get(key)),
            )
        })
        .collect()
}
