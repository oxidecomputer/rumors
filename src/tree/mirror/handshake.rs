//! Transport handshake shared by both mirror protocols.
//!
//! Every wire session first exchanges one fixed-size [`Preamble`] carrying
//! the wire dialect, network, and session intent. Only after it succeeds does
//! either mirror exchange its greeting, whose format is the selected
//! protocol's own: V1 sends its causal [`Version`](crate::Version) alone,
//! while V2 front-loads the session parameters its protocol negotiates,
//! inventoried where they are defined — the [`streaming`](super::streaming)
//! module's `Greeting` message.
//! Keeping these phases separate permits a provider to learn that its peer is
//! bootstrapping before it atomically snapshots the tree and forks its party.
//!
//! The preamble's spelling is the selected dialect's own:
//!
//! - **V2**: one self-described CBOR item, so a V2 control stream is a
//!   CBOR sequence from its very first byte —
//!   `55799(["rumors", version: uint, network: bstr, intent: uint])`.
//!   Every field's head is one byte at the values the dialect admits, so
//!   the item is 30 bytes, fixed; that width is part of the dialect, so
//!   no redundant frame length precedes it.
//! - **V1**: the legacy fixed frame,
//!   `[ magic = b"RUMORS": 6B | version: 2B (big-endian) | network: 16B |
//!   intent: 1B ]`, 25 bytes.
//!
//! Validation diagnoses magic, then protocol version, followed by the
//! semantic network/intent combination. Only after that validation may a
//! protocol trust peer-declared lengths. A V2 endpoint additionally
//! recognizes the legacy magic and diagnoses it as a version mismatch
//! rather than a foreign protocol, so a cross-dialect pairing reports
//! what it is.

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::{
    Network, Protocol,
    tree::mirror::cbor::{self, MAJOR_BSTR, MAJOR_UINT},
};

/// Bytes occupied by the legacy fixed protocol marker.
const MAGIC_LEN: usize = crate::PROTOCOL_MAGIC.len();

/// Bytes occupied by the legacy big-endian wire-version field.
const VERSION_LEN: usize = std::mem::size_of::<u16>();

/// Canonical width of one network identifier.
const NETWORK_LEN: usize = 16;

/// Bytes occupied by the legacy intent discriminant.
const INTENT_LEN: usize = std::mem::size_of::<u8>();

/// Offset at which the legacy wire version begins.
const VERSION_AT: usize = MAGIC_LEN;

/// Offset at which the legacy network identifier begins.
const NETWORK_AT: usize = VERSION_AT + VERSION_LEN;

/// Offset at which the legacy intent discriminant sits.
const INTENT_AT: usize = NETWORK_AT + NETWORK_LEN;

/// Length of the complete legacy fixed preamble.
const LEGACY_PREAMBLE_LEN: usize = INTENT_AT + INTENT_LEN;

/// The V2 preamble's fixed prefix: the self-described CBOR tag, the
/// four-item array head, and the text item `"rumors"`.
///
/// A literal so validation is one comparison; `prefix_matches_the_writers`
/// pins it against the head writers' own rendering.
const V2_PREFIX: [u8; 11] = [
    0xd9, 0xd9, 0xf7, 0x84, 0x66, b'r', b'u', b'm', b'o', b'r', b's',
];

/// Length of the complete V2 preamble item.
pub(crate) const V2_PREAMBLE_LEN: usize = V2_PREFIX.len() + 1 + (1 + NETWORK_LEN) + INTENT_LEN;

/// The widest preamble either dialect reads.
const PREAMBLE_MAX: usize = {
    // The buffer must hold whichever dialect is selected.
    if V2_PREAMBLE_LEN > LEGACY_PREAMBLE_LEN {
        V2_PREAMBLE_LEN
    } else {
        LEGACY_PREAMBLE_LEN
    }
};

