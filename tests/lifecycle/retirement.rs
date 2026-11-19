//! Concurrent retirement through many handles settles once and preserves the
//! unique writable identity.

use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use before::Party;
use futures::future::join_all;
use futures::{FutureExt, StreamExt};
use proptest::prelude::*;
use rumors::testing::{MEMORY_CAPACITY, memory_connection, run_to_quiescence};
use rumors::{Connection, Join, Peer, Retire};
use rumors_testkit::common::flaky::{
    DurableStore, FaultFeed, FlakyInMemoryBookmark, persisted_record,
};
use rumors_testkit::common::property::{FLEET_DIVISOR, SESSION_DIVISOR};
use rumors_testkit::common::seed::{NetworkSeed, network_seed};
use rumors_testkit::common::sim::assert_parties_partition;
use rumors_testkit::divided_proptest;
use tokio::io::{AsyncBufRead, AsyncRead, AsyncWrite, ReadBuf};

use crate::common::wire::{join_fork, join_fork_async};

/// A read half at the end of its stream, which records whether a session read
/// it.
struct DeadRead(Arc<AtomicBool>);

/// Report the end of the stream, recording the attempt.
impl AsyncRead for DeadRead {
    /// Read nothing, recording the attempt.
    fn poll_read(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        _: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        self.0.store(true, Ordering::SeqCst);
        Poll::Ready(Ok(()))
    }
}

/// Report the end of the stream, recording the attempt.
impl AsyncBufRead for DeadRead {
    /// Buffer nothing, recording the attempt.
    fn poll_fill_buf(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<&[u8]>> {
        self.0.store(true, Ordering::SeqCst);
        Poll::Ready(Ok(&[]))
    }

    /// Consume nothing, since nothing is buffered.
    fn consume(self: Pin<&mut Self>, _: usize) {}
}

/// One retiring handle's connection and fate.
#[derive(Clone, Copy, Debug)]
struct Retirer {
    /// Whether the counterparty serves this handle's retirement; otherwise
    /// its connection is dead.
    served: bool,
    /// Whether it is cancelled while a non-retiring handle still exists.
    cancel_waiting: bool,
    /// Whether it is cancelled once the generation is quiescent, after one
    /// poll in which no counterparty has run.
    cancel_quiescent: bool,
}

/// Retiring handles, the first of which is never cancelled, so that some
/// handle remains to settle the generation.
fn retirers() -> impl Strategy<Value = Vec<Retirer>> {
    prop::collection::vec((any::<bool>(), any::<bool>(), any::<bool>()), 1..6).prop_map(|drawn| {
        drawn
            .into_iter()
            .enumerate()
            .map(
                |(index, (served, cancel_waiting, cancel_quiescent))| Retirer {
                    served,
                    cancel_waiting: index > 0 && cancel_waiting,
                    cancel_quiescent: index > 0 && cancel_quiescent,
                },
            )
            .collect()
    })
}

/// How a retiring handle's call ended, without its connection's halves, so
/// that the counterparty serving an unused connection meets its end.
enum Settled {
    /// The peer retired; whether the connection came back reusable.
    Retired { reusable: bool },
    /// Every retirement failed; the handle the call returned.
    Remained(Peer<u64>),
}

/// A retiring handle's call: in flight, settled, or cancelled.
enum Slot<F> {
    /// The call is in flight.
    Waiting(Pin<Box<F>>),
    /// The call returned.
    Settled(Settled),
    /// The call was dropped before it returned.
    Cancelled,
}

/// Poll calls by hand, so that the test decides which make progress.
impl<F: Future<Output = Settled>> Slot<F> {
    /// Poll an in-flight call once, keeping its result if it returns.
    fn poll_once(&mut self) {
        if let Slot::Waiting(call) = self
            && let Poll::Ready(settled) =
                call.as_mut().poll(&mut Context::from_waker(Waker::noop()))
        {
            *self = Slot::Settled(settled);
        }
    }

