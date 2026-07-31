//! The operation roster: which public operations the atlas measures, how
//! each row's inputs are drawn, and how each runs in the fuel guest.
//!
//! One table row per operation — adding an operation is one [`OpSpec`]
//! entry. Each row names its input space (which picks the samplers and
//! the size measure — the row's `size_measure` string is stamped on its
//! render), the coverage roster rows it covers, and a `measure` function
//! that stages the sampled packed inputs into the guest and runs exactly
//! one measured kernel, returning that call's fuel. Register loading and
//! all other staging happen before the measured call, so a reading prices
//! one public operation (plus the guest's constant dispatch overhead,
//! identical for every sample).
//!
//! # Totality over the public surface
//!
//! Coverage is bound to the surface-coverage suite's committed roster
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
    /// A version slice: arity drawn uniformly from every count the
    /// column's budget can feed (`1..=size`, one byte per operand), then
    /// the total split uniformly over the compositions into one exact
    /// size per operand.
    ///
    /// Arity is stratified deliberately, not left to the composition
    /// count: uniform over whole compositions would concentrate nearly
    /// all mass at many-tiny-operand slices (the compositions of `N`
    /// into `k` parts peak at `k ≈ N/2`), starving the thin-and-wide
    /// and thick-and-narrow shapes a fold's cost actually turns on. The
    /// stratified draw represents every arity equally per column; the
    /// committed fold-cure families mark the adversarial arity ramps on
    /// top of this bulk measure.
    VersionSlice,
    /// One packed party followed by a version slice, composed into
    /// disjoint clocks in the guest: the clock-fold row's input space.
    ///
    /// The clock count is drawn uniformly from every count the column's
    /// budget can feed (`1..=size − 1`: one byte for the party, one per
    /// version), then the total splits uniformly over the compositions
    /// into one exact size per operand, party first. The party is
    /// fork-split in preparation so each drawn version rides its own
    /// disjoint party — independent uniform parties are almost never
    /// pairwise disjoint, so the fold's domain is reached by splitting
    /// one.
    ///
    /// The clock count is stratified for the same reason as
    /// [`VersionSlice`](Inputs::VersionSlice)'s arity: a panel can read
    /// a fold's arity factor only if the arity varies across samples.
    ClockSlice,
    /// One packed party of exactly the column's size, split into a
    /// drawn number of balanced shares in the guest: the party-fold
    /// row's input space.
    ///
    /// The share count is drawn uniformly over
    /// `1..=size` — a declared range, since the shares are minted in
    /// the guest and the byte budget therefore does not bound the
    /// count; `1..=size` mirrors the slice rows' arity-per-column
    /// envelope so the fold panels read comparably. Any party admits
    /// any share count (a balanced split subdivides leaves as far as it
    /// needs), so the draw never rejects, and the size axis stays the
    /// one party's own packed bytes.
    PartyShares,
}

impl Inputs {
    /// The smallest total size (bytes) a column of this input space can
    /// have: one byte per operand.
    pub fn min_bytes(&self) -> usize {
        match self {
            Inputs::Packed(operands) | Inputs::PackedDistinct(operands) => operands.len().max(1),
            // One one-byte operand: the smallest slice is unary.
            Inputs::VersionSlice => 1,
            // One byte for the party, one for the smallest slice.
            Inputs::ClockSlice => 2,
            // The one party; the share count costs no bytes.
            Inputs::PartyShares => 1,
        }
    }
}

/// The share count of the `party_forks` row: the size axis is the
/// party's packed bytes, the arity a declared constant.
const FORKS_SHARES: u32 = 8;

/// The tick count the `version_ticks` row drives, a declared constant
/// large enough that iterated single ticks could never reach it.
///
/// The panel reads at the fused walk's flat cost, not a
/// count-proportional one: the walk is at most two fused passes and one
/// splice at any count, and the count itself enters only as its bit
/// width.
const TICKS_COUNT: u32 = 1_000_000_000;

/// One measured operation: a roster row.
pub struct OpSpec {
    /// The atlas name (also the output file stem).
    pub name: &'static str,
    /// The input space the row samples.
    pub inputs: Inputs,
    /// The coverage roster rows (`before::surface` op names) this panel
    /// prices; the parity test holds panels ∪ exemptions total over the
    /// roster.
    pub covers: &'static [&'static str],
    /// The declared size measure, stamped verbatim on the render.
    pub size_measure: &'static str,
    /// Stage `inputs` (one packed encoding per operand) and run the one
    /// measured kernel, returning its fuel.
    ///
    /// The last argument is the sample's drawn arity. Every host-drawn
    /// row can read its arity off `inputs` itself (the slice rows do),
    /// so those rows ignore it; the guest-split fold row
    /// ([`PartyShares`](Inputs::PartyShares)) mints its fold operands
    /// inside the guest from the single drawn party, so its drawn share
    /// count reaches the kernel only through this argument.
    pub measure: fn(&mut Guest, &[Vec<u8>], usize) -> Measured,
}

