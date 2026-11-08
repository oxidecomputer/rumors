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

/// An in-memory reader yielding at most a scheduled number of bytes per read
/// call, cycling its schedule, so every partial-read seam of the chunked
/// payload loop is exercised.
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

    /// The bytes no read has taken yet.
    fn unread(&self) -> &[u8] {
        &self.data[self.at..]
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

    /// Resuming a payload read behind a prefix is observably identical to
    /// reading the whole payload from its start, whatever spare capacity
    /// the prefix's buffer carries and however the rest is chunked.
    ///
    /// The reference, `read_payload`, is `resume_payload` from an empty
    /// prefix, so the comparison alone would be a self-check; the
    /// absolute clauses carry the test: full delivery recovers exactly
    /// the payload and leaves the following bytes unread on both sides,
    /// and any truncation surfaces as `UnexpectedEof` on both.
    #[test]
    fn resumed_read_matches_whole_read(
        (len, prefix_len) in (0usize..=2 * PAYLOAD_CHUNK_LEN + 130)
            .prop_flat_map(|len| (Just(len), 0..=len)),
        spare in 0usize..=PAYLOAD_CHUNK_LEN + 7,
        seed in any::<u8>(),
        schedule in prop::collection::vec(1usize..=PAYLOAD_CHUNK_LEN + 7, 1..8),
        cut in proptest::option::of(0f64..1f64),
    ) {
        let payload = pattern(len, seed);
        let trailing = b"next-frame-bytes";
        let delivered = match cut {
            None => len,
            Some(fraction) => prefix_len + (fraction * (len - prefix_len) as f64) as usize,
        };
        let mut transcript = payload[..delivered].to_vec();
        if delivered == len {
            transcript.extend_from_slice(trailing);
        }

        let mut prefix = Vec::with_capacity(prefix_len + spare);
        prefix.extend_from_slice(&payload[..prefix_len]);
        let mut chunked = ChunkedRead::new(transcript[prefix_len..].to_vec(), schedule);
        let via_resume = pollster::block_on(resume_payload(&mut chunked, prefix, len));
        let mut cursor: &[u8] = &transcript;
        let via_whole = pollster::block_on(read_payload(&mut cursor, len));

        match (via_resume, via_whole) {
            (Ok(resumed), Ok(whole)) => {
                prop_assert_eq!(&resumed, &whole);
                prop_assert_eq!(resumed, payload);
                prop_assert_eq!(chunked.unread(), trailing.as_slice());
                prop_assert_eq!(cursor, trailing.as_slice());
            }
            (Err(resumed), Err(whole)) => {
                prop_assert_eq!(resumed.kind(), std::io::ErrorKind::UnexpectedEof);
                prop_assert_eq!(whole.kind(), std::io::ErrorKind::UnexpectedEof);
            }
            (resumed, whole) => {
                prop_assert!(false, "readers disagree: {:?} vs {:?}", resumed, whole);
            }
        }
    }
}

/// Truncation cuts landing one byte short of, exactly on, and one byte past
/// each payload chunk boundary all surface as `UnexpectedEof`: chunking never
/// changes how a mid-payload close classifies.
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

/// A payload read consumes exactly the declared byte count: the following bytes
/// stay unconsumed in the reader, whatever the payload chunking.
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

/// A resumed payload read consumes exactly the bytes still owed: spare capacity
/// on the caller's buffer never draws in the following bytes, which stay
/// unconsumed in the reader.
#[test]
fn resumed_read_never_consumes_beyond_the_declared_length() {
    let len = 9;
    let mut transcript = pattern(len, 5);
    let trailing = *b"next-frame-bytes";
    transcript.extend_from_slice(&trailing);
    let mut prefix = Vec::with_capacity(64);
    prefix.extend_from_slice(&transcript[..3]);

    let mut cursor: &[u8] = &transcript[3..];
    let decoded = pollster::block_on(resume_payload(&mut cursor, prefix, len)).unwrap();
    assert_eq!(decoded, &transcript[..len]);
    assert_eq!(cursor, trailing.as_slice());
}

/// A prefix longer than the declared length is a caller error: reported as
/// `InvalidData`, with nothing read from the transport.
#[test]
fn a_prefix_longer_than_the_declared_length_is_invalid_data() {
    let mut cursor: &[u8] = b"unused";
    let error = pollster::block_on(resume_payload(&mut cursor, vec![0u8; 12], 9)).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert_eq!(cursor, b"unused");
}

