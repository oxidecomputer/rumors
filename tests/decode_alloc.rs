//! Allocator metering for the wire decoders' framed-payload reads.
//!
//! A counting global allocator prices each decode in bytes requested from
//! the allocator, holding decoder memory against the bytes a peer actually
//! delivered. The two metered entries are the framing layer's frame read
//! and the streaming codec's supply read — the two places a peer-declared
//! `u32` length stands ahead of a variable body. The counters are
//! process-global, so a mutex serializes every metered region; the suite
//! is correct under any test runner's threading.

use std::alloc::System;
use std::sync::Mutex;

use rumors::error::{CodecDecodeErrorKind, FramePart};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, Stats, StatsAlloc};

#[global_allocator]
static ALLOCATOR: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

/// Serializes metered regions: the allocator counters are process-global,
/// so concurrent tests would attribute each other's traffic.
static METER_LOCK: Mutex<()> = Mutex::new(());

/// The payload length a corrupt stream or conformance-buggy peer declares
/// while delivering no or few payload bytes behind it.
const DECLARED_LEN: usize = 256 * 1024 * 1024;

/// The payload length of the honest fully-delivered frames the liveness
/// floors meter.
const HONEST_LEN: usize = 8 * 1024 * 1024;

/// A non-power-of-two honest payload length.
///
/// A growth policy whose capacity overshoots the declared length
/// (amortized doubling lands on the next power of two, near twice this)
/// breaches the honest ceiling here, where a power-of-two length would
/// mask it.
const HONEST_ODD_LEN: usize = 8 * 1024 * 1024 + 37;

/// Allocator-noise allowance for a metered decode beyond its derived
/// bound.
///
/// Covers waker and error-construction incidentals, sub-KiB in total:
/// orders of magnitude below every bound it pads, so it cannot mask a
/// declared-length prepay or a capacity overshoot.
const METER_SLACK: usize = 1024;

/// Allocation-event allowance beyond the derived growth schedule: waker
/// and run-validation incidentals. Far below the events a per-granule
/// reservation policy would produce (payload length over chunk length).
const EVENT_SLACK: usize = 16;

/// Allocator counter movement while `f` runs, serialized by the meter
/// lock.
///
/// `bytes_allocated` counts fresh allocation sizes plus reallocation
/// growth deltas, so an up-front `with_capacity` of a declared length and
/// incremental growth to the same capacity read identically.
fn metered<T>(f: impl FnOnce() -> T) -> (Stats, T) {
    let _guard = METER_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let region = Region::new(ALLOCATOR);
    let value = f();
    (region.change(), value)
}

/// Reservation events of the doubling-clamped growth schedule for a fully
/// delivered `len`-byte payload: one initial granule, then one doubling
/// per factor of two between the granule and `len`.
fn growth_events(len: usize) -> usize {
    let chunk = rumors::testing::frame_payload_chunk_len();
    usize::try_from(len.div_ceil(chunk).next_power_of_two().trailing_zeros())
        .expect("a u32 bit index fits in usize")
        + 1
}

/// A framing-layer frame declaring `declared` payload bytes, delivering
/// `payload` behind the header.
fn framed(declared: usize, payload: &[u8]) -> Vec<u8> {
    let mut bytes = u32::try_from(declared)
        .expect("declared lengths in this suite fit the u32 header")
        .to_be_bytes()
        .to_vec();
    bytes.extend_from_slice(payload);
    bytes
}

/// A streaming supply frame declaring `declared` run bytes, delivering
/// `body` behind the signal and length header.
fn supply_frame(declared: usize, body: &[u8]) -> Vec<u8> {
    let mut bytes = vec![rumors::testing::supply_signal_byte()];
    bytes.extend_from_slice(
        &u32::try_from(declared)
            .expect("declared lengths in this suite fit the u32 header")
            .to_be_bytes(),
    );
    bytes.extend_from_slice(body);
    bytes
}

/// A run body of `len` total bytes that passes run-record framing: one
/// record whose header claims the rest of the body. Record contents decode
/// lazily, so arbitrary bytes suffice for the meter.
fn run_body(len: usize) -> Vec<u8> {
    let record_len = len - rumors::testing::run_record_header_len();
    let mut body = u32::try_from(record_len)
        .expect("record lengths in this suite fit the u32 header")
        .to_be_bytes()
        .to_vec();
    body.extend((0..record_len).map(|i| i as u8));
    body
}

