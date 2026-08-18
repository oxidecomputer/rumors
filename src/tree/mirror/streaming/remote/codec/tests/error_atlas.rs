//! Stable witnesses for every codec error reachable without resource exhaustion.
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

use std::{
    error::Error,
    fmt::Write as _,
    io,
    pin::Pin,
    task::{Context, Poll},
};

use borsh::BorshSerialize;
use tokio::io::AsyncWrite;

use super::super::{
    DecodeError, DecodeErrorKind, DecodeLeafError, DecodeSignalError, EncodeError, EncodeErrorKind,
    Flow, Frame, FrameWrite, LeafRunError, Reaction, RunBudget, Speaker, Stream, WireFrame, decode,
    decode_exact, encode,
    frame::{LeafRun, QUERY_CHILD_LEN},
    signal::{Signal, WireSignal},
};
use crate::{Version, message::Message, tree::typed::Hash};

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
    "kind: InvalidRun::TruncatedHeader(",
    "kind: InvalidRun::TruncatedRecord(",
    "kind: OverbatchedRun(declared=",
    "kind: TrailingBytes(count=",
    // DecodeLeafError (describe_leaf_kind).
    "kind: Record::Version(io=",
    "kind: Record::Message(io=",
    "kind: Record::TrailingBytes(count=",
    // FramePart: every frame component must fail somewhere. These ride the
    // encode Write witnesses; FramePart has no exhaustive match here, so a
    // new component's marker must be added by hand alongside its witnesses.
    "part=Signal",
    "part=QueryCount",
    "part=QueryChildren",
    "part=SupplyLength",
    "part=SupplyRun",
];

/// Variants deliberately absent from the atlas, each with the reason it is
/// unreachable without resource exhaustion.
const EXEMPT_MARKERS: &[(&str, &str)] = &[(
    "kind: SupplyTooLarge(",
    "requires a run body past the u32 frame ceiling: a >4 GiB in-memory run \
     is resource exhaustion by construction; the ceiling itself is pinned at \
     its exact boundary in frame/tests.rs",
)];

/// Interior stream used where both speakers admit every signal state.
const INTERIOR_STREAM: u8 = 8;

/// First reserved semantic-state byte.
const FIRST_RESERVED_SIGNAL: u8 = WireSignal::BYTE_COUNT;

/// Build a supply run holding one leaf record.
fn one_record_run<T: borsh::BorshSerialize>(version: Version, value: T) -> LeafRun<T> {
    let mut run = LeafRun::new();
    run.push(&version, &Message::new(value))
        .expect("an atlas record fits the run framing");
    run
}

