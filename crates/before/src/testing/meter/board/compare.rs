//! Exact comparison of two board runs: a capture of every cell's readings at
//! every ladder size, and a comparison that classifies each difference by how
//! it changes with input size.
//!
//! The rendered board rounds fitted exponents to hundredths and per-unit
//! constants to tenths, and a fitted exponent moves when only a fixed cost
//! changes: for a reading near `a + b·n`, shrinking `a` raises the log-log
//! slope toward 1 although nothing grows faster. Rendered text therefore cannot
//! tell a fixed-cost change from a proportional one. A capture keeps the exact
//! counters instead, and [`compare`] works on their differences.
//!
//! # The capture format
//!
//! A capture is UTF-8 text. Its first line is a header of four tab-separated
//! fields: the format name and version; whether debug assertions were
//! compiled into the code that ran the cells (`debug_assertions=<bool>`); the
//! target architecture and operating system that code was built for
//! (`target=<arch>-<os>`), which for a fuel run is the wasm guest, not the
//! host that wrote the capture; and the source tree it measured
//! (`tree=<id>`, the runner's `AMP_BOARD_SOURCE_TREE`). [`compare`] compares
//! readings only between captures whose builds match, because debug
//! assertions add metered work and some operations allocate differently by
//! target; comparing two trees is what it is for. A calibration merges
//! readings only from captures of the tree it measures itself.
//!
//! Each line after the header, but for the last, holds one reading in seven
//! tab-separated fields:
//!
//! ```text
//! operation  family  currency  scale  sample  axis  reading
//! ```
//!
//! - `currency` is `heap`, `scan`, `touch`, or `fuel`. A capture holds rows for
//!   the currencies its run measured: heap, scan, and touch from the native
//!   board, fuel from the fuel board.
//! - `scale` is the sampling scale and `sample` is `1` for its scaled size or
//!   `2` for that size's double. Each cell and currency has one pair of lines
//!   per sampling scale, in ladder order: the small-input pair
//!   ([`SMALL_INPUT_SCALE`](super::SMALL_INPUT_SCALE)), then the growth
//!   ladder's two pairs ([`DEFAULT_SCALE`](super::DEFAULT_SCALE), then
//!   [`LADDER_TOP_SCALE`](super::LADDER_TOP_SCALE)).
//! - `axis` is the size the board fits that currency's growth trend against,
//!   in the cell's growth units (input bytes, unless the rendered board's row
//!   names another unit),
//!   so two captures' readings compare at equal size.
//! - `reading` is the exact counter value.
//!
//! Rows follow board order. The last line, `end` and the number of rows
//! (cells times currencies), marks the capture complete, so a capture cut
//! short by a failed run is refused rather than read as a smaller board. The
//! format belongs to this tool and carries no stability promise beyond its
//! version tag.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Write};
use std::path::Path;

use super::currency::{ByCurrency, Currency};
use super::files::write_atomically;
use super::judge::trend_units;
use super::ladder::{Ladder, LADDER_SCALES};
use super::measure::Sample;

#[cfg(test)]
mod tests;

/// The format name and version that open every capture's header.
const FORMAT: &str = "amp-board readings v2";

/// The readings each cell and currency carries: two samples at every scale,
/// in [`LADDER_SCALES`] order.
const LADDER_SIZES: usize = 2 * LADDER_SCALES.len();

/// The ladder index of the first growth-ladder size; the small-input pair
/// precedes it.
const GROWTH_START: usize = 2;

/// One reading and the size it was measured at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Point {
    /// The units the board fits this currency's growth trend against.
    axis: u64,
    /// The exact counter value.
    reading: u64,
}

/// One cell's readings in one currency, at every ladder size.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Row {
    /// The board operation's name.
    op: String,
    /// The input family's name.
    family: String,
    /// The counter read.
    currency: Currency,
    /// The readings in ladder order.
    points: [Point; LADDER_SIZES],
}

