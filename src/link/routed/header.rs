//! The connect header: the bytes that route a connection to its link.
//!
//! Every connection dialed by this module opens with one header, written
//! by the dialer before any protocol byte, and read by the accepting
//! router as its only I/O on the connection. Two kinds exist: `LINK`
//! establishes a link (the connection becomes the control stream, and
//! the header carries the dialer's advertised name for reverse dials),
//! `STREAM` attaches one data stream to an existing link. `LINK`
//! connections receive a one-byte acknowledgement; see the [module
//! docs](super) for the race it closes.
//!
//! Layout, integers big-endian, lengths fixed per kind:
//!
//! ```text
//! magic    10  b"ROUTEDLINK"
//! version   1  0x01
//! kind      1  0x01 LINK | 0x02 STREAM
//! token    16  the link's identity
//! LINK only:
//! addr_len  1  advertised-name length, 1..=MAX_ADDR_LEN
//! addr      …  Addr::encode() of the dialer's advertised name
//! ```
//!
//! The header carries no epoch and no stream index: the session labels
//! its streams itself (the label is the stream's first payload bytes,
//! after this header), the router routes on the token alone, and a
//! second copy of the session's label would be a consistency obligation
//! with no checker. The version byte is the compatibility door: any
//! future shape (a reusable-lease kind, say) arrives as a new version
//! or kind, never a mutation of these.

use std::fmt;
use std::io;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};

use tokio::io::{AsyncRead, AsyncReadExt};

/// The header's opening magic.
///
/// Named for what the bytes introduce (a routed-link connect header),
/// deliberately unrelated to the session preamble's own magic: the two
/// travel adjacent on the wire (a control connection carries this
/// header, then the session preamble), and distinct magics turn a
/// misrouted or misaligned connection into a precise first-read error.
const MAGIC: &[u8; 10] = b"ROUTEDLINK";

/// The one wire version this module speaks.
const VERSION: u8 = 1;

/// Kind byte: this connection establishes a link and becomes its
/// control stream.
const KIND_LINK: u8 = 1;

/// Kind byte: this connection is one data stream of an existing link.
const KIND_STREAM: u8 = 2;

/// The fixed-width header prefix every kind shares: magic, version,
/// kind, token.
pub(super) const PREFIX_LEN: usize = MAGIC.len() + 2 + TOKEN_LEN;

/// The acknowledgement byte a router writes back on a `LINK`
/// connection once the link's token is registered and its end of the
/// link is on its way to the application.
pub(super) const ACK: u8 = 1;

/// Bytes in a [`Token`].
const TOKEN_LEN: usize = 16;

/// Longest advertised name a `LINK` header carries.
///
/// [`Addr::encode`] must fit within it; the one-byte length prefix
/// makes the bound structural, and keeps the router's header read
/// bounded by construction.
pub const MAX_ADDR_LEN: usize = u8::MAX as usize;

/// One link's routing identity: 16 random bytes minted at
/// establishment.
///
/// Both routers key the link's connection queue by its token, and every
/// data-stream dial quotes it. The width makes collision handling a
/// non-problem; the token is routing state, never a credential (the
/// transport below is authenticated, per the [module docs](super)).
/// Tokens are minted by [`Endpoint::link`](super::Endpoint::link) and
/// observed through [`LinkInfo`](super::LinkInfo); they cannot be
/// constructed.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Token([u8; TOKEN_LEN]);

impl Token {
    /// Mint a fresh random token.
    pub(super) fn new() -> Self {
        Token(rand::random())
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Token({})", hex::encode(self.0))
    }
}

/// Names a dialable peer in the caller's transport namespace.
///
/// An address is whatever the caller's [`Dial`](super::Dial) dials by:
/// a socket address, a filesystem path, an overlay peer id. The adapter
/// ferries one address per link — the dialer's advertised name, inside
/// the `LINK` header — and never interprets it; encoding exists so the
/// name survives the trip.
///
/// `encode` must produce between 1 and [`MAX_ADDR_LEN`] bytes (an
/// endpoint's own advertised name is checked at construction), and
/// `decode` must invert it: `decode(&encode(a))` yields an address that
/// dials the same peer as `a`. An implementation whose namespace holds
/// names its encoding cannot carry faithfully must refuse them (panic in
/// `encode`) rather than silently alter the dialed peer. `decode`
/// returns `None` for bytes that name nothing; the router drops the
/// connection that carried them.
pub trait Addr: Clone + Send + Sync + 'static {
    /// Encode this name for the wire.
    fn encode(&self) -> Vec<u8>;

    /// Decode a wire name, or `None` if the bytes name nothing.
    fn decode(bytes: &[u8]) -> Option<Self>;
}

/// Socket-address bytes: a 16-byte IP (IPv4 mapped into IPv6) and a
/// big-endian port.
const SOCKET_ADDR_LEN: usize = 18;