/// Every feasible typed failure pins its origin, fields, and source chain.
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
             variant or an explicit exemption in EXEMPT_MARKERS",
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
    let query: WireFrame<u8> = (
        stream,
        Frame::Reaction(
            Reaction::Query(vec![(1, Hash::default()), (2, Hash::default())]),
            Flow::Continue,
        ),
    );
    let supply: WireFrame<u8> = (
        stream,
        Frame::Reaction(
            Reaction::Supply(one_record_run(Version::new(), 7)),
            Flow::Continue,
        ),
    );

    for speaker in [Speaker::Initiator, Speaker::Responder] {
        for (label, frame, offset) in [
            ("write/signal", &query, 0),
            ("write/query-count", &query, 1),
            ("write/query-children", &query, 2),
            ("write/supply-length", &supply, 1),
            ("write/supply-run", &supply, 5),
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
            Frame::<u8>::Reaction(
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
        (
            stream,
            Frame::<u8>::Reaction(Reaction::Match, Flow::Continue),
        ),
    );

    for speaker in [Speaker::Initiator, Speaker::Responder] {
        let error = decode::<u8>(
            speaker,
            RunBudget::default(),
            &mut FailAfterReader::new(matched.clone(), 0),
        )
        .unwrap_err();
        record_decode(atlas, &format!("{speaker:?}/read/signal"), &error);

        for (label, offset) in [("query-count", 1), ("query-children", 2)] {
            let error = decode::<u8>(
                speaker,
                RunBudget::default(),
                &mut FailAfterReader::new(query.clone(), offset),
            )
            .unwrap_err();
            record_decode(atlas, &format!("{speaker:?}/read/{label}"), &error);
        }
        for (label, offset) in [("supply-length", 1), ("supply-run", 5)] {
            let error = decode::<u8>(
                speaker,
                RunBudget::default(),
                &mut FailAfterReader::new(supply.clone(), offset),
            )
            .unwrap_err();
            record_decode(atlas, &format!("{speaker:?}/read/{label}"), &error);
        }

        for (label, bytes) in [
            ("signal", &[][..]),
            ("query-count", &query[..1]),
            ("query-children", &query[..2]),
            ("supply-length", &supply[..1]),
            ("supply-run", &supply[..5]),
        ] {
            let error = decode_exact::<u8>(speaker, RunBudget::default(), bytes).unwrap_err();
            record_decode(atlas, &format!("{speaker:?}/truncated/{label}"), &error);
        }

        let error = decode_exact::<u8>(speaker, RunBudget::default(), &[FIRST_RESERVED_SIGNAL])
            .unwrap_err();
        record_decode(atlas, &format!("{speaker:?}/reserved-signal"), &error);

        let mut unordered = query.clone();
        unordered[2] = 2;
        unordered[2 + QUERY_CHILD_LEN] = 1;
        let error = decode_exact::<u8>(speaker, RunBudget::default(), &unordered).unwrap_err();
        record_decode(atlas, &format!("{speaker:?}/query-out-of-order"), &error);

        let error = decode_exact::<u64>(
            speaker,
            RunBudget::default(),
            &raw_supply(stream, Flow::Continue, &[]),
        )
        .unwrap_err();
        record_decode(atlas, &format!("{speaker:?}/run/empty"), &error);

        let error = decode_exact::<u64>(
            speaker,
            RunBudget::default(),
            &raw_supply(stream, Flow::Continue, &[0, 0]),
        )
        .unwrap_err();
        record_decode(atlas, &format!("{speaker:?}/run/truncated-header"), &error);

        let mut overrun = 2_u32.to_be_bytes().to_vec();
        overrun.push(0);
        let error = decode_exact::<u64>(
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
        let error = decode_exact::<u8>(speaker, RunBudget::from_bytes(0), &batched).unwrap_err();
        record_decode(atlas, &format!("{speaker:?}/run/overbatched"), &error);

        let mut trailing = matched.clone();
        trailing.push(0);
        let error = decode_exact::<u8>(speaker, RunBudget::default(), &trailing).unwrap_err();
        record_decode(atlas, &format!("{speaker:?}/frame/trailing"), &error);
    }

    for (label, speaker, stream, frame) in placement_witnesses() {
        let signal = frame_signal(&frame);
        let invalid = WireSignal::new(speaker, stream, signal).unwrap_err();
        let error =
            decode_exact::<u8>(speaker, RunBudget::default(), &[invalid.byte()]).unwrap_err();
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

    // A zero-length record is structurally valid; its empty body fails
    // at the version decoder.
    let run = LeafRun::<u64>::from_encoded(framed_record(&[])).unwrap();
    record_leaf(atlas, "record/version", &next_record_error(&run));

    // A record ending after its version fails at the message decoder.
    let mut version = Vec::new();
    Version::new().serialize(&mut version).unwrap();
    let run = LeafRun::<u64>::from_encoded(framed_record(&version)).unwrap();
    record_leaf(atlas, "record/message", &next_record_error(&run));

    // Bytes past the canonical pair are trailing.
    let mut padded = version.clone();
    0_u64.serialize(&mut padded).unwrap();
    padded.push(u8::MIN);
    let run = LeafRun::<u64>::from_encoded(framed_record(&padded)).unwrap();
    record_leaf(atlas, "record/trailing", &next_record_error(&run));
}

/// Frame one record body with its length header, as a run body.
fn framed_record(record: &[u8]) -> Vec<u8> {
    let mut body = (record.len() as u32).to_be_bytes().to_vec();
    body.extend_from_slice(record);
    body
}

/// The first record's decode failure from a structurally valid run.
fn next_record_error(run: &LeafRun<u64>) -> DecodeLeafError {
    run.records()
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
        DecodeLeafError::TrailingBytes { count } => {
            write!(out, "Record::TrailingBytes(count={count})").unwrap()
        }
    }
}

fn placement_witnesses() -> [(&'static str, Speaker, Stream, Frame<u8>); 3] {
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

fn encoded<T>(speaker: Speaker, frame: WireFrame<T>) -> Vec<u8> {
    let mut bytes = Vec::new();
    encode(speaker, &frame, &mut bytes).unwrap();
    bytes
}

fn raw_supply(stream: Stream, flow: Flow, body: &[u8]) -> Vec<u8> {
    let signal = WireSignal::new(Speaker::Initiator, stream, Signal::Supply(flow))
        .unwrap()
        .to_byte();
    let mut encoded = vec![signal];
    encoded.extend_from_slice(&(body.len() as u32).to_be_bytes());
    encoded.extend_from_slice(body);
    encoded
}

fn frame_signal<T>(frame: &Frame<T>) -> Signal {
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
        DecodeErrorKind::InvalidRun(LeafRunError::TruncatedHeader { remaining }) => {
            write!(out, "InvalidRun::TruncatedHeader(remaining={remaining})").unwrap()
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

impl borsh::io::Write for FailAfterWriter {
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

impl borsh::io::Read for FailAfterReader {
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
