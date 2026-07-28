//! The operation roster: which public operations the atlas measures, and
//! how each runs in the fuel guest.
//!
//! One table row per operation — adding an operation is one [`OpSpec`]
//! entry. Each row names its operand types (which pick the samplers and
//! the size measure) and a `measure` function that stages the sampled
//! packed inputs into the guest and runs exactly one measured kernel,
//! returning that call's fuel. Register loading and staging happen before
//! the measured call, so a reading prices one public operation (plus the
//! guest's constant dispatch overhead, identical for every sample).
//!
//! The causal comparison is `PartialOrd` (`ff_version_cmp`) — the crate
//! exposes no separate comparison entry point — with `concurrent` as its
//! own row since it is a distinct public operation.

use fuzzfit_harness::wasm::{Guest, Measured};

/// The packed input type an operand position takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operand {
    /// A canonical packed `Version`.
    Version,
    /// A canonical packed `Party`.
    Party,
}

/// One measured operation: a roster row.
pub struct OpSpec {
    /// The atlas name (also the output file stem).
    pub name: &'static str,
    /// Operand types, in kernel argument order. One operand samples the
    /// whole column size; two split it (the plan documents the split).
    pub operands: &'static [Operand],
    /// Stage `inputs` (one packed encoding per operand) and run the one
    /// measured kernel, returning its fuel.
    pub measure: fn(&mut Guest, &[Vec<u8>]) -> Measured,
}

/// Stage packed bytes and decode them into a version register
/// (unmeasured preparation; the decode's own fuel is discarded).
fn load_version(guest: &mut Guest, reg: u32, bytes: &[u8]) {
    guest.stage_write(bytes);
    let r = guest.call("ff_version_decode", &[reg]);
    assert_eq!(r.ret, 0, "prep: guest rejected a sampled version");
}

/// Stage packed bytes and decode them into a party register (unmeasured).
fn load_party(guest: &mut Guest, reg: u32, bytes: &[u8]) {
    guest.stage_write(bytes);
    let r = guest.call("ff_party_decode", &[reg]);
    assert_eq!(r.ret, 0, "prep: guest rejected a sampled party");
}

/// The measured operations. Every kernel's return value is nonnegative on
/// success (comparison kernels encode their verdict as 0..=3); the plan
/// asserts that, so a misuse can never be read as a fuel value.
pub const ROSTER: &[OpSpec] = &[
    OpSpec {
        name: "version_decode",
        operands: &[Operand::Version],
        measure: |g, inputs| {
            g.stage_write(&inputs[0]);
            g.call("ff_version_decode", &[0])
        },
    },
    OpSpec {
        name: "version_encode",
        operands: &[Operand::Version],
        measure: |g, inputs| {
            load_version(g, 0, &inputs[0]);
            g.call("ff_version_encode", &[0])
        },
    },
    OpSpec {
        name: "version_rank",
        operands: &[Operand::Version],
        measure: |g, inputs| {
            load_version(g, 0, &inputs[0]);
            g.call("ff_version_rank", &[1, 0])
        },
    },
    OpSpec {
        name: "version_min_ticks",
        operands: &[Operand::Version],
        measure: |g, inputs| {
            load_version(g, 0, &inputs[0]);
            g.call_i64("ff_version_min_ticks", &[0])
        },
    },
    OpSpec {
        name: "version_tick",
        operands: &[Operand::Version, Operand::Party],
        measure: |g, inputs| {
            load_version(g, 0, &inputs[0]);
            load_party(g, 1, &inputs[1]);
            g.call("ff_version_tick", &[0, 1])
        },
    },
    OpSpec {
        name: "version_cmp",
        operands: &[Operand::Version, Operand::Version],
        measure: |g, inputs| {
            load_version(g, 0, &inputs[0]);
            load_version(g, 1, &inputs[1]);
            g.call("ff_version_cmp", &[0, 1])
        },
    },
    OpSpec {
        name: "version_concurrent",
        operands: &[Operand::Version, Operand::Version],
        measure: |g, inputs| {
            load_version(g, 0, &inputs[0]);
            load_version(g, 1, &inputs[1]);
            g.call("ff_version_concurrent", &[0, 1])
        },
    },
    OpSpec {
        name: "version_join",
        operands: &[Operand::Version, Operand::Version],
        measure: |g, inputs| {
            load_version(g, 0, &inputs[0]);
            load_version(g, 1, &inputs[1]);
            g.call("ff_version_join", &[2, 0, 1])
        },
    },
    OpSpec {
        name: "version_meet",
        operands: &[Operand::Version, Operand::Version],
        measure: |g, inputs| {
            load_version(g, 0, &inputs[0]);
            load_version(g, 1, &inputs[1]);
            g.call("ff_version_meet", &[2, 0, 1])
        },
    },
    OpSpec {
        name: "version_distance",
        operands: &[Operand::Version, Operand::Version],
        measure: |g, inputs| {
            load_version(g, 0, &inputs[0]);
            load_version(g, 1, &inputs[1]);
            g.call("ff_version_distance", &[2, 0, 1])
        },
    },
    OpSpec {
        name: "version_lag",
        operands: &[Operand::Version, Operand::Version],
        measure: |g, inputs| {
            load_version(g, 0, &inputs[0]);
            load_version(g, 1, &inputs[1]);
            g.call("ff_version_lag", &[2, 0, 1])
        },
    },
    OpSpec {
        name: "party_decode",
        operands: &[Operand::Party],
        measure: |g, inputs| {
            g.stage_write(&inputs[0]);
            g.call("ff_party_decode", &[0])
        },
    },
    OpSpec {
        name: "party_encode",
        operands: &[Operand::Party],
        measure: |g, inputs| {
            load_party(g, 0, &inputs[0]);
            g.call("ff_party_encode", &[0])
        },
    },
    OpSpec {
        name: "party_is_disjoint",
        operands: &[Operand::Party, Operand::Party],
        measure: |g, inputs| {
            load_party(g, 0, &inputs[0]);
            load_party(g, 1, &inputs[1]);
            g.call("ff_party_is_disjoint", &[0, 1])
        },
    },
    OpSpec {
        name: "party_covers",
        operands: &[Operand::Party, Operand::Party],
        measure: |g, inputs| {
            load_party(g, 0, &inputs[0]);
            load_party(g, 1, &inputs[1]);
            g.call("ff_party_covers", &[0, 1])
        },
    },
];
