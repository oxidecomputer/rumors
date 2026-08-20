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
/// the typed boundary go through the checked downcast
/// ([`arc`](Self::arc)).
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
/// The typed read panics on a payload type mismatch; see
/// [`arc`](Self::arc).
#[derive(Clone)]
pub struct Message {
    message: Arc<dyn Any + Send + Sync>,
    serialized: Bytes,
}

/// The default payload nesting-depth limit: 256 scopes.
///
/// This is exactly the decode bound the CBOR decoder ([`ciborium`]'s
/// `from_reader`) applies by default, so a fleet upgrading together sees
/// no acceptance change on existing content; the only new rejections land
/// on the author of an over-deep value at send time. Wire interop across
/// releases is governed by the greeting's format, not by this constant.
pub const DEFAULT_PAYLOAD_DEPTH_LIMIT: PayloadDepthLimit = PayloadDepthLimit(256);

/// A peer's payload nesting-depth limit, counted in CBOR scopes
/// (containers and tags).
///
/// Selected by [`Peer::payload_depth_limit`](crate::Peer::payload_depth_limit)
/// (whose docs carry the full contract) and defaulting to
/// [`DEFAULT_PAYLOAD_DEPTH_LIMIT`]. The scope accounting is the CBOR
/// decoder's own recursion accounting: arrays, maps, and tags each open
/// a scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PayloadDepthLimit(u64);

impl PayloadDepthLimit {
    /// A limit of exactly `scopes` nesting scopes: a payload value whose
    /// encoding nests deeper is rejected.
    pub const fn new(scopes: u64) -> Self {
        PayloadDepthLimit(scopes)
    }

    /// The limit, in scopes.
    pub const fn get(self) -> u64 {
        self.0
    }

    /// The limit as the decoder's `usize` recursion bound.
    ///
    /// Saturating: a limit past `usize::MAX` admits every input that can
    /// physically exist, because a value's nesting depth never exceeds its
    /// encoding's byte length, which a slice caps well below `usize::MAX`.
    pub(crate) fn recursion_limit(self) -> usize {
        usize::try_from(self.0).unwrap_or(usize::MAX)
    }
}

impl Default for PayloadDepthLimit {
    fn default() -> Self {
        DEFAULT_PAYLOAD_DEPTH_LIMIT
    }
}

impl fmt::Display for PayloadDepthLimit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} scopes", self.0)
    }
}

/// Deserializes one exact CBOR payload encoding into a type-erased payload
/// value, bounding its nesting at the given depth limit; the deserializing
/// half of a [`PayloadCodec`].
pub(crate) type PayloadDeserializer =
    fn(&[u8], PayloadDepthLimit) -> io::Result<Arc<dyn Any + Send + Sync>>;

/// A peer's payload codec: the typed payload boundary minted once at
/// [`Peer`](crate::Peer) construction and carried by every session.
///
/// The payload type's serde obligations concentrate at the mint; the fn
/// pointer inside is the type's only residue afterwards, so everything
/// that carries a codec stays non-generic. The configured
/// [`PayloadDepthLimit`] rides beside the pointer as data (a plain fn
/// pointer cannot capture it), which is what makes the limit unmissable:
/// every ingress parse in the peer's orbit goes through this one value.
#[derive(Clone, Copy)]
pub(crate) struct PayloadCodec {
    deserialize: PayloadDeserializer,
    limit: PayloadDepthLimit,
}

impl PayloadCodec {
    /// Mint the codec for payloads of type `T` at the given depth limit.
    pub(crate) fn mint<T>(limit: PayloadDepthLimit) -> Self
    where
        T: DeserializeOwned + Send + Sync + 'static,
    {
        PayloadCodec {
            deserialize: Message::deserializer::<T>(),
            limit,
        }
    }

    /// Replace the carried depth limit, keeping the minted pointer.
    #[must_use]
    pub(crate) fn with_limit(self, limit: PayloadDepthLimit) -> Self {
        PayloadCodec { limit, ..self }
    }