/// A whole capture: its header's build stamp and source tree, and its rows
/// in board order.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Capture {
    /// The build that measured the readings.
    measured_on: BuildStamp,
    /// The source tree the build was compiled from, as the runner named it.
    source_tree: String,
    /// Every cell and currency's readings.
    rows: Vec<Row>,
}

/// What a build measures with: whether debug assertions are compiled in, and
/// the target it is built for.
///
/// Every shard child reports its own stamp, so a ladder records the build
/// that measured it even when another program merges the readings. It
/// renders, and parses, as `debug_assertions=<bool>` and
/// `target=<arch>-<os>`, tab-separated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct BuildStamp {
    /// Whether debug assertions are compiled in.
    pub(super) debug_assertions: bool,
    /// The target architecture, as [`std::env::consts::ARCH`] names it.
    pub(super) arch: String,
    /// The target operating system, as [`std::env::consts::OS`] names it.
    pub(super) os: String,
}

impl BuildStamp {
    /// This build's stamp.
    pub(super) fn current() -> BuildStamp {
        BuildStamp {
            debug_assertions: cfg!(debug_assertions),
            arch: std::env::consts::ARCH.to_owned(),
            os: std::env::consts::OS.to_owned(),
        }
    }

    /// Parses a rendered stamp, or returns `None` if `text` is not one.
    pub(super) fn parse(text: &str) -> Option<BuildStamp> {
        let (debug, target) = text.split_once('\t')?;
        let debug_assertions = debug.strip_prefix("debug_assertions=")?.parse().ok()?;
        let (arch, os) = target.strip_prefix("target=")?.split_once('-')?;
        Some(BuildStamp {
            debug_assertions,
            arch: arch.to_owned(),
            os: os.to_owned(),
        })
    }
}

/// Renders the stamp as [`BuildStamp::parse`] reads it.
impl std::fmt::Display for BuildStamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "debug_assertions={}\ttarget={}-{}",
            self.debug_assertions, self.arch, self.os
        )
    }
}

/// The environment variable through which a runner names the source tree
/// its build was compiled from; the just recipes set it to the git tree of
/// the checkout's tracked files.
pub const SOURCE_TREE_ENV: &str = "AMP_BOARD_SOURCE_TREE";

/// Returns the source tree [`SOURCE_TREE_ENV`] names.
///
/// # Errors
///
/// Returns [`io::ErrorKind::NotFound`] if the variable is unset or empty: a
/// capture or calibration must name the tree it measured.
pub fn source_tree() -> io::Result<String> {
    std::env::var(SOURCE_TREE_ENV)
        .ok()
        .filter(|tree| !tree.is_empty())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "{SOURCE_TREE_ENV} must name the source tree the build was compiled from \
                     (the just recipes set it)"
                ),
            )
        })
}

/// Every currency, in display order.
///
/// Built from a [`ByCurrency`] so that adding a currency cannot leave the
/// capture's parser unable to read it.
fn currencies() -> [Currency; 4] {
    ByCurrency::from_fn(|_| ())
        .each()
        .map(|(currency, _)| currency)
}

