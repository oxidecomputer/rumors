use borsh::BorshSerialize;
use tokio::sync::watch;

use crate::message::Message;
use crate::tree::Action;
use crate::{Inner, Key};

/// A batch of insertions and redactions against a [`Rumors`](crate::Rumors),
/// applied in one commit.
///
/// Returned by [`send`](crate::Rumors::send),
/// [`redact`](crate::Rumors::redact), and [`batch`](crate::Rumors::batch) on
/// [`Rumors`](crate::Rumors). Dropping the batch commits it: the single-action
/// case reads as a plain call (`rumors.send(message);` commits at the end of
/// the statement), and chaining accumulates
/// (`rumors.batch().send(a).send(b).redact(key);`) into one commit.
///
/// # A batch is a performance optimization, not an atomicity guarantee
///
/// Batching coalesces several actions into one tree traversal, one commit
/// moment, and at most one internal gossip wakeup, instead of one per
/// action. When the batch drops:
///
/// - **Dropped normally**, the batch commits everything queued so far, as
///   one commit: observers and concurrent gossip sessions see all of it
///   land at once, never a partially applied commit.
/// - **Dropped by a panic's unwind**, the batch commits nothing: the
///   caller never finished building it, so nothing it holds publishes.
/// - **Dropped by async cancellation** (the future holding it across an
///   `.await` is dropped), the batch commits the prefix queued before the
///   cancellation point. Cancellation runs no unwind, so this drop is
///   indistinguishable from an ordinary end-of-statement commit.
///
/// An application that needs several pieces delivered all-or-nothing even
/// under panic or cancellation should not reach for a batch: bundle the
/// pieces into one application-level message in your definition of the
/// application's message type `T`.
///
/// Building a [`Batch`] holds no lock; batches are serialized only upon
/// commit. Because building holds no lock, concurrent gossip rounds
/// can land between building and committing, and two batches carry no
/// guaranteed causal relationship to one another unless the application
/// synchronizes them itself.
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
        // A drop reached by a panic's unwind commits nothing: the caller
        // never finished building the batch, and nothing a half-built
        // batch holds may publish. This also covers an unrelated panic
        // unwinding over a held batch: RAII-transaction style, an unwound
        // batch aborts. The guard sees only unwinds: a drop by async
        // cancellation arrives outside any panic and commits the queued
        // prefix, the documented hazard the type docs state, pinned by
        // `a_cancelled_batch_commits_its_prefix` in `tests/single_peer.rs`.
        if std::thread::panicking() {
            return;
        }
        if self.actions.is_empty() {
            return;
        }
        let actions = std::mem::take(&mut self.actions);
        self.inner.send_if_modified(|inner| {
            // The party is present on every reachable handle: `retire`
            // consumes the `Peer`, and the `Peer`/`Rumors` XOR keeps a
            // retiring set's handles from coexisting with it.
            let Some(party) = inner.party.as_ref() else {
                debug_assert!(false, "no party to tick in a `Batch` commit");
                return false;
            };
            // Notify observers iff the batch changed the tree, straight from
            // `act`'s changed flag: no root hash is read inside this critical
            // section (`Tree::act` states the flag's contract).
            inner.tree.act(party, actions)
        });
    }
}
