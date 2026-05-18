//! Unordered gossip with redaction.
//!
//! `rumors` is a CRDT-backed gossip set: each peer holds a [`Local<T>`] rumor
//! set, and peers reconcile their sets by exchanging only the parts that
//! differ via a hash-tree mirror protocol. Once redacted, a message stops
//! propagating; redactions themselves spread contagiously to every peer the
//! redactor (transitively) gossips with.
//!
//! # Quickstart
//!
//! ```
//! use rumors::Local;
//!
//! // Each peer is identified by an arbitrary byte string. It is the caller's
//! // responsibility to ensure that party identifiers are globally unique
//! // within the gossip network.
//! let mut alice: Local<String> = Local::for_party("alice");
//!
//! // Add some messages. The closure fires once per newly-observed message
//! // with an opaque `Key` (used later for redaction), the causal `Version`
//! // at which the message was observed, and the value itself.
//! let mut observed = 0;
//! alice.message(
//!     ["hello".to_string(), "world".to_string()],
//!     |_key, _version, _message| observed += 1,
//! );
//! assert_eq!(observed, 2);
//! ```
//!
//! # Redaction
//!
//! Once a peer decides that a message is no longer needed, it can [`redact`]
//! the corresponding [`Key`], and the redaction propagates contagiously to
//! every other peer it gossips with. This is the mechanism by which the
//! network-wide rumor set is garbage-collected: any peer's local "we're done
//! with this" decision triggers global cleanup, without requiring consensus.
//!
//! [`redact`]: Local::redact
//!
//! ```
//! use rumors::{Local, Key};
//!
//! let mut alice: Local<String> = Local::for_party("alice");
//! let mut keys: Vec<Key> = Vec::new();
//! alice.message(["stale rumor".to_string()], |key, _, _| keys.push(key));
//! // Once alice (or any peer holding these keys) deems the message obsolete,
//! // redact it to evict it from every connected peer's rumor set.
//! alice.redact(keys);
//! ```
//!
//! # Concurrent rumor sets
//!
//! [`Local`] is cheap to clone: the underlying tree is structurally shared
//! and copy-on-write. Threads or tasks can each hold a clone and mutate it
//! independently, then recombine via [`Local::process`].
//!
//! # Gossiping with peers
//!
//! For inter-process synchronization, pair an asynchronous reader and writer
//! (e.g. a [`tokio`]-style socket) into a [`Remote`] and call
//! [`Remote::gossip`]. For synchronous I/O (e.g. [`std::net::TcpStream`]),
//! wrap the [`Remote`] in [`Sync`].
//!
//! ```
//! use rumors::{Local, Remote};
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() {
//! // Stand in for a real network with an in-memory bidirectional pipe.
//! let (a, b) = tokio::io::duplex(1024);
//! let (a_r, a_w) = tokio::io::split(a);
//! let (b_r, b_w) = tokio::io::split(b);
//!
//! let mut alice: Local<String> = Local::for_party("alice");
//! alice.message(["hello".to_string()], |_, _, _| {});
//! let bob: Local<String> = Local::for_party("bob");
//!
//! let mut alice_peer = Remote::new(a_r, a_w);
//! let mut bob_peer = Remote::new(b_r, b_w);
//!
//! // After gossip, bob has learned of alice's "hello".
//! let (alice, bob) = tokio::join!(
//!     alice_peer.gossip(alice, |_, _, _| {}),
//!     bob_peer.gossip(bob, |_, _, m| assert_eq!(m.as_ref(), "hello")),
//! );
//! let (_alice, _bob) = (alice.unwrap(), bob.unwrap());
//! # }
//! ```
//!
//! # Message serialization
//!
//! Messages are serialized with [`borsh`], which is re-exported here so
//! callers can derive [`borsh::BorshSerialize`] and [`borsh::BorshDeserialize`]
//! on their own message types without depending on borsh separately.

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
///
/// [`Local`] rumor sets are extremely cheap to clone, and can be efficiently
/// merged using [`Local::process`]; in concurrent applications, it's recommended
/// to use a clone per thread/task.
///
/// # Example
///
/// ```
/// use rumors::{Local, Key};
///
/// // Start with an empty rumor set tagged to a globally-unique party id.
/// let mut alice: Local<String> = Local::for_party("alice");
///
/// // Add some messages, capturing the keys they were assigned.
/// let mut keys: Vec<Key> = Vec::new();
/// alice.message(
///     ["hello".to_string(), "world".to_string()],
///     |key, _version, _message| keys.push(key),
/// );
///
/// // Later, redact one of them; redaction propagates to every peer alice
/// // gossips with.
/// alice.redact([keys[0]]);
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Local<T>(Tree<T>);

/// A remote connection to another peer in the gossip network.
///
/// This supports *asynchronous* [`Remote::gossip`]; if the underlying I/O is
/// instead synchronous, wrap it in [`Sync`].
///
/// # Example
///
/// ```
/// use rumors::{Local, Remote};
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// // Stand in for a real network with an in-memory bidirectional pipe.
/// let (a, b) = tokio::io::duplex(1024);
/// let (a_r, a_w) = tokio::io::split(a);
/// let (b_r, b_w) = tokio::io::split(b);
///
/// let mut alice: Local<String> = Local::for_party("alice");
/// alice.message(["hello".to_string()], |_, _, _| {});
/// let bob: Local<String> = Local::for_party("bob");
///
/// // Both ends drive the gossip protocol concurrently; bob learns of
/// // alice's "hello" and the call returns the updated rumor sets.
/// let mut alice_peer = Remote::new(a_r, a_w);
/// let mut bob_peer = Remote::new(b_r, b_w);
/// let (alice, bob) = tokio::join!(
///     alice_peer.gossip(alice, |_, _, _| {}),
///     bob_peer.gossip(bob, |_, _, m| assert_eq!(m.as_ref(), "hello")),
/// );
/// let (_alice, _bob) = (alice.unwrap(), bob.unwrap());
/// # }
/// ```
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
///
/// # Example
///
/// Two parties exchange rumors over an in-memory bidirectional channel built
/// from a pair of [`std::io::pipe`]s, each driving its side from its own
/// thread. Any other synchronous duplex transport works the same way.
///
/// ```
/// use std::io::pipe;
/// use std::thread;
/// use rumors::{Local, Remote, Sync};
///
/// // One pipe per direction makes a synchronous bidirectional channel.
/// let (a_to_b_r, a_to_b_w) = pipe().unwrap();
/// let (b_to_a_r, b_to_a_w) = pipe().unwrap();
///
/// // Bob accepts on another thread, starting from an empty rumor set, and
/// // reports back the messages he learns.
/// let bob_thread = thread::spawn(move || {
///     let mut peer = Sync(Remote::new(a_to_b_r, b_to_a_w));
///     let mut learned: Vec<String> = Vec::new();
///     peer.gossip(
///         Local::<String>::for_party("bob"),
///         |_, _, m| learned.push(m.as_ref().clone()),
///     )
///     .unwrap();
///     learned
/// });
///
/// // Alice has a single rumor and gossips it across.
/// let mut alice_peer = Sync(Remote::<String, _, _>::new(b_to_a_r, a_to_b_w));
/// let mut alice: Local<String> = Local::for_party("alice");
/// alice.message(["hello".to_string()], |_, _, _| {});
/// alice_peer.gossip(alice, |_, _, _| {}).unwrap();
///
/// assert_eq!(bob_thread.join().unwrap(), vec!["hello".to_string()]);
/// ```
#[derive(Clone, Debug)]
pub struct Sync<T, R, W>(pub Remote<T, R, W>);

/// The error type returned by [`Remote::gossip`] and [`Sync::gossip`].
///
/// # Example
///
/// ```
/// use rumors::{Error, Local, Remote};
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// // A reader that has already closed surfaces as an I/O error during gossip.
/// let (a, b) = tokio::io::duplex(64);
/// let (a_r, a_w) = tokio::io::split(a);
/// drop(b);
///
/// let mut peer = Remote::<String, _, _>::new(a_r, a_w);
/// let result = peer.gossip(Local::for_party("alice"), |_, _, _| {}).await;
/// assert!(matches!(result, Err(Error::Io(_))));
/// # }
/// ```
pub use mirror::remote::Error;

/// An opaque identifier for a single message in a [`Local`] rumor set.
///
/// Keys are produced by the `on_message` callbacks of [`Local::message`],
/// [`Local::process`], and [`Remote::gossip`]. They are stable across peers:
/// the key for any given message is the same on every peer that observes it,
/// so a key obtained from one peer can be used to redact the message on any
/// other.
///
/// Two *content-identical* messages always receive distinct keys — every
/// insert advances the local party's version vector before its key is
/// derived, so even repeated submission of the same value within a single
/// [`Local::message`] batch yields distinct keys.
///
/// # Example
///
/// ```
/// use rumors::{Key, Local};
///
/// let mut alice: Local<String> = Local::for_party("alice");
/// let mut keys: Vec<Key> = Vec::new();
/// // The same value inserted twice, in the same batch, still gets two
/// // distinct keys.
/// alice.message(
///     ["echo".to_string(), "echo".to_string()],
///     |k, _, _| keys.push(k),
/// );
/// assert_eq!(keys.len(), 2);
/// assert_ne!(keys[0], keys[1]);
/// ```
pub use tree::Key;

/// A causal version vector tagging when a message was observed.
///
/// Versions are surfaced to the `on_message` callbacks of [`Local::message`],
/// [`Local::process`], and [`Remote::gossip`]. They implement a [`PartialOrd`]
/// that captures causal ordering: `a <= b` iff every party's counter in `a`
/// is at most the corresponding counter in `b`. Two versions produced by
/// concurrent events are incomparable, so `partial_cmp` returns `None`.
///
/// # Example
///
/// ```
/// use rumors::{Local, Version};
///
/// let mut alice: Local<String> = Local::for_party("alice");
/// let mut versions: Vec<Version> = Vec::new();
/// alice.message(
///     ["first".to_string(), "second".to_string()],
///     |_, v, _| versions.push(v.clone()),
/// );
/// // Successive messages from the same party are causally ordered.
/// assert!(versions[0] < versions[1]);
/// ```
pub use version::Version;

/// The [`borsh`] crate, re-exported.
///
/// Messages are serialized with borsh, so message types must implement both
/// [`BorshSerialize`](borsh::BorshSerialize) and
/// [`BorshDeserialize`](borsh::BorshDeserialize). Re-exporting borsh here
/// means callers can derive those traits without taking a separate
/// dependency on borsh themselves.
///
/// # Example
///
/// ```
/// use rumors::{Local, borsh};
///
/// #[derive(borsh::BorshSerialize, borsh::BorshDeserialize)]
/// struct Rumor {
///     subject: String,
///     count: u32,
/// }
///
/// let mut alice: Local<Rumor> = Local::for_party("alice");
/// let mut observed = 0;
/// alice.message(
///     [Rumor { subject: "weather".into(), count: 3 }],
///     |_, _, _| observed += 1,
/// );
/// assert_eq!(observed, 1);
/// ```
pub use borsh;

impl<T> Local<T> {
    /// Create a new set of rumors, localized to the given party.
    ///
    /// It is assumed that parties are *globally unique* within the context
    /// of the gossip protocol. If multiple peers identify as the same party,
    /// then unintuitive behavior, including missed messages, may occur.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::Local;
    ///
    /// let _alice: Local<String> = Local::for_party("alice");
    /// ```
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
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::Local;
    ///
    /// let mut alice: Local<String> = Local::for_party("alice");
    /// let mut observed = Vec::new();
    /// alice.message(
    ///     ["hello".to_string(), "world".to_string()],
    ///     |key, version, message| {
    ///         observed.push((key, version.clone(), message.as_ref().clone()));
    ///     },
    /// );
    /// assert_eq!(observed.len(), 2);
    /// ```
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
    /// This is how the rumor set is garbage-collected network-wide: any peer
    /// can locally decide that a message is no longer needed, and the rest
    /// of the network will follow.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::{Local, Key};
    ///
    /// let mut alice: Local<String> = Local::for_party("alice");
    /// let mut keys: Vec<Key> = Vec::new();
    /// alice.message(["transient announcement".to_string()], |k, _, _| keys.push(k));
    /// // Once we've decided this message has served its purpose, redact it:
    /// // every peer we gossip with will drop it too.
    /// alice.redact(keys);
    /// ```
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
    ///
    /// # Example
    ///
    /// The canonical use is recombining clones that have gossiped against
    /// distinct remote peers; here we approximate that with one clone that
    /// observes a fresh message and one that does not.
    ///
    /// ```
    /// use rumors::Local;
    ///
    /// let mut alice: Local<String> = Local::for_party("alice");
    /// let mut helper = alice.clone();
    ///
    /// // Suppose `helper` learned a new rumor on another task — e.g. by
    /// // gossiping with a remote peer.
    /// helper.message(["new rumor".to_string()], |_, _, _| {});
    ///
    /// // Recombine: alice learns about every message the helper observed
    /// // that alice hadn't seen.
    /// let mut learned = Vec::new();
    /// alice.process(helper, |_, _, m| learned.push(m.as_ref().clone()));
    /// assert_eq!(learned, vec!["new rumor".to_string()]);
    /// ```
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
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::Remote;
    ///
    /// // Any (R, W) pair will do; here we use a tokio in-memory duplex.
    /// let (a, b) = tokio::io::duplex(64);
    /// let (a_r, a_w) = tokio::io::split(a);
    /// let _peer = Remote::<String, _, _>::new(a_r, a_w);
    /// # drop(b);
    /// ```
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
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::{Local, Remote};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// // Stand in for a real network with an in-memory bidirectional pipe.
    /// let (a, b) = tokio::io::duplex(1024);
    /// let (a_r, a_w) = tokio::io::split(a);
    /// let (b_r, b_w) = tokio::io::split(b);
    ///
    /// let mut alice: Local<String> = Local::for_party("alice");
    /// alice.message(["hello".to_string()], |_, _, _| {});
    ///
    /// let mut alice_peer = Remote::new(a_r, a_w);
    /// let mut bob_peer = Remote::new(b_r, b_w);
    ///
    /// // Both ends drive the protocol concurrently. Bob learns "hello".
    /// let mut bob_learned: Vec<String> = Vec::new();
    /// let (alice, bob) = tokio::join!(
    ///     alice_peer.gossip(alice, |_, _, _| {}),
    ///     bob_peer.gossip(
    ///         Local::<String>::for_party("bob"),
    ///         |_, _, m| bob_learned.push(m.as_ref().clone()),
    ///     ),
    /// );
    /// let (_alice, _bob) = (alice.unwrap(), bob.unwrap());
    /// assert_eq!(bob_learned, vec!["hello".to_string()]);
    /// # }
    /// ```
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
    ///
    /// # Example
    ///
    /// ```
    /// use std::io::pipe;
    /// use std::thread;
    /// use rumors::{Local, Remote, Sync};
    ///
    /// let (a_to_b_r, a_to_b_w) = pipe().unwrap();
    /// let (b_to_a_r, b_to_a_w) = pipe().unwrap();
    ///
    /// // Bob runs on another thread, reading from one pipe and writing the
    /// // other, and reports back the messages he learns.
    /// let bob_thread = thread::spawn(move || {
    ///     let mut peer = Sync(Remote::new(a_to_b_r, b_to_a_w));
    ///     let mut learned: Vec<String> = Vec::new();
    ///     peer.gossip(
    ///         Local::<String>::for_party("bob"),
    ///         |_, _, m| learned.push(m.as_ref().clone()),
    ///     )
    ///     .unwrap();
    ///     learned
    /// });
    ///
    /// let mut alice_peer = Sync(Remote::<String, _, _>::new(b_to_a_r, a_to_b_w));
    /// let mut alice: Local<String> = Local::for_party("alice");
    /// alice.message(["hello".to_string()], |_, _, _| {});
    /// alice_peer.gossip(alice, |_, _, _| {}).unwrap();
    ///
    /// assert_eq!(bob_thread.join().unwrap(), vec!["hello".to_string()]);
    /// ```
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
