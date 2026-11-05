//! How much work a session does on the in-memory tree's branches and their
//! memos, on fixed sessions.
//!
//! A session reassembles each scope it explores, and publishing its result
//! computes every memo a reassembled node lacks: its hash, its version
//! bounds, and the largest encoded version beneath it. On small sessions
//! that work is a large share of the session's time, and unlike time it is
//! deterministic, so this suite counts it ([`memo_work`]): a change to how a
//! session reassembles or how a node computes its memos moves a committed
//! number.
//!
//! Each session runs both peers in their own tasks on a current-thread
//! runtime over an in-memory connection, as `tests/transport_calls.rs`
//! runs its fixtures, and the counts cover both sides, their commits
//! included.

use rumors::DEFAULT_SYNC_MEMORY_BUDGET;
use rumors::testing::{MemoWork, memo_work, memo_work_reset, memory_connection};
use rumors_testkit::bench::session::{Shape, pair};

/// Bytes each direction of the connection buffers: more than any fixture
/// sends in one direction, so no write waits for its reader.
const CAPACITY: usize = 4 << 20;

/// Reconcile the pair `shape` describes once, each peer in its own task,
/// and return the work both sides did on tree nodes, from the session's
/// start through both commits.
///
/// Building the pair runs the peers' own joining sessions, before counting
/// starts.
fn session(shape: Shape) -> MemoWork {
    let (a, b) = pair(shape, DEFAULT_SYNC_MEMORY_BUDGET);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("build a current-thread runtime");
    let (a_end, b_end) = memory_connection(CAPACITY);
    memo_work_reset();
    runtime.block_on(async {
        let a = tokio::spawn(async move { a.gossip_once(a_end.read, a_end.write).await });
        let b = tokio::spawn(async move { b.gossip_once(b_end.read, b_end.write).await });
        for side in [a, b] {
            side.await
                .expect("a peer's task completes")
                .expect("an in-memory session completes");
        }
    });
    memo_work()
}

/// The fixed sessions, each a name and its pair: identical replicas, one
/// and a hundred differences beside a shared set, a catch-up, two disjoint
/// sets, and a shared set of which one side has redacted a tenth.
fn fixtures() -> [(&'static str, Shape); 6] {
    let shared = 10_000;
    [
        ("identical", Shape::overlapping(shared, 100)),
        (
            "one difference",
            Shape {
                shared,
                a: 1,
                ..Shape::catch_up(0)
            },
        ),
        (
            "hundred differences",
            Shape {
                shared,
                a: 50,
                b: 50,
                ..Shape::catch_up(0)
            },
        ),
        ("catch-up", Shape::catch_up(10_000)),
        ("disjoint", Shape::symmetric(5_000)),
        (
            "redactions",
            Shape {
                shared,
                ..Shape::catch_up(0)
            }
            .with_redactions(shared / 10, 0),
        ),
    ]
}

/// The fixed sessions' work on tree nodes equals the committed baseline.
///
/// The counts are deterministic: the fixtures' networks are seeded, the
/// runtime polls its tasks in a fixed order, and nothing a session builds
/// or computes depends on a clock. They move when how a session reassembles
/// its scopes or how a node computes its memos changes, or a fixture does,
/// and a commit that moves them says why.
#[test]
fn memo_work_matches_the_baseline() {
    let work = |branches, hashes, bounds_full, version_bytes_full| MemoWork {
        branches,
        hashes,
        bounds_full,
        version_bytes_full,
    };
    let baseline = [
        work(0, 0, 0, 0),
        work(4, 5, 4, 4),
        work(194, 307, 194, 194),
        work(2, 10_949, 949, 949),
        work(1_406, 12_254, 1_570, 1_570),
        work(522, 923, 522, 522),
    ];
    let measured: Vec<(&str, MemoWork)> = fixtures()
        .into_iter()
        .map(|(name, shape)| (name, session(shape)))
        .collect();
    let expected: Vec<(&str, MemoWork)> = fixtures()
        .into_iter()
        .map(|(name, _)| name)
        .zip(baseline)
        .collect();
    assert_eq!(measured, expected);
}
