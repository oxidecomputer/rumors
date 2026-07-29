//! The operation roster: which public operations the atlas measures, how
//! each row's inputs are drawn, and how each runs in the fuel guest.
//!
//! One table row per operation — adding an operation is one [`OpSpec`]
//! entry. Each row names its input space (which picks the samplers and
//! the size measure — the row's `size_measure` string is stamped on its
//! render), the triangle roster rows it covers, and a `measure` function
//! that stages the sampled packed inputs into the guest and runs exactly
//! one measured kernel, returning that call's fuel. Register loading and
//! all other staging happen before the measured call, so a reading prices
//! one public operation (plus the guest's constant dispatch overhead,
//! identical for every sample).
//!
//! # Totality over the public surface
//!
//! Coverage is bound to the triangle suite's committed roster
//! (`before::surface`): every roster row is either claimed by a panel's
//! `covers` list or carries a one-line reason in [`EXEMPTIONS`], and the
//! parity test in `tests.rs` holds both directions mechanically — a new
//! public operation cannot ship without a panel or a reviewed exemption,
//! and a renamed one fails by name. The exemption reasons are the
//! reviewed artifact; membership is enforced, never remembered.
//!
//! The causal comparison is `PartialOrd` (`ff_version_cmp`) — the crate
//! exposes no separate comparison entry point — with `concurrent` as its
//! own row since it is a distinct public operation. Clock rows compose
//! their operand from a sampled party and version (`Clock::from_parts`
//! in unmeasured preparation); a clock's canonical encoding is exactly
//! its party's bytes followed by its version's, so the constituents'
//! total packed size is the clock's own packed size.

use fuzzfit_harness::wasm::{Guest, Measured};

#[cfg(test)]
mod tests;

/// The packed input type an operand position takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operand {
    /// A canonical packed `Version`.
    Version,
    /// A canonical packed `Party`.
    Party,
}

/// How one row's inputs are drawn at a column's total size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inputs {
    /// A fixed operand list; the column's total packed size splits
    /// uniformly over the compositions into one exact size per operand.
    Packed(&'static [Operand]),
    /// [`Packed`](Inputs::Packed), with byte-identical operand pairs
    /// rejected at draw time (whole-sample rejection, so the measure is
    /// exactly uniform on the distinct pairs).
    PackedDistinct(&'static [Operand]),
    /// A version slice: arity drawn uniformly from [`SLICE_ARITIES`]
    /// capped by the column size, then the total split uniformly over
    /// the compositions into one exact size per operand.
    VersionSlice,
}

impl Inputs {
    /// The smallest total size (bytes) a column of this input space can
    /// have: one byte per operand.
    pub fn min_bytes(&self) -> usize {
        match self {
            Inputs::Packed(operands) | Inputs::PackedDistinct(operands) => operands.len().max(1),
            // The smallest slice arity.
            Inputs::VersionSlice => 2,
        }
    }
}

/// The arity set a slice row draws from (uniformly, capped by the
/// column's total size: arity `k` needs `k` one-byte operands).
pub const SLICE_ARITIES: &[usize] = &[2, 4, 8, 16];

/// The share count of the `party_forks` row: the size axis is the
/// party's packed bytes, the arity a declared constant.
const FORKS_SHARES: u32 = 8;

/// One measured operation: a roster row.
pub struct OpSpec {
    /// The atlas name (also the output file stem).
    pub name: &'static str,
    /// The input space the row samples.
    pub inputs: Inputs,
    /// The triangle roster rows (`before::surface` op names) this panel
    /// prices; the parity test holds panels ∪ exemptions total over the
    /// roster.
    pub covers: &'static [&'static str],
    /// The declared size measure, stamped verbatim on the render.
    pub size_measure: &'static str,
    /// Stage `inputs` (one packed encoding per operand) and run the one
    /// measured kernel, returning its fuel.
    pub measure: fn(&mut Guest, &[Vec<u8>]) -> Measured,
}

/// The size measure of a one-operand packed row.
const M_UNARY: &str = "exact packed bytes, uniform per size";
/// The size measure of a two-operand packed row.
const M_BINARY: &str = "total packed bytes; split uniform across the two operands";
/// The size measure of a slice row.
const M_SLICE: &str =
    "total packed bytes; arity uniform over {2, 4, 8, 16} capped by size, split uniform";
