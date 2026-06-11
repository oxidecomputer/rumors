// Deliberately undocumented for now: the prose lives on the async API at the
// crate root and will be adapted here once polished.

//! A synchronous interface to the crate.
//!
//! Prefer the main crate's interface in an async context.

use std::io::{Read, Write};
use std::sync::Arc;

use borsh::{BorshDeserialize, BorshSerialize};
use futures::io::AllowStdIo;
use tokio_util::compat::{FuturesAsyncReadCompatExt, FuturesAsyncWriteCompatExt};

pub use crate::{
    Batch, Error, Key, Network, PROTOCOL_MAGIC, PROTOCOL_VERSION, Snapshot, Version, causally,
};
pub use ::before;
pub use ::borsh;

pub struct Peer<T>(crate::Peer<T>);

impl<T> std::fmt::Debug for Peer<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

pub struct Rumors<T>(crate::Rumors<T>);

impl<T> std::fmt::Debug for Rumors<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

#[must_use = "a declined or recovered retirement hands the Peer back; dropping it leaks the identity"]
#[derive(Debug)]
pub enum Retire<T> {
    Retired,
    Declined { peer: Peer<T> },
    Recovered { peer: Peer<T>, error: Error },
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

impl<T> Peer<T> {
    pub fn seed() -> Self {
        Peer(crate::Peer::seed())
    }

    #[doc(hidden)]
    pub fn seed_rng<R: rand::RngCore + ?Sized>(rng: &mut R) -> Self {
        Peer(crate::Peer::seed_rng(rng))
    }

    pub fn network(&self) -> Network {
        self.0.network()
    }

    pub fn bootstrap<R, W>(read: &mut R, write: &mut W) -> Result<Option<Self>, Error>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync,
        R: Read + Send,
        W: Write + Send,
    {
        let mut read = AllowStdIo::new(read).compat();
        let mut write = AllowStdIo::new(write).compat_write();
        pollster::block_on(crate::Peer::<T>::bootstrap(&mut read, &mut write))
            .map(|known| known.map(Peer))
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
            crate::Retire::Declined { peer } => Retire::Declined { peer: Peer(peer) },
            crate::Retire::Recovered { peer, error } => Retire::Recovered {
                peer: Peer(peer),
                error,
            },
            crate::Retire::Uncertain { error } => Retire::Uncertain { error },
        }
    }

    pub fn into_rumors(self) -> Rumors<T> {
        Rumors(self.0.into_rumors())
    }

    #[doc(hidden)]
    pub fn warm_caches(&self) {
        self.0.warm_caches();
    }

    #[cfg(any(test, feature = "test-internals"))]
    #[doc(hidden)]
    pub fn dangerously_alias_party(&self) -> Option<before::Party> {
        self.0.dangerously_alias_party()
    }
}

impl<T> Clone for Rumors<T> {
    fn clone(&self) -> Self {
        Rumors(self.0.clone())
    }
}

impl<T> Rumors<T> {
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

    pub fn try_into_peer(self) -> Option<Peer<T>> {
        pollster::block_on(self.0.try_into_peer()).map(Peer)
    }

    pub fn messages(&self) -> Messages<T>
    where
        T: Send + Sync,
    {
        self.messages_since(Version::new())
    }

    pub fn messages_since(&self, since: Version) -> Messages<T>
    where
        T: Send + Sync,
    {
        Messages(self.0.messages_since(since))
    }

    pub fn causal_messages(&self) -> CausalMessages<T>
    where
        T: Send + Sync,
    {
        self.causal_messages_since(Version::new())
    }

    pub fn causal_messages_since(&self, since: Version) -> CausalMessages<T>
    where
        T: Send + Sync,
    {
        CausalMessages(self.0.causal_messages_since(since))
    }

    pub fn network(&self) -> Network {
        self.0.network()
    }

    pub fn snapshot(&self) -> Snapshot<T> {
        self.0.snapshot()
    }

    #[doc(hidden)]
    pub fn warm_caches(&self) {
        self.0.warm_caches();
    }

    #[cfg(any(test, feature = "test-internals"))]
    #[doc(hidden)]
    pub fn dangerously_alias_party(&self) -> Option<before::Party> {
        self.0.dangerously_alias_party()
    }
}
