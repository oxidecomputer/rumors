use std::any::Any;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::io;
use std::sync::Arc;

use bytes::Bytes;

use serde::Serialize;
use serde::Serializer;
use serde::de::DeserializeOwned;

/// A type-erased payload and its cached CBOR encoding.
///
/// Clones share the payload and cache. Typed API boundaries recover the payload
/// through [`arc`](Self::arc); tree traversal and wire encoding use the cached
/// bytes without knowing the payload type. Received encodings are preserved
/// exactly rather than serialized again.
///
/// Payload bytes do not determine a message's identity; its version does.
/// CBOR field and variant names are the payload's wire contract, and callers
/// need not provide a canonical encoding.
///
/// # Panics
///
/// Serializing constructors panic if the payload's [`Serialize`] implementation
/// fails. Every payload value must serialize, as required by the crate's payload
/// contract. A typed read with the wrong type also panics (see [`arc`](Self::arc)).
#[derive(Clone)]
pub struct Message {
    /// The caller's payload allocation, unsized without copying.
    message: Arc<dyn Any + Send + Sync>,
    /// The exact bytes used for gossip and size accounting.
    serialized: Bytes,
}

/// The default payload nesting-depth limit: 256 decode recursion steps.
///
/// Exactly the CBOR decoder's own default recursion bound, so a fleet
/// upgrading together sees no acceptance change on existing content.
/// Wire interop across releases is governed by the greeting's format,
/// not by this constant.
pub const DEFAULT_PAYLOAD_DEPTH_LIMIT: PayloadDepthLimit = PayloadDepthLimit(256);

/// A peer's payload nesting-depth limit, counted in the CBOR decode
/// engine's recursion steps.
///
/// Selected by [`Peer::payload_depth_limit`](crate::Peer::payload_depth_limit)
/// (whose docs carry the full contract); defaults to
/// [`DEFAULT_PAYLOAD_DEPTH_LIMIT`]. A step is the engine's own
/// accounting (arrays, maps, tags, and type-driven wrappers such as an
/// enum's variant scope), not a structural property of the bytes —
/// which is why admission at send runs the decode itself rather than
/// counting anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PayloadDepthLimit(u64);

/// Construct and inspect the decoder's recursion limit.
impl PayloadDepthLimit {
    /// A limit of exactly `steps` decode recursion steps: a payload
    /// value whose decode recurses deeper is rejected.
    pub const fn new(steps: u64) -> Self {
        PayloadDepthLimit(steps)
    }

    /// The limit, in decode recursion steps.
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

/// Use the crate's default payload depth limit.
impl Default for PayloadDepthLimit {
    /// Return the default recursion limit.
    fn default() -> Self {
        DEFAULT_PAYLOAD_DEPTH_LIMIT
    }
}

/// Format the limit with its unit.
impl fmt::Display for PayloadDepthLimit {
    /// Display the number of decode steps.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} steps", self.0)
    }
}

