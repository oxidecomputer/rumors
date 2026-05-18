use std::{
    io::{Read, Write},
    marker::PhantomData,
    sync::Arc,
};

use borsh::{BorshDeserialize, BorshSerialize};
use futures::io::AllowStdIo;
use tokio_util::compat::{FuturesAsyncReadCompatExt, FuturesAsyncWriteCompatExt};

use message::Message;
use tokio::io::{AsyncRead, AsyncWrite};
use tree::{Action, Tree, mirror};

mod imbl_borsh;
mod message;
mod tree;
mod version;

/// A local set of rumors, which we can add to, remove from, and gossip to peers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Local<T>(Tree<T>);

/// A remote connection to another peer in the gossip network.
///
/// This supports *asynchronous* [`Remote::gossip`]; if the underlying I/O is
/// instead synchronous, wrap it in [`Sync`].
#[derive(Clone, Debug)]
pub struct Remote<T, R, W> {
    read: R,
    write: W,
    _phantom: PhantomData<fn() -> T>,
}

/// An adapter which converts a [`Remote`] into one supporting synchronous I/O.
///
/// Use this when your reader/writer implement [`Read`]/[`Write`] rather than
/// [`AsyncRead`]/[`AsyncWrite`].
#[derive(Clone, Debug)]
pub struct Sync<T, R, W>(pub Remote<T, R, W>);

pub use mirror::remote::Error;
pub use tree::Key;
pub use version::Version;

pub use borsh;

impl<T> Local<T> {
    /// Create a new set of rumors, localized to the given party.
    ///
    /// It is assumed that parties are *globally unique* within the context
    /// of the gossip protocol. If multiple peers identify as the same party,
    /// then unintuitive behavior, including missed messages, may occur.
    pub fn for_party(party: impl AsRef<[u8]>) -> Self {
        Local(Tree::for_party(party))
    }

    /// Add messages to this set of rumors, executing the given closure for
    /// each new message as it is processed into the set.
    ///
    /// The closure receives an opaque [`Key`] which can be used to later
    /// [`garbage`](Self::garbage)-collect the corresponding message from the
    /// set of rumors, as well as the causal [`Version`]-vector of the message,
    /// and an [`Arc<T>`](Arc) holding the original message.
    ///
    /// The order of execution for `on_message` is *arbitrary* and *does not
    /// correspond to the order of the messages*.
    pub fn message<OnMessage, I>(&mut self, messages: I, mut on_message: OnMessage)
    where
        T: BorshSerialize,
        I: IntoIterator<Item = T>,
        OnMessage: FnMut(Key, &Version, &Arc<T>),
    {
        self.0.act(
            messages.into_iter().map(Message::from).map(Action::Insert),
            |v, k, m| m.as_ref().iter().for_each(|m| on_message(k, v, m.as_ref())),
        );
    }

    /// Redact a set of message keys so that they will no longer be gossiped to
    /// other peers, and those peers we gossip with will in turn redact them.
    ///
    /// The [`Key`] required to redact a message is provided originally to
    /// whichever `on_message` closure observed the message during insertion,
    /// in one of [`Local::message`], [`Local::process`], or [`Remote::gossip`].
    ///
    /// Once a message key is redacted by one peer, this is contagious to all
    /// other peers without them needing to redact the message themselves.
    pub fn redact<I: IntoIterator<Item = Key>>(&mut self, redacted: I) {
        self.0
            .act(redacted.into_iter().map(Action::Forget), |_, _, _| {});
    }

    /// Local rumor sets can be trivially cloned to allow concurrent gossiping;
    /// after this is done, they may be merged back together using this method.
    ///
    /// The closure receives an opaque [`Key`] which can be used to later
    /// [`garbage`](Self::garbage)-collect the corresponding message from the
    /// set of rumors, as well as the causal [`Version`]-vector of the message,
    /// and an [`Arc<T>`](Arc) holding the original message.
    ///
    /// All new messages present in the `new` but not in `self` will be processed
    /// by `on_message`, *but not the converse*. In other words, `process` treats
    /// `self` as "already known" and `new` as... new.
    pub fn process<OnMessage>(&mut self, new: Local<T>, mut on_message: OnMessage)
    where
        OnMessage: FnMut(Key, &Version, &Arc<T>),
    {
        // Do nothing on the given message
        let x = message_fn(|_, _, _| {});

        // Process the given message as instructed by the caller
        let on_message = message_fn(|v, k, m| on_message(k, v, Message::as_ref(m)));

        // Instantiate the two sides of the mirror exchange, both local
        let l = mirror::local::Exchange::start(self.0.root.clone(), x, on_message);
        let r = mirror::local::Exchange::start(new.0.root, x, x);

        // Drive them to completion: we know they don't need a "real" executor
        Ok((self.0.root, _)) = pollster::block_on(mirror(l, r));
    }
}

