//! Publication preserves concurrent changes, notifications, and payload ownership.

use std::cell::{Cell, RefCell};
use std::convert::Infallible;
use std::rc::Rc;
use std::sync::{Arc, Barrier, mpsc};
use std::thread;
use std::time::Duration;

use before::Party;
use proptest::prelude::*;
use tokio::sync::watch;

use super::{Inner, OPTIMISTIC_ATTEMPTS};
use crate::message::Message;
use crate::tree::{Action, Tree};

/// Add one event to a tree using its owner's current identity.
fn insert(tree: &mut Tree<u64>, party: &Party, value: u64) {
    tree.act(party, [Action::Insert(Message::new(value))]);
}

thread_local! {
    /// Interleave a local commit after candidate preparation, on this test's thread.
    static BEFORE_SWAP: RefCell<Option<Box<dyn FnMut()>>> = RefCell::new(None);
}

/// Give tests a deterministic conflict between preparing and swapping a root.
pub(super) fn before_swap() {
    BEFORE_SWAP.with_borrow_mut(|hook| {
        if let Some(hook) = hook {
            hook();
        }
    });
}

/// Detect synchronous lock deadlocks without blocking the test's own thread.
fn completes(test: impl FnOnce() + Send + 'static) {
    let (done, finished) = mpsc::channel();
    let worker = thread::spawn(move || {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(test));
        done.send(outcome).unwrap();
    });
    let outcome = finished
        .recv_timeout(Duration::from_secs(10))
        .expect("publication must complete without deadlocking");
    worker.join().unwrap();
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

/// A local payload whose destruction may read or write the replica.
#[derive(serde::Serialize)]
struct OnDrop {
    /// A local callback, excluded from the payload's wire representation.
    #[serde(skip)]
    callback: Box<dyn Fn() + Send + Sync>,
}

/// Encode a message that runs a local callback when released.
fn on_drop(callback: impl Fn() + Send + Sync + 'static) -> Message {
    Message::new(OnDrop {
        callback: Box::new(callback),
    })
}

impl Drop for OnDrop {
    /// Run the payload's callback when its last message handle is released.
    fn drop(&mut self) {
        (self.callback)();
    }
}

