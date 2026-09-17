//! Measure protocol bytes across representative gossip sessions.
//!
//! Each cell records bytes written in both directions for a deterministic
//! session. Messages carry an empty byte string, whose one-byte CBOR encoding
//! leaves almost all measured bytes attributable to the protocol. The grid
//! varies three factors that matter to users: shared set size, the number of
//! changes reconciled together, and whether those changes insert or redact.
//!
//! The cells are deliberately sparse. The shared-history sweep checks the
//! small-divergence case quoted in the crate docs. The batching sweep checks
//! that larger reconciliations amortize the tree walk. The mixed sweep covers
//! insertions and redactions in the same session. Exact directional counts
//! make an unintended wire change visible without prescribing an internal
//! poll schedule.
//!
//! Every cell builds a fresh network. A bootstrap changes the parties that
//! stamp later messages, so reusing a corpus would change their versions and
//! paths. A `Rumors` clone shares the same replica and cannot serve as a copy.

mod common;

use bytes::Bytes;
use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use rumors::{Peer, Rumors, Version};

/// Seed used for every independently built network.
const NETWORK_SEED: u64 = 0x4f56_4552_4845_4144;

/// The message changes made after the replicas fork.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Shape {
    /// Messages held by both replicas before the fork.
    shared: usize,
    /// Messages inserted by the left replica after the fork.
    left_inserts: usize,
    /// Messages inserted by the right replica after the fork.
    right_inserts: usize,
    /// Shared messages redacted by the left replica.
    left_redacts: usize,
    /// Shared messages redacted by the right replica.
    right_redacts: usize,
}

/// Inspect and derive values from a session shape.
impl Shape {
    /// Describe one measured session.
    const fn new(
        shared: usize,
        left_inserts: usize,
        right_inserts: usize,
        left_redacts: usize,
        right_redacts: usize,
    ) -> Self {
        Self {
            shared,
            left_inserts,
            right_inserts,
            left_redacts,
            right_redacts,
        }
    }

    /// Count the changes the session must reconcile.
    const fn changes(self) -> usize {
        self.left_inserts + self.right_inserts + self.left_redacts + self.right_redacts
    }

    /// Compute the live set size after reconciliation.
    const fn converged_len(self) -> usize {
        self.shared + self.left_inserts + self.right_inserts
            - self.left_redacts
            - self.right_redacts
    }
}

/// One exact directional byte-count pin.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Cell {
    /// Session shape being measured.
    shape: Shape,
    /// Bytes written by the left replica.
    left_to_right: usize,
    /// Bytes written by the right replica.
    right_to_left: usize,
}

/// Inspect one measured cell.
impl Cell {
    /// Define one pinned grid cell.
    const fn new(shape: Shape, left_to_right: usize, right_to_left: usize) -> Self {
        Self {
            shape,
            left_to_right,
            right_to_left,
        }
    }

    /// Count bytes written in both directions.
    const fn total(self) -> usize {
        self.left_to_right + self.right_to_left
    }

    /// Whether this cell costs less per change than `other`.
    fn is_cheaper_per_change_than(self, other: Self) -> bool {
        self.total() * other.shape.changes() < other.total() * self.shape.changes()
    }
}

/// Build a deterministic pair with the requested divergence.
fn build(shape: Shape) -> (Rumors<Bytes>, Rumors<Bytes>) {
    assert!(
        shape.left_redacts + shape.right_redacts <= shape.shared,
        "redaction sets must be disjoint subsets of the shared set",
    );
    let left = Peer::seed_rng(&mut ChaChaRng::seed_from_u64(NETWORK_SEED))
        .sync_window_floor()
        .into_rumors();
    left.send_all((0..shape.shared).map(|_| Bytes::new()))
        .expect("send shared messages");
    let mut shared: Vec<Version> = left
        .snapshot()
        .iter()
        .map(|(version, _)| version.clone())
        .collect();
    shared.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));

    let right = common::wire::bootstrap_fork(&left);
    left.send_all((0..shape.left_inserts).map(|_| Bytes::new()))
        .expect("send left insertions");
    right
        .send_all((0..shape.right_inserts).map(|_| Bytes::new()))
        .expect("send right insertions");
    left.redact_all(shared.iter().take(shape.left_redacts));
    right.redact_all(shared.iter().rev().take(shape.right_redacts));
    (left, right)
}