impl<R, W, T> Remote<T, R, W> {
    /// Make a new remote endpoint for gossip, constructed from a reader and writer.
    ///
    /// The reader/writer pair may be asynchronous or synchronous; use [`Sync`] in
    /// the case where the reader/writer is synchronous.
    pub fn new(read: R, write: W) -> Self {
        Self {
            read,
            write,
            _phantom: PhantomData,
        }
    }

    /// Gossip with a remote peer to synchronize rumor sets, invoking `on_message`
    /// whenever we learn of a new message.
    ///
    /// The closure receives an opaque [`Key`] which can be used to later
    /// [`garbage`](Self::garbage)-collect the corresponding message from the
    /// set of rumors, as well as the causal [`Version`]-vector of the message,
    /// and an [`Arc<T>`](Arc) holding the original message.
    ///
    /// This method is asynchronous, and requires that the reader/writer implement
    /// asynchronous ([`tokio::io`]) [`AsyncRead`]/[`AsyncWrite`]. For use with
    /// synchronous I/O, wrap this remote peer in [`Sync`] and use [`Sync::gossip`].
    pub async fn gossip<OnMessage>(
        &mut self,
        mut old: Local<T>,
        mut on_message: OnMessage,
    ) -> Result<Local<T>, Error>
    where
        T: BorshDeserialize + BorshSerialize,
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
        OnMessage: FnMut(Key, &Version, &Arc<T>),
    {
        // Do nothing on the given message
        let x = message_fn(|_, _, _| {});

        // Process the given message as instructed by the caller
        let on_message = message_fn(|v, k, m| on_message(k, v, Message::as_ref(m)));

        // Instantiate the two sides of the mirror exchange: local and remote
        let l = mirror::local::Exchange::start(old.0.root, x, on_message);
        let r = mirror::remote::Exchange::start(&mut self.read, &mut self.write);

        // Drive them to completion against each other
        (old.0.root, _) = mirror(l, r).await.map_err(|e| {
            // The only possible error is a server error
            let mirror::Error::Server(e) = e;
            e
        })?;

        Ok(old)
    }
}

impl<T, R, W> Sync<T, R, W> {
    /// Gossip with a remote peer to synchronize rumor sets, invoking `on_message`
    /// whenever we learn of a new message.
    ///
    /// The closure receives an opaque [`Key`] which can be used to later
    /// [`garbage`](Self::garbage)-collect the corresponding message from the
    /// set of rumors, as well as the causal [`Version`]-vector of the message,
    /// and an [`Arc<T>`](Arc) holding the original message.
    ///
    /// This method is synchronous, and requires that the reader/writer implement
    /// synchronous ([`std::io`]) [`Read`]/[`Write`]. For use with asynchronous I/O,
    /// don't use [`Sync`] and instead directly use [`Remote::gossip`].
    pub fn gossip<OnMessage>(
        &mut self,
        old: Local<T>,
        on_message: OnMessage,
    ) -> Result<Local<T>, Error>
    where
        T: BorshDeserialize + BorshSerialize,
        R: Read + Unpin,
        W: Write + Unpin,
        OnMessage: FnMut(Key, &Version, &Arc<T>),
    {
        let Remote { read, write, .. } = &mut self.0;
        let mut new = Remote::new(
            AllowStdIo::new(read).compat(),
            AllowStdIo::new(write).compat_write(),
        );
        pollster::block_on(new.gossip(old, on_message))
    }
}

// Coerce the type into the correct HRTB shape to preserve inference
fn message_fn<T, F>(f: F) -> F
where
    F: for<'a, 'b> FnMut(&'a Version, Key, &'b Message<T>),
{
    f
}
