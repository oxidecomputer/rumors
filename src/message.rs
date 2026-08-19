use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::io;
use std::sync::Arc;

use bytes::Bytes;

use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;
use serde::de::DeserializeOwned;
/// A message of type `T` paired with its cached serialization.
///
/// The cache avoids repeated roundtrips through serialization: a `Message<T>`
/// always carries the exact CBOR bytes its `T` was encoded to or decoded
/// from. Cloning is cheap, because the serialized bytes are shared and the
/// message is enclosed in an `Arc<T>`.
///
/// The payload encoding is CBOR (via [`ciborium`]): self-describing, so
/// field and variant *names* are the wire contract — a decoder pairs fields
/// by name, tolerating reordering — and no canonical encoding is required
/// of `T`, because payload bytes carry no identity (a leaf's identity is
/// its version).
///
/// # Panics
///
/// Every value of `T` must serialize: methods that serialize (`new`,
/// `from_arc`, `From<T>`) panic if `T`'s [`serde::Serialize`]
/// implementation reports an error. Encoding runs into an in-memory
/// buffer and CBOR imposes no format-driven failures (any map key, any
/// nesting), so the only trigger is the implementation itself declining a
/// value — which this crate treats as a bug in `T`, exactly as `Ord`'s
/// totality is trusted. Types whose `Serialize` is data-dependently
/// fallible (for example `std::path::PathBuf`, which errors on non-UTF-8
/// paths) violate that obligation and must not be used as message types.
pub struct Message<T> {
    message: Arc<T>,
    serialized: Bytes,
}

impl<T> Clone for Message<T> {
    fn clone(&self) -> Self {
        Self {
            message: self.message.clone(),
            serialized: self.serialized.clone(),
        }
    }
}

/// Map a ciborium deserialization failure into `io::Error`, keeping the
/// truncation/corruption split callers classify by: a reader's own error
/// passes through, everything else is invalid data.
fn de_error(error: ciborium::de::Error<io::Error>) -> io::Error {
    match error {
        ciborium::de::Error::Io(error) => error,
        error => io::Error::new(io::ErrorKind::InvalidData, error.to_string()),
    }
}

/// Encode one value as CBOR into a fresh buffer.
///
/// # Panics
///
/// If `T`'s `Serialize` implementation reports an error ([`Message`]'s
/// panic contract: serializability is the caller's obligation). Writing
/// into a `Vec` cannot fail.
fn to_vec<T: Serialize>(value: &T) -> Vec<u8> {
    let mut buf = Vec::new();
    ciborium::ser::into_writer(value, &mut buf)
        .expect("every message value must serialize (see Message's panic contract)");
    buf
}

impl<T> Message<T> {
    /// Creates a `Message` pairing the given object with its cached
    /// serialization.
    ///
    /// # Panics
    ///
    /// If the message cannot be serialized (see [`Message`]).
    pub fn new(message: T) -> Self
    where
        T: Serialize,
    {
        Message {
            serialized: Bytes::from(to_vec(&message)),
            message: Arc::new(message),
        }
    }

