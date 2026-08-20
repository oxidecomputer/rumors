//! Stable witnesses for every frame-stream codec error reachable without
//! resource exhaustion.
//!
//! The scope is the frame stream's own taxonomies — the encode, decode,
//! and record-iteration errors the `describe_*` matches below inventory.
//! The codec's handshake-layer surface is witnessed where it lives:
//! `GreetingError` in greeting/tests.rs, beside the greeting reader; and
//! `ListingIssue`, which the frame decoder collapses into this taxonomy
//! (witnessed here as `QueryOutOfOrder` and `Malformed(part=QueryChildren)`),
//! carries its typed surface through the greeting, witnessed in the same
//! suite. Both hold exemption entries below so a witness landing here is
//! flagged for promotion.
//!
//! Coverage is enforced from both ends: every `describe_*` match below is
//! wildcard-free, so a new error variant fails compilation until it is
//! described, and `atlas_covers_every_error_variant` requires each described
//! variant's rendered marker to appear in the atlas — or to carry an explicit,
//! reasoned exemption in `EXEMPT_MARKERS`. The one hole neither half can see:
//! a variant whose new match arm is added without extending the marker tables
//! or the witnesses; the comments at each match carry that obligation. An
//! exemption also outlives its variant silently — the check asserts absence,
//! which a deleted variant satisfies trivially — so pruning a variant must
//! prune its `EXEMPT_MARKERS` entry by hand.

use crate::message::{PayloadCodec, PayloadDepthLimit};
use std::{
    error::Error,
    fmt::Write as _,
    io,
    pin::Pin,
    task::{Context, Poll},
};

use tokio::io::AsyncWrite;

use super::super::{
    DecodeError, DecodeErrorKind, DecodeLeafError, DecodeSignalError, EncodeError, EncodeErrorKind,
    Flow, Frame, FrameWrite, LeafRunError, Reaction, RunBudget, Speaker, Stream, WireFrame, decode,
    decode_exact, encode,
    frame::LeafRun,
    signal::{Signal, WireSignal},
};
use crate::tree::mirror::cbor::{self, MAJOR_BSTR, MAJOR_TAG, TAG_CBOR_SEQUENCE};
use crate::{Version, message::Message, tree::typed::Hash};

use serde::Serialize;
/// One rendered marker per error variant the atlas must witness.
///
/// Grouped by the enum whose `describe_*` match is the compile-time
/// tripwire for it; a new variant's match arm and its entry here land
/// together, with a witness below (or an exemption in `EXEMPT_MARKERS`).
const WITNESS_MARKERS: &[&str] = &[
    // EncodeErrorKind (describe_encode_kind).
    "kind: Write(part=",
    "kind: Flush(io=",
    // DecodeErrorKind, including its nested DecodeSignalError and
    // LeafRunError variants (describe_decode_kind).
    "kind: Read(part=",
    "kind: InvalidSignal::Reserved(",
    "kind: InvalidSignal::Placement(",
    "kind: Truncated(missing=",
    "kind: QueryOutOfOrder(previous=",
    "kind: InvalidRun::Empty",
    "kind: InvalidRun::Head(",
    "kind: InvalidRun::NotARecord(",
    "kind: InvalidRun::TruncatedRecord(",
    "kind: OverbatchedRun(declared=",
    "kind: TrailingBytes(count=",
    "kind: FrameShape(",
    "kind: FrameArity(",
    "kind: Malformed(part=",
    // DecodeLeafError (describe_leaf_kind).
    "kind: Record::Version(io=",
    "kind: Record::Message(io=",
    // FramePart: every frame component must fail somewhere, whichever
    // side witnesses it — the encode Write witnesses carry most parts,
    // while Signal renders only from decode-side witnesses (the encoder
    // never fails at the signal separately from the frame head). FramePart
    // has no exhaustive match here, so a new component's marker must be
    // added by hand alongside its witnesses.
    "part=FrameHead",
    "part=Signal",
    "part=QueryChildren",
    "part=SupplyLength",
    "part=SupplyRun",
];