proptest! {
    /// Exhausting optimistic swaps reaches the exclusive fallback, preserving
    /// every intervening commit and applying the identity callback once.
    #[test]
    fn conflicts_reach_exclusive_publication(
        conflicts in 0usize..=OPTIMISTIC_ATTEMPTS,
        remote_events in 0usize..8,
        redact in any::<bool>(),
        reject in any::<bool>(),
    ) {
        completes(move || {
            let mut party = Party::seed();
            let remote = party.fork();
            let prior = Tree::new();
            let mut incoming = prior.clone();
            for i in 0..remote_events { insert(&mut incoming, &remote, i as u64); }
            let sender = watch::Sender::new(Inner::new(party, prior.clone()));
            let receiver = Rc::new(RefCell::new(sender.subscribe()));
            let live = Rc::new(RefCell::new(prior.clone()));
            let attempts = Rc::new(Cell::new(0));
            BEFORE_SWAP.with_borrow_mut(|hook| {
                let sender = sender.clone();
                let live = live.clone();
                let attempts = attempts.clone();
                let receiver = receiver.clone();
                *hook = Some(Box::new(move || {
                    let attempt = attempts.get();
                    attempts.set(attempt + 1);
                    if attempt < conflicts {
                        Inner::commit(&sender, |inner| {
                            insert(&mut inner.tree, inner.party.as_ref().unwrap(), 100 + attempt as u64);
                            if redact {
                                let path = crate::tree::typed::Path::for_leaf(inner.tree.latest());
                                inner.tree.act(inner.party.as_ref().unwrap(), [Action::Forget(path)]);
                            }
                            true
                        });
                        *live.borrow_mut() = Inner::snapshot(&sender);
                        receiver.borrow_mut().borrow_and_update();
                    }
                }));
            });
            let gate = sender.borrow().commit_gate.clone();
            let mut called = 0;
            let result = Inner::publish(&sender, &prior, &incoming, |_| {
                called += 1;
                assert_eq!(gate.try_read().is_err(), conflicts == OPTIMISTIC_ATTEMPTS,
                    "only exhausted retries should hold the gate exclusively");
                if reject { Err(()) } else { Ok(()) }
            });
            BEFORE_SWAP.with_borrow_mut(|hook| *hook = None);
            assert_eq!(called, 1);
            assert_eq!(attempts.get(), (conflicts + 1).min(OPTIMISTIC_ATTEMPTS));
            let mut expected = live.borrow().clone();
            if !reject { expected.join(incoming); }
            assert_eq!(result.is_err(), reject);
            assert_eq!(Inner::snapshot(&sender), expected);
            let changed = expected != *live.borrow();
            assert_eq!(receiver.borrow().has_changed().unwrap(), !reject && changed);
        });
    }

    /// Optimistic and exclusive publication both retain local changes made
    /// after the session snapshot, and notify exactly when state changes.
    #[test]
    fn publication_preserves_concurrent_changes(
        local_events in 0usize..8,
        remote_events in 0usize..8,
        redact_local in any::<bool>(),
        redact_remote in any::<bool>(),
        exclusive in any::<bool>(),
    ) {
        let mut local = Party::seed();
        let mut prior = Tree::new();
        insert(&mut prior, &local, 0);
        let original = prior.latest().clone();
        let remote = local.fork();
        let mut incoming = prior.clone();
        let mut live = prior.clone();
        for i in 0..local_events { insert(&mut live, &local, 100 + i as u64); }
        for i in 0..remote_events { insert(&mut incoming, &remote, 200 + i as u64); }
        if redact_local {
            live.act(&local, [Action::Forget(crate::tree::typed::Path::for_leaf(&original))]);
        }
        if redact_remote {
            incoming.act(&remote, [Action::Forget(crate::tree::typed::Path::for_leaf(&original))]);
        }
        let mut expected = live.clone();
        expected.join(incoming.clone());
        let changed = expected != live;
        let sender = watch::Sender::new(Inner::new(local, live));
        let mut receiver = sender.subscribe();
        let gate = sender.borrow().commit_gate.clone();
        let mut called = 0;
        let mut update = |_: &mut Inner<u64>| {
            called += 1;
            Ok::<_, Infallible>(())
        };
        if exclusive {
            Inner::publish_exclusive(&sender, &incoming, &gate, &mut update).unwrap();
        } else {
            Inner::publish(&sender, &prior, &incoming, &mut update).unwrap();
        }
        prop_assert_eq!(called, 1);
        prop_assert_eq!(&sender.borrow().tree, &expected);
        prop_assert_eq!(receiver.has_changed().unwrap(), changed);

        // Publishing the same state again must not cause an echo notification.
        receiver.borrow_and_update();
        Inner::publish(&sender, &expected, &expected, |_| Ok::<_, Infallible>(())).unwrap();
        prop_assert!(!receiver.has_changed().unwrap());
    }

    /// A stale snapshot cannot publish content or an identity update; the
    /// candidate stays available for rebasing, including across empty trees.
    #[test]
    fn stale_swaps_leave_state_and_callbacks_untouched(empty in any::<bool>()) {
        let party = Party::seed();
        let mut prior = Tree::new();
        if !empty { insert(&mut prior, &party, 0); }
        let mut live = prior.clone();
        insert(&mut live, &party, 1);
        if empty {
            let inserted = live.latest().clone();
            live.act(&party, [Action::Forget(crate::tree::typed::Path::for_leaf(&inserted))]);
        }
        let sender = watch::Sender::new(Inner::new(party, live.clone()));
        let receiver = sender.subscribe();
        let mut candidate = prior.clone();
        let mut called = false;
        let mut update = |_: &mut Inner<u64>| {
            called = true;
            Ok::<_, Infallible>(())
        };
        let gate = sender.borrow().commit_gate.clone();
        let result = {
            let _hold = gate.read().unwrap();
            Inner::try_swap(&sender, &prior, &mut candidate, &mut update)
        };
        prop_assert!(result.is_none());
        prop_assert!(!called);
        prop_assert_eq!(candidate, prior);
        prop_assert_eq!(&sender.borrow().tree, &live);
        prop_assert!(!receiver.has_changed().unwrap());
    }

    /// A rejected identity update publishes neither the candidate tree nor a
    /// notification, on either the optimistic or exclusive path.
    #[test]
    fn rejected_updates_do_not_publish(exclusive in any::<bool>(), events in 1usize..8) {
        let party = Party::seed();
        let prior = Tree::new();
        let mut incoming = prior.clone();
        for i in 0..events { insert(&mut incoming, &party, i as u64); }
        let sender = watch::Sender::new(Inner::new(party, prior.clone()));
        let receiver = sender.subscribe();
        let gate = sender.borrow().commit_gate.clone();
        let result = if exclusive {
            Inner::publish_exclusive(&sender, &incoming, &gate, &mut |_| Err::<(), _>("overlap"))
        } else {
            Inner::publish(&sender, &prior, &incoming, |_| Err::<(), _>("overlap"))
        };
        prop_assert_eq!(result, Err("overlap"));
        prop_assert_eq!(&sender.borrow().tree, &prior);
        prop_assert!(!receiver.has_changed().unwrap());
    }

    /// Competing gossip publishers and local commits all finish, and their
    /// combined result matches a join of the states actually committed.
    #[test]
    fn concurrent_publication_keeps_every_update(
        remote_events in prop::collection::vec(1usize..12, 2..5),
        local_events in 1usize..20,
        exclusive in any::<bool>(),
    ) {
        completes(move || {
            let mut local = Party::seed();
            let prior = Tree::new();
            let incoming: Vec<_> = remote_events.into_iter().map(|events| {
                let remote = local.fork();
                let mut tree = prior.clone();
                for i in 0..events { insert(&mut tree, &remote, i as u64); }
                tree
            }).collect();
            let mut expected = prior.clone();
            let sender = watch::Sender::new(Inner::new(local, prior.clone()));
            let start = Arc::new(Barrier::new(incoming.len() + 1));
            thread::scope(|scope| {
                for tree in &incoming {
                    let sender = &sender;
                    let prior = &prior;
                    let start = &start;
                    scope.spawn(move || {
                        start.wait();
                        if exclusive {
                            let gate = sender.borrow().commit_gate.clone();
                            Inner::publish_exclusive(sender, tree, &gate, &mut |_| Ok::<_, Infallible>(())).unwrap();
                        } else {
                            Inner::publish(sender, prior, tree, |_| Ok::<_, Infallible>(())).unwrap();
                        }
                    });
                }
                start.wait();
                for i in 0..local_events {
                    Inner::commit(&sender, |inner| {
                        insert(&mut inner.tree, inner.party.as_ref().unwrap(), i as u64);
                        if i + 1 == local_events {
                            // These stamps include any gossip already published;
                            // replaying the sends in isolation would mint others.
                            expected = inner.tree.clone();
                        }
                        true
                    });
                    thread::yield_now();
                }
            });
            for tree in incoming { expected.join(tree); }
            assert_eq!(Inner::snapshot(&sender), expected);
        });
    }

    /// Releasing a redacted payload after either publication path permits a
    /// reentrant read and commit; its new message survives the outer commit.
    #[test]
    fn publication_releases_payloads_after_unlocking(exclusive in any::<bool>()) {
        completes(move || {
            let sender = watch::Sender::new(Inner::new(Party::seed(), Tree::<OnDrop>::new()));
            let prior = Inner::snapshot(&sender);
            let callback_sender = sender.clone();
            Inner::commit(&sender, |inner| {
                let message = on_drop(move || {
                    assert!(Inner::snapshot(&callback_sender).is_empty());
                    Inner::commit(&callback_sender, |inner| {
                        inner.tree.act(inner.party.as_ref().unwrap(), [Action::Insert(on_drop(|| {}))]);
                        true
                    });
                });
                inner.tree.act(inner.party.as_ref().unwrap(), [Action::Insert(message)]);
                true
            });
            // Only the published tree retains the payload. The incoming state
            // has observed and redacted it since this session's empty snapshot.
            let mut incoming = Inner::snapshot(&sender);
            {
                let inner = sender.borrow();
                let path = crate::tree::typed::Path::for_leaf(incoming.latest());
                incoming.act(inner.party.as_ref().unwrap(), [Action::Forget(path)]);
            }
            if exclusive {
                let gate = sender.borrow().commit_gate.clone();
                Inner::publish_exclusive(&sender, &incoming, &gate, &mut |_| Ok::<_, Infallible>(())).unwrap();
            } else {
                Inner::publish(&sender, &prior, &incoming, |_| Ok::<_, Infallible>(())).unwrap();
            }
            assert_eq!(Inner::snapshot(&sender).len(), 1);
        });
    }
}

