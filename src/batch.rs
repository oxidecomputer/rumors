use std::{borrow::Borrow, sync::Arc};

use tokio::sync::watch;

use crate::message::{EncodeError, PayloadCodec};
use crate::tree::Action;
use crate::{Inner, Version};

/// Sent and redacted messages queued up for one atomic commit.
///
/// [`Rumors::batch`](crate::Rumors::batch) takes a closure and provides it with
/// a [`Batch`] to manipulate using [`send`](Self::send) and
/// [`redact`](Self::redact). If the closure returns `Ok`, all queued changes
/// are committed atomically; if it returns `Err` or panics, nothing is
/// committed.
///
/// Other operations may commit to the peer while the provided closure runs. As
/// a result, two concurrently prepared batches have no guaranteed ordering
/// relative to one another unless the application synchronizes them.
pub struct Batch<'a, T: Send + Sync + 'static> {
    /// The replica that receives the completed batch.
    inner: &'a watch::Sender<Inner<T>>,
    /// Validate and encode messages as they are queued.
    codec: PayloadCodec,
    /// The operations to apply together at commit.
    actions: Vec<Action>,
}

/// Debug a batch without requiring its payloads to be debuggable.
impl<T: Send + Sync + 'static> std::fmt::Debug for Batch<'_, T> {
    ///  Formats the counts of queued operations.
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

    /// Queue a message for this batch.
    ///
    /// The message is validated immediately, following the rules of
    /// [`Rumors::send`](crate::Rumors::send). A rejected message is not queued
    /// for the batch.
    ///
    /// If you propagate the potential
    /// [`EncodeError`] out of the closure, this will
    /// cancel the whole batch; handling it in place allows you to continue
    /// onwards within the same batch.
    ///
    /// [`Committed::sent`] counts the number of successful sent messages in the
    /// batch.
    pub fn send(&mut self, message: T) -> Result<(), EncodeError> {
        let message = self.codec.message(Arc::new(message))?;
        self.actions.push(Action::Insert(message));
        Ok(())
    }

    /// Queue the redaction of the message stamped with `version` for this
    /// batch.
    ///
    /// [`Committed::redacted`] counts the number of successful redacted
    /// messages in the batch.
    pub fn redact(&mut self, version: &Version) {
        self.actions.push(Action::Forget(version.clone()));
    }

    /// Queue every message in `messages` for this batch.
    ///
    /// This calls [`send`](Self::send) on each message until the first error.
    /// On any error, earlier messages remain queued in the batch, and later
    /// messages are not drawn from the iterator. Propagating the error out of
    /// the closure cancels the whole batch.
    pub fn send_all<I>(&mut self, messages: I) -> Result<(), EncodeError>
    where
        I: IntoIterator<Item = T>,
    {
        for message in messages {
            self.send(message)?;
        }
        Ok(())
    }

    /// Queue the redaction of every message stamped with any of `versions`.
    ///
    /// This is equivalent to calling [`redact`](Self::redact) on each in turn.
    pub fn redact_all<I>(&mut self, versions: I)
    where
        I: IntoIterator,
        I::Item: Borrow<Version>,
    {
        for version in versions {
            self.redact(version.borrow());
        }
    }

    /// Apply the queued actions; return the replica's resulting frontier, how
    /// many messages the batch sent, and how many the queued redactions
    /// removed.
    pub(crate) fn commit(self) -> Commit {
        let Batch { inner, actions, .. } = self;
        let sent = actions
            .iter()
            .filter(|action| matches!(action, Action::Insert(_)))
            .count();
        let mut commit = None;
        Inner::commit(inner, |party, tree| {
            // A later action may discard an earlier insert. Keep the queued
            // handles until the commit releases the lock, even on unwind.
            let acted = tree.act(party, actions.iter().cloned());
            commit = Some(Commit {
                latest: tree.latest().clone(),
                sent,
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
    /// How many messages the batch sent.
    pub(crate) sent: usize,
    /// How many messages the batch's redactions removed.
    pub(crate) redacted: usize,
}

/// The result of a committed [`Rumors::batch`](crate::Rumors::batch).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Committed<R> {
    /// The value originally returned by the batch closure.
    pub value: R,
    /// How many messages were sent in the batch.
    pub sent: usize,
    /// How many messages were redacted in the batch.
    pub redacted: usize,
    /// The [`Peer`](crate::Peer)'s latest [`Version`] at the instant the batch
    /// committed.
    pub latest: Version,
}
