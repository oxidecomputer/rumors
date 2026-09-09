//! Payload destructors may read and change their own replica.

mod common;

use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;

use rumors::{Peer, Rumors, Version};
use serde::{Deserialize, Serialize};

use common::wire::{bootstrap_fork, wire_gossip};

/// The callback belongs to the local payload, not its serialized value.
#[derive(Clone, Serialize, Deserialize)]
struct Payload {
    value: u64,
    #[serde(skip)]
    on_drop: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl PartialEq for Payload {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl Eq for Payload {}

impl Drop for Payload {
    fn drop(&mut self) {
        if let Some(callback) = self.on_drop.take() {
            callback();
        }
    }
}

/// Replace this message with a new one when the commit releases it.
fn replaces_itself(replica: &Rumors<Payload>) -> Payload {
    let replica = replica.clone();
    Payload {
        value: 1,
        on_drop: Some(Arc::new(move || {
            assert!(replica.snapshot().is_empty());
            replica
                .send(Payload {
                    value: 2,
                    on_drop: None,
                })
                .unwrap();
        })),
    }
}

/// A lock deadlock blocks the worker thread, so detect it from another thread.
fn completes(test: impl FnOnce() + Send + 'static) {
    let (done, finished) = mpsc::channel();
    let worker = thread::spawn(move || {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(test));
        done.send(outcome).unwrap();
    });
    let outcome = finished
        .recv_timeout(Duration::from_secs(10))
        .expect("the commit and its payload destructor must finish");
    worker.join().unwrap();
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

/// Return the version without retaining a snapshot or payload handle.
fn only_version(replica: &Rumors<Payload>) -> Version {
    replica.snapshot().iter().next().unwrap().0.clone()
}

/// The destructor ran once and its send survived the surrounding commit.
fn assert_replaced(replica: &Rumors<Payload>) {
    let snapshot = replica.snapshot();
    assert_eq!(snapshot.len(), 1);
    assert_eq!(snapshot.iter().next().unwrap().1.value, 2);
}

/// Redacting a resident message allows its destructor to read and write the replica.
#[test]
fn redact_allows_destructor_access() {
    completes(|| {
        let replica = Peer::seed().into_rumors();
        replica.send(replaces_itself(&replica)).unwrap();
        replica.redact(&only_version(&replica));
        assert_replaced(&replica);
    });
}

/// A message inserted and redacted in one batch can access the replica when dropped.
#[test]
fn batch_allows_destructor_access() {
    completes(|| {
        let replica = Peer::seed().into_rumors();
        let mut version = replica.snapshot().latest().clone();
        version.tick(&replica.dangerously_alias_party().unwrap());
        replica
            .batch(|batch| {
                batch.send(replaces_itself(&replica))?;
                batch.redact(&version);
                Ok::<(), rumors::EncodeError>(())
            })
            .unwrap();
        assert_replaced(&replica);
    });
}

/// A gossiped redaction permits a destructor's local send, which later converges.
#[test]
fn gossip_allows_destructor_access() {
    completes(|| {
        let replica = Peer::seed().into_rumors();
        replica.send(replaces_itself(&replica)).unwrap();
        let other = bootstrap_fork(&replica);
        other.redact(&only_version(&other));
        wire_gossip(&replica, &other);
        assert_replaced(&replica);
        wire_gossip(&replica, &other);
        assert!(replica.snapshot() == other.snapshot(), "replicas converge");
    });
}
