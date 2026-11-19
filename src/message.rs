use std::any::Any;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::io;
use std::sync::Arc;

use bytes::Bytes;

use serde::{Serialize, Serializer, de::DeserializeOwned};

use crate::default;

/// A type-erased payload and its cached CBOR encoding.
///
/// Clones share the payload and cache. Typed API boundaries recover the payload
/// through [`arc`](Self::arc); tree traversal and wire encoding use the cached
/// bytes without knowing the payload type. Received encodings are preserved
/// exactly rather than serialized again.
///
/// Payload bytes do not determine a message's identity; its version does. CBOR
/// field and variant names are the payload's wire contract, and callers need
/// not provide a canonical encoding.
///
/// # Panics
///
/// A typed read with the wrong type panics (see [`arc`](Self::arc)).
#[derive(Clone)]
pub struct Message {
    /// The caller's payload allocation, unsized without copying.
    message: Arc<dyn Any + Send + Sync>,
    /// The exact bytes used for gossip and size accounting.
    serialized: Bytes,
}

/// A peer's message nesting-depth limit, counted in the CBOR decode engine's
/// recursion steps.
///
/// Selected by [`PeerBuilder::message_depth_limit`](crate::PeerBuilder::message_depth_limit)
/// (whose docs carry the full contract); defaults to
/// [`default::MESSAGE_DEPTH_LIMIT`]. A step is the engine's own accounting
/// (arrays, maps, tags, and type-driven wrappers such as an enum's variant
/// scope), not a structural property of the bytes — which is why admission at
/// send runs the decode itself rather than counting anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct MessageDepthLimit(u64);

/// Construct and inspect the decoder's recursion limit.
impl MessageDepthLimit {
    /// A limit of exactly `steps` decode recursion steps: a payload value whose
    /// decode recurses deeper is rejected.
    pub(crate) const fn new(steps: u64) -> Self {
        MessageDepthLimit(steps)
    }

