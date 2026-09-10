use std::sync::Arc;

use tokio::sync::watch;

use crate::message::{EncodeError, PayloadCodec};
use crate::tree::Action;
use crate::tree::typed::Path;
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
pub struct Batch<'a, T: Send + Sync> {
    inner: &'a watch::Sender<Inner<T>>,
    /// Validate and encode messages as they are queued.
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
    pub fn send(&mut self, message: T) -> Result<(), EncodeError>
    where
        T: 'static,
    {
        let message = self.codec.message(Arc::new(message))?;
        self.actions.push(Action::Insert(message));
        Ok(())
    }

    /// Queue a redaction of the message stamped with `version`.
    ///
    /// Redacting a version not held at commit time is a no-op.
    pub fn redact(&mut self, version: &Version) {
        self.actions.push(Action::Forget(Path::for_leaf(version)));
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
        T: 'static,
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
    /// versions not held at commit time are no-ops.
    pub fn redact_all<'v, I>(&mut self, versions: I)
    where
        I: IntoIterator<Item = &'v Version>,
    {
        for version in versions {
            self.redact(version);
        }
    }

    /// Apply the queued actions, notifying observers if the tree changed.
    pub(crate) fn commit(self) {
        let Batch { inner, actions, .. } = self;
        Inner::commit(inner, |inner| {
            // A later action may discard an earlier insert. Keep the queued
            // handles until the commit releases the lock, even on unwind.
            inner.tree.act(&inner.party, actions.iter().cloned())
        });
    }
}
