//! Ceilings calibrated for each board cell, so that each cell is bounded by
//! its own worst reading rather than only by a ceiling the costliest cells
//! set.
//!
//! A board-wide ceiling binds only the cells that read near it. Fuel per byte
//! differs between cells by orders of magnitude, so no board-wide fuel
//! ceiling could bind more than the costliest cells, and a cell reading 2
//! heap bytes per input byte could regress several-fold under the board-wide
//! heap ceiling ([`MAX_HEAP_BYTES_PER_INPUT_BYTE`](super::MAX_HEAP_BYTES_PER_INPUT_BYTE))
//! without turning red. A *per-cell ceiling* is derived from the cell's own
//! readings instead.
//!
//! # The rule
//!
//! Each ceiling follows the measured-ceiling rule of the `ceilings` module,
//! applied to all of one cell's samples (both sizes at each of the ladder's
//! samplings), with two differences: [`CELL_CEILING_HEADROOM`]
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
//! ([`FUEL_ARCHS`]). Heap readings differ between aarch64 and x86_64 wherever
//! an operation reaches an architecture-conditional allocation (the board
//! module's "Target dependence" section), so the heap table is calibrated
//! from both ([`HEAP_ARCHS`]): each cell's ceiling is derived from the larger
//! of its two readings at every sample. Each table lives in a generated child
//! module named for its currency, and [`TABLES`] lists them.
//!
//! A run on an architecture its currency's table does not name is judged
//! without the table: heap falls back to its outer bound, and the rendered
//! board names the missing table. Fuel has no outer bound to fall back to,
//! so a fuel run without its table is refused rather than judged. A currency
//! with no table at all is judged by its outer bound alone.
//!
//! A cell that a table holding for the run omits, because the cell is new or
//! renamed since the calibration, is red (`cell uncalibrated`) whatever its
//! readings; a heap cell is still judged against its outer bound beside
//! that. It is red rather than refused so that the run still
//! completes and writes its capture, which the recalibration below needs
//! from the other architecture of record.
//!
//! # Recalibrating
//!
//! `just amp-board-calibrate <capture>` sweeps the board on this host and
//! merges a capture taken on the other architecture of record
//! (`just amp-board-acceptance --capture <file>` there). It refuses a capture
//! of another source tree, and refuses to write a table whose architectures
//! would differ from its currency's architectures of record. It rewrites the
//! heap table from the larger reading at every sample, and the fuel table from
//! a fresh fuel sweep, printing each cell whose ceiling moved. Fuel readings do
//! not depend on the host, so `just amp-board-calibrate-fuel` rewrites the fuel
//! table alone, from one host, after a change that moves fuel and not heap.
//!
//! A capture records, beside each reading, the units its ceiling divides it
//! by, so the rule applies to a capture alone ([`recalibrate_from_captures`]).
//! After `just amp-board-fuel --capture <file>`, `just amp-board-calibrate-fuel
//! <file>` derives the fuel table from that capture without sweeping again,
//! refusing a capture of another source tree or build. A toolchain, Wasmtime,
//! or cost-table change still needs the sweep: the fuel board refuses to
//! measure a guest built under anything but the table's provenance, so it
//! writes no capture until the table is recalibrated.
//!
//! Review the diff like a snapshot (one line per cell), and commit it with the
//! movement explained.

mod fuel;
mod heap;

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::Path;

use super::compare::{ladder_readings, measured, readings, Readings};
use super::files::write_atomically;
use super::judge::currency_intercept;
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

/// The architectures whose readings the heap table must be calibrated from:
/// the 64-bit architectures the board of record runs on.
pub const HEAP_ARCHS: [&str; 2] = ["aarch64", "x86_64"];

/// The architectures whose readings the fuel table must be calibrated from:
/// the guest's alone, since fuel is measured nowhere else.
pub const FUEL_ARCHS: [&str; 1] = ["wasm32"];

/// The architectures `currency`'s table must be calibrated from; empty for a
/// currency with no table.
fn archs_of_record(currency: Currency) -> &'static [&'static str] {
    match currency {
        Currency::Heap => &HEAP_ARCHS,
        Currency::Fuel => &FUEL_ARCHS,
    }
}

