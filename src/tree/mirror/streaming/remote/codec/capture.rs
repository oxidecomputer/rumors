//! Stable semantic rendering of captured V2 traffic.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
};

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

/// Render both physical V2 directions without retaining cross-stream order.
///
/// Preamble and causal-version frames remain byte-exact. Streaming frames are
/// grouped by logical stream, retaining exact bytes and order within each
/// stream while sorting the stream groups. Any trailing party hand-off remains
/// a final exact byte block. Parsing accounts for every captured byte once.
pub fn render_v2_capture(a_to_b: &[u8], b_to_a: &[u8]) -> String {
    let a = Direction::parse(a_to_b);
    let b = Direction::parse(b_to_a);

    let (a_streams, b_streams) = match (&a.version, &b.version) {
        (None, None) => (None, None),
        (Some(a_version), Some(b_version)) if a_version == b_version => (None, None),
        (Some(a_version), Some(b_version)) => {
            let a_speaker = match b_version.as_bytes().cmp(a_version.as_bytes()) {
                std::cmp::Ordering::Less => Speaker::Initiator,
                std::cmp::Ordering::Greater => Speaker::Responder,
                std::cmp::Ordering::Equal => unreachable!("equal versions handled above"),
            };
            (
                Some(Streams::parse(a_speaker, &a.body)),
                Some(Streams::parse(a_speaker.other(), &b.body)),
            )
        }
        _ => panic!("both directions must either carry or omit a version frame"),
    };

    let mut rendered = String::new();
    render_direction("A -> B", &a, a_streams.as_ref(), &mut rendered);
    rendered.push('\n');
    render_direction("B -> A", &b, b_streams.as_ref(), &mut rendered);
    rendered
}

/// The fixed prefix, optional version frame, and remaining session bytes.
struct Direction {
    preamble: Vec<u8>,
    version_frame: Option<Vec<u8>>,
    version: Option<Version>,
    body: Vec<u8>,
}

impl Direction {
    /// Split one captured physical direction at its exact fixed boundaries.
    fn parse(bytes: &[u8]) -> Self {
        assert!(bytes.len() >= PREAMBLE_LEN, "capture omitted the preamble");
        let (preamble, rest) = bytes.split_at(PREAMBLE_LEN);
        if rest.is_empty() {
            return Self {
                preamble: preamble.to_vec(),
                version_frame: None,
                version: None,
                body: Vec::new(),
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
            body: rest[frame_end..].to_vec(),
        }
    }
}

/// Exact frames grouped by their logical stream, plus bytes after streaming.
struct Streams {
    speaker: Speaker,
    streams: BTreeMap<Stream, Vec<CapturedFrame>>,
    trailing: Vec<u8>,
}

impl Streams {
    /// Decode until every logical stream has emitted its stream-end control.
    fn parse(speaker: Speaker, bytes: &[u8]) -> Self {
        let mut rest = bytes;
        let mut streams: BTreeMap<Stream, Vec<CapturedFrame>> = BTreeMap::new();
        let mut ended = BTreeSet::new();

        while ended.len() < usize::from(Stream::COUNT) {
            let (stream, signal, consumed) = raw_frame(speaker, rest);
            let is_stream_end = matches!(signal, Signal::End(End::Stream));
            streams.entry(stream).or_default().push(CapturedFrame {
                semantic: format!("{signal:?}"),
                bytes: rest[..consumed].to_vec(),
            });
            rest = &rest[consumed..];
            if is_stream_end {
                assert!(ended.insert(stream), "duplicate captured stream end");
            }
        }

        Self {
            speaker,
            streams,
            trailing: rest.to_vec(),
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
fn render_direction(
    label: &str,
    direction: &Direction,
    streams: Option<&Streams>,
    out: &mut String,
) {
    writeln!(out, "direction {label}").unwrap();
    render_block("preamble", &direction.preamble, out);
    if let Some(version) = &direction.version {
        writeln!(out, "version: {version}").unwrap();
        render_block(
            "version frame",
            direction.version_frame.as_deref().expect("version frame"),
            out,
        );
    }

    if let Some(streams) = streams {
        for (stream, frames) in &streams.streams {
            writeln!(
                out,
                "{:?} stream {} (height {})",
                streams.speaker,
                stream.index(),
                stream.height(streams.speaker),
            )
            .unwrap();
            for (index, frame) in frames.iter().enumerate() {
                writeln!(out, "  frame {index}: {}", frame.semantic).unwrap();
                render_hex(&frame.bytes, "    ", out);
            }
        }
        if !streams.trailing.is_empty() {
            render_block("trailing frame", &streams.trailing, out);
        }
    } else if !direction.body.is_empty() {
        render_block("trailing frame", &direction.body, out);
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
