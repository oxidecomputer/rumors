//! Deterministic calibration pins for the dispute wire-cost law.
//!
//! The closed form in `Peer::sync_memory_budget`'s docs takes a
//! session's mean encoded record size through the per-message wire law
//! pinned here, and the design-record anchor (`DISPUTE_WIRE_BYTES`) is
//! that law's value at one stated record size. Nothing else in the
//! suite ties either to actual wire bytes (the operator suite
//! self-calibrates its link rate). These pins close the loop with pure
//! byte counts — every write on the control stream and on every data
//! stream of an in-memory session is tallied, no timing anywhere — so a
//! wire-format change that moves the real cost of a disputed message
//! fails here instead of silently letting the documented law go stale.
//!
//! What the counts establish: the current format's end-to-end cost of one
//! disputed message — its question share, reply share, and leaf record —
//! is affine in the record's encoded payload, the calibrated intercept
//! plus the payload's CBOR encoding. The constant is that cost at the
//! design point's [`DESIGN_ENCODED_PAYLOAD_BYTES`]-byte record; leaner
//! records cost proportionally less wire per message. Three cells pin
//! the line — the intercept, an interior point, and the design point —
//! so a change to the per-record framing or the record body moves at
//! least one loudly, and the linearity claim is itself gated.
//!
//! Payload corpora are [`bytes::Bytes`], which serde carries as a CBOR
//! byte string: a fixed-length payload has one deterministic encoded
//! size (a header plus the raw bytes), which is what lets each cell
//! state its encoded payload size exactly.

mod common;

use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};

use bytes::Bytes;
use rand::rngs::SmallRng;
use rand::{RngCore, SeedableRng};
use rumors::link::{Connector, Done, Link, LinkParts, MemoryLink};
use rumors::testing::{dispute_overhead_bytes, envelope_and_wire_bytes};
use rumors::{Peer, Rumors};
use tokio::io::AsyncWrite;

use crate::common::wire::block_on;

use serde::Serialize;
use serde::de::DeserializeOwned;
/// Messages both peers share before the fork: enough that the disputed
/// frontier crosses shared structure, as real sessions do.
const COMMON: usize = 2_048;

/// Messages each side originates alone: the disputed messages whose
/// crossings the byte count is divided by. Large enough that the fixed
/// greeting and epilogue overhead (kilobytes) amortizes below one byte
/// per message.
const DIVERGENT: usize = 8_192;

/// Roomy per-stream buffering: the count is schedule-independent, so the
/// pipe only needs to never distort the session's shape.
const LINK_CAPACITY: usize = 8 * 1024 * 1024;

/// End-to-end wire bytes of one disputed message beyond its record's
/// encoded payload — the crate's calibrated intercept.
///
/// Read through
/// [`dispute_overhead_bytes`] so the cells here pin the constant the
/// closed form quotes, not a test-local copy of it.
fn fixed_overhead_bytes() -> usize {
    dispute_overhead_bytes()
}

/// The `Bytes` payload length whose CBOR encoding (a 2-byte byte-string
/// header plus the bytes, 172 B) prices a disputed message at exactly
/// `DISPUTE_WIRE_BYTES` under the current format.
///
/// This is the record size the design-point constant is denominated in.
const DESIGN_PAYLOAD_LEN: usize = 170;

/// A mid-size `Bytes` payload length (64 B encoded behind CBOR's 2-byte
/// byte-string header): the interior cell that holds the affine law
/// between the minimal and design endpoints.
const MID_PAYLOAD_LEN: usize = 62;

/// CBOR's byte-string header width for lengths in `24..=255`: the major
/// type byte plus one length byte.
const CBOR_BSTR_HEADER_BYTES: usize = 2;

/// The design record's encoded payload: [`DESIGN_PAYLOAD_LEN`] bytes
/// behind CBOR's byte-string header.
const DESIGN_ENCODED_PAYLOAD_BYTES: usize = CBOR_BSTR_HEADER_BYTES + DESIGN_PAYLOAD_LEN;

/// A random `u64`'s CBOR encoding: the one-byte major-type header plus
/// eight value bytes (every seeded draw exceeds 2³², so the width is
/// deterministic for the minimal cell's corpus).
const U64_ENCODED_BYTES: usize = 9;

/// Slack on each pinned per-message figure, in bytes. The counts are
/// deterministic, so the slack only absorbs integer-division adjacency;
/// any per-record format change of a byte or more trips a pin.
const TOLERANCE_BYTES: usize = 2;

/// An `AsyncWrite` that tallies every byte accepted by the inner writer.
struct CountingWrite<W> {
    inner: W,
    written: Arc<AtomicUsize>,
}

