//! The sampling plan: which cells run, and the deterministic execution
//! that fills them.
//!
//! A plan is a geometric size grid (byte doublings) crossed with the
//! operation roster, a fixed number of samples per column, and one base
//! seed. Every cell's RNG is [`crate::sample::cell_rng`] over
//! (operation, size, sample index), so the whole run is a pure function
//! of the plan: rayon's execution order cannot change a single draw, and
//! any cell replays alone.
//!
//! **The size measure, declared per row.** A unary operation's column at
//! size `N` draws its one input uniformly from the canonical inputs of
//! exactly `N` packed bytes. A k-operand operation's column at *total*
//! size `N` draws a split uniformly from the compositions of `N` into
//! `k` positive parts, then each operand uniformly at its exact size; a
//! slice row first draws its arity from the declared set. So the x-axis
//! is total packed input bytes everywhere, and every rendered plot
//! carries its row's exact declaration ([`crate::ops::OpSpec`]'s
//! `size_measure`).

use rand::Rng;
use rayon::prelude::*;

use fuzzfit_harness::wasm::Guest;

use crate::families::overlay_inputs;
use crate::ops::{Inputs, OpSpec, Operand, SLICE_ARITIES};
use crate::sample::{cell_rng, PartySampler, VersionSampler};

#[cfg(test)]
mod tests;

/// One atlas run's configuration.
pub struct Plan {
    /// The base seed every cell RNG derives from (stamped on renders).
    pub base_seed: u64,
    /// Samples per size column.
    pub samples_per_column: usize,
    /// The top of the geometric size grid, in packed bytes.
    pub max_bytes: usize,
}

impl Plan {
    /// The size columns for an input space's minimum total size: byte
    /// doublings from the smallest size the signature admits (one byte
    /// per operand).
    pub fn columns(&self, min_bytes: usize) -> Vec<usize> {
        let mut out = Vec::new();
        let mut n = min_bytes.max(1);
        while n <= self.max_bytes {
            out.push(n);
            n *= 2;
        }
        out
    }
}

/// One measured bulk sample.
pub struct CellSample {
    /// The column's total input size in packed bytes.
    pub size: usize,
    /// Fuel consumed by the one measured kernel call.
    pub fuel: u64,
    /// Whole-sample rejections spent drawing the inputs: version
    /// nonnegativity redraws, plus whole-pair redraws for rows whose
    /// input space declares a validity condition.
    pub rejected: u64,
}

/// One measured adversarial overlay point.
pub struct OverlayPoint {
    /// The family generator's name.
    pub family: &'static str,
    /// Total packed input bytes.
    pub size: usize,
    /// Fuel consumed by the one measured kernel call.
    pub fuel: u64,
}

/// One operation's complete atlas: the uniform cloud and the overlay.
pub struct OpAtlas {
    /// The roster row.
    pub op: &'static OpSpec,
    /// Every bulk sample, all columns.
    pub samples: Vec<CellSample>,
    /// The adversarial family points.
    pub overlay: Vec<OverlayPoint>,
}

/// The samplers, shared across cells (the tables are read-only).
pub struct Samplers {
    /// The version sampler, table built to the plan's span.
    pub version: VersionSampler,
    /// The party sampler, table built to the plan's span.
    pub party: PartySampler,
}

impl Samplers {
    /// Build both tables to the plan's span, concurrently.
    pub fn build(plan: &Plan) -> Samplers {
        Self::build_with_progress(plan, |_, _, _| {})
    }