/// Ceiling: a frame declaring 256 MiB with zero delivered payload bytes
/// requests at most one payload chunk (plus sub-KiB noise).
///
/// Decoder memory tracks bytes actually received, never the peer-declared
/// length: with nothing delivered, at most one granule is reserved.
#[test]
fn framing_zero_delivered_costs_at_most_one_chunk() {
    let bytes = framed(DECLARED_LEN, &[]);
    let (change, result) =
        metered(|| pollster::block_on(rumors::testing::read_framed_payload(&bytes[..])));
    let error = result.expect_err("a frame with nothing behind its header cannot complete");
    assert_eq!(error.kind(), std::io::ErrorKind::UnexpectedEof);
    let ceiling = rumors::testing::frame_payload_chunk_len() + METER_SLACK;
    assert!(
        change.bytes_allocated <= ceiling,
        "framing read requested {} bytes with zero delivered; \
         the ceiling is one payload chunk plus slack, {ceiling}",
        change.bytes_allocated
    );
}

/// Ceiling: a supply frame declaring a 256 MiB run with zero delivered
/// body bytes requests at most one payload chunk (plus sub-KiB noise),
/// and the failure classifies as a truncated `SupplyRun`.
///
/// The typed assertion keeps this ceiling non-vacuous: a decoder that
/// started rejecting the frame before its body read would no longer
/// exercise the allocation path this test prices.
#[test]
fn supply_zero_delivered_costs_at_most_one_chunk() {
    let bytes = supply_frame(DECLARED_LEN, &[]);
    let (change, result) =
        metered(|| pollster::block_on(rumors::testing::decode_supply_frame(&bytes[..])));
    let error = result.expect_err("a supply frame with nothing behind its header cannot complete");
    assert!(
        matches!(
            error.kind,
            CodecDecodeErrorKind::Truncated {
                missing: FramePart::SupplyRun,
                ..
            }
        ),
        "the failure classifies as a truncated supply run, got: {error}"
    );
    let ceiling = rumors::testing::frame_payload_chunk_len() + METER_SLACK;
    assert!(
        change.bytes_allocated <= ceiling,
        "supply read requested {} bytes with zero delivered; \
         the ceiling is one payload chunk plus slack, {ceiling}",
        change.bytes_allocated
    );
}

/// Ceiling: a frame declaring 256 MiB with only k delivered payload bytes
/// requests at most 2k plus one chunk (plus sub-KiB noise).
///
/// Allocation must track receipt at every prefix, not merely at zero: a
/// decoder that consumes one byte and then allocates the declared length
/// passes the zero-delivered ceiling and the floors, and reds here.
#[test]
fn framing_partial_delivery_costs_receipt_proportional() {
    let chunk = rumors::testing::frame_payload_chunk_len();
    let delivered_len = 2 * chunk + 37;
    let delivered: Vec<u8> = (0..delivered_len).map(|i| i as u8).collect();
    let bytes = framed(DECLARED_LEN, &delivered);
    let (change, result) =
        metered(|| pollster::block_on(rumors::testing::read_framed_payload(&bytes[..])));
    let error = result.expect_err("a partially delivered frame cannot complete");
    assert_eq!(error.kind(), std::io::ErrorKind::UnexpectedEof);
    let ceiling = 2 * delivered_len + chunk + METER_SLACK;
    assert!(
        change.bytes_allocated <= ceiling,
        "framing read requested {} bytes after {delivered_len} delivered; \
         the receipt-proportional ceiling is {ceiling}",
        change.bytes_allocated
    );
}

/// Liveness floor: an honest fully-delivered frame of N payload bytes
/// meters at least N requested bytes and decodes byte-identically, so the
/// meter provably counts the framing payload path it prices.
#[test]
fn framing_full_delivery_meters_at_least_payload() {
    let payload: Vec<u8> = (0..HONEST_LEN).map(|i| i as u8).collect();
    let bytes = framed(HONEST_LEN, &payload);
    let (change, result) =
        metered(|| pollster::block_on(rumors::testing::read_framed_payload(&bytes[..])));
    assert_eq!(
        result.expect("a fully delivered frame decodes"),
        payload,
        "the decoded payload is byte-identical to what the peer sent"
    );
    assert!(
        change.bytes_allocated >= HONEST_LEN,
        "framing read requested {} bytes for a {HONEST_LEN}-byte payload; \
         the meter must count at least the payload it materialized",
        change.bytes_allocated
    );
}

