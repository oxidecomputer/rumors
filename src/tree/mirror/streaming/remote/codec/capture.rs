//! Stable semantic rendering of captured V2 traffic.

use std::{collections::BTreeMap, fmt::Write as _};

use borsh::BorshDeserialize;

use crate::Version;

use super::{
    End, Speaker, Stream,
    frame::{QUERY_CHILD_LEN, QUERY_COUNT_BIAS},
    signal::{Signal, WireSignal},
};

/// Bytes occupied by the fixed session preamble.
const PREAMBLE_LEN: usize = 25;

/// Bytes occupied by one exact-frame length header.
const FRAME_LEN: usize = std::mem::size_of::<u32>();

/// Bytes occupied by a data stream's leading label: epoch, then stream index.
const LABEL_LEN: usize = 2;

/// Everything one endpoint sent during a captured session.
///
/// The link keeps logical streams physically separate, so a capture is
/// already demultiplexed: the control stream's exact bytes, plus each opened
/// data stream's exact bytes (label included), in open order.
pub struct LinkCapture {
    /// The control stream's outgoing bytes: preamble, causal-version frame,
    /// and any trailing party hand-off, in order.
    pub control: Vec<u8>,
    /// Each opened data stream's outgoing bytes: its two-byte label, then
    /// its frames through the explicit end control.
    pub streams: Vec<Vec<u8>>,
}

/// Render both endpoints' captures without retaining cross-stream order.
///
/// Control bytes remain byte-exact. Data streams are keyed by their labeled
/// stream index — exact bytes and order within each stream, stream groups
/// sorted — discarding the incidental order in which independent streams
/// were opened. Parsing accounts for every captured byte once.
pub fn render_v2_capture(a: &LinkCapture, b: &LinkCapture) -> String {
    let a_control = Control::parse(&a.control);
    let b_control = Control::parse(&b.control);

    let (a_streams, b_streams) = match (&a_control.version, &b_control.version) {
        (None, None) => (None, None),
        (Some(a_version), Some(b_version)) if a_version == b_version => {
            assert!(
                a.streams.is_empty() && b.streams.is_empty(),
                "equal versions open no data streams",
            );
            (None, None)
        }
        (Some(a_version), Some(b_version)) => {
            let a_speaker = match b_version.as_bytes().cmp(a_version.as_bytes()) {
                std::cmp::Ordering::Less => Speaker::Initiator,
                std::cmp::Ordering::Greater => Speaker::Responder,
                std::cmp::Ordering::Equal => unreachable!("equal versions handled above"),
            };
            (
                Some(Streams::parse(a_speaker, &a.streams)),
                Some(Streams::parse(a_speaker.other(), &b.streams)),
            )
        }
        _ => panic!("both directions must either carry or omit a version frame"),
    };

    let mut rendered = String::new();
    render_direction("A -> B", &a_control, a_streams.as_ref(), &mut rendered);
    rendered.push('\n');
    render_direction("B -> A", &b_control, b_streams.as_ref(), &mut rendered);
    rendered
}

/// The control stream's fixed prefix, optional version frame, and trailing
/// session bytes.
struct Control {
    preamble: Vec<u8>,
    version_frame: Option<Vec<u8>>,
    version: Option<Version>,
    trailing: Vec<u8>,
}

impl Control {
    /// Split one captured control direction at its exact fixed boundaries.
    fn parse(bytes: &[u8]) -> Self {
        assert!(bytes.len() >= PREAMBLE_LEN, "capture omitted the preamble");
        let (preamble, rest) = bytes.split_at(PREAMBLE_LEN);
        if rest.is_empty() {
            return Self {
                preamble: preamble.to_vec(),
                version_frame: None,
                version: None,
                trailing: Vec::new(),
            };
        }

        assert!(rest.len() >= FRAME_LEN, "truncated version frame header");
        let len = u32::from_be_bytes(rest[..FRAME_LEN].try_into().expect("header width")) as usize;
        let frame_end = FRAME_LEN + len;
        assert!(rest.len() >= frame_end, "truncated version frame");
        let version = Version::try_from_slice(&rest[FRAME_LEN..frame_end])
            .expect("captured version frame is canonical");
        Self {
            preamble: preamble.to_vec(),
            version_frame: Some(rest[..frame_end].to_vec()),
            version: Some(version),
            trailing: rest[frame_end..].to_vec(),
        }
    }
}

/// One direction's exact data streams, keyed by their labeled stream index.
struct Streams {
    speaker: Speaker,
    streams: BTreeMap<Stream, CapturedStream>,
}

