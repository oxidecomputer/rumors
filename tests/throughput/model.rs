//! The link model's own timing and delivery, against values computed by
//! hand and against the paused-clock latency link.
//!
//! The session suites read their verdicts from [`Link`]'s timing, so these
//! tests pin that timing on traffic simple enough to compute exactly: a
//! one-way transfer, a chain of request-response exchanges, a transfer
//! whose reader pauses, and arbitrary concurrent traffic for delivery. The
//! last test checks the model's latency-only link against an independent
//! implementation of the same quantity on Tokio's paused clock.

use std::future::poll_fn;
use std::task::Poll;
use std::time::Duration;

use proptest::prelude::*;
use rumors_testkit::bench::latency;
use rumors_testkit::bench::link::{End, Link, LinkConfig, SEGMENT_BYTES, Side, drive};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::fixture::{self, Case, Shape};

/// Writes `data` in chunks of at most `chunk` bytes, then shuts down.
async fn write_in_chunks(mut write: impl AsyncWrite + Unpin, data: &[u8], chunk: usize) {
    for piece in data.chunks(chunk) {
        write
            .write_all(piece)
            .await
            .expect("the link accepts writes");
    }
    write.shutdown().await.expect("the link shuts down");
}

/// Reads to end of stream in reads of at most `buffer` bytes.
async fn read_in_chunks(mut read: impl AsyncRead + Unpin, buffer: usize) -> Vec<u8> {
    let mut received = Vec::new();
    let mut scratch = vec![0; buffer];
    loop {
        let count = read.read(&mut scratch).await.expect("the link reads");
        if count == 0 {
            return received;
        }
        received.extend_from_slice(&scratch[..count]);
    }
}

/// Writes `sent` through `end` in chunks of at most `chunk` bytes while
/// reading what the peer sends in reads of at most `buffer` bytes, and
/// returns what it read.
async fn write_and_read(end: End, sent: &[u8], chunk: usize, buffer: usize) -> Vec<u8> {
    let ((), received) = tokio::join!(
        write_in_chunks(end.write, sent, chunk),
        read_in_chunks(end.read, buffer),
    );
    received
}