/// Liveness floor: an honest fully-delivered supply run of N body bytes
/// meters at least N requested bytes, so the meter provably counts the
/// codec's supply body path it prices.
#[test]
fn supply_full_delivery_meters_at_least_payload() {
    let body = run_body(HONEST_LEN);
    let bytes = supply_frame(HONEST_LEN, &body);
    let (change, result) =
        metered(|| pollster::block_on(rumors::testing::decode_supply_frame(&bytes[..])));
    result.expect("a fully delivered, well-framed run decodes");
    assert!(
        change.bytes_allocated >= HONEST_LEN,
        "supply read requested {} bytes for a {HONEST_LEN}-byte body; \
         the meter must count at least the body it materialized",
        change.bytes_allocated
    );
}

/// Ceilings: an honest fully-delivered frame of non-power-of-two length N
/// requests at most N plus one chunk in bytes, within a logarithmic
/// budget of allocation events, and still meters the >= N floor.
///
/// The byte ceiling fails a growth policy whose capacity overshoots the
/// declared length (doubling to the next power of two reads near 2N
/// here); the event ceiling fails a per-granule reservation policy, whose
/// event count is N over the chunk length while its netted byte reading
/// stays N.
#[test]
fn framing_full_delivery_costs_at_most_payload_plus_chunk() {
    let payload: Vec<u8> = (0..HONEST_ODD_LEN).map(|i| i as u8).collect();
    let bytes = framed(HONEST_ODD_LEN, &payload);
    let (change, result) =
        metered(|| pollster::block_on(rumors::testing::read_framed_payload(&bytes[..])));
    assert_eq!(
        result.expect("a fully delivered frame decodes"),
        payload,
        "the decoded payload is byte-identical to what the peer sent"
    );
    let chunk = rumors::testing::frame_payload_chunk_len();
    let byte_ceiling = HONEST_ODD_LEN + chunk + METER_SLACK;
    assert!(
        change.bytes_allocated >= HONEST_ODD_LEN && change.bytes_allocated <= byte_ceiling,
        "framing read requested {} bytes for a {HONEST_ODD_LEN}-byte payload; \
         the honest band is [{HONEST_ODD_LEN}, {byte_ceiling}]",
        change.bytes_allocated
    );
    let events = change.allocations + change.reallocations;
    let event_ceiling = growth_events(HONEST_ODD_LEN) + EVENT_SLACK;
    assert!(
        events <= event_ceiling,
        "framing read performed {events} allocation events for a \
         {HONEST_ODD_LEN}-byte payload; the doubling-schedule ceiling is {event_ceiling}"
    );
}

/// Ceiling: an honest fully-delivered supply run of non-power-of-two
/// length N requests at most N plus one chunk in bytes, and still meters
/// the >= N floor.
///
/// The supply path shares the framing growth policy; this pins that its
/// run buffer also reaches the adapter without capacity overshoot.
#[test]
fn supply_full_delivery_costs_at_most_payload_plus_chunk() {
    let body = run_body(HONEST_ODD_LEN);
    let bytes = supply_frame(HONEST_ODD_LEN, &body);
    let (change, result) =
        metered(|| pollster::block_on(rumors::testing::decode_supply_frame(&bytes[..])));
    result.expect("a fully delivered, well-framed run decodes");
    let byte_ceiling = HONEST_ODD_LEN + rumors::testing::frame_payload_chunk_len() + METER_SLACK;
    assert!(
        change.bytes_allocated >= HONEST_ODD_LEN && change.bytes_allocated <= byte_ceiling,
        "supply read requested {} bytes for a {HONEST_ODD_LEN}-byte body; \
         the honest band is [{HONEST_ODD_LEN}, {byte_ceiling}]",
        change.bytes_allocated
    );
}
