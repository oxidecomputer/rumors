//! Defaults for sessions over TLS 1.3 on TCP, across an Ethernet link.
//!
//! These defaults assume the deployment most sessions run in: TLS 1.3
//! over TCP over IPv4, on a path whose MTU is Ethernet's 1,500 bytes,
//! with the TCP timestamps option on, as Linux, the BSDs and macOS send it
//! by default. Under those assumptions one full TCP segment carries 1,448
//! bytes of payload, and a TLS 1.3 record adds 22 bytes to the plaintext
//! it seals.
//!
//! For another deployment, recompute what the default derives from:
//!
//! - **Plain TCP** has no record framing. Start the write batch range at
//!   the segment payload, 1,448 bytes, and end it where you like: no
//!   record limit applies, so a larger end, up to the session's write
//!   buffer, makes fewer writes.
//! - **IPv6** headers are 20 bytes longer than IPv4's: subtract 20 from
//!   the start, giving 1,406 bytes over TLS.
//! - **Tunnels and VPNs** lower the MTU by their encapsulation, such as
//!   about 60 to 80 bytes for WireGuard or IPsec: subtract it from the
//!   start as well.
//! - **Larger MTUs**, such as 9,000-byte jumbo frames, raise the start by
//!   the same amount: 8,926 bytes over TLS.
//!
//! The end, TLS 1.3's largest record, holds for TLS over any link.

use std::ops::RangeInclusive;

use crate::tree::mirror::streaming::remote::WRITE_BUFFER_BYTES;

/// The write batch range a peer uses unless you set one
/// ([`Peer::write_batch_range`](crate::Peer::write_batch_range)): from
/// 1,426 to 16,384 bytes, inclusive.
///
/// It runs from the most plaintext one TLS 1.3 record carries while the
/// record fills one TCP segment to the most plaintext any TLS 1.3 record
/// carries.
///
/// The start is the *segment size*. A TCP segment on a 1,500-byte
/// Ethernet link over IPv4 carries 1,448 bytes of payload: the MTU less
/// 20 bytes of IPv4 header, 20 of TCP header, and 12 of the TCP timestamps
/// option. A TLS 1.3 record adds 22 bytes to its plaintext: a 5-byte record
/// header, a 1-byte inner content type, and a 16-byte AEAD tag. So a batch
/// of at most 1,448 − 22 = 1,426 bytes leaves as one record of at most
/// 1,448 bytes, which fits one segment.
///
/// The end is the *largest batch*: TLS 1.3 limits a record's plaintext to
/// 2<sup>14</sup> = 16,384 bytes (RFC 8446, section 5.1), so a batch of up
/// to 16,384 bytes leaves as one record, and a larger one would take two.
pub const WRITE_BATCH_RANGE: RangeInclusive<usize> = SEGMENT_PLAINTEXT..=TLS13_MAX_PLAINTEXT;

/// The Ethernet MTU, in bytes: the largest IP packet one frame carries.
const ETHERNET_MTU: usize = 1_500;

/// The bytes of an IPv4 header without options.
const IPV4_HEADER_BYTES: usize = 20;

/// The bytes of a TCP header without options.
const TCP_HEADER_BYTES: usize = 20;

/// The bytes the TCP timestamps option adds to every segment, padding
/// included.
const TCP_TIMESTAMPS_BYTES: usize = 12;

/// The TCP payload one full-size segment carries over IPv4 on Ethernet.
const TCP_SEGMENT_PAYLOAD: usize =
    ETHERNET_MTU - IPV4_HEADER_BYTES - TCP_HEADER_BYTES - TCP_TIMESTAMPS_BYTES;

/// The bytes of a TLS record header: content type, version, and length.
const TLS_RECORD_HEADER_BYTES: usize = 5;

/// The bytes of a TLS 1.3 record's inner content type, encrypted with its
/// plaintext.
const TLS13_INNER_CONTENT_TYPE_BYTES: usize = 1;

/// The bytes of the AEAD tag every TLS 1.3 cipher suite appends.
const AEAD_TAG_BYTES: usize = 16;

/// The bytes a TLS 1.3 record adds to the plaintext it carries, without
/// padding.
const TLS13_RECORD_OVERHEAD: usize =
    TLS_RECORD_HEADER_BYTES + TLS13_INNER_CONTENT_TYPE_BYTES + AEAD_TAG_BYTES;

/// The most plaintext one TLS 1.3 record carries while the record fits
/// one full-size TCP segment.
const SEGMENT_PLAINTEXT: usize = TCP_SEGMENT_PAYLOAD - TLS13_RECORD_OVERHEAD;

/// The most plaintext any TLS 1.3 record carries, 2<sup>14</sup> bytes
/// (RFC 8446, section 5.1).
const TLS13_MAX_PLAINTEXT: usize = 1 << 14;

const _: () = assert!(
    TLS13_MAX_PLAINTEXT <= WRITE_BUFFER_BYTES,
    "the default largest batch fits the write buffer, so it applies as given rather than saturating"
);
