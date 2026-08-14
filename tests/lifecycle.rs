//! End-to-end lifecycle pins for the V2 session epilogue and link
//! poisoning, at the public-API tier.
//!
//! The crate docs' "What a session promises" section states the contract
//! these tests hold the public API to: `Ok` certifies both replicas
//! committed and leaves the link reusable; `Err` and cancellation leave the
//! local replica unchanged and the link poisoned, failing every further
//! session on it fast; and the one irreducible residue (a lost final
//! epilogue marker) surfaces as the distinguished post-commit
//! [`rumors::Error::Epilogue`]. The in-crate unit tests (`src/tests.rs`)
//! pin the same mechanisms at exact byte boundaries with forged peers;
//! these are their integration-tier complements, driving whole sessions
//! between real replicas.

mod common;

use std::future::Future;
use std::pin::pin;
use std::task::Context;

use futures::task::noop_waker_ref;
use rumors::testing::{
    IoFault, IoFaultUnit, IoOperation, IoPlan, IoSide, run_to_quiescence, wrap_link,
};
use rumors::{Error, Peer, Rumors};

use crate::common::wire::{block_on, bootstrap_fork_async, wire_gossip};

/// Messages each side commits before a divergent session: enough that
/// reconciliation opens several data streams and takes many polls to
/// complete.
const DIVERGENT_MESSAGES: u64 = 32;

/// Polls granted to the joined in-flight sessions before they are dropped.
///
/// Each poll is asserted still pending, so the cancellation provably lands
/// mid-session; the reports separately prove wire traffic had begun and a
/// data stream was already open at each end: the descent itself, not just
/// the handshake, was in flight.
const MID_FLIGHT_POLLS: usize = 4;

/// Build a party-disjoint pair with [`DIVERGENT_MESSAGES`] unshared
/// messages committed on each side.
///
/// Construction is deterministic apart from the random network id (which
/// has a fixed wire length): versions derive from the bootstrap order and
/// message keys from `(version, payload)`, so two calls build pairs whose
/// gossip sessions are byte-for-byte the same size. The epilogue-residue
/// test's measure-then-replay rests on this.
async fn divergent_pair() -> (Rumors<u64>, Rumors<u64>) {
    let a: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
    let b = bootstrap_fork_async(&a).await;
    for v in 0..DIVERGENT_MESSAGES {
        a.send(v);
        b.send(1_000 + v);
    }
    (a, b)
}

/// A gossip session cancelled mid-descent poisons the link on both ends.
///
/// The next session attempt on the same link fails immediately with
/// [`Error::LinkPoisoned`] (no wire traffic, no hang, byte-counters
/// unchanged), and both replicas come through unharmed, converging fully
/// over a fresh link.
#[test]
fn a_session_cancelled_mid_descent_poisons_the_link() {
    let (a, b) = block_on(divergent_pair());
    let (a_link, b_link) = rumors::link::memory();
    let (mut a_link, a_report) = wrap_link(IoSide::Left, IoPlan::default(), a_link);
    let (mut b_link, b_report) = wrap_link(IoSide::Right, IoPlan::default(), b_link);

    {
        let mut cx = Context::from_waker(noop_waker_ref());
        let mut session =
            pin!(async { tokio::join!(a.gossip(&mut a_link), b.gossip(&mut b_link)) });
        for _ in 0..MID_FLIGHT_POLLS {
            assert!(
                session.as_mut().poll(&mut cx).is_pending(),
                "the joined sessions must still be in flight when cancelled"
            );
        }
        // Dropping the joined future here cancels both sessions mid-descent.
    }
    let (a_cut, b_cut) = (a_report.snapshot(), b_report.snapshot());
    assert!(
        a_cut.write_bytes > 0 && b_cut.write_bytes > 0,
        "the cancellation must land after wire traffic began on both ends"
    );
    assert!(
        a_cut.connects + a_cut.accepts > 0 && b_cut.connects + b_cut.accepts > 0,
        "the cancellation must land mid-descent: a data stream open at each end, \
         not merely a handshake in flight"
    );

    // Reuse fails fast on both ends. The closed-world harness itself is the
    // no-hang proof: the fail-fast resolves with no counterparty driving.
    let (a_before, b_before) = (a_report.snapshot(), b_report.snapshot());
    let retry = run_to_quiescence(a.gossip(&mut a_link)).expect("the fail-fast needs no peer");
    assert!(
        matches!(retry, Err(Error::LinkPoisoned)),
        "a poisoned link must fail A's next session fast, got {retry:?}"
    );
    let retry = run_to_quiescence(b.gossip(&mut b_link)).expect("the fail-fast needs no peer");
    assert!(
        matches!(retry, Err(Error::LinkPoisoned)),
        "a poisoned link must fail B's next session fast, got {retry:?}"
    );
    assert_eq!(
        a_report.snapshot(),
        a_before,
        "the fail-fast must perform no wire traffic on A's link"
    );
    assert_eq!(
        b_report.snapshot(),
        b_before,
        "the fail-fast must perform no wire traffic on B's link"
    );

    // The replicas themselves are unharmed: a fresh link converges the pair.
    wire_gossip(&a, &b);
    assert_eq!(a.snapshot().hash(), b.snapshot().hash());
    assert_eq!(a.snapshot().len(), 2 * DIVERGENT_MESSAGES as usize);
}