/// Variants deliberately absent from the atlas, each with the reason —
/// unreachable without resource exhaustion, or witnessed in another
/// layer's own suite.
const EXEMPT_MARKERS: &[(&str, &str)] = &[
    (
        "kind: SupplyTooLarge(",
        "requires a run body past the wire's run byte cap: a >4 GiB in-memory \
         run is resource exhaustion by construction; the cap itself is pinned \
         at its exact boundary in frame/tests.rs",
    ),
    (
        "kind: Greeting",
        "GreetingError is the handshake layer's surface, not a frame-stream \
         error: its variants are witnessed in greeting/tests.rs, beside the \
         greeting reader",
    ),
    (
        "kind: Listing",
        "ListingIssue never surfaces from the frame decoders: they collapse \
         it into QueryOutOfOrder and Malformed(part=QueryChildren), both \
         witnessed here; its typed surface is the greeting's \
         (GreetingError::Listing), witnessed in greeting/tests.rs",
    ),
];

/// Interior stream used where both speakers admit every signal state.
const INTERIOR_STREAM: u8 = 8;

/// First reserved semantic-state byte.
const FIRST_RESERVED_SIGNAL: u8 = WireSignal::BYTE_COUNT;

/// Build a supply run holding one leaf record.
fn one_record_run<T: Serialize + Send + Sync + 'static>(version: Version, value: T) -> LeafRun {
    let mut run = LeafRun::new();
    run.push(&version, &Message::new(value))
        .expect("an atlas record fits the run framing");
    run
}

/// Every feasible typed failure pins its fields and source chain, and its
/// origin where one exists (the record-level witnesses carry none;
/// `record_errors` says why).
#[test]
fn codec_error_atlas_snapshot() {
    insta::assert_snapshot!(build_atlas());
}

/// Every inventoried error variant has an atlas witness, and every
/// exemption is genuinely absent (a witnessed exemption is stale).
#[test]
fn atlas_covers_every_error_variant() {
    let atlas = build_atlas();
    for marker in WITNESS_MARKERS {
        assert!(
            atlas.contains(marker),
            "no atlas witness renders {marker:?}: add a witness for the \
             variant, or move its marker out of WITNESS_MARKERS into a \
             reasoned EXEMPT_MARKERS entry",
        );
    }
    for (marker, reason) in EXEMPT_MARKERS {
        assert!(
            !atlas.contains(marker),
            "exempt variant {marker:?} now has a witness: move it into \
             WITNESS_MARKERS (recorded exemption reason: {reason})",
        );
    }
}

fn build_atlas() -> String {
    let mut atlas = String::new();
    encode_errors(&mut atlas);
    decode_errors(&mut atlas);
    record_errors(&mut atlas);
    atlas
}

fn encode_errors(atlas: &mut String) {
    writeln!(atlas, "ENCODE").unwrap();
    let stream = Stream::new(INTERIOR_STREAM).unwrap();
    let query: WireFrame = (
        stream,
        Frame::Reaction(
            Reaction::Query(vec![(1, Hash::default()), (2, Hash::default())]),
            Flow::Continue,
        ),
    );
    let supply: WireFrame = (
        stream,
        Frame::Reaction(
            Reaction::Supply(one_record_run(Version::new(), 7)),
            Flow::Continue,
        ),
    );

    for speaker in [Speaker::Initiator, Speaker::Responder] {
        // Offsets in whole delivered bytes: the interior-stream query and
        // supply frames open with a three-byte frame head (one array byte,
        // a two-byte signal head), and the small supply run's own heads
        // take three more.
        for (label, frame, offset) in [
            ("write/frame-head", &query, 0),
            ("write/query-children", &query, 3),
            ("write/supply-length", &supply, 3),
            ("write/supply-run", &supply, 6),
        ] {
            let error = encode(speaker, frame, &mut FailAfterWriter::new(offset)).unwrap_err();
            record_encode(atlas, &format!("{speaker:?}/{label}"), &error);
        }

        let mut writer = FrameWrite::new(speaker, FlushFailingWriter);
        let error = pollster::block_on(writer.frame(&query)).unwrap_err();
        record_encode(atlas, &format!("{speaker:?}/flush"), &error);
    }
}

