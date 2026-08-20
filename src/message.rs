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
/// payload's [`serde::Serialize`] implementation reports an error —
/// always a bug in the payload type, since CBOR itself imposes no
/// format-driven failures. The crate docs' "choosing a payload type"
/// section is the contract of record for this obligation.
///
/// The typed read panics on a payload type mismatch; see
/// [`arc`](Self::arc).
#[derive(Clone)]
pub struct Message {
    message: Arc<dyn Any + Send + Sync>,
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

impl Default for PayloadDepthLimit {
    fn default() -> Self {
        DEFAULT_PAYLOAD_DEPTH_LIMIT
    }
}

impl fmt::Display for PayloadDepthLimit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} steps", self.0)
    }
}

/// A message payload failed admission at its author: the send is refused
/// before anything is stored or gossiped.
///
/// Admission runs the exact decode every receiver's wire ingress runs,
/// so a payload this error rejects is one a receiver would have failed
/// to decode — surfaced at the author instead. A [`serde::Serialize`]
/// failure is never this error: it keeps the panic contract documented
/// at [`Rumors::send`](crate::Rumors::send).
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
    serialize: PayloadSerializer,
    deserialize: PayloadDeserializer,
    limit: PayloadDepthLimit,
}

impl PayloadCodec {
    /// Mint the codec for payloads of type `T` at the given depth limit.
    ///
    /// All of `T`'s payload obligations land here, at the mint: a peer
    /// demands `Serialize` at construction even if it never sends,
    /// symmetric with demanding `DeserializeOwned` even if it never
    /// receives (forwarding needs neither bound, since gossip re-supplies
    /// cached bytes), and `Eq` so send-side admission can hold every
    /// encoding to decode back equal to the value sent.
    pub(crate) fn mint<T>(limit: PayloadDepthLimit) -> Self
    where
        T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
    {
        fn serialize_payload<T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static>(
            payload: Arc<dyn Any + Send + Sync>,
            limit: PayloadDepthLimit,
        ) -> Result<Message, EncodeError> {
            let payload: Arc<T> = payload
                .downcast()
                .unwrap_or_else(|_| panic!("a codec serializes exactly its minted payload type"));
            Message::try_from_arc(payload, limit)
        }
        PayloadCodec {
            serialize: serialize_payload::<T>,
            deserialize: Message::deserializer::<T>(),
            limit,
        }
    }

    /// Serialize one payload value of the minted type into an
    /// admission-checked [`Message`] at the carried limit
    /// ([`Message::try_new`]'s contract).
    ///
    /// # Panics
    ///
    /// If the value is not the codec's minted payload type: a crate bug,
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

    /// Replace the carried depth limit, keeping the minted pointer.
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

/// Decode exactly one CBOR value of type `T` from `bytes`, bounding the
/// decode's recursion at `limit`: the one payload parse behind every
/// typed constructor and the minted deserializer.
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

impl Message {
    /// Creates a `Message` pairing the given object with its cached
    /// serialization, with no admission check.
    ///
    /// No unchecked constructor can reach a peer's set: insertion happens
    /// only through [`Rumors::send`](crate::Rumors::send) and
    /// [`Batch::send`](crate::Batch::send), which mint admission-checked
    /// messages through the peer's codec ([`try_new`](Self::try_new)),
    /// and through wire ingress, which runs the same decode admission
    /// runs. `new` and [`from_arc`](Self::from_arc) construct
    /// free-standing messages (trees built outside any peer, fixtures,
    /// size probes).
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

    /// Creates an admission-checked `Message`: the constructor behind
    /// [`Rumors::send`](crate::Rumors::send) and
    /// [`Batch::send`](crate::Batch::send).
    ///
    /// Serializes `message` and admits it only if the exact decode
    /// every receiver's wire ingress runs for `T` reads the encoding
    /// back within `limit`, to a value equal (by `T`'s own `Eq`) to the
    /// one sent. Because admission is the receiving computation itself,
    /// there is no second accounting to drift: a payload a receiver
    /// would reject or misread fails here instead, at its author, as
    /// the typed [`EncodeError`] (its variants name the causes). A
    /// [`Serialize`] failure keeps [`Message`]'s documented panic
    /// contract.
    pub fn try_new<T>(message: T, limit: PayloadDepthLimit) -> Result<Self, EncodeError>
    where
        T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
    {
        Self::try_from_arc(Arc::new(message), limit)
    }

