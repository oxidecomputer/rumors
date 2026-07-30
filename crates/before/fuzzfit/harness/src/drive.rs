//! The driver: replays one program in the mirror and the guest in lockstep,
//! collecting one fuel sample per measured step.
//!
//! Each step runs natively first (denominator, expected return), then in the
//! guest (fuel). Any disagreement — return code, or the end-of-program
//! byte-compare of every live register — panics: with a deterministic guest
//! the disagreement is a harness bug or a genuine wasm-vs-native divergence,
//! and both must stop the run loudly.

use proptest::strategy::{Strategy, ValueTree};
use proptest::test_runner::TestRunner;

use crate::ops::{Malformed, Mirror, Op};
use crate::strategies::{any_family, build, Family};
use crate::wasm::Guest;

/// One measured step: the band key (kernel × outcome), the denominated
/// size, and the fuel.
#[derive(Debug, Clone, Copy)]
pub struct Sample {
    /// The kernel name (with `rejected`, the band key).
    pub kernel: &'static str,
    /// Whether the step's outcome is an operation rejection (the mirror's
    /// prediction, asserted against the guest): rejection arms are priced
    /// as their own bands.
    pub rejected: bool,
    /// The step's denominated size in bits.
    pub denom_bits: u64,
    /// Fuel the guest consumed on this step.
    pub fuel: u64,
}

/// Replay `program` natively and in a fresh guest; return per-step samples.
///
/// # Panics
///
/// Panics when the guest's return code or any live register's final bytes
/// disagree with the mirror (a differential failure), or when a kernel
/// traps. Returns `Err` only for malformed programs (generator bugs).
pub fn run_program(program: &[Op]) -> Result<Vec<Sample>, Malformed> {
    let mut mirror = Mirror::new();
    let mut guest = Guest::new();
    let mut samples = Vec::with_capacity(program.len());
    for op in program {
        let step = mirror.step(op)?;
        let args = op.args();
        let measured = if op.returns_i64() {
            guest.call_i64(op.kernel(), &args)
        } else {
            guest.call(op.kernel(), &args)
        };
        assert_eq!(
            measured.ret, step.expect,
            "guest/native disagreement on {op:?}: guest returned {}, mirror expected {}",
            measured.ret, step.expect
        );
        // Identity-outcome steps (operands dispatching an identity-law
        // fast path: one clone-shared buffer under a comparison, equal
        // versions under a metric) are measured for the differential but
        // never sampled. Their cost is O(1) by mechanism, not a size
        // law, and fitting them alongside the walked cloud makes both
        // bands decoration-wide; their liveness has its own instrument
        // (before's `identity_fast_paths` meter pins assert exactly zero
        // walk work on the fast paths, beside walking legs), so a lost
        // fast path reads red there while a walk regression reads red
        // here, on the samples that walk.
        if !step.identity() {
            samples.push(Sample {
                kernel: op.kernel(),
                rejected: step.rejected(),
                denom_bits: step.denom_bits,
                fuel: measured.fuel,
            });
        }
    }
    // The end-of-program differential: every live register must byte-match
    // between the two executions (the snapshot calls are unrecorded, so they
    // never pollute the samples).
    for (reg, tag) in mirror.live_regs() {
        let expected = mirror
            .snapshot(reg)
            .expect("live_regs only reports live slots");
        let kernel = match tag {
            b'v' => "ff_version_encode",
            b'p' => "ff_party_encode",
            b'c' => "ff_clock_encode",
            b'r' => "ff_rank_display",
            _ => unreachable!("mirror tags are v/p/c/r"),
        };
        let ret = guest.call(kernel, &[reg]).ret;
        assert_eq!(ret, 0, "guest snapshot of r{reg} ({kernel}) failed: {ret}");
        let got = guest.stage_read();
        assert_eq!(
            got, expected,
            "differential failure: guest r{reg} bytes diverge from the native mirror"
        );
    }
    Ok(samples)
}

/// Drive the deterministic corpus, visiting each program's samples.
///
/// The family stream comes from proptest's deterministic runner and each
/// program's seed is its case index, so any two consumers observe
/// byte-identical samples for the same `programs` count. The calibration
/// sweep pins bands from this stream; the enforcement suite's staleness
/// cross-check refits a prefix of it, and the two agree exactly because
/// they are the same stream.
///
/// # Panics
///
/// Propagates [`run_program`]'s differential panics; a malformed program
/// (a generator bug) also panics, since the deterministic stream is the
/// corpus of record and must be entirely runnable.
pub fn for_each_deterministic_program(
    programs: usize,
    mut visit: impl FnMut(usize, &Family, &[Sample]),
) {
    let mut runner = TestRunner::deterministic();
    let strategy = any_family();
    for case in 0..programs {
        let family = strategy
            .new_tree(&mut runner)
            .expect("family strategy cannot fail")
            .current();
        let program = build(&family, case as u64);
        let samples = run_program(&program)
            .unwrap_or_else(|m| panic!("malformed program from {family:?}: {}", m.op));
        visit(case, &family, &samples);
    }
}