fn decode_errors(atlas: &mut String) {
    writeln!(atlas, "DECODE").unwrap();
    let stream = Stream::new(INTERIOR_STREAM).unwrap();
    let query = encoded(
        Speaker::Initiator,
        (
            stream,
            Frame::Reaction(
                Reaction::Query(vec![(1, Hash::default()), (2, Hash::default())]),
                Flow::Continue,
            ),
        ),
    );
    let supply = encoded(
        Speaker::Initiator,
        (
            stream,
            Frame::Reaction(
                Reaction::Supply(one_record_run(Version::new(), 7_u8)),
                Flow::Continue,
            ),
        ),
    );
    let matched = encoded(
        Speaker::Initiator,
        (stream, Frame::Reaction(Reaction::Match, Flow::Continue)),
    );

    for speaker in [Speaker::Initiator, Speaker::Responder] {
        // The three-byte frame head and the small supply run's heads
        // locate every read failure below.
        let error = decode(
            speaker,
            RunBudget::default(),
            &mut FailAfterReader::new(matched.clone(), 0),
        )
        .unwrap_err();
        record_decode(atlas, &format!("{speaker:?}/read/frame-head"), &error);

        let error = decode(
            speaker,
            RunBudget::default(),
            &mut FailAfterReader::new(matched.clone(), 1),
        )
        .unwrap_err();
        record_decode(atlas, &format!("{speaker:?}/read/signal"), &error);

        let error = decode(
            speaker,
            RunBudget::default(),
            &mut FailAfterReader::new(query.clone(), 3),
        )
        .unwrap_err();
        record_decode(atlas, &format!("{speaker:?}/read/query-children"), &error);
        for (label, offset) in [("supply-length", 3), ("supply-run", 6)] {
            let error = decode(
                speaker,
                RunBudget::default(),
                &mut FailAfterReader::new(supply.clone(), offset),
            )
            .unwrap_err();
            record_decode(atlas, &format!("{speaker:?}/read/{label}"), &error);
        }

        for (label, bytes) in [
            ("frame-head", &[][..]),
            ("signal", &matched[..1]),
            ("query-children", &query[..3]),
            ("supply-length", &supply[..4]),
            ("supply-run", &supply[..6]),
        ] {
            let error = decode_exact(speaker, RunBudget::default(), bytes).unwrap_err();
            record_decode(atlas, &format!("{speaker:?}/truncated/{label}"), &error);
        }

        let mut reserved = Vec::new();
        cbor::write_head(&mut reserved, cbor::MAJOR_ARRAY, 1);
        cbor::write_head(
            &mut reserved,
            cbor::MAJOR_UINT,
            u64::from(FIRST_RESERVED_SIGNAL),
        );
        let error = decode_exact(speaker, RunBudget::default(), &reserved).unwrap_err();
        record_decode(atlas, &format!("{speaker:?}/reserved-signal"), &error);

        // The frame item's own shape violations: a non-array item, an
        // arity contradicting the signal, and non-canonical heads.
        let error = decode_exact(speaker, RunBudget::default(), &[0x00]).unwrap_err();
        record_decode(atlas, &format!("{speaker:?}/frame/not-an-array"), &error);

        let mut mismatched = Vec::new();
        cbor::write_head(&mut mismatched, cbor::MAJOR_ARRAY, 2);
        mismatched.extend_from_slice(&matched[1..]);
        let error = decode_exact(speaker, RunBudget::default(), &mismatched).unwrap_err();
        record_decode(atlas, &format!("{speaker:?}/frame/arity"), &error);

        // The matched frame's signal is a one-byte head (a small code),
        // so its code byte is the head itself; respell it widened.
        let widened = [0x81, 0x19, 0x00, matched[1]];
        let error = decode_exact(speaker, RunBudget::default(), &widened).unwrap_err();
        record_decode(atlas, &format!("{speaker:?}/frame/widened-signal"), &error);

        let unordered = encoded(
            speaker,
            (
                stream,
                Frame::Reaction(
                    Reaction::Query(vec![(2, Hash::default()), (1, Hash::default())]),
                    Flow::Continue,
                ),
            ),
        );
        let error = decode_exact(speaker, RunBudget::default(), &unordered).unwrap_err();
        record_decode(atlas, &format!("{speaker:?}/query-out-of-order"), &error);

        // A listing whose first key is a well-formed head of the wrong
        // kind (a byte string where a radix belongs) reaches the listing
        // gate and collapses into this taxonomy as
        // Malformed(part=QueryChildren).
        let listing_signal = WireSignal::new(speaker, stream, Signal::Query(Flow::Continue))
            .unwrap()
            .to_byte();
        let mut defective_listing = Vec::new();
        cbor::write_head(&mut defective_listing, cbor::MAJOR_ARRAY, 2);
        cbor::write_head(
            &mut defective_listing,
            cbor::MAJOR_UINT,
            u64::from(listing_signal),
        );
        cbor::write_head(&mut defective_listing, cbor::MAJOR_MAP, 1);
        cbor::write_head(&mut defective_listing, MAJOR_BSTR, 0);
        let error = decode_exact(speaker, RunBudget::default(), &defective_listing).unwrap_err();
        record_decode(atlas, &format!("{speaker:?}/query/listing-key"), &error);

        let error = decode_exact(
            speaker,
            RunBudget::default(),
            &raw_supply(stream, Flow::Continue, &[]),
        )
        .unwrap_err();
        record_decode(atlas, &format!("{speaker:?}/run/empty"), &error);

        let error = decode_exact(
            speaker,
            RunBudget::default(),
            &raw_supply(stream, Flow::Continue, &[0, 0]),
        )
        .unwrap_err();
        record_decode(atlas, &format!("{speaker:?}/run/not-a-record"), &error);

        let error = decode_exact(
            speaker,
            RunBudget::default(),
            &raw_supply(stream, Flow::Continue, &[0xd8, 0x3f, 0x58, 0x01, 0x00]),
        )
        .unwrap_err();
        record_decode(atlas, &format!("{speaker:?}/run/widened-head"), &error);

        let mut overrun = Vec::new();
        cbor::write_head(&mut overrun, MAJOR_TAG, TAG_CBOR_SEQUENCE);
        cbor::write_head(&mut overrun, MAJOR_BSTR, 2);
        overrun.push(0);
        let error = decode_exact(
            speaker,
            RunBudget::default(),
            &raw_supply(stream, Flow::Continue, &overrun),
        )
        .unwrap_err();
        record_decode(atlas, &format!("{speaker:?}/run/truncated-record"), &error);

        // Two records whose shared frame outsizes a zero budget: not the
        // lone-record overhang, so the ingress gate rejects the batching.
        let mut batched = Vec::new();
        let mut run = one_record_run(Version::new(), 7_u8);
        run.push(&Version::new(), &Message::new(7_u8))
            .expect("an atlas record fits the run framing");
        encode(
            speaker,
            &(
                stream,
                Frame::Reaction(Reaction::Supply(run), Flow::Continue),
            ),
            &mut batched,
        )
        .unwrap();
        let error = decode_exact(speaker, RunBudget::from_bytes(0), &batched).unwrap_err();
        record_decode(atlas, &format!("{speaker:?}/run/overbatched"), &error);

        let mut trailing = matched.clone();
        trailing.push(0);
        let error = decode_exact(speaker, RunBudget::default(), &trailing).unwrap_err();
        record_decode(atlas, &format!("{speaker:?}/frame/trailing"), &error);
    }

    for (label, speaker, stream, frame) in placement_witnesses() {
        let signal = frame_signal(&frame);
        let invalid = WireSignal::new(speaker, stream, signal).unwrap_err();
        let mut bytes = Vec::new();
        cbor::write_head(&mut bytes, cbor::MAJOR_ARRAY, 1);
        cbor::write_head(&mut bytes, cbor::MAJOR_UINT, u64::from(invalid.byte()));
        let error = decode_exact(speaker, RunBudget::default(), &bytes).unwrap_err();
        record_decode(atlas, &format!("{label}/decode"), &error);
    }
}

