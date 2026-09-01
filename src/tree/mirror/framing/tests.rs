use std::pin::Pin;
use std::task::{Context, Poll};

use proptest::prelude::*;
use tokio::io::{AsyncRead, AsyncReadExt, ReadBuf};

use super::*;

/// A deterministic `len`-byte payload varying with `seed`, so byte-identity
/// assertions compare real content rather than zero fill.
fn pattern(len: usize, seed: u8) -> Vec<u8> {
    (0..len)
        .map(|i| (i as u8).wrapping_mul(31).wrapping_add(seed))
        .collect()
}

/// An in-memory reader yielding at most a scheduled number of bytes per
/// read call, cycling its schedule, so every partial-read seam of the
/// chunked payload loop is exercised.
struct ChunkedRead {
    data: Vec<u8>,
    at: usize,
    schedule: Vec<usize>,
    step: usize,
}

impl ChunkedRead {
    fn new(data: Vec<u8>, schedule: Vec<usize>) -> Self {
        Self {
            data,
            at: 0,
            schedule,
            step: 0,
        }
    }
}

impl AsyncRead for ChunkedRead {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = &mut *self;
        if this.at >= this.data.len() {
            return Poll::Ready(Ok(()));
        }
        let cap = this.schedule[this.step % this.schedule.len()].max(1);
        this.step += 1;
        let granted = cap.min(buf.remaining()).min(this.data.len() - this.at);
        buf.put_slice(&this.data[this.at..this.at + granted]);
        this.at += granted;
        Poll::Ready(Ok(()))
    }
}

/// A whole-read reference: allocate the declared length up front, then one
/// exact read. The differential proptest holds the chunked reader to this
/// shape's observable behavior.
async fn whole_read_reference(mut bytes: &[u8], len: usize) -> std::io::Result<Vec<u8>> {
    let mut payload = vec![0u8; len];
    bytes.read_exact(&mut payload).await?;
    Ok(payload)
}

proptest! {
    /// The chunked payload read is observably identical to a whole-read
    /// reference across arbitrary read schedules and truncation points.
    ///
    /// Full delivery yields the byte-identical payload, and any truncation
    /// surfaces as `UnexpectedEof` on both.
    #[test]
    fn chunked_read_matches_whole_read_reference(
        len in 0usize..=2 * PAYLOAD_CHUNK_LEN + 130,
        seed in any::<u8>(),
        schedule in prop::collection::vec(1usize..=PAYLOAD_CHUNK_LEN + 7, 1..8),
        cut in proptest::option::of(0f64..1f64),
    ) {
        let payload = pattern(len, seed);
        let delivered = match cut {
            None => payload.len(),
            Some(fraction) => (fraction * payload.len() as f64) as usize,
        };
        let truncated = &payload[..delivered];

        let mut chunked = ChunkedRead::new(truncated.to_vec(), schedule);
        let via_chunks = pollster::block_on(read_payload(&mut chunked, len));
        let via_whole = pollster::block_on(whole_read_reference(truncated, len));

        match (via_chunks, via_whole) {
            (Ok(chunked), Ok(whole)) => {
                prop_assert_eq!(&chunked, &whole);
                prop_assert_eq!(chunked, payload);
            }
            (Err(chunked), Err(whole)) => {
                prop_assert_eq!(chunked.kind(), std::io::ErrorKind::UnexpectedEof);
                prop_assert_eq!(whole.kind(), std::io::ErrorKind::UnexpectedEof);
            }
            (chunked, whole) => {
                prop_assert!(false, "readers disagree: {:?} vs {:?}", chunked, whole);
            }
        }
    }
}

/// Truncation cuts landing one byte short of, exactly on, and one byte
/// past each payload chunk boundary all surface as `UnexpectedEof`:
/// chunking never changes how a mid-payload close classifies.
#[test]
fn truncation_at_chunk_boundaries_is_unexpected_eof() {
    let len = 2 * PAYLOAD_CHUNK_LEN + 5;
    let payload = pattern(len, 7);
    for delivered in chunk_boundary_cuts(len) {
        let mut truncated = &payload[..delivered];
        let error = pollster::block_on(read_payload(&mut truncated, len)).unwrap_err();
        assert_eq!(
            error.kind(),
            std::io::ErrorKind::UnexpectedEof,
            "cut after {delivered} delivered payload bytes"
        );
    }
}

/// A payload read consumes exactly the declared byte count: the following
/// bytes stay untouched in the transport, whatever the payload chunking.
#[test]
fn payload_read_never_consumes_beyond_the_declared_length() {
    let len = PAYLOAD_CHUNK_LEN + 3;
    let payload = pattern(len, 3);
    let trailing = *b"next-frame-bytes";
    let mut transcript = payload.clone();
    transcript.extend_from_slice(&trailing);

    let mut cursor: &[u8] = &transcript;
    let decoded = pollster::block_on(read_payload(&mut cursor, len)).unwrap();
    assert_eq!(decoded, payload);
    assert_eq!(cursor, trailing.as_slice());
}