    /// Decode one exact CBOR payload encoding into a type-erased payload
    /// value, bounded at the carried depth limit.
    pub(crate) fn decode(&self, bytes: &[u8]) -> io::Result<Arc<dyn Any + Send + Sync>> {
        (self.deserialize)(bytes, self.limit)
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

/// Decode exactly one CBOR value of type `T` from `bytes`, bounding its
/// nesting at `limit`: the one payload parse behind every typed
/// constructor and the minted deserializer.
///
/// Trailing bytes are rejected as invalid data, so a cache built from the
/// input is always the value's exact encoding. A value nested past the
/// limit surfaces as invalid data too (the decoder's recursion-limit
/// error), keeping depth violations observable without a panic path.
fn decode_exact<T: DeserializeOwned>(bytes: &[u8], limit: PayloadDepthLimit) -> io::Result<T> {
    let mut input = bytes;
    let message: T =
        ciborium::de::from_reader_with_recursion_limit(&mut input, limit.recursion_limit())
            .map_err(de_error)?;
    if !input.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} trailing bytes after the message payload", input.len()),
        ));
    }
    Ok(message)
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
    /// object derived by deserializing them as a `T`, bounding the
    /// payload's nesting at `limit`.
    ///
    /// The bytes must be exactly one CBOR value: trailing bytes are
    /// rejected as invalid data, so the cache is always the value's exact
    /// encoding. The bytes are the caller's own (there is no peer context
    /// here), so the caller supplies the limit explicitly: an application
    /// rehydrating its stored messages passes the limit its fleet is
    /// configured with, and a value nested past it is rejected as invalid
    /// data (the decoder's recursion-limit error).
    pub fn from_slice<T>(bytes: &[u8], limit: PayloadDepthLimit) -> io::Result<Self>
    where
        T: DeserializeOwned + Send + Sync + 'static,
    {
        let message: T = decode_exact(bytes, limit)?;
        Ok(Message {
            message: Arc::new(message),
            serialized: Bytes::copy_from_slice(bytes),
        })
    }

    /// Decodes wire payload bytes into a `Message` through the peer's
    /// [`PayloadCodec`]: the one deserialization every gossip ingress
    /// performs.
    ///
    /// The codec carries the payload type the peer was constructed with
    /// and the depth limit it was configured with.
    ///
    /// The codec validates the bytes are exactly one CBOR value of its
    /// type, nested within its limit ([`from_slice`](Self::from_slice)'s
    /// contract), so the cache is always the payload's exact encoding and
    /// a malformed or over-deep payload fails here, at the wire boundary.
    pub(crate) fn from_wire(bytes: Bytes, codec: PayloadCodec) -> io::Result<Self> {
        Ok(Message {
            message: codec.decode(&bytes)?,
            serialized: bytes,
        })
    }

    /// The deserializer for payloads of type `T`: the deserializing half
    /// of the [`PayloadCodec`] a [`Peer`](crate::Peer) mints at
    /// construction, applied at every session's wire ingress
    /// ([`from_wire`](Self::from_wire)).
    ///
    /// A plain function pointer, so everything that carries it stays
    /// non-generic: the payload type's only residue in a running session.
    /// The depth limit arrives as an argument because a fn pointer cannot
    /// capture one; the codec pairs the two.
    pub(crate) fn deserializer<T>() -> PayloadDeserializer
    where
        T: DeserializeOwned + Send + Sync + 'static,
    {
        fn deserialize<T: DeserializeOwned + Send + Sync + 'static>(
            bytes: &[u8],
            limit: PayloadDepthLimit,
        ) -> io::Result<Arc<dyn Any + Send + Sync>> {
            let message: T = decode_exact(bytes, limit)?;
            Ok(Arc::new(message))
        }
        deserialize::<T>
    }

    /// Creates a `Message` from already-shared serialized bytes, without
    /// copying, bounding the payload's nesting at `limit`.
    ///
    /// The bytes are deserialized as a `T` to produce the paired object,
    /// under [`from_slice`](Self::from_slice)'s exactly-one-value,
    /// within-limit contract (its docs state why the caller supplies the
    /// limit).
    pub fn from_bytes<T>(bytes: Bytes, limit: PayloadDepthLimit) -> io::Result<Self>
    where
        T: DeserializeOwned + Send + Sync + 'static,
    {
        let message: T = decode_exact(bytes.as_ref(), limit)?;
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
    /// the peer's payload codec. The outer parse is a flat byte string,
    /// so it runs at the decoder's default recursion bound; the codec
    /// bounds the payload inside. Trailing data after the byte string
    /// survives for the next field: the property the wire codec's
    /// mid-stream decodes rest on. Gated to the alternating protocol's
    /// codec, its only production consumer.
    #[cfg(any(test, feature = "protocol-v1"))]
    pub(crate) fn from_reader<R>(reader: R, codec: PayloadCodec) -> io::Result<Self>
    where
        R: io::Read,
    {
        let bytes: Vec<u8> = ciborium::de::from_reader(reader).map_err(de_error)?;
        Self::from_wire(Bytes::from(bytes), codec)
    }

    /// Clones out an owned handle to the payload: a reference bump on the
    /// same shared allocation.
    ///
    /// # Panics
    ///
    /// If the payload is not a `T`. A mismatch is always a crate bug,
    /// never an input: every message reachable from a typed facade was
    /// constructed with that facade's payload type — local sends through
    /// the same `Peer`'s type, wire ingress through its typed decode —
    /// so no gossip input can place a differently-typed payload here.
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
