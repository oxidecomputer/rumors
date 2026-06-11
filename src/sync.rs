// Deliberately undocumented for now: the prose lives on the async API at the
// crate root and will be adapted here once polished.

use std::future::Future;
use std::io::{Read, Write};
use std::pin::Pin;
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

pub struct BroadcastComplete<'a, T>(Pin<Box<dyn Future<Output = crate::Known<T>> + Send + 'a>>);

#[must_use = "a declined or recovered retirement hands the Known back; dropping it leaks the identity"]
#[derive(Debug)]
pub enum Retire<T> {
    Retired,
    Declined { known: Known<T> },
    Recovered { known: Known<T>, error: Error },
    Uncertain { error: Error },
}

pub struct Messages<T>(crate::Messages<T>);

impl<T> Messages<T> {
    pub fn borrow_next(&mut self) -> Option<(Key, &Version, &Arc<T>)>
    where
        T: Send + Sync,
    {
        pollster::block_on(self.0.borrow_next())
    }

    pub fn cursor(&self) -> &Version {
        self.0.cursor()
    }
}

impl<T: Send + Sync + 'static> Iterator for Messages<T> {
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

    pub fn broadcast<'a>(self) -> (Broadcast<T>, BroadcastComplete<'a, T>)
    where
        T: Send + Sync + 'a,
    {
        let (broadcast, until_no_broadcasts) = self.0.broadcast();
        (
            Broadcast(broadcast),
            BroadcastComplete(Box::pin(until_no_broadcasts)),
        )
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

    #[doc(hidden)]
    pub fn warm_caches(&self) {
        self.0.warm_caches();
    }
}

impl<T> BroadcastComplete<'_, T> {
    pub fn wait(self) -> Known<T> {
        Known(pollster::block_on(self.0))
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
