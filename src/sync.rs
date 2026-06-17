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
use crate::bookmark::{NoBookmark, Persist};
use ::before::Clock;

// Re-exports to keep `crate::sync` at parity with `crate`:
pub use crate::bookmark::BookmarkError;
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

/// The synchronous [`Messages`](crate::Messages).
pub type Messages<T> = crate::Messages<T, Blocking>;

/// The synchronous [`CausalMessages`](crate::CausalMessages).
pub type CausalMessages<T> = crate::CausalMessages<T, Blocking>;

/// The synchronous [`Changes`](crate::Changes).
pub type Changes<T> = crate::Changes<T, Blocking>;

/// The synchronous [`Bookmark`](crate::Bookmark).
pub trait Bookmark: BookmarkError {
    /// Read the persisted record, or an empty map if nothing is stored yet.
    fn read(&self) -> Result<BTreeMap<Network, Vec<Clock>>, Self::Error>;

    /// Durably (atomically) replace the persisted record with `bookmarks`.
    fn write(&self, bookmarks: &BTreeMap<Network, Vec<Clock>>) -> Result<(), Self::Error>;
}

impl Bookmark for NoBookmark {
    fn read(&self) -> Result<BTreeMap<Network, Vec<Clock>>, Self::Error> {
        Ok(Default::default())
    }

    fn write(&self, _: &BTreeMap<Network, Vec<Clock>>) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// Give a blocking [`Bookmark`] the awaitable shape the engine's one async body
/// expects, by settling each blocking call into a ready future. Sound because a
/// [`Blocking`] peer drives that body to completion with [`pollster`] on the
/// calling thread: the "await" never yields to a runtime, so blocking inside it
/// blocks exactly the thread the caller already devoted to the session.
impl<B: Bookmark> Persist<Blocking> for B {
    fn read(
        &self,
    ) -> impl Future<Output = Result<BTreeMap<Network, Vec<Clock>>, Self::Error>> + Send {
        std::future::ready(Bookmark::read(self))
    }

    fn write(
        &self,
        bookmarks: &BTreeMap<Network, Vec<Clock>>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        std::future::ready(Bookmark::write(self, bookmarks))
    }
}
