//! Question-ownership and lazy-establishment pins for the supply-only
//! opening.
//!
//! The opening's early supplies are Left-arm-only: they answer, and never
//! ask. The property that admits them — and that any rework of the opening
//! must preserve — is that **every scope has exactly one question-owner**:
//! for any node the descent visits, exactly one side ever asks a question
//! about it. These tests pin the observable consequences on the wire: a
//! divergent root child draws exactly one nonempty `Query` frame in the
//! whole session, and a session whose initiator holds no exclusive root
//! children never opens the opening-supply stream at all.

mod common;

use rand::SeedableRng;
use rand::rngs::SmallRng;
use rumors::{Peer, Rumors, Version};

use crate::common::gossip_snapshot::capture_gossip_returning;
use crate::common::shape::{ballast_avoiding, keep_only, path_radix, pool, send_pool};
use crate::common::wire::{block_on, bootstrap_fork_async};

/// A peer seeded from a fixed RNG so the capture is deterministic.
fn seeded<T: serde::de::DeserializeOwned + Send + Sync + 'static>() -> Rumors<T> {
    Peer::seed_rng(&mut SmallRng::seed_from_u64(0)).into_rumors()
}

/// Pool size for the one-byte path search staging the disputed sibling.
///
/// The search must hit one *specific* root radix, a direct-hit search with
/// mean 256, so the pool is sized well past it (`common::shape` explains
/// the search-and-redact staging; it is deterministic under the seeded
/// universe).
const RADIX_POOL: u64 = 2048;

/// Payload base and pool size for the responder ballast: a disjoint
/// payload range so pool cleanups never touch the other side's messages.
const BALLAST_POOL: (u64, u64) = (10_000, 16);

/// Count the frames whose semantic label starts with `label` in a rendered
/// wire capture, across both directions.
///
/// A frame's semantic is the comment on its signal line
/// (`<dense code> / <semantic> /`); the bare-digit code distinguishes
/// signal lines from every other annotated line in the rendering.
fn frames_labeled(capture: &str, label: &str) -> usize {
    capture
        .lines()
        .filter_map(|line| {
            let (code, rest) = line.trim_start().split_once(" / ")?;
            if code.is_empty() || !code.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            rest.strip_suffix(" /")
        })
        .filter(|semantic| semantic.starts_with(label))
        .count()
}

/// A divergent root child draws exactly one nonempty `Query` in the whole
/// session: its single question-owner is the responder's root reply.
///
/// The initiator holds two leaves under one root radix, the responder one
/// of them plus ballast elsewhere: the shared radix is disputed, so the
/// responder's opening reply queries it, the initiator answers with a
/// `Match` and a `Supply` one level down, and no other question about that
/// scope ever crosses. A second question-owner — the failure mode of
/// symmetrizing the whole merge-join instead of only its Left arm — would
/// double exactly this count.
#[test]
fn divergent_root_child_has_one_question_owner() {
    // Stage: message `1` exists before the fork, so both sides hold it; a
    // pool search lands a second initiator leaf under the same root radix
    // (the disputed child), and the responder keeps three ballast leaves
    // outside that radix, making it the larger set.
    let (a, b) = block_on(async {
        let a: Rumors<u64> = seeded();
        a.send(1);
        let b = bootstrap_fork_async(&a).await;
        (a, b)
    });
    let radix = path_radix(
        &a.snapshot()
            .iter()
            .find_map(|(v, m)| (*m == 1).then_some(v.clone()))
            .expect("message 1 is live"),
    );
    send_pool(&a, 2, RADIX_POOL);
    let sibling = pool(&a, 2, RADIX_POOL)
        .into_iter()
        .find(|(_, v)| path_radix(v) == radix)
        .map(|(value, _)| value)
        .expect("some pool leaf lands under message 1's radix");
    keep_only(&a, 2, RADIX_POOL, &[1, sibling]);
    let (ballast_from, ballast_pool) = BALLAST_POOL;
    send_pool(&b, ballast_from, ballast_pool);
    let ballast = ballast_avoiding(&pool(&b, ballast_from, ballast_pool), radix, 3);
    keep_only(&b, ballast_from, ballast_pool, &ballast);

    // Fixture self-checks: one shared radix, disputed; the subtree holder
    // is the smaller set and initiates. A leaf's path is the full-width
    // BLAKE3 hash of its version's canonical bytes.
    let path_radix = |version: &Version| blake3::hash(version.as_bytes()).as_bytes()[0];
    let apaths: Vec<u8> = a.snapshot().iter().map(|(v, _)| path_radix(v)).collect();
    assert_eq!(apaths.len(), 2, "the initiator holds the sibling pair");
    assert_eq!(
        apaths.first(),
        apaths.last(),
        "the pair shares a root radix"
    );
    let radix = apaths[0];
    assert_eq!(
        b.snapshot()
            .iter()
            .filter(|(v, _)| path_radix(v) == radix)
            .count(),
        1,
        "the responder holds exactly one leaf under the disputed radix"
    );
    assert!(
        a.snapshot().len() < b.snapshot().len(),
        "the disputed-subtree holder must initiate"
    );

    let expected: usize = a.snapshot().len() + b.snapshot().len() - 1;
    let (capture, a, b) = capture_gossip_returning(a, b);
    assert_eq!(
        a.snapshot().len(),
        expected,
        "the session converges on the union"
    );
    assert_eq!(b.snapshot().len(), expected, "both sides hold the union");

    assert_eq!(
        frames_labeled(&capture, "Query("),
        1,
        "the disputed root child has exactly one question-owner: the \
         responder's one root-level Query"
    );
    assert_eq!(
        frames_labeled(&capture, "QueryEmpty"),
        0,
        "nothing is requested whole: every one-sided child is an answerer \
         exclusive, supplied without being asked"
    );

    // The initiator's fan is covered by the responder's listing, so the
    // early set is empty and the opening-supply stream never exists on the
    // wire: lazy establishment, pinned from the capture's stream census.
    assert!(
        !capture.contains("Initiator stream 0 (height 31)"),
        "an empty early set opens no opening-supply stream"
    );
}
