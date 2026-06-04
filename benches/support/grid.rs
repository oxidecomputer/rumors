//! Shared fixtures for the reconciliation benchmarks.
//!
//! Both [`in_memory`](../in_memory.rs) (which reconciles two peers in-process
//! via [`Known::join`](rumors::sync::Known::join)) and
//! [`gossip_grid`](../gossip_grid.rs) (which reconciles them over a simulated
//! wire via [`Known::gossip`](rumors::sync::Known::gossip)) `#[path]`-include
//! this module so they measure *the same divergence shapes* — the in-memory
//! merge and the over-the-wire protocol can be compared cell for cell.
//!
//! # The divergence grid
//!
//! Two peers fork from a shared prefix and then each diverges along two axes:
//!
//! - `common`: messages inserted *before* the fork, so both peers already
//!   agree on them. Reconciliation should short-circuit these by hash.
//! - `differing`: fresh messages each peer originates *after* the fork; the
//!   other must learn all of them.
//! - `redacted`: a slice of the shared prefix each peer forgets after the fork;
//!   the other must honor the deletion. The two peers redact *disjoint* blocks
//!   (`left` forgets `shared[..redacted]`, `right` forgets
//!   `shared[redacted..2*redacted]`), so each side carries `redacted` deletions
//!   the other did not originate. This requires `common >= 2 * redacted`.
//!
//! Every axis sweeps powers of ten starting at zero, so the named shapes the
//! old bench hard-coded fall out as corners of the cube:
//!
//! - disjoint    = `common = 0,  differing = n, redacted = 0`
//! - small-delta = `common = n,  differing = k, redacted = 0` (small `k`)
//! - identical   = `common = n,  differing = 0, redacted = 0`

use rumors::sync::{Key, Known};

/// Live message counts for the non-grid benchmarks (`message`, `iter`,
/// `redact`), spanning three orders of magnitude.
#[allow(unused)]
pub const SIZES: &[usize] = &[100, 10_000, 1_000_000];

/// Shared-prefix sizes. Capped at 100k: building two trees per Criterion
/// iteration is the runtime bottleneck, and a million-message prefix can't
/// complete Criterion's ten-sample floor in reasonable time.
pub const COMMON: &[usize] = &[0, 1, 10, 100, 1_000, 10_000, 100_000];

/// Post-fork messages each peer originates. Same powers-of-ten sweep as
/// [`COMMON`]; at `differing = n, common = 0` this reproduces the fully
/// disjoint shape.
pub const DIFFERING: &[usize] = &[0, 1, 10, 100, 1_000, 10_000, 100_000];

/// Shared-prefix messages each peer redacts after the fork, in disjoint blocks
/// (see the module docs). Bounded per cell by `common / 2`.
pub const REDACTED: &[usize] = &[0, 1, 10, 100, 1_000, 10_000, 100_000];

/// An iterator yielding `n` unit payloads. `()` borsh-encodes to zero bytes, so
/// fixtures measure tree / clock / hashing work, not payload serialization.
pub fn units(n: usize) -> impl Iterator<Item = ()> + Send {
    std::iter::repeat_n((), n)
}

/// Criterion samples for a fixture of the given build magnitude. The largest
/// fixtures are expensive to (re)build in untimed setup, so they take
/// Criterion's floor of 10.
pub fn sample_size_for(n: usize) -> usize {
    match n {
        n if n >= 100_000 => 10,
        n if n >= 10_000 => 20,
        _ => 100,
    }
}

/// One cell of the divergence grid: a point in `(common, differing, redacted)`
/// space.
#[derive(Clone, Copy, Debug)]
pub struct Cell {
    pub common: usize,
    pub differing: usize,
    pub redacted: usize,
}

impl Cell {
    /// The per-side divergence a single reconciliation must transfer: the
    /// `differing` messages plus the `redacted` deletions the *other* peer
    /// originated. Throughput is charged against this, not the shared size:
    /// reconciliation cost tracks the difference between the peers, not how
    /// much they already agree on. Zero for the identical corner.
    pub fn divergence(&self) -> u64 {
        (self.differing + self.redacted) as u64
    }

    /// The dominant fixture-build cost: the messages inserted per side. Used to
    /// pick a sample count via [`sample_size_for`]. (Redactions only remove
    /// keys, so they don't grow the tree.)
    pub fn build_magnitude(&self) -> usize {
        self.common + self.differing
    }

    /// A stable, human-readable Criterion parameter id for this cell.
    pub fn id(&self) -> String {
        format!(
            "common={},differing={},redacted={}",
            self.common, self.differing, self.redacted
        )
    }
}

/// Every valid cell of the grid.
///
/// Skips two kinds of degenerate cells:
///
/// - `redacted > 0 && common < 2 * redacted`: the disjoint-block redaction
///   scheme needs two distinct `redacted`-sized slices of the shared prefix.
/// - the empty origin `(0, 0, 0)`: nothing is built and nothing reconciled.
pub fn cells() -> impl Iterator<Item = Cell> {
    COMMON.iter().flat_map(|&common| {
        DIFFERING.iter().flat_map(move |&differing| {
            REDACTED.iter().filter_map(move |&redacted| {
                if redacted > 0 && common < 2 * redacted {
                    return None;
                }
                if common == 0 && differing == 0 && redacted == 0 {
                    return None;
                }
                Some(Cell {
                    common,
                    differing,
                    redacted,
                })
            })
        })
    })
}

/// Build the two peers for one grid cell.
///
/// Both descend from a fresh [`Known::seed`] so their parties are disjoint (the
/// precondition for `join` / `gossip`). The shared prefix is inserted before
/// the fork; the `differing` messages and `redacted` deletions are applied
/// independently to each side after it.
pub fn build(cell: Cell) -> (Known<()>, Known<()>) {
    let Cell {
        common,
        differing,
        redacted,
    } = cell;

    let mut left: Known<()> = Known::seed();
    let mut shared: Vec<Key> = Vec::with_capacity(common);
    left.message_then(units(common), |k, _, _| shared.push(k));

    let mut right = left.fork();
    left.message(units(differing));
    right.message(units(differing));

    if redacted > 0 {
        // Disjoint blocks: each side forgets a distinct slice of the shared
        // prefix, so the other must honor `redacted` deletions it never made.
        // `cells` guarantees `common >= 2 * redacted`, so the slices don't
        // overlap and are in bounds.
        left.redact(shared[..redacted].iter().copied());
        right.redact(shared[redacted..2 * redacted].iter().copied());
    }

    (left, right)
}
