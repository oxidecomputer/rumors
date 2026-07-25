//! Ingress validation of the V1 framed-message reader.
//!
//! Every V1 wire message reaches the protocol through [`recv_msg`]: one
//! length-delimited frame pulled off the transport, then one exact borsh
//! decode of the frame's body. Both halves parse peer-controlled bytes, so
//! this suite feeds the reader crafted frames — truncations at each
//! structural boundary, length lies in both directions, trailing garbage —
//! and pins that each surfaces as the typed [`Error::Io`], never a panic,
//! never a hang, and never a partial message. Body validity beyond framing
//! (canonical channel order, node structure) is pinned in
//! `alternating/message/tests.rs`; the exact wire bytes in
//! `alternating/wire_snapshot.rs`.

use borsh::BorshDeserialize;
use proptest::collection::vec;
use proptest::prelude::*;

use super::super::super::message;
use super::{FrameRead, recv_msg};
use crate::tree::arb::nth_party;
use crate::tree::mirror::framing::LENGTH_HEADER_LEN;
use crate::tree::typed::height::UnderRoot;
use crate::{Error, Version};

/// Length-delimit one frame body exactly as [`super::send_msg`] does.
fn frame(body: &[u8]) -> Vec<u8> {
    let len = u32::try_from(body.len()).expect("test frame bodies fit in u32");
    let mut bytes = len.to_be_bytes().to_vec();
    bytes.extend_from_slice(body);
    bytes
}

/// Pull one message from crafted wire bytes through the production ingress.
fn recv<M: BorshDeserialize>(bytes: &[u8]) -> Result<M, Error> {
    pollster::block_on(async {
        let mut reader = FrameRead::new(bytes);
        recv_msg::<M, _>(&mut reader).await
    })
}

/// The canonical encoding of a greeting whose version is nonempty, so a
/// truncation leaves bytes to cut.
fn handshake_bytes() -> Vec<u8> {
    let party = nth_party(0);
    let mut version = Version::new();
    version.tick(&party);
    borsh::to_vec(&message::Handshake { version }).expect("test handshakes encode")
}

/// Unwrap the sole error variant this ingress can produce.
fn io_error(result: Result<(), Error>) -> borsh::io::Error {
    match result {
        Err(Error::Io(error)) => error,
        other => panic!("the framed ingress fails as Error::Io, got {other:?}"),
    }
}

/// A peer that closes instead of sending the expected message surfaces a
/// typed EOF naming the missing message.
///
/// The close-at-a-boundary cut is remapped by [`recv_msg`] onto a
/// deliberate diagnosis — "peer closed before sending expected message" —
/// so a protocol desync reads as such rather than as a bare I/O fragment.
#[test]
fn close_before_a_message_is_a_typed_eof() {
    let error = io_error(recv::<message::Handshake>(&[]).map(|_| ()));
    assert_eq!(error.kind(), borsh::io::ErrorKind::UnexpectedEof);
    assert!(
        error.to_string().contains("peer closed"),
        "the boundary close carries its diagnosis, got {error}",
    );
}

/// A close inside the four-byte length header is a typed EOF, not a hang.
///
/// The header is the first peer-controlled structure of every frame; each
/// strict prefix of it must resolve to [`Error::Io`] with `UnexpectedEof`.
#[test]
fn truncated_length_header_is_a_typed_eof() {
    for cut in 1..LENGTH_HEADER_LEN {
        let error = io_error(recv::<message::Handshake>(&vec![0; cut]).map(|_| ()));
        assert_eq!(
            error.kind(),
            borsh::io::ErrorKind::UnexpectedEof,
            "cut after {cut} header bytes must be an unexpected EOF",
        );
    }
}

