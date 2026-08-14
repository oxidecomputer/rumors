//! Compacting a dump into the rustdoc fuelscape widgets' committed data.
//!
//! The raw dump is the measurement of record, and it is far too large to
//! ship inside the `before` crate package — while the rustdoc fuelscape
//! widgets need only per-column fuel histograms, three orders of
//! magnitude smaller. This module derives that compact form: the
//! `fuelscape-widget-data` documents committed under
//! `crates/before/fuelscape/`, which `before`'s `build.rs` formats into
//! the doc islands. Compaction is a pure function of (dump, roster), so
//! the committed files are re-derivable and byte-comparable at any time
//! (the `fuelscape-verify` recipe is that comparison).
//!
//! # Layout
//!
//! A compact dataset is a directory of JSON files, in the dump's idiom:
//!
//! - `index.json` — the format banner, the measuring run's
//!   [`RenderMeta`], and the operation names in the dump's order.
//! - `<op>.json` — one document per operation: the same banner and meta,
//!   and the operation's [`WidgetOp`] — identity, the doc-facing
//!   complexity strings, the per-column fuel histograms, and the overlay
//!   points carried through for later widget revisions.
//!
//! # Binning
//!
//! Each column's fuel values become a histogram over log₂(fuel) at
//! [`RES`] octaves per bin: bin `k` covers `[2^(k·RES), 2^((k+1)·RES))`,
//! `k0` is the lowest occupied bin, and `c` counts occupancy from `k0`
//! upward. The widget derives quantiles and the density field from these
//! counts alone; raw samples never leave the dump.
//!
//! # Doc-facing strings
//!
//! Each operation's row in [`ROSTER`](crate::ops::ROSTER) carries the two
//! complexity statements its doc island shows: the rustdoc *contract*
//! (structure-size variables, e.g. `O(|self| + |party|)`) and the
//! *claim* in the widget's expression grammar, denominated in total
//! packed input bytes (e.g. `n log n`). Compaction stamps both into the
//! operation's document, and rejects a dump whose recorded
//! `size_measure` differs from the roster row's current one: measurements
//! drawn from an input space the roster no longer declares must not
//! silently caption today's docs — re-measure, or re-point the roster,
//! deliberately.
//!
//! # Strictness
//!
//! A compact dataset is environmental input to `before`'s doc build, so
//! [`read`] rejects malformed data as errors, never panics: unknown
//! fields, banner mismatches, meta drift between files, name
//! disagreements, unsorted size axes, empty or non-canonical histograms
//! (a leading or trailing zero bin means `k0` or the length lies), a
//! non-positive bin resolution, empty complexity strings, and overlay
//! points no log-scale plot could place.

use std::collections::BTreeMap;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::dump;
use crate::render::{AtlasData, OverlayData, RenderMeta};

#[cfg(test)]
mod tests;

/// The index file's name inside a compact dataset directory.
pub const INDEX_FILE: &str = "index.json";

/// The index document's format banner.
const INDEX_FORMAT: &str = "fuelscape-widget-index";

/// The per-operation document's format banner.
const OP_FORMAT: &str = "fuelscape-widget-data";

/// The compact format version both banners carry.
const FORMAT_VERSION: u32 = 2;

/// Histogram resolution: octaves of fuel per bin.
///
/// The one binning constant in the pipeline; the widget reads it from
/// each document's `res` field rather than assuming it, so changing it
/// here re-bins the committed data on the next compaction and the docs
/// follow with no widget change.
pub const RES: f64 = 0.05;

/// The index document: run provenance plus the ordered operation list.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct IndexDoc {
    /// Always [`INDEX_FORMAT`].
    format: String,
    /// Always [`FORMAT_VERSION`].
    version: u32,
    /// The measuring run's provenance, identical in every file.
    meta: RenderMeta,
    /// Operation names in the dump's order; one `<name>.json` per entry.
    ops: Vec<String>,
}

/// One operation's document: banner, meta, and the widget's data.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OpDoc {
    /// Always [`OP_FORMAT`].
    format: String,
    /// Always [`FORMAT_VERSION`].
    version: u32,
    /// The measuring run's provenance, identical in every file.
    meta: RenderMeta,
    /// The operation's compact render input.
    op: WidgetOp,
}

