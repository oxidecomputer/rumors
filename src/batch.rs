use borsh::BorshSerialize;
use tokio::sync::watch;

use crate::message::Message;
use crate::tree::Action;
use crate::{Inner, Key};

/// A batch of insertions and redactions against a [`Rumors`](crate::Rumors),
/// committed atomically.
///
/// Returned by [`send`](crate::Rumors::send),
/// [`redact`](crate::Rumors::redact), and [`batch`](crate::Rumors::batch) on
/// [`Rumors`](crate::Rumors). Dropping the batch commits it: the single-action
/// case reads as a plain call (`rumors.send(message);` commits at the end of
/// the statement), and chaining accumulates
/// (`rumors.batch().send(a).send(b).redact(key);`) into one commit.
///
/// Building a [`Batch`] holds no lock; committing locks the rumor set
/// momentarily.
///
/// Commit is the causal moment: a sent message's version dominates
/// everything this replica had observed when the batch committed, not when
/// the batch was built ([`Rumors::send`](crate::Rumors::send) states the
/// contract and its boundary). Because building holds no lock,
/// concurrent synchronization can land between building and committing,
/// and two batches carry no guaranteed causal relationship to one another
/// unless the application synchronizes them itself.
pub struct Batch<'a, T: Send + Sync> {
    inner: &'a watch::Sender<Inner<T>>,
    actions: Vec<Action<T>>,
}

impl<'a, T: Send + Sync> Batch<'a, T> {
    pub(crate) fn new(inner: &'a watch::Sender<Inner<T>>) -> Self {
        Self {
            inner,
            actions: Vec::new(),
        }
    }

    /// Sends a message as part of this batch.
    ///
    /// # Panics
    ///
    /// If `message` fails to serialize. Serialization runs here, not at
    /// commit: the failure surfaces at the offending call.
    pub fn send(&mut self, message: T) -> &mut Self
    where
        T: BorshSerialize,
    {
        self.actions.push(Action::Insert(Message::from(message)));
        self
    }

    /// Redacts a [`Key`] as part of this batch.
    ///
    /// Redacting a key not held at commit time is a no-op.
    pub fn redact(&mut self, key: Key) -> &mut Self {
        self.actions.push(Action::Forget(key));
        self
    }
}

impl<T: Send + Sync> Drop for Batch<'_, T> {
    fn drop(&mut self) {
        if self.actions.is_empty() {
            return;
        }
        let actions = std::mem::take(&mut self.actions);
        // A root-replacing commit, run entirely inside one critical section.
        // `Drop` cannot await, so this path cannot take the peer's commit
        // lock — and it does not need it: the closure is atomic on its own,
        // and the in-memory build is instantaneous. The body still runs the
        // commit protocol's phases in order (prep: stamp every action
        // against the commit-time frontier; build: apply; publish: wake
        // observers once iff the tree changed), so an explicit async commit
        // replacing this one only moves the build out of the critical
        // section, behind the commit lock — the phases themselves stand.
        self.inner.send_if_modified(|inner| {
            // The party is present on every reachable handle: `retire`
            // consumes the `Peer`, and the `Peer`/`Rumors` XOR keeps a
            // retiring set's handles from coexisting with it.
            let Some(party) = inner.party.as_ref() else {
                debug_assert!(false, "no party to tick in a `Batch` commit");
                return false;
            };
            // Prep, build, and publish in one traversal: `act` is
            // `assign` (stamp every action against the commit-time
            // frontier) composed with `react` (apply); its changed flag is
            // the single observer wakeup, and no root hash is read inside
            // this critical section (`Tree::act` states the flag's
            // contract). The explicit async commit calls the two halves
            // separately, with the build between them run off this
            // critical section.
            inner.tree.act(party, actions)
        });
    }
}
