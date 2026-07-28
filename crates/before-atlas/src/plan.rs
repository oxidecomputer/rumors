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
//! **The size measure, declared once.** A unary operation's column at
//! size `N` draws its one input uniformly from the canonical inputs of
//! exactly `N` packed bytes. A binary operation's column at *total* size
//! `N` draws a split `n₁ ∈ {1, …, N−1}` uniformly, then each operand
//! uniformly at its exact size — so the x-axis is total packed input
//! bytes everywhere, and every rendered plot carries the declaration.

use rand::Rng;
use rayon::prelude::*;

use fuzzfit_harness::wasm::Guest;

use crate::families::overlay_inputs;
use crate::ops::{OpSpec, Operand};
use crate::sample::{cell_rng, PartySampler, VersionSampler};

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
    /// The size columns for an operand count: byte doublings from the
    /// smallest size the signature admits (binary needs 1 byte per side).
    pub fn columns(&self, operands: usize) -> Vec<usize> {
        let mut out = Vec::new();
        let mut n = operands.max(1);
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
    /// Whole-sample rejections spent drawing the inputs (version
    /// nonnegativity; zero for party-only rows).
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
    /// Build both tables to the plan's span.
    pub fn build(plan: &Plan) -> Samplers {
        Samplers {
            version: VersionSampler::new(plan.max_bytes),
            party: PartySampler::new(plan.max_bytes),
        }
    }
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
    // Split the total uniformly across two operands; a unary op takes it
    // whole. The split draw precedes the member draws so the stream is
    // stable however the samplers consume randomness.
    let sizes: Vec<usize> = match op.operands.len() {
        1 => vec![size],
        2 => {
            let n1 = rng.gen_range(1..size);
            vec![n1, size - n1]
        }
        n => panic!("no split rule for {n}-ary operations"),
    };
    let mut rejected = 0;
    let inputs = op
        .operands
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

/// Run one operation's whole atlas: every column in parallel across
/// samples (each sample instantiates a fresh guest, so no state leaks
/// between measurements), then the overlay points.
pub fn run_op(plan: &Plan, samplers: &Samplers, op: &'static OpSpec) -> OpAtlas {
    let samples: Vec<CellSample> = plan
        .columns(op.operands.len())
        .into_par_iter()
        .flat_map_iter(|size| (0..plan.samples_per_column).map(move |index| (size, index)))
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

    let overlay = overlay_inputs(op.operands, plan.max_bytes)
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