/// The stock instantiation for IP transports: 18 bytes, IPv4 carried
/// v4-mapped.
///
/// Decoding canonicalizes: a v4-mapped IPv6 address comes back as the
/// IPv4 address it maps, which dials the same peer. An IPv6 `flowinfo`
/// labels a flow, not a peer, so it is not part of the name: it is not
/// carried, and a decoded address bears flowinfo zero — still the same
/// peer.
///
/// # Panics
///
/// Encoding an IPv6 address with a nonzero `scope_id`. The scope names
/// the interface a link-local peer is reachable through; the 18-byte
/// name cannot carry it, and the unscoped address does *not* dial the
/// same peer, so a scoped advertised name is a configuration bug caught
/// at endpoint construction (the one place the router encodes). A
/// link-local deployment needs a caller-supplied [`Addr`] whose
/// encoding carries the scope.
impl Addr for SocketAddr {
    fn encode(&self) -> Vec<u8> {
        if let SocketAddr::V6(v6) = self {
            assert_eq!(
                v6.scope_id(),
                0,
                "a scoped IPv6 address has no 18-byte wire name: dropping \
                 the scope would dial a different peer",
            );
        }
        let ip = match self.ip() {
            IpAddr::V4(v4) => v4.to_ipv6_mapped(),
            IpAddr::V6(v6) => v6,
        };
        let mut bytes = Vec::with_capacity(SOCKET_ADDR_LEN);
        bytes.extend_from_slice(&ip.octets());
        bytes.extend_from_slice(&self.port().to_be_bytes());
        bytes
    }

    fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != SOCKET_ADDR_LEN {
            return None;
        }
        let ip: [u8; 16] = bytes[..16].try_into().expect("length checked above");
        let port = u16::from_be_bytes(bytes[16..].try_into().expect("length checked above"));
        let ip = Ipv6Addr::from(ip);
        let ip = match ip.to_ipv4_mapped() {
            Some(v4) => IpAddr::V4(v4),
            None => IpAddr::V6(ip),
        };
        Some(SocketAddr::new(ip, port))
    }
}

/// One parsed connect header.
#[derive(Debug)]
pub(super) enum Header<A> {
    /// Establish a link: the connection is its control stream, and
    /// `peer` is the dialer's advertised name for reverse dials.
    Link {
        /// The new link's identity.
        token: Token,
        /// Where this link's outgoing data streams dial.
        peer: A,
    },
    /// Attach one data stream to the link `token` names.
    Stream {
        /// The owning link's identity.
        token: Token,
    },
}

/// Encode a `LINK` header.
///
/// `addr` is the dialer's advertised name, already encoded and already
/// length-checked (the endpoint validates it at construction).
pub(super) fn link_header(token: &Token, addr: &[u8]) -> Vec<u8> {
    debug_assert!((1..=MAX_ADDR_LEN).contains(&addr.len()));
    let mut bytes = Vec::with_capacity(PREFIX_LEN + 1 + addr.len());
    bytes.extend_from_slice(MAGIC);
    bytes.push(VERSION);
    bytes.push(KIND_LINK);
    bytes.extend_from_slice(&token.0);
    bytes.push(addr.len() as u8);
    bytes.extend_from_slice(addr);
    bytes
}

/// Encode a `STREAM` header.
pub(super) fn stream_header(token: &Token) -> [u8; PREFIX_LEN] {
    let mut bytes = [0; PREFIX_LEN];
    bytes[..MAGIC.len()].copy_from_slice(MAGIC);
    bytes[MAGIC.len()] = VERSION;
    bytes[MAGIC.len() + 1] = KIND_STREAM;
    bytes[MAGIC.len() + 2..].copy_from_slice(&token.0);
    bytes
}

/// Shorthand for this module's malformed-header errors.
fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

/// Read and parse one connect header from a fresh inbound connection.
///
/// This is the router's only read on any connection: a fixed-width
/// prefix, then (for `LINK`) a length byte and exactly that many name
/// bytes, so the read is bounded by construction.
///
/// # Errors
///
/// Malformed bytes (wrong magic, unknown version or kind, a name that
/// does not decode) and truncation both fail with the underlying
/// classification; the router responds to either by dropping the
/// connection.
pub(super) async fn read<A: Addr, R: AsyncRead + Unpin>(conn: &mut R) -> io::Result<Header<A>> {
    let mut prefix = [0; PREFIX_LEN];
    conn.read_exact(&mut prefix).await?;
    if &prefix[..MAGIC.len()] != MAGIC {
        return Err(invalid("connect header: not a routed-link connection"));
    }
    if prefix[MAGIC.len()] != VERSION {
        return Err(invalid("connect header: unknown version"));
    }
    let kind = prefix[MAGIC.len() + 1];
    let token = Token(
        prefix[MAGIC.len() + 2..]
            .try_into()
            .expect("prefix layout fixes the token width"),
    );
    match kind {
        KIND_STREAM => Ok(Header::Stream { token }),
        KIND_LINK => {
            let mut len = [0; 1];
            conn.read_exact(&mut len).await?;
            let len = usize::from(len[0]);
            if len == 0 {
                return Err(invalid("connect header: empty advertised name"));
            }
            let mut addr = vec![0; len];
            conn.read_exact(&mut addr).await?;
            let peer = A::decode(&addr)
                .ok_or_else(|| invalid("connect header: undecodable advertised name"))?;
            Ok(Header::Link { token, peer })
        }
        _ => Err(invalid("connect header: unknown kind")),
    }
}

#[cfg(test)]
mod tests;