/// Excluding competing writers leaves snapshot readers free to proceed.
#[test]
fn exclusive_gate_allows_snapshots() {
    completes(|| {
        let sender = watch::Sender::new(Inner::new(Party::seed(), Tree::<u64>::new()));
        let gate = sender.borrow().commit_gate.clone();
        let _hold = gate.write().unwrap();
        let reader = thread::spawn(move || assert!(Inner::snapshot(&sender).is_empty()));
        reader.join().unwrap();
    });
}

/// An unwinding fallback leaves the candidate unpublished and releases its
/// exclusive gate, allowing a later commit to proceed despite lock poisoning.
#[test]
fn exclusive_publication_unwind_allows_later_commits() {
    completes(|| {
        let mut party = Party::seed();
        let remote = party.fork();
        let prior = Tree::new();
        let mut incoming = prior.clone();
        insert(&mut incoming, &remote, 0);
        let sender = watch::Sender::new(Inner::new(party, prior.clone()));
        BEFORE_SWAP.with_borrow_mut(|hook| {
            let sender = sender.clone();
            *hook = Some(Box::new(move || {
                Inner::commit(&sender, |inner| {
                    insert(&mut inner.tree, inner.party.as_ref().unwrap(), 1);
                    true
                });
            }));
        });
        let gate = sender.borrow().commit_gate.clone();
        let mut expected = None;
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            Inner::publish(
                &sender,
                &prior,
                &incoming,
                |inner| -> Result<(), Infallible> {
                    assert!(
                        gate.try_read().is_err(),
                        "the fallback holds the gate exclusively"
                    );
                    expected = Some(inner.tree.clone());
                    panic!("publication callback unwinds");
                },
            )
            .unwrap();
        }));
        BEFORE_SWAP.with_borrow_mut(|hook| *hook = None);
        assert!(outcome.is_err());
        assert_eq!(Inner::snapshot(&sender), expected.unwrap());
        Inner::commit(&sender, |inner| {
            insert(&mut inner.tree, inner.party.as_ref().unwrap(), 2);
            true
        });
        assert_eq!(Inner::snapshot(&sender).len(), OPTIMISTIC_ATTEMPTS + 1);
    });
}