/// One captured data stream: its label and its exact frames.
struct CapturedStream {
    epoch: u8,
    frames: Vec<CapturedFrame>,
}

impl Streams {
    /// Decode every captured stream through its explicit end control.
    fn parse(speaker: Speaker, streams: &[Vec<u8>]) -> Self {
        let mut parsed = BTreeMap::new();
        for bytes in streams {
            assert!(
                bytes.len() >= LABEL_LEN,
                "captured stream omitted its label"
            );
            let (label, mut rest) = bytes.split_at(LABEL_LEN);
            let [epoch, index] = label.try_into().expect("label width");
            let labeled = Stream::new(index).expect("captured label names a logical stream");

            let mut frames = Vec::new();
            let mut ended = false;
            while !ended {
                let (stream, signal, consumed) = raw_frame(speaker, rest);
                assert_eq!(stream, labeled, "captured frame contradicts its label");
                ended = matches!(signal, Signal::End(End::Stream));
                frames.push(CapturedFrame {
                    semantic: format!("{signal:?}"),
                    bytes: rest[..consumed].to_vec(),
                });
                rest = &rest[consumed..];
            }
            assert!(rest.is_empty(), "captured bytes after the stream end");
            let previous = parsed.insert(labeled, CapturedStream { epoch, frames });
            assert!(previous.is_none(), "duplicate captured stream label");
        }
        Self {
            speaker,
            streams: parsed,
        }
    }
}

/// Parse one honest frame's boundary without decoding its supplied payload.
fn raw_frame(speaker: Speaker, bytes: &[u8]) -> (Stream, Signal, usize) {
    let (&byte, body) = bytes.split_first().expect("captured stream ended early");
    let (stream, signal) = WireSignal::from_byte(speaker, byte)
        .expect("captured signal is valid")
        .into_parts();
    let body_len = match signal {
        Signal::Match(_) | Signal::QueryEmpty(_) | Signal::End(_) => 0,
        Signal::Query(_) => {
            let (&count, _) = body.split_first().expect("captured query has a count");
            1 + (usize::from(count) + QUERY_COUNT_BIAS) * QUERY_CHILD_LEN
        }
        Signal::Supply(_) => {
            assert!(body.len() >= FRAME_LEN, "captured supply has a length");
            let len = u32::from_be_bytes(body[..FRAME_LEN].try_into().expect("header width"));
            FRAME_LEN + len as usize
        }
    };
    let consumed = 1 + body_len;
    assert!(bytes.len() >= consumed, "captured frame is truncated");
    (stream, signal, consumed)
}

/// One semantically decoded frame and the exact bytes which produced it.
struct CapturedFrame {
    semantic: String,
    bytes: Vec<u8>,
}

/// Render one physical direction in stable logical order.
fn render_direction(label: &str, control: &Control, streams: Option<&Streams>, out: &mut String) {
    writeln!(out, "direction {label}").unwrap();
    render_block("preamble", &control.preamble, out);
    if let Some(version) = &control.version {
        writeln!(out, "version: {version}").unwrap();
        render_block(
            "version frame",
            control.version_frame.as_deref().expect("version frame"),
            out,
        );
    }

    if let Some(streams) = streams {
        for (stream, captured) in &streams.streams {
            writeln!(
                out,
                "{:?} stream {} (height {}), epoch {}",
                streams.speaker,
                stream.index(),
                stream.height(streams.speaker),
                captured.epoch,
            )
            .unwrap();
            for (index, frame) in captured.frames.iter().enumerate() {
                writeln!(out, "  frame {index}: {}", frame.semantic).unwrap();
                render_hex(&frame.bytes, "    ", out);
            }
        }
    }
    if !control.trailing.is_empty() {
        render_block("trailing frame", &control.trailing, out);
    }
}

/// Render one named exact byte block.
fn render_block(label: &str, bytes: &[u8], out: &mut String) {
    writeln!(out, "{label}: {} bytes", bytes.len()).unwrap();
    render_hex(bytes, "  ", out);
}

/// Render stable eight-byte hexdump lines with a caller-selected indent.
fn render_hex(bytes: &[u8], indent: &str, out: &mut String) {
    for (line, chunk) in bytes.chunks(8).enumerate() {
        write!(out, "{indent}{:04x}:", line * 8).unwrap();
        for byte in chunk {
            write!(out, " {byte:02x}").unwrap();
        }
        out.push('\n');
    }
}
