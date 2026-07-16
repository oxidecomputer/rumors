//! Transport handshake shared by both mirror protocols.
//!
//! Every wire session first exchanges one fixed-size [`Preamble`] carrying
//! the wire dialect, network, and session intent. Only after it succeeds does
//! either mirror exchange its causal [`Version`](crate::Version). Keeping
//! these phases separate permits a provider to learn that its peer is
//! bootstrapping before it atomically snapshots the tree and forks its party.
//!
//! ```text
//! [ magic = b"RUMORS": 6B | version: 2B (big-endian)
//! | network: 16B | intent: 1B ]
//! ```
//!
//! Its 25-byte size is part of the wire dialect, so no redundant frame length
//! precedes it. Validation diagnoses magic, then protocol version, followed by
//! the semantic network/intent combination. Only after that validation may a
//! protocol trust peer-declared lengths.

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::Network;

/// Bytes occupied by the fixed protocol marker.
const MAGIC_LEN: usize = crate::PROTOCOL_MAGIC.len();

/// Bytes occupied by the big-endian wire-version field.
const VERSION_LEN: usize = std::mem::size_of::<u16>();

/// Canonical width of one network identifier.
const NETWORK_LEN: usize = 16;

/// Bytes occupied by the intent discriminant.
const INTENT_LEN: usize = std::mem::size_of::<u8>();

/// Offset at which the wire version begins.
const VERSION_AT: usize = MAGIC_LEN;

/// Offset at which the network identifier begins.
const NETWORK_AT: usize = VERSION_AT + VERSION_LEN;

/// Offset at which the intent discriminant sits.
const INTENT_AT: usize = NETWORK_AT + NETWORK_LEN;

/// Length of the complete fixed preamble.
const PREAMBLE_LEN: usize = INTENT_AT + INTENT_LEN;

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

    /// Render the intent to its one-byte wire discriminant.
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
    /// Render one complete fixed-width preamble.
    fn encode(self) -> [u8; PREAMBLE_LEN] {
        let mut bytes = [0; PREAMBLE_LEN];
        bytes[..MAGIC_LEN].copy_from_slice(&crate::PROTOCOL_MAGIC);
        bytes[VERSION_AT..NETWORK_AT].copy_from_slice(&crate::PROTOCOL_VERSION.to_be_bytes());
        bytes[NETWORK_AT..INTENT_AT].copy_from_slice(&self.network.to_bytes());
        bytes[INTENT_AT] = self.intent.to_byte();
        bytes
    }

    /// Parse and validate one complete peer-controlled preamble.
    fn decode(bytes: &[u8; PREAMBLE_LEN]) -> Result<Self, Error> {
        let remote_magic = bytes[..MAGIC_LEN].try_into().expect("magic width");
        if remote_magic != crate::PROTOCOL_MAGIC {
            return Err(Error::MagicMismatch { remote_magic });
        }
        let remote_version = u16::from_be_bytes(
            bytes[VERSION_AT..NETWORK_AT]
                .try_into()
                .expect("version width"),
        );
        if remote_version != crate::PROTOCOL_VERSION {
            return Err(Error::VersionMismatch { remote_version });
        }

        let network = Network::from_bytes(
            bytes[NETWORK_AT..INTENT_AT]
                .try_into()
                .expect("network width"),
        );
        let intent = Intent::from_byte(bytes[INTENT_AT])?;
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
    #[error("peer is not a rumors stream (remote magic: {remote_magic:x?})")]
    MagicMismatch { remote_magic: [u8; 6] },
    /// The peer speaks a different wire dialect.
    #[error(
        "peer speaks rumors protocol version {remote_version}, we speak {}",
        crate::PROTOCOL_VERSION
    )]
    VersionMismatch { remote_version: u16 },
    /// The peer's intent byte has no defined meaning.
    #[error("peer sent an invalid intent byte ({byte:#04x})")]
    IntentInvalid { byte: u8 },
    /// A peer cannot simultaneously receive and donate an identity.
    #[error("peer claimed to bootstrap and retire in the same session")]
    BootstrapRetireConflict,
}

/// A cancel-safe, partially received fixed preamble.
pub(crate) struct Staged {
    buf: [u8; PREAMBLE_LEN],
    filled: usize,
}

impl Staged {
    /// Start with no received preamble bytes.
    pub(crate) fn new() -> Self {
        Self {
            buf: [0; PREAMBLE_LEN],
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
        R: AsyncRead + Unpin,
    {
        while self.filled < self.buf.len() {
            match reader.read(&mut self.buf[self.filled..]).await? {
                0 if self.filled == 0 => return Ok(Fill::Closed),
                0 => {
                    return Err(Error::Io(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "peer closed mid-preamble",
                    )));
                }
                read => self.filled += read,
            }
        }
        Ok(Fill::Filled)
    }

    /// Validate a completely received frame in diagnostic order.
    fn validate(&self) -> Result<Preamble, Error> {
        debug_assert_eq!(self.filled, PREAMBLE_LEN, "validate before full");
        Preamble::decode(&self.buf)
    }
}

/// Exchange the fixed preamble before either protocol trusts framed traffic.
pub(crate) async fn preamble<R, W>(
    network: Network,
    intent: Intent,
    staged: &mut Staged,
    reader: &mut R,
    writer: &mut W,
) -> Result<Preamble, Error>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let local = Preamble { network, intent }.encode();

    let write = async {
        writer.write_all(&local).await.map_err(Error::Io)?;
        writer.flush().await.map_err(Error::Io)
    };
    let read = async {
        match staged.fill(reader).await? {
            Fill::Filled => Ok(()),
            Fill::Closed => Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "peer closed before sending its preamble",
            ))),
        }
    };
    futures_util::future::try_join(write, read).await?;
    staged.validate()
}

/// Progress of a cancel-safe preamble arrival.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Fill {
    /// All 25 bytes have arrived.
    Filled,
    /// The peer closed before sending any preamble byte.
    Closed,
}

#[cfg(test)]
mod tests;
