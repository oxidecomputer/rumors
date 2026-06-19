//! A synchronous interface to the crate, for applications without an async
//! runtime.
//!
//! The blocking face is the same
//! [`Peer`](crate::Peer)/[`Rumors`](crate::Rumors) engine in [`Blocking`] mode:
//! it blocks the calling thread wherever the async face would await. The types
//! here are aliases that pin the mode, so `sync::Peer::seed()` and
//! `sync::Peer::bootstrap(..)` build a blocking peer without naming the mode
//! more explicitly. Async functions become ordinary functions,
//! [`Stream`](futures::Stream)s become [`Iterator`]s, and gossip runs over
//! [`std::io::Read`]/[`Write`](std::io::Write) instead of [`tokio`]'s
//! [`AsyncRead`](tokio::io::AsyncRead)/[`AsyncWrite`](tokio::io::AsyncWrite).
//!
//! Modulo blocking, the behavior is almost identical to the main asynchronous
//! interface; each blocking method links to its async counterpart for the
//! contract. Read the [crate docs](crate) first.
//!
//! # Differences from the asynchronous interface
//!
//! Blocking calls cannot be cancelled: where the main asynchronous interface
//! lets you drop a session future ([crate
//! docs](crate#what-a-session-promises)), the synchronous interface returns
//! only when the session has finished or failed. Use your transport's own
//! timeouts (e.g. socket read timeouts, which surface here as session errors)
//! to bound a stalled counterparty.
//!
//! The change-driven driver ([`crate::Rumors::gossip_when`]) has no blocking
//! equivalent: it is one task racing a policy stream against the wire, which is
//! concurrency a blocking call cannot express. A blocking application schedules
//! its own [`gossip`](crate::Rumors::gossip) calls instead; [`Changes`] can
//! wake other change-driven work, but is not a gossip schedule by itself,
//! because one must wait concurrently for local and remote changes to implement
//! bidirectional push-based gossip.
//!
//! # Example
//!
//! The crate-root example, with no runtime anywhere: plain threads and OS
//! pipes.
//!
//! ```
//! use rumors::sync::Peer;
//!
//! let alice = Peer::<String>::seed().into_rumors();
//! alice.send("the meeting is at noon".to_string());
//!
//! // Any Read/Write pair carries a session; here, two OS pipes.
//! let (mut alice_read, mut bob_write) = std::io::pipe()?;
//! let (mut bob_read, mut alice_write) = std::io::pipe()?;
//!
//! // Alice serves one gossip session from a plain thread...
//! let mut serve = alice.clone();
//! let serving = std::thread::spawn(move || {
//!     serve.gossip(&mut alice_read, &mut alice_write)
//! });
//!
//! // ...and Bob joins through it, blocking until he holds a full replica.
//! let bob = Peer::<String>::bootstrap(&mut bob_read, &mut bob_write)?
//!     .expect("alice is established, not herself bootstrapping");
//! serving.join().expect("serving thread panicked")?;
//!
//! let snapshot = bob.into_rumors().snapshot();
//! let (_key, _version, message) = snapshot.iter().next().expect("one live message");
//! assert_eq!(message.as_str(), "the meeting is at noon");
//! # Ok::<(), rumors::Error>(())
//! ```

use std::collections::BTreeMap;

use crate::Blocking;
use crate::bookmark::{NoBookmark, Persist, format};
use ::before::Clock;

// Re-exports to keep `crate::sync` at parity with `crate`:
pub use crate::bookmark::{
    BOOKMARK_FORMAT_VERSION, BOOKMARK_MAGIC, BookmarkError, BookmarkIo, FormatError,
};
pub use crate::{
    Batch, Error, Key, MERKLE_HASH_LEN, Network, PROTOCOL_MAGIC, PROTOCOL_VERSION, Snapshot,
    Version, causally,
    rumors::{TryNext, TryTick},
};
pub use ::before;
pub use ::borsh;

/// The synchronous [`Peer`](crate::Peer).
pub type Peer<T, B = NoBookmark> = crate::Peer<T, B, Blocking>;

/// The synchronous [`Rumors`](crate::Rumors).
pub type Rumors<T, B = NoBookmark> = crate::Rumors<T, B, Blocking>;

/// The synchronous [`Retire`](crate::Retire).
pub type Retire<T, B = NoBookmark> = crate::Retire<T, B, Blocking>;

/// The synchronous [`Unbookmarked`](crate::Unbookmarked).
pub type Unbookmarked<T, B = NoBookmark> = crate::Unbookmarked<T, B, Blocking>;

/// The synchronous [`UnorderedMessages`](crate::UnorderedMessages).
pub type Messages<T> = crate::UnorderedMessages<T, Blocking>;

/// The synchronous [`CausalMessages`](crate::CausalMessages).
pub type CausalMessages<T> = crate::CausalMessages<T, Blocking>;

/// The synchronous [`Changes`](crate::Changes).
pub type Changes<T> = crate::Changes<T, Blocking>;

/// The synchronous [`Bookmark`](crate::Bookmark).
///
/// The blocking counterpart of the async trait: it lends plain
/// [`Read`](std::io::Read)/[`Write`](std::io::Write) byte storage and the crate
/// owns the framed format.
pub trait Bookmark: BookmarkError {
    /// The byte source [`load`](Self::load) hands back.
    type Reader: std::io::Read;

    /// Open the stored record for reading, or `Ok(None)` if nothing is stored.
    ///
    /// The blocking [`Bookmark::load`](crate::Bookmark::load).
    fn load(&self) -> Result<Option<Self::Reader>, Self::Error>;

    /// Atomically replace the stored record by writing the crate's serialized
    /// frame into the lent writer.
    ///
    /// The blocking [`Bookmark::store`](crate::Bookmark::store).
    fn store<F>(&self, write: F) -> Result<(), Self::Error>
    where
        F: FnOnce(&mut dyn std::io::Write) -> std::io::Result<()>;
}

impl Bookmark for NoBookmark {
    type Reader = std::io::Empty;

    fn load(&self) -> Result<Option<Self::Reader>, Self::Error> {
        Ok(None)
    }

    fn store<F>(&self, _write: F) -> Result<(), Self::Error>
    where
        F: FnOnce(&mut dyn std::io::Write) -> std::io::Result<()>,
    {
        Ok(())
    }
}

/// Give a blocking [`Bookmark`] the awaitable shape the engine's one async body
/// expects, by settling each blocking call into a ready future.
impl<B: Bookmark> Persist<Blocking> for B {
    fn read(
        &self,
    ) -> impl Future<Output = Result<BTreeMap<Network, Vec<Clock>>, BookmarkIo<Self::Error>>> + Send
    {
        std::future::ready((|| match Bookmark::load(self).map_err(BookmarkIo::Io)? {
            None => Ok(BTreeMap::new()),
            Some(mut reader) => {
                let mut bytes = Vec::new();
                std::io::Read::read_to_end(&mut reader, &mut bytes)
                    .map_err(|e| BookmarkIo::Format(FormatError::Read(e)))?;
                format::decode(&bytes).map_err(BookmarkIo::Format)
            }
        })())
    }

    fn write(
        &self,
        bookmarks: &BTreeMap<Network, Vec<Clock>>,
    ) -> impl Future<Output = Result<(), BookmarkIo<Self::Error>>> + Send {
        let bytes = format::encode(bookmarks);
        std::future::ready(Bookmark::store(self, |w| w.write_all(&bytes)).map_err(BookmarkIo::Io))
    }
}
