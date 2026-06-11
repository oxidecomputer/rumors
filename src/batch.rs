use borsh::BorshSerialize;
use tokio::sync::watch;

use crate::message::Message;
use crate::tree::Action;
use crate::{Inner, Key};

/// An accumulated batch of insertions and redactions against one rumor set,
/// committed atomically when dropped.
///
/// Returned by [`send`](crate::Rumors::send), [`redact`](crate::Rumors::redact),
/// and [`batch`](crate::Rumors::batch) on [`Rumors`](crate::Rumors). Dropping
/// the batch commits it: the single-action case reads as a plain call
/// (`rumors.send(message);` commits at the end of the statement), and
/// chaining accumulates — `rumors.batch().send(a).send(b).redact(key);` —
/// into one commit.
///
/// Building holds no lock and observes nothing: actions accumulate in the
/// batch alone, and the rumor set is locked only inside the commit. A batch
/// commits in one tree traversal and one change notification; each action
/// still advances the version once, so inserts in a batch carry strictly
/// increasing versions (content-identical messages get distinct keys), and a
/// [`redact`](Self::redact) of a key minted earlier in the same batch
/// overrides that insert (the last action on a key wins).
///
/// An empty batch commits nothing.
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

    /// Append a message insertion to the batch.
    pub fn send(&mut self, message: T) -> &mut Self
    where
        T: BorshSerialize,
    {
        self.actions.push(Action::Insert(Message::from(message)));
        self
    }

    /// Append a redaction to the batch. When committed, the corresponding
    /// message is contagiously purged from the rumor set for all peers who
    /// gossip with us, and will be unobserved by any future peers who did
    /// not already observe it.
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