/// The size measure of a composed-clock unary row.
const M_CLOCK: &str = "total packed bytes of the clock's party and version parts, split uniform";
/// The size measure of the fork-split disjoint-clock rows.
const M_CLOCK_PAIR: &str = "total packed bytes of one party and two versions, split uniform \
     three ways; the party fork-split into the clocks' disjoint parties";

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

/// Run an unmeasured preparation kernel that must succeed (its fuel is
/// discarded; a failure is a roster bug, never a measurement).
fn prep(guest: &mut Guest, kernel: &str, args: &[u32]) {
    let r = guest.call(kernel, args);
    assert_eq!(r.ret, 0, "prep: {kernel} reported {}", r.ret);
}

/// Compose a clock into register 0 from packed party and version
/// encodings (registers 1 and 2 hold the consumed parts; unmeasured).
fn compose_clock(guest: &mut Guest, party: &[u8], version: &[u8]) {
    load_party(guest, 1, party);
    load_version(guest, 2, version);
    prep(guest, "ff_clock_from_parts", &[0, 1, 2]);
}

/// Compose two disjoint-party clocks into registers 4 and 5: one party
/// (register 0) fork-split in the guest, one version per side
/// (unmeasured preparation for the join/sync rows).
fn compose_clock_pair(guest: &mut Guest, inputs: &[Vec<u8>]) {
    load_party(guest, 0, &inputs[0]);
    prep(guest, "ff_party_fork", &[1, 0]);
    load_version(guest, 2, &inputs[1]);
    load_version(guest, 3, &inputs[2]);
    prep(guest, "ff_clock_from_parts", &[4, 0, 2]);
    prep(guest, "ff_clock_from_parts", &[5, 1, 3]);
}

/// Load a slice row's versions into registers `0..k` (unmeasured).
fn load_slice(guest: &mut Guest, inputs: &[Vec<u8>]) -> u32 {
    for (i, v) in inputs.iter().enumerate() {
        load_version(guest, i as u32, v);
    }
    inputs.len() as u32
}