    /// [`build`](Self::build), reporting `(table, entries done, entries
    /// total)` after every finished table entry.
    ///
    /// The two tables build concurrently, so calls for different tables
    /// interleave in scheduler order; each table's own call sequence is
    /// deterministic.
    pub fn build_with_progress(
        plan: &Plan,
        progress: impl Fn(&'static str, usize, usize) + Sync,
    ) -> Samplers {
        let (version, party) = rayon::join(
            || {
                VersionSampler::new_with_progress(plan.max_bytes, |done, total| {
                    progress("version", done, total)
                })
            },
            || {
                PartySampler::new_with_progress(plan.max_bytes, |done, total| {
                    progress("party", done, total)
                })
            },
        );
        Samplers { version, party }
    }
}

/// Split `total` uniformly over its compositions into `parts` positive
/// parts: draw `parts − 1` distinct cut points in `1..total`, sort, and
/// difference. Redrawing a collided cut is rejection over the cut-point
/// subsets, which keeps the composition draw exactly uniform.
///
/// For one part this draws nothing; for two it is a single
/// `gen_range(1..total)` — the binary split rule, unchanged.
fn split_budget(total: usize, parts: usize, rng: &mut rand_chacha::ChaCha12Rng) -> Vec<usize> {
    assert!(
        total >= parts && parts >= 1,
        "a column of {total} bytes cannot feed {parts} one-byte-minimum operands"
    );
    let mut cuts = std::collections::BTreeSet::new();
    while cuts.len() < parts - 1 {
        cuts.insert(rng.gen_range(1..total));
    }
    let mut out = Vec::with_capacity(parts);
    let mut prev = 0;
    for cut in cuts {
        out.push(cut - prev);
        prev = cut;
    }
    out.push(total - prev);
    out
}

/// Draw exact-size members for `operands` over a drawn split of `size`.
///
/// The split draw precedes the member draws so the stream is stable
/// however the samplers consume randomness. Returns the encodings
/// (operand order) and the version rejections spent.
fn draw_packed(
    operands: &[Operand],
    samplers: &Samplers,
    size: usize,
    rng: &mut rand_chacha::ChaCha12Rng,
) -> (Vec<Vec<u8>>, u64) {
    let sizes = split_budget(size, operands.len(), rng);
    let mut rejected = 0;
    let inputs = operands
        .iter()
        .zip(sizes)
        .map(|(operand, n)| match operand {
            Operand::Version => {
                let draw = samplers
                    .version
                    .sample_bytes(n, rng)
                    .expect("every byte size down to 1 has canonical versions");
                rejected += draw.rejected;
                draw.bytes
            }
            Operand::Party => {
                samplers
                    .party
                    .sample_bytes(n, rng)
                    .expect("every byte size down to 1 has canonical parties")
                    .bytes
            }
        })
        .collect();
    (inputs, rejected)
}

/// Draw one operation's packed inputs for a column of total size `size`.
///
/// Returns the encodings (operand order) and the rejection count spent.
fn draw_inputs(
    op: &OpSpec,
    samplers: &Samplers,
    size: usize,
    rng: &mut rand_chacha::ChaCha12Rng,
) -> (Vec<Vec<u8>>, u64) {
    match op.inputs {
        Inputs::Packed(operands) => draw_packed(operands, samplers, size, rng),
        Inputs::PackedDistinct(operands) => {
            // Whole-sample rejection of byte-identical pairs: restricting
            // the uniform pair measure to the distinct pairs, exactly.
            let mut rejected = 0;
            loop {
                let (inputs, r) = draw_packed(operands, samplers, size, rng);
                rejected += r;
                if inputs.iter().any(|i| *i != inputs[0]) {
                    return (inputs, rejected);
                }
                rejected += 1;
            }
        }
        Inputs::VersionSlice => {
            // Arity first, then the split, then the members, so the
            // stream is stable however the samplers consume randomness.
            let allowed: Vec<usize> = SLICE_ARITIES
                .iter()
                .copied()
                .filter(|&k| k <= size)
                .collect();
            assert!(
                !allowed.is_empty(),
                "slice columns start at the smallest declared arity"
            );
            let arity = allowed[rng.gen_range(0..allowed.len())];
            let sizes = split_budget(size, arity, rng);
            let mut rejected = 0;
            let inputs = sizes
                .into_iter()
                .map(|n| {
                    let draw = samplers
                        .version
                        .sample_bytes(n, rng)
                        .expect("every byte size down to 1 has canonical versions");
                    rejected += draw.rejected;
                    draw.bytes
                })
                .collect();
            (inputs, rejected)
        }
    }
}

/// Run one operation's whole atlas: every cell in parallel (each sample
/// instantiates a fresh guest, so no state leaks between measurements),
/// then the overlay points.
///
/// The cell list is flattened before the parallel iteration so rayon
/// splits across every (column, sample) pair, not merely across columns:
/// the geometric grid makes the largest column cost about as much as all
/// the others combined, so column-granular scheduling would idle every
/// worker but one for half of each operation's run.
pub fn run_op(plan: &Plan, samplers: &Samplers, op: &'static OpSpec) -> OpAtlas {
    let cells: Vec<(usize, usize)> = plan
        .columns(op.inputs.min_bytes())
        .into_iter()
        .flat_map(|size| (0..plan.samples_per_column).map(move |index| (size, index)))
        .collect();
    let samples: Vec<CellSample> = cells
        .into_par_iter()
        .map(|(size, index)| {
            let mut rng = cell_rng(plan.base_seed, op.name, size, index);
            let (inputs, rejected) = draw_inputs(op, samplers, size, &mut rng);
            let mut guest = Guest::new();
            let measured = (op.measure)(&mut guest, &inputs);
            assert!(
                measured.ret >= 0,
                "{}: guest kernel reported {} at size {size} sample {index}",
                op.name,
                measured.ret
            );
            CellSample {
                size,
                fuel: measured.fuel,
                rejected,
            }
        })
        .collect();

    let overlay = overlay_inputs(op, plan.max_bytes)
        .into_iter()
        .map(|fam| {
            let size = fam.inputs.iter().map(Vec::len).sum();
            let mut guest = Guest::new();
            let measured = (op.measure)(&mut guest, &fam.inputs);
            assert!(
                measured.ret >= 0,
                "{}: guest kernel reported {} on family {}",
                op.name,
                measured.ret,
                fam.family
            );
            OverlayPoint {
                family: fam.family,
                size,
                fuel: measured.fuel,
            }
        })
        .collect();

    OpAtlas {
        op,
        samples,
        overlay,
    }
}
