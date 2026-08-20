use std::any::Any;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::io;
use std::sync::Arc;

use bytes::Bytes;

use serde::Serialize;
use serde::Serializer;
use serde::de::DeserializeOwned;
/// A stored message: a type-erased payload paired with its cached
/// serialization.
///
/// The payload is held as `Arc<dyn Any + Send + Sync>` — the caller's own
/// `Arc<T>` allocation, unsized in place — so the tree and the gossip
/// sessions handle messages without being generic over the payload type:
/// they compile once, and only the thin typed facades at the crate's API
/// boundary name `T`. Construction goes through the typed constructors
/// ([`new`](Self::new), [`from_slice`](Self::from_slice), ...); reads at
/// the typed boundary go through the checked downcasts
/// ([`message`](Self::message), [`arc`](Self::arc)).
///
/// The cache avoids repeated roundtrips through serialization: a `Message`
/// always carries the exact CBOR bytes its payload was encoded to or
/// decoded from, and every identity-blind consumer — the wire encoders,
/// size accounting — reads the cached bytes, never the payload. Cloning is
/// cheap: both fields are shared handles.
///
/// The payload encoding is CBOR (via [`ciborium`]): self-describing, so
/// field and variant *names* are the wire contract — a decoder pairs fields
/// by name, tolerating reordering — and no canonical encoding is required
/// of the payload type, because payload bytes carry no identity (a leaf's
/// identity is its version).
///
/// # Panics
///
/// Every payload value must serialize: methods that serialize
/// ([`new`](Self::new), [`from_arc`](Self::from_arc)) panic if the
/// payload's [`serde::Serialize`] implementation reports an error.
/// Encoding runs into an in-memory buffer and CBOR imposes no
/// format-driven failures (any map key, any nesting), so the only trigger
/// is the implementation itself declining a value — which this crate
/// treats as a bug in the payload type, exactly as `Ord`'s totality is
/// trusted. Types whose `Serialize` is data-dependently fallible (for
/// example `std::path::PathBuf`, which errors on non-UTF-8 paths) violate
/// that obligation and must not be used as message types.
///
/// The typed reads panic on a payload type mismatch; see
/// [`message`](Self::message).
#[derive(Clone)]
pub struct Message {
    message: Arc<dyn Any + Send + Sync>,
    serialized: Bytes,
}

/// Deserializes one exact CBOR payload encoding into a type-erased payload
/// value; see [`Message::deserializer`].
pub(crate) type PayloadDeserializer = fn(&[u8]) -> io::Result<Arc<dyn Any + Send + Sync>>;

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

impl Message {
    /// Creates a `Message` pairing the given object with its cached
    /// serialization.
    ///
    /// # Panics
    ///
    /// If the message cannot be serialized (see [`Message`]).
    pub fn new<T>(message: T) -> Self
    where
        T: Serialize + Send + Sync + 'static,
    {
        Message {
            serialized: Bytes::from(to_vec(&message)),
            message: Arc::new(message),
        }
    }

    /// Creates a `Message` pairing the given serialized bytes with the
    /// object derived by deserializing them as a `T`.
    ///
    /// The bytes must be exactly one CBOR value: trailing bytes are
    /// rejected as invalid data, so the cache is always the value's exact
    /// encoding.
    pub fn from_slice<T>(bytes: &[u8]) -> io::Result<Self>
    where
        T: DeserializeOwned + Send + Sync + 'static,
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

    /// Decodes wire payload bytes into a `Message` through a
    /// [`PayloadDeserializer`]: the one deserialization every gossip
    /// ingress performs, with the deserializer carrying the payload type
    /// the peer was constructed with.
    ///
    /// The deserializer validates the bytes are exactly one CBOR value of
    /// its type ([`from_slice`](Self::from_slice)'s contract), so the
    /// cache is always the payload's exact encoding and a malformed
    /// payload fails here, at the wire boundary.
    pub(crate) fn from_wire(bytes: Bytes, deserializer: PayloadDeserializer) -> io::Result<Self> {
        Ok(Message {
            message: deserializer(&bytes)?,
            serialized: bytes,
        })
    }

