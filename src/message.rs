use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use borsh::{BorshDeserialize, BorshSerialize};
use bytes::Bytes;

/// A message of type `T` paired with its cached serialization.
///
/// The cache avoids repeated roundtrips through serialization: a `Message<T>`
/// always serializes identically to a `T`. Cloning is cheap, because the
/// serialized bytes are shared and the message is enclosed in an `Arc<T>`.
///
/// # Panics
///
/// All messages of type `T` are assumed serializable; methods that attempt
/// serialization panic if serialization fails.
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

impl<T> Message<T> {
    /// Creates a `Message` pairing the given object with its cached
    /// serialization.
    ///
    /// # Panics
    ///
    /// If the message cannot be serialized.
    pub fn new(message: T) -> Self
    where
        T: BorshSerialize,
    {
        Message {
            serialized: Bytes::from(borsh::to_vec(&message).unwrap()),
            message: Arc::new(message),
        }
    }

    /// Creates a `Message` pairing the given serialized bytes with the
    /// object derived by deserializing them.
    pub fn from_slice(bytes: &[u8]) -> borsh::io::Result<Self>
    where
        T: BorshDeserialize,
    {
        Ok(Message {
            message: Arc::new(borsh::from_slice(bytes)?),
            serialized: Bytes::copy_from_slice(bytes),
        })
    }

    /// Creates a `Message` from already-shared serialized bytes, without
    /// copying.
    ///
    /// The bytes are deserialized to produce the paired object.
    pub fn from_bytes(bytes: Bytes) -> borsh::io::Result<Self>
    where
        T: BorshDeserialize,
    {
        Ok(Message {
            message: Arc::new(borsh::from_slice(bytes.as_ref())?),
            serialized: bytes,
        })
    }

    /// Creates a `Message` from an existing [`Arc`], without copying.
    ///
    /// # Panics
    ///
    /// If the message cannot be serialized.
    pub fn from_arc(arc: Arc<T>) -> Self
    where
        T: BorshSerialize,
    {
        Message {
            serialized: Bytes::from(borsh::to_vec(&*arc).unwrap()),
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
    /// Used by enumeration paths (the tree's point lookups and borrowing
    /// test-oracle walks) that hand out borrowed `&Arc<T>` exactly as the
    /// public observers clone them out.
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

impl<T: BorshSerialize> From<T> for Message<T> {
    /// Creates a `Message` pairing the given object with its cached
    /// serialization.
    ///
    /// # Panics
    ///
    /// If the message cannot be serialized.
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

// Borsh impls let `Message<T>` nest inside other borsh types with the same
// on-the-wire representation as `T` itself.

impl<T> BorshSerialize for Message<T> {
    fn serialize<W: borsh::io::Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
        // Write the cached bytes directly: the whole point of `Message<T>` is
        // to avoid reserializing.
        writer.write_all(&self.serialized)
    }
}

impl<T: BorshDeserialize> BorshDeserialize for Message<T> {
    fn deserialize_reader<R: borsh::io::Read>(reader: &mut R) -> borsh::io::Result<Self> {
        // Tee the reader so we capture exactly the bytes consumed while
        // parsing `T`, and use them as the cached serialization.
        let mut captured = Vec::new();
        let mut tee = TeeReader {
            inner: reader,
            buf: &mut captured,
        };
        let message = Arc::new(T::deserialize_reader(&mut tee)?);
        Ok(Message {
            message,
            serialized: captured.into(),
        })
    }
}

struct TeeReader<'a, R: ?Sized> {
    inner: &'a mut R,
    buf: &'a mut Vec<u8>,
}

impl<R: borsh::io::Read + ?Sized> borsh::io::Read for TeeReader<'_, R> {
    fn read(&mut self, out: &mut [u8]) -> borsh::io::Result<usize> {
        let n = self.inner.read(out)?;
        self.buf.extend_from_slice(&out[..n]);
        Ok(n)
    }
}

#[cfg(test)]
mod tests;