    /// [`try_new`](Self::try_new) from an existing [`Arc`], without
    /// copying: the same allocation, unsized in place.
    pub(crate) fn try_from_arc<T>(
        arc: Arc<T>,
        limit: PayloadDepthLimit,
    ) -> Result<Self, EncodeError>
    where
        T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
    {
        let serialized = to_vec(&*arc);
        // Admission is the receiver's computation: the minted deserializer
        // — the same fn every receiver's wire ingress runs for this
        // payload type — reads the just-serialized bytes back at the same
        // limit.
        let decoded = match Self::deserializer::<T>()(&serialized, limit) {
            Ok(decoded) => decoded,
            Err(PayloadDecodeError::Depth(limit)) => return Err(EncodeError::Depth { limit }),
            Err(PayloadDecodeError::Io(source)) => return Err(EncodeError::Roundtrip(source)),
        };
        // Faithfulness: what a receiver reads must be the value that was
        // sent, judged by the payload type's own equality.
        let decoded: Arc<T> = decoded
            .downcast()
            .unwrap_or_else(|_| panic!("a payload decodes to its own type"));
        if *decoded != *arc {
            return Err(EncodeError::Unfaithful);
        }
        Ok(Message {
            serialized: Bytes::from(serialized),
            message: arc,
        })
    }

    /// Creates a `Message` pairing the given serialized bytes with the
    /// object derived by deserializing them as a `T`, bounding the
    /// decode's recursion at `limit`.
    ///
    /// Crate-internal rehydration over bytes that arrive outside any
    /// peer's orbit (fixtures, capture tooling), so the limit is an
    /// explicit parameter rather than a codec's: a caller rehydrating
    /// bytes written under a raised limit passes that limit. The bytes
    /// must be exactly one CBOR value: trailing bytes are rejected as
    /// invalid data, so the cache is always the value's exact encoding,
    /// and a value whose decode recurses past the limit is invalid data
    /// too.
    pub fn from_slice<T>(bytes: &[u8], limit: PayloadDepthLimit) -> io::Result<Self>
    where
        T: DeserializeOwned + Send + Sync + 'static,
    {
        let message: T = decode_exact(bytes, limit).map_err(PayloadDecodeError::into_io)?;
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
    /// type, decoded within its limit, so the cache is always the
    /// payload's exact encoding and a malformed or over-deep payload
    /// fails here, at the wire boundary, as invalid data.
    pub(crate) fn from_wire(bytes: Bytes, codec: PayloadCodec) -> io::Result<Self> {
        Ok(Message {
            message: codec.decode(&bytes).map_err(PayloadDecodeError::into_io)?,
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
    /// capture one; the codec pairs the two. Send-side admission
    /// ([`try_new`](Self::try_new)) runs this same fn over its own
    /// output, which is what makes admission and ingress one computation.
    pub(crate) fn deserializer<T>() -> PayloadDeserializer
    where
        T: DeserializeOwned + Send + Sync + 'static,
    {
        fn deserialize<T: DeserializeOwned + Send + Sync + 'static>(
            bytes: &[u8],
            limit: PayloadDepthLimit,
        ) -> Result<Arc<dyn Any + Send + Sync>, PayloadDecodeError> {
            let message: T = decode_exact(bytes, limit)?;
            Ok(Arc::new(message))
        }
        deserialize::<T>
    }

    /// Creates a `Message` from already-shared serialized bytes, without
    /// copying, bounding the decode's recursion at `limit`.
    ///
    /// The bytes are deserialized as a `T` to produce the paired object,
    /// under [`from_slice`](Self::from_slice)'s exactly-one-value,
    /// within-limit contract (its docs state why the caller supplies the
    /// limit).
    pub fn from_bytes<T>(bytes: Bytes, limit: PayloadDepthLimit) -> io::Result<Self>
    where
        T: DeserializeOwned + Send + Sync + 'static,
    {
        let message: T =
            decode_exact(bytes.as_ref(), limit).map_err(PayloadDecodeError::into_io)?;
        Ok(Message {
            message: Arc::new(message),
            serialized: bytes,
        })
    }

    /// Creates a `Message` from an existing [`Arc`], without copying: the
    /// same allocation, unsized in place.
    ///
    /// Like [`new`](Self::new), no depth admission: `new`'s docs state
    /// why the unlimited constructors cannot reach a peer's set.
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
