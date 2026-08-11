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
/// (`rumors.batch().send(a).send(b).redact(key);`) into one commit. A batch
/// dropped during a panic's unwind commits nothing: none of it, never a
/// prefix.
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
        // A drop reached by a panic's unwind commits nothing: the caller
        // never finished building the batch, and atomicity ("none of the
        // batch or all of it, never a prefix") rules out publishing the
        // prefix queued before the panic. This also covers an unrelated
        // panic unwinding over a held batch: RAII-transaction style, an
        // unwound batch aborts.
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