/// The exact preamble width of one dialect.
fn preamble_len(protocol: Protocol) -> usize {
    match protocol {
        #[cfg(any(test, feature = "protocol-v1"))]
        Protocol::V1 => LEGACY_PREAMBLE_LEN,
        Protocol::V2 => V2_PREAMBLE_LEN,
    }
}

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

    /// Render the intent to its wire discriminant, shared by both
    /// dialects (V1 spells it as a raw byte, V2 as a one-byte uint item).
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
    /// Render one complete preamble in the selected dialect.
    fn encode(self, protocol: Protocol) -> Vec<u8> {
        match protocol {
            #[cfg(any(test, feature = "protocol-v1"))]
            Protocol::V1 => {
                let mut bytes = [0; LEGACY_PREAMBLE_LEN];
                bytes[..MAGIC_LEN].copy_from_slice(&crate::PROTOCOL_MAGIC);
                bytes[VERSION_AT..NETWORK_AT].copy_from_slice(&(protocol as u16).to_be_bytes());
                bytes[NETWORK_AT..INTENT_AT].copy_from_slice(&self.network.to_bytes());
                bytes[INTENT_AT] = self.intent.to_byte();
                bytes.to_vec()
            }
            Protocol::V2 => {
                let mut bytes = Vec::with_capacity(V2_PREAMBLE_LEN);
                bytes.extend_from_slice(&V2_PREFIX);
                cbor::write_head(&mut bytes, MAJOR_UINT, protocol as u64);
                cbor::write_head(&mut bytes, MAJOR_BSTR, NETWORK_LEN as u64);
                bytes.extend_from_slice(&self.network.to_bytes());
                cbor::write_head(&mut bytes, MAJOR_UINT, u64::from(self.intent.to_byte()));
                debug_assert_eq!(bytes.len(), V2_PREAMBLE_LEN, "the dialect width is fixed");
                bytes
            }
        }
    }

    /// Parse and validate one complete peer-controlled preamble.
    fn decode(bytes: &[u8], protocol: Protocol) -> Result<Self, Error> {
        match protocol {
            #[cfg(any(test, feature = "protocol-v1"))]
            Protocol::V1 => Self::decode_legacy(bytes, protocol),
            Protocol::V2 => Self::decode_v2(bytes, protocol),
        }
    }

    /// Parse the legacy fixed frame.
    #[cfg(any(test, feature = "protocol-v1"))]
    fn decode_legacy(bytes: &[u8], protocol: Protocol) -> Result<Self, Error> {
        let remote_magic = bytes[..MAGIC_LEN].try_into().expect("magic width");
        if remote_magic != crate::PROTOCOL_MAGIC {
            // A V2-opening peer is a version mismatch, not a foreign
            // protocol — the mirror of the V2 decoder's legacy detection.
            if bytes[..V2_PREFIX.len()] == V2_PREFIX && bytes[V2_PREFIX.len()] < 24 {
                return Err(Error::VersionMismatch {
                    local_protocol: protocol,
                    remote_version: u64::from(bytes[V2_PREFIX.len()]),
                });
            }
            return Err(Error::MagicMismatch { remote_magic });
        }
        let remote_version = u16::from_be_bytes(
            bytes[VERSION_AT..NETWORK_AT]
                .try_into()
                .expect("version width"),
        );
        if remote_version != protocol as u16 {
            return Err(Error::VersionMismatch {
                local_protocol: protocol,
                remote_version: u64::from(remote_version),
            });
        }

        let network = Network::from_bytes(
            bytes[NETWORK_AT..INTENT_AT]
                .try_into()
                .expect("network width"),
        );
        let intent = Intent::from_byte(bytes[INTENT_AT])?;
        Self::admit(network, intent)
    }

    /// Parse the V2 self-described item.
    fn decode_v2(bytes: &[u8], protocol: Protocol) -> Result<Self, Error> {
        if bytes[..V2_PREFIX.len()] != V2_PREFIX {
            // A legacy-magic peer is a version mismatch, not a foreign
            // protocol: report what it is.
            if bytes[..MAGIC_LEN] == crate::PROTOCOL_MAGIC {
                let remote_version = u16::from_be_bytes(
                    bytes[VERSION_AT..NETWORK_AT]
                        .try_into()
                        .expect("version width"),
                );
                return Err(Error::VersionMismatch {
                    local_protocol: protocol,
                    remote_version: u64::from(remote_version),
                });
            }
            return Err(Error::MagicMismatch {
                remote_magic: bytes[..MAGIC_LEN].try_into().expect("magic width"),
            });
        }
        let mut input = &bytes[V2_PREFIX.len()..];
        let malformed = |detail| Error::Malformed { detail };
        let version = cbor::read_head(&mut input)
            .ok()
            .filter(|head| head.major == MAJOR_UINT)
            .ok_or(malformed("preamble version is not an unsigned int"))?;
        if version.value != protocol as u64 {
            return Err(Error::VersionMismatch {
                local_protocol: protocol,
                remote_version: version.value,
            });
        }
        cbor::read_head(&mut input)
            .ok()
            .filter(|head| head.major == MAJOR_BSTR && head.value == NETWORK_LEN as u64)
            .ok_or(malformed("preamble network is not a 16-byte string"))?;
        if input.len() < NETWORK_LEN {
            return Err(malformed("preamble network is truncated"));
        }
        let (network, rest) = input.split_at(NETWORK_LEN);
        input = rest;
        let network = Network::from_bytes(network.try_into().expect("network width"));
        let intent = cbor::read_head(&mut input)
            .ok()
            .filter(|head| head.major == MAJOR_UINT)
            .ok_or(malformed("preamble intent is not an unsigned int"))?;
        if !input.is_empty() {
            return Err(malformed("preamble carries trailing bytes"));
        }
        let intent = u8::try_from(intent.value)
            .map_err(|_| Error::IntentInvalid { byte: u8::MAX })
            .and_then(Intent::from_byte)?;
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
    /// The preamble opened correctly but a field was not canonical.
    #[error("peer preamble is malformed: {detail}")]
    Malformed { detail: &'static str },
    /// The peer's intent has no defined meaning.
    #[error("peer sent an invalid intent ({byte:#04x})")]
    IntentInvalid { byte: u8 },
    /// A peer cannot simultaneously receive and donate an identity.
    #[error("peer claimed to bootstrap and retire in the same session")]
    BootstrapRetireConflict,
}

/// A cancel-safe, partially received fixed preamble.
pub(crate) struct Staged {
    buf: [u8; PREAMBLE_MAX],
    /// The selected dialect's exact width, filled before validation.
    want: usize,
    protocol: Protocol,
    filled: usize,
}

impl Staged {
    /// Start with no received preamble bytes, sized for one dialect.
    pub(crate) fn new(protocol: Protocol) -> Self {
        Self {
            buf: [0; PREAMBLE_MAX],
            want: preamble_len(protocol),
            protocol,
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
        while self.filled < self.want {
            match reader.read(&mut self.buf[self.filled..self.want]).await? {
                0 if self.filled == 0 => return Ok(Fill::Closed),
                0 => {
                    // A V2 endpoint reading a legacy 25-byte preamble sees
                    // the close five bytes early; diagnose the dialect
                    // rather than reporting a bare cut. (Never under V1,
                    // whose own preamble legitimately opens with the
                    // magic, and never when the claimed version matches —
                    // that is not a dialect skew.)
                    if self.want == V2_PREAMBLE_LEN
                        && self.filled >= NETWORK_AT
                        && self.buf[..MAGIC_LEN] == crate::PROTOCOL_MAGIC
                    {
                        let remote_version = u64::from(u16::from_be_bytes(
                            self.buf[VERSION_AT..NETWORK_AT]
                                .try_into()
                                .expect("version width"),
                        ));
                        if remote_version != self.protocol as u64 {
                            return Err(Error::VersionMismatch {
                                local_protocol: self.protocol,
                                remote_version,
                            });
                        }
                    }
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
        debug_assert_eq!(self.filled, self.want, "validate before full");
        Preamble::decode(&self.buf[..self.want], self.protocol)
    }
}

/// Exchange the fixed preamble before either protocol trusts framed traffic.
pub(crate) async fn preamble<R, W>(
    protocol: Protocol,
    network: Network,
    intent: Intent,
    staged: &mut Staged,
    reader: &mut R,
    writer: &mut W,
) -> Result<Preamble, Error>
where
    R: AsyncRead + Unpin + ?Sized,
    W: AsyncWrite + Unpin + ?Sized,
{
    let local = Preamble { network, intent }.encode(protocol);

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
    /// The dialect's full preamble has arrived.
    Filled,
    /// The peer closed before sending any preamble byte.
    Closed,
}

#[cfg(test)]
mod tests;