    /// Creates a `Message` pairing the given serialized bytes with the
    /// object derived by deserializing them.
    ///
    /// The bytes must be exactly one CBOR value: trailing bytes are
    /// rejected as invalid data, so the cache is always the value's exact
    /// encoding.
    pub fn from_slice(bytes: &[u8]) -> io::Result<Self>
    where
        T: DeserializeOwned,
    {
        let mut input = bytes;
        let message: T = ciborium::de::from_reader(&mut input).map_err(de_error)?;
        if !input.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{} trailing bytes after the message payload", input.len()),
            ));
        }
        Ok(Message {
            message: Arc::new(message),
            serialized: Bytes::copy_from_slice(bytes),
        })
    }

    /// Pairs an already-decoded object with the exact bytes it was decoded
    /// from.
    ///
    /// The caller certifies the pairing: `serialized` must be exactly the
    /// CBOR encoding `message` was parsed out of (the wire codec's record
    /// parser upholds this — it hands over precisely the bytes its parse
    /// consumed).
    pub(crate) fn from_decoded(message: T, serialized: Bytes) -> Self {
        Message {
            message: Arc::new(message),
            serialized,
        }
    }

    /// Creates a `Message` from already-shared serialized bytes, without
    /// copying.
    ///
    /// The bytes are deserialized to produce the paired object, under
    /// [`from_slice`](Self::from_slice)'s exactly-one-value contract.
    pub fn from_bytes(bytes: Bytes) -> io::Result<Self>
    where
        T: DeserializeOwned,
    {
        let mut input = bytes.as_ref();
        let message: T = ciborium::de::from_reader(&mut input).map_err(de_error)?;
        if !input.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{} trailing bytes after the message payload", input.len()),
            ));
        }
        Ok(Message {
            message: Arc::new(message),
            serialized: bytes,
        })
    }

    /// Creates a `Message` from an existing [`Arc`], without copying.
    ///
    /// # Panics
    ///
    /// If the message cannot be serialized (see [`Message`]).
    pub fn from_arc(arc: Arc<T>) -> Self
    where
        T: Serialize,
    {
        Message {
            serialized: Bytes::from(to_vec(&*arc)),
            message: arc,
        }
    }

    /// Returns a reference to the object represented by this message.
    pub fn message(&self) -> &T {
        &self.message
    }

    /// Returns a reference to the shared [`Arc`] holding this message's
    /// object, without cloning it.
    ///
    /// Used by enumeration paths (e.g. [`Tree::iter`]) that hand out
    /// borrowed `&Arc<T>` exactly as the public observers do.
    ///
    /// [`Tree::iter`]: crate::tree::Tree::iter
    pub fn as_arc(&self) -> &Arc<T> {
        &self.message
    }

    /// Returns the serialized bytes corresponding to this message.
    pub fn as_slice(&self) -> &[u8] {
        self.serialized.as_ref()
    }

    /// Returns a cheaply-clonable handle to the shared serialized bytes.
    pub fn bytes(&self) -> &Bytes {
        &self.serialized
    }

    /// Consumes the message and returns the inner object, dropping the cached
    /// serialization.
    pub fn into_inner(self) -> Arc<T> {
        self.message
    }

    /// Consumes the message and returns the inner object, dropping the cached
    /// serialization and cloning the inner object if necessary.
    pub fn clone_into_inner(self) -> T
    where
        T: Clone,
    {
        Arc::unwrap_or_clone(self.message)
    }

    /// Consumes the message and returns the inner object along with the
    /// shared serialized bytes.
    pub fn into_parts(self) -> (Arc<T>, Bytes)
    where
        T: Clone,
    {
        (self.message, self.serialized)
    }
}

impl<T: Serialize> From<T> for Message<T> {
    /// Creates a `Message` pairing the given object with its cached
    /// serialization.
    ///
    /// # Panics
    ///
    /// If the message cannot be serialized (see [`Message`]).
    fn from(message: T) -> Self {
        Self::new(message)
    }
}

impl<T> AsRef<T> for Message<T> {
    fn as_ref(&self) -> &T {
        &self.message
    }
}

impl<T> AsRef<Arc<T>> for Message<T> {
    fn as_ref(&self) -> &Arc<T> {
        &self.message
    }
}

// Manual trait implementations that treat `Message<T>` as a transparent wrapper
// around `T`, ignoring the cached serialized bytes. Two messages holding equal
// `T` values compare equal even if their cached bytes differ (e.g. produced by
// different serializer versions).

impl<T: fmt::Debug> fmt::Debug for Message<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(f)
    }
}

impl<T: PartialEq> PartialEq for Message<T> {
    fn eq(&self, other: &Self) -> bool {
        self.message == other.message
    }
}

impl<T: Eq> Eq for Message<T> {}

impl<T: PartialOrd> PartialOrd for Message<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.message.partial_cmp(&other.message)
    }
}

impl<T: Ord> Ord for Message<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.message.cmp(&other.message)
    }
}

impl<T: Hash> Hash for Message<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.message.hash(state);
    }
}

// The serde form lets `Message<T>` nest inside larger CBOR values without
// re-encoding: one byte string wrapping the cached CBOR payload. The
// wrapper is what makes a nested message self-delimiting wherever the
// container does not delimit it.

impl<T> Serialize for Message<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(&self.serialized)
    }
}

impl<'de, T: DeserializeOwned> Deserialize<'de> for Message<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bytes = <Vec<u8>>::deserialize(deserializer)?;
        Message::from_slice(&bytes).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests;