/// One operation's compact widget data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WidgetOp {
    /// The operation's atlas name (also the document's file stem).
    pub op_name: String,
    /// The declared size measure of the measuring run, verbatim.
    pub size_measure: String,
    /// The variant label distinguishing this operation's chart where
    /// one doc site stacks several; empty for a site's only chart.
    pub variant: String,
    /// The rustdoc complexity contract, display text
    /// (e.g. `O(|self| + |party|)`).
    pub contract: String,
    /// The claimed growth in the widget's expression grammar,
    /// denominated in total packed input bytes (e.g. `n log n`).
    pub claim: String,
    /// Octaves of fuel per histogram bin ([`RES`] at compaction time).
    pub res: f64,
    /// The size axis: total packed input bytes per column, strictly
    /// ascending.
    pub sizes: Vec<usize>,
    /// One fuel histogram per entry of `sizes`.
    pub cols: Vec<WidgetCol>,
    /// The committed adversarial-family points, carried through for
    /// later widget revisions (the first widget release draws none).
    pub overlay: Vec<OverlayData>,
}

/// One column's fuel histogram.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WidgetCol {
    /// The lowest occupied bin: fuel `f` lands in bin
    /// `floor(log2(f) / res)`.
    pub k0: i64,
    /// Occupancy counts from `k0` upward; first and last entries are
    /// nonzero (the histogram is tight at both ends).
    pub c: Vec<u32>,
}

/// Compact one atlas: bin its samples per column and stamp the
/// doc-facing complexity strings.
///
/// # Errors
///
/// A zero-fuel sample (the guest meters every call, so zero fuel is a
/// measurement bug the log-scale histogram cannot even place), and an
/// empty sample list.
pub fn compact(
    data: &AtlasData,
    variant: &str,
    contract: &str,
    claim: &str,
) -> io::Result<WidgetOp> {
    let mut by: BTreeMap<usize, Vec<u64>> = BTreeMap::new();
    for s in &data.samples {
        if s.fuel == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{}: a size-{} sample reads zero fuel; the guest meters \
                     every call, so this is a measurement bug",
                    data.op_name, s.size
                ),
            ));
        }
        by.entry(s.size).or_default().push(s.fuel);
    }
    if by.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{}: the atlas has no samples", data.op_name),
        ));
    }
    let (sizes, cols) = by
        .into_iter()
        .map(|(size, fuels)| {
            // libm, not std: the committed files are re-derived and
            // byte-compared on other hosts (`fuelscape-verify`), and
            // platform log2 kernels disagree by an ulp at bin
            // boundaries; libm's pure-Rust kernel does not.
            let ks: Vec<i64> = fuels
                .iter()
                .map(|&f| (libm::log2(f as f64) / RES).floor() as i64)
                .collect();
            let k0 = *ks.iter().min().expect("every column has a sample");
            let hi = *ks.iter().max().expect("every column has a sample");
            let mut c = vec![0u32; usize::try_from(hi - k0 + 1).expect("bin span fits")];
            for k in ks {
                c[usize::try_from(k - k0).expect("bin offset fits")] += 1;
            }
            (size, WidgetCol { k0, c })
        })
        .unzip();
    Ok(WidgetOp {
        op_name: data.op_name.clone(),
        size_measure: data.size_measure.clone(),
        variant: variant.to_string(),
        contract: contract.to_string(),
        claim: claim.to_string(),
        res: RES,
        sizes,
        cols,
        overlay: data.overlay.clone(),
    })
}

/// Compact a whole dump into `out`: one document per operation, plus the
/// index, all written atomically.
///
/// Each operation's contract and claim come from its
/// [`ROSTER`](crate::ops::ROSTER) row, and the row's declared
/// `size_measure` must equal the dump's recorded one (module doc,
/// *Doc-facing strings*). Returns the operation names written, in the
/// dump's order.
///
/// # Errors
///
/// Any dump-loading failure, a dump operation missing from the roster,
/// a `size_measure` disagreement, and any I/O failure writing `out`.
pub fn compact_dump(dump_path: &Path, out: &Path) -> io::Result<Vec<String>> {
    let (meta, atlases) = dump::read(dump_path)?;
    let mut ops = Vec::with_capacity(atlases.len());
    for data in &atlases {
        let spec = crate::ops::ROSTER
            .iter()
            .find(|spec| spec.name == data.op_name)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{}: the dump holds an operation the roster no longer \
                         names; re-measure, or re-point the roster, deliberately",
                        data.op_name
                    ),
                )
            })?;
        if spec.size_measure != data.size_measure {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{}: the dump's size measure ({:?}) differs from the \
                     roster row's ({:?}); the measurements describe an input \
                     space the roster no longer declares — re-measure, or \
                     re-point the roster, deliberately",
                    data.op_name, data.size_measure, spec.size_measure
                ),
            ));
        }
        ops.push(compact(data, spec.variant, spec.contract, spec.claim)?);
    }
    write(out, &meta, &ops)?;
    Ok(ops.into_iter().map(|op| op.op_name).collect())
}