/// Witness the record-level decode failures a supplied leaf can carry.
///
/// These surface from the run's record iterator rather than the frame
/// decoder — a structurally valid run defers canonical decoding of each
/// record — so they have no `Origin` and no speaker dimension.
fn record_errors(atlas: &mut String) {
    writeln!(atlas, "RECORD").unwrap();

    // An empty-content record is structurally valid; its missing
    // version-atom tag fails at the version decoder.
    let run = LeafRun::from_encoded(framed_record(&[])).unwrap();
    record_leaf(atlas, "record/version", &next_record_error(&run));

    // A record ending after its tagged version fails at the message
    // decoder.
    let mut version = Vec::new();
    cbor::write_head(&mut version, MAJOR_TAG, crate::tags::VERSION_TAG);
    ciborium::ser::into_writer(&Version::new(), &mut version).unwrap();
    let run = LeafRun::from_encoded(framed_record(&version)).unwrap();
    record_leaf(atlas, "record/message", &next_record_error(&run));

    // Bytes past the canonical pair are a malformed payload: the payload
    // runs to the record's end, so the deserializer rejects the excess.
    let mut padded = version.clone();
    ciborium::ser::into_writer(&0_u64, &mut padded).unwrap();
    padded.push(u8::MIN);
    let run = LeafRun::from_encoded(framed_record(&padded)).unwrap();
    record_leaf(atlas, "record/trailing", &next_record_error(&run));
}