/// Write every cell's exact readings from one ladder sweep to `out` in the
/// capture format.
///
/// The ladder is the one acceptance judges, so a capture reads what the board
/// of record reads. The capture records every currency the ladder measured,
/// and only those, under `source_tree`, the runner's name for the source the
/// ladder's build was compiled from.
///
/// # Panics
///
/// Panics if a currency is measured in some samples but not others (its rows
/// would be incomplete), or if `source_tree` contains a tab or newline.
pub fn capture(ladder: &Ladder, source_tree: &str, out: &mut dyn Write) -> io::Result<()> {
    assert!(
        !source_tree.contains(['\t', '\n']),
        "amp-board capture: a source tree's name may not contain a tab or newline"
    );
    let sweeps = ladder.by_scale().map(|(_, cells)| cells);
    let measured = ByCurrency::from_fn(|currency| measured(ladder, currency));
    let cells = sweeps[0].len();
    let mut rows = Vec::with_capacity(cells * currencies().len());
    for cell in 0..cells {
        let (op, family, ..) = sweeps[0][cell];
        // The ladder's sweeps share one cell grid, so position `cell` is the
        // same operation and family in each.
        let samples: Vec<&Sample> = sweeps
            .iter()
            .flat_map(|sweep| {
                let (_, _, s1, s2) = &sweep[cell];
                [s1, s2]
            })
            .collect();
        for currency in currencies() {
            if !*measured.get(currency) {
                continue;
            }
            let points = std::array::from_fn(|index| {
                let sample = samples[index];
                Point {
                    axis: trend_units(sample, currency),
                    reading: sample
                        .readings
                        .get(currency)
                        .expect("a measured currency reads in every sample"),
                }
            });
            rows.push(Row {
                op: op.to_owned(),
                family: family.to_owned(),
                currency,
                points,
            });
        }
    }
    write_capture(
        &Capture {
            measured_on: ladder.measured_on.clone(),
            source_tree: source_tree.to_owned(),
            rows,
        },
        out,
    )
}

/// Writes [`capture`]'s text for `ladder` under `source_tree` to the file at
/// `path`, replacing any file there only once the whole capture is written.
///
/// A failed write leaves any earlier capture at `path` intact.
///
/// # Errors
///
/// Returns any error writing the capture beside `path` or renaming it into
/// place.
///
/// # Panics
///
/// Panics as [`capture`] does.
pub fn capture_file(ladder: &Ladder, source_tree: &str, path: &Path) -> io::Result<()> {
    let mut text = Vec::new();
    capture(ladder, source_tree, &mut text)?;
    write_atomically(path, &text)
}

/// Whether the ladder measured `currency`.
///
/// # Panics
///
/// Panics if the ladder measured `currency` in some samples but not others.
pub(super) fn measured(ladder: &Ladder, currency: Currency) -> bool {
    let read: BTreeSet<bool> = ladder
        .by_scale()
        .into_iter()
        .flat_map(|(_, cells)| cells.iter().flat_map(|(_, _, s1, s2)| [s1, s2]))
        .map(|sample| sample.readings.get(currency).is_some())
        .collect();
    assert!(
        read.len() <= 1,
        "amp-board capture: the {} counter read in some samples but not others",
        currency.label()
    );
    read.contains(&true)
}

/// One capture's build stamp, source tree, and every reading it holds, keyed
/// by operation, family, and currency, each with the readings' axes in ladder
/// order.
pub(super) struct Readings {
    /// The build that measured the readings.
    pub(super) measured_on: BuildStamp,
    /// The source tree the build was compiled from, as the runner named it.
    pub(super) source_tree: String,
    /// Each row's `(axis, reading)` pairs in ladder order.
    pub(super) rows: BTreeMap<(String, String, Currency), [(u64, u64); LADDER_SIZES]>,
}

/// Reads a capture's stamp and readings, for a calibration that merges
/// another build's readings with its own.
///
/// `name` labels the capture in errors.
///
/// # Errors
///
/// Returns [`io::ErrorKind::InvalidData`] if `text` is not a well-formed
/// capture.
pub(super) fn readings(name: &str, text: &str) -> io::Result<Readings> {
    let capture = parse(name, text)?;
    let rows = capture
        .rows
        .into_iter()
        .map(|row| {
            let points = row.points.map(|point| (point.axis, point.reading));
            ((row.op, row.family, row.currency), points)
        })
        .collect();
    Ok(Readings {
        measured_on: capture.measured_on,
        source_tree: capture.source_tree,
        rows,
    })
}