    /// Drop an in-flight call; a returned one keeps its result.
    fn cancel(&mut self) {
        if let Slot::Waiting(_) = self {
            *self = Slot::Cancelled;
        }
    }
}

/// A connection half the retirement reads, of whichever kind.
type Read = Box<dyn AsyncBufRead + Unpin + Send>;
/// A connection half the retirement writes, of whichever kind.
type Write = Box<dyn AsyncWrite + Unpin + Send>;

proptest! {
    /// Handles retiring together wait for every ordinary handle, survive
    /// cancellation, and settle the generation exactly once.
    ///
    /// No retiring call returns while an ordinary handle exists. Once none
    /// does, the retiring handles take turns, and a dead connection's failure
    /// passes the turn to the next. If any surviving handle's counterparty
    /// serves it, every surviving call returns `Retired`, the counterparty's
    /// party alone covers the whole identity space, and a handle whose turn
    /// never came gets its connection back reusable. Otherwise every surviving
    /// call returns `Recovered` with a handle to the same identity, and none
    /// returns before every surviving handle has tried its dead connection;
    /// the returned handles form the next generation, which retires the same
    /// way. Cancellation lands while ordinary handles exist, and again once
    /// the generation is quiescent, where it can fall on the handle whose
    /// session is in flight, before its counterparty has replied.
    #[test]
    fn concurrent_retirements_settle_once(
        generations in prop::collection::vec((retirers(), 1usize..4), 1..4),
        network in network_seed(),
    ) {
        run_to_quiescence(async {
            let counterparty: Peer<u64> = network.builder().sync_memory_budget(0).seed();
            let mut live = join_fork_async(&counterparty).await;
            let identity = live.dangerously_alias_party();
            for (generation, (plan, holders)) in generations.into_iter().enumerate() {
                live.send(generation as u64).unwrap();
                let attempted: Vec<Arc<AtomicBool>> = plan.iter().map(|_| Arc::default()).collect();
                // The dead connections every surviving handle must have tried
                // before any call returns a handle.
                let tried: Arc<Mutex<Vec<Arc<AtomicBool>>>> = Arc::default();
                let mut serves = Vec::new();
                let mut slots: Vec<_> = plan
                    .iter()
                    .zip(&attempted)
                    .map(|(retirer, attempted)| {
                        let (read, write): (Read, Write) = if retirer.served {
                            let (near, far) = memory_connection(MEMORY_CAPACITY);
                            serves.push(counterparty.gossip_once(far.read, far.write));
                            (Box::new(near.read), Box::new(near.write))
                        } else {
                            (Box::new(DeadRead(attempted.clone())), Box::new(tokio::io::sink()))
                        };
                        let retiring = live.clone().retire(read, write);
                        let tried = tried.clone();
                        Slot::Waiting(Box::pin(async move {
                            match retiring.await {
                                Retire::Retired { connection } => Settled::Retired {
                                    reusable: matches!(connection, Connection::Reusable { .. }),
                                },
                                Retire::Recovered { peer, .. } => {
                                    for attempted in tried.lock().unwrap().iter() {
                                        assert!(
                                            attempted.load(Ordering::SeqCst),
                                            "a handle returned before every dead connection was tried"
                                        );
                                    }
                                    Settled::Remained(peer)
                                }
                                Retire::Declined { .. } => {
                                    panic!("the counterparty serves plain gossip and never declines")
                                }
                            }
                        }))
                    })
                    .collect();
                let mut keepers: Vec<_> = (1..holders).map(|_| live.clone()).collect();
                keepers.push(live);

                for slot in &mut slots {
                    slot.poll_once();
                    assert!(matches!(slot, Slot::Waiting(_)), "an ordinary handle still exists");
                }
                for (slot, retirer) in slots.iter_mut().zip(&plan) {
                    if retirer.cancel_waiting {
                        slot.cancel();
                    }
                }
                let survivors = |slots: &[Slot<_>]| -> Vec<Arc<AtomicBool>> {
                    slots
                        .iter()
                        .zip(&plan)
                        .zip(&attempted)
                        .filter(|((slot, retirer), _)| !retirer.served && !matches!(slot, Slot::Cancelled))
                        .map(|(_, attempted)| attempted.clone())
                        .collect()
                };
                *tried.lock().unwrap() = survivors(&slots);
                drop(keepers);

                // One poll each while no counterparty runs: a served handle
                // holding the turn waits on its counterparty's reply, before
                // the point of no return, so cancelling it returns the view.
                for slot in &mut slots {
                    slot.poll_once();
                }
                for (slot, retirer) in slots.iter_mut().zip(&plan) {
                    if retirer.cancel_quiescent {
                        slot.cancel();
                    }
                }
                *tried.lock().unwrap() = survivors(&slots);

                let served_survives = slots
                    .iter()
                    .zip(&plan)
                    .any(|(slot, retirer)| retirer.served && !matches!(slot, Slot::Cancelled));
                let in_flight: Vec<_> = slots
                    .iter_mut()
                    .enumerate()
                    .filter_map(|(index, slot)| match std::mem::replace(slot, Slot::Cancelled) {
                        Slot::Waiting(call) => Some(async move { (index, call.await) }),
                        other => {
                            *slot = other;
                            None
                        }
                    })
                    .collect();
                let (finished, _) = futures::join!(join_all(in_flight), join_all(serves));
                for (index, settled) in finished {
                    slots[index] = Slot::Settled(settled);
                }

                let mut returned = Vec::new();
                for ((slot, retirer), attempted) in slots.into_iter().zip(&plan).zip(&attempted) {
                    match slot {
                        Slot::Cancelled => {}
                        Slot::Waiting(_) => unreachable!("every surviving call was driven to its end"),
                        Slot::Settled(Settled::Retired { reusable }) => {
                            assert!(served_survives, "a peer retired only through a served handle");
                            if !retirer.served {
                                assert_eq!(
                                    reusable,
                                    !attempted.load(Ordering::SeqCst),
                                    "a dead connection comes back reusable exactly when unused"
                                );
                            } else {
                                assert!(reusable, "a served connection ends at a session boundary");
                            }
                        }
                        Slot::Settled(Settled::Remained(peer)) => {
                            assert!(!served_survives, "a served handle retires the peer");
                            returned.push(peer);
                        }
                    }
                }
                if served_survives {
                    assert!(returned.is_empty());
                    assert_parties_partition(vec![counterparty.dangerously_alias_party()], 0);
                    return;
                }
                for peer in &returned {
                    assert_eq!(peer.dangerously_alias_party(), identity);
                    assert_eq!(peer.snapshot().len(), generation + 1);
                }
                assert_parties_partition(
                    vec![counterparty.dangerously_alias_party(), identity.dangerously_alias()],
                    0,
                );
                live = returned.pop().expect("the first handle never is cancelled");
            }
        })
        .expect("retiring calls make progress once the ordinary handles drop");
    }
}

/// Poll `future` once with a waker that never fires.
fn poll_once<F: Future + ?Sized>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

/// How a waiting retirer's call ended when another handle's retirement was
/// cancelled at some poll.
#[derive(Debug)]
enum WaiterFate {
    /// The waiter's call returned `Retired`; whether its connection came back
    /// reusable.
    Retired { reusable: bool },
    /// The waiter's call returned a handle.
    Recovered(Peer<u64>),
    /// The served retirement finished before its cancellation point.
    ServedFinished,
}

/// Retire one handle of a joined peer over a served connection, cancelling
/// that call at its `cancel_at`-th poll, while a second handle waits for its
/// turn over a dead connection.
///
/// Returns the waiter's fate, the counterparty, the counterparty's party
/// before, and the retiree's identity.
fn cancel_served_at(
    network: NetworkSeed,
    cancel_at: usize,
) -> (WaiterFate, Peer<u64>, Party, Party) {
    let counterparty: Peer<u64> = network.builder().sync_memory_budget(0).seed();
    let retiree = join_fork(&counterparty);
    retiree.send(7).unwrap();
    let before = counterparty.dangerously_alias_party();
    let identity = retiree.dangerously_alias_party();
    let served = retiree.clone();
    let (near, far) = memory_connection(MEMORY_CAPACITY);
    let mut serve = Box::pin(counterparty.gossip_once(far.read, far.write));
    let mut waiter = Box::pin(retiree.retire(tokio::io::empty(), tokio::io::sink()));
    assert!(
        poll_once(waiter.as_mut()).is_pending(),
        "the served handle still exists"
    );
    let mut retiring = Some(Box::pin(served.retire(near.read, near.write)));
    let (mut polls, mut serve_done, mut served_finished) = (0, false, false);
    for _ in 0..MAX_CANCELLATION_POLLS {
        if let Some(call) = retiring.as_mut() {
            polls += 1;
            if polls == cancel_at {
                retiring = None;
            } else if let Poll::Ready(outcome) = poll_once(call.as_mut()) {
                assert!(
                    matches!(outcome, Retire::Retired { .. }),
                    "a served retirement retires: {outcome:?}"
                );
                served_finished = true;
                retiring = None;
            }
        }
        if let Poll::Ready(outcome) = poll_once(waiter.as_mut()) {
            while !serve_done {
                serve_done = poll_once(serve.as_mut()).is_ready();
            }
            let fate = match outcome {
                _ if served_finished => {
                    assert!(matches!(outcome, Retire::Retired { .. }));
                    WaiterFate::ServedFinished
                }
                Retire::Retired { connection } => WaiterFate::Retired {
                    reusable: matches!(connection, Connection::Reusable { .. }),
                },
                Retire::Recovered { peer, .. } => WaiterFate::Recovered(peer),
                Retire::Declined { .. } => {
                    panic!("the counterparty serves plain gossip and never declines")
                }
            };
            drop(serve);
            return (fate, counterparty, before, identity);
        }
        if !serve_done {
            serve_done = poll_once(serve.as_mut()).is_ready();
        }
    }
    panic!("no progress with the served call cancelled at poll {cancel_at}");
}

/// The most polls [`cancel_served_at`] gives its calls before failing.
const MAX_CANCELLATION_POLLS: usize = 100_000;

divided_proptest! {
    #![divisor(FLEET_DIVISOR)]

    /// Cancelling the turn holder's call at any poll settles the generation
    /// once, by which side of the point of no return the cancellation fell.
    ///
    /// The sweep cancels a served retirement at every poll, from the first
    /// until the poll after it finishes, while a second handle waits over a
    /// dead connection. Before the point of no return the waiter's turn
    /// follows and fails, so it gets a handle to the same identity, which with
    /// the counterparty's party partitions the identity space. After it, the
    /// waiter returns `Retired` with its unused connection reusable, and the
    /// counterparty holds the retiree's identity whole or not at all. Both
    /// sides of the point of no return occur in every sweep.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn cancelling_the_turn_holder_at_any_poll_settles_once(network in network_seed()) {
        let (mut retired, mut recovered) = (0, 0);
        for cancel_at in 1.. {
            let (fate, counterparty, before, identity) = cancel_served_at(network, cancel_at);
            let party = counterparty.dangerously_alias_party();
            match fate {
                WaiterFate::Retired { reusable } => {
                    retired += 1;
                    assert!(reusable, "poll {cancel_at}: an unused dead connection comes back reusable");
                    assert!(
                        party == Party::seed() || party == before,
                        "poll {cancel_at}: the counterparty holds part of the retiree's identity"
                    );
                }
                WaiterFate::Recovered(peer) => {
                    recovered += 1;
                    assert_eq!(peer.dangerously_alias_party(), identity, "poll {cancel_at}");
                    assert_eq!(party, before, "poll {cancel_at}: absorbed yet returned");
                    assert_parties_partition(vec![party, peer.dangerously_alias_party()], 0);
                }
                WaiterFate::ServedFinished => {
                    assert_eq!(party, Party::seed());
                    break;
                }
            }
        }
        assert!(retired > 0, "some cancellation falls after the point of no return");
        assert!(recovered > 0, "some cancellation falls before it");
    }
}