/// Write a compact dataset: every operation document, then the index,
/// each atomically.
pub fn write(dir: &Path, meta: &RenderMeta, ops: &[WidgetOp]) -> io::Result<()> {
    std::fs::create_dir_all(dir)?;
    for op in ops {
        let doc = OpDoc {
            format: OP_FORMAT.to_string(),
            version: FORMAT_VERSION,
            meta: meta.clone(),
            op: op.clone(),
        };
        // Compact serialization: the histograms dominate, and the
        // readers are loaders, not people.
        dump::write_atomic(
            &dir.join(format!("{}.json", op.op_name)),
            &serde_json::to_vec(&doc)?,
        )?;
    }
    let index = IndexDoc {
        format: INDEX_FORMAT.to_string(),
        version: FORMAT_VERSION,
        meta: meta.clone(),
        ops: ops.iter().map(|op| op.op_name.clone()).collect(),
    };
    // Pretty for the index: it is the file an operator opens.
    dump::write_atomic(&dir.join(INDEX_FILE), &serde_json::to_vec_pretty(&index)?)
}

/// Load a whole compact dataset: the measuring run's provenance and
/// every operation's widget data, in the index's order.
///
/// `path` names the dataset: its `index.json` or the directory holding
/// it.
///
/// # Errors
///
/// Any I/O failure, and every strictness rejection the module doc
/// enumerates; the error message names the offending file and check.
pub fn read(path: &Path) -> io::Result<(RenderMeta, Vec<WidgetOp>)> {
    let index_path = if path.is_dir() {
        path.join(INDEX_FILE)
    } else {
        path.to_path_buf()
    };
    let dir = index_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let index: IndexDoc = dump::parse(&index_path)?;
    dump::check_banner(
        &index_path,
        &index.format,
        INDEX_FORMAT,
        index.version,
        FORMAT_VERSION,
    )?;
    if index.ops.is_empty() {
        return Err(dump::malformed(
            &index_path,
            "the dataset names no operations",
        ));
    }

    let mut ops = Vec::with_capacity(index.ops.len());
    for name in &index.ops {
        let op_path = dir.join(format!("{name}.json"));
        let doc: OpDoc = dump::parse(&op_path)?;
        dump::check_banner(
            &op_path,
            &doc.format,
            OP_FORMAT,
            doc.version,
            FORMAT_VERSION,
        )?;
        if doc.meta != index.meta {
            return Err(dump::malformed(&op_path, "meta differs from the index's"));
        }
        if doc.op.op_name != *name {
            return Err(dump::malformed(
                &op_path,
                &format!(
                    "holds operation {:?}, index claims {name:?}",
                    doc.op.op_name
                ),
            ));
        }
        validate(&op_path, &doc.op)?;
        ops.push(doc.op);
    }
    Ok((index.meta, ops))
}

/// Reject a widget document violating any structural invariant the
/// module doc enumerates.
fn validate(path: &Path, op: &WidgetOp) -> io::Result<()> {
    let reject = |why: &str| Err(dump::malformed(path, why));
    if !(op.res.is_finite() && op.res > 0.0) {
        return reject("bin resolution must be finite and positive");
    }
    if op.contract.trim().is_empty() || op.claim.trim().is_empty() {
        return reject("the contract and claim strings must be non-empty");
    }
    if op.sizes.is_empty() {
        return reject("the size axis is empty");
    }
    if !op.sizes.windows(2).all(|w| w[0] < w[1]) {
        return reject("the size axis must be strictly ascending");
    }
    if op.sizes.len() != op.cols.len() {
        return reject("one histogram per size column is required");
    }
    for col in &op.cols {
        let (Some(first), Some(last)) = (col.c.first(), col.c.last()) else {
            return reject("a column's histogram is empty");
        };
        if *first == 0 || *last == 0 {
            return reject("a column's histogram must be tight (nonzero first and last bins)");
        }
    }
    for point in &op.overlay {
        if point.fuel == 0 || point.size == 0 {
            return reject(
                "an overlay point has zero size or fuel, which no log-scale plot places",
            );
        }
    }
    Ok(())
}
