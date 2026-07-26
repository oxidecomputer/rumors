//! The driver: replays one program in the mirror and the guest in lockstep,
//! collecting one fuel sample per measured step.
//!
//! Each step runs natively first (denominator, expected return), then in the
//! guest (fuel). Any disagreement — return code, or the end-of-program
//! byte-compare of every live register — panics: with a deterministic guest
//! the disagreement is a harness bug or a genuine wasm-vs-native divergence,
//! and both must stop the run loudly.

use crate::ops::{Malformed, Mirror, Op};
use crate::wasm::Guest;

/// One measured step: the band key, the denominated size, and the fuel.
#[derive(Debug, Clone, Copy)]
pub struct Sample {
    /// The kernel name (the calibration's and enforcement's band key).
    pub kernel: &'static str,
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
        samples.push(Sample {
            kernel: op.kernel(),
            denom_bits: step.denom_bits,
            fuel: measured.fuel,
        });
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