/// Write `capture` in the capture format.
fn write_capture(capture: &Capture, out: &mut dyn Write) -> io::Result<()> {
    writeln!(
        out,
        "{FORMAT}\t{stamp}\ttree={tree}",
        stamp = capture.measured_on,
        tree = capture.source_tree
    )?;
    for row in &capture.rows {
        for (index, point) in row.points.iter().enumerate() {
            writeln!(
                out,
                "{op}\t{family}\t{currency}\t{scale}\t{sample}\t{axis}\t{reading}",
                op = row.op,
                family = row.family,
                currency = row.currency.label(),
                scale = LADDER_SCALES[index / 2],
                sample = index % 2 + 1,
                axis = point.axis,
                reading = point.reading,
            )?;
        }
    }
    writeln!(out, "end {}", capture.rows.len())
}

/// A malformed capture's error, naming the capture and the line.
fn malformed(name: &str, line: usize, detail: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("amp-board compare: {name} capture, line {line}: {detail}"),
    )
}

/// Parse a capture, refusing anything [`write_capture`] would not write; in
/// particular, every proper prefix of a capture is refused.
///
/// `name` labels the capture in errors.
fn parse(name: &str, text: &str) -> io::Result<Capture> {
    let Some(body) = text.strip_suffix('\n') else {
        return Err(malformed(
            name,
            text.split('\n').count(),
            "the capture does not end with a newline, so it is truncated",
        ));
    };
    let lines: Vec<(&str, usize)> = body.split('\n').zip(1..).collect();
    let header = lines[0].0;
    let fields = header
        .strip_prefix(FORMAT)
        .and_then(|rest| rest.strip_prefix('\t'))
        .ok_or_else(|| malformed(name, 1, &format!("the header must begin with {FORMAT:?}")))?;
    let (stamp, tree) = fields
        .rsplit_once('\t')
        .ok_or_else(|| malformed(name, 1, "the header must end with the source tree"))?;
    let measured_on = BuildStamp::parse(stamp)
        .ok_or_else(|| malformed(name, 1, "the header carries no build stamp"))?;
    let source_tree = tree
        .strip_prefix("tree=")
        .ok_or_else(|| malformed(name, 1, "the header must end with `tree=<id>`"))?
        .to_owned();
    let (end, readings) = match lines[1..].split_last() {
        Some((&(line, number), readings)) => match line.strip_prefix("end ") {
            Some(count) => ((count, number), readings),
            None => return Err(malformed(name, number, "the capture has no end line")),
        },
        None => return Err(malformed(name, 1, "the capture has no end line")),
    };
    if readings.len() % LADDER_SIZES != 0 {
        return Err(malformed(name, end.1, "the last row is truncated"));
    }
    let mut rows = Vec::with_capacity(readings.len() / LADDER_SIZES);
    let mut seen = BTreeSet::new();
    for row_lines in readings.chunks_exact(LADDER_SIZES) {
        let (op, family, currency, first) = parse_line(name, row_lines[0], 0)?;
        let mut points = [first; LADDER_SIZES];
        for (index, &line) in row_lines.iter().enumerate().skip(1) {
            let (line_op, line_family, line_currency, point) = parse_line(name, line, index)?;
            if (line_op, line_family, line_currency) != (op, family, currency) {
                return Err(malformed(
                    name,
                    line.1,
                    "a row's readings must share one operation, family, and currency",
                ));
            }
            points[index] = point;
        }
        if !seen.insert((op, family, currency.label())) {
            return Err(malformed(name, row_lines[0].1, "duplicate row"));
        }
        rows.push(Row {
            op: op.to_owned(),
            family: family.to_owned(),
            currency,
            points,
        });
    }
    if end.0 != rows.len().to_string() {
        return Err(malformed(
            name,
            end.1,
            &format!("the end line must count the {} rows read", rows.len()),
        ));
    }
    Ok(Capture {
        measured_on,
        source_tree,
        rows,
    })
}

