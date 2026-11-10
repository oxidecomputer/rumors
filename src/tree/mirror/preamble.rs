//! The fixed preamble opening every mirror session.
//!
//! Every wire session first exchanges one fixed-size [`Preamble`] carrying the
//! wire dialect's version, the network's public identifier (never its secret),
//! and the session intent. Only after it succeeds does the mirror exchange its
//! greeting, which front-loads the session parameters the protocol negotiates,
//! inventoried where they are defined — the [`streaming`](super::streaming)
//! module's `Greeting` message. Keeping these phases separate permits a
//! provider to learn that its peer is bootstrapping before it atomically
//! snapshots the tree and forks its party, and to send that peer the network's
//! secret (the [`secret`](super::secret) item) before the greeting.
//!
//! A bootstrapping peer sends its key item right after its preamble, in the
//! same write, and the exchange reads the key item of any counterparty whose
//! preamble declares bootstrapping, returning it with who the counterparty is
//! ([`Counterparty`]).
//!
//! The preamble is one self-described CBOR item, so each direction of a
//! session's byte stream is a CBOR sequence from its very first byte —
//! `55799(["rumors", version: uint, network: bstr, intent: uint])`. Every
//! field's head is one byte at the values the dialect admits, so the item has
//! the fixed [`V2_PREAMBLE_LEN`] width. That width is part of the dialect, so
//! no redundant frame length precedes it.
//!
//! Validation diagnoses the opening, then the protocol version, followed by the
//! semantic network/intent combination. Only after that validation may the
//! protocol trust peer-declared lengths.

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::{
    Network, Protocol,
    counter::widen,
    error::TransportOperation,
    network::NETWORK_BYTES,
    observe::SessionHandle,
    tree::mirror::cbor::{self, MAJOR_BSTR, MAJOR_UINT},
};

/// Leading bytes quoted by [`Error::MagicMismatch`] when a peer's opening is
/// not a rumors preamble: enough to recognize a familiar protocol in a hex dump
/// without echoing a whole frame.
const MISMATCH_PREVIEW_LEN: usize = 6;

/// The V2 preamble's fixed prefix: the self-described CBOR tag, the four-item
/// array head, and the text item `"rumors"`.
///
/// One flat constant so validation is one comparison; the tag's bytes come from
/// the shared spelling, and `prefix_matches_the_writers` pins the whole prefix
/// against the head writers' own rendering.
const V2_PREFIX: [u8; 11] = {
    let [a, b, c] = cbor::SELF_DESCRIBED_HEAD;
    [a, b, c, 0x84, 0x66, b'r', b'u', b'm', b'o', b'r', b's']
};

/// Length of the complete V2 preamble item: the prefix, the one-byte version
/// item, the network byte string with its one-byte head, and the one-byte
/// intent item.
pub(crate) const V2_PREAMBLE_LEN: usize = V2_PREFIX.len() + 1 + (1 + NETWORK_BYTES) + 1;

/// A peer's declared purpose for one reconciliation session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Intent {
    /// Participate and retain (or, bootstrapping, receive) an identity.
    Remain,
    /// Reconcile, then donate the peer's identity in a trailing hand-off.
    Retire,
}

impl Intent {
    /// Whether the sender promises a trailing identity donation.
    pub(crate) fn retiring(self) -> bool {
        self == Intent::Retire
    }

    /// Render the intent to its wire discriminant (a one-byte uint item).
    fn to_byte(self) -> u8 {
        match self {
            Intent::Remain => 0,
            Intent::Retire => 1,
        }
    }

    /// Parse one peer-controlled wire discriminant.
    fn from_byte(byte: u8) -> Result<Self, Error> {
        match byte {
            0 => Ok(Intent::Remain),
            1 => Ok(Intent::Retire),
            byte => Err(Error::IntentInvalid { byte }),
        }
    }
}

/// The validated network and intent carried ahead of the greeting exchange.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Preamble {
    /// The peer's network's public identifier, or the bootstrap placeholder.
    pub(crate) network: Network,
    /// Whether the peer remains or retires after reconciliation.
    pub(crate) intent: Intent,
}