/// The size measure of a one-operand packed row.
const M_UNARY: &str = "exact packed bytes, uniform per size";
/// The size measure of a two-operand packed row.
const M_BINARY: &str = "total packed bytes; split uniform across the two operands";
/// The size measure of a slice row.
const M_SLICE: &str =
    "total packed bytes; arity uniform over 1..=size, split uniform over the compositions";
/// The size measure of a composed-clock unary row.
const M_CLOCK: &str = "total packed bytes of the clock's party and version parts, split uniform";
/// The size measure of the fork-split disjoint-clock rows.
const M_CLOCK_PAIR: &str = "total packed bytes of one party and two versions, split uniform \
     three ways; the party fork-split into the clocks' disjoint parties";
/// The size measure of the span rows whose span is composed in
/// preparation as two sampled operands' pair hull.
const M_SPAN_HULL: &str = "total packed bytes of the two operands whose pair hull is the \
     span (hull composed in unmeasured preparation; the endpoints' sizes are on the \
     operands' scale, not exactly their sum)";
/// The size measure of the span placement rows: a hulled pair plus a
/// probe.
const M_SPAN_PROBE: &str = "total packed bytes of the two hull operands and the probe, \
     split uniform three ways (the span composed in unmeasured preparation as the \
     operands' pair hull, so the measured fused walk reads the probe against the \
     hull's meet and join)";
/// The size measure of the binary span-operator rows: two spans, each
/// composed in preparation as a sampled pair's hull.
const M_SPAN_PAIR: &str = "total packed bytes of the four operands whose pair hulls are \
     the two spans, split uniform four ways (hulls composed in unmeasured preparation; \
     the endpoints' sizes are on the operands' scale, not exactly their sum)";
/// The size measure of the n-ary span-door rows: drawn versions
/// composed into spans as cyclically adjacent pair hulls.
const M_SPAN_FOLD: &str = "total packed bytes; arity uniform over 1..=size, split \
     uniform over the compositions, the k drawn versions composed in unmeasured \
     preparation into k spans (span i the pair hull of versions i and i+1, \
     cyclically, so each operand rides in two adjacent hulls), the first span \
     riding as the fold's receiver";