proptest! {
    /// A turn whose counterparty declines passes the turn on, and the
    /// declined handle's call maps by how the generation settles.
    ///
    /// Two handles of one peer retire; the first to take its turn meets a
    /// counterparty that is itself retiring, so both decline. When the second
    /// handle's counterparty serves, both calls return `Retired` with reusable
    /// connections, and the serving counterparty and the decliner partition
    /// the identity space. When the second handle's counterparty declines too,
    /// both calls return `Declined` with handles to the same identity.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn a_declined_turn_passes_on_and_maps_by_the_settlement(
        network in network_seed(),
        second_declines in any::<bool>(),
    ) {
        run_to_quiescence(async {
            let counterparty: Peer<u64> = network.builder().sync_memory_budget(0).seed();
            let retiree = join_fork_async(&counterparty).await;
            let decliner = join_fork_async(&counterparty).await;
            let second = join_fork_async(&counterparty).await;
            let identity = retiree.dangerously_alias_party();
            let first = retiree.clone();
            let (first_near, first_far) = memory_connection(MEMORY_CAPACITY);
            let (second_near, second_far) = memory_connection(MEMORY_CAPACITY);
            // The second handle enrolls first, so the first handle, enrolling
            // last, observes quiescence first and takes the first turn.
            let mut later = Box::pin(retiree.retire(second_near.read, second_near.write));
            assert!(poll_once(later.as_mut()).is_pending(), "the first handle still exists");
            // A serving counterparty keeps a second handle, for the party check;
            // a retiring one may not, or its retirement would wait on it.
            let serving = (!second_declines).then(|| second.clone());
            let second_side = async move {
                if second_declines {
                    Some(second.retire(second_far.read, second_far.write).await)
                } else {
                    second.gossip_once(second_far.read, second_far.write).await.expect("serving succeeds");
                    None
                }
            };
            let (first_out, declined, second_out, second_side) = futures::join!(
                first.retire(first_near.read, first_near.write),
                decliner.retire(first_far.read, first_far.write),
                later,
                second_side,
            );
            let Retire::Declined { peer: decliner, .. } = declined else {
                panic!("a retiring counterparty declines: {declined:?}");
            };
            if second_declines {
                let Some(Retire::Declined { peer: second, .. }) = second_side else {
                    panic!("the second counterparty declines");
                };
                let handles: Vec<_> = [first_out, second_out]
                    .into_iter()
                    .map(|out| match out {
                        Retire::Declined { peer, .. } => peer,
                        other => panic!("every turn declined: {other:?}"),
                    })
                    .collect();
                for handle in &handles {
                    assert_eq!(handle.dangerously_alias_party(), identity);
                }
                assert_parties_partition(
                    vec![
                        counterparty.dangerously_alias_party(),
                        identity.dangerously_alias(),
                        decliner.dangerously_alias_party(),
                        second.dangerously_alias_party(),
                    ],
                    0,
                );
            } else {
                for out in [first_out, second_out] {
                    assert!(
                        matches!(out, Retire::Retired { connection: Connection::Reusable { .. } }),
                        "a declined turn retires with its peer: {out:?}"
                    );
                }
                assert_parties_partition(
                    vec![
                        counterparty.dangerously_alias_party(),
                        decliner.dangerously_alias_party(),
                        serving.expect("a serving counterparty keeps a handle").dangerously_alias_party(),
                    ],
                    0,
                );
            }
        })
        .expect("declined turns make progress");
    }

    /// Once every retiring call is cancelled and every other handle dropped,
    /// the replica's observers drain and end.
    ///
    /// The calls are cancelled either while an ordinary handle still exists,
    /// before quiescence, or after it, where the first call holds the turn
    /// mid-session. Either way, nothing the cancelled calls left behind keeps
    /// the replica alive.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn cancelling_every_retirer_then_dropping_the_peer_ends_its_observers(
        network in network_seed(),
        retirers in 1usize..4,
        quiescent in any::<bool>(),
    ) {
        run_to_quiescence(async {
            let peer: Peer<u64> = network.builder().sync_memory_budget(0).seed();
            peer.send(1).unwrap();
            let mut observer = peer.unordered_messages();
            let keeper = (!quiescent).then(|| peer.clone());
            // The far ends stay open, so a session in flight waits for them.
            let mut ends: Vec<_> = (0..retirers).map(|_| memory_connection(MEMORY_CAPACITY)).collect();
            let mut calls: Vec<_> = ends
                .iter_mut()
                .map(|(near, _)| Box::pin(peer.clone().retire(&mut near.read, &mut near.write)))
                .collect();
            drop(peer);
            for call in &mut calls {
                assert!(poll_once(call.as_mut()).is_pending(), "no counterparty ever replies");
            }
            drop(calls);
            drop(keeper);
            let mut drained = 0;
            while observer.next().await.is_some() {
                drained += 1;
            }
            assert_eq!(drained, 1, "the observer drains the message before it ends");
        })
        .expect("the observer ends once every handle is gone");
    }
}

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// On a multi-threaded runtime, handles retiring and handles dropping
    /// from other tasks, all at once, settle the generation once.
    ///
    /// When one retirer's counterparty serves, no call returns a handle and
    /// the counterparty alone holds the identity space; otherwise every
    /// retirer gets a handle to the same identity, which with the
    /// counterparty's party partitions the space.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn concurrent_retirement_settles_once_across_threads(
        network in network_seed(),
        retirers in 1usize..6,
        droppers in 0usize..4,
        served in any::<bool>(),
    ) {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .build()
            .unwrap();
        runtime.block_on(async move {
            let counterparty: Peer<u64> = network.builder().sync_memory_budget(0).seed();
            let retiree = join_fork_async(&counterparty).await;
            let identity = retiree.dangerously_alias_party();
            let mut calls = Vec::new();
            let mut serve = None;
            for index in 0..retirers {
                let handle = retiree.clone();
                let (read, write): (Read, Write) = if served && index == retirers / 2 {
                    let (near, far) = memory_connection(MEMORY_CAPACITY);
                    let serving = counterparty.clone();
                    serve = Some(tokio::spawn(async move {
                        drop(serving.gossip_once(far.read, far.write).await);
                    }));
                    (Box::new(near.read), Box::new(near.write))
                } else {
                    (Box::new(tokio::io::empty()), Box::new(tokio::io::sink()))
                };
                calls.push(tokio::spawn(async move {
                    match handle.retire(read, write).await {
                        Retire::Retired { .. } => None,
                        Retire::Recovered { peer, .. } | Retire::Declined { peer, .. } => Some(peer),
                    }
                }));
            }
            for _ in 0..droppers {
                let handle = retiree.clone();
                tokio::spawn(async move {
                    tokio::task::yield_now().await;
                    drop(handle);
                });
            }
            drop(retiree);
            let mut returned = Vec::new();
            for call in calls {
                returned.extend(call.await.unwrap());
            }
            if let Some(serve) = serve {
                serve.await.unwrap();
            }
            if served {
                assert!(returned.is_empty(), "a served retirement returns no handle");
                assert_parties_partition(vec![counterparty.dangerously_alias_party()], 0);
            } else {
                assert_eq!(returned.len(), retirers, "every failed retirer gets a handle");
                for peer in &returned {
                    assert_eq!(peer.dangerously_alias_party(), identity);
                }
                assert_parties_partition(
                    vec![counterparty.dangerously_alias_party(), identity.dangerously_alias()],
                    0,
                );
            }
        });
    }
}

