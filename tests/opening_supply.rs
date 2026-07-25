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
use rumors::{Peer, Rumors};

use crate::common::gossip_snapshot::capture_gossip;
use crate::common::wire::{block_on, bootstrap_fork_async};

/// A peer seeded from a fixed RNG so the capture is deterministic.
fn seeded<T>() -> Rumors<T> {
    Peer::seed_rng(&mut SmallRng::seed_from_u64(0)).into_rumors()
}

/// A second message whose key shares its first byte with message `1`'s in
/// this staging (keys `b8 11` and `b8 bc`, found by search), so the two
/// sides dispute one root child.
///
/// The initiator holds both leaves, the
/// responder — forked between the two sends — only the first.
const DISPUTED_SIBLING_VALUE: u64 = 151;

/// First of three consecutive responder ballast values whose keys' first
/// bytes (`24`, `7a`, `b5`) avoid the disputed radix (`b8`) and make the
/// responder the larger set, so the disputed-subtree holder initiates.
const BALLAST_FROM: u64 = 100;

/// Count the frames whose semantic label starts with `label` in a rendered
/// wire capture, across both directions.
fn frames_labeled(capture: &str, label: &str) -> usize {
    capture
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix("frame "))
        .filter(|frame| {
            let (_, semantic) = frame.split_once(": ").expect("frame lines are labeled");
            semantic.starts_with(label)
        })
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
    let (a, b) = block_on(async {
        let a: Rumors<u64> = seeded();
        a.send(1);
        let b = bootstrap_fork_async(&a).await;
        a.send(DISPUTED_SIBLING_VALUE);
        let y = BALLAST_FROM;
        b.batch().send(y).send(y + 1).send(y + 2);
        (a, b)
    });

    // Fixture self-checks: one shared radix, disputed; the subtree holder
    // is the smaller set and initiates.
    let akeys: Vec<u8> = a
        .snapshot()
        .iter()
        .map(|(k, _, _)| k.as_bytes()[0])
        .collect();
    assert_eq!(akeys.len(), 2, "the initiator holds the sibling pair");
    assert_eq!(akeys.first(), akeys.last(), "the pair shares a root radix");
    let radix = akeys[0];
    assert_eq!(
        b.snapshot()
            .iter()
            .filter(|(k, _, _)| k.as_bytes()[0] == radix)
            .count(),
        1,
        "the responder holds exactly one leaf under the disputed radix"
    );
    assert!(
        a.snapshot().len() < b.snapshot().len(),
        "the disputed-subtree holder must initiate"
    );

    let expected: usize = a.snapshot().len() + b.snapshot().len() - 1;
    let capture = capture_gossip(a.clone(), b.clone());
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
