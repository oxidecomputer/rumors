//! Synchronous mirror of the [crate root](crate).
//!
//! Same types and methods, but callbacks are `FnMut` instead of `AsyncFnMut`,
//! and I/O is [`std::io::Read`] / [`std::io::Write`] instead of
//! [`AsyncRead`](tokio::io::AsyncRead) / [`AsyncWrite`](tokio::io::AsyncWrite).
//! Use this module when your I/O is synchronous (e.g.
//! [`std::net::TcpStream`]).
//!
//! The two surfaces are distinct at the type level: a [`sync::Local`](Local)
//! is not a [`crate::Local`], and [`ignore`] cannot be passed to
//! [`crate::Local::message`] (which requires an async callback) or vice
//! versa.
//!
//! # Quickstart
//!
//! ```
//! use rumors::sync::Local;
//!
//! let mut alice: Local<String> = Local::for_party("alice");
//! let mut observed = 0;
//! alice.message(
//!     ["hello".to_string(), "world".to_string()],
//!     |_key, _version, _message| observed += 1,
//! );
//! assert_eq!(observed, 2);
//! ```

use std::io::{Read, Write};
use std::ops::{Add, AddAssign};
use std::sync::Arc;

use borsh::{BorshDeserialize, BorshSerialize};
use futures::io::AllowStdIo;
use tokio_util::compat::{FuturesAsyncReadCompatExt, FuturesAsyncWriteCompatExt};

/// The error type returned by [`Local::gossip`]. Re-export of
/// [`crate::Error`].
///
/// Surfaces I/O failures from the underlying reader/writer as well as
/// framing errors encountered while parsing messages off the wire.
pub use crate::mirror::remote::Error;

/// An opaque identifier for a single message in a [`Local`] rumor set.
/// Re-export of [`crate::Key`].
///
/// Keys are produced by the `on_message` callbacks of [`Local::message`],
/// [`Local::process`], and [`Local::gossip`], and are stable across peers:
/// a key obtained from one peer can redact the message on any other.
///
/// Two content-identical messages always receive distinct keys: every
/// insert advances the local version vector before the key is derived.
///
/// # Example
///
/// ```
/// use rumors::sync::{Local, Key};
///
/// let mut alice: Local<String> = Local::for_party("alice");
/// let mut keys: Vec<Key> = Vec::new();
/// alice.message(
///     ["echo".to_string(), "echo".to_string()],
///     |k, _, _| keys.push(k),
/// );
/// assert_ne!(keys[0], keys[1]);
/// ```
pub use crate::tree::Key;

/// A causal version vector tagging when a message was observed. Re-export
/// of [`crate::Version`].
///
/// Surfaced to the `on_message` callbacks of [`Local::message`],
/// [`Local::process`], and [`Local::gossip`]. [`PartialOrd`] captures causal
/// ordering: `a <= b` iff every party's counter in `a` is at most the
/// corresponding counter in `b`. Versions produced by concurrent events are
/// incomparable (`partial_cmp` returns `None`).
///
/// # Example
///
/// ```
/// use rumors::sync::{Local, Version};
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
pub use crate::version::Version;

/// The [`borsh`] crate, re-exported. Same export as [`crate::borsh`].
///
/// Message types must implement [`BorshSerialize`] and [`BorshDeserialize`];
/// re-exporting borsh here lets callers derive both without a separate
/// dependency.
///
/// # Example
///
/// ```
/// use rumors::sync::{Local, borsh, ignore};
///
/// #[derive(borsh::BorshSerialize, borsh::BorshDeserialize)]
/// struct Rumor { subject: String, count: u32 }
///
/// let mut alice: Local<Rumor> = Local::for_party("alice");
/// alice.message(
///     [Rumor { subject: "weather".into(), count: 3 }],
///     ignore,
/// );
/// ```
pub use ::borsh;

/// A local set of rumors with synchronous callbacks.
///
/// Wraps [`crate::Local`], exposing [`message`](Self::message),
/// [`process`](Self::process), and [`gossip`](Self::gossip) with `FnMut`
/// rather than `AsyncFnMut`. The wrapped async [`Local`](crate::Local) is
/// publicly accessible as `.0`, providing an escape hatch into the async
/// surface when the caller has its own runtime.
///
/// # Example
///
/// ```
/// use rumors::sync::Local;
/// use rumors::Key;
///
/// let mut alice: Local<String> = Local::for_party("alice");
/// let mut keys: Vec<Key> = Vec::new();
/// alice.message(
///     ["hello".to_string(), "world".to_string()],
///     |key, _, _| keys.push(key),
/// );
/// alice.redact([keys[0]]);
/// ```
#[derive(Debug, Eq)]
pub struct Local<T>(pub crate::Local<T>);

// Hand-rolled Clone/PartialEq so they don't impose `T: Clone`/`T: PartialEq`
// the way `derive` would. `crate::Local<T>` itself is unconditionally
// Clone+PartialEq (the inner tree is structurally shared and compared by
// hash), so the synchronous mirror should match that contract.
impl<T> Clone for Local<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> PartialEq for Local<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

