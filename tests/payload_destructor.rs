//! Payload destructors may read and change their own replica.

use rumors_testkit::common;

use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;

use rumors::Rumors;
use serde::{Deserialize, Serialize};

use common::wire::{bootstrap_fork, wire_gossip};
use rumors_testkit::common::property::{LOCAL_DIVISOR, SESSION_DIVISOR};
use rumors_testkit::common::seed::network_seed;
use rumors_testkit::divided_proptest;

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

/// The destructor ran once and its send survived the surrounding commit.
fn assert_replaced(replica: &Rumors<Payload>) {
    let snapshot = replica.snapshot();
    assert_eq!(snapshot.len(), 1);
    assert_eq!(snapshot.iter().next().unwrap().1.value, 2);
}

divided_proptest! {
    #![divisor(LOCAL_DIVISOR)]

    /// Redacting a resident message allows its destructor to read and write the replica.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn redact_allows_destructor_access(network in network_seed()) {
        completes(move || {
            let replica = network.peer().into_rumors();
            let version = replica.send(replaces_itself(&replica)).unwrap();
            replica.redact(&version);
            assert_replaced(&replica);
        });
    }

    /// A message inserted and redacted in one batch can access the replica when dropped.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn batch_allows_destructor_access(network in network_seed()) {
        completes(move || {
            let replica = network.peer().into_rumors();
            let mut version = replica.snapshot().latest().clone();
            version.tick(&replica.dangerously_alias_party());
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
}

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// A gossiped redaction permits a destructor's local send, which later converges.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn gossip_allows_destructor_access(network in network_seed()) {
        completes(move || {
            let replica = network.peer().into_rumors();
            let version = replica.send(replaces_itself(&replica)).unwrap();
            let other = bootstrap_fork(&replica);
            other.redact(&version);
            wire_gossip(&replica, &other);
            assert_replaced(&replica);
            wire_gossip(&replica, &other);
            assert!(replica.snapshot() == other.snapshot(), "replicas converge");
        });
    }
}
