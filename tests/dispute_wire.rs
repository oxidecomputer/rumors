//! Check the sizing model's wire-cost estimate against complete sessions.
//!
//! Count writes on both endpoints' control and data streams, divide by the
//! messages transferred, and subtract their encoded payload sizes. The
//! remainder includes hashes, versions, framing, and session setup, but no
//! underlying transport overhead. Seeded corpora make this reproducible.
//!
//! The smaller fixtures pin rounded-down means at several payload sizes.
//! The larger fixture checks the approximation after fixed and structural
//! costs have been spread across enough messages. These are measurements of
//! particular workloads, not a universal per-message cost: shared history,
//! tree shape, versions, and batching all affect how much metadata accompanies
//! each message.

mod common;

use bytes::Bytes;
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaChaRng;
use rumors::testing::{dispute_overhead_bytes, reference_wire_bytes};
use rumors::{Peer, Rumors};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::common::window::WindowChoice;

/// Shared history in the smaller calibration fixtures.
const COMMON: usize = 2_048;

/// New messages per side in the smaller calibration fixtures.
const DIVERGENT: usize = 8_192;

/// A byte-string payload that encodes to the 100-byte reference size.
const REFERENCE_PAYLOAD_LEN: usize = 98;

/// A byte-string payload that encodes to 64 bytes.
const MID_PAYLOAD_LEN: usize = 62;

/// CBOR's byte-string header width for lengths in `24..=255`: the major
/// type byte plus one length byte.
const CBOR_BSTR_HEADER_BYTES: usize = 2;

/// The reference record's encoded payload: [`REFERENCE_PAYLOAD_LEN`] bytes
/// behind CBOR's byte-string header.
const REFERENCE_ENCODED_PAYLOAD_BYTES: usize = CBOR_BSTR_HEADER_BYTES + REFERENCE_PAYLOAD_LEN;

/// A random `u64`'s CBOR encoding: the one-byte major-type header plus
/// eight value bytes (every seeded draw exceeds 2³², so the width is
/// deterministic for the minimal cell's corpus).
const U64_ENCODED_BYTES: usize = 9;

/// The small-record fixture's rounded mean falls one byte below the estimate.
/// Smaller records amortize their shared frame headers over more messages.
const MINIMAL_CELL_RESIDUAL: usize = 1;

/// Fork one network after shared history, then add distinct messages on each side.
fn diverged<T>(
    common: usize,
    divergent: usize,
    window: WindowChoice,
    mut make: impl FnMut(&mut ChaChaRng) -> T,
) -> (Rumors<T>, Rumors<T>)
where
    T: Serialize + DeserializeOwned + Eq + Send + Sync + Clone + 'static,
{
    let left = window.apply(Peer::seed()).into_rumors();
    let mut rng = ChaChaRng::seed_from_u64(0x0b05_2026_d15b_073e);
    let mut send = |rumors: &Rumors<T>, n: usize, rng: &mut ChaChaRng| {
        rumors.send_all((0..n).map(|_| make(rng))).unwrap();
    };
    send(&left, common, &mut rng);
    let right = common::wire::bootstrap_fork_with_window(&left, window);
    send(&left, divergent, &mut rng);
    send(&right, divergent, &mut rng);
    (left, right)
}

/// The end-to-end wire bytes of one session between `a` and `b`: control
/// stream plus every data stream, both directions, counted at the write
/// side of each end.
fn session_wire_bytes<T>(a: &Rumors<T>, b: &Rumors<T>) -> usize
where
    T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
{
    let (a_to_b, b_to_a) = common::count::session_wire_bytes(a, b);
    a_to_b + b_to_a
}

/// The smaller fixture's mean bytes per differing message, rounded down.
/// Both directions contribute writes; each new message crosses once.
fn implied_bytes_per_message<T>(make: impl FnMut(&mut ChaChaRng) -> T) -> usize
where
    T: Serialize + DeserializeOwned + Eq + Send + Sync + Clone + 'static,
{
    let (left, right) = diverged(COMMON, DIVERGENT, WindowChoice::Floor, make);
    let total = session_wire_bytes(&left, &right);
    assert_eq!(
        left.snapshot().hash(),
        right.snapshot().hash(),
        "the calibration session must actually converge",
    );
    assert_eq!(left.snapshot().len(), COMMON + 2 * DIVERGENT);
    total / (2 * DIVERGENT)
}