/// An `on_message` callback that discards every observation.
///
/// Pass this when you only care about mutating the rumor set, not about
/// inspecting individual messages. Sync analogue of [`crate::ignore`]; the
/// two are not interchangeable at the type level.
///
/// # Example
///
/// ```
/// use rumors::sync::{Local, ignore};
///
/// let mut alice: Local<String> = Local::for_party("alice");
/// alice.message(["hello".to_string(), "world".to_string()], ignore);
/// ```
pub fn ignore<T>(_key: Key, _version: &Version, _message: &Arc<T>) {}

impl<T> Local<T> {
    /// Create an empty rumor set tagged with the given party identifier.
    ///
    /// Party identifiers must be globally unique within the gossip network;
    /// reusing one across peers causes missed messages and undefined
    /// behavior.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::sync::Local;
    ///
    /// let _alice: Local<String> = Local::for_party("alice");
    /// ```
    pub fn for_party(party: impl AsRef<[u8]>) -> Self {
        Self(crate::Local::for_party(party))
    }

    /// Insert messages into the rumor set, invoking `on_message` once per
    /// newly-observed message.
    ///
    /// The callback receives an opaque [`Key`] (usable later with
    /// [`redact`](Self::redact)), the causal [`Version`] at which the message
    /// was observed, and an [`Arc<T>`](Arc) holding the message. Callback
    /// order is unspecified.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::sync::Local;
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
        pollster::block_on(
            self.0
                .message(messages, async |k, v, m| on_message(k, v, m)),
        );
    }

    /// Redact the given keys: stop gossiping the corresponding messages, and
    /// instruct every peer we synchronize with to do the same.
    ///
    /// Redaction is contagious — a single peer's call evicts the message
    /// network-wide without consensus. This is a forwarding wrapper around
    /// [`crate::Local::redact`], which is already synchronous; it exists so
    /// the [`Local`] surface is uniform.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::sync::Local;
    /// use rumors::Key;
    ///
    /// let mut alice: Local<String> = Local::for_party("alice");
    /// let mut keys: Vec<Key> = Vec::new();
    /// alice.message(["transient".to_string()], |k, _, _| keys.push(k));
    /// alice.redact(keys);
    /// ```
    pub fn redact<I: IntoIterator<Item = Key>>(&mut self, redacted: I) {
        self.0.redact(redacted);
    }

    /// Merge `new` into `self`, invoking `on_message` for each message in
    /// `new` that `self` had not already observed.
    ///
    /// The callback signature matches [`Local::message`]. Messages present
    /// in `self` but missing from `new` do not fire it.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::sync::{Local, ignore};
    ///
    /// let mut alice: Local<String> = Local::for_party("alice");
    /// let mut helper = alice.clone();
    ///
    /// // Suppose `helper` learned a new rumor on another thread.
    /// helper.message(["new rumor".to_string()], ignore);
    ///
    /// let mut learned = Vec::new();
    /// alice.process(helper, |_, _, m| learned.push(m.as_ref().clone()));
    /// assert_eq!(learned, vec!["new rumor".to_string()]);
    /// ```
    pub fn process<OnMessage>(&mut self, new: Local<T>, mut on_message: OnMessage)
    where
        OnMessage: FnMut(Key, &Version, &Arc<T>),
    {
        pollster::block_on(self.0.process(new.0, async |k, v, m| on_message(k, v, m)));
    }

    /// Synchronize rumor sets with a remote peer over synchronous I/O,
    /// invoking `on_message` for each message learned from the peer.
    ///
    /// Bridges the sync reader/writer into the crate's async core via
    /// [`AllowStdIo`] and [`pollster::block_on`]. As with
    /// [`crate::Local::gossip`], both ends must drive `gossip` concurrently
    /// (typically on separate threads). The callback signature matches
    /// [`Local::message`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rumors::sync::{Local, ignore};
    /// use std::net::TcpStream;
    ///
    /// let mut write = TcpStream::connect("127.0.0.1:9000").unwrap();
    /// let mut read = write.try_clone().unwrap();
    /// let alice: Local<String> = Local::for_party("alice");
    /// let _alice = alice.gossip(&mut read, &mut write, ignore).unwrap();
    /// ```
    pub fn gossip<OnMessage, R, W>(
        self,
        read: &mut R,
        write: &mut W,
        mut on_message: OnMessage,
    ) -> Result<Self, Error>
    where
        T: BorshDeserialize + BorshSerialize,
        R: Read + Unpin,
        W: Write + Unpin,
        OnMessage: FnMut(Key, &Version, &Arc<T>),
    {
        // Bridge the synchronous reader/writer to the async I/O the protocol
        // expects: `AllowStdIo` adapts `Read`/`Write` to `futures::io`'s async
        // traits, and the tokio-compat layer adapts those to `tokio::io`'s.
        let mut read = AllowStdIo::new(read).compat();
        let mut write = AllowStdIo::new(write).compat_write();
        pollster::block_on(
            self.0
                .gossip(&mut read, &mut write, async |k, v, m| on_message(k, v, m)),
        )
        .map(Local)
    }
}

/// Combine two rumor sets via [`Local::process`].
impl<T> Add for Local<T> {
    type Output = Local<T>;

    fn add(mut self, rhs: Self) -> Self::Output {
        self.process(rhs, ignore);
        self
    }
}

/// Absorb `rhs` into `self` via [`Local::process`].
impl<T> AddAssign for Local<T> {
    fn add_assign(&mut self, rhs: Self) {
        *self = self.clone().add(rhs);
    }
}
