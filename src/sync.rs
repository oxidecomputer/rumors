// Deliberately undocumented for now: the prose lives on the async API at the
// crate root and will be adapted here once polished.

use std::io::{Read, Write};
use std::sync::Arc;

use borsh::{BorshDeserialize, BorshSerialize};
use futures::io::AllowStdIo;
use tokio_util::compat::{FuturesAsyncReadCompatExt, FuturesAsyncWriteCompatExt};

pub use crate::{
    Batch, Error, Key, Network, PROTOCOL_MAGIC, PROTOCOL_VERSION, Snapshot, Version, causally,
};
pub use ::borsh;

pub struct Known<T>(crate::Known<T>);

impl<T> std::fmt::Debug for Known<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

pub struct Broadcast<T>(crate::Broadcast<T>);

impl<T> std::fmt::Debug for Broadcast<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

#[must_use = "a declined or recovered retirement hands the Known back; dropping it leaks the identity"]
#[derive(Debug)]
pub enum Retire<T> {
    Retired,
    Declined { known: Known<T> },
    Recovered { known: Known<T>, error: Error },
    Uncertain { error: Error },
}

pub struct Messages<T>(crate::Messages<T>);

/// The outcome of one non-blocking observer step ([`Messages::try_next`]).
#[derive(Debug)]
pub enum TryNext<'a, T> {
    /// The observer yielded a message, lent until the observer's next call
    /// (exactly as [`Messages::borrow_next`] lends).
    Message((Key, &'a Version, &'a Arc<T>)),
    /// Nothing new to report right now; actors are still live, so more may
    /// come. Ask again later.
    Quiet,
    /// Every handle is gone and the complete final state has been yielded;
    /// no further message is possible.
    Ended,
}

impl<T> Messages<T> {
    pub fn borrow_next(&mut self) -> Option<(Key, &Version, &Arc<T>)>
    where
        T: Send + Sync,
    {
        pollster::block_on(self.0.borrow_next())
    }

    /// Advance to the next message only if one is already available,
    /// distinguishing a *quiet* observer (nothing new, actors live) from an
    /// *ended* one — where [`borrow_next`](Self::borrow_next) would block
    /// through the quiet case. The non-blocking face for callers that catch
    /// up opportunistically between their own work, e.g.
    /// `while let TryNext::Message(m) = messages.try_next() { … }` to drain
    /// whatever is pending.
    pub fn try_next(&mut self) -> TryNext<'_, T>
    where
        T: Send + Sync,
    {
        use futures::FutureExt;
        match self.0.borrow_next().now_or_never() {
            None => TryNext::Quiet,
            Some(None) => TryNext::Ended,
            Some(Some(message)) => TryNext::Message(message),
        }
    }

    pub fn checkpoint(&self) -> &Version {
        self.0.checkpoint()
    }
}

impl<T: Send + Sync + 'static> Iterator for Messages<T> {
    type Item = (Key, Version, Arc<T>);

    fn next(&mut self) -> Option<Self::Item> {
        pollster::block_on(futures::StreamExt::next(&mut self.0))
    }
}

pub struct CausalMessages<T>(crate::CausalMessages<T>);

impl<T> CausalMessages<T> {
    pub fn borrow_next(&mut self) -> Option<(Key, &Version, &Arc<T>)>
    where
        T: Send + Sync,
    {
        pollster::block_on(self.0.borrow_next())
    }

    /// The non-blocking face, exactly as [`Messages::try_next`]: a message
    /// only if one is already deliverable, distinguishing quiet from ended.
    /// Note that a backlog's first message becomes deliverable only once
    /// its whole pass has been ingested, which this call performs eagerly.
    pub fn try_next(&mut self) -> TryNext<'_, T>
    where
        T: Send + Sync,
    {
        use futures::FutureExt;
        match self.0.borrow_next().now_or_never() {
            None => TryNext::Quiet,
            Some(None) => TryNext::Ended,
            Some(Some(message)) => TryNext::Message(message),
        }
    }

    pub fn checkpoint(&self) -> &Version {
        self.0.checkpoint()
    }
}

impl<T: Send + Sync + 'static> Iterator for CausalMessages<T> {
    type Item = (Key, Version, Arc<T>);

    fn next(&mut self) -> Option<Self::Item> {
        pollster::block_on(futures::StreamExt::next(&mut self.0))
    }
}

impl<T> Known<T> {
    pub fn seed() -> Self {
        Known(crate::Known::seed())
    }

    #[doc(hidden)]
    pub fn seed_rng<R: rand::RngCore + ?Sized>(rng: &mut R) -> Self {
        Known(crate::Known::seed_rng(rng))
    }

    pub fn bootstrap<R, W>(read: &mut R, write: &mut W) -> Result<Option<Self>, Error>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync,
        R: Read + Send,
        W: Write + Send,
    {
        let mut read = AllowStdIo::new(read).compat();
        let mut write = AllowStdIo::new(write).compat_write();
        pollster::block_on(crate::Known::<T>::bootstrap(&mut read, &mut write))
            .map(|known| known.map(Known))
    }

    pub fn send(&self, message: T) -> Batch<'_, T>
    where
        T: BorshSerialize + Send + Sync,
    {
        self.0.send(message)
    }

