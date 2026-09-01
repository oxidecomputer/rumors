//! The transport handshake opening every mirror session.
//!
//! Every wire session first exchanges one fixed-size [`Preamble`] carrying
//! the wire dialect's version, the network, and the session intent. Only
//! after it succeeds does the mirror exchange its greeting, which
//! front-loads the session parameters the protocol negotiates,
//! inventoried where they are defined — the [`streaming`](super::streaming)
//! module's `Greeting` message.
//! Keeping these phases separate permits a provider to learn that its peer is
//! bootstrapping before it atomically snapshots the tree and forks its party.
//!
//! The preamble is one self-described CBOR item, so a control stream is a
//! CBOR sequence from its very first byte —
//! `55799(["rumors", version: uint, network: bstr, intent: uint])`.
//! Every field's head is one byte at the values the dialect admits, so
//! the item is 30 bytes, fixed; that width is part of the dialect, so
//! no redundant frame length precedes it.
//!
//! Validation diagnoses the opening, then the protocol version, followed
//! by the semantic network/intent combination. Only after that validation
//! may the protocol trust peer-declared lengths.

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::{
    Network, Protocol,
    observe::SessionHandle,
    tree::mirror::cbor::{self, MAJOR_BSTR, MAJOR_UINT},
};

/// Canonical width of one network identifier.
const NETWORK_LEN: usize = 16;

/// Leading bytes quoted by [`Error::MagicMismatch`] when a peer's opening
/// is not a rumors preamble: enough to recognize a familiar protocol in a
/// hex dump without echoing a whole frame.
const MISMATCH_PREVIEW_LEN: usize = 6;

/// The V2 preamble's fixed prefix: the self-described CBOR tag, the
/// four-item array head, and the text item `"rumors"`.
///
/// One flat constant so validation is one comparison; the tag's bytes
/// come from the shared spelling, and `prefix_matches_the_writers` pins
/// the whole prefix against the head writers' own rendering.
const V2_PREFIX: [u8; 11] = {
    let [a, b, c] = cbor::SELF_DESCRIBED_HEAD;
    [a, b, c, 0x84, 0x66, b'r', b'u', b'm', b'o', b'r', b's']
};

/// Length of the complete V2 preamble item: the prefix, the one-byte
/// version item, the network byte string with its one-byte head, and the
/// one-byte intent item.
pub(crate) const V2_PREAMBLE_LEN: usize = V2_PREFIX.len() + 1 + (1 + NETWORK_LEN) + 1;

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

/// The validated identity and intent carried ahead of version exchange.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Preamble {
    /// The peer's causal universe, or the bootstrap placeholder.
    pub(crate) network: Network,
    /// Whether the peer remains or retires after reconciliation.
    pub(crate) intent: Intent,
}

impl Preamble {
    /// Render one complete preamble.
    fn encode(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(V2_PREAMBLE_LEN);
        bytes.extend_from_slice(&V2_PREFIX);
        cbor::write_head(&mut bytes, MAJOR_UINT, Protocol::V2 as u64);
        cbor::write_head(&mut bytes, MAJOR_BSTR, NETWORK_LEN as u64);
        bytes.extend_from_slice(&self.network.to_bytes());
        cbor::write_head(&mut bytes, MAJOR_UINT, u64::from(self.intent.to_byte()));
        debug_assert_eq!(bytes.len(), V2_PREAMBLE_LEN, "the dialect width is fixed");
        bytes
    }