/// A payload rejected before it is stored or sent.
///
/// See [`Rumors::send`](crate::Rumors::send) for the payload requirements.
#[derive(Debug, thiserror::Error)]
pub enum EncodeError {
    /// The payload value's CBOR encoding nests deeper than the peer's
    /// configured [`PayloadDepthLimit`].
    #[error("message payload nests deeper than the configured payload depth limit ({limit})")]
    Depth {
        /// The configured limit the payload's decode exceeded.
        limit: PayloadDepthLimit,
    },
    /// The payload type's [`serde::Deserialize`] implementation rejected
    /// the bytes its own [`serde::Serialize`] implementation produced:
    /// admitted, such a value would fail at every receiver instead.
    #[error("message payload does not survive its own serde round-trip: {0}")]
    Roundtrip(#[source] io::Error),
    /// The payload value's encoding decodes to a different value (by
    /// the payload type's own `Eq`): the serde pairing is lossy for
    /// this value.
    ///
    /// The canonical example is a nested `Option` holding `Some(None)`,
    /// which decodes as `None`. The check is send-side only: ingress
    /// holds no original to compare against.
    #[error("message payload's encoding decodes to a different value")]
    Unfaithful,
}

/// Why a payload decode failed: the crate-internal split between the
/// depth case and everything else.
///
/// The depth case stays typed end to end so send-side admission can
/// surface it as [`EncodeError::Depth`] without string matching; wire
/// ingress folds both cases back into the `io::Error` its surface
/// speaks ([`Message::from_wire`]). The decode-side counterpart of
/// [`EncodeError`].
#[derive(Debug)]
pub(crate) enum PayloadDecodeError {
    /// The payload's decode recursed past the given limit (the decode
    /// engine's recursion-limit error, preserved as a variant).
    Depth(PayloadDepthLimit),
    /// Truncation (the reader's own error, passed through) or invalid
    /// data (corruption, a type mismatch, trailing bytes).
    Io(io::Error),
}

/// Convert decode failures for callers that use I/O errors.
impl PayloadDecodeError {
    /// Fold into `io::Error`, the wire-ingress surface: the depth case
    /// becomes invalid data naming the exceeded limit.
    fn into_io(self) -> io::Error {
        match self {
            PayloadDecodeError::Depth(limit) => io::Error::new(
                io::ErrorKind::InvalidData,
                format!("message payload nests deeper than the payload depth limit ({limit})"),
            ),
            PayloadDecodeError::Io(error) => error,
        }
    }
}

/// Serializes one type-erased payload value into an admission-checked
/// [`Message`]; the serializing half of a [`PayloadCodec`].
pub(crate) type PayloadSerializer =
    fn(Arc<dyn Any + Send + Sync>, PayloadDepthLimit) -> Result<Message, EncodeError>;

/// Deserializes one exact CBOR payload encoding into a type-erased payload
/// value, bounding the decode's recursion at the given depth limit; the
/// deserializing half of a [`PayloadCodec`].
pub(crate) type PayloadDeserializer =
    fn(&[u8], PayloadDepthLimit) -> Result<Arc<dyn Any + Send + Sync>, PayloadDecodeError>;

/// A peer's payload codec: the typed payload boundary created once at
/// [`Peer`](crate::Peer) construction and carried by every session.
///
/// The payload type's serde obligations concentrate at construction; the fn
/// pointer inside is the type's only residue afterwards, so everything
/// that carries a codec stays non-generic. The configured
/// [`PayloadDepthLimit`] rides beside the pointer as data (a plain fn
/// pointer cannot capture it), which is what makes the limit unmissable:
/// every ingress parse in the peer's orbit goes through this one value.
#[derive(Clone, Copy)]
pub(crate) struct PayloadCodec {
    /// Serialize and check admission for the peer's payload type.
    serialize: PayloadSerializer,
    /// Decode the peer's payload type into shared, type-erased storage.
    deserialize: PayloadDeserializer,
    /// The recursion limit used for admission and wire ingress.
    limit: PayloadDepthLimit,
}

/// Apply one payload type and depth limit at every typed boundary.
impl PayloadCodec {
    /// Construct the codec for payloads of type `T` at the given depth limit.
    ///
    /// All of `T`'s payload obligations land here, at construction: a peer
    /// demands `Serialize` at construction even if it never sends,
    /// symmetric with demanding `DeserializeOwned` even if it never
    /// receives (forwarding needs neither bound, since gossip re-supplies
    /// cached bytes), and `Eq` so send-side admission can hold every
    /// encoding to decode back equal to the value sent.
    pub(crate) fn new<T>(limit: PayloadDepthLimit) -> Self
    where
        T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
    {
        /// Recover the payload type and check its encoding before storage.
        fn serialize_payload<T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static>(
            payload: Arc<dyn Any + Send + Sync>,
            limit: PayloadDepthLimit,
        ) -> Result<Message, EncodeError> {
            let payload: Arc<T> = payload.downcast().unwrap_or_else(|_| {
                panic!("a codec serializes exactly the payload type it was built for")
            });
            Message::try_from_arc(payload, limit)
        }
        PayloadCodec {
            serialize: serialize_payload::<T>,
            deserialize: Message::deserializer::<T>(),
            limit,
        }
    }

    /// Encode a payload and verify that decoding it at the configured limit
    /// recovers the same value, retaining the caller's allocation on success.
    ///
    /// # Panics
    ///
    /// If the value is not the payload type the codec was built for: a crate bug,
    /// never an input — every caller hands in the `T` its own typed
    /// signature names. A `Serialize` failure keeps [`Message`]'s
    /// documented panic contract.
    pub(crate) fn message(
        &self,
        payload: Arc<dyn Any + Send + Sync>,
    ) -> Result<Message, EncodeError> {
        (self.serialize)(payload, self.limit)
    }

    /// The configured payload depth limit this codec enforces: what the
    /// greeting declares and the handshake holds to equality.
    pub(crate) fn limit(&self) -> PayloadDepthLimit {
        self.limit
    }

    /// Replace the carried depth limit, keeping the codec's fn pointers.
    #[must_use]
    pub(crate) fn with_limit(self, limit: PayloadDepthLimit) -> Self {
        PayloadCodec { limit, ..self }
    }

