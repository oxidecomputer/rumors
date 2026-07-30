use std::io;
use std::net::{IpAddr, SocketAddr};

use proptest::prelude::*;

use super::*;

/// Parse a header from raw bytes, as the router would from a fresh
/// connection.
fn parse(mut bytes: &[u8]) -> io::Result<Header<SocketAddr>> {
    pollster::block_on(read::<SocketAddr, _>(&mut bytes))
}

/// A `STREAM` header carries its token through encode and parse
/// unchanged: the router routes on exactly the identity the dialer
/// quoted.
#[test]
fn stream_header_roundtrips() {
    let token = Token::new();
    match parse(&stream_header(&token)).expect("a well-formed header parses") {
        Header::Stream { token: parsed } => assert_eq!(parsed, token),
        Header::Link { .. } => panic!("a STREAM header must parse as a stream"),
    }
}

/// A `LINK` header carries both its token and the dialer's advertised
/// name through encode and parse: the accept side dials back exactly
/// the name the dialer advertised.
#[test]
fn link_header_roundtrips() {
    let token = Token::new();
    let advertised: SocketAddr = "127.0.0.1:7000".parse().expect("literal address");
    let bytes = link_header(&token, &advertised.encode());
    match parse(&bytes).expect("a well-formed header parses") {
        Header::Link {
            token: parsed,
            peer,
        } => {
            assert_eq!(parsed, token);
            assert_eq!(peer, advertised);
        }
        Header::Stream { .. } => panic!("a LINK header must parse as a link"),
    }
}

/// Bytes that do not open with the routed-link magic are rejected as
/// invalid data: a connection misrouted from another protocol fails at
/// the first read, precisely.
#[test]
fn wrong_magic_is_rejected() {
    let mut bytes = stream_header(&Token::new()).to_vec();
    bytes[0] = b'X';
    let error = parse(&bytes).expect_err("a foreign magic must not parse");
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
}

/// An unknown version byte is rejected: the version is the
/// compatibility door, and this parser speaks exactly one.
#[test]
fn unknown_version_is_rejected() {
    let mut bytes = stream_header(&Token::new()).to_vec();
    bytes[MAGIC.len()] = VERSION + 1;
    let error = parse(&bytes).expect_err("an unknown version must not parse");
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
}

/// An unknown kind byte is rejected: kinds are closed vocabulary, and
/// anything else is a peer speaking a wire this router does not.
#[test]
fn unknown_kind_is_rejected() {
    let mut bytes = stream_header(&Token::new()).to_vec();
    bytes[MAGIC.len() + 1] = 9;
    let error = parse(&bytes).expect_err("an unknown kind must not parse");
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
}

/// A `LINK` header advertising a zero-length name is rejected: a
/// nameless peer cannot be dialed back, so the establishment is
/// malformed by construction.
#[test]
fn empty_advertised_name_is_rejected() {
    let token = Token::new();
    let mut bytes = link_header(&token, &[0]);
    // Rewrite the length byte to zero and drop the placeholder name.
    bytes[PREFIX_LEN] = 0;
    bytes.truncate(PREFIX_LEN + 1);
    let error = parse(&bytes).expect_err("an empty name must not parse");
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
}

/// An advertised name the address type cannot decode is rejected: the
/// router refuses to build a link whose reverse dials would name
/// nothing.
#[test]
fn undecodable_advertised_name_is_rejected() {
    let token = Token::new();
    let bytes = link_header(&token, b"not-a-socket-address");
    let error = parse(&bytes).expect_err("an undecodable name must not parse");
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
}

/// A connection that closes mid-header surfaces as truncation, never a
/// partial parse: the router's read is exact-length by construction.
#[test]
fn truncation_is_rejected() {
    let token = Token::new();
    let advertised: SocketAddr = "[::1]:9".parse().expect("literal address");
    let full = link_header(&token, &advertised.encode());
    for len in 0..full.len() {
        let error = parse(&full[..len]).expect_err("a truncated header must not parse");
        assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof);
    }
}

proptest! {
    /// Every socket address survives the wire: decoding an encoded
    /// address yields one that encodes identically and dials the same
    /// peer.
    ///
    /// The wire form is canonical: IPv4 addresses come back exactly,
    /// and a v4-mapped IPv6 address canonicalizes to the IPv4 it maps.
    #[test]
    fn socket_addr_roundtrips(ip in any::<[u8; 16]>(), v4 in any::<bool>(), port in any::<u16>()) {
        let addr = if v4 {
            let octets: [u8; 4] = ip[..4].try_into().expect("four bytes");
            SocketAddr::new(IpAddr::from(octets), port)
        } else {
            SocketAddr::new(IpAddr::from(ip), port)
        };
        let encoded = addr.encode();
        prop_assert_eq!(encoded.len(), SOCKET_ADDR_LEN);
        let decoded = SocketAddr::decode(&encoded).expect("an encoded address decodes");
        prop_assert_eq!(decoded.port(), addr.port());
        prop_assert_eq!(&decoded.encode(), &encoded);
        if v4 {
            prop_assert_eq!(decoded, addr);
        }
    }
}