impl Preamble {
    /// Render one complete preamble.
    fn encode(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(V2_PREAMBLE_LEN);
        bytes.extend_from_slice(&V2_PREFIX);
        cbor::write_head(&mut bytes, MAJOR_UINT, Protocol::V2.wire_version());
        cbor::write_head(&mut bytes, MAJOR_BSTR, widen(NETWORK_BYTES));
        bytes.extend_from_slice(&self.network.to_bytes());
        cbor::write_head(&mut bytes, MAJOR_UINT, u64::from(self.intent.to_byte()));
        debug_assert_eq!(bytes.len(), V2_PREAMBLE_LEN, "the dialect width is fixed");
        bytes
    }

    /// Parse and validate one complete peer-controlled preamble.
    fn decode(bytes: &[u8; V2_PREAMBLE_LEN]) -> Result<Self, Error> {
        if bytes[..V2_PREFIX.len()] != V2_PREFIX {
            return Err(Error::MagicMismatch {
                remote_magic: bytes[..MISMATCH_PREVIEW_LEN]
                    .try_into()
                    .expect("preview width"),
            });
        }
        let mut input = &bytes[V2_PREFIX.len()..];
        let malformed = |defect| Error::Malformed { defect };
        let version = cbor::read_head(&mut input)
            .ok()
            .filter(|head| head.major == MAJOR_UINT)
            .ok_or(malformed(PreambleDefect::Version))?;
        if version.value != Protocol::V2.wire_version() {
            return Err(Error::VersionMismatch {
                local_protocol: Protocol::V2,
                remote_version: version.value,
            });
        }
        cbor::read_head(&mut input)
            .ok()
            .filter(|head| head.major == MAJOR_BSTR && head.value == widen(NETWORK_BYTES))
            .ok_or(malformed(PreambleDefect::Network))?;
        // The canonical one-byte version and network heads leave exactly the
        // network bytes and one-byte intent in this fixed-width input.
        let (network, rest) = input.split_at(NETWORK_BYTES);
        input = rest;
        let network = Network::from_bytes(network.try_into().expect("network width"));
        let intent = cbor::read_head(&mut input)
            .ok()
            .filter(|head| head.major == MAJOR_UINT)
            .ok_or(malformed(PreambleDefect::Intent))?;
        let intent = Intent::from_byte(u8::try_from(intent.value).expect(
            "the fixed-width preamble leaves exactly one byte for the intent item, \
             whose one-byte head's value is at most 23",
        ))?;
        Self::admit(network, intent)
    }

    /// Enforce the semantic network/intent combination.
    fn admit(network: Network, intent: Intent) -> Result<Self, Error> {
        if network.is_bootstrap() && intent.retiring() {
            return Err(Error::BootstrapRetireConflict);
        }
        Ok(Self { network, intent })
    }
}

/// A malformed, incompatible, or truncated preamble.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    /// Reading or writing the fixed frame failed.
    #[error("{operation} failed during the preamble")]
    Io {
        /// The failed control-stream operation.
        operation: TransportOperation,
        /// The transport's original error.
        #[source]
        source: std::io::Error,
    },
    /// The peer is not speaking the rumors protocol.
    #[error("peer is not a rumors stream (leading bytes: {remote_magic:x?})")]
    MagicMismatch {
        /// The peer's leading bytes, retained to diagnose the wrong protocol.
        remote_magic: [u8; MISMATCH_PREVIEW_LEN],
    },
    /// The peer speaks a different wire dialect.
    #[error(
        "peer speaks rumors protocol version {remote_version}, while this build speaks {local_protocol:?}"
    )]
    VersionMismatch {
        /// The dialect spoken by this build.
        local_protocol: Protocol,
        /// The version number advertised by the peer.
        remote_version: u64,
    },
    /// The preamble opened correctly but a field of it is not spelled the way
    /// the dialect demands.
    #[error("peer preamble is malformed")]
    Malformed {
        /// The field or encoding rule the preamble violated.
        #[source]
        defect: PreambleDefect,
    },
    /// The peer closed the stream inside its preamble.
    #[error("peer closed after sending {received} of its {expected} preamble bytes")]
    Truncated {
        /// The preamble bytes received before the stream closed.
        received: usize,
        /// The bytes required for a complete preamble.
        expected: usize,
    },
    /// The peer's intent has no defined meaning.
    #[error("peer sent an invalid intent ({byte:#04x})")]
    IntentInvalid {
        /// The unrecognized intent byte.
        byte: u8,
    },
    /// A peer cannot simultaneously receive and donate an identity.
    #[error("peer claimed to bootstrap and retire in the same session")]
    BootstrapRetireConflict,
}