/// Frame one record content behind its embedded-sequence heads, as a run
/// body.
fn framed_record(record: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    cbor::write_head(&mut body, MAJOR_TAG, TAG_CBOR_SEQUENCE);
    cbor::write_head(&mut body, MAJOR_BSTR, record.len() as u64);
    body.extend_from_slice(record);
    body
}

/// The first record's decode failure from a structurally valid run.
fn next_record_error(run: &LeafRun) -> DecodeLeafError {
    run.records(PayloadCodec::new::<u64>(PayloadDepthLimit::default()))
        .next()
        .expect("the run holds one record")
        .unwrap_err()
}

fn record_leaf(atlas: &mut String, label: &str, error: &DecodeLeafError) {
    writeln!(atlas, "  {label}").unwrap();
    writeln!(atlas, "    display: {error}").unwrap();
    write!(atlas, "    kind: ").unwrap();
    describe_leaf_kind(atlas, error);
    atlas.push('\n');
    record_sources(atlas, error);
}

// No wildcard arm, deliberately: a new DecodeLeafError variant must be
// described here, marked in WITNESS_MARKERS, and witnessed above (or
// exempted with a reason).
fn describe_leaf_kind(out: &mut String, kind: &DecodeLeafError) {
    match kind {
        DecodeLeafError::Version(source) => {
            write!(out, "Record::Version(io={:?})", source.kind()).unwrap()
        }
        DecodeLeafError::Message(source) => {
            write!(out, "Record::Message(io={:?})", source.kind()).unwrap()
        }
    }
}