/// Parse one reading line, numbered `line.1`, which must hold the ladder size
/// at `index`.
fn parse_line<'a>(
    name: &str,
    line: (&'a str, usize),
    index: usize,
) -> io::Result<(&'a str, &'a str, Currency, Point)> {
    let (text, number) = line;
    let fields: Vec<&str> = text.split('\t').collect();
    let [op, family, currency, scale, sample, axis, reading] = fields[..] else {
        return Err(malformed(
            name,
            number,
            "expected seven tab-separated fields",
        ));
    };
    let currency = currencies()
        .into_iter()
        .find(|known| known.label() == currency)
        .ok_or_else(|| malformed(name, number, "unknown currency"))?;
    let (expected_scale, expected_sample) = (LADDER_SCALES[index / 2], index % 2 + 1);
    if scale != expected_scale.to_string() || sample != expected_sample.to_string() {
        return Err(malformed(
            name,
            number,
            &format!("expected ladder scale {expected_scale} sample {expected_sample} here"),
        ));
    }
    let count = |field: &str| {
        field
            .parse::<u64>()
            .map_err(|_| malformed(name, number, "malformed axis or reading"))
    };
    let point = Point {
        axis: count(axis)?,
        reading: count(reading)?,
    };
    Ok((op, family, currency, point))
}

/// How one cell and currency changed between two captures.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Outcome {
    /// The difference is zero at every ladder size.
    Unchanged,
    /// The difference is `shift` at every growth-ladder size.
    Constant {
        /// The difference at each growth-ladder size.
        shift: i128,
    },
    /// The difference moves further from zero at every step up the growth
    /// ladder, never crossing it.
    Growing {
        /// The difference's change across the growth ladder per axis unit,
        /// `None` when the axis does not grow across it.
        slope: Option<f64>,
    },
    /// The difference changes along the growth ladder without growing in
    /// magnitude at every step.
    Uneven,
    /// The captures measured the cell at different sizes.
    Resized,
    /// The row is in the before capture only.
    OnlyBefore,
    /// The row is in the after capture only.
    OnlyAfter,
}

impl Outcome {
    /// The outcome's printed label.
    fn label(self) -> &'static str {
        match self {
            Outcome::Growing { .. } => "growing",
            Outcome::Uneven => "uneven",
            Outcome::Resized => "resized",
            Outcome::Constant { .. } => "constant",
            Outcome::OnlyBefore => "only-before",
            Outcome::OnlyAfter => "only-after",
            Outcome::Unchanged => "unchanged",
        }
    }

    /// The outcome's place in the printed order: the classes that can signal
    /// growth or a changed result first.
    fn rank(self) -> u8 {
        match self {
            Outcome::Growing { .. } => 0,
            Outcome::Uneven => 1,
            Outcome::Resized => 2,
            Outcome::Constant { .. } => 3,
            Outcome::OnlyBefore => 4,
            Outcome::OnlyAfter => 5,
            Outcome::Unchanged => 6,
        }
    }
}

/// One compared row: the outcome and the evidence printed beside it.
#[derive(Clone, Debug)]
struct Compared {
    /// The row's identity and the after capture's readings, or the before
    /// capture's for a row only it holds.
    row: Row,
    /// The before capture's readings, where it holds the row.
    before: Option<[Point; LADDER_SIZES]>,
    /// How the row changed.
    outcome: Outcome,
}

/// Classify the difference `after - before` at the ladder's sizes, measured at
/// the same axes in both captures.
///
/// The classes are [`compare`]'s; its documentation states the rule.
fn classify(differences: &[i128; LADDER_SIZES], axes: &[u64; LADDER_SIZES]) -> Outcome {
    if differences.iter().all(|&difference| difference == 0) {
        return Outcome::Unchanged;
    }
    let growth = &differences[GROWTH_START..];
    if growth.iter().all(|&difference| difference == growth[0]) {
        return Outcome::Constant { shift: growth[0] };
    }
    // Each step must move the difference further from zero, never across it:
    // a difference shrinking toward zero is bounded by its first value.
    let grows = growth.windows(2).all(|pair| {
        pair[1].unsigned_abs() > pair[0].unsigned_abs() && pair[0].signum() * pair[1].signum() >= 0
    });
    if grows {
        let (first, last) = (axes[GROWTH_START], axes[LADDER_SIZES - 1]);
        let slope = (last > first).then(|| {
            (differences[LADDER_SIZES - 1] - differences[GROWTH_START]) as f64
                / (last - first) as f64
        });
        return Outcome::Growing { slope };
    }
    Outcome::Uneven
}

