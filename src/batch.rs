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
/// Building a [`Batch`] holds no lock; the rumor set is locked momentarily when
/// the batch commits.
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

    /// Send a message.
    pub fn send(&mut self, message: T) -> &mut Self
    where
        T: BorshSerialize,
    {
        self.actions.push(Action::Insert(Message::from(message)));
        self
    }

    /// Redact a [`Key`].
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
        self.inner.send_if_modified(|inner| {
            // The party is present on every reachable handle: `retire`
            // consumes the `Peer`, and the `Peer`/`Rumors` XOR keeps a
            // retiring set's handles from coexisting with it.
            let Some(party) = inner.party.as_ref() else {
                debug_assert!(false, "no party to tick in a `Batch` commit");
                return false;
            };
            let hash_before = inner.tree.hash();
            inner.tree.act(party, actions);
            inner.tree.hash() != hash_before
        });
    }
}