/// Immediate teardown after `Ok` is safe under the epilogue.
///
/// Each side drops its entire link the instant its own `Ok` lands, so the
/// slower side must conclude its session, through the peer's epilogue
/// marker, from bytes already buffered in the link, with the deferred
/// supply-closure path absorbing the peer's vanished stream supply. Both
/// sides conclude `Ok`, committed and converged.
#[test]
fn dropping_the_link_immediately_after_ok_is_clean() {
    let (a, b) = block_on(divergent_pair());
    let (a_link, b_link) = rumors::link::memory();
    let (a_out, b_out) = block_on(async {
        tokio::join!(
            async {
                let mut a_link = a_link;
                a.gossip(&mut a_link).await
                // A's whole link drops right here, at its own outcome.
            },
            async {
                let mut b_link = b_link;
                b.gossip(&mut b_link).await
            },
        )
    });
    a_out.expect("A's session");
    b_out.expect("B must conclude Ok off buffered bytes after A's teardown");
    assert_eq!(a.snapshot().hash(), b.snapshot().hash());
    assert_eq!(a.snapshot().len(), 2 * DIVERGENT_MESSAGES as usize);
}

/// One byte inside the epilogue boundary, the session is still pre-commit.
///
/// With B's read budget *two* bytes short of a clean session — one byte
/// before the lost-marker residue's cut — the withheld bytes are the tail
/// of reconciliation plus A's epilogue marker, so B fails before it
/// commits: a pre-commit error class (never [`Error::Epilogue`]), B's
/// replica byte-unchanged (snapshot hash equal to its pre-session value),
/// and B's link poisoned, failing the next session fast with
/// [`Error::LinkPoisoned`] and no wire traffic. Together with
/// [`a_lost_epilogue_marker_is_distinguished_and_post_commit`], one budget
/// unit apart, this brackets where a session commits relative to its wire
/// schedule: a drift in either direction fails one leg's assertion class.
#[test]
fn a_cut_before_the_epilogue_fails_pre_commit_and_unchanged() {
    // Probe: measure B's total incoming bytes across one clean divergent
    // session, exactly as the inside leg does.
    let clean_read_bytes = {
        let (a, b) = block_on(divergent_pair());
        let (a_link, b_link) = rumors::link::memory();
        let (mut b_link, report) = wrap_link(IoSide::Right, IoPlan::default(), b_link);
        let (a_out, b_out) = block_on(async {
            let mut a_link = a_link;
            tokio::join!(a.gossip(&mut a_link), b.gossip(&mut b_link))
        });
        a_out.expect("probe session A");
        b_out.expect("probe session B");
        report.snapshot().read_bytes
    };

    // Replay with B's reads failing two bytes short: the cut lands in the
    // reconciliation's tail, before B's commit point.
    let (a, b) = block_on(divergent_pair());
    let b_hash_before = b.snapshot().hash();
    let (a_link, b_link) = rumors::link::memory();
    let plan = IoPlan {
        fault: Some(IoFault {
            operation: IoOperation::Read,
            after: clean_read_bytes - 2,
            unit: IoFaultUnit::Bytes,
        }),
        ..IoPlan::default()
    };
    let (mut b_link, b_report) = wrap_link(IoSide::Right, plan, b_link);

    // B resolves on its own injected fault; A, still awaiting the epilogue
    // marker B never sends, is cancelled when the race concludes (its link
    // drops with it). B's link outlives the race for the poison probe.
    let b_out = block_on(async {
        let b_session = b.gossip(&mut b_link);
        let a_session = async {
            let mut a_link = a_link;
            a.gossip(&mut a_link).await
        };
        tokio::select! {
            biased;
            b_out = b_session => b_out,
            a_out = a_session => panic!(
                "A cannot conclude while B's marker is outstanding, got {a_out:?}"
            ),
        }
    });

    // Pre-commit: the failure is a session-failure class, never the
    // distinguished post-commit residue.
    assert!(
        !matches!(b_out, Ok(_) | Err(Error::Epilogue(_))),
        "a cut before the epilogue must fail pre-commit, got {b_out:?}"
    );
    // Unchanged: B holds exactly its own pre-session sends.
    assert_eq!(
        b.snapshot().hash(),
        b_hash_before,
        "a pre-commit failure must leave B's replica byte-unchanged"
    );
    assert_eq!(b.snapshot().len(), DIVERGENT_MESSAGES as usize);

    // Poisoned: the next session on B's link fails fast, no wire traffic.
    let before = b_report.snapshot();
    let retry = run_to_quiescence(b.gossip(&mut b_link)).expect("the fail-fast needs no peer");
    assert!(
        matches!(retry, Err(Error::LinkPoisoned)),
        "a poisoned link must fail B's next session fast, got {retry:?}"
    );
    assert_eq!(
        b_report.snapshot(),
        before,
        "the fail-fast must perform no wire traffic on B's link"
    );
}