/// Every per-cell table, at most one for each currency.
const TABLES: [&CellCeilings; 2] = [&heap::CEILINGS, &fuel::CEILINGS];

impl CellCeilings {
    /// Returns cell `op` × `family`'s ceiling, in reading per constant unit,
    /// or `None` if the table holds no ceiling for the cell: the cell is new,
    /// or renamed, since the table was calibrated.
    pub(super) fn ceiling(&self, op: &str, family: &str) -> Option<f64> {
        let index = self
            .entries
            .binary_search_by(|&(o, f, _)| (o, f).cmp(&(op, family)))
            .ok()?;
        Some(self.entries[index].2 as f64 / CEILING_RESOLUTION as f64)
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
/// currency's architectures of record ([`HEAP_ARCHS`], [`FUEL_ARCHS`]), so a
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
    recalibrate_tables(&TABLES, ladder, source_tree, captures, provenance)
}

/// [`recalibrate`], against `tables` in place of the committed tables.
fn recalibrate_tables(
    tables: &[&CellCeilings],
    ladder: &Ladder,
    source_tree: &str,
    captures: &[(&str, &str)],
    provenance: Option<&str>,
) -> io::Result<Vec<Recalibration>> {
    assert!(
        !ladder.debug_assertions(),
        "amp-board: per-cell ceilings are calibrated from a release build's readings"
    );
    let swept = ladder_readings(ladder, source_tree);
    let others = release_captures(source_tree, captures)?;
    tables
        .iter()
        .filter(|table| measured(ladder, table.currency))
        .map(|table| recalibrate_table(table, &swept, &others, provenance))
        .collect()
}

/// Recalibrates `currency`'s table from captures alone, from the larger of
/// their readings at every sample, recording `provenance`.
///
/// The result is the table [`recalibrate`] writes from a sweep that read
/// what the first capture holds, merged with the other captures.
///
/// A capture records each sample's reading and the units its ceiling divides
/// by, which is all the rule needs, so a calibration can reuse the readings
/// of an acceptance run instead of sweeping again. The first capture
/// supplies the cells, which every other capture must hold at the same
/// sizes. Every capture must come from a release build of `source_tree`,
/// and together they must cover exactly the currency's architectures of
/// record.
///
/// # Errors
///
/// Returns [`io::ErrorKind::InvalidInput`] if there is no capture or
/// `currency` has no per-cell table, and [`io::ErrorKind::InvalidData`] if
/// the first capture holds no reading in `currency`, or for anything
/// [`recalibrate`] refuses in a capture.
pub fn recalibrate_from_captures(
    currency: Currency,
    source_tree: &str,
    captures: &[(&str, &str)],
    provenance: Option<&str>,
) -> io::Result<Recalibration> {
    recalibrate_tables_from_captures(&TABLES, currency, source_tree, captures, provenance)
}

/// [`recalibrate_from_captures`], against `tables` in place of the committed
/// tables.
fn recalibrate_tables_from_captures(
    tables: &[&CellCeilings],
    currency: Currency,
    source_tree: &str,
    captures: &[(&str, &str)],
    provenance: Option<&str>,
) -> io::Result<Recalibration> {
    let table = tables
        .iter()
        .find(|table| table.currency == currency)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "amp-board calibrate: {} has no per-cell table",
                    currency.label()
                ),
            )
        })?;
    let mut captures = release_captures(source_tree, captures)?.into_iter();
    let (name, first) = captures.next().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "amp-board calibrate: a calibration from captures needs at least one capture",
        )
    })?;
    if !first.rows.keys().any(|&(_, _, read)| read == currency) {
        return Err(invalid(
            name,
            &format!("holds no {} readings", currency.label()),
        ));
    }
    let others: Vec<(&str, Readings)> = captures.collect();
    recalibrate_table(table, &first, &others, provenance)
}

