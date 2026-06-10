use crate::{Error, Key, Known, Message, Network, Snapshot, Version, tree::Action};
use borsh::{BorshDeserialize, BorshSerialize};
use std::future::ready;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::watch,
};

/// A broadcast handle for a set of rumors.
pub struct Broadcast<T> {
    pub(crate) known: Known<T>,
    /// Liveness token for the future returned by [`Known::broadcast`]: every
    /// clone of this `Broadcast` holds a clone of this receiver, and that
    /// future awaits the paired sender's
    /// [`closed`](tokio::sync::watch::Sender::closed), which resolves exactly
    /// when the last receiver (the last `Broadcast`) drops. Nothing is ever
    /// sent on this channel.
    pub(crate) alive: watch::Receiver<()>,
}

impl<T> Clone for Broadcast<T> {
    fn clone(&self) -> Self {
        Self {
            known: Known {
                network: self.known.network,
                inner: self.known.inner.clone(),
            },
            alive: self.alive.clone(),
        }
    }
}

impl<T> Broadcast<T> {
    /// Send messages to all listeners.
    pub fn send<'a, I>(&'a mut self, messages: I)
    where
        T: BorshSerialize + Send + Sync + 'a,
        I: IntoIterator<Item = T> + Send,
        I::IntoIter: Send,
    {
        self.known.inner.send_if_modified(|inner| {
            let party = inner
                .party
                .as_ref()
                .expect("party must be present for send");
            let hash_before = inner.tree.hash();
            pollster::block_on(inner.tree.act(
                |batch| {
                    batch.tick(party);
                },
                messages.into_iter().map(Message::from).map(Action::Insert),
                |_, _, _| ready(()),
            ));
            inner.tree.hash() != hash_before
        });
    }

    /// Redact the given keys for all listeners.
    ///
    /// The corresponding messages will be contagiously purged from the
    /// [`Known`] set for all peers who gossip with us, and will be unobserved
    /// by any future peers who did not already observe the messages.
    pub fn redact<I: IntoIterator<Item = Key>>(&mut self, redacted: I)
    where
        T: Send + Sync,
    {
        self.known.inner.send_if_modified(|inner| {
            let party = inner
                .party
                .as_ref()
                .expect("party must be present for redact");
            let hash_before = inner.tree.hash();
            pollster::block_on(inner.tree.act(
                |batch| {
                    batch.tick(party);
                },
                redacted.into_iter().map(Action::Forget),
                |_, _, _| ready(()),
            ));
            inner.tree.hash() != hash_before
        });
    }

    /// Gossip with a remote peer to synchronize rumor sets.
    pub async fn gossip<'a, R, W>(&mut self, read: &'a mut R, write: &'a mut W) -> Result<(), Error>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'a,
        R: AsyncRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        self.known.gossip(read, write).await
    }

    /// The identifier shared by every peer that descends from the same
    /// [`seed`](Known::seed).
    pub fn network(&self) -> Network {
        self.known.network()
    }

    /// The latest version of any message ever tracked by this [`Known`].
    pub fn latest(&self) -> Version {
        self.known.latest()
    }

    /// The earliest version of any message currently present in this [`Known`], or
    /// `None` if it has never seen a message.
    pub fn earliest(&self) -> Option<Version> {
        self.known.earliest()
    }

    /// Determine if there are any current messages in this [`Known`].
    pub fn is_empty(&self) -> bool {
        self.known.is_empty()
    }

    /// The number of live messages in this [`Known`].
    pub fn len(&self) -> usize {
        self.known.len()
    }

    /// Take a consistent snapshot of the current state.
    pub fn snapshot(&self) -> Snapshot<T> {
        self.known.snapshot()
    }

    /// Force this set's tree to compute its lazy structural memos (observable
    /// hash and ceiling/floor version bounds), so a subsequent operation is
    /// timed against its own work. For benchmark and test calibration only.
    #[doc(hidden)]
    pub fn warm_caches(&self) {
        self.known.warm_caches();
    }
}
