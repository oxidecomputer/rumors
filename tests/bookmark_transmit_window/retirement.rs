//! Retirement failures distinguish recoverable ownership from possible handoff.

use before::Party;
use ciborium::value::Value;
use futures::channel::oneshot;
use proptest::prelude::*;
use rumors::error::Phase;
use rumors::{Error, Network, Retire};

use super::{GatedBookmark, Msg, Peer, Rumors, block_on, boot_from, gossip};
use crate::common::fault::{self, FaultPlan, Vanish};
use crate::common::flaky::{DurableStore, persisted_record};

/// A converged pair with current durable records at both endpoints.
struct Pair {
    /// The donor, consumed by retirement.
    donor: Rumors<Msg, GatedBookmark>,
    /// The recipient, which remains writable throughout the session.
    receiver: Rumors<Msg, GatedBookmark>,
    /// Control over the donor's durable writes.
    donor_bookmark: GatedBookmark,
    /// Control over the recipient's durable writes.
    receiver_bookmark: GatedBookmark,
}

/// Construct the state needed to isolate identity transfer from reconciliation.
impl Pair {
    /// Bootstrap and settle both records so the next writes concern donation.
    fn new(initial: usize) -> Self {
        block_on(async {
            let receiver_bookmark = GatedBookmark::new(DurableStore::default());
            let donor_bookmark = GatedBookmark::new(DurableStore::default());
            let receiver = Peer::<Msg>::seed()
                .bookmark(receiver_bookmark.clone())
                .await
                .unwrap()
                .into_rumors();
            receiver.send_all(0..initial as u64).unwrap();
            let donor = boot_from(&receiver, donor_bookmark.clone()).await;
            gossip(&donor, &receiver).await;
            Self {
                donor,
                receiver,
                donor_bookmark,
                receiver_bookmark,
            }
        })
    }
}

/// Join the identity regions actually present in the durable record.
fn recorded(bookmark: &GatedBookmark, network: Network) -> Option<Party> {
    persisted_record(&bookmark.store)
        .remove(&network)?
        .into_iter()
        .map(|clock| clock.into_parts().0)
        .reduce(|mut a, b| {
            a.join(b).unwrap();
            a
        })
}

/// Encode a protocol item independently to locate its byte boundary.
fn encoded_len(value: Value) -> usize {
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(&value, &mut bytes).unwrap();
    bytes.len()
}

/// Measure the donation's start and length, and the donor's total incoming bytes.
fn boundaries(initial: usize) -> (usize, usize, usize) {
    let Pair {
        donor, receiver, ..
    } = Pair::new(initial);
    let identity = donor.dangerously_alias_party();
    let frame = encoded_len(Value::Tag(
        rumors::tags::PARTY_TAG,
        Box::new(Value::Bytes(identity.as_bytes().to_vec())),
    ));
    // The final control item is the completion marker, a CBOR text ".".
    let completion = encoded_len(Value::Text(".".into()));
    block_on(async {
        let donor = donor.try_into_peer().await.unwrap();
        let (a, mut b) = rumors::link::memory();
        let (mut a, meter) = fault::metered(a);
        let (outcome, accepted) = tokio::join!(donor.retire(&mut a), receiver.gossip_once(&mut b));
        assert!(matches!(outcome, Retire::Retired));
        accepted.unwrap();
        assert_eq!(
            meter.streams_opened(),
            0,
            "converged retirement uses only control"
        );
        (meter.written() - frame - completion, frame, meter.read())
    })
}

/// Check the phase as well as the outcome so a misplaced cut cannot pass.
fn transport_phase(error: &Error<GatedBookmark>, expected: Phase) {
    assert!(
        matches!(error, Error::Transport(e) if e.context.phase == expected),
        "expected {expected:?}, got {error:?}"
    );
}

/// A failure location whose recovery outcome is fixed by the handoff contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Failure {
    /// The opening preamble has not been sent.
    Preamble,
    /// Reconciliation has not finished.
    Reconciliation,
    /// Sending the identity fails on its first write.
    DonationStart,
    /// Sending the identity fails after a strict prefix.
    DonationPrefix,
    /// The recipient's final confirmation is lost.
    Completion,
    /// Removing the donor's durable identity fails before transmission.
    Removal,
}

