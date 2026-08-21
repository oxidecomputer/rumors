use std::sync::Arc;

use tokio::sync::watch;

use crate::message::{EncodeError, PayloadCodec};
use crate::tree::Action;
use crate::tree::typed::Path;
use crate::{Inner, Version};

/// The scope handle for a batch of insertions and redactions against a
/// [`Rumors`](crate::Rumors), applied in one all-or-nothing commit.
///
/// Handed exclusively to the closure [`Rumors::batch`](crate::Rumors::batch)
/// runs: queue actions on it with [`send`](Self::send) and
/// [`redact`](Self::redact), and the batch commits — atomically, as one
/// commit — exactly when the closure returns `Ok`. Any other exit
/// (a returned `Err`, a panic) commits nothing; [`Rumors::batch`] states
/// the full lifecycle.
///
/// [`Rumors::batch`]: crate::Rumors::batch
///
/// Building a batch holds no lock; a batch is serialized against other
/// commits only at its own commit. Because building holds no lock,
/// concurrent gossip rounds can land between building and committing, and
/// two batches carry no guaranteed causal relationship to one another
/// unless the application synchronizes them itself.
pub struct Batch<'a, T: Send + Sync> {
    inner: &'a watch::Sender<Inner<T>>,
    /// The peer's payload codec: every queued send serializes and
    /// depth-checks through it.
    codec: PayloadCodec,
    actions: Vec<Action>,
}

impl<'a, T: Send + Sync> Batch<'a, T> {
    pub(crate) fn new(inner: &'a watch::Sender<Inner<T>>, codec: PayloadCodec) -> Self {
        Self {
            inner,
            codec,
            actions: Vec::new(),
        }
    }

    /// Queues a message for this batch's commit.
    ///
    /// Serialization and admission run here, not at commit: the message
    /// is serialized through the peer's codec immediately, and a payload
    /// a receiver would reject or misread — one nesting deeper than the
    /// peer's
    /// [`payload_depth_limit`](crate::Peer::payload_depth_limit), one
    /// whose type does not survive its own serde round-trip, or one
    /// whose encoding decodes to a different value — is the typed
    /// [`EncodeError`], surfacing at the offending call
    /// ([`Rumors::send`](crate::Rumors::send) states the admission
    /// contract). Propagating the
    /// error out of the closure cancels the whole batch
    /// ([`Rumors::batch`](crate::Rumors::batch)'s commit-on-`Ok`
    /// contract); handling it locally keeps the batch alive with the
    /// offending message not queued.
    ///
    /// # Panics
    ///
    /// If `message` fails to serialize: a violation of the payload
    /// contract ([choosing a payload
    /// type](crate#choosing-a-payload-type)), exactly as
    /// [`Rumors::send`](crate::Rumors::send) treats it.
    pub fn send(&mut self, message: T) -> Result<(), EncodeError>
    where
        T: 'static,
    {
        let message = self.codec.message(Arc::new(message))?;
        self.actions.push(Action::Insert(message));
        Ok(())
    }

    /// Queues a redaction of the message stamped with `version` for this
    /// batch's commit.
    ///
    /// Redacting a version not held at commit time is a no-op.
    pub fn redact(&mut self, version: &Version) {
        self.actions.push(Action::Forget(Path::for_leaf(version)));
    }

    /// Commit everything queued, as one commit.
    ///
    /// Observers and concurrent gossip sessions see all of it land at
    /// once, in at most one observer wakeup. Runs iff the caller's
    /// closure returned `Ok`
    /// ([`Rumors::batch`](crate::Rumors::batch) owns that decision).
    pub(crate) fn commit(self) {
        let Batch { inner, actions, .. } = self;
        // An empty action list needs no special case: `Tree::act`
        // documents an empty batch as a complete no-op, and its false
        // changed flag suppresses the wakeup.
        inner.send_if_modified(|inner| {
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
