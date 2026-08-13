//! Persisting a survey's raw atlases: measure once, render many times.
//!
//! A full survey spends hours measuring in the guest and milliseconds
//! rendering; coupling the two means every figure adjustment costs a
//! re-measure. The dump decouples them: a measuring run persists every
//! operation's complete render input as it finishes, and a later run
//! replays rendering from the files in seconds — any font scale, any
//! future styling — with no guest, no count tables, and no samplers.
//!
//! # Layout
//!
//! A dump is a directory of JSON files:
//!
//! - `atlas.json` — the index: the format banner, the run's
//!   [`RenderMeta`], and the operation names in measuring order (the
//!   gallery's order). Rewritten atomically after every finished
//!   operation, so a run that dies keeps an index naming exactly the
//!   operations whose files landed.
//! - `<op>.json` — one document per operation: the same banner and meta
//!   (each file stands alone), the operation's [`AtlasData`] (identity,
//!   every raw sample, every overlay point), and its [`HeatGrid`] (the
//!   binned heatmap exactly as rendered: axis domains, per-column
//!   medians, peaks, and bin counts).
//!
//! Every file is one well-formed JSON document, so document tooling
//! (typst's `json()` loader) reads both the raw samples and the
//! render-ready cell matrix directly — no converter, no re-binning.
//!
//! A committed dump stores its documents gzipped (`<name>.json.gz`, for
//! tree weight); [`read`] finds and decompresses them transparently
//! wherever the plain file is absent.
//!
//! # Strictness
//!
//! A dump is environmental input, so [`read`] rejects malformed dumps
//! as errors, never panics: unknown fields, banner mismatches, meta
//! that differs between the index and an operation file, an operation
//! file whose name disagrees with the index entry that claimed it, an
//! empty sample list, and a stored grid that does not equal the grid
//! recomputed from the raw samples by [`aggregate`]. The grid check is
//! the two-ways seam: the grid is derivable data persisted for
//! downstream consumers, and a dump whose stored grid disagrees with
//! this renderer's aggregation must not silently re-render into
//! figures that contradict the cells other documents read.

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::render::{aggregate, AtlasData, HeatGrid, RenderMeta};

#[cfg(test)]
mod tests;

/// The index file's name inside a dump directory.
pub const INDEX_FILE: &str = "atlas.json";

/// The index document's format banner.
const INDEX_FORMAT: &str = "fuelscape-atlas-index";

/// The per-operation document's format banner.
const OP_FORMAT: &str = "fuelscape-op-atlas";

/// The dump format version both banners carry.
const FORMAT_VERSION: u32 = 1;

/// The index document: run provenance plus the ordered operation list.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct IndexDoc {
    /// Always [`INDEX_FORMAT`].
    format: String,
    /// Always [`FORMAT_VERSION`].
    version: u32,
    /// The run's provenance, identical in every file of the dump.
    meta: RenderMeta,
    /// Operation names in measuring order (the gallery's order); one
    /// `<name>.json` per entry.
    ops: Vec<String>,
}

/// One operation's document: identity, raw data, and the binned form.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OpDoc {
    /// Always [`OP_FORMAT`].
    format: String,
    /// Always [`FORMAT_VERSION`].
    version: u32,
    /// The run's provenance, identical in every file of the dump.
    meta: RenderMeta,
    /// The operation's complete render input.
    op: AtlasData,
    /// The binned heatmap, exactly [`aggregate`] of `op`.
    grid: HeatGrid,
}

/// An incremental dump writer: one `append` per measured operation.
///
/// Construction writes an empty index; every append writes the
/// operation's document and rewrites the index, both atomically
/// (temporary file, then rename), so the dump on disk is well-formed
/// after every step.
pub struct DumpWriter {
    dir: PathBuf,
    meta: RenderMeta,
    ops: Vec<String>,
}

impl DumpWriter {
    /// Open a dump in `dir` (created if absent) and write the empty
    /// index.
    pub fn new(dir: &Path, meta: RenderMeta) -> io::Result<DumpWriter> {
        std::fs::create_dir_all(dir)?;
        let writer = DumpWriter {
            dir: dir.to_path_buf(),
            meta,
            ops: Vec::new(),
        };
        writer.write_index()?;
        Ok(writer)
    }

    /// Persist one operation's atlas (raw data plus its computed
    /// [`HeatGrid`]) and extend the index, returning the operation
    /// file's path.
    pub fn append(&mut self, data: &AtlasData) -> io::Result<PathBuf> {
        let doc = OpDoc {
            format: OP_FORMAT.to_string(),
            version: FORMAT_VERSION,
            meta: self.meta.clone(),
            op: data.clone(),
            grid: aggregate(data),
        };
        let path = self.dir.join(format!("{}.json", data.op_name));
        // Compact for the operation documents: the sample list dominates
        // them, and their readers are loaders, not people.
        write_atomic(&path, &serde_json::to_vec(&doc)?)?;
        self.ops.push(data.op_name.clone());
        self.write_index()?;
        Ok(path)
    }