/// Compare two parsed captures row by row, in the after capture's order, then
/// the rows only the before capture holds, in its order.
fn compare_captures(before: Capture, after: Capture) -> Vec<Compared> {
    let mut remaining: BTreeMap<(String, String, &'static str), Row> = before
        .rows
        .iter()
        .map(|row| {
            (
                (row.op.clone(), row.family.clone(), row.currency.label()),
                row.clone(),
            )
        })
        .collect();
    let mut compared: Vec<Compared> = after
        .rows
        .into_iter()
        .map(|row| {
            let key = (row.op.clone(), row.family.clone(), row.currency.label());
            let Some(old) = remaining.remove(&key) else {
                return Compared {
                    row,
                    before: None,
                    outcome: Outcome::OnlyAfter,
                };
            };
            let axes = row.points.map(|point| point.axis);
            let outcome = if old.points.map(|point| point.axis) == axes {
                let differences = std::array::from_fn(|index| {
                    i128::from(row.points[index].reading) - i128::from(old.points[index].reading)
                });
                classify(&differences, &axes)
            } else {
                Outcome::Resized
            };
            Compared {
                row,
                before: Some(old.points),
                outcome,
            }
        })
        .collect();
    for row in before.rows {
        let key = (row.op.clone(), row.family.clone(), row.currency.label());
        if remaining.remove(&key).is_some() {
            compared.push(Compared {
                row,
                before: None,
                outcome: Outcome::OnlyBefore,
            });
        }
    }
    compared
}

/// Render one compared row's evidence: the per-size differences as
/// `axis:difference`, the small-input pair set apart from the growth ladder;
/// for a resized row, both captures' axes; for a row in one capture only,
/// nothing.
fn evidence(compared: &Compared) -> String {
    let after = &compared.row.points;
    let Some(before) = &compared.before else {
        return String::new();
    };
    if compared.outcome == Outcome::Resized {
        let axes = |points: &[Point; LADDER_SIZES]| {
            points
                .iter()
                .map(|point| point.axis.to_string())
                .collect::<Vec<_>>()
                .join(" ")
        };
        return format!("axes before {} after {}", axes(before), axes(after));
    }
    let difference = |index: usize| {
        let (old, new) = (before[index], after[index]);
        format!(
            "{}:{:+}",
            new.axis,
            i128::from(new.reading) - i128::from(old.reading)
        )
    };
    let span =
        |indices: std::ops::Range<usize>| indices.map(difference).collect::<Vec<_>>().join(" ");
    format!(
        "small {}  growth {}",
        span(0..GROWTH_START),
        span(GROWTH_START..LADDER_SIZES)
    )
}

/// Compare two captures and print how every cell's readings changed, each
/// currency classified by how its difference changes with input size.
///
/// `before` and `after` are capture texts, as [`capture`] writes them. The
/// first printed line counts the rows (one per cell and currency) in each
/// class. Then every row that changed prints on one line: its class, currency,
/// operation and family, the class's figure (a constant's shift, a growing
/// difference's slope), and the difference at each ladder size as
/// `axis:difference`, the small-input pair apart from the growth ladder. Rows
/// print grouped by class in the order listed below, and unchanged rows are
/// only counted.
///
/// # The classification
///
/// A row present in both captures and measured at the same axes is classified
/// by its difference `after − before`. The four growth-ladder sizes decide its
/// shape, because they alone feed the board's growth fit; minimum allocation
/// sizes dominate the small-input pair, which would otherwise read as changes
/// in shape.
///
/// - **growing**: the difference moves further from zero at every step up the
///   growth ladder, never crossing it. The slope is the difference's
///   change from the first growth size to the last, divided by their axis
///   distance: reading units per axis unit. A negative slope is a saving that
///   grows with input.
/// - **uneven**: the difference changes along the growth ladder without
///   growing in magnitude at every step. A fixed change to the heap can land
///   here rather than in constant: a transient allocation raises the
///   peak only while it overlaps the peak, so larger inputs' own allocations
///   absorb it and the difference shrinks toward zero across the ladder.
/// - **constant**: the difference is the same nonzero value at all four
///   growth sizes, or zero there with a change in the small-input pair alone
///   (a zero shift).
/// - **unchanged**: the difference is zero at all six sizes.
///
/// Rows outside that rule are reported, never classified: **resized** when the
/// captures measured the row at different axes (an I/O-denominated
/// operation's output size changed), printed before the constant rows, and
/// **only-before** or **only-after**, last, when one capture lacks the row.
///
/// # What the classification cannot distinguish
///
/// - A step from growth seen late. A fixed cost that appears once an
///   allocation crosses a size threshold inside the ladder, such as a 112-byte
///   block present only at the largest size, is uneven; so is a proportional
///   change too small to register below the largest sizes. Telling them apart
///   takes the mechanism, not more of these four sizes.
/// - Growth below the counter's granularity at every step, or growth that
///   begins beyond the ladder's top, reads as constant or unchanged.
/// - The order of growth. A linear and a quadratic difference are both
///   growing, and the slope is a secant across the ladder. The printed
///   differences show the order: a proportional difference doubles from each
///   size to its double.
/// - A fixed shift beside a proportional change. The row is growing when the
///   proportional part pushes the difference further from zero at every step,
///   which opposite signs allow only once the proportional part outweighs the
///   shift at the first growth size, and the slope omits the shift. Otherwise
///   the difference shrinks toward zero or crosses it, and the row is uneven.
///
/// # Errors
///
/// Returns [`io::ErrorKind::InvalidData`] if either text is not a well-formed
/// capture, or if the captures' builds differ (in debug-assertion settings or
/// targets), and any error writing to `out`. Their source trees may differ.
pub fn compare(before: &str, after: &str, out: &mut dyn Write) -> io::Result<()> {
    let before = parse("before", before)?;
    let after = parse("after", after)?;
    if before.measured_on != after.measured_on {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "amp-board compare: the captures come from different builds, so their \
                 readings are not comparable: before {}, after {}",
                before.measured_on, after.measured_on
            ),
        ));
    }
    let mut compared = compare_captures(before, after);
    compared.sort_by_key(|row| row.outcome.rank());
    let mut counts: BTreeMap<u8, (&'static str, usize)> = BTreeMap::new();
    for row in &compared {
        counts
            .entry(row.outcome.rank())
            .or_insert((row.outcome.label(), 0))
            .1 += 1;
    }
    let summary: Vec<String> = counts
        .values()
        .map(|(label, count)| format!("{count} {label}"))
        .collect();
    writeln!(
        out,
        "amp-board compare: {} rows: {}",
        compared.len(),
        summary.join(", ")
    )?;
    for row in compared
        .iter()
        .filter(|row| row.outcome != Outcome::Unchanged)
    {
        let figure = match row.outcome {
            Outcome::Constant { shift } => format!("shift {shift:+}"),
            Outcome::Growing { slope: Some(slope) } => format!("slope {slope:+.3e}"),
            Outcome::Growing { slope: None } => "slope n/a".to_owned(),
            _ => String::new(),
        };
        writeln!(
            out,
            "{label:<11} {currency:<5} {op} x {family}  {figure}  {evidence}",
            label = row.outcome.label(),
            currency = row.row.currency.label(),
            op = row.row.op,
            family = row.row.family,
            evidence = evidence(row),
        )?;
    }
    Ok(())
}