    /// The deserializer for payloads of type `T`: what a
    /// [`Peer`](crate::Peer) mints at construction and threads to every
    /// session's wire ingress ([`from_wire`](Self::from_wire)).
    ///
    /// A plain function pointer, so everything that carries it stays
    /// non-generic: the payload type's only residue in a running session.
    pub(crate) fn deserializer<T>() -> PayloadDeserializer
    where
        T: DeserializeOwned + Send + Sync + 'static,
    {
        fn deserialize<T: DeserializeOwned + Send + Sync + 'static>(
            bytes: &[u8],
        ) -> io::Result<Arc<dyn Any + Send + Sync>> {
            let mut input = bytes;
            let message: T = ciborium::de::from_reader(&mut input).map_err(de_error)?;
            if !input.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{} trailing bytes after the message payload", input.len()),
                ));
            }
            Ok(Arc::new(message))
        }
        deserialize::<T>
    }

    /// Creates a `Message` from already-shared serialized bytes, without
    /// copying.
    ///
    /// The bytes are deserialized as a `T` to produce the paired object,
    /// under [`from_slice`](Self::from_slice)'s exactly-one-value contract.
    pub fn from_bytes<T>(bytes: Bytes) -> io::Result<Self>
    where
        T: DeserializeOwned + Send + Sync + 'static,
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

    /// Creates a `Message` from an existing [`Arc`], without copying: the
    /// same allocation, unsized in place.
    ///
    /// # Panics
    ///
    /// If the message cannot be serialized (see [`Message`]).
    pub fn from_arc<T>(arc: Arc<T>) -> Self
    where
        T: Serialize + Send + Sync + 'static,
    {
        Message {
            serialized: Bytes::from(to_vec(&*arc)),
            message: arc,
        }
    }

    /// Reads one `Message` off a byte stream, consuming exactly its bytes.
    ///
    /// The shape is one CBOR byte string wrapping the payload's own CBOR
    /// encoding (the same shape [`Serialize`] writes), decoded through
    /// the peer's payload deserializer. Trailing data after the byte
    /// string survives for the next field: the property the wire codec's
    /// mid-stream decodes rest on. Gated to the alternating protocol's
    /// codec, its only production consumer.
    #[cfg(any(test, feature = "protocol-v1"))]
    pub(crate) fn from_reader<R>(reader: R, deserializer: PayloadDeserializer) -> io::Result<Self>
    where
        R: io::Read,
    {
        let bytes: Vec<u8> = ciborium::de::from_reader(reader).map_err(de_error)?;
        Self::from_wire(Bytes::from(bytes), deserializer)
    }

    /// Borrows the payload as its concrete type.
    ///
    /// # Panics
    ///
    /// If the payload is not a `T`. A mismatch is always a crate bug,
    /// never an input: every message reachable from a typed facade was
    /// constructed with that facade's payload type — local sends through
    /// the same `Peer`'s type, wire ingress through its typed decode —
    /// so no gossip input can place a differently-typed payload here.
    pub fn message<T: 'static>(&self) -> &T {
        self.message
            .downcast_ref::<T>()
            .expect("a message's payload type matches its tree's")
    }

    /// Clones out an owned handle to the payload: a reference bump on the
    /// same shared allocation.
    ///
    /// # Panics
    ///
    /// If the payload is not a `T` (see [`message`](Self::message)).
    pub fn arc<T: Send + Sync + 'static>(&self) -> Arc<T> {
        self.message
            .clone()
            .downcast::<T>()
            .unwrap_or_else(|_| panic!("a message's payload type matches its tree's"))
    }

    /// Returns the serialized bytes corresponding to this message.
    pub fn as_slice(&self) -> &[u8] {
        self.serialized.as_ref()
    }

    /// Returns a cheaply-clonable handle to the shared serialized bytes.
    pub fn bytes(&self) -> &Bytes {
        &self.serialized
    }
}

/// Shows the cached serialization, not the payload: the payload's type is
/// erased here, so its own `Debug` is out of reach.
impl fmt::Debug for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Message")
            .field("serialized", &hex::encode(&self.serialized))
            .finish_non_exhaustive()
    }
}

// Equality, and the `Hash` that must agree with it, compare the cached
// serialization: with the payload's type erased, its bytes are the whole
// observable content. Two messages built from the same value by the same
// constructor always carry equal bytes.

impl PartialEq for Message {
    fn eq(&self, other: &Self) -> bool {
        self.serialized == other.serialized
    }
}

impl Eq for Message {}

impl Hash for Message {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.serialized.hash(state);
    }
}

/// One CBOR byte string wrapping the cached CBOR payload, so a message
/// nests inside larger CBOR values without re-encoding.
///
/// The wrapper is what makes a nested message self-delimiting wherever
/// the container does not delimit it; [`from_reader`](Message::from_reader)
/// is the typed decoder of the same shape.
impl Serialize for Message {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(&self.serialized)
    }
}

#[cfg(test)]
mod tests;
