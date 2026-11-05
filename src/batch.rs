use std::{borrow::Borrow, sync::Arc};

use tokio::sync::watch;

use crate::message::{EncodeError, PayloadCodec};
use crate::tree::Action;
use crate::{Inner, Version};

/// Insertions and redactions queued for one atomic commit.
///
/// [`Rumors::batch`](crate::Rumors::batch) gives its closure a batch to fill
/// with [`send`](Self::send) and [`redact`](Self::redact). Returning `Ok`
/// commits all queued changes; returning `Err` or panicking commits none.
///
/// Building a batch holds no lock. Other operations may commit while the
/// closure runs; two batches have no guaranteed causal order unless the
/// application synchronizes them.
pub struct Batch<'a, T: Send + Sync + 'static> {
    /// The replica that receives the completed batch.
    inner: &'a watch::Sender<Inner<T>>,
    /// Validate and encode messages as they are queued.
    codec: PayloadCodec,
    /// The operations to apply together at commit.
    actions: Vec<Action>,
}

/// Summarize a batch without requiring its payloads to be debuggable.
impl<T: Send + Sync + 'static> std::fmt::Debug for Batch<'_, T> {
    /// Formats the number of queued operations.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Batch")
            .field("queued", &self.actions.len())
            .finish_non_exhaustive()
    }
}

impl<'a, T: Send + Sync + 'static> Batch<'a, T> {
    /// Start an empty batch for `inner`, using its payload codec.
    pub(crate) fn new(inner: &'a watch::Sender<Inner<T>>, codec: PayloadCodec) -> Self {
        Self {
            inner,
            codec,
            actions: Vec::new(),
        }
    }

    /// Queue a message for this batch's commit.
    ///
    /// Validate the message immediately, following the admission rules of
    /// [`Rumors::send`](crate::Rumors::send). A rejected message is not queued.
    /// Propagating the error out of the batch closure cancels the whole batch;
    /// handling it there leaves earlier messages queued.
    ///
    /// # Panics
    ///
    /// Panics if `message` fails to serialize, as with
    /// [`Rumors::send`](crate::Rumors::send).
    pub fn send(&mut self, message: T) -> Result<(), EncodeError> {
        let message = self.codec.message(Arc::new(message))?;
        self.actions.push(Action::Insert(message));
        Ok(())
    }

    /// Queue a redaction of the message stamped with `version`.
    ///
    /// Redacting a version not held at commit time is a no-op. The commit's
    /// [`Committed::redacted`] counts the queued redactions that removed a
    /// message.
    pub fn redact(&mut self, version: &Version) {
        self.actions.push(Action::Forget(version.clone()));
    }

    /// Queue every message `messages` yields.
    ///
    /// Call [`send`](Self::send) on each message until the first error.
    /// Earlier messages remain queued; later messages are not drawn from
    /// the iterator. Propagating the error out of the batch closure cancels
    /// the whole batch.
    ///
    /// # Panics
    ///
    /// Panics if a message fails to serialize, as with [`send`](Self::send).
    pub fn send_all<I>(&mut self, messages: I) -> Result<(), EncodeError>
    where
        I: IntoIterator<Item = T>,
    {
        for message in messages {
            self.send(message)?;
        }
        Ok(())
    }

    /// Queue a redaction of every version `versions` yields.
    ///
    /// Equivalent to calling [`redact`](Self::redact) on each in turn;
    /// versions not held at commit time are no-ops, and
    /// [`Committed::redacted`] counts the rest.
    pub fn redact_all<I>(&mut self, versions: I)
    where
        I: IntoIterator,
        I::Item: Borrow<Version>,
    {
        for version in versions {
            self.redact(version.borrow());
        }
    }

    /// Apply the queued actions; return the replica's resulting frontier and
    /// how many messages the queued redactions removed.
    pub(crate) fn commit(self) -> Commit {
        let Batch { inner, actions, .. } = self;
        let mut commit = None;
        Inner::commit(inner, |party, tree| {
            // A later action may discard an earlier insert. Keep the queued
            // handles until the commit releases the lock, even on unwind.
            let acted = tree.act(party, actions.iter().cloned());
            commit = Some(Commit {
                latest: tree.latest().clone(),
                redacted: acted.removed,
            });
            acted.changed
        });
        commit.expect("commit closure always runs")
    }
}

/// What one batch commit did, for the local operations built on batches.
pub(crate) struct Commit {
    /// The replica's frontier right after the commit.
    pub(crate) latest: Version,
    /// How many messages the batch's redactions removed.
    pub(crate) redacted: usize,
}

/// The result of a committed [`Rumors::batch`](crate::Rumors::batch): the
/// closure's value, and what the batch's redactions removed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Committed<R> {
    /// The value the batch closure returned.
    pub value: R,
    /// How many messages the batch's [`redact`](Batch::redact)s removed.
    ///
    /// A redaction removes a message only if this replica held it, live, when
    /// the batch committed. A version it never received, or had already
    /// redacted, adds nothing, and nor does a second redaction of one version
    /// in the same batch. A redaction that removes nothing changes nothing,
    /// here or elsewhere: a message this replica never received is live if
    /// it arrives later, and a message it already redacted stays redacted.
    pub redacted: usize,
}