/// The measured operations. Every kernel's return value is nonnegative on
/// success (comparison kernels encode their verdict as 0..=3); the plan
/// asserts that, so a misuse can never be read as a fuel value.
pub const ROSTER: &[OpSpec] = &[
    // ───────────────────────────── Version ─────────────────────────────
    OpSpec {
        name: "version_decode",
        inputs: Inputs::Packed(&[Operand::Version]),
        covers: &["Version::decode"],
        size_measure: M_UNARY,
        measure: |g, inputs| {
            g.stage_write(&inputs[0]);
            g.call("ff_version_decode", &[0])
        },
    },
    OpSpec {
        name: "version_encode",
        inputs: Inputs::Packed(&[Operand::Version]),
        covers: &["Version::encode"],
        size_measure: M_UNARY,
        measure: |g, inputs| {
            load_version(g, 0, &inputs[0]);
            g.call("ff_version_encode", &[0])
        },
    },
    OpSpec {
        name: "version_display",
        inputs: Inputs::Packed(&[Operand::Version]),
        covers: &["Version Display / FromStr / TryFrom literals"],
        size_measure: "exact packed bytes, uniform per size (fuel includes writing the \
             text output)",
        measure: |g, inputs| {
            load_version(g, 0, &inputs[0]);
            g.call("ff_version_display", &[0])
        },
    },
    OpSpec {
        name: "version_fromstr",
        inputs: Inputs::Packed(&[Operand::Version]),
        covers: &["Version Display / FromStr / TryFrom literals"],
        size_measure: "packed bytes of the sampled value, rendered to text by Display \
             (the value measure pushed through rendering — not uniform over text; the \
             adversarial text families keep the corner coverage)",
        measure: |g, inputs| {
            load_version(g, 0, &inputs[0]);
            prep(g, "ff_version_display", &[0]);
            g.call("ff_version_fromstr", &[1])
        },
    },
    OpSpec {
        name: "version_rank",
        inputs: Inputs::Packed(&[Operand::Version]),
        covers: &["Version::rank"],
        size_measure: M_UNARY,
        measure: |g, inputs| {
            load_version(g, 0, &inputs[0]);
            g.call("ff_version_rank", &[1, 0])
        },
    },
    OpSpec {
        name: "version_min_ticks",
        inputs: Inputs::Packed(&[Operand::Version]),
        covers: &["Version::min_ticks"],
        size_measure: M_UNARY,
        measure: |g, inputs| {
            load_version(g, 0, &inputs[0]);
            g.call_i64("ff_version_min_ticks", &[0])
        },
    },
    OpSpec {
        name: "version_tick",
        inputs: Inputs::Packed(&[Operand::Version, Operand::Party]),
        covers: &["Version::tick"],
        size_measure: M_BINARY,
        measure: |g, inputs| {
            load_version(g, 0, &inputs[0]);
            load_party(g, 1, &inputs[1]);
            g.call("ff_version_tick", &[0, 1])
        },
    },
    OpSpec {
        name: "version_project",
        inputs: Inputs::Packed(&[Operand::Version, Operand::Party]),
        covers: &[
            "&Version / &Party (Div — the lazy projection view)",
            "OwnVersion::to_version",
            "From<OwnVersion> for Version (explicit materialization)",
        ],
        size_measure: "total packed bytes; split uniform across the two operands (the \
             projection view materialized via to_version)",
        measure: |g, inputs| {
            load_version(g, 0, &inputs[0]);
            load_party(g, 1, &inputs[1]);
            g.call("ff_version_project", &[2, 0, 1])
        },
    },
    OpSpec {
        name: "version_cmp",
        inputs: Inputs::Packed(&[Operand::Version, Operand::Version]),
        covers: &["Version PartialOrd (the comparison matrix, owned and borrowed)"],
        size_measure: M_BINARY,
        measure: |g, inputs| {
            load_version(g, 0, &inputs[0]);
            load_version(g, 1, &inputs[1]);
            g.call("ff_version_cmp", &[0, 1])
        },
    },
    OpSpec {
        name: "version_concurrent",
        inputs: Inputs::Packed(&[Operand::Version, Operand::Version]),
        covers: &["Version::concurrent"],
        size_measure: M_BINARY,
        measure: |g, inputs| {
            load_version(g, 0, &inputs[0]);
            load_version(g, 1, &inputs[1]);
            g.call("ff_version_concurrent", &[0, 1])
        },
    },
    OpSpec {
        name: "version_join",
        inputs: Inputs::Packed(&[Operand::Version, Operand::Version]),
        covers: &["Version | Version (BitOr/BitOrAssign, owned and borrowed)"],
        size_measure: M_BINARY,
        measure: |g, inputs| {
            load_version(g, 0, &inputs[0]);
            load_version(g, 1, &inputs[1]);
            g.call("ff_version_join", &[2, 0, 1])
        },
    },
    OpSpec {
        name: "version_meet",
        inputs: Inputs::Packed(&[Operand::Version, Operand::Version]),
        covers: &["Version & Version (BitAnd/BitAndAssign, owned and borrowed)"],
        size_measure: M_BINARY,
        measure: |g, inputs| {
            load_version(g, 0, &inputs[0]);
            load_version(g, 1, &inputs[1]);
            g.call("ff_version_meet", &[2, 0, 1])
        },
    },
    OpSpec {
        name: "version_distance",
        inputs: Inputs::Packed(&[Operand::Version, Operand::Version]),
        covers: &["Version::distance"],
        size_measure: M_BINARY,
        measure: |g, inputs| {
            load_version(g, 0, &inputs[0]);
            load_version(g, 1, &inputs[1]);
            g.call("ff_version_distance", &[2, 0, 1])
        },
    },
    OpSpec {
        name: "version_lag",
        inputs: Inputs::Packed(&[Operand::Version, Operand::Version]),
        covers: &["Version::lag"],
        size_measure: M_BINARY,
        measure: |g, inputs| {
            load_version(g, 0, &inputs[0]);
            load_version(g, 1, &inputs[1]);
            g.call("ff_version_lag", &[2, 0, 1])
        },
    },
    OpSpec {
        name: "version_join_all",
        inputs: Inputs::VersionSlice,
        covers: &["Version::join_all"],
        size_measure: M_SLICE,
        measure: |g, inputs| {
            let n = load_slice(g, inputs);
            g.call("ff_version_join_all", &[n, 0, n])
        },
    },
    OpSpec {
        name: "version_meet_all",
        inputs: Inputs::VersionSlice,
        covers: &["Version::meet_all"],
        size_measure: M_SLICE,
        measure: |g, inputs| {
            let n = load_slice(g, inputs);
            g.call("ff_version_meet_all", &[n, 0, n])
        },
    },
    // ───────────────────────────── Party ─────────────────────────────
    OpSpec {
        name: "party_decode",
        inputs: Inputs::Packed(&[Operand::Party]),
        covers: &["Party::decode"],
        size_measure: M_UNARY,
        measure: |g, inputs| {
            g.stage_write(&inputs[0]);
            g.call("ff_party_decode", &[0])
        },
    },
    OpSpec {
        name: "party_encode",
        inputs: Inputs::Packed(&[Operand::Party]),
        covers: &["Party::encode"],
        size_measure: M_UNARY,
        measure: |g, inputs| {
            load_party(g, 0, &inputs[0]);
            g.call("ff_party_encode", &[0])
        },
    },
    OpSpec {
        name: "party_display",
        inputs: Inputs::Packed(&[Operand::Party]),
        covers: &["Party Display / FromStr / TryFrom literals"],
        size_measure: "exact packed bytes, uniform per size (fuel includes writing the \
             text output)",
        measure: |g, inputs| {
            load_party(g, 0, &inputs[0]);
            g.call("ff_party_display", &[0])
        },
    },
    OpSpec {
        name: "party_fromstr",
        inputs: Inputs::Packed(&[Operand::Party]),
        covers: &["Party Display / FromStr / TryFrom literals"],
        size_measure: "packed bytes of the sampled value, rendered to text by Display \
             (the value measure pushed through rendering — not uniform over text; the \
             adversarial text families keep the corner coverage)",
        measure: |g, inputs| {
            load_party(g, 0, &inputs[0]);
            prep(g, "ff_party_display", &[0]);
            g.call("ff_party_fromstr", &[1])
        },
    },
    OpSpec {
        name: "party_fork",
        inputs: Inputs::Packed(&[Operand::Party]),
        covers: &["Party::fork"],
        size_measure: M_UNARY,
        measure: |g, inputs| {
            load_party(g, 0, &inputs[0]);
            g.call("ff_party_fork", &[1, 0])
        },
    },
    OpSpec {
        name: "party_forks",
        inputs: Inputs::Packed(&[Operand::Party]),
        covers: &["Party::forks"],
        size_measure: "exact packed bytes, uniform per size (share count a declared \
             constant, 8)",
        measure: |g, inputs| {
            load_party(g, 0, &inputs[0]);
            g.call("ff_party_forks", &[1, 0, FORKS_SHARES])
        },
    },
    OpSpec {
        name: "party_join",
        inputs: Inputs::Packed(&[Operand::Party]),
        covers: &["Party::join"],
        size_measure: "packed bytes of one uniform party; the operands are its two \
             fork halves (independent uniform pairs are almost never disjoint, so the \
             partial domain is reached by re-merging a split)",
        measure: |g, inputs| {
            load_party(g, 0, &inputs[0]);
            prep(g, "ff_party_fork", &[1, 0]);
            g.call("ff_party_join", &[0, 1])
        },
    },
    OpSpec {
        name: "party_is_disjoint",
        inputs: Inputs::Packed(&[Operand::Party, Operand::Party]),
        covers: &["Party::is_disjoint"],
        size_measure: M_BINARY,
        measure: |g, inputs| {
            load_party(g, 0, &inputs[0]);
            load_party(g, 1, &inputs[1]);
            g.call("ff_party_is_disjoint", &[0, 1])
        },
    },
    OpSpec {
        name: "party_covers",
        inputs: Inputs::Packed(&[Operand::Party, Operand::Party]),
        covers: &["Party::covers"],
        size_measure: M_BINARY,
        measure: |g, inputs| {
            load_party(g, 0, &inputs[0]);
            load_party(g, 1, &inputs[1]);
            g.call("ff_party_covers", &[0, 1])
        },
    },
    OpSpec {
        name: "party_without",
        inputs: Inputs::PackedDistinct(&[Operand::Party, Operand::Party]),
        covers: &["Party::without"],
        size_measure: "total packed bytes; split uniform; byte-equal pairs rejected \
             and operand order chosen so the difference exists",
        measure: |g, inputs| {
            load_party(g, 0, &inputs[0]);
            load_party(g, 1, &inputs[1]);
            // Unmeasured verdict: when the second operand covers the
            // first, the difference in that order is empty — swap so it
            // exists. Byte-equal pairs (both orders empty) were rejected
            // at draw time, so the swapped order is never itself empty.
            let covers = g.call("ff_party_covers", &[1, 0]);
            assert!(
                covers.ret >= 0,
                "prep: ff_party_covers reported {}",
                covers.ret
            );
            if covers.ret == 1 {
                g.call("ff_party_without", &[2, 1, 0])
            } else {
                g.call("ff_party_without", &[2, 0, 1])
            }
        },
    },
    // ───────────────────────────── Clock ─────────────────────────────
    OpSpec {
        name: "clock_decode",
        inputs: Inputs::Packed(&[Operand::Party, Operand::Version]),
        covers: &["Clock::decode"],
        size_measure: M_CLOCK,
        measure: |g, inputs| {
            compose_clock(g, &inputs[0], &inputs[1]);
            // The unmeasured encode leaves the clock's canonical bytes in
            // the staging buffer for the measured decode to read.
            prep(g, "ff_clock_encode", &[0]);
            g.call("ff_clock_decode", &[3])
        },
    },
    OpSpec {
        name: "clock_encode",
        inputs: Inputs::Packed(&[Operand::Party, Operand::Version]),
        covers: &["Clock::encode"],
        size_measure: M_CLOCK,
        measure: |g, inputs| {
            compose_clock(g, &inputs[0], &inputs[1]);
            g.call("ff_clock_encode", &[0])
        },
    },
    OpSpec {
        name: "clock_tick",
        inputs: Inputs::Packed(&[Operand::Party, Operand::Version]),
        covers: &["Clock::tick"],
        size_measure: M_CLOCK,
        measure: |g, inputs| {
            compose_clock(g, &inputs[0], &inputs[1]);
            g.call("ff_clock_tick", &[0])
        },
    },
    OpSpec {
        name: "clock_fork",
        inputs: Inputs::Packed(&[Operand::Party, Operand::Version]),
        covers: &["Clock::fork"],
        size_measure: M_CLOCK,
        measure: |g, inputs| {
            compose_clock(g, &inputs[0], &inputs[1]);
            g.call("ff_clock_fork", &[3, 0])
        },
    },
    OpSpec {
        name: "clock_send",
        inputs: Inputs::Packed(&[Operand::Party, Operand::Version]),
        covers: &["Clock::send"],
        size_measure: M_CLOCK,
        measure: |g, inputs| {
            compose_clock(g, &inputs[0], &inputs[1]);
            g.call("ff_clock_send", &[0])
        },
    },
    OpSpec {
        name: "clock_recv",
        inputs: Inputs::Packed(&[Operand::Party, Operand::Version, Operand::Version]),
        covers: &["Clock::recv"],
        size_measure: "total packed bytes of the clock's parts and the received \
             version, split uniform three ways",
        measure: |g, inputs| {
            compose_clock(g, &inputs[0], &inputs[1]);
            load_version(g, 3, &inputs[2]);
            g.call("ff_clock_recv", &[0, 3])
        },
    },
    OpSpec {
        name: "clock_join",
        inputs: Inputs::Packed(&[Operand::Party, Operand::Version, Operand::Version]),
        covers: &["Clock::join"],
        size_measure: M_CLOCK_PAIR,
        measure: |g, inputs| {
            compose_clock_pair(g, inputs);
            g.call("ff_clock_join", &[4, 5])
        },
    },
    OpSpec {
        name: "clock_sync",
        inputs: Inputs::Packed(&[Operand::Party, Operand::Version, Operand::Version]),
        covers: &["Clock::sync"],
        size_measure: M_CLOCK_PAIR,
        measure: |g, inputs| {
            compose_clock_pair(g, inputs);
            g.call("ff_clock_sync", &[4, 5])
        },
    },
    OpSpec {
        name: "clock_from_parts",
        inputs: Inputs::Packed(&[Operand::Party, Operand::Version]),
        covers: &["Clock::from_parts"],
        size_measure: M_CLOCK,
        measure: |g, inputs| {
            load_party(g, 0, &inputs[0]);
            load_version(g, 1, &inputs[1]);
            g.call("ff_clock_from_parts", &[2, 0, 1])
        },
    },
    OpSpec {
        name: "clock_into_parts",
        inputs: Inputs::Packed(&[Operand::Party, Operand::Version]),
        covers: &["Clock::into_parts"],
        size_measure: M_CLOCK,
        measure: |g, inputs| {
            compose_clock(g, &inputs[0], &inputs[1]);
            g.call("ff_clock_into_parts", &[3, 4, 0])
        },
    },
    OpSpec {
        name: "clock_own_version",
        inputs: Inputs::Packed(&[Operand::Party, Operand::Version]),
        covers: &["Clock::own_version"],
        size_measure: "total packed bytes of the clock's party and version parts, \
             split uniform (the view materialized via to_version)",
        measure: |g, inputs| {
            compose_clock(g, &inputs[0], &inputs[1]);
            g.call("ff_clock_own_version", &[3, 0])
        },
    },
    // ───────────────────────────── Rank ─────────────────────────────
    OpSpec {
        name: "rank_add",
        inputs: Inputs::Packed(&[Operand::Version, Operand::Version]),
        covers: &["Rank ZERO / Add / AddAssign / Sum / Ord / Eq / Hash / Display"],
        size_measure: "total packed bytes of the two versions whose ranks are added, \
             split uniform (ranks derived by Version::rank in preparation)",
        measure: |g, inputs| {
            load_version(g, 0, &inputs[0]);
            load_version(g, 1, &inputs[1]);
            prep(g, "ff_version_rank", &[2, 0]);
            prep(g, "ff_version_rank", &[3, 1]);
            g.call("ff_rank_add", &[4, 2, 3])
        },
    },
    OpSpec {
        name: "rank_cmp",
        inputs: Inputs::Packed(&[Operand::Version, Operand::Version]),
        covers: &["Rank ZERO / Add / AddAssign / Sum / Ord / Eq / Hash / Display"],
        size_measure: "total packed bytes of the two versions whose ranks are compared, \
             split uniform (ranks derived by Version::rank in preparation)",
        measure: |g, inputs| {
            load_version(g, 0, &inputs[0]);
            load_version(g, 1, &inputs[1]);
            prep(g, "ff_version_rank", &[2, 0]);
            prep(g, "ff_version_rank", &[3, 1]);
            g.call("ff_rank_cmp", &[2, 3])
        },
    },
    OpSpec {
        name: "rank_checked_sub",
        inputs: Inputs::Packed(&[Operand::Version, Operand::Version]),
        covers: &["Rank::checked_sub"],
        size_measure: "total packed bytes of the two versions whose ranks are \
             subtracted, split uniform; operands ordered by rank so the difference \
             exists",
        measure: |g, inputs| {
            load_version(g, 0, &inputs[0]);
            load_version(g, 1, &inputs[1]);
            prep(g, "ff_version_rank", &[2, 0]);
            prep(g, "ff_version_rank", &[3, 1]);
            // Unmeasured verdict (0 Less, 1 Equal, 2 Greater): subtract
            // the smaller rank from the larger so the difference exists.
            let cmp = g.call("ff_rank_cmp", &[2, 3]);
            assert!(cmp.ret >= 0, "prep: ff_rank_cmp reported {}", cmp.ret);
            if cmp.ret == 0 {
                g.call("ff_rank_checked_sub", &[4, 3, 2])
            } else {
                g.call("ff_rank_checked_sub", &[4, 2, 3])
            }
        },
    },
    OpSpec {
        name: "rank_display",
        inputs: Inputs::Packed(&[Operand::Version]),
        covers: &["Rank ZERO / Add / AddAssign / Sum / Ord / Eq / Hash / Display"],
        size_measure: "packed bytes of the version whose rank is rendered (rank \
             derived by Version::rank in preparation; fuel includes writing the text \
             output)",
        measure: |g, inputs| {
            load_version(g, 0, &inputs[0]);
            prep(g, "ff_version_rank", &[1, 0]);
            g.call("ff_rank_display", &[1])
        },
    },
];