impl<W: AsyncWrite + Unpin> AsyncWrite for CountingWrite<W> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let poll = Pin::new(&mut self.inner).poll_write(cx, buf);
        if let Poll::Ready(Ok(accepted)) = &poll {
            self.written.fetch_add(*accepted, Ordering::Relaxed);
        }
        poll
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

/// A [`Connector`] whose opened streams tally their writes into the
/// shared counter.
#[derive(Clone)]
struct CountingConnector<C> {
    inner: C,
    written: Arc<AtomicUsize>,
}

impl<C: Connector> Connector for CountingConnector<C> {
    type Tx = CountingWrite<C::Tx>;

    async fn connect(&self) -> io::Result<(Self::Tx, Done<Self::Tx>)> {
        let (inner, _) = self.inner.connect().await?;
        Ok((
            CountingWrite {
                inner,
                written: self.written.clone(),
            },
            Done::discard(),
        ))
    }
}

/// Decorate one in-memory link end so its control writes and every data
/// stream it opens tally into `written`.
fn counting(
    link: MemoryLink,
    written: &Arc<AtomicUsize>,
) -> Link<
    tokio::io::DuplexStream,
    CountingWrite<tokio::io::DuplexStream>,
    CountingConnector<rumors::link::MemoryConnector>,
    rumors::link::MemoryAcceptor,
> {
    let parts = link.into_parts();
    LinkParts {
        control_read: parts.control_read,
        control_write: CountingWrite {
            inner: parts.control_write,
            written: written.clone(),
        },
        connector: CountingConnector {
            inner: parts.connector,
            written: written.clone(),
        },
        acceptor: parts.acceptor,
        session: parts.session,
    }
    .into_link()
}

/// Two floor-window peers sharing [`COMMON`] messages, then diverged by
/// [`DIVERGENT`] minted payloads on each side, deterministically.
fn diverged<T>(mut mint: impl FnMut(&mut SmallRng) -> T) -> (Rumors<T>, Rumors<T>)
where
    T: Serialize + DeserializeOwned + Send + Sync + Clone + 'static,
{
    let left = Peer::seed().sync_window_floor().into_rumors();
    let mut rng = SmallRng::seed_from_u64(0x0b05_2026_d15b_073e);
    let mut send = |rumors: &Rumors<T>, n: usize, rng: &mut SmallRng| {
        let mut batch = rumors.batch();
        for _ in 0..n {
            batch.send(mint(rng));
        }
    };
    send(&left, COMMON, &mut rng);
    let right = common::wire::bootstrap_fork(&left);
    send(&left, DIVERGENT, &mut rng);
    send(&right, DIVERGENT, &mut rng);
    (left, right)
}

/// The end-to-end wire bytes of one session between `a` and `b`: control
/// stream plus every data stream, both directions, counted at the write
/// side of each end.
fn session_wire_bytes<T>(a: &Rumors<T>, b: &Rumors<T>) -> usize
where
    T: Serialize + DeserializeOwned + Send + Sync + 'static,
{
    let written = Arc::new(AtomicUsize::new(0));
    let (a_link, b_link) = rumors::link::memory_with_capacity(LINK_CAPACITY);
    let mut a_link = counting(a_link, &written);
    let mut b_link = counting(b_link, &written);
    block_on(async {
        let (near, far) = tokio::join!(a.gossip(&mut a_link), b.gossip(&mut b_link));
        near.expect("gossip completes over the counting link");
        far.expect("gossip completes over the counting link");
    });
    written.load(Ordering::Relaxed)
}

/// The implied end-to-end bytes per disputed message over one measured
/// session of the given corpus: total wire bytes over the messages that
/// crossed.
///
/// Each side's divergence crosses once; shared content crosses
/// only as dispute-descent overhead, which is part of the per-message
/// cost the constant states.
fn implied_bytes_per_message<T>(mint: impl FnMut(&mut SmallRng) -> T) -> usize
where
    T: Serialize + DeserializeOwned + Send + Sync + Clone + 'static,
{
    let (left, right) = diverged(mint);
    let total = session_wire_bytes(&left, &right);
    assert_eq!(
        left.snapshot().hash(),
        right.snapshot().hash(),
        "the calibration session must actually converge",
    );
    total / (2 * DIVERGENT)
}