/// The size measure of the masked span placement rows: a hulled pair,
/// the masking party, and the probe.
const M_OWN_SPAN_PROBE: &str = "total packed bytes of the two hull operands, the \
     masking party, and the probe, split uniform four ways (the span composed in \
     unmeasured preparation as the operands' pair hull; the measured verdict runs \
     the masked co-walks against the projected endpoints, no materialization)";

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
        measure: |g, inputs, _| {
            g.stage_write(&inputs[0]);
            g.call("ff_version_decode", &[0])
        },
    },
    OpSpec {
        name: "version_encode",
        inputs: Inputs::Packed(&[Operand::Version]),
        covers: &["Version::encode"],
        size_measure: M_UNARY,
        measure: |g, inputs, _| {
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
        measure: |g, inputs, _| {
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
        measure: |g, inputs, _| {
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
        measure: |g, inputs, _| {
            load_version(g, 0, &inputs[0]);
            g.call("ff_version_rank", &[1, 0])
        },
    },
    OpSpec {
        name: "version_min_ticks",
        inputs: Inputs::Packed(&[Operand::Version]),
        covers: &["Version::min_ticks"],
        size_measure: M_UNARY,
        measure: |g, inputs, _| {
            load_version(g, 0, &inputs[0]);
            g.call_i64("ff_version_min_ticks", &[0])
        },
    },
    OpSpec {
        name: "version_tick",
        inputs: Inputs::Packed(&[Operand::Version, Operand::Party]),
        covers: &["Version::tick"],
        size_measure: M_BINARY,
        measure: |g, inputs, _| {
            load_version(g, 0, &inputs[0]);
            load_party(g, 1, &inputs[1]);
            g.call("ff_version_tick", &[0, 1])
        },
    },
    OpSpec {
        name: "version_ticks",
        inputs: Inputs::Packed(&[Operand::Version, Operand::Party]),
        covers: &["Version::ticks"],
        size_measure: "total packed bytes; split uniform across the two operands (tick \
             count a declared constant, 10⁹: the fused multi-tick walk is flat in the \
             count — at most two fused passes and one splice — so one panel at one \
             large count is the whole n-dependence)",
        measure: |g, inputs, _| {
            load_version(g, 0, &inputs[0]);
            load_party(g, 1, &inputs[1]);
            g.call("ff_version_ticks", &[0, 1, TICKS_COUNT])
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
        measure: |g, inputs, _| {
            load_version(g, 0, &inputs[0]);
            load_party(g, 1, &inputs[1]);
            g.call("ff_version_project", &[2, 0, 1])
        },
    },
    OpSpec {
        name: "own_version_cmp",
        inputs: Inputs::Packed(&[Operand::Version, Operand::Party, Operand::Version]),
        covers: &[
            "OwnVersion vs Version comparisons (PartialEq/PartialOrd, both directions, owned and borrowed)",
        ],
        size_measure: "total packed bytes of the projected version, its masking party, \
             and the compared version, split uniform three ways (the fused \
             three-stream co-walk, no materialization; view construction is O(1) \
             preparation, and the equality entry runs the same fused mechanism)",
        measure: |g, inputs, _| {
            load_version(g, 0, &inputs[0]);
            load_party(g, 1, &inputs[1]);
            load_version(g, 2, &inputs[2]);
            g.call("ff_own_version_cmp", &[0, 1, 2])
        },
    },
    OpSpec {
        name: "own_version_pair_cmp",
        inputs: Inputs::Packed(&[
            Operand::Version,
            Operand::Party,
            Operand::Version,
            Operand::Party,
        ]),
        covers: &["OwnVersion vs OwnVersion comparisons (the four-stream co-walk, owned and borrowed)"],
        size_measure: "total packed bytes of the two views' versions and masking \
             parties, split uniform four ways (the fused four-stream co-walk, no \
             materialization; view construction is O(1) preparation, and the equality \
             entry runs the same fused mechanism)",
        measure: |g, inputs, _| {
            load_version(g, 0, &inputs[0]);
            load_party(g, 1, &inputs[1]);
            load_version(g, 2, &inputs[2]);
            load_party(g, 3, &inputs[3]);
            g.call("ff_own_version_pair_cmp", &[0, 1, 2, 3])
        },
    },
    OpSpec {
        name: "version_cmp",
        inputs: Inputs::Packed(&[Operand::Version, Operand::Version]),
        covers: &["Version PartialOrd (the comparison matrix, owned and borrowed)"],
        size_measure: M_BINARY,
        measure: |g, inputs, _| {
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
        measure: |g, inputs, _| {
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
        measure: |g, inputs, _| {
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
        measure: |g, inputs, _| {
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
        measure: |g, inputs, _| {
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
        measure: |g, inputs, _| {
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
        measure: |g, inputs, _| {
            let n = load_slice(g, inputs);
            g.call("ff_version_join_all", &[n, 0, n])
        },
    },
    OpSpec {
        name: "version_meet_all",
        inputs: Inputs::VersionSlice,
        covers: &["Version::meet_all"],
        size_measure: M_SLICE,
        measure: |g, inputs, _| {
            let n = load_slice(g, inputs);
            g.call("ff_version_meet_all", &[n, 0, n])
        },
    },
    OpSpec {
        name: "version_span",
        inputs: Inputs::Packed(&[Operand::Version, Operand::Version]),
        covers: &["Version::span"],
        size_measure: M_BINARY,
        measure: |g, inputs, _| {
            load_version(g, 0, &inputs[0]);
            load_version(g, 1, &inputs[1]);
            g.call("ff_version_span", &[2, 0, 1])
        },
    },
    OpSpec {
        name: "version_span_all",
        inputs: Inputs::VersionSlice,
        covers: &["Version::span_all"],
        size_measure: "total packed bytes; arity uniform over 1..=size, split uniform \
             over the compositions (the first drawn operand rides as the hull fold's \
             receiver, feed order preserved)",
        measure: |g, inputs, _| {
            let n = load_slice(g, inputs);
            g.call("ff_version_span_all", &[n, 0, n])
        },
    },
    // ───────────────────────────── Party ─────────────────────────────
    OpSpec {
        name: "party_decode",
        inputs: Inputs::Packed(&[Operand::Party]),
        covers: &["Party::decode"],
        size_measure: M_UNARY,
        measure: |g, inputs, _| {
            g.stage_write(&inputs[0]);
            g.call("ff_party_decode", &[0])
        },
    },
    OpSpec {
        name: "party_encode",
        inputs: Inputs::Packed(&[Operand::Party]),
        covers: &["Party::encode"],
        size_measure: M_UNARY,
        measure: |g, inputs, _| {
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
        measure: |g, inputs, _| {
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
        measure: |g, inputs, _| {
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
        measure: |g, inputs, _| {
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
        measure: |g, inputs, _| {
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
        measure: |g, inputs, _| {
            load_party(g, 0, &inputs[0]);
            prep(g, "ff_party_fork", &[1, 0]);
            g.call("ff_party_join", &[0, 1])
        },
    },
    OpSpec {
        name: "party_join_all",
        inputs: Inputs::PartyShares,
        covers: &["Party::join_all"],
        size_measure: "packed bytes of one uniform party; share count uniform over \
             1..=size (drawn per sample; the balanced split is minted in the guest, \
             so the size axis stays the party's own bytes), and the measured fold \
             re-merges the shares into the residual — independent uniform parties \
             are almost never pairwise disjoint, so the n-ary domain is reached by \
             re-merging a split",
        measure: |g, inputs, arity| {
            let shares = arity as u32;
            load_party(g, 0, &inputs[0]);
            prep(g, "ff_party_forks", &[1, 0, shares]);
            g.call("ff_party_join_all", &[0, 1, shares])
        },
    },
    OpSpec {
        name: "party_is_disjoint",
        inputs: Inputs::Packed(&[Operand::Party, Operand::Party]),
        covers: &["Party::is_disjoint"],
        size_measure: M_BINARY,
        measure: |g, inputs, _| {
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
        measure: |g, inputs, _| {
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
        measure: |g, inputs, _| {
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
        measure: |g, inputs, _| {
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
        measure: |g, inputs, _| {
            compose_clock(g, &inputs[0], &inputs[1]);
            g.call("ff_clock_encode", &[0])
        },
    },
    OpSpec {
        name: "clock_tick",
        inputs: Inputs::Packed(&[Operand::Party, Operand::Version]),
        covers: &["Clock::tick"],
        size_measure: M_CLOCK,
        measure: |g, inputs, _| {
            compose_clock(g, &inputs[0], &inputs[1]);
            g.call("ff_clock_tick", &[0])
        },
    },
    OpSpec {
        name: "clock_fork",
        inputs: Inputs::Packed(&[Operand::Party, Operand::Version]),
        covers: &["Clock::fork"],
        size_measure: M_CLOCK,
        measure: |g, inputs, _| {
            compose_clock(g, &inputs[0], &inputs[1]);
            g.call("ff_clock_fork", &[3, 0])
        },
    },
    OpSpec {
        name: "clock_send",
        inputs: Inputs::Packed(&[Operand::Party, Operand::Version]),
        covers: &["Clock::send"],
        size_measure: M_CLOCK,
        measure: |g, inputs, _| {
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
        measure: |g, inputs, _| {
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
        measure: |g, inputs, _| {
            compose_clock_pair(g, inputs);
            g.call("ff_clock_join", &[4, 5])
        },
    },
    OpSpec {
        name: "clock_join_all",
        inputs: Inputs::ClockSlice,
        covers: &["Clock::join_all"],
        size_measure: "total packed bytes of one party and the drawn versions, split \
             uniform over the compositions; clock count uniform over 1..=size−1 \
             (every count the budget can feed), the party fork-split in preparation \
             so the parties partition one region, and the measured fold reunites the \
             drawn clocks into the first",
        measure: |g, inputs, _| {
            // The drawn clock count rides in-band: one party, then one
            // version per clock.
            let clocks = (inputs.len() - 1) as u32;
            load_party(g, 0, &inputs[0]);
            // `clocks − 1` shares into registers 1..clocks; register 0
            // keeps the residual, so the `clocks` parties partition the
            // sampled region (a one-clock draw splits nothing and the
            // measured fold is the empty fold into that clock).
            prep(g, "ff_party_forks", &[1, 0, clocks - 1]);
            for (i, version) in inputs[1..].iter().enumerate() {
                load_version(g, clocks + i as u32, version);
            }
            for i in 0..clocks {
                prep(g, "ff_clock_from_parts", &[2 * clocks + i, i, clocks + i]);
            }
            g.call("ff_clock_join_all", &[2 * clocks, 2 * clocks + 1, clocks - 1])
        },
    },
    OpSpec {
        name: "clock_sync",
        inputs: Inputs::Packed(&[Operand::Party, Operand::Version, Operand::Version]),
        covers: &["Clock::sync"],
        size_measure: M_CLOCK_PAIR,
        measure: |g, inputs, _| {
            compose_clock_pair(g, inputs);
            g.call("ff_clock_sync", &[4, 5])
        },
    },
    OpSpec {
        name: "clock_from_parts",
        inputs: Inputs::Packed(&[Operand::Party, Operand::Version]),
        covers: &["Clock::from_parts"],
        size_measure: M_CLOCK,
        measure: |g, inputs, _| {
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
        measure: |g, inputs, _| {
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
        measure: |g, inputs, _| {
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
        measure: |g, inputs, _| {
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
        measure: |g, inputs, _| {
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
        measure: |g, inputs, _| {
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
        measure: |g, inputs, _| {
            load_version(g, 0, &inputs[0]);
            prep(g, "ff_version_rank", &[1, 0]);
            g.call("ff_rank_display", &[1])
        },
    },
    // ───────────────────────────────── Span ─────────────────────────────────
    OpSpec {
        name: "span_place",
        inputs: Inputs::Packed(&[Operand::Version, Operand::Version, Operand::Version]),
        covers: &["Span::place"],
        size_measure: M_SPAN_PROBE,
        measure: |g, inputs, _| {
            load_version(g, 0, &inputs[0]);
            load_version(g, 1, &inputs[1]);
            load_version(g, 2, &inputs[2]);
            prep(g, "ff_version_span", &[3, 0, 1]);
            g.call("ff_span_place", &[3, 2])
        },
    },
    OpSpec {
        name: "span_dominance",
        inputs: Inputs::Packed(&[Operand::Version, Operand::Version, Operand::Version]),
        covers: &["Span::dominance_of"],
        size_measure: M_SPAN_PROBE,
        measure: |g, inputs, _| {
            load_version(g, 0, &inputs[0]);
            load_version(g, 1, &inputs[1]);
            load_version(g, 2, &inputs[2]);
            prep(g, "ff_version_span", &[3, 0, 1]);
            g.call("ff_span_dominance", &[3, 2])
        },
    },
    OpSpec {
        name: "span_encode",
        inputs: Inputs::Packed(&[Operand::Version, Operand::Version]),
        covers: &["Span::encode"],
        size_measure: M_SPAN_HULL,
        measure: |g, inputs, _| {
            load_version(g, 0, &inputs[0]);
            load_version(g, 1, &inputs[1]);
            prep(g, "ff_version_span", &[2, 0, 1]);
            g.call("ff_span_encode", &[2])
        },
    },
    OpSpec {
        name: "span_decode",
        inputs: Inputs::Packed(&[Operand::Version, Operand::Version]),
        covers: &["Span::decode"],
        size_measure: M_SPAN_HULL,
        measure: |g, inputs, _| {
            load_version(g, 0, &inputs[0]);
            load_version(g, 1, &inputs[1]);
            prep(g, "ff_version_span", &[2, 0, 1]);
            // The unmeasured encode leaves the span's canonical composite
            // in the staging buffer for the measured decode to read.
            prep(g, "ff_span_encode", &[2]);
            g.call("ff_span_decode", &[3])
        },
    },
    OpSpec {
        name: "span_union",
        inputs: Inputs::Packed(&[
            Operand::Version,
            Operand::Version,
            Operand::Version,
            Operand::Version,
        ]),
        covers: &["Span | Span (BitOr, owned and borrowed — the containment join)"],
        size_measure: M_SPAN_PAIR,
        measure: |g, inputs, _| {
            load_version(g, 0, &inputs[0]);
            load_version(g, 1, &inputs[1]);
            load_version(g, 2, &inputs[2]);
            load_version(g, 3, &inputs[3]);
            prep(g, "ff_version_span", &[4, 0, 1]);
            prep(g, "ff_version_span", &[5, 2, 3]);
            g.call("ff_span_union", &[6, 4, 5])
        },
    },
    OpSpec {
        name: "span_intersect",
        inputs: Inputs::Packed(&[
            Operand::Version,
            Operand::Version,
            Operand::Version,
            Operand::Version,
        ]),
        covers: &["Span & Span (BitAnd, owned and borrowed — the containment meet)"],
        size_measure: M_SPAN_PAIR,
        measure: |g, inputs, _| {
            load_version(g, 0, &inputs[0]);
            load_version(g, 1, &inputs[1]);
            load_version(g, 2, &inputs[2]);
            load_version(g, 3, &inputs[3]);
            prep(g, "ff_version_span", &[4, 0, 1]);
            prep(g, "ff_version_span", &[5, 2, 3]);
            // Both verdicts (1 shared segment, 0 empty intersection) are
            // measured outcomes of the one kernel call.
            g.call("ff_span_intersect", &[6, 4, 5])
        },
    },
    OpSpec {
        name: "span_sum",
        inputs: Inputs::Packed(&[
            Operand::Version,
            Operand::Version,
            Operand::Version,
            Operand::Version,
        ]),
        covers: &["Span + Span (Add, owned and borrowed — the pointwise join)"],
        size_measure: M_SPAN_PAIR,
        measure: |g, inputs, _| {
            load_version(g, 0, &inputs[0]);
            load_version(g, 1, &inputs[1]);
            load_version(g, 2, &inputs[2]);
            load_version(g, 3, &inputs[3]);
            prep(g, "ff_version_span", &[4, 0, 1]);
            prep(g, "ff_version_span", &[5, 2, 3]);
            g.call("ff_span_sum", &[6, 4, 5])
        },
    },
    OpSpec {
        name: "span_product",
        inputs: Inputs::Packed(&[
            Operand::Version,
            Operand::Version,
            Operand::Version,
            Operand::Version,
        ]),
        covers: &["Span * Span (Mul, owned and borrowed — the pointwise meet)"],
        size_measure: M_SPAN_PAIR,
        measure: |g, inputs, _| {
            load_version(g, 0, &inputs[0]);
            load_version(g, 1, &inputs[1]);
            load_version(g, 2, &inputs[2]);
            load_version(g, 3, &inputs[3]);
            prep(g, "ff_version_span", &[4, 0, 1]);
            prep(g, "ff_version_span", &[5, 2, 3]);
            g.call("ff_span_product", &[6, 4, 5])
        },
    },
    OpSpec {
        name: "span_union_all",
        inputs: Inputs::VersionSlice,
        covers: &["Span::union_all"],
        size_measure: M_SPAN_FOLD,
        measure: |g, inputs, _| {
            let n = load_slice(g, inputs);
            for i in 0..n {
                prep(g, "ff_version_span", &[n + i, i, (i + 1) % n]);
            }
            g.call("ff_span_union_all", &[2 * n, n, n])
        },
    },
    OpSpec {
        name: "span_intersect_all",
        inputs: Inputs::VersionSlice,
        covers: &["Span::intersect_all"],
        size_measure: M_SPAN_FOLD,
        measure: |g, inputs, _| {
            let n = load_slice(g, inputs);
            for i in 0..n {
                prep(g, "ff_version_span", &[n + i, i, (i + 1) % n]);
            }
            // Both verdicts (1 shared segment, 0 empty intersection) are
            // measured outcomes of the one fold call.
            g.call("ff_span_intersect_all", &[2 * n, n, n])
        },
    },
    OpSpec {
        name: "span_sum_all",
        inputs: Inputs::VersionSlice,
        covers: &["Span::sum_all"],
        size_measure: M_SPAN_FOLD,
        measure: |g, inputs, _| {
            let n = load_slice(g, inputs);
            for i in 0..n {
                prep(g, "ff_version_span", &[n + i, i, (i + 1) % n]);
            }
            g.call("ff_span_sum_all", &[2 * n, n, n])
        },
    },
    OpSpec {
        name: "span_product_all",
        inputs: Inputs::VersionSlice,
        covers: &["Span::product_all"],
        size_measure: M_SPAN_FOLD,
        measure: |g, inputs, _| {
            let n = load_slice(g, inputs);
            for i in 0..n {
                prep(g, "ff_version_span", &[n + i, i, (i + 1) % n]);
            }
            g.call("ff_span_product_all", &[2 * n, n, n])
        },
    },
    OpSpec {
        name: "span_project",
        inputs: Inputs::Packed(&[Operand::Version, Operand::Version, Operand::Party]),
        covers: &[
            "&Span / &Party (Div — the lazy span projection view)",
            "OwnSpan::to_span",
            "From<OwnSpan> for Span (explicit materialization)",
        ],
        size_measure: "total packed bytes of the two hull operands and the projecting \
             party, split uniform three ways (the span composed in unmeasured \
             preparation as the operands' pair hull; the view is O(1) preparation, \
             and the measured kernel materializes both projected endpoints)",
        measure: |g, inputs, _| {
            load_version(g, 0, &inputs[0]);
            load_version(g, 1, &inputs[1]);
            load_party(g, 2, &inputs[2]);
            prep(g, "ff_version_span", &[3, 0, 1]);
            g.call("ff_span_project", &[4, 3, 2])
        },
    },
    OpSpec {
        name: "own_span_place",
        inputs: Inputs::Packed(&[
            Operand::Version,
            Operand::Version,
            Operand::Party,
            Operand::Version,
        ]),
        covers: &["OwnSpan::place"],
        size_measure: M_OWN_SPAN_PROBE,
        measure: |g, inputs, _| {
            load_version(g, 0, &inputs[0]);
            load_version(g, 1, &inputs[1]);
            load_party(g, 2, &inputs[2]);
            load_version(g, 3, &inputs[3]);
            prep(g, "ff_version_span", &[4, 0, 1]);
            g.call("ff_own_span_place", &[4, 2, 3])
        },
    },
    OpSpec {
        name: "own_span_dominance",
        inputs: Inputs::Packed(&[
            Operand::Version,
            Operand::Version,
            Operand::Party,
            Operand::Version,
        ]),
        covers: &["OwnSpan::dominance_of"],
        size_measure: M_OWN_SPAN_PROBE,
        measure: |g, inputs, _| {
            load_version(g, 0, &inputs[0]);
            load_version(g, 1, &inputs[1]);
            load_party(g, 2, &inputs[2]);
            load_version(g, 3, &inputs[3]);
            prep(g, "ff_version_span", &[4, 0, 1]);
            g.call("ff_own_span_dominance", &[4, 2, 3])
        },
    },
];

/// The coverage roster rows deliberately without a panel, each with its
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
    (
        "Party::seed",
        "constant-input constructor: no size axis to plot",
    ),
    (
        "Version::new",
        "constant-input constructor: no size axis to plot",
    ),
    (
        "Clock::seed",
        "constant-input constructor: no size axis to plot",
    ),
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
    (
        "Version::as_bytes",
        "O(1) borrow of the stored packed bytes",
    ),
    (
        "Party::encoded_bits",
        "stored-length accessor over the packed form",
    ),
    (
        "Version::encoded_bits",
        "stored-length accessor over the packed form",
    ),
    (
        "Clock::encoded_bits",
        "sum of the two parts' stored-length accessors",
    ),
    ("Clock::party", "O(1) reference accessor"),
    (
        "Clock::version",
        "O(1) reference accessor (the guest's clock-to-version bridge kernel prices \
         a clone, not this accessor)",
    ),
    (
        "Version::ranked",
        "O(1) borrowing view construction (the version_rank panel prices the walk \
         its comparisons run)",
    ),
    ("Ranked::version", "O(1) borrow of the viewed version"),
    (
        "Ranked::into_owned",
        "at most one refcount-bump clone of the borrowed version; no walk, no byte copy",
    ),
    // ── delegating wrappers priced at the operation they wrap ──
    (
        "Party::tick",
        "the identical event walk as Version::tick, entered from the party (the \
         version_tick panel prices it)",
    ),
    (
        "Party::ticks",
        "the identical fused multi-tick walk as Version::ticks, entered from the \
         party (the version_ticks panel prices it)",
    ),
    (
        "Clock::ticks",
        "the identical fused multi-tick walk as Version::ticks, entered on the \
         clock's own parts (the version_ticks panel prices it)",
    ),
    (
        "Clock::forks",
        "composition of the balanced party split (the party_forks panel prices the \
         walk) with one version refcount-bump clone per share; no distinct walk",
    ),
    (
        "Clock Display / FromStr / TryFrom",
        "composition of the party and version text walks (the party and version \
         text panels price them) plus a top-level delimiter scan over the same \
         text; no distinct walk",
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
        "From<Clock> for [Clock; N] (consuming balanced split)",
        "consuming form of the Clock::forks composition: the balanced party split \
         (party_forks panel) plus one version refcount-bump clone per share",
    ),
    (
        "iter::Party / iter::Clock (Forks iterators, drop folds back)",
        "hand-out mechanics over the balanced split the party_forks panel prices",
    ),
    (
        "Ranked comparisons and the Ranked / Rank From conversions (the total order)",
        "the fused signed instance of the pair co-sweep the version_rank panel's \
         integrator runs (plus one byte compare on rank ties); no guest kernel \
         exports the rank view (adding kernels is a crates/before change outside \
         this crate)",
    ),
    (
        "Ranked::to_rank",
        "the identical rank walk as Version::rank, entered from the view (the \
         version_rank panel prices it)",
    ),
    (
        "Ranked::encode",
        "the rank walk plus a linear emission and one version byte copy; no guest \
         kernel exports the composite key",
    ),
    (
        "Ranked::encode_rank",
        "the rank walk plus a linear emission; no guest kernel exports the encoding",
    ),
    (
        "Rank::encode",
        "a linear emission over an in-memory rank; no guest kernel exports the encoding",
    ),
    (
        "Rank::encode_to",
        "the identical emission with a writer sink; priced as Rank::encode",
    ),
    (
        "Ranked::encode_to",
        "the composite key emission with a writer sink; priced as Ranked::encode",
    ),
    (
        "Ranked::encode_rank_to",
        "the fused rank walk and emission with a writer sink; priced as \
         Ranked::encode_rank",
    ),
    (
        "Rank::decode",
        "a linear strict parse into an in-memory rank; no guest kernel exports the \
         decoding",
    ),
    (
        "Ranked::decode",
        "a linear strict parse plus the verifying rank walk the version_rank panel \
         prices; no guest kernel exports the composite key",
    ),
    (
        "Span::encode_to",
        "the identical composite emission with a writer sink; the span_encode panel \
         prices the emission",
    ),
    (
        "OwnSpan::meet",
        "O(1) accessor handing out the bound OwnVersion endpoint view",
    ),
    (
        "OwnSpan::join",
        "O(1) accessor handing out the bound OwnVersion endpoint view",
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
        "deliberate linearity escape hatch: a refcount-bump clone of the shared stored buffer, no guest kernel",
    ),
    (
        "Clock::dangerously_alias",
        "deliberate linearity escape hatch: a two-field refcount-bump clone of the \
         shared stored buffers, no guest kernel",
    ),
    // ── no guest kernel exports the operation ──
    (
        "serde / borsh impls (feature-gated, strict-decode pinned)",
        "feature-gated shims over the packed codecs; the decode/encode panels price \
         the walks, and no guest kernel exports the shims (the guest builds the \
         crate at default features)",
    ),
    (
        "causally::all",
        "O(1) unbounded-range constructor: both bounds unbounded, no comparison, \
         no walk",
    ),
    (
        "causally::since",
        "O(1) range view constructor over a borrowed version",
    ),
    (
        "causally::not_before",
        "O(1) range view constructor over a borrowed version",
    ),
    (
        "causally::known_at",
        "O(1) range view constructor over a borrowed version",
    ),
    (
        "causally::before",
        "O(1) range view constructor over a borrowed version",
    ),
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
    (
        "causally::Range::since",
        "O(1) bound replacement plus one causal comparison (the version_cmp panel \
         prices the comparison)",
    ),
    (
        "causally::Range::not_before",
        "O(1) bound replacement plus one causal comparison (the version_cmp panel \
         prices the comparison)",
    ),
    (
        "causally::Range::known_at",
        "O(1) bound replacement plus one causal comparison (the version_cmp panel \
         prices the comparison)",
    ),
    (
        "causally::Range::before",
        "O(1) bound replacement plus one causal comparison (the version_cmp panel \
         prices the comparison)",
    ),
    (
        "causally::Range::contains",
        "placement_of's Equal arm: the identical fused walk under an O(1) verdict \
         fold; the span_place and version_cmp panels price the walk",
    ),
    (
        "causally::Range::placement_of",
        "bounded's fused walk coarsened by bound kind, an O(1) fold over the \
         verdict; the span_place and version_cmp panels price the walk",
    ),
    (
        "causally::Range::bounded",
        "the fused placement co-walk with range-verdict hooks (branch-only, no \
         stream or accumulator work of their own): the span_place panel prices the \
         two-bounded walk, and the one-bound form degenerates to the version_cmp \
         panel's pair sweep — both identities meter-pinned",
    ),
    (
        "Span::new",
        "validating constructor: the check is literally one Version PartialOrd \
         call (the version_cmp panel's measured operation) around an O(1) \
         construction, so a panel would re-measure version_cmp under another name",
    ),
    (
        "Span::new_unchecked",
        "O(1) span constructor over two borrowed versions (the trusted \
         door performs no comparison)",
    ),
    (
        "Span::meet",
        "O(1) borrow of a stored endpoint: no walk, no comparison",
    ),
    (
        "Span::join",
        "O(1) borrow of a stored endpoint: no walk, no comparison",
    ),
    (
        "Span::into_parts",
        "borrow-settling destructure: at most one refcount-bump clone per \
         endpoint, no walk, no byte copy",
    ),
    (
        "Span::reborrow",
        "O(1) span over two fresh borrows of the stored endpoints: no walk, \
         no comparison",
    ),
    (
        "Span::into_owned",
        "borrow-settling conversion: at most one refcount-bump clone per \
         endpoint, no walk, no byte copy",
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
