//! Ingress validation of the V2 session epilogue marker.
//!
//! The epilogue reads exactly one peer-controlled byte from the control
//! stream after all session work; it is the last wire ingress of every V2
//! session. This suite exhausts that byte space and its truncation directly
//! against [`epilogue`]: every non-marker byte is a typed protocol
//! violation, an honest cut is a typed EOF, both as the distinguished
//! post-commit [`Error::Epilogue`] — never a panic and never a hang — and a
//! clean exchange leaves the next session's bytes untouched. The end-to-end
//! commit-boundary consequences are pinned in `tests/lifecycle.rs` and
//! `src/tests.rs`.

use tokio::io::{duplex, split};

use super::{EPILOGUE_MARKER, epilogue};
use crate::Error;

/// Unwrap the sole error variant the epilogue can produce.
fn epilogue_error(result: Result<(), Error>) -> std::io::Error {
    match result {
        Err(Error::Epilogue(error)) => error,
        other => panic!("the epilogue fails as Error::Epilogue, got {other:?}"),
    }
}

/// Both sides exchange markers over a one-byte transport without deadlock.
///
/// Each side writes and flushes before its read resolves, so the exchange
/// completes even when the transport holds a single byte in flight; both
/// sides return `Ok`, the mutual completion certificate.
#[test]
fn concurrent_exchange_is_symmetric() {
    let (left_io, right_io) = duplex(1);
    let (mut left_read, mut left_write) = split(left_io);
    let (mut right_read, mut right_write) = split(right_io);

    let (left, right) = pollster::block_on(async {
        tokio::join!(
            epilogue(&mut left_read, &mut left_write),
            epilogue(&mut right_read, &mut right_write),
        )
    });
    left.expect("left epilogue completes");
    right.expect("right epilogue completes");
}

/// Marker decoding is exhaustive: exactly the one marker byte is accepted
/// and every other byte is a typed protocol violation.
///
/// A non-marker byte — a desynchronized peer's next preamble included —
/// must surface [`Error::Epilogue`] with `InvalidData`, distinguishing a
/// protocol violation from an honest wire cut; the marker itself completes
/// the session.
#[test]
fn marker_byte_space_is_exhaustive() {
    for byte in u8::MIN..=u8::MAX {
        let bytes = [byte];
        let mut reader = &bytes[..];
        let mut writer = tokio::io::sink();
        let result = pollster::block_on(epilogue(&mut reader, &mut writer));
        if byte == EPILOGUE_MARKER {
            result.expect("the marker byte completes the epilogue");
        } else {
            let error = epilogue_error(result);
            assert_eq!(
                error.kind(),
                std::io::ErrorKind::InvalidData,
                "byte {byte:#04x} must be a typed protocol violation",
            );
        }
    }
}

/// A peer that closes before its marker is a typed EOF, not a hang.
///
/// The honest wire cut must surface [`Error::Epilogue`] with
/// `UnexpectedEof` — the arm the two-generals residue lands on — kept
/// distinct from the `InvalidData` violation above so operators can tell a
/// dead link from a desynchronized peer.
#[test]
fn close_before_the_marker_is_a_typed_eof() {
    let mut reader: &[u8] = &[];
    let mut writer = tokio::io::sink();
    let result = pollster::block_on(epilogue(&mut reader, &mut writer));
    let error = epilogue_error(result);
    assert_eq!(error.kind(), std::io::ErrorKind::UnexpectedEof);
}

/// Reading the marker consumes exactly one byte, leaving later bytes
/// untouched.
///
/// A next session's preamble may already sit behind the marker on a reused
/// link; the epilogue must not slurp it. After a clean exchange the
/// following bytes remain unread in the transport.
#[test]
fn bytes_after_the_marker_stay_untouched() {
    let bytes = [EPILOGUE_MARKER, b'R', b'U'];
    let mut reader = &bytes[..];
    let mut writer = tokio::io::sink();
    pollster::block_on(epilogue(&mut reader, &mut writer)).expect("the marker completes");
    assert_eq!(reader, b"RU", "the next session's bytes were consumed");
}