/// Measure one cell and verify that the session converges as intended.
fn measure(shape: Shape) -> Cell {
    let (left, right) = build(shape);
    let (left_to_right, right_to_left) = common::count::session_wire_bytes(&left, &right);
    assert_eq!(
        left.snapshot().hash(),
        right.snapshot().hash(),
        "the measured session must converge",
    );
    assert_eq!(
        left.snapshot().len(),
        shape.converged_len(),
        "the left replica must have the expected live set",
    );
    assert_eq!(
        right.snapshot().len(),
        shape.converged_len(),
        "the right replica must have the expected live set",
    );
    Cell::new(shape, left_to_right, right_to_left)
}

/// Measure every cell before reporting any pin mismatch.
fn check(expected: &[Cell]) -> Vec<Cell> {
    let measured: Vec<_> = expected.iter().map(|cell| measure(cell.shape)).collect();
    for cell in &measured {
        eprintln!("{cell:?}");
    }
    assert_eq!(
        measured, expected,
        "the protocol-overhead grid changed; update these pins only for a deliberate wire change",
    );
    measured
}

/// Pin two-insertion costs at the shared-set sizes quoted in the crate docs.
#[test]
fn small_reconciliation_cost_at_documented_shared_sizes() {
    let measured = check(&[
        Cell::new(Shape::new(0, 1, 1, 0, 0), 188, 191),
        Cell::new(Shape::new(100, 1, 1, 0, 0), 2_543, 2_789),
        Cell::new(Shape::new(1_000, 1, 1, 0, 0), 7_246, 8_186),
        Cell::new(Shape::new(10_000, 1, 1, 0, 0), 7_537, 10_187),
        Cell::new(Shape::new(65_536, 1, 1, 0, 0), 8_333, 17_305),
    ]);
    assert!(
        measured
            .windows(2)
            .all(|pair| pair[0].total() < pair[1].total()),
        "small-reconciliation cost should grow across these sampled shared-set sizes",
    );
}

/// At 10,000 shared messages, the sampled insertion batches cost less per change.
#[test]
fn insertion_batches_reduce_cost_per_change() {
    let measured = check(&[
        Cell::new(Shape::new(10_000, 1, 1, 0, 0), 7_537, 10_187),
        Cell::new(Shape::new(10_000, 10, 10, 0, 0), 9_405, 25_900),
        Cell::new(Shape::new(10_000, 100, 100, 0, 0), 25_287, 146_858),
        Cell::new(Shape::new(10_000, 1_000, 1_000, 0, 0), 71_999, 314_132),
    ]);
    assert!(
        measured
            .windows(2)
            .all(|pair| pair[1].is_cheaper_per_change_than(pair[0])),
        "protocol bytes per insertion should fall across these sampled batch sizes",
    );
}

/// At 10,000 shared messages, the sampled redaction batches cost less per change.
#[test]
fn redaction_batches_reduce_cost_per_change() {
    let measured = check(&[
        Cell::new(Shape::new(10_000, 0, 0, 1, 0), 7_425, 9_299),
        Cell::new(Shape::new(10_000, 0, 0, 10, 0), 8_432, 17_911),
        Cell::new(Shape::new(10_000, 0, 0, 100, 0), 16_950, 95_138),
        Cell::new(Shape::new(10_000, 0, 0, 1_000, 0), 38_123, 264_202),
    ]);
    assert!(
        measured
            .windows(2)
            .all(|pair| pair[1].is_cheaper_per_change_than(pair[0])),
        "protocol bytes per redaction should fall across these sampled batch sizes",
    );
}

/// Mixed insertion and redaction sessions remain covered by exact directional pins.
#[test]
fn mixed_changes_have_stable_directional_costs() {
    check(&[
        Cell::new(Shape::new(10_000, 1, 0, 0, 1), 10_215, 7_521),
        Cell::new(Shape::new(10_000, 100, 0, 0, 100), 153_290, 23_522),
        Cell::new(Shape::new(10_000, 1_000, 0, 0, 1_000), 315_597, 45_527),
    ]);
}

/// The grid payload occupies exactly one encoded byte.
#[test]
fn empty_byte_string_has_one_byte_encoding() {
    let mut encoded = Vec::new();
    ciborium::into_writer(&Bytes::new(), &mut encoded).expect("encode empty byte string");
    assert_eq!(encoded, [0x40]);
}
