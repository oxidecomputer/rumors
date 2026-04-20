use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};

use borsh::{BorshDeserialize, BorshSerialize};
use bytes::Bytes;

/// A message paired with its cached serialization, to avoid roundtripping
/// repeatedly through serialization/deserialization.
///
/// If it is cheap to clone `T`, then it is also cheap to clone `Message<T>`,
/// because the serialized bytes are shared.
///
/// It is assumed that all messages of type `T` are serializable; methods that
/// attempt serialization will panic in the event that serialization fails.
#[derive(Clone)]
pub struct Message<T> {
    message: T,
    serialized: Bytes,
}

impl<T> Message<T> {
    /// Create a new `Message` pairing the given object with its cached
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
            message,
        }
    }

    /// Create a new `Message` pairing the given serialized bytes with the
    /// object derived by deserializing them.
    pub fn from_slice(bytes: &[u8]) -> borsh::io::Result<Self>
    where
        T: BorshDeserialize,
    {
        Ok(Message {
            message: borsh::from_slice(bytes)?,
            serialized: Bytes::copy_from_slice(bytes),
        })
    }

    /// Create a new `Message` from already-shared serialized bytes, without
    /// copying. The bytes are deserialized to produce the paired object.
    pub fn from_bytes(bytes: Bytes) -> borsh::io::Result<Self>
    where
        T: BorshDeserialize,
    {
        Ok(Message {
            message: borsh::from_slice(bytes.as_ref())?,
            serialized: bytes,
        })
    }

    /// Get a reference to the object represented by this message.
    pub fn message(&self) -> &T {
        &self.message
    }

    /// Get the serialized bytes corresponding to this message.
    pub fn bytes(&self) -> &[u8] {
        self.serialized.as_ref()
    }

    /// Get a cheaply-clonable handle to the shared serialized bytes.
    pub fn shared_bytes(&self) -> Bytes {
        self.serialized.clone()
    }

    /// Borrow the inner object mutably through a guard that recomputes the
    /// cached serialization when dropped.
    ///
    /// # Panics
    ///
    /// The returned guard panics on drop if reserializing the mutated value
    /// fails. Recovering the old serialization isn't meaningful, since it no
    /// longer matches the object, so there's no way to recover except to panic.
    pub fn as_mut(&mut self) -> MessageMut<'_, T>
    where
        T: BorshSerialize,
    {
        MessageMut { message: self }
    }

    /// Consume the message and return the inner object, dropping the cached
    /// serialization.
    pub fn into_inner(self) -> T {
        self.message
    }

    /// Consume the message and return the inner object along with the shared
    /// serialized bytes.
    pub fn into_parts(self) -> (T, Bytes) {
        (self.message, self.serialized)
    }
}

impl<T: BorshSerialize> From<T> for Message<T> {
    fn from(message: T) -> Self {
        Self::new(message)
    }
}

impl<T> AsRef<T> for Message<T> {
    fn as_ref(&self) -> &T {
        &self.message
    }
}

/// RAII guard returned by [`Message::as_mut`]. Dereferences to the inner `T`
/// and recomputes the cached serialization when dropped.
pub struct MessageMut<'a, T: BorshSerialize> {
    message: &'a mut Message<T>,
}

impl<T: BorshSerialize> Deref for MessageMut<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.message.message
    }
}

impl<T: BorshSerialize> DerefMut for MessageMut<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.message.message
    }
}

impl<T: BorshSerialize> Drop for MessageMut<'_, T> {
    fn drop(&mut self) {
        match borsh::to_vec(&self.message.message) {
            Ok(bytes) => self.message.serialized = bytes.into(),
            Err(e) => panic!("failed to reserialize Message on drop: {e}"),
        }
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
        let message = T::deserialize_reader(&mut tee)?;
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
mod test;