proptest! {
    /// Handles of a bookmark-backed peer retiring together settle the
    /// generation once, as an unbookmarked peer's do.
    ///
    /// With a served retirer, every call returns `Retired`, the counterparty
    /// alone holds the identity space, and the bookmark no longer records the
    /// retiree's identity. Otherwise every call returns a bookmark-backed
    /// handle to the same identity, and a later session on one of them
    /// completes, taking the bookmark's lock and storing to it.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn concurrent_retirement_of_a_bookmarked_peer_settles_once(
        network in network_seed(),
        plan in prop::collection::vec(any::<bool>(), 1..4),
    ) {
        run_to_quiescence(async {
            let counterparty: Peer<u64> = network.builder().sync_memory_budget(0).seed();
            let store = DurableStore::default();
            let storage = FlakyInMemoryBookmark::new(
                store.clone(),
                Arc::new(Mutex::new(FaultFeed::new(vec![], vec![]))),
                0,
            );
            let (mut near, mut far) = memory_connection(MEMORY_CAPACITY);
            let (served, joined) = futures::join!(
                counterparty.gossip_once(&mut near.read, &mut near.write),
                Peer::<u64>::builder().bookmark(storage).join(&mut far.read, &mut far.write),
            );
            served.expect("serving a reliable join succeeds");
            let Join::Connected { peer: retiree, .. } = joined else {
                panic!("a reliable join with working storage connects");
            };
            let identity = retiree.dangerously_alias_party();
            let net = retiree.network();
            let mut serves = Vec::new();
            let calls: Vec<_> = plan
                .iter()
                .map(|&served| {
                    let (read, write): (Read, Write) = if served {
                        let (near, far) = memory_connection(MEMORY_CAPACITY);
                        serves.push(counterparty.gossip_once(far.read, far.write));
                        (Box::new(near.read), Box::new(near.write))
                    } else {
                        (Box::new(tokio::io::empty()), Box::new(tokio::io::sink()))
                    };
                    // Each call drops its connection's halves as it returns, so
                    // the counterparty serving an unused connection meets its end.
                    retiree.clone().retire(read, write).map(|outcome| match outcome {
                        Retire::Retired { .. } => None,
                        Retire::Recovered { peer, .. } => Some(peer),
                        Retire::Declined { .. } => {
                            panic!("the counterparty serves plain gossip and never declines")
                        }
                    })
                })
                .collect();
            drop(retiree);
            let (outcomes, _) = futures::join!(join_all(calls), join_all(serves));
            if plan.iter().any(|&served| served) {
                assert!(outcomes.iter().all(Option::is_none), "a served retirement returns no handle");
                assert_parties_partition(vec![counterparty.dangerously_alias_party()], 0);
                let recorded = persisted_record(&store);
                assert!(
                    recorded.get(&net).is_none_or(|clocks| clocks
                        .iter()
                        .all(|clock| clock.party().is_disjoint(&identity))),
                    "the retired identity leaves the bookmark"
                );
            } else {
                let mut returned: Vec<Peer<u64, FlakyInMemoryBookmark>> = outcomes
                    .into_iter()
                    .map(|outcome| outcome.expect("a dead connection fails before the point of no return"))
                    .collect();
                for peer in &returned {
                    assert_eq!(peer.dangerously_alias_party(), identity);
                }
                let live = returned.pop().expect("at least one handle retires");
                drop(returned);
                live.send(9).unwrap();
                let (mut near, mut far) = memory_connection(MEMORY_CAPACITY);
                let (local, remote) = futures::join!(
                    live.gossip_once(&mut near.read, &mut near.write),
                    counterparty.gossip_once(&mut far.read, &mut far.write),
                );
                local.expect("the returned handle's session completes");
                remote.expect("its counterparty's session completes");
                assert!(persisted_record(&store).contains_key(&net), "the session stored its checkpoint");
            }
        })
        .expect("bookmarked retirements make progress");
    }
}