/// The most a `len`-byte [`read_payload`] holds at once, by its growth policy.
///
/// Its first reservation is `len` itself when that is at most one
/// [`PAYLOAD_CHUNK_LEN`] granule, and otherwise one granule, doubled each time
/// it fills and clamped to `len`, so its peak is the moment its last
/// reallocation holds the `len`-byte block beside the one it outgrew.
fn payload_read_peak(len: usize) -> usize {
    if len <= PAYLOAD_CHUNK_LEN {
        return len;
    }
    let mut outgrown = PAYLOAD_CHUNK_LEN;
    while outgrown * 2 < len {
        outgrown *= 2;
    }
    len + outgrown
}

/// The granules within which [`payload_read_peak`]'s tests probe lengths.
const PEAK_GRANULES: usize = 8;

/// Read a whole `len`-byte payload through [`read_payload`], polled once, and
/// require it to hold exactly [`payload_read_peak`] at its peak: its length
/// alone within one granule, and less than twice its length beyond.
///
/// # Panics
///
/// Panics if the read does not complete in one poll, returns other bytes, or
/// peaks elsewhere.
fn assert_payload_read_peak(len: usize) {
    let payload = pattern(len, 7);
    let mut source = payload.as_slice();
    let mut reading = std::pin::pin!(read_payload(&mut source, len));
    let mut cx = Context::from_waker(std::task::Waker::noop());
    let (stats, polled) = alloc_meter::measure(|| reading.as_mut().poll(&mut cx));
    let Poll::Ready(read) = polled else {
        panic!("an in-memory reader never waits");
    };
    let read = read.expect("the whole payload is delivered");
    assert_eq!(read.as_slice(), payload.as_slice());
    assert_eq!(
        stats.peak_bytes,
        payload_read_peak(len),
        "a {len}-byte read"
    );
    assert!(
        len <= PAYLOAD_CHUNK_LEN || stats.peak_bytes < 2 * len,
        "a {len}-byte read peaked at {} B, twice its length or more",
        stats.peak_bytes
    );
}

/// A whole payload read peaks as its growth policy says at every length within
/// a byte of a granule boundary, up to [`PEAK_GRANULES`] granules.
///
/// At one byte short of each boundary, exactly on it, and one byte past it, the
/// read holds exactly its length when that is at most one granule, and
/// otherwise its length beside the buffer it last outgrew.
///
/// The boundaries are where the growth policy changes what it reserves, so the
/// test exhausts them;
/// [`a_payload_read_holds_its_length_beside_the_buffer_it_outgrew`] samples the
/// lengths between.
#[test]
fn a_payload_read_peaks_by_its_policy_at_every_granule_boundary() {
    for granules in 1..=PEAK_GRANULES {
        for len in [-1isize, 0, 1]
            .map(|offset| (granules * PAYLOAD_CHUNK_LEN).saturating_add_signed(offset))
        {
            assert_payload_read_peak(len);
        }
    }
}

proptest! {
    /// A whole payload read holds exactly its length at once when that is
    /// at most one granule, and otherwise its length beside the buffer it
    /// last outgrew, which is less than its length: never as much as twice
    /// its length.
    ///
    /// The greeting and the party hand-off read their bodies this way,
    /// before anything bounds their declared lengths, and no part of the
    /// memory budget sets their reads aside. This bounds what each holds:
    /// a body of at most [`PAYLOAD_CHUNK_LEN`] costs one block of its own
    /// length, and only a larger one, such as a greeting whose version has
    /// grown past about a granule, pays the doubling's transient. The
    /// lengths are drawn within [`PEAK_GRANULES`] granules, and the meter
    /// counts both blocks of each reallocation, so the peak is exact;
    /// [`a_payload_read_peaks_by_its_policy_at_every_granule_boundary`]
    /// exhausts the boundaries.
    #[test]
    fn a_payload_read_holds_its_length_beside_the_buffer_it_outgrew(
        len in 0..=PEAK_GRANULES * PAYLOAD_CHUNK_LEN,
    ) {
        assert_payload_read_peak(len);
    }
}