/// The invalid field in a fully received preamble.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PreambleDefect {
    /// The version item is not a shortest-form unsigned int.
    #[error("the version item is not an unsigned int")]
    Version,

    /// The network item is not a 16-byte byte string.
    #[error("the network item is not a 16-byte byte string")]
    Network,

    /// The intent item is not a shortest-form unsigned int.
    #[error("the intent item is not an unsigned int")]
    Intent,
}

/// A cancel-safe, partially received fixed preamble.
pub(crate) struct Staged {
    /// The fixed frame's bytes, received without reading into the next item.
    buf: [u8; V2_PREAMBLE_LEN],
    /// Initialized prefix of `buf`.
    filled: usize,
}

/// Preserve received bytes across the driver's idle wait and active exchange.
impl Staged {
    /// Start with no received preamble bytes.
    pub(crate) fn new() -> Self {
        Self {
            buf: [0; V2_PREAMBLE_LEN],
            filled: 0,
        }
    }

    /// Whether no preamble byte has arrived, so EOF can be a clean goodbye.
    pub(crate) fn is_empty(&self) -> bool {
        self.filled == 0
    }

    /// Wait for initiation bytes; return false for EOF at an empty boundary.
    ///
    /// Return after the first read so the driver can start its session deadline
    /// even if the peer stops partway through the preamble.
    pub(crate) async fn wait_for_start<R>(&mut self, reader: &mut R) -> Result<bool, Error>
    where
        R: AsyncRead + Unpin + ?Sized,
    {
        if self.is_empty() {
            self.read_more(reader).await?;
        }
        Ok(!self.is_empty())
    }

    /// Continue receiving the fixed frame without losing cancelled progress.
    pub(crate) async fn fill<R>(&mut self, reader: &mut R) -> Result<Fill, Error>
    where
        R: AsyncRead + Unpin + ?Sized,
    {
        while self.filled < V2_PREAMBLE_LEN {
            match self.read_more(reader).await? {
                0 if self.filled == 0 => return Ok(Fill::Closed),
                0 => {
                    return Err(Error::Truncated {
                        received: self.filled,
                        expected: V2_PREAMBLE_LEN,
                    });
                }
                _ => {}
            }
        }
        Ok(Fill::Filled)
    }

    /// Append one transport read to the frame, leaving later items unread.
    async fn read_more<R>(&mut self, reader: &mut R) -> Result<usize, Error>
    where
        R: AsyncRead + Unpin + ?Sized,
    {
        let read = reader
            .read(&mut self.buf[self.filled..])
            .await
            .map_err(|source| Error::Io {
                operation: TransportOperation::Read,
                source,
            })?;
        self.filled += read;
        Ok(read)
    }

    /// Validate a completely received frame in diagnostic order.
    fn validate(&self) -> Result<Preamble, Error> {
        debug_assert_eq!(self.filled, V2_PREAMBLE_LEN, "validate before full");
        Preamble::decode(&self.buf)
    }

    /// The completely received frame's bytes.
    fn received(&self) -> &[u8] {
        debug_assert_eq!(self.filled, V2_PREAMBLE_LEN, "read back before full");
        &self.buf
    }
}

/// A bootstrapping peer's key item, which follows its preamble.
pub(crate) type KeyItem = [u8; crate::tree::mirror::secret::KEY_ITEM_LEN];

/// Who the counterparty is, read from its validated preamble: what every
/// session decides by before its greeting.
///
/// A peer is either joining, and then has no network, no intent but to remain,
/// and a key item following its preamble, or a member of a network remaining or
/// retiring after reconciliation. One value carries each case whole, so a
/// session never checks the bootstrap placeholder and the key item's presence
/// separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Counterparty {
    /// A peer joining a network, with the key item that followed its preamble:
    /// a provider seals the network's secret for that key.
    Bootstrapping(KeyItem),
    /// A member of a network.
    Established {
        /// The member's network's public identifier.
        network: Network,
        /// Whether the member remains or retires after reconciliation.
        intent: Intent,
    },
}