/// `DISPUTE_WIRE_BYTES` is the measured end-to-end cost of one disputed
/// message at the design record size.
///
/// The invariant: total session wire bytes over a known mutual
/// divergence of [`DESIGN_PAYLOAD_LEN`]-byte payloads, divided by the
/// messages that crossed, brackets the constant within
/// [`TOLERANCE_BYTES`] — so the constant that denominates the default
/// budget and both operator equations is tied to the wire format by
/// deterministic byte counts. The corpus is seeded and the link is
/// in-memory: the figure is exact across runs and machines, and any
/// per-record change to the format moves it out of the band.
#[test]
fn dispute_wire_bytes_is_the_design_record_cost() {
    let mut mint = |rng: &mut SmallRng| {
        let mut payload = vec![0u8; DESIGN_PAYLOAD_LEN];
        rng.fill_bytes(&mut payload);
        Bytes::from(payload)
    };
    let implied = implied_bytes_per_message::<Bytes>(&mut mint);
    let (_, constant) = envelope_and_wire_bytes();
    eprintln!(
        "design-record cell: implied {implied} B/message at {DESIGN_ENCODED_PAYLOAD_BYTES} B \
         encoded payload (constant {constant})",
    );
    assert!(
        constant.abs_diff(implied) <= TOLERANCE_BYTES,
        "DISPUTE_WIRE_BYTES ({constant}) must equal the measured {implied} B per disputed \
         message at the design record size: re-derive the constant (and the default \
         budget and operator ratio built on it) against the current wire format",
    );
}

/// The per-message fixed overhead — question share, reply share, and
/// record framing beyond the encoded payload — is pinned at the
/// minimal-payload end of the line.
///
/// The invariant: a `u64` corpus ([`U64_ENCODED_BYTES`]-byte encoded
/// payloads) implies the calibrated intercept + that width per disputed
/// message. Together with the design-record cell this pins both
/// parameters of the affine cost `overhead + encoded_payload`, so
/// framing drift cannot hide inside the design cell's payload term. It
/// is also the honest floor: minimal-payload sessions cost several
/// times less wire per disputed message than the design constant, and
/// correspondingly need more scopes in flight to fill the same link.
#[test]
fn minimal_records_pin_the_fixed_overhead() {
    let implied = implied_bytes_per_message::<u64>(|rng| rng.next_u64());
    let expected = fixed_overhead_bytes() + U64_ENCODED_BYTES;
    eprintln!("minimal-record cell: implied {implied} B/message (expected {expected})");
    assert!(
        expected.abs_diff(implied) <= TOLERANCE_BYTES,
        "the fixed per-message overhead moved: measured {implied} B at \
         {U64_ENCODED_BYTES} B encoded payloads against the pinned {expected} B",
    );
}

/// A mid-size record cell holds the affine law between the endpoints:
/// the cost is intercept-plus-payload in the interior, not just at the
/// two pinned ends.
///
/// The invariant: a corpus of [`MID_PAYLOAD_LEN`]-byte payloads (64 B
/// encoded) implies the calibrated intercept + 64 bytes per disputed
/// message, within [`TOLERANCE_BYTES`]. With the minimal and design
/// cells this makes the linearity claim itself a committed, gated
/// assertion — three collinear points — rather than a calibration-time
/// observation.
#[test]
fn mid_size_records_ride_the_affine_law() {
    let mut mint = |rng: &mut SmallRng| {
        let mut payload = vec![0u8; MID_PAYLOAD_LEN];
        rng.fill_bytes(&mut payload);
        Bytes::from(payload)
    };
    let implied = implied_bytes_per_message::<Bytes>(&mut mint);
    let expected = fixed_overhead_bytes() + CBOR_BSTR_HEADER_BYTES + MID_PAYLOAD_LEN;
    eprintln!("mid-record cell: implied {implied} B/message (expected {expected})");
    assert!(
        expected.abs_diff(implied) <= TOLERANCE_BYTES,
        "the affine cost law broke in the interior: measured {implied} B at \
         {} B encoded payloads against the pinned {expected} B",
        CBOR_BSTR_HEADER_BYTES + MID_PAYLOAD_LEN,
    );
}

/// Negative control: the counting instrument observes bytes even when
/// nothing is disputed.
///
/// A converged pair's session is greeting-and-epilogue only; if the
/// counter read zero there, the calibration cells could pass vacuously
/// with a dead instrument. The control also pins the fixed session
/// overhead to kilobytes, which is what makes it negligible against the
/// per-message division at [`DIVERGENT`] scale.
#[test]
fn counting_link_sees_a_converged_sessions_greeting() {
    let (left, right) = diverged::<u64>(|rng| rng.next_u64());
    // Converge first over an uncounted link, then count a session that
    // disputes nothing.
    common::wire::wire_gossip(&left, &right);
    let total = session_wire_bytes(&left, &right);
    assert!(
        total > 0,
        "a converged session still exchanges greetings: a zero count is a dead instrument",
    );
    assert!(
        total < 64 * 1024,
        "a converged session's fixed overhead stays in kilobytes ({total} bytes measured): \
         it must stay negligible against the per-message division at calibration scale",
    );
}