    /// Rewrite the index for the operations appended so far.
    fn write_index(&self) -> io::Result<()> {
        let doc = IndexDoc {
            format: INDEX_FORMAT.to_string(),
            version: FORMAT_VERSION,
            meta: self.meta.clone(),
            ops: self.ops.clone(),
        };
        // Pretty for the index: it is small, and it is the file an
        // operator opens to see what a dump holds.
        write_atomic(
            &self.dir.join(INDEX_FILE),
            &serde_json::to_vec_pretty(&doc)?,
        )
    }
}

/// Write `bytes` to `path` atomically: a sibling temporary file, then a
/// rename, so a crash never leaves a torn document where a whole one
/// stood.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

/// Load a whole dump: the run's provenance and every operation's atlas,
/// in the index's order.
///
/// `path` names the dump: its `atlas.json` or the directory holding it.
///
/// # Errors
///
/// Any I/O failure, and every strictness rejection the module doc
/// enumerates; the error message names the offending file and check.
pub fn read(path: &Path) -> io::Result<(RenderMeta, Vec<AtlasData>)> {
    let index_path = if path.is_dir() {
        path.join(INDEX_FILE)
    } else {
        path.to_path_buf()
    };
    let dir = index_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let index: IndexDoc = parse(&index_path)?;
    check_banner(
        &index_path,
        &index.format,
        INDEX_FORMAT,
        index.version,
        FORMAT_VERSION,
    )?;
    if index.ops.is_empty() {
        return Err(malformed(&index_path, "the dump names no operations"));
    }

    let mut atlases = Vec::with_capacity(index.ops.len());
    for name in &index.ops {
        let op_path = dir.join(format!("{name}.json"));
        let doc: OpDoc = parse(&op_path)?;
        check_banner(
            &op_path,
            &doc.format,
            OP_FORMAT,
            doc.version,
            FORMAT_VERSION,
        )?;
        if doc.meta != index.meta {
            return Err(malformed(&op_path, "meta differs from the index's"));
        }
        if doc.op.op_name != *name {
            return Err(malformed(
                &op_path,
                &format!(
                    "holds operation {:?}, index claims {name:?}",
                    doc.op.op_name
                ),
            ));
        }
        if doc.op.samples.is_empty() {
            return Err(malformed(&op_path, "has no samples"));
        }
        if doc.grid != aggregate(&doc.op) {
            return Err(malformed(
                &op_path,
                "stored grid does not match the grid recomputed from its samples \
                 (the dump was altered, or it predates a change to the aggregation)",
            ));
        }
        atlases.push(doc.op);
    }
    Ok((index.meta, atlases))
}

/// Read and strictly deserialize one dump document.
///
/// A document may be stored gzipped under `<name>.json.gz` (the
/// committed dump is, for tree weight); when `path` itself is absent,
/// the `.gz` sibling is read and decompressed transparently, and every
/// strictness rejection still names the file it came from.
pub(crate) fn parse<T: serde::de::DeserializeOwned>(path: &Path) -> io::Result<T> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let mut gz = path.as_os_str().to_owned();
            gz.push(".gz");
            let gz = Path::new(&gz);
            let file = std::fs::File::open(gz).map_err(|_| {
                // Report the plain path's absence: it is the name the
                // caller asked for; the `.gz` probe is an implementation
                // detail of the storage.
                io::Error::new(
                    e.kind(),
                    format!("{}: {e} (nor a .gz sibling)", path.display()),
                )
            })?;
            let mut bytes = Vec::new();
            io::Read::read_to_end(&mut flate2::read::GzDecoder::new(file), &mut bytes)
                .map_err(|e| io::Error::new(e.kind(), format!("{}: {e}", gz.display())))?;
            bytes
        }
        Err(e) => return Err(io::Error::new(e.kind(), format!("{}: {e}", path.display()))),
    };
    serde_json::from_slice(&bytes).map_err(|e| malformed(path, &e.to_string()))
}

/// Reject a document whose format banner is not the expected one.
pub(crate) fn check_banner(
    path: &Path,
    format: &str,
    expected: &str,
    version: u32,
    expected_version: u32,
) -> io::Result<()> {
    if format != expected || version != expected_version {
        return Err(malformed(
            path,
            &format!(
                "format {format:?} v{version}, this reader wants {expected:?} v{expected_version}"
            ),
        ));
    }
    Ok(())
}

/// A strictness rejection, named after the offending file.
pub(crate) fn malformed(path: &Path, why: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("{}: {why}", path.display()),
    )
}