/// Parses each capture, refusing one that came from a debug build or from a
/// source tree other than `source_tree`.
///
/// # Errors
///
/// Returns [`io::ErrorKind::InvalidData`] naming the first capture that is
/// malformed or refused.
fn release_captures<'a>(
    source_tree: &str,
    captures: &[(&'a str, &str)],
) -> io::Result<Vec<(&'a str, Readings)>> {
    captures
        .iter()
        .map(|&(name, text)| {
            let capture = readings(name, text)?;
            if capture.measured_on.debug_assertions {
                return Err(invalid(name, "comes from a build with debug assertions"));
            }
            if capture.source_tree != source_tree {
                return Err(invalid(
                    name,
                    &format!(
                        "measured source tree {}, not {source_tree}",
                        capture.source_tree
                    ),
                ));
            }
            Ok((name, capture))
        })
        .collect()
}

/// Regenerates `table` by the module doc's rule from the larger of
/// `reference`'s and the other captures' readings at every sample, checking
/// that together they cover exactly its architectures of record.
///
/// # Errors
///
/// Returns [`io::ErrorKind::InvalidData`] if the architectures differ from
/// the table's architectures of record, or as [`derive_ceilings`] does.
fn recalibrate_table(
    table: &CellCeilings,
    reference: &Readings,
    others: &[(&str, Readings)],
    provenance: Option<&str>,
) -> io::Result<Recalibration> {
    let archs: BTreeSet<&str> = std::iter::once(reference.measured_on.arch.as_str())
        .chain(
            others
                .iter()
                .map(|(_, other)| other.measured_on.arch.as_str()),
        )
        .collect();
    let of_record: BTreeSet<&str> = archs_of_record(table.currency).iter().copied().collect();
    if archs != of_record {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "amp-board calibrate: the {} table needs readings from {:?}, but the \
                 readings came from {:?}",
                table.currency.label(),
                of_record,
                archs
            ),
        ));
    }
    let entries = derive_ceilings(reference, others, table.currency)?;
    Ok(Recalibration {
        path: format!("{TABLE_DIR}/{}", table.file()),
        source: render_table(table, &archs, provenance, &entries),
        movement: movement(table, &entries),
    })
}

/// An invalid capture's error, naming the capture.
fn invalid(name: &str, detail: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("amp-board calibrate: capture {name} {detail}"),
    )
}

/// Derives the `currency` ceiling of every cell `reference` holds, in
/// thousandths, sorted by operation and then family.
///
/// Each follows the module doc's rule, from the larger of `reference`'s and
/// the other captures' reading at every sample.
///
/// # Errors
///
/// Returns [`io::ErrorKind::InvalidData`] if a capture lacks a cell's row or
/// measured one of its samples at a different size than `reference` did.
fn derive_ceilings<'a>(
    reference: &'a Readings,
    others: &[(&str, Readings)],
    currency: Currency,
) -> io::Result<Vec<(&'a str, &'a str, u64)>> {
    let intercept = currency_intercept(currency);
    let mut entries = Vec::new();
    // The rows are keyed by operation, family, and currency, so those of
    // one currency come out sorted by operation and then family.
    for ((op, family, read), points) in &reference.rows {
        if *read != currency {
            continue;
        }
        let mut worst = f64::NEG_INFINITY;
        for (index, point) in points.iter().enumerate() {
            let mut reading = point.reading;
            for (name, other) in others {
                let key = (op.clone(), family.clone(), currency);
                let other_points = other.rows.get(&key).ok_or_else(|| {
                    invalid(
                        name,
                        &format!("holds no {} row for {op} x {family}", currency.label()),
                    )
                })?;
                let other_point = other_points[index];
                if (other_point.axis, other_point.units) != (point.axis, point.units) {
                    return Err(invalid(
                        name,
                        &format!("measured {op} x {family} at a different size"),
                    ));
                }
                reading = reading.max(other_point.reading);
            }
            let per_unit = (reading as f64 - intercept) / point.units as f64;
            worst = worst.max(per_unit);
        }
        entries.push((op.as_str(), family.as_str(), ceiling_thousandths(worst)));
    }
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