    /// The limit, in decode recursion steps.
    pub(crate) const fn get(self) -> u64 {
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

/// Use the crate's default message depth limit.
impl Default for MessageDepthLimit {
    /// Return the default recursion limit.
    fn default() -> Self {
        MessageDepthLimit(default::MESSAGE_DEPTH_LIMIT)
    }
}

/// Why [`Peer::send`](crate::Peer::send) rejected a message, before any
/// replica stored or gossiped it.
///
/// Before accepting a message, `send` checks that every receiver would decode
/// exactly the message sent. It serializes the message, deserializes it back
/// again, and compares the decoded value with the original. The batch
/// operations ([`Batch::send`](crate::Batch::send) and the `send_all` methods)
/// run the same checks on each message.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum EncodeError {
    /// The message type's [`Serialize`] implementation returned an error for
    /// this value.
    ///
    /// The source carries the implementation's own error. A type whose
    /// `Serialize` fails on some values, such as
    /// [`PathBuf`](std::path::PathBuf) on a path that is not UTF-8, returns
    /// this for those values.
    #[error("message failed to serialize")]
    Serialize(#[source] io::Error),
    /// The message's encoding nests deeper than the [`Peer`](crate::Peer)'s
    /// configured [`message_depth_limit`](crate::PeerBuilder::message_depth_limit).
    ///
    /// Every peer in a gossip network decodes under the same limit, so every
    /// receiver would reject this message.
    #[error("message nests deeper than the configured message depth limit ({limit} steps)")]
    TooDeep {
        /// The configured
        /// [`message_depth_limit`](crate::PeerBuilder::message_depth_limit) which the
        /// message would have exceeded.
        limit: u64,
    },
    /// The message type's [`Deserialize`](serde::Deserialize) implementation
    /// rejected the output of its own [`Serialize`] implementation.
    ///
    /// Every receiver decodes with the same implementation, so every receiver
    /// would reject this message.
    #[error("message's encoding does not decode as its own type")]
    Deserialize(#[source] io::Error),
    /// The message's encoding decodes to a *different value*, as the message
    /// type's [`Eq`] judges it.
    ///
    /// For example, CBOR encodes `Some(None)` of type `Option<Option<T>>`
    /// exactly as it encodes `None`, so that value decodes back as `None`. If
    /// `send` accepted such a value, this replica would hold a different value
    /// than every replica that received it.
    #[error("message's encoding decodes to a different value")]
    Roundtrip,
}

/// Why a payload decode failed: the crate-internal split between the depth case
/// and everything else.
///
/// The depth case stays typed end to end so send-side admission can surface it
/// as [`EncodeError::TooDeep`] without string matching; wire ingress folds both
/// cases back into the `io::Error` its surface speaks ([`Message::from_wire`]).
/// The decode-side counterpart of [`EncodeError`].
#[derive(Debug)]
pub(crate) enum PayloadDecodeError {
    /// The payload's decode recursed past the given limit (the decode engine's
    /// recursion-limit error, preserved as a variant).
    Depth(MessageDepthLimit),
    /// Truncation (the reader's own error, passed through) or invalid data
    /// (corruption, a type mismatch, trailing bytes).
    Io(io::Error),
}

/// Convert decode failures for callers that use I/O errors.
impl PayloadDecodeError {
    /// Fold into `io::Error`, the wire-ingress surface: the depth case becomes
    /// invalid data naming the exceeded limit.
    fn into_io(self) -> io::Error {
        match self {
            PayloadDecodeError::Depth(limit) => {
                let limit = limit.get();
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("message nests deeper than the message depth limit ({limit} steps)"),
                )
            }
            PayloadDecodeError::Io(error) => error,
        }
    }
}

/// Serializes one type-erased payload value into an admission-checked
/// [`Message`]; the serializing half of a [`PayloadCodec`].
pub(crate) type PayloadSerializer =
    fn(Arc<dyn Any + Send + Sync>, MessageDepthLimit) -> Result<Message, EncodeError>;

/// Deserializes one exact CBOR payload encoding into a type-erased payload
/// value, bounding the decode's recursion at the given depth limit; the
/// deserializing half of a [`PayloadCodec`].
pub(crate) type PayloadDeserializer =
    fn(&[u8], MessageDepthLimit) -> Result<Arc<dyn Any + Send + Sync>, PayloadDecodeError>;

/// A peer's payload codec: the typed payload boundary created once at
/// [`Peer`](crate::Peer) construction and carried by every session.
///
/// The payload type's serde obligations concentrate at construction; the fn
/// pointer inside is the type's only residue afterwards, so everything that
/// carries a codec stays non-generic. The configured [`MessageDepthLimit`]
/// rides beside the pointer as data (a plain fn pointer cannot capture it),
/// which is what makes the limit unmissable: every ingress parse in the peer's
/// orbit goes through this one value.
#[derive(Clone, Copy)]
pub(crate) struct PayloadCodec {
    /// Serialize and check admission for the peer's payload type.
    serialize: PayloadSerializer,
    /// Decode the peer's payload type into shared, type-erased storage.
    deserialize: PayloadDeserializer,
    /// The recursion limit used for admission and wire ingress.
    limit: MessageDepthLimit,
}

/// Apply one payload type and depth limit at every typed boundary.
impl PayloadCodec {
    /// Construct the codec for payloads of type `T` at the given depth limit.
    ///
    /// All of `T`'s payload obligations land here, at construction: a peer
    /// demands `Serialize` at construction even if it never sends, symmetric
    /// with demanding `DeserializeOwned` even if it never receives (forwarding
    /// needs neither bound, since gossip re-supplies cached bytes), and `Eq` so
    /// send-side admission can hold every encoding to decode back equal to the
    /// value sent.
    pub(crate) fn new<T>(limit: MessageDepthLimit) -> Self
    where
        T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
    {
        /// Recover the payload type and check its encoding before storage.
        fn serialize_payload<T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static>(
            payload: Arc<dyn Any + Send + Sync>,
            limit: MessageDepthLimit,
        ) -> Result<Message, EncodeError> {
            let payload: Arc<T> = payload.downcast().unwrap_or_else(|_| {
                panic!("a codec serializes exactly the payload type it was built for")
            });
            Message::try_from_arc(payload, limit)
        }

        /// Decode one value into shared, type-erased storage.
        fn deserialize_payload<T: DeserializeOwned + Send + Sync + 'static>(
            bytes: &[u8],
            limit: MessageDepthLimit,
        ) -> Result<Arc<dyn Any + Send + Sync>, PayloadDecodeError> {
            Ok(Arc::new(decode_exact::<T>(bytes, limit)?))
        }

        PayloadCodec {
            serialize: serialize_payload::<T>,
            deserialize: deserialize_payload::<T>,
            limit,
        }
    }

    /// Encode a payload and verify that decoding it at the configured limit
    /// recovers the same value, retaining the caller's allocation on success.
    ///
    /// # Panics
    ///
    /// If the value is not the payload type the codec was built for: a crate
    /// bug, never an input — every caller hands in the `T` its own typed
    /// signature names.
    pub(crate) fn message(
        &self,
        payload: Arc<dyn Any + Send + Sync>,
    ) -> Result<Message, EncodeError> {
        (self.serialize)(payload, self.limit)
    }

    /// The configured message depth limit this codec enforces: what the
    /// greeting declares and the handshake holds to equality.
    pub(crate) fn limit(&self) -> MessageDepthLimit {
        self.limit
    }

    /// Decode one exact CBOR payload encoding into a type-erased payload value,
    /// bounded at the carried depth limit.
    pub(crate) fn decode(
        &self,
        bytes: &[u8],
    ) -> Result<Arc<dyn Any + Send + Sync>, PayloadDecodeError> {
        (self.deserialize)(bytes, self.limit)
    }
}

/// Map a ciborium deserialization failure into `io::Error`, keeping the
/// truncation/corruption split callers classify by: a reader's own error passes
/// through, everything else is invalid data.
fn de_error(error: ciborium::de::Error<io::Error>) -> io::Error {
    match error {
        ciborium::de::Error::Io(error) => error,
        error => io::Error::new(io::ErrorKind::InvalidData, error.to_string()),
    }
}

/// Map a ciborium serialization failure into `io::Error`, the dual of
/// [`de_error`]: a writer's own error passes through, and a value the payload's
/// `Serialize` refused becomes invalid input carrying its message.
///
/// We map the value case ourselves because ciborium's `Display` for this error
/// is its `Debug` rendering.
fn ser_error(error: ciborium::ser::Error<io::Error>) -> io::Error {
    match error {
        ciborium::ser::Error::Io(error) => error,
        ciborium::ser::Error::Value(message) => {
            io::Error::new(io::ErrorKind::InvalidInput, message)
        }
    }
}

/// Encode one CBOR value, discarding spare capacity before caching it.
///
/// Fails if the value's `Serialize` implementation fails. The writer is a
/// `Vec`, whose writes cannot fail, so every error is the value's own.
fn encode<T: Serialize>(value: &T) -> io::Result<Bytes> {
    let mut buf = Vec::new();
    ciborium::ser::into_writer(value, &mut buf).map_err(ser_error)?;
    Ok(if buf.len() == buf.capacity() {
        Bytes::from(buf)
    } else {
        // A shrink can leave the original allocation in place. Copy into a
        // fresh allocation so the long-lived cache sheds the growth buffer's
        // spare capacity; the allocator may still round up the requested size.
        Bytes::copy_from_slice(&buf)
    })
}

/// Decode exactly one CBOR value of type `T` from `bytes`, bounding the
/// decode's recursion at `limit`: the one payload parse behind every typed
/// constructor and the codec's deserializer.
///
/// Trailing bytes are rejected as invalid data, so a cache built from the input
/// is always the value's exact encoding. A value whose decode recurses past the
/// limit is the typed depth case, kept distinct so send-side admission can
/// classify it without string matching.
fn decode_exact<T: DeserializeOwned>(
    bytes: &[u8],
    limit: MessageDepthLimit,
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
        limit: MessageDepthLimit,
    ) -> Result<Self, EncodeError>
    where
        T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
    {
        let serialized = encode(&*arc).map_err(EncodeError::Serialize)?;
        // Run the same typed decode that wire ingress wraps in an `Arc`, so
        // admission cannot accept a value that a receiver would reject.
        let decoded = match decode_exact::<T>(&serialized, limit) {
            Ok(decoded) => decoded,
            Err(PayloadDecodeError::Depth(limit)) => {
                return Err(EncodeError::TooDeep { limit: limit.get() });
            }
            Err(PayloadDecodeError::Io(source)) => return Err(EncodeError::Deserialize(source)),
        };
        // Decoding successfully is not enough: the value must survive intact.
        if decoded != *arc {
            return Err(EncodeError::Roundtrip);
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

/// One CBOR byte string wrapping the cached CBOR payload, so a message nests
/// inside larger CBOR values without re-encoding.
///
/// The wrapper is what makes a nested message self-delimiting wherever the
/// container does not delimit it.
impl Serialize for Message {
    /// Pass the cached encoding to the serializer as a byte string.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(&self.serialized)
    }
}

#[cfg(test)]
mod tests;
