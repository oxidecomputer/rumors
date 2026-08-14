//! Allocator metering for the wire decoders' framed-payload reads.
//!
//! A counting global allocator prices each decode in bytes requested from
//! the allocator, holding decoder memory against the bytes a peer actually
//! delivered. The two metered entries are the framing layer's frame read
//! and the streaming codec's supply read — the two places a peer-declared
//! `u32` length stands ahead of a variable body. The counter is
//! process-global, so every test here must own its process (nextest runs
//! one test per process).

use std::alloc::System;

use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};

#[global_allocator]
static ALLOCATOR: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

/// The payload length a corrupt stream or conformance-buggy peer declares
/// while delivering no payload bytes behind it.
const DECLARED_LEN: usize = 256 * 1024 * 1024;

/// The payload length of the honest fully-delivered frames the liveness
/// floors meter.
const HONEST_LEN: usize = 8 * 1024 * 1024;

/// Bytes of the `u32` length header ahead of each leaf record in a supply
/// run body.
const RECORD_HEADER_LEN: usize = 4;

/// Allocator-noise allowance for a metered decode beyond the payload
/// chunk: error construction and other sub-KiB incidentals. Orders of
/// magnitude below the chunk bound, so it cannot mask a declared-length
/// prepay.
const METER_SLACK: usize = 1024;

/// Bytes requested from the allocator while `f` runs: fresh allocations
/// plus positive reallocation growth, so an up-front `with_capacity` of a
/// declared length and incremental growth both count.
fn requested_bytes<T>(f: impl FnOnce() -> T) -> (usize, T) {
    let region = Region::new(ALLOCATOR);
    let value = f();
    let change = region.change();
    let grown = usize::try_from(change.bytes_reallocated.max(0))
        .expect("a non-negative isize fits in usize");
    (change.bytes_allocated + grown, value)
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
    let record_len = len - RECORD_HEADER_LEN;
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
/// length: with nothing delivered, at most one chunk is pre-touched.
#[test]
fn framing_zero_delivered_costs_at_most_one_chunk() {
    let bytes = framed(DECLARED_LEN, &[]);
    let (requested, result) =
        requested_bytes(|| pollster::block_on(rumors::testing::read_framed_payload(&bytes[..])));
    let error = result.expect_err("a frame with nothing behind its header cannot complete");
    assert_eq!(error.kind(), std::io::ErrorKind::UnexpectedEof);
    let ceiling = rumors::testing::frame_payload_chunk_len() + METER_SLACK;
    assert!(
        requested <= ceiling,
        "framing read requested {requested} bytes with zero delivered; \
         the ceiling is one payload chunk plus slack, {ceiling}"
    );
}

/// Ceiling: a supply frame declaring a 256 MiB run with zero delivered
/// body bytes requests at most one payload chunk (plus sub-KiB noise).
///
/// Decoder memory tracks bytes actually received, never the peer-declared
/// length: with nothing delivered, at most one chunk is pre-touched.
#[test]
fn supply_zero_delivered_costs_at_most_one_chunk() {
    let bytes = supply_frame(DECLARED_LEN, &[]);
    let (requested, result) =
        requested_bytes(|| pollster::block_on(rumors::testing::decode_supply_frame(&bytes[..])));
    result.expect_err("a supply frame with nothing behind its header cannot complete");
    let ceiling = rumors::testing::frame_payload_chunk_len() + METER_SLACK;
    assert!(
        requested <= ceiling,
        "supply read requested {requested} bytes with zero delivered; \
         the ceiling is one payload chunk plus slack, {ceiling}"
    );
}

/// Liveness floor: an honest fully-delivered frame of N payload bytes
/// meters at least N requested bytes and decodes byte-identically, so the
/// meter provably counts the framing payload path it prices.
#[test]
fn framing_full_delivery_meters_at_least_payload() {
    let payload: Vec<u8> = (0..HONEST_LEN).map(|i| i as u8).collect();
    let bytes = framed(HONEST_LEN, &payload);
    let (requested, result) =
        requested_bytes(|| pollster::block_on(rumors::testing::read_framed_payload(&bytes[..])));
    assert_eq!(
        result.expect("a fully delivered frame decodes"),
        payload,
        "the decoded payload is byte-identical to what the peer sent"
    );
    assert!(
        requested >= HONEST_LEN,
        "framing read requested {requested} bytes for a {HONEST_LEN}-byte payload; \
         the meter must count at least the payload it materialized"
    );
}

/// Liveness floor: an honest fully-delivered supply run of N body bytes
/// meters at least N requested bytes, so the meter provably counts the
/// codec's supply body path it prices.
#[test]
fn supply_full_delivery_meters_at_least_payload() {
    let body = run_body(HONEST_LEN);
    let bytes = supply_frame(HONEST_LEN, &body);
    let (requested, result) =
        requested_bytes(|| pollster::block_on(rumors::testing::decode_supply_frame(&bytes[..])));
    result.expect("a fully delivered, well-framed run decodes");
    assert!(
        requested >= HONEST_LEN,
        "supply read requested {requested} bytes for a {HONEST_LEN}-byte body; \
         the meter must count at least the body it materialized"
    );
}