impl Counterparty {
    /// Whether the counterparty is joining a network.
    pub(crate) fn bootstrapping(&self) -> bool {
        matches!(self, Counterparty::Bootstrapping(_))
    }

    /// Whether the counterparty retires into this side after reconciliation.
    pub(crate) fn retiring(&self) -> bool {
        matches!(
            self,
            Counterparty::Established {
                intent: Intent::Retire,
                ..
            }
        )
    }
}

/// Exchange the fixed preamble before the protocol trusts framed traffic.
///
/// `follow` is one whole item this side sends right after its preamble, in the
/// same write and flush, before it has read the peer's preamble: a
/// bootstrapping peer's key item, so that its key costs no round trip. Empty
/// for every other session.
///
/// Returns who the peer is, with the key item that follows a bootstrapping
/// peer's preamble ([`Counterparty`]). The key item is read while this side's
/// own write is still in flight: two bootstrapping peers each write a preamble
/// and a key item before reading anything past the other's preamble, so if
/// either waited for its write to finish before draining the other's key item,
/// a transport that buffers less than both could leave them waiting on each
/// other forever.
// The arguments are the exchange's inputs, one per concern.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn preamble<R, W>(
    network: Network,
    intent: Intent,
    follow: &[u8],
    staged: &mut Staged,
    reader: &mut R,
    writer: &mut W,
    observe: &SessionHandle,
) -> Result<Counterparty, Error>
where
    R: AsyncRead + Unpin + ?Sized,
    W: AsyncWrite + Unpin + ?Sized,
{
    let local = Preamble { network, intent }.encode();
    let mut sent = Vec::with_capacity(local.len() + follow.len());
    sent.extend_from_slice(&local);
    sent.extend_from_slice(follow);

    let write = async {
        writer.write_all(&sent).await.map_err(|source| Error::Io {
            operation: TransportOperation::Write,
            source,
        })?;
        writer.flush().await.map_err(|source| Error::Io {
            operation: TransportOperation::Flush,
            source,
        })?;
        observe.control_sent(&local);
        if !follow.is_empty() {
            observe.control_sent(follow);
        }
        Ok(())
    };
    let read = async {
        match staged.fill(reader).await? {
            Fill::Filled => {}
            // The peer hung up without sending a byte: a zero-length
            // truncation, distinct from a transport failure.
            Fill::Closed => {
                return Err(Error::Truncated {
                    received: 0,
                    expected: V2_PREAMBLE_LEN,
                });
            }
        }
        // A validation failure is reported once both directions are done;
        // only a valid bootstrapping preamble promises a key item to read.
        let counterparty = match staged.validate() {
            Ok(preamble) if preamble.network.is_bootstrap() => {
                let mut item = [0; crate::tree::mirror::secret::KEY_ITEM_LEN];
                reader
                    .read_exact(&mut item)
                    .await
                    .map_err(|source| Error::Io {
                        operation: TransportOperation::Read,
                        source,
                    })?;
                Ok(Counterparty::Bootstrapping(item))
            }
            Ok(Preamble { network, intent }) => Ok(Counterparty::Established { network, intent }),
            Err(error) => Err(error),
        };
        Ok(counterparty)
    };
    let ((), counterparty) = futures::future::try_join(write, read).await?;
    let counterparty = counterparty?;
    // Only a validated frame is delivered: the item contract holds for
    // conforming exchanges, and a malformed preamble aborts the session
    // instead of feeding observers a non-item.
    observe.control_received(staged.received());
    if let Counterparty::Bootstrapping(item) = &counterparty {
        observe.control_received(item);
    }
    Ok(counterparty)
}

/// Progress of a cancel-safe preamble arrival.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Fill {
    /// The dialect's full preamble has arrived.
    Filled,
    /// The peer closed before sending any preamble byte.
    Closed,
}

#[cfg(test)]
mod tests;