/// A persistence or wire boundary at which the retirement future is dropped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cancellation {
    /// The donor still records its identity.
    BeforeRemoval,
    /// Durable removal completed, but transmission has not begun.
    AfterRemoval,
    /// Only part of the identity frame reached the recipient.
    DonationPrefix,
    /// The recipient holds and records the identity, but has not confirmed it.
    AfterAbsorption,
}

proptest! {
    /// Wire stalls expire through the public deadline, recovering ownership only
    /// before donation starts and never leaving the interrupted link reusable.
    #[test]
    fn retirement_deadlines_respect_wire_handoff(
        initial in 0usize..6,
        offset in any::<proptest::sample::Index>(),
    ) {
        let (start, frame, _) = boundaries(initial);
        for stage in [Failure::Preamble, Failure::Reconciliation,
            Failure::DonationStart, Failure::DonationPrefix] {
            let pair = Pair::new(initial);
            let identity = pair.donor.dangerously_alias_party();
            let receiver_before = pair.receiver.dangerously_alias_party();
            let network = pair.donor.network();
            let cut = match stage {
                Failure::Preamble => 0,
                Failure::Reconciliation => start - 1,
                Failure::DonationStart => start,
                Failure::DonationPrefix => start + 1 + offset.index(frame - 1),
                _ => unreachable!(),
            };
            let (outcome, accepted) = block_on(async {
                let (a, mut b) = rumors::link::memory();
                let (mut a, stalled) = fault::stall_at(a, Vanish::OnControl { offset: cut });
                let deadline = std::sync::Mutex::new(Some(stalled));
                let donor = pair.donor.try_into_peer().await.unwrap()
                    .session_deadline(move || deadline.lock().unwrap().take().unwrap());
                let outgoing = async move {
                    let outcome = donor.retire(&mut a).await;
                    assert!(a.into_parts().session.poisoned());
                    outcome
                };
                tokio::join!(outgoing, pair.receiver.gossip_once(&mut b))
            });
            if matches!(stage, Failure::Preamble | Failure::Reconciliation) {
                let Retire::Recovered { peer, error: Error::DeadlineExceeded } = outcome else {
                    panic!("expiry before handoff must recover, stage {stage:?}: {outcome:?}");
                };
                assert_eq!(peer.dangerously_alias_party(), identity);
                let live = peer.session_deadline(std::future::pending).into_rumors();
                live.send(999).unwrap();
                block_on(gossip(&live, &pair.receiver));
                assert_eq!(live.snapshot().hash(), pair.receiver.snapshot().hash());
                assert_eq!(recorded(&pair.donor_bookmark, network), Some(identity));
            } else {
                assert!(matches!(outcome, Retire::Uncertain { error: Error::DeadlineExceeded }));
                assert!(recorded(&pair.donor_bookmark, network).is_none());
            }
            assert!(accepted.is_err(), "an incomplete donation cannot be accepted");
            assert_eq!(pair.receiver.dangerously_alias_party(), receiver_before);
            assert_eq!(recorded(&pair.receiver_bookmark, network), Some(receiver_before));
        }
    }

    /// Expiry during persistence preserves a recoverable donor before handoff,
    /// even after durable removal, and consumes it after the recipient absorbs it.
    #[test]
    fn retirement_deadlines_respect_durable_handoff(initial in 0usize..6) {
        for stage in [Cancellation::BeforeRemoval, Cancellation::AfterRemoval,
            Cancellation::AfterAbsorption] {
            let pair = Pair::new(initial);
            let identity = pair.donor.dangerously_alias_party();
            let mut expected_receiver = pair.receiver.dangerously_alias_party();
            let network = pair.donor.network();
            let gate = if stage == Cancellation::AfterAbsorption {
                &pair.receiver_bookmark
            } else {
                &pair.donor_bookmark
            };
            if stage == Cancellation::BeforeRemoval { gate.arm(); } else { gate.arm_after_write(); }
            let (outcome, _) = block_on(async {
                let (expire, deadline) = oneshot::channel();
                let deadline = std::sync::Mutex::new(Some(deadline));
                let donor = pair.donor.try_into_peer().await.unwrap()
                    .session_deadline(move || {
                        let deadline = deadline.lock().unwrap().take().unwrap();
                        async move { deadline.await.unwrap() }
                    });
                let (mut a, mut b) = rumors::link::memory();
                let outgoing = async move {
                    let outcome = {
                        let mut retiring = std::pin::pin!(donor.retire(&mut a));
                        tokio::select! {
                            biased;
                            () = gate.entered() => {},
                            outcome = &mut retiring => panic!("retirement bypassed the gate: {outcome:?}"),
                        }
                        expire.send(()).unwrap();
                        retiring.await
                    };
                    assert!(a.into_parts().session.poisoned());
                    // The recipient may be paused after absorption. Let it
                    // finish its durable write and observe the closed link.
                    gate.release();
                    outcome
                };
                tokio::join!(outgoing, pair.receiver.gossip_once(&mut b))
            });
            let expected_donor = if stage == Cancellation::BeforeRemoval {
                Some(identity.dangerously_alias())
            } else { None };
            assert_eq!(recorded(&pair.donor_bookmark, network), expected_donor);
            if stage == Cancellation::AfterAbsorption {
                assert!(matches!(outcome, Retire::Uncertain { error: Error::DeadlineExceeded }));
                expected_receiver.join(identity).unwrap();
            } else {
                let Retire::Recovered { peer, error: Error::DeadlineExceeded } = outcome else {
                    panic!("expiry before handoff must recover: {outcome:?}");
                };
                assert_eq!(peer.dangerously_alias_party(), identity);
                let live = peer.session_deadline(std::future::pending).into_rumors();
                live.send(999).unwrap();
                block_on(gossip(&live, &pair.receiver));
                assert_eq!(live.snapshot().hash(), pair.receiver.snapshot().hash());
                // Recovery must record ownership again, including when the
                // interrupted removal had already reached durable storage.
                assert_eq!(recorded(&pair.donor_bookmark, network), Some(identity));
            }
            assert_eq!(pair.receiver.dangerously_alias_party(), expected_receiver);
            assert_eq!(recorded(&pair.receiver_bookmark, network), Some(expected_receiver));
        }
    }

    /// Failures before handoff return a usable peer; donation and completion
    /// failures consume it, with durable and live identities matching the stage.
    #[test]
    fn retirement_errors_preserve_the_recovery_boundary(
        initial in 0usize..6,
        offset in any::<proptest::sample::Index>(),
    ) {
        let (start, frame, incoming) = boundaries(initial);
        for stage in [Failure::Preamble, Failure::Reconciliation, Failure::DonationStart,
            Failure::DonationPrefix, Failure::Completion, Failure::Removal] {
            let pair = Pair::new(initial);
            let identity = pair.donor.dangerously_alias_party();
            let receiver_before = pair.receiver.dangerously_alias_party();
            let network = pair.donor.network();
            let plan = match stage {
                Failure::Preamble => FaultPlan { write_cut: Some(0), ..FaultPlan::NONE },
                Failure::Reconciliation => FaultPlan { write_cut: Some(start - 1), ..FaultPlan::NONE },
                Failure::DonationStart => FaultPlan { write_cut: Some(start), ..FaultPlan::NONE },
                Failure::DonationPrefix => FaultPlan { write_cut: Some(start + 1 + offset.index(frame - 1)), ..FaultPlan::NONE },
                Failure::Completion => FaultPlan { read_cut: Some(incoming - 1), ..FaultPlan::NONE },
                Failure::Removal => { pair.donor_bookmark.fail_at(1); FaultPlan::NONE },
            };
            let (outcome, accepted) = block_on(async {
                let donor = pair.donor.try_into_peer().await.unwrap();
                let (a, mut b) = rumors::link::memory();
                let outgoing = async move {
                    let mut a = fault::faulty(a, plan);
                    donor.retire(&mut a).await
                };
                tokio::join!(outgoing, pair.receiver.gossip_once(&mut b))
            });
            if matches!(stage, Failure::Preamble | Failure::Reconciliation | Failure::Removal) {
                let Retire::Recovered { peer, error } = outcome else {
                    panic!("before handoff must recover, stage {stage:?}: {outcome:?}");
                };
                if stage == Failure::Removal {
                    assert!(matches!(error, Error::Bookmark(_)));
                }
                assert_eq!(peer.dangerously_alias_party(), identity);
                let live = peer.into_rumors();
                live.send(999).unwrap();
                assert_eq!(live.snapshot().len(), initial + 1);
                assert_eq!(recorded(&pair.donor_bookmark, network), Some(identity.dangerously_alias()));
            } else {
                let Retire::Uncertain { error } = outcome else {
                    panic!("possible handoff must consume the peer, stage {stage:?}: {outcome:?}");
                };
                transport_phase(&error, if stage == Failure::Completion { Phase::Completion } else { Phase::IdentityTransfer });
                assert!(recorded(&pair.donor_bookmark, network).is_none());
            }
            let mut expected = receiver_before;
            if stage == Failure::Completion {
                accepted.expect("only the recipient's outgoing confirmation was lost");
                expected.join(identity).unwrap();
            } else {
                assert!(accepted.is_err(), "the recipient cannot accept this failed donation");
            }
            assert_eq!(pair.receiver.dangerously_alias_party(), expected);
            assert_eq!(recorded(&pair.receiver_bookmark, network), Some(expected));
        }
    }

    /// Cancellation before removal leaves the donor's durable identity intact;
    /// afterwards it survives only if the recipient has durably absorbed it.
    #[test]
    fn retirement_cancellation_respects_durable_handoff(
        initial in 0usize..6,
        offset in any::<proptest::sample::Index>(),
    ) {
        let (start, frame, _) = boundaries(initial);
        for stage in [Cancellation::BeforeRemoval, Cancellation::AfterRemoval,
            Cancellation::DonationPrefix, Cancellation::AfterAbsorption] {
            let pair = Pair::new(initial);
            let identity = pair.donor.dangerously_alias_party();
            let mut expected_receiver = pair.receiver.dangerously_alias_party();
            let network = pair.donor.network();
            block_on(async {
                let donor = pair.donor.try_into_peer().await.unwrap();
                let (mut a, mut b) = rumors::link::memory();
                if stage == Cancellation::DonationPrefix {
                    // Drop the actual retirement future with a strict prefix of
                    // its party frame on the wire, not an injected I/O error.
                    let plan = FaultPlan {
                        vanish: Some(Vanish::OnControl { offset: start + 1 + offset.index(frame - 1) }),
                        ..FaultPlan::NONE
                    };
                    let (driven, received) = tokio::join!(
                        fault::drive(a, plan, async move |link: &mut fault::FaultyLink| donor.retire(link).await),
                        pair.receiver.gossip_once(&mut b),
                    );
                    assert!(driven.vanished(), "the retirement future must be cancelled");
                    assert!(received.is_err(), "the recipient cannot accept an incomplete party");
                } else {
                    let gate = if stage == Cancellation::AfterAbsorption { &pair.receiver_bookmark } else { &pair.donor_bookmark };
                    if stage == Cancellation::BeforeRemoval { gate.arm(); } else { gate.arm_after_write(); }
                    // Leaving this scope cancels both futures at the announced
                    // persistence boundary; their memory remains inspectable.
                    let mut retiring = std::pin::pin!(donor.retire(&mut a));
                    let mut receiving = std::pin::pin!(pair.receiver.gossip_once(&mut b));
                    tokio::select! {
                        biased;
                        () = gate.entered() => {},
                        outcome = &mut retiring => panic!("retirement finished before cancellation: {outcome:?}"),
                        outcome = &mut receiving => panic!("recipient bypassed the gate: {outcome:?}"),
                    }
                }
            });
            let expected_donor = if stage == Cancellation::BeforeRemoval { Some(identity.dangerously_alias()) } else { None };
            assert_eq!(recorded(&pair.donor_bookmark, network), expected_donor);
            if stage == Cancellation::AfterAbsorption {
                expected_receiver.join(identity).unwrap();
            }
            assert_eq!(pair.receiver.dangerously_alias_party(), expected_receiver);
            assert_eq!(recorded(&pair.receiver_bookmark, network), Some(expected_receiver));
        }
    }
}
