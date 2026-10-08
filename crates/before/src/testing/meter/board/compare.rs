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
//! A capture is UTF-8 text. Its first line is a header: the format name and
//! version, whether debug assertions were compiled in, and the target
//! architecture and operating system, tab-separated. Readings are comparable
//! only between captures whose headers match, because debug assertions add
//! metered work and some operations allocate differently by target.
//!
//! Each line after the header, but for the last, holds one reading in seven
//! tab-separated fields:
//!
//! ```text
//! operation  family  currency  scale  sample  axis  reading
//! ```
//!
//! - `currency` is `heap`, `scan`, or `touch`.
//! - `scale` is the sampling scale and `sample` is `1` for its scaled size or
//!   `2` for that size's double. Each cell and currency has six lines, in
//!   ladder order: the small-input pair ([`SMALL_INPUT_SCALE`]), then the
//!   growth ladder's two pairs ([`DEFAULT_SCALE`], then [`LADDER_TOP_SCALE`]).
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

use super::ceilings::{DEFAULT_SCALE, LADDER_TOP_SCALE, SMALL_INPUT_SCALE};
use super::currency::{ByCurrency, Currency};
use super::judge::trend_units;
use super::measure::Sample;
use super::shard::{merge_samples, ShardSpawner};

#[cfg(test)]
mod tests;

/// The format name and version that open every capture's header.
const FORMAT: &str = "amp-board readings v1";

/// The measurement ladder's sampling scales in capture order: the small-input
/// pair's scale, then the growth ladder's two.
const LADDER_SCALES: [f64; 3] = [SMALL_INPUT_SCALE, DEFAULT_SCALE, LADDER_TOP_SCALE];

/// The readings each cell and currency carries: two samples at every scale.
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

/// A whole capture: its header line and its rows in board order.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Capture {
    /// The header line, without its newline.
    header: String,
    /// Every cell and currency's readings.
    rows: Vec<Row>,
}

/// The header this build writes: format, debug assertions, and target.
fn header() -> String {
    format!(
        "{FORMAT}\tdebug_assertions={debug}\ttarget={arch}-{os}",
        debug = cfg!(debug_assertions),
        arch = std::env::consts::ARCH,
        os = std::env::consts::OS,
    )
}

/// Every currency, in display order.
///
/// Built from a [`ByCurrency`] so that adding a currency cannot leave the
/// capture's parser unable to read it.
fn currencies() -> [Currency; 3] {
    ByCurrency {
        heap: (),
        scan: (),
        touch: (),
    }
    .each()
    .map(|(currency, _)| currency)
}

/// Sweep the board at every ladder scale across `shards` child processes and
/// write every cell's exact readings to `out` in the capture format.
///
/// `spawn` is invoked once per ladder scale, smallest first. The sweeps are the
/// ones acceptance judges, so a capture reads what the board of record reads.
///
/// # Panics
///
/// Panics if a counter is not compiled into this run (a capture records every
/// currency, so it requires the `touch-meter` and `scan-meter` features), if
/// the sweeps measure different cell grids, and on any protocol violation in a
/// child capture.
pub fn capture(shards: usize, spawn: ShardSpawner<'_>, out: &mut dyn Write) -> io::Result<()> {
    let mut sweeps = Vec::with_capacity(LADDER_SCALES.len());
    for scale in LADDER_SCALES {
        sweeps.push(merge_samples(scale, shards, &spawn(scale)?));
    }
    let cells = sweeps[0].len();
    assert!(
        sweeps.iter().all(|sweep| sweep.len() == cells),
        "amp-board capture: the ladder's sweeps measured different cell grids"
    );
    let mut rows = Vec::with_capacity(cells * currencies().len());
    for cell in 0..cells {
        let (op, family, ..) = sweeps[0][cell];
        let samples: Vec<&Sample> = sweeps
            .iter()
            .flat_map(|sweep| {
                let (sweep_op, sweep_family, s1, s2) = &sweep[cell];
                assert_eq!(
                    (*sweep_op, *sweep_family),
                    (op, family),
                    "amp-board capture: the ladder's sweeps measured different cell grids"
                );
                [s1, s2]
            })
            .collect();
        for currency in currencies() {
            let points = std::array::from_fn(|index| {
                let sample = samples[index];
                Point {
                    axis: trend_units(sample, currency) as u64,
                    reading: sample.readings.get(currency).unwrap_or_else(|| {
                        panic!(
                            "amp-board capture: the {} counter is not compiled in",
                            currency.label()
                        )
                    }),
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
            header: header(),
            rows,
        },
        out,
    )
}

/// Write `capture` in the capture format.
fn write_capture(capture: &Capture, out: &mut dyn Write) -> io::Result<()> {
    writeln!(out, "{}", capture.header)?;
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
    let header = match lines[0] {
        (line, _) if line.split('\t').next() == Some(FORMAT) => line.to_owned(),
        _ => {
            return Err(malformed(
                name,
                1,
                &format!("the header must begin with {FORMAT:?}"),
            ))
        }
    };
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
    Ok(Capture { header, rows })
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
/// capture, or if the headers differ (the captures come from different
/// formats, debug-assertion settings, or targets), and any error writing to
/// `out`.
pub fn compare(before: &str, after: &str, out: &mut dyn Write) -> io::Result<()> {
    let before = parse("before", before)?;
    let after = parse("after", after)?;
    if before.header != after.header {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "amp-board compare: the captures' headers differ, so their readings are not \
                 comparable: before {:?}, after {:?}",
                before.header, after.header
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
