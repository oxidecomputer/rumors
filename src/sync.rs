// Deliberately undocumented for now: the prose lives on the async API at the
// crate root and will be adapted here once polished.

use std::future::{Future, Ready, ready};
use std::io::{Read, Write};
use std::pin::Pin;
use std::sync::Arc;

use borsh::{BorshDeserialize, BorshSerialize};
use futures::io::AllowStdIo;
use tokio_util::compat::{FuturesAsyncReadCompatExt, FuturesAsyncWriteCompatExt};

pub use crate::{
    Error, Key, Network, PROTOCOL_MAGIC, PROTOCOL_VERSION, Snapshot, Version, causally,
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

fn into_async<T, F>(mut on_message: F) -> impl FnMut(Key, &Version, &Arc<T>) -> Ready<()>
where
    F: FnMut(Key, &Version, &Arc<T>),
{
    move |key, version, message| {
        on_message(key, version, message);
        ready(())
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

    pub fn send<'a, I>(&'a mut self, messages: I)
    where
        T: BorshSerialize + Send + Sync + 'a,
        I: IntoIterator<Item = T> + Send,
        I::IntoIter: Send,
    {
        self.0.send(messages);
    }

    pub fn redact<I: IntoIterator<Item = Key>>(&mut self, redacted: I)
    where
        T: Send + Sync,
    {
        self.0.redact(redacted);
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

    pub fn retire<R, W>(self, read: &mut R, write: &mut W) -> (Option<Self>, Result<(), Error>)
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync,
        R: Read + Send,
        W: Write + Send,
    {
        let mut read = AllowStdIo::new(read).compat();
        let mut write = AllowStdIo::new(write).compat_write();
        let (known, result) = pollster::block_on(self.0.retire(&mut read, &mut write));
        (known.map(Known), result)
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
    pub fn send<'a, I>(&'a mut self, messages: I)
    where
        T: BorshSerialize + Send + Sync + 'a,
        I: IntoIterator<Item = T> + Send,
        I::IntoIter: Send,
    {
        self.0.send(messages);
    }

    pub fn redact<I: IntoIterator<Item = Key>>(&mut self, redacted: I)
    where
        T: Send + Sync,
    {
        self.0.redact(redacted);
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

    pub fn listen<F>(self, on_message: F) -> Version
    where
        T: Send + Sync,
        F: FnMut(Key, &Version, &Arc<T>) + Send,
    {
        pollster::block_on(self.0.listen(into_async(on_message)))
    }

    pub fn listen_from<F>(self, since: Version, on_message: F) -> Version
    where
        T: Send + Sync,
        F: FnMut(Key, &Version, &Arc<T>) + Send,
    {
        pollster::block_on(self.0.listen_from(since, into_async(on_message)))
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