    /// Decode one exact CBOR payload encoding into a type-erased payload
    /// value, bounded at the carried depth limit.
    pub(crate) fn decode(
        &self,
        bytes: &[u8],
    ) -> Result<Arc<dyn Any + Send + Sync>, PayloadDecodeError> {
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

/// Encode one CBOR value, discarding spare capacity before caching it.
///
/// Panics if the payload cannot serialize, as required by [`Message`].
fn encode<T: Serialize>(value: &T) -> Bytes {
    let mut buf = Vec::new();
    ciborium::ser::into_writer(value, &mut buf)
        .expect("every message value must serialize (see Message's panic contract)");
    if buf.len() == buf.capacity() {
        Bytes::from(buf)
    } else {
        // A shrink can leave the original allocation in place. Copy into a
        // fresh allocation so the long-lived cache sheds the growth buffer's
        // spare capacity; the allocator may still round up the requested size.
        Bytes::copy_from_slice(&buf)
    }
}

/// Decode exactly one CBOR value of type `T` from `bytes`, bounding the
/// decode's recursion at `limit`: the one payload parse behind every
/// typed constructor and the codec's deserializer.
///
/// Trailing bytes are rejected as invalid data, so a cache built from the
/// input is always the value's exact encoding. A value whose decode
/// recurses past the limit is the typed depth case, kept distinct so
/// send-side admission can classify it without string matching.
fn decode_exact<T: DeserializeOwned>(
    bytes: &[u8],
    limit: PayloadDepthLimit,
) -> Result<T, PayloadDecodeError> {
    let mut input = bytes;
    let message: T =
        ciborium::de::from_reader_with_recursion_limit(&mut input, limit.recursion_limit())
            .map_err(|error| match error {
                ciborium::de::Error::RecursionLimitExceeded => PayloadDecodeError::Depth(limit),
                error => PayloadDecodeError::Io(de_error(error)),
            })?;
    if !input.is_empty() {
        return Err(PayloadDecodeError::Io(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} trailing bytes after the message payload", input.len()),
        )));
    }
    Ok(message)
}

/// Construct cached messages and recover their typed payloads.
impl Message {
    /// Check admission and cache the encoding, sharing the payload's [`Arc`].
    pub(crate) fn try_from_arc<T>(
        arc: Arc<T>,
        limit: PayloadDepthLimit,
    ) -> Result<Self, EncodeError>
    where
        T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
    {
        let serialized = encode(&*arc);
        // Use the receiver's decoder and limit so admission cannot accept a
        // value that wire ingress would reject.
        let decoded = match Self::deserializer::<T>()(&serialized, limit) {
            Ok(decoded) => decoded,
            Err(PayloadDecodeError::Depth(limit)) => return Err(EncodeError::Depth { limit }),
            Err(PayloadDecodeError::Io(source)) => return Err(EncodeError::Roundtrip(source)),
        };
        // Decoding successfully is not enough: the value must survive intact.
        let decoded: Arc<T> = decoded
            .downcast()
            .unwrap_or_else(|_| panic!("a payload decodes to its own type"));
        if *decoded != *arc {
            return Err(EncodeError::Unfaithful);
        }
        Ok(Message {
            serialized,
            message: arc,
        })
    }

    /// Decode a wire payload using the peer's type and depth limit.
    ///
    /// The codec rejects malformed, trailing, or excessively nested data.
    /// Successful decoding retains the received encoding without copying it.
    pub(crate) fn from_wire(bytes: Bytes, codec: PayloadCodec) -> io::Result<Self> {
        Ok(Message {
            message: codec.decode(&bytes).map_err(PayloadDecodeError::into_io)?,
            serialized: bytes,
        })
    }

    /// The shared decoder for send-side admission and wire ingress.
    ///
    /// Returning a function pointer keeps [`PayloadCodec`] non-generic; the
    /// payload type and its serde implementation stay inside this function.
    pub(crate) fn deserializer<T>() -> PayloadDeserializer
    where
        T: DeserializeOwned + Send + Sync + 'static,
    {
        /// Decode one value into shared, type-erased storage.
        fn deserialize<T: DeserializeOwned + Send + Sync + 'static>(
            bytes: &[u8],
            limit: PayloadDepthLimit,
        ) -> Result<Arc<dyn Any + Send + Sync>, PayloadDecodeError> {
            let message: T = decode_exact(bytes, limit)?;
            Ok(Arc::new(message))
        }
        deserialize::<T>
    }

    /// Share the stored payload as its original type.
    ///
    /// # Panics
    ///
    /// If the payload is not a `T`. Typed facades must request the same type
    /// that their constructors and wire decoders stored.
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
}

/// Shows the cached serialization, not the payload: the payload's type is
/// erased here, so its own `Debug` is out of reach.
impl fmt::Debug for Message {
    /// Display the cached bytes as hexadecimal.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Message")
            .field("serialized", &hex::encode(&self.serialized))
            .finish_non_exhaustive()
    }
}

/// Compare the cached encodings without recovering the erased payload types.
impl PartialEq for Message {
    /// Test byte-for-byte equality of the cached encodings.
    fn eq(&self, other: &Self) -> bool {
        self.serialized == other.serialized
    }
}

/// Cached byte equality is an equivalence relation.
impl Eq for Message {}

/// Hash the same cached bytes that determine equality.
impl Hash for Message {
    /// Feed the cached encoding into the caller's hasher.
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.serialized.hash(state);
    }
}

/// One CBOR byte string wrapping the cached CBOR payload, so a message
/// nests inside larger CBOR values without re-encoding.
///
/// The wrapper is what makes a nested message self-delimiting wherever
/// the container does not delimit it.
impl Serialize for Message {
    /// Pass the cached encoding to the serializer as a byte string.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(&self.serialized)
    }
}

#[cfg(test)]
mod tests;