/// Waits until `link`'s clock reaches `resume_at`, then reads to end of
/// stream, and returns what it read with the time of its first read.
///
/// The wait registers no waker: it relies on a sibling future of the same
/// task, joined with it, being woken while the clock runs.
async fn read_after(read: impl AsyncRead + Unpin, link: Link, resume_at: u64) -> (Vec<u8>, u64) {
    poll_fn(|_| {
        if link.now() >= resume_at {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    })
    .await;
    let resumed = link.now();
    (read_in_chunks(read, 1 << 16).await, resumed)
}

/// Generates a link: a bandwidth-delay product, an unsent limit up to twice
/// it, and whether it serializes.
fn arb_link() -> impl Strategy<Value = LinkConfig> {
    (1u64..200_000, 0u64..=2, any::<bool>()).prop_map(|(half, limit_products, latency_only)| {
        let config = LinkConfig::with_bdp(2 * half).with_unsent_limit(limit_products * 2 * half);
        if latency_only {
            config.latency_only()
        } else {
            config
        }
    })
}

proptest! {
    /// A writer that hands bytes over as fast as the link admits them, to a
    /// reader that takes them as they arrive, keeps the wire busy from the
    /// first byte to the last.
    ///
    /// `D` bytes finish arriving at exactly `D + delay`, or at `delay` on a
    /// latency-only link, however the writes and reads are sized. The
    /// in-flight limit therefore never throttles a back-to-back sender,
    /// and neither the unsent queue nor the in-flight bytes exceed their
    /// stated bounds.
    #[test]
    fn back_to_back_transfer_keeps_the_wire_busy(
        config in arb_link(),
        bytes in 1usize..300_000,
        chunk in 1usize..20_000,
        buffer in 1usize..20_000,
    ) {
        let data: Vec<u8> = (0..bytes).map(|index| index as u8).collect();
        let link = Link::new(config);
        let (a, b) = (link.end(Side::A), link.end(Side::B));
        let (_, received, report) = drive(
            &link,
            write_in_chunks(a.write, &data, chunk),
            read_in_chunks(b.read, buffer),
        );
        prop_assert_eq!(received, data);
        let serialization = if config.serializes() { bytes as u64 } else { 0 };
        prop_assert_eq!(report.completed[Side::B.index()], serialization + config.one_way_delay());
        let sent = report.sent_by(Side::A);
        if serialization > 0 {
            prop_assert!(sent.max_unsent() <= config.unsent_limit() + SEGMENT_BYTES);
            prop_assert!(sent.max_in_flight <= config.bdp() + 2 * SEGMENT_BYTES);
            prop_assert_eq!(sent.busy_between(0, sent.last_serialized()), bytes as u64);
        }
    }

    /// `k` request-response exchanges, each answered only once the whole
    /// request has arrived, cost exactly `k · (request + response +
    /// round trip)` byte-times: each message serializes, then crosses.
    #[test]
    fn exchanges_cost_serialization_and_round_trips(
        half in 1u64..50_000,
        request in 1usize..20_000,
        response in 1usize..20_000,
        exchanges in 1u64..6,
    ) {
        let config = LinkConfig::with_bdp(2 * half);
        let link = Link::new(config);
        let (mut a, mut b) = (link.end(Side::A), link.end(Side::B));
        let asker = async {
            let mut reply = vec![0; response];
            for _ in 0..exchanges {
                a.write.write_all(&vec![1; request]).await.expect("the link accepts writes");
                a.read.read_exact(&mut reply).await.expect("the reply arrives");
            }
        };
        let answerer = async {
            let mut asked = vec![0; request];
            for _ in 0..exchanges {
                b.read.read_exact(&mut asked).await.expect("the request arrives");
                b.write.write_all(&vec![2; response]).await.expect("the link accepts writes");
            }
        };
        let ((), (), report) = drive(&link, asker, answerer);
        let exchange = request as u64 + response as u64 + config.rtt();
        prop_assert_eq!(report.completed[Side::A.index()], exchanges * exchange);
    }

    /// A reader that stops taking bytes stops a back-to-back sender at
    /// exactly one bandwidth-delay product plus two segments in flight.
    ///
    /// The sender starts again exactly one one-way delay after the reader
    /// takes them, when their acknowledgement arrives.
    ///
    /// The reader pauses until well after the sender has stalled and every
    /// byte in flight has arrived, then reads everything at once. Its side
    /// meanwhile sends a stream of its own, which the sender's side reads
    /// as it arrives: those writes' wakes poll the pausing reader, so the
    /// clock keeps moving while it pauses. Every byte arrives intact in
    /// both directions.
    #[test]
    fn a_pausing_reader_stops_the_sender_at_the_in_flight_limit(
        half in 1_000u64..50_000,
        limit_products in 0u64..=2,
        chunk in 1usize..20_000,
    ) {
        let config = LinkConfig::with_bdp(2 * half).with_unsent_limit(limit_products * 2 * half);
        let in_flight_limit = config.bdp() + 2 * SEGMENT_BYTES;
        let unsent_capacity = config.unsent_limit() + SEGMENT_BYTES;
        // The stall begins once the limit has departed, about
        // `in_flight_limit` byte-times in, and its bytes have all arrived a
        // delay later; the reader resumes well after both.
        let resume_at = 2 * (in_flight_limit + config.one_way_delay());
        let data: Vec<u8> = (0..4 * in_flight_limit).map(|index| index as u8).collect();
        // The return stream outlasts the pause: its writer finishes once
        // everything but its unsent queue and the bytes in flight has
        // serialized.
        let returned: Vec<u8> = (0..resume_at + 2 * in_flight_limit + unsent_capacity)
            .map(|index| (index * 3) as u8)
            .collect();
        let link = Link::new(config);
        let (a, b) = (link.end(Side::A), link.end(Side::B));
        let (to_a, ((), (to_b, resumed)), report) = drive(
            &link,
            write_and_read(a, &data, chunk, chunk),
            async {
                tokio::join!(
                    write_in_chunks(b.write, &returned, chunk),
                    read_after(b.read, link.clone(), resume_at),
                )
            },
        );
        prop_assert_eq!(&to_b, &data);
        prop_assert_eq!(&to_a, &returned);
        let sent = report.sent_by(Side::A);
        prop_assert_eq!(sent.max_in_flight, in_flight_limit);
        let before: u64 = sent
            .segments
            .iter()
            .filter(|segment| segment.start < resumed)
            .map(|segment| segment.bytes)
            .sum();
        prop_assert_eq!(before, in_flight_limit, "the sender stops at the limit");
        let restart = sent.segments.iter().find(|segment| segment.start >= resumed);
        prop_assert_eq!(
            restart.map(|segment| segment.start),
            Some(resumed + config.one_way_delay()),
            "the sender starts again when the read's acknowledgement arrives",
        );
    }

    /// Bytes written concurrently in both directions, in any write sizes,
    /// reach the other side intact and in order, whatever the reads' sizes
    /// and the link's parameters.
    #[test]
    fn concurrent_traffic_arrives_intact(
        config in arb_link(),
        lengths in [0usize..100_000, 0usize..100_000],
        chunks in [1usize..10_000, 1usize..10_000],
        buffers in [1usize..10_000, 1usize..10_000],
    ) {
        let data = lengths.map(|length| (0..length).map(|index| (index * 7) as u8).collect::<Vec<u8>>());
        let link = Link::new(config);
        let (a, b) = (link.end(Side::A), link.end(Side::B));
        let (to_a, to_b, _) = drive(
            &link,
            write_and_read(a, &data[0], chunks[0], buffers[0]),
            write_and_read(b, &data[1], chunks[1], buffers[1]),
        );
        prop_assert_eq!(&to_b, &data[0]);
        prop_assert_eq!(&to_a, &data[1]);
    }
}

/// A divergent session's critical path on the latency-only link, in
/// one-way hops, equals the hops the paused-clock latency link measures for
/// the same pair: two implementations of one quantity agree.
///
/// Both runs reconcile the same deterministic pair, 500 disjoint messages a
/// side with an unlimited budget, so nothing but each link's notion of a
/// hop separates them.
#[test]
fn latency_only_hops_match_the_paused_clock() {
    const MESSAGES: u64 = 500;
    /// The latency link's one-way delay: whole milliseconds, its timer's grain.
    const DELAY: Duration = Duration::from_millis(10);
    /// The latency link's bytes in flight per direction: far above the
    /// session's transfer, so only delay shapes its timing.
    const CAPACITY: usize = 64 << 20;

    let shape = Shape::symmetric(MESSAGES);
    let modelled = fixture::run(Case {
        shape,
        budget: usize::MAX,
        link: LinkConfig::with_bdp(2_000).latency_only(),
    });
    let paused = latency::session_hops(CAPACITY, DELAY, fixture::pair(shape, usize::MAX));
    assert_eq!(modelled.hops(), u64::from(paused));
}