    pub fn redact(&self, key: Key) -> Batch<'_, T>
    where
        T: Send + Sync,
    {
        self.0.redact(key)
    }

    pub fn batch(&self) -> Batch<'_, T>
    where
        T: Send + Sync,
    {
        self.0.batch()
    }

    pub fn gossip<R, W>(&mut self, read: &mut R, write: &mut W) -> Result<(), Error>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync,
        R: Read + Send,
        W: Write + Send,
    {
        let mut read = AllowStdIo::new(read).compat();
        let mut write = AllowStdIo::new(write).compat_write();
        pollster::block_on(self.0.gossip(&mut read, &mut write))
    }

    pub fn retire<R, W>(self, read: &mut R, write: &mut W) -> Retire<T>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync,
        R: Read + Send,
        W: Write + Send,
    {
        let mut read = AllowStdIo::new(read).compat();
        let mut write = AllowStdIo::new(write).compat_write();
        match pollster::block_on(self.0.retire(&mut read, &mut write)) {
            crate::Retire::Retired => Retire::Retired,
            crate::Retire::Declined { known } => Retire::Declined {
                known: Known(known),
            },
            crate::Retire::Recovered { known, error } => Retire::Recovered {
                known: Known(known),
                error,
            },
            crate::Retire::Uncertain { error } => Retire::Uncertain { error },
        }
    }

    pub fn broadcast(self) -> Broadcast<T> {
        Broadcast(self.0.broadcast())
    }

    pub fn snapshot(&self) -> Snapshot<T> {
        self.0.snapshot()
    }

    pub fn network(&self) -> Network {
        self.0.network()
    }

    pub fn latest(&self) -> Version {
        self.0.latest()
    }

    pub fn earliest(&self) -> Option<Version> {
        self.0.earliest()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn hash(&self) -> [u8; 32] {
        self.0.hash()
    }

    pub fn get(&self, key: &Key) -> Option<(Version, Arc<T>)> {
        self.0.get(key)
    }

    pub fn messages(&self) -> Messages<T>
    where
        T: Send + Sync,
    {
        self.messages_from(Version::new())
    }

    pub fn messages_from(&self, since: Version) -> Messages<T>
    where
        T: Send + Sync,
    {
        Messages(self.0.messages_from(since))
    }

    pub fn causal_messages(&self) -> CausalMessages<T>
    where
        T: Send + Sync,
    {
        self.causal_messages_from(Version::new())
    }

    pub fn causal_messages_from(&self, since: Version) -> CausalMessages<T>
    where
        T: Send + Sync,
    {
        CausalMessages(self.0.causal_messages_from(since))
    }

    #[doc(hidden)]
    pub fn warm_caches(&self) {
        self.0.warm_caches();
    }
}

impl<T> Clone for Broadcast<T> {
    fn clone(&self) -> Self {
        Broadcast(self.0.clone())
    }
}

impl<T> Broadcast<T> {
    pub fn send(&self, message: T) -> Batch<'_, T>
    where
        T: BorshSerialize + Send + Sync,
    {
        self.0.send(message)
    }

    pub fn redact(&self, key: Key) -> Batch<'_, T>
    where
        T: Send + Sync,
    {
        self.0.redact(key)
    }

    pub fn batch(&self) -> Batch<'_, T>
    where
        T: Send + Sync,
    {
        self.0.batch()
    }

    pub fn gossip<R, W>(&mut self, read: &mut R, write: &mut W) -> Result<(), Error>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync,
        R: Read + Send,
        W: Write + Send,
    {
        let mut read = AllowStdIo::new(read).compat();
        let mut write = AllowStdIo::new(write).compat_write();
        pollster::block_on(self.0.gossip(&mut read, &mut write))
    }

    pub fn reunite(self) -> Option<Known<T>> {
        pollster::block_on(self.0.reunite()).map(Known)
    }

    pub fn messages(&self) -> Messages<T>
    where
        T: Send + Sync,
    {
        self.messages_from(Version::new())
    }

    pub fn messages_from(&self, since: Version) -> Messages<T>
    where
        T: Send + Sync,
    {
        Messages(self.0.messages_from(since))
    }

    pub fn causal_messages(&self) -> CausalMessages<T>
    where
        T: Send + Sync,
    {
        self.causal_messages_from(Version::new())
    }

    pub fn causal_messages_from(&self, since: Version) -> CausalMessages<T>
    where
        T: Send + Sync,
    {
        CausalMessages(self.0.causal_messages_from(since))
    }

    pub fn network(&self) -> Network {
        self.0.network()
    }

    pub fn latest(&self) -> Version {
        self.0.latest()
    }

    pub fn earliest(&self) -> Option<Version> {
        self.0.earliest()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn snapshot(&self) -> Snapshot<T> {
        self.0.snapshot()
    }

    pub fn hash(&self) -> [u8; 32] {
        self.0.hash()
    }

    pub fn get(&self, key: &Key) -> Option<(Version, Arc<T>)> {
        self.0.get(key)
    }

    #[doc(hidden)]
    pub fn warm_caches(&self) {
        self.0.warm_caches();
    }
}
