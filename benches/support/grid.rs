//! Shared fixtures for the reconciliation benchmarks.
//!
//! [`gossip_grid`](../gossip_grid.rs) reconciles two diverged peers over a
//! simulated wire via [`Known::gossip`](rumors::sync::Known::gossip) across
//! the divergence grid below; [`in_memory`](../in_memory.rs) shares the size
//! sweep and sample-size policy for the single-set surface (inserts,
//! iteration, ranges, observers, lookups).
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

use std::io::pipe;
use std::thread;

use borsh::{BorshDeserialize, BorshSerialize};
use rumors::sync::{Key, Known};

/// Mint a genuine party-disjoint originator that inherits `parent`'s content.
///
/// A peer that will independently `send`/`redact` (as both sides of every
/// grid cell do after the split) needs its own disjoint Interval Tree Clock
/// region. We mint one by serving a bootstrap from `parent` over a pair of
/// pipes: the newcomer pulls `parent`'s whole tree through the ordinary
/// mirror descent and is handed a fresh disjoint party, forked in the same
/// critical section that snapshots the served tree.
fn bootstrap_fork<T>(parent: &mut Known<T>) -> Known<T>
where
    T: BorshSerialize + BorshDeserialize + Clone + Send + Sync + 'static,
{
    let (mut p2n_r, mut p2n_w) = pipe().expect("pipe parent->newcomer");
    let (mut n2p_r, mut n2p_w) = pipe().expect("pipe newcomer->parent");
    thread::scope(|s| {
        let newcomer = s.spawn(move || {
            Known::<T>::bootstrap(&mut p2n_r, &mut n2p_w)
                .expect("bootstrap newcomer")
                .expect("provider served bootstrap")
        });
        parent
            .gossip(&mut n2p_r, &mut p2n_w)
            .expect("serve bootstrap");
        newcomer.join().expect("join bootstrap thread")
    })
}

/// Live message counts for the single-set benchmarks (`batch_insert`,
/// `iter`, `redact`, `range_delta`, …), spanning three orders of magnitude.
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

/// Commit `n` unit payloads to `known` as one batch. `()` borsh-encodes to
/// zero bytes, so fixtures measure tree / clock / hashing work, not payload
/// serialization.
pub fn send_units(known: &Known<()>, n: usize) {
    let mut batch = known.batch();
    for _ in 0..n {
        batch.send(());
    }
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
/// `left` is a fresh [`Known::seed`]; `right` is a genuine disjoint peer minted
/// from it via [`bootstrap_fork`], so their parties are disjoint (the
/// precondition for `gossip`). The shared prefix is inserted before the
/// split; the `differing` messages and `redacted` deletions are applied
/// independently to each side after it.
pub fn build(cell: Cell) -> (Known<()>, Known<()>) {
    let Cell {
        common,
        differing,
        redacted,
    } = cell;

    let mut left: Known<()> = Known::seed();
    send_units(&left, common);
    // The shared prefix's keys, for carving the redaction blocks; order is
    // immaterial (the blocks only need to be disjoint and deterministic, and
    // the snapshot iterates in a stable order).
    let shared: Vec<Key> = left.snapshot().iter().map(|(k, _, _)| k).collect();

    let right = bootstrap_fork(&mut left);
    send_units(&left, differing);
    send_units(&right, differing);

    if redacted > 0 {
        // Disjoint blocks: each side forgets a distinct slice of the shared
        // prefix, so the other must honor `redacted` deletions it never made.
        // `cells` guarantees `common >= 2 * redacted`, so the slices don't
        // overlap and are in bounds.
        let mut batch = left.batch();
        for key in &shared[..redacted] {
            batch.redact(*key);
        }
        drop(batch);
        let mut batch = right.batch();
        for key in &shared[redacted..2 * redacted] {
            batch.redact(*key);
        }
    }

    (left, right)
}