    /// Parse and validate one complete peer-controlled preamble.
    fn decode(bytes: &[u8]) -> Result<Self, Error> {
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
        if version.value != Protocol::V2 as u64 {
            return Err(Error::VersionMismatch {
                local_protocol: Protocol::V2,
                remote_version: version.value,
            });
        }
        cbor::read_head(&mut input)
            .ok()
            .filter(|head| head.major == MAJOR_BSTR && head.value == NETWORK_LEN as u64)
            .ok_or(malformed(PreambleDefect::Network))?;
        // Defensive: a validated version and network head leave 17 of the
        // fixed item's 30 bytes here, so the 16 network bytes always fit;
        // the bound keeps `split_at` in range under any layout drift.
        if input.len() < NETWORK_LEN {
            return Err(malformed(PreambleDefect::NetworkTruncated));
        }
        let (network, rest) = input.split_at(NETWORK_LEN);
        input = rest;
        let network = Network::from_bytes(network.try_into().expect("network width"));
        let intent = cbor::read_head(&mut input)
            .ok()
            .filter(|head| head.major == MAJOR_UINT)
            .ok_or(malformed(PreambleDefect::Intent))?;
        // Defensive: the one-byte intent item consumes the fixed item's
        // last byte, so nothing can trail; the check guards any caller
        // handing the decoder non-fixed input.
        if !input.is_empty() {
            return Err(malformed(PreambleDefect::TrailingBytes));
        }
        let intent = Intent::from_byte(u8::try_from(intent.value).expect(
            "the 30-byte preamble leaves exactly one byte for the intent item, \
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
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// The peer is not speaking the rumors protocol.
    #[error("peer is not a rumors stream (leading bytes: {remote_magic:x?})")]
    MagicMismatch { remote_magic: [u8; 6] },
    /// The peer speaks a different wire dialect.
    #[error("peer speaks rumors protocol version {remote_version}, we selected {local_protocol:?}")]
    VersionMismatch {
        local_protocol: Protocol,
        remote_version: u64,
    },
    /// The preamble opened correctly but a field of it is not spelled
    /// the way the dialect demands.
    #[error("peer preamble is malformed: {defect}")]
    Malformed { defect: PreambleDefect },
    /// The peer closed the stream inside its preamble.
    #[error("peer closed after sending {received} of its {expected} preamble bytes")]
    Truncated { received: usize, expected: usize },
    /// The peer's intent has no defined meaning.
    #[error("peer sent an invalid intent ({byte:#04x})")]
    IntentInvalid { byte: u8 },
    /// A peer cannot simultaneously receive and donate an identity.
    #[error("peer claimed to bootstrap and retire in the same session")]
    BootstrapRetireConflict,
}

/// Which field of a correctly-opened preamble failed to parse.
///
/// Carried by
/// [`Error::PreambleMalformed`](crate::Error::PreambleMalformed): the
/// peer opened as a rumors stream of the selected dialect, but one
/// field is not spelled the way the wire demands. The preamble is
/// deterministic-encoding CBOR — one spelling per field — so every
/// defect here is a counterparty bug, never an alternate encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PreambleDefect {
    /// The version item is not a shortest-form unsigned int.
    #[error("the version item is not an unsigned int")]
    Version,

    /// The network item is not a 16-byte byte string.
    #[error("the network item is not a 16-byte byte string")]
    Network,

    /// The network byte string's bytes end inside the preamble item.
    ///
    /// Defensively reachable only: in the fixed 30-byte V2 preamble, a
    /// validated version and network head always leave 17 bytes — the
    /// 16 network bytes and the one-byte intent — so this variant
    /// guards the decoder's width arithmetic against layout drift, not
    /// any input the current dialect admits.
    #[error("the network bytes end inside the preamble item")]
    NetworkTruncated,

    /// The intent item is not a shortest-form unsigned int.
    #[error("the intent item is not an unsigned int")]
    Intent,

    /// Bytes trail the preamble's single item.
    ///
    /// Defensively reachable only: in the fixed 30-byte V2 preamble
    /// with a validated version and network head, the one-byte intent
    /// item consumes the last byte, so this variant guards the
    /// decoder's width arithmetic against layout drift, not any input
    /// the current dialect admits.
    #[error("bytes trail the preamble item")]
    TrailingBytes,
}

/// A cancel-safe, partially received fixed preamble.
pub(crate) struct Staged {
    buf: [u8; V2_PREAMBLE_LEN],
    filled: usize,
}

impl Staged {
    /// Start with no received preamble bytes.
    pub(crate) fn new() -> Self {
        Self {
            buf: [0; V2_PREAMBLE_LEN],
            filled: 0,
        }
    }

    /// Whether an idle-boundary hang-up can still be a clean goodbye.
    pub(crate) fn is_empty(&self) -> bool {
        self.filled == 0
    }

    /// Continue receiving the fixed frame without losing cancelled progress.
    pub(crate) async fn fill<R>(&mut self, reader: &mut R) -> Result<Fill, Error>
    where
        R: AsyncRead + Unpin + ?Sized,
    {
        while self.filled < V2_PREAMBLE_LEN {
            match reader.read(&mut self.buf[self.filled..]).await? {
                0 if self.filled == 0 => return Ok(Fill::Closed),
                0 => {
                    return Err(Error::Truncated {
                        received: self.filled,
                        expected: V2_PREAMBLE_LEN,
                    });
                }
                read => self.filled += read,
            }
        }
        Ok(Fill::Filled)
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

/// Exchange the fixed preamble before the protocol trusts framed traffic.
pub(crate) async fn preamble<R, W>(
    network: Network,
    intent: Intent,
    staged: &mut Staged,
    reader: &mut R,
    writer: &mut W,
    observe: &SessionHandle,
) -> Result<Preamble, Error>
where
    R: AsyncRead + Unpin + ?Sized,
    W: AsyncWrite + Unpin + ?Sized,
{
    let local = Preamble { network, intent }.encode();

    let write = async {
        writer.write_all(&local).await.map_err(Error::Io)?;
        writer.flush().await.map_err(Error::Io)?;
        observe.control_sent(&local);
        Ok(())
    };
    let read = async {
        match staged.fill(reader).await? {
            Fill::Filled => Ok(()),
            // The peer hung up without sending a byte: a zero-length
            // truncation, distinct from a transport failure.
            Fill::Closed => Err(Error::Truncated {
                received: 0,
                expected: V2_PREAMBLE_LEN,
            }),
        }
    };
    futures_util::future::try_join(write, read).await?;
    let preamble = staged.validate()?;
    // Only a validated frame is delivered: the item contract holds for
    // conforming exchanges, and a malformed preamble aborts the session
    // instead of feeding observers a non-item.
    observe.control_received(staged.received());
    Ok(preamble)
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
