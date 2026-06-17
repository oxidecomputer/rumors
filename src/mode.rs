//! The I/O mode witness: a type-level selector between the async and blocking
//! faces of [`Peer`](crate::Peer) and [`Rumors`](crate::Rumors).
//!
//! A replica's engine is identical either way; the mode chooses only *how a
//! caller drives I/O*. [`Async`] sessions and observers are plain futures and
//! streams over [`tokio::io`]; [`Blocking`] ones drive that same machinery to
//! completion with [`pollster`] over [`std::io`], for callers with no async
//! runtime.
//!
//! The mode is a phantom type parameter (`M`) on the handle types, defaulting
//! to [`Async`]. It never flows as a value and is fixed at construction:
//! [`Peer::seed`](crate::Peer::seed) and friends produce an [`Async`] peer,
//! while the [`sync`](crate::sync) module's aliases produce a [`Blocking`] one.
//! There is no method to convert between modes — a bookmarked peer's mode is
//! tied to the kind of bookmark it carries.

mod sealed {
    pub trait Sealed {}
}

/// Witnesses how a [`Peer`](crate::Peer) or [`Rumors`](crate::Rumors) performs
/// I/O.
///
/// This trait is sealed: the only modes are [`Async`] and [`Blocking`]. It is a
/// pure type-level marker — it has no methods, and no value of an implementing
/// type is ever constructed.
pub trait Mode: sealed::Sealed {}

/// Futures-based I/O over [`tokio::io::AsyncRead`]/[`AsyncWrite`]. The default
/// mode.
///
/// [`AsyncWrite`]: tokio::io::AsyncWrite
#[derive(Debug)]
pub struct Async;

/// Blocking I/O over [`std::io::Read`]/[`Write`].
///
/// [`Write`]: std::io::Write
#[derive(Debug)]
pub struct Blocking;

impl sealed::Sealed for Async {}
impl sealed::Sealed for Blocking {}

impl Mode for Async {}
impl Mode for Blocking {}