fn placement_witnesses() -> [(&'static str, Speaker, Stream, Frame); 3] {
    [
        (
            "placement/opening-supplies",
            Speaker::Initiator,
            Stream::new(0).unwrap(),
            Frame::Reaction(Reaction::Match, Flow::Continue),
        ),
        (
            "placement/leaf-parent",
            Speaker::Initiator,
            Stream::new(Stream::MAX).unwrap(),
            Frame::Reaction(Reaction::Query(vec![(0, Hash::default())]), Flow::Continue),
        ),
        (
            "placement/terminal-leaf",
            Speaker::Responder,
            Stream::new(Stream::MAX).unwrap(),
            Frame::Reaction(Reaction::Match, Flow::Continue),
        ),
    ]
}

fn encoded(speaker: Speaker, frame: WireFrame) -> Vec<u8> {
    let mut bytes = Vec::new();
    encode(speaker, &frame, &mut bytes).unwrap();
    bytes
}

fn raw_supply(stream: Stream, flow: Flow, body: &[u8]) -> Vec<u8> {
    let signal = WireSignal::new(Speaker::Initiator, stream, Signal::Supply(flow))
        .unwrap()
        .to_byte();
    let mut encoded = Vec::new();
    cbor::write_head(&mut encoded, cbor::MAJOR_ARRAY, 2);
    cbor::write_head(&mut encoded, cbor::MAJOR_UINT, u64::from(signal));
    cbor::write_head(&mut encoded, MAJOR_TAG, TAG_CBOR_SEQUENCE);
    cbor::write_head(&mut encoded, MAJOR_BSTR, body.len() as u64);
    encoded.extend_from_slice(body);
    encoded
}

fn frame_signal(frame: &Frame) -> Signal {
    match frame {
        Frame::Reaction(Reaction::Match, flow) => Signal::Match(*flow),
        Frame::Reaction(Reaction::Query(children), flow) if children.is_empty() => {
            Signal::QueryEmpty(*flow)
        }
        Frame::Reaction(Reaction::Query(_), flow) => Signal::Query(*flow),
        Frame::Reaction(Reaction::Supply(_), flow) => Signal::Supply(*flow),
        Frame::End(end) => Signal::End(*end),
    }
}

fn record_encode(atlas: &mut String, label: &str, error: &EncodeError) {
    writeln!(atlas, "  {label}").unwrap();
    writeln!(atlas, "    display: {error}").unwrap();
    writeln!(atlas, "    origin: {}", error.origin).unwrap();
    write!(atlas, "    kind: ").unwrap();
    describe_encode_kind(atlas, &error.kind);
    atlas.push('\n');
    record_sources(atlas, error);
}

// No wildcard arm, deliberately: a new EncodeErrorKind variant must be
// described here, marked in WITNESS_MARKERS, and witnessed above (or
// exempted with a reason).
fn describe_encode_kind(out: &mut String, kind: &EncodeErrorKind) {
    match kind {
        EncodeErrorKind::Write { part, source } => {
            write!(out, "Write(part={part:?}, io={:?})", source.kind()).unwrap()
        }
        EncodeErrorKind::Flush(source) => write!(out, "Flush(io={:?})", source.kind()).unwrap(),
        EncodeErrorKind::SupplyTooLarge(error) => write!(out, "SupplyTooLarge({error})").unwrap(),
    }
}

fn record_decode(atlas: &mut String, label: &str, error: &DecodeError) {
    writeln!(atlas, "  {label}").unwrap();
    writeln!(atlas, "    display: {error}").unwrap();
    writeln!(atlas, "    origin: {}", error.origin).unwrap();
    write!(atlas, "    kind: ").unwrap();
    describe_decode_kind(atlas, &error.kind);
    atlas.push('\n');
    record_sources(atlas, error);
}

// No wildcard arm, deliberately — including the nested DecodeSignalError
// and LeafRunError patterns: a new variant in any of the three enums must
// be described here, marked in WITNESS_MARKERS, and witnessed above (or
// exempted with a reason).
fn describe_decode_kind(out: &mut String, kind: &DecodeErrorKind) {
    match kind {
        DecodeErrorKind::Read { part, source } => {
            write!(out, "Read(part={part:?}, io={:?})", source.kind()).unwrap()
        }
        DecodeErrorKind::InvalidSignal(DecodeSignalError::Reserved(invalid)) => write!(
            out,
            "InvalidSignal::Reserved(byte={:02x}, state={})",
            invalid.byte(),
            invalid.state()
        )
        .unwrap(),
        DecodeErrorKind::InvalidSignal(DecodeSignalError::Placement(invalid)) => write!(
            out,
            "InvalidSignal::Placement(byte={:02x}, class={:?})",
            invalid.byte(),
            invalid.class()
        )
        .unwrap(),
        DecodeErrorKind::Truncated { missing, source } => write!(
            out,
            "Truncated(missing={missing:?}, io={:?})",
            source.kind()
        )
        .unwrap(),
        DecodeErrorKind::QueryOutOfOrder(error) => write!(
            out,
            "QueryOutOfOrder(previous={}, radix={})",
            error.previous, error.radix
        )
        .unwrap(),
        DecodeErrorKind::InvalidRun(LeafRunError::Empty) => {
            write!(out, "InvalidRun::Empty").unwrap()
        }
        DecodeErrorKind::InvalidRun(LeafRunError::Head { remaining, source }) => write!(
            out,
            "InvalidRun::Head(remaining={remaining}, source={source})"
        )
        .unwrap(),
        DecodeErrorKind::InvalidRun(LeafRunError::NotARecord { remaining, detail }) => write!(
            out,
            "InvalidRun::NotARecord(remaining={remaining}, {detail})"
        )
        .unwrap(),
        DecodeErrorKind::FrameShape { detail } => write!(out, "FrameShape({detail})").unwrap(),
        DecodeErrorKind::FrameArity { expected, found } => {
            write!(out, "FrameArity(expected={expected}, found={found})").unwrap()
        }
        DecodeErrorKind::Malformed { part, detail } => {
            write!(out, "Malformed(part={part:?}, {detail})").unwrap()
        }
        DecodeErrorKind::InvalidRun(LeafRunError::TruncatedRecord { len, remaining }) => write!(
            out,
            "InvalidRun::TruncatedRecord(len={len}, remaining={remaining})"
        )
        .unwrap(),
        DecodeErrorKind::OverbatchedRun { declared, budget } => {
            write!(out, "OverbatchedRun(declared={declared}, budget={budget})").unwrap()
        }
        DecodeErrorKind::TrailingBytes { count } => {
            write!(out, "TrailingBytes(count={count})").unwrap()
        }
    }
}

fn record_sources(out: &mut String, error: &(dyn Error + 'static)) {
    let mut depth = 0;
    let mut source = error.source();
    while let Some(current) = source {
        if let Some(io) = current.downcast_ref::<io::Error>() {
            writeln!(out, "    source[{depth}]: Io({:?})", io.kind()).unwrap();
        } else {
            writeln!(out, "    source[{depth}]: {current}").unwrap();
        }
        depth += 1;
        source = current.source();
    }
}

struct FailAfterWriter {
    remaining: usize,
}

impl FailAfterWriter {
    fn new(remaining: usize) -> Self {
        Self { remaining }
    }
}

impl std::io::Write for FailAfterWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.remaining == 0 {
            return Err(io::ErrorKind::Other.into());
        }
        let written = self.remaining.min(bytes.len());
        self.remaining -= written;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct FlushFailingWriter;

impl AsyncWrite for FlushFailingWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        Poll::Ready(Ok(bytes.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Err(io::ErrorKind::Other.into()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

struct FailAfterReader {
    bytes: Vec<u8>,
    position: usize,
    remaining: usize,
}

impl FailAfterReader {
    fn new(bytes: Vec<u8>, remaining: usize) -> Self {
        Self {
            bytes,
            position: 0,
            remaining,
        }
    }
}

impl std::io::Read for FailAfterReader {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if self.remaining == 0 {
            return Err(io::ErrorKind::Other.into());
        }
        let available = self.bytes.len() - self.position;
        let read = self.remaining.min(available).min(out.len());
        out[..read].copy_from_slice(&self.bytes[self.position..self.position + read]);
        self.position += read;
        self.remaining -= read;
        Ok(read)
    }
}
