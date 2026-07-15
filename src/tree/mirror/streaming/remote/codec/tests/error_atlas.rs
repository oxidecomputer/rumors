//! Stable witnesses for every codec error reachable without resource exhaustion.

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
    EncodeLeafError, End, Flow, Frame, FramePart, FrameWrite, Reaction, Speaker, Stream, WireFrame,
    decode, decode_exact, encode,
    frame::QUERY_CHILD_LEN,
    signal::{Signal, WireSignal},
};
use crate::{Version, message::Message, tree::typed::Hash};

/// Interior stream used where both speakers admit every signal state.
const INTERIOR_STREAM: u8 = 8;

/// First reserved semantic-state byte.
const FIRST_RESERVED_SIGNAL: u8 = WireSignal::BYTE_COUNT;

/// A one-byte Version prefix whose gamma integer is incomplete.
const TRUNCATED_VERSION: &[u8] = &[1];

/// Every feasible typed failure pins its origin, fields, and source chain.
#[test]
fn codec_error_atlas_snapshot() {
    let mut atlas = String::new();
    encode_errors(&mut atlas);
    decode_errors(&mut atlas);
    insta::assert_snapshot!(atlas);
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
            Reaction::Supply(Version::new(), Message::new(7)),
            Flow::Continue,
        ),
    );

    for speaker in [Speaker::Initiator, Speaker::Responder] {
        for (label, frame, offset) in [
            ("write/signal", &query, 0),
            ("write/query-count", &query, 1),
            ("write/query-children", &query, 2),
            ("write/supply-length", &supply, 1),
            ("leaf/version", &supply, 5),
            ("leaf/message", &supply, 6),
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
                Reaction::Supply(Version::new(), Message::new(7_u8)),
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
        let error =
            decode::<u8>(speaker, &mut FailAfterReader::new(matched.clone(), 0)).unwrap_err();
        record_decode(atlas, &format!("{speaker:?}/read/signal"), &error);

        for (label, offset) in [("query-count", 1), ("query-children", 2)] {
            let error = decode::<u8>(speaker, &mut FailAfterReader::new(query.clone(), offset))
                .unwrap_err();
            record_decode(atlas, &format!("{speaker:?}/read/{label}"), &error);
        }
        for (label, offset) in [("supply-length", 1), ("supply-leaf", 5)] {
            let error = decode::<u8>(speaker, &mut FailAfterReader::new(supply.clone(), offset))
                .unwrap_err();
            record_decode(atlas, &format!("{speaker:?}/read/{label}"), &error);
        }

        for (label, bytes) in [
            ("signal", &[][..]),
            ("query-count", &query[..1]),
            ("query-children", &query[..2]),
            ("supply-length", &supply[..1]),
            ("supply-leaf", &supply[..5]),
        ] {
            let error = decode_exact::<u8>(speaker, bytes).unwrap_err();
            record_decode(atlas, &format!("{speaker:?}/truncated/{label}"), &error);
        }

        let error = decode_exact::<u8>(speaker, &[FIRST_RESERVED_SIGNAL]).unwrap_err();
        record_decode(atlas, &format!("{speaker:?}/reserved-signal"), &error);

        let mut unordered = query.clone();
        unordered[2] = 2;
        unordered[2 + QUERY_CHILD_LEN] = 1;
        let error = decode_exact::<u8>(speaker, &unordered).unwrap_err();
        record_decode(atlas, &format!("{speaker:?}/query-out-of-order"), &error);

        let error = decode_exact::<u64>(
            speaker,
            &raw_supply(stream, Flow::Continue, TRUNCATED_VERSION),
        )
        .unwrap_err();
        record_decode(atlas, &format!("{speaker:?}/leaf/version"), &error);

        let mut body = Vec::new();
        Version::new().serialize(&mut body).unwrap();
        let error =
            decode_exact::<u64>(speaker, &raw_supply(stream, Flow::Continue, &body)).unwrap_err();
        record_decode(atlas, &format!("{speaker:?}/leaf/message"), &error);

        0_u64.serialize(&mut body).unwrap();
        body.push(0);
        let error =
            decode_exact::<u64>(speaker, &raw_supply(stream, Flow::Continue, &body)).unwrap_err();
        record_decode(atlas, &format!("{speaker:?}/leaf/trailing"), &error);

        let mut trailing = matched.clone();
        trailing.push(0);
        let error = decode_exact::<u8>(speaker, &trailing).unwrap_err();
        record_decode(atlas, &format!("{speaker:?}/frame/trailing"), &error);
    }

    for (label, speaker, stream, frame) in placement_witnesses() {
        let signal = frame_signal(&frame);
        let invalid = WireSignal::new(speaker, stream, signal).unwrap_err();
        let error = decode_exact::<u8>(speaker, &[invalid.byte()]).unwrap_err();
        record_decode(atlas, &format!("{label}/decode"), &error);
    }
}

fn placement_witnesses() -> [(&'static str, Speaker, Stream, Frame<u8>); 3] {
    [
        (
            "placement/opening-question",
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
        Frame::Reaction(Reaction::Supply(_, _), flow) => Signal::Supply(*flow),
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

fn describe_encode_kind(out: &mut String, kind: &EncodeErrorKind) {
    match kind {
        EncodeErrorKind::Write { part, source } => {
            write!(out, "Write(part={part:?}, io={:?})", source.kind()).unwrap()
        }
        EncodeErrorKind::Flush(source) => write!(out, "Flush(io={:?})", source.kind()).unwrap(),
        EncodeErrorKind::InvalidLeaf(EncodeLeafError::Version(source)) => {
            write!(out, "InvalidLeaf::Version(io={:?})", source.kind()).unwrap()
        }
        EncodeErrorKind::InvalidLeaf(EncodeLeafError::Message(source)) => {
            write!(out, "InvalidLeaf::Message(io={:?})", source.kind()).unwrap()
        }
        EncodeErrorKind::SupplyLengthOverflow {
            version_len,
            message_len,
        } => write!(
            out,
            "SupplyLengthOverflow(version_len={version_len}, message_len={message_len})"
        )
        .unwrap(),
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
        DecodeErrorKind::InvalidLeaf(DecodeLeafError::Version(source)) => {
            write!(out, "InvalidLeaf::Version(io={:?})", source.kind()).unwrap()
        }
        DecodeErrorKind::InvalidLeaf(DecodeLeafError::Message(source)) => {
            write!(out, "InvalidLeaf::Message(io={:?})", source.kind()).unwrap()
        }
        DecodeErrorKind::InvalidLeaf(DecodeLeafError::TrailingBytes { count }) => {
            write!(out, "InvalidLeaf::TrailingBytes(count={count})").unwrap()
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
