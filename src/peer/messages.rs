mod causal;
mod changes;
mod channel;
mod unordered;

pub use causal::CausalMessages;
pub use changes::{Changes, TryTick};
pub use unordered::{TryNext, UnorderedMessages};

use std::borrow::Borrow;

use super::Peer;
use crate::bookmark::MaybeBookmark;
use crate::message::EncodeError;
use crate::{Batch, Committed, Snapshot, Version};

impl<T: Send + Sync + 'static, B: MaybeBookmark> Peer<T, B> {
    /// Send a new message, returning the sent message's [`Version`].
    ///
    /// The returned [`Version`] uniquely identifies this message throughout the
    /// gossip network and can be passed directly to [`redact`](Self::redact) in
    /// the future.
    ///
    /// To send many messages all at once, use [`send_all`](Self::send_all); to
    /// mix sends and redactions in one commit, use [`batch`](Self::batch).
    ///
    /// # Errors
    ///
    /// Before accepting the message, [`send`](Self::send) checks that every
    /// receiver would decode exactly the message you sent, returning an
    /// [`EncodeError`] if a check fails:
    ///
    /// - [`Serialize`](EncodeError::Serialize) if the message's
    ///   [`Serialize`](serde::Serialize) implementation fails;
    ///
    /// - [`Deserialize`](EncodeError::Deserialize) if its
    ///   [`Deserialize`](serde::Deserialize) implementation rejects that
    ///   serialization;
    /// - [`Roundtrip`](EncodeError::Roundtrip) if the result of serializing and
    ///   deserializing is something not equal to the original `message`;
    ///
    /// - [`TooDeep`](EncodeError::TooDeep) if serializing the message requires
    ///   deeper recursion than
    ///   [`PeerBuilder::message_depth_limit`](crate::PeerBuilder::message_depth_limit).
    ///
    /// A rejected message is neither stored nor sent.
    pub fn send(&self, message: T) -> Result<Version, EncodeError> {
        self.replica.send(message)
    }

    /// Redact an existing message by supplying its unique [`Version`],
    /// returning `true` if the message previously existed.
    ///
    /// This removes the message, if any exists, which is stamped with
    /// `version`. The removal propagates automatically during gossip.
    ///
    /// To redact many versions all at once, use
    /// [`redact_all`](Self::redact_all); to mix redactions and sends in one
    /// commit, use [`batch`](Self::batch).
    pub fn redact(&self, version: &Version) -> bool {
        self.replica.redact(version)
    }

    /// Send everything in `messages`, in one all-or-nothing commit.
    ///
    /// Returns the last message's [`Version`], or `None` for an empty iterator.
    pub fn send_all<I>(&self, messages: I) -> Result<Option<Version>, EncodeError>
    where
        I: IntoIterator<Item = T>,
    {
        self.replica.send_all(messages)
    }

    /// Redact everything in `versions`, in one commit.
    ///
    /// Returns how many messages it removed.
    pub fn redact_all<I>(&self, versions: I) -> usize
    where
        I: IntoIterator,
        I::Item: Borrow<Version>,
    {
        self.replica.redact_all(versions)
    }

    /// Apply several changes to the set of messages, atomically.
    ///
    /// Runs `f` with a [`Batch`] handle for queueing [`send`](Batch::send)s and
    /// [`redact`](Batch::redact)s, and commits everything only if `f` returns
    /// `Ok`. Any other exit (i.e. `Err` or panic) does not commit anything.
    ///
    /// Returns [`Committed`], which annotates the closure's original value with
    /// information about how many messages the batch sent and redacted, and the
    /// [`Peer`]'s [`Version`] right after the commit.
    ///
    /// The closure is synchronous and the type system prevents the handle from
    /// leaving it, so async cancellation cannot observe a half-built batch. The
    /// closure may use the same `Peer` handle safely, though perhaps
    /// confusingly, since such nested operations commit *inside-out*,
    /// separately.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::{Peer, error::EncodeError};
    ///
    /// let peer = Peer::<String>::seed();
    /// peer.batch(|batch| {
    ///     batch.send("a".to_string())?;
    ///     batch.send("b".to_string())?;
    ///     Ok::<(), EncodeError>(())
    /// })?;
    /// // Both landed, in a single commit.
    /// assert_eq!(peer.snapshot().len(), 2);
    /// # Ok::<(), EncodeError>(())
    /// ```
    ///
    /// The scope handle cannot be moved out of the closure...
    ///
    /// ```compile_fail
    /// let peer = rumors::Peer::<String>::seed();
    /// let mut stash = None;
    /// let _ = peer.batch::<_, (), _>(|batch| {
    ///     stash = Some(batch);
    ///     Ok(())
    /// });
    /// ```
    ///
    /// ...and cannot be returned from it either:
    ///
    /// ```compile_fail
    /// let peer = rumors::Peer::<String>::seed();
    /// let escaped = peer.batch::<_, (), _>(|batch| Ok(batch));
    /// ```
    pub fn batch<R, E, F>(&self, f: F) -> Result<Committed<R>, E>
    where
        F: for<'s> FnOnce(&'s mut Batch<'_, T>) -> Result<R, E>,
    {
        self.replica.batch(f)
    }

    /// Take a consistent point-in-time snapshot of the set of messages.
    pub fn snapshot(&self) -> Snapshot<T> {
        self.replica.snapshot()
    }

    /// Iterate through a stream of every message in this [`Peer`], however it
    /// arrived, in arbitrary (*non-causal*) order.
    ///
    /// See [`UnorderedMessages`] for details.
    pub fn unordered_messages(&self) -> UnorderedMessages<T> {
        self.replica.unordered_messages()
    }

    /// Iterate through a stream of every message not causally contained in
    /// `since`, then everything learned afterwards, in arbitrary (*non-causal*)
    /// order.
    ///
    /// See [`UnorderedMessages`] for details.
    pub fn unordered_messages_since(&self, since: Version) -> UnorderedMessages<T> {
        self.replica.unordered_messages_since(since)
    }

    /// Iterate through a stream of every message in this [`Peer`], however it
    /// arrived, in *causal order*.
    ///
    /// See [`CausalMessages`] for details.
    pub fn causal_messages(&self) -> CausalMessages<T> {
        self.replica.causal_messages()
    }

    /// Iterate through a stream of every message not causally contained in
    /// `since`, then everything learned afterwards, in *causal order*.
    ///
    /// See [`CausalMessages`] for details.
    pub fn causal_messages_since(&self, since: Version) -> CausalMessages<T> {
        self.replica.causal_messages_since(since)
    }

    /// Iterate through a stream of notifications *that* this [`Peer`]
    /// changed, without observing what changed.
    ///
    /// The result is a coalescing stream that yields `()` immediately on first
    /// poll, then again every time the set of messages changes.
    ///
    /// See [`Changes`] for details.
    pub fn changes(&self) -> Changes<T> {
        self.replica.changes()
    }
}