/// The peer-committed-or-not residue is distinguished and post-commit.
///
/// With A's final epilogue marker withheld (B's read budget is one byte
/// short of a clean session, measured from a byte-identical probe run),
/// A's `Ok` still lands, while B fails with [`Error::Epilogue`] rather
/// than any pre-commit class, and B's replica nonetheless holds the fully
/// reconciled content. A cut landing anywhere earlier would fail these
/// assertions with a different error class and missing content, so the
/// test also validates its own budget placement.
#[test]
fn a_lost_epilogue_marker_is_distinguished_and_post_commit() {
    // Probe: measure B's total incoming bytes across one clean divergent
    // session. `divergent_pair` builds byte-identical universes, so the
    // replay below sees the same byte schedule.
    let clean_read_bytes = {
        let (a, b) = block_on(divergent_pair());
        let (a_link, b_link) = rumors::link::memory();
        let (mut b_link, report) = wrap_link(IoSide::Right, IoPlan::default(), b_link);
        let (a_out, b_out) = block_on(async {
            let mut a_link = a_link;
            tokio::join!(a.gossip(&mut a_link), b.gossip(&mut b_link))
        });
        a_out.expect("probe session A");
        b_out.expect("probe session B");
        assert!(
            report.snapshot().accepts >= 2,
            "the divergence must be deep enough to open several data streams, got {}",
            report.snapshot().accepts
        );
        report.snapshot().read_bytes
    };

    // Replay on an identically-built pair, with B's reads failing after
    // every byte but the last: the lost byte is A's epilogue marker, the
    // final byte a clean session delivers to B.
    let (a, b) = block_on(divergent_pair());
    let (a_link, b_link) = rumors::link::memory();
    let plan = IoPlan {
        fault: Some(IoFault {
            operation: IoOperation::Read,
            after: clean_read_bytes - 1,
            unit: IoFaultUnit::Bytes,
        }),
        ..IoPlan::default()
    };
    let (b_link, _report) = wrap_link(IoSide::Right, plan, b_link);
    let (a_out, b_out) = block_on(async {
        // Each future owns its link: A's teardown after its `Ok` surfaces
        // as end-of-stream to B if it wins the race against the injected
        // cut; either way B's marker read fails, post-commit.
        tokio::join!(
            async {
                let mut a_link = a_link;
                a.gossip(&mut a_link).await
            },
            async {
                let mut b_link = b_link;
                b.gossip(&mut b_link).await
            },
        )
    });

    // B's own marker went out before its failing read, so A's Ok lands:
    // from A's seat this is a fully certified session.
    a_out.expect("A's session concludes Ok; only its marker toward B is lost");
    // B committed before its epilogue, so the failure is the distinguished
    // post-commit residue, not a session-failure class.
    assert!(
        matches!(b_out, Err(Error::Epilogue(_))),
        "a lost final marker must surface as the post-commit Epilogue, got {b_out:?}"
    );
    // Post-commit means exactly this: B holds the reconciled content.
    assert_eq!(
        a.snapshot().hash(),
        b.snapshot().hash(),
        "B's replica must hold the reconciled content despite the Err"
    );
    assert_eq!(b.snapshot().len(), 2 * DIVERGENT_MESSAGES as usize);
}