/// The triangle roster rows deliberately without a panel, each with its
/// one-line reason.
///
/// The reviewed half of the totality binding: the parity test in
/// `tests.rs` enforces membership both ways, so these reasons are for
/// the owner's eyes.
///
/// Three recurring genres: constant-input constructors have no size axis
/// to plot; O(1)-scale accessors and delegating wrappers are priced at
/// the operation they wrap; and operations without a guest kernel cannot
/// be measured in the atlas's fuel currency (adding kernels is a
/// `crates/before` change outside this crate).
pub const EXEMPTIONS: &[(&str, &str)] = &[
    // ── constructors without a size axis ──
    ("Party::seed", "constant-input constructor: no size axis to plot"),
    ("Version::new", "constant-input constructor: no size axis to plot"),
    ("Clock::seed", "constant-input constructor: no size axis to plot"),
    // ── O(1)-scale accessors and predicates ──
    (
        "Party::is_seed",
        "equality against the constant 1-byte seed form: O(1)-scale, no fuel story",
    ),
    (
        "Version::is_empty",
        "an O(1) bit test against the canonical 2-bit empty stream",
    ),
    ("Party::as_bytes", "O(1) borrow of the stored packed bytes"),
    ("Version::as_bytes", "O(1) borrow of the stored packed bytes"),
    ("Party::encoded_bits", "stored-length accessor over the packed form"),
    ("Version::encoded_bits", "stored-length accessor over the packed form"),
    ("Clock::encoded_bits", "sum of the two parts' stored-length accessors"),
    ("Clock::party", "O(1) reference accessor"),
    (
        "Clock::version",
        "O(1) reference accessor (the guest's clock-to-version bridge kernel prices \
         a clone, not this accessor)",
    ),
    (
        "Ranked::version",
        "O(1) accessor over the precomputed (rank, version) pair",
    ),
    ("Ranked::rank", "O(1) accessor over the precomputed (rank, version) pair"),
    (
        "Ranked::into_parts",
        "O(1) decomposition of the precomputed (rank, version) pair",
    ),
    // ── delegating wrappers priced at the operation they wrap ──
    (
        "Party::tick",
        "the identical event walk as Version::tick, entered from the party (the \
         version_tick panel prices it)",
    ),
    (
        "Party::encode_to",
        "the encode walk with a writer sink; the party_encode panel prices the walk",
    ),
    (
        "Version::encode_to",
        "the encode walk with a writer sink; the version_encode panel prices the walk",
    ),
    (
        "Clock::encode_to",
        "the encode walk with a writer sink; the clock_encode panel prices the walk",
    ),
    (
        "Version Sum / FromIterator (owned and borrowed)",
        "delegates to the join_all fold; the version_join_all panel prices the \
         mechanism",
    ),
    (
        "Clock | Version and Version | Clock (heterogeneous joins, |=)",
        "the version-join walk entered through the clock's version; the version_join \
         panel prices the walk (no guest kernel for the heterogeneous entry)",
    ),
    (
        "From<Party> for [Party; N] (consuming balanced split)",
        "consuming form of the balanced split the party_forks panel prices",
    ),
    (
        "iter::Party / iter::Clock (Forks iterators, drop folds back)",
        "hand-out mechanics over the balanced split the party_forks panel prices",
    ),
    (
        "Ranked Ord / From<Version> (byte tiebreak)",
        "composition of the rank walk (version_rank panel) with byte comparison",
    ),
    // ── representation mechanics ──
    (
        "Version Eq / Hash (canonical byte compare)",
        "byte compare/hash of the canonical form; representation mechanics, not a \
         tree walk",
    ),
    (
        "Party Eq / Hash (canonical byte compare)",
        "byte compare/hash of the canonical form; representation mechanics, not a \
         tree walk",
    ),
    (
        "Ticks ZERO / From / FromStr / Display / Add / Sum / Ord / Eq / Hash",
        "the opaque count carrier's own arithmetic and text, not a tree walk; its \
         semantics are priced at the min_ticks panel",
    ),
    // ── linearity escape hatches ──
    (
        "Party::dangerously_alias",
        "deliberate linearity escape hatch: a byte-copy clone, no guest kernel",
    ),
    (
        "Clock::dangerously_alias",
        "deliberate linearity escape hatch: a two-field byte-copy clone, no guest \
         kernel",
    ),
    // ── no guest kernel exports the operation ──
    (
        "Party::ticks",
        "Ticks-argument walk with no guest kernel; the version_tick panel prices the \
         walk's unit step",
    ),
    (
        "Version::ticks",
        "Ticks-argument walk with no guest kernel; the version_tick panel prices the \
         walk's unit step",
    ),
    (
        "Clock::ticks",
        "Ticks-argument walk with no guest kernel; the clock_tick panel prices the \
         walk's unit step",
    ),
    (
        "Party::join_all",
        "n-ary party fold with no guest kernel (the guest's party join is binary); \
         the party_join panel prices the merge step",
    ),
    (
        "Clock::join_all",
        "n-ary clock fold with no guest kernel; the clock_join panel prices the \
         merge step and the version_join_all panel the version-side fold",
    ),
    (
        "Clock::forks",
        "no guest kernel; composition of the party split (party_forks panel) and a \
         version clone per share",
    ),
    (
        "Clock Display / FromStr / TryFrom",
        "no guest kernel for clock text; the party and version text panels price \
         both constituent walks",
    ),
    (
        "serde / borsh impls (feature-gated, strict-decode pinned)",
        "feature-gated shims over the packed codecs; the decode/encode panels price \
         the walks, and no guest kernel exports the shims",
    ),
    (
        "OwnVersion vs Version comparisons (PartialEq/PartialOrd, both directions, owned and borrowed)",
        "no guest kernel for the fused masked comparison; the version_project and \
         version_cmp panels price the constituent walks",
    ),
    (
        "OwnVersion vs OwnVersion comparisons (the four-stream co-walk, owned and borrowed)",
        "no guest kernel for the fused four-stream co-walk; the version_project and \
         version_cmp panels price the constituent walks",
    ),
    (
        "causally::all",
        "O(1) unbounded-range constructor; no guest kernel exports the causally \
         combinators",
    ),
    ("causally::since", "O(1) range view constructor over a borrowed version"),
    ("causally::not_before", "O(1) range view constructor over a borrowed version"),
    ("causally::known_at", "O(1) range view constructor over a borrowed version"),
    ("causally::before", "O(1) range view constructor over a borrowed version"),
    (
        "causally::delta",
        "range constructor whose validity check is one causal comparison; the \
         version_cmp panel prices the walk",
    ),
    (
        "causally::delta_before",
        "range constructor whose validity check is one causal comparison; the \
         version_cmp panel prices the walk",
    ),
    ("causally::Range::since", "O(1) bound replacement plus one causal comparison"),
    (
        "causally::Range::not_before",
        "O(1) bound replacement plus one causal comparison",
    ),
    ("causally::Range::known_at", "O(1) bound replacement plus one causal comparison"),
    ("causally::Range::before", "O(1) bound replacement plus one causal comparison"),
    (
        "causally::Range::contains",
        "definitional combination of causal comparisons against the bounds; the \
         version_cmp panel prices the walk",
    ),
    (
        "causally::Range::placement_of",
        "definitional combination of causal comparisons against the bounds; the \
         version_cmp panel prices the walk",
    ),
    // ── not operations ──
    (
        "unbounded depth (beyond the differential grids)",
        "a depth regime, not an operation; the atlas plots operations over size",
    ),
    (
        "meter / error / iter plumbing",
        "instrumentation and data plumbing, not ITC operations",
    ),
];