/// A frame declaring more bytes than the peer sends is a typed EOF.
///
/// The over-declared length makes the exact body read run off the end of
/// the stream; the lie must surface as [`Error::Io`] with `UnexpectedEof`,
/// never as a partially filled frame handed to the message decoder.
#[test]
fn over_declared_frame_is_a_typed_eof() {
    let mut bytes = 16_u32.to_be_bytes().to_vec();
    bytes.extend_from_slice(&[1, 2, 3, 4]);

    let error = io_error(recv::<message::Handshake>(&bytes).map(|_| ()));
    assert_eq!(error.kind(), borsh::io::ErrorKind::UnexpectedEof);
}

/// A message cut short inside an honestly sized frame is a typed error.
///
/// The frame's header matches its body, but the body is a strict prefix of
/// the message's canonical encoding — the under-declared-length lie. The
/// decoder runs out of bytes and must surface [`Error::Io`], never accept
/// a shorter message.
#[test]
fn under_declared_frame_is_a_typed_error() {
    let mut body = handshake_bytes();
    body.truncate(body.len() - 1);

    let error = io_error(recv::<message::Handshake>(&frame(&body)).map(|_| ()));
    assert_eq!(error.kind(), borsh::io::ErrorKind::UnexpectedEof);
}

/// A frame with bytes after its message is rejected as non-canonical.
///
/// [`recv_msg`] decodes the frame body exactly: one message, no remainder.
/// Trailing garbage must surface as typed `InvalidData` rather than being
/// silently dropped, which would let two frames name one message.
#[test]
fn trailing_frame_bytes_are_rejected() {
    let mut body = handshake_bytes();
    body.push(0xFF);

    let error = io_error(recv::<message::Handshake>(&frame(&body)).map(|_| ()));
    assert_eq!(error.kind(), borsh::io::ErrorKind::InvalidData);
}

/// Reading one message consumes exactly its frame, leaving later bytes
/// untouched.
///
/// The exact-read framing contract is what lets one connection host
/// back-to-back sessions: after a clean message the next session's bytes
/// (here a fake preamble) must still sit unread in the transport.
#[test]
fn bytes_after_the_frame_stay_untouched() {
    let mut bytes = frame(&handshake_bytes());
    bytes.extend_from_slice(b"RUMORS");

    pollster::block_on(async {
        let mut cursor = &bytes[..];
        let mut reader = FrameRead::new(&mut cursor);
        recv_msg::<message::Handshake, _>(&mut reader)
            .await
            .expect("a canonical greeting frame decodes");
        assert_eq!(cursor, b"RUMORS", "the next session's bytes were consumed");
    });
}

proptest! {
    /// Arbitrary frame bodies decode to a message or the typed [`Error::Io`]
    /// for every V1 message type — never a panic.
    ///
    /// The frame is honestly sized around an arbitrary body, so the fuzz
    /// lands on the borsh body decoders (the version bit codec, the channel
    /// order checks, the typed node reconstruction) rather than on the
    /// allocator via a lied length header — the header lies are pinned
    /// deterministically above. Decoding a slice always terminates, so a
    /// completed run is also the no-hang witness.
    #[test]
    fn arbitrary_frame_bodies_never_panic(body in vec(any::<u8>(), 0..64)) {
        let framed = frame(&body);
        prop_assert!(matches!(
            recv::<message::Handshake>(&framed).map(|_| ()),
            Ok(()) | Err(Error::Io(_)),
        ));
        prop_assert!(matches!(
            recv::<message::Initiate>(&framed).map(|_| ()),
            Ok(()) | Err(Error::Io(_)),
        ));
        prop_assert!(matches!(
            recv::<message::Opening>(&framed).map(|_| ()),
            Ok(()) | Err(Error::Io(_)),
        ));
        prop_assert!(matches!(
            recv::<message::Exchange<u64, UnderRoot>>(&framed).map(|_| ()),
            Ok(()) | Err(Error::Io(_)),
        ));
        prop_assert!(matches!(
            recv::<message::Closing<u64>>(&framed).map(|_| ()),
            Ok(()) | Err(Error::Io(_)),
        ));
        prop_assert!(matches!(
            recv::<message::Complete<u64>>(&framed).map(|_| ()),
            Ok(()) | Err(Error::Io(_)),
        ));
    }
}