/// The reference fixture's rounded mean equals the calibrated wire cost.
#[test]
fn dispute_wire_bytes_is_the_reference_record_cost() {
    let mut make = |rng: &mut ChaChaRng| {
        let mut payload = vec![0u8; REFERENCE_PAYLOAD_LEN];
        rng.fill_bytes(&mut payload);
        Bytes::from(payload)
    };
    let implied = implied_bytes_per_message::<Bytes>(&mut make);
    let constant = reference_wire_bytes();
    eprintln!(
        "reference-record cell: implied {implied} B/message at {REFERENCE_ENCODED_PAYLOAD_BYTES} B \
         encoded payload (constant {constant})",
    );
    assert_eq!(
        implied, constant,
        "reference wire-cost estimate {constant} differs from the measured mean {implied}",
    );
}

/// Tiny records' rounded mean stays one byte below payload plus the estimate.
#[test]
fn minimal_records_cost_less_than_the_reference_estimate() {
    let implied = implied_bytes_per_message::<u64>(|rng| rng.next_u64());
    let expected = dispute_overhead_bytes() + U64_ENCODED_BYTES - MINIMAL_CELL_RESIDUAL;
    eprintln!("minimal-record cell: implied {implied} B/message (expected {expected})");
    assert_eq!(
        implied, expected,
        "small-record mean changed: measured {implied} B at \
         {U64_ENCODED_BYTES} B encoded payloads against the pinned {expected} B",
    );
}

/// At 64 encoded bytes, the fixture's rounded mean equals payload plus overhead.
#[test]
fn mid_size_records_match_the_reference_estimate() {
    let mut make = |rng: &mut ChaChaRng| {
        let mut payload = vec![0u8; MID_PAYLOAD_LEN];
        rng.fill_bytes(&mut payload);
        Bytes::from(payload)
    };
    let implied = implied_bytes_per_message::<Bytes>(&mut make);
    let expected = dispute_overhead_bytes() + CBOR_BSTR_HEADER_BYTES + MID_PAYLOAD_LEN;
    eprintln!("mid-record cell: implied {implied} B/message (expected {expected})");
    assert_eq!(
        implied,
        expected,
        "mid-size mean changed: measured {implied} B at \
         {} B encoded payloads against the pinned {expected} B",
        CBOR_BSTR_HEADER_BYTES + MID_PAYLOAD_LEN,
    );
}

/// A large sample stays within two bytes of the table's overhead estimate.
///
/// At 75,000 messages per side, structural costs are sufficiently amortized
/// for this tolerance while the test remains affordable. Check the unrounded
/// mean: truncation would hide a material part of this error.
#[test]
fn table_corpus_has_similar_protocol_overhead() {
    let messages = 75_000;
    let (left, right) = diverged(0, messages, WindowChoice::Default, |rng| {
        let mut payload = vec![0u8; REFERENCE_PAYLOAD_LEN];
        rng.fill_bytes(&mut payload);
        Bytes::from(payload)
    });
    let total = session_wire_bytes(&left, &right);
    assert_eq!(left.snapshot().hash(), right.snapshot().hash());
    assert_eq!(left.snapshot().len(), 2 * messages);
    let overhead = total as f64 / (2 * messages) as f64 - REFERENCE_ENCODED_PAYLOAD_BYTES as f64;
    let estimate = dispute_overhead_bytes() as f64;
    assert!(
        (overhead - estimate).abs() < 2.0,
        "table overhead estimate {estimate} differs from measured {overhead:.3}",
    );
}

/// Even an already converged pair has nonzero session overhead.
/// This confirms that the counter includes more than transferred payloads.
#[test]
fn counting_link_sees_a_converged_sessions_greeting() {
    let (left, right) = diverged(COMMON, DIVERGENT, WindowChoice::Floor, |rng| rng.next_u64());
    // Converge first over an uncounted link, then count a session that
    // disputes nothing.
    common::wire::wire_gossip(&left, &right);
    let total = session_wire_bytes(&left, &right);
    eprintln!("converged-session control: {total} B fixed overhead");
    assert!(
        total > 0,
        "a converged session still exchanges greetings: a zero count is a dead instrument",
    );
    assert!(
        total < 2 * DIVERGENT,
        "converged-session overhead ({total} bytes) exceeds one byte per transferred \
         message at calibration scale",
    );
}
