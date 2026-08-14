//! Stable semantic rendering of captured V2 traffic.
//!
//! Every frame renders its exact bytes AND the parse tree those bytes
//! decode to — the greeting's size words, the root-fan listing's
//! children, a query's `(radix, hash)` children, and a supply run's
//! per-record versions with each message's byte count (the leaf payload
//! type is the caller's, so message bytes stay exact-but-opaque) — so a
//! snapshot re-accept diff names the semantic field that moved instead
//! of asking a reviewer to diff hex. Payload bytes that do not decode
//! render as an explicit failure line above their hex; they never pass
//! as silent hex. Structural violations of the capture itself (a
//! truncated frame, a mislabeled stream) stay panics: they mean the
//! capture harness, not the peer, is broken.

use std::{collections::BTreeMap, fmt::Write as _};

use borsh::BorshDeserialize;

use crate::Version;
use crate::tree::mirror::framing::{GREETING_WORD_LEN, LENGTH_HEADER_LEN};
use crate::tree::mirror::streaming::message::initiates;
use crate::tree::typed::Hash;

use super::{
    End, Speaker, Stream,
    frame::{LeafRun, QUERY_CHILD_LEN, QUERY_COUNT_BIAS},
    signal::{Signal, WireSignal},
};

#[cfg(test)]
mod tests;

/// Bytes occupied by the fixed session preamble.
const PREAMBLE_LEN: usize = 25;

// The label's width is defined canonically beside the sender that writes
// it; captures parse with the same constant.
use super::super::streams::LABEL_LEN;

/// Everything one endpoint sent during a captured session.
///
/// The link keeps logical streams physically separate, so a capture is
/// already demultiplexed: the control stream's exact bytes, plus each opened
/// data stream's exact bytes (label included), in open order.
pub struct LinkCapture {
    /// The control stream's outgoing bytes: preamble, the greeting's
    /// causal-version and root-fan listing frames, and any trailing party
    /// hand-off, in order.
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
            // Mirror the session's role election: the smaller advertised
            // set initiates, canonical version bytes break ties.
            let a_len = a_control
                .set_len
                .expect("a version frame carries its set size");
            let b_len = b_control
                .set_len
                .expect("a version frame carries its set size");
            let a_speaker = if initiates(a_len, a_version, b_len, b_version) {
                Speaker::Initiator
            } else {
                Speaker::Responder
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

/// The control stream's fixed prefix, optional greeting frames, and trailing
/// session bytes.
struct Control {
    preamble: Vec<u8>,
    version_frame: Option<Vec<u8>>,
    version: Option<Version>,
    /// The version frame's leading word: the sender's advertised set size,
    /// the role election's primary key.
    set_len: Option<u64>,
    /// The version frame's remaining size words: the sender's
    /// version-size bound and target message size.
    max_version_bytes: Option<u64>,
    target_message_size: Option<u64>,
    /// The greeting's second frame: the sender's root-fan listing.
    listing_frame: Option<Vec<u8>>,
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
                set_len: None,
                max_version_bytes: None,
                target_message_size: None,
                listing_frame: None,
                trailing: Vec::new(),
            };
        }

        // A session that ends before its causal greeting (a mutual retire
        // declining at the preamble) still closes with the one-byte session
        // epilogue marker: control bytes too short to be a version frame
        // header are that trailing marker, not a truncated frame.
        if rest.len() < LENGTH_HEADER_LEN {
            return Self {
                preamble: preamble.to_vec(),
                version_frame: None,
                version: None,
                set_len: None,
                max_version_bytes: None,
                target_message_size: None,
                listing_frame: None,
                trailing: rest.to_vec(),
            };
        }
        let (version_frame, rest) = split_frame(rest, "version");
        // The version frame's body leads with the sender's eight-byte set
        // size, version-size bound, and message-size target; the version
        // encoding follows them.
        let word = |index: usize| {
            let at = LENGTH_HEADER_LEN + index * GREETING_WORD_LEN;
            u64::from_le_bytes(
                version_frame[at..at + GREETING_WORD_LEN]
                    .try_into()
                    .expect("captured version frame carries its three size words"),
            )
        };
        let set_len = word(0);
        let max_version_bytes = word(1);
        let target_message_size = word(2);
        let version = Version::try_from_slice(
            &version_frame
                [LENGTH_HEADER_LEN + crate::tree::mirror::framing::GREETING_SIZE_WORDS_LEN..],
        )
        .expect("captured version frame is canonical");
        // The greeting always carries its listing frame directly behind the
        // version frame (empty tree = empty listing, still framed).
        let (listing_frame, rest) = split_frame(rest, "listing");
        Self {
            preamble: preamble.to_vec(),
            version_frame: Some(version_frame),
            version: Some(version),
            set_len: Some(set_len),
            max_version_bytes: Some(max_version_bytes),
            target_message_size: Some(target_message_size),
            listing_frame: Some(listing_frame),
            trailing: rest.to_vec(),
        }
    }
}

/// Split one exact length-delimited frame (header included) off `bytes`.
fn split_frame<'a>(bytes: &'a [u8], what: &str) -> (Vec<u8>, &'a [u8]) {
    assert!(
        bytes.len() >= LENGTH_HEADER_LEN,
        "truncated {what} frame header"
    );
    let len =
        u32::from_be_bytes(bytes[..LENGTH_HEADER_LEN].try_into().expect("header width")) as usize;
    let frame_end = LENGTH_HEADER_LEN + len;
    assert!(bytes.len() >= frame_end, "truncated {what} frame");
    (bytes[..frame_end].to_vec(), &bytes[frame_end..])
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
                    payload: payload_lines(&signal, &rest[..consumed]),
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
            assert!(
                body.len() >= LENGTH_HEADER_LEN,
                "captured supply has a length"
            );
            let len =
                u32::from_be_bytes(body[..LENGTH_HEADER_LEN].try_into().expect("header width"));
            LENGTH_HEADER_LEN + len as usize
        }
    };
    let consumed = 1 + body_len;
    assert!(bytes.len() >= consumed, "captured frame is truncated");
    (stream, signal, consumed)
}

/// One semantically decoded frame and the exact bytes which produced it.
struct CapturedFrame {
    semantic: String,
    /// The frame's decoded payload tree (or its explicit decode
    /// failure), one rendered line per entry; empty for payload-free
    /// frames.
    payload: Vec<String>,
    bytes: Vec<u8>,
}

/// Render a byte string as bare lowercase hex, for hash and field lines.
fn hex_string(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Decode one captured frame's peer-supplied payload into rendered lines.
///
/// Match, empty-query, and end frames carry no payload. A query decodes
/// to its `(radix, hash)` children and a supply to its leaf-record run;
/// payload bytes that do not decode render as an explicit failure line —
/// the hex below them then stands as the witness, never as the only
/// account.
fn payload_lines(signal: &Signal, frame: &[u8]) -> Vec<String> {
    match signal {
        Signal::Match(_) | Signal::QueryEmpty(_) | Signal::End(_) => Vec::new(),
        // Signal byte, count byte, then the exact children — the frame
        // boundary already validated the arithmetic.
        Signal::Query(_) => query_lines(&frame[2..]),
        // Signal byte and length header, then the run body.
        Signal::Supply(_) => supply_lines(frame[1 + LENGTH_HEADER_LEN..].to_vec()),
    }
}

/// Render a nonempty query's children: each child's radix and hash.
fn query_lines(children: &[u8]) -> Vec<String> {
    let mut lines = vec![format!(
        "query: {} child(ren)",
        children.len() / QUERY_CHILD_LEN
    )];
    for child in children.chunks(QUERY_CHILD_LEN) {
        lines.push(format!(
            "  child 0x{:x}: {}",
            child[0],
            hex_string(&child[1..]),
        ));
    }
    lines
}

/// Render a supply frame's leaf-record run: each record's version and
/// its message's byte count.
///
/// The message payload type belongs to the caller, so message bytes are
/// counted, not decoded — they remain exact in the hex below. A run
/// whose record framing or version encoding does not decode renders the
/// failure explicitly.
fn supply_lines(run: Vec<u8>) -> Vec<String> {
    let run = match LeafRun::<()>::from_encoded(run) {
        Ok(run) => run,
        Err(err) => {
            return vec![format!(
                "supply run undecodable ({err}); the exact bytes stand below"
            )];
        }
    };
    let mut lines = vec![format!("supply run: {} record(s)", run.record_count())];
    for (index, record) in run.record_slices().enumerate() {
        let mut input = record;
        match Version::deserialize(&mut input) {
            Ok(version) => lines.push(format!(
                "  record {index}: version {version}, message {} byte(s)",
                input.len(),
            )),
            Err(err) => lines.push(format!(
                "  record {index} undecodable ({err}); the exact bytes stand below"
            )),
        }
    }
    lines
}

/// Render one root-fan listing frame's children, or its explicit decode
/// failure (the listing is peer-controlled borsh, so the renderer must
/// never present undecodable bytes as a quietly hex-only frame).
fn listing_lines(body: &[u8]) -> Vec<String> {
    match <Vec<(u8, Hash)>>::try_from_slice(body) {
        Ok(children) => {
            let mut lines = vec![format!("listing: {} child(ren)", children.len())];
            for (radix, hash) in &children {
                lines.push(format!("  child 0x{radix:x}: {}", hex_string(&hash.0)));
            }
            lines
        }
        Err(err) => vec![format!(
            "listing undecodable ({err}); the exact bytes stand below"
        )],
    }
}

/// Render one physical direction in stable logical order.
fn render_direction(label: &str, control: &Control, streams: Option<&Streams>, out: &mut String) {
    writeln!(out, "direction {label}").unwrap();
    render_block("preamble", &control.preamble, out);
    if let Some(version) = &control.version {
        writeln!(out, "version: {version}").unwrap();
        writeln!(
            out,
            "greeting words: set len {}, version-size bound {}, message-size target {}",
            control
                .set_len
                .expect("a version frame carries its set size"),
            control
                .max_version_bytes
                .expect("a version frame carries its version-size bound"),
            control
                .target_message_size
                .expect("a version frame carries its message-size target"),
        )
        .unwrap();
        render_block(
            "version frame",
            control.version_frame.as_deref().expect("version frame"),
            out,
        );
        let listing_frame = control.listing_frame.as_deref().expect("listing frame");
        for line in listing_lines(&listing_frame[LENGTH_HEADER_LEN..]) {
            writeln!(out, "{line}").unwrap();
        }
        render_block("listing frame", listing_frame, out);
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
                for line in &frame.payload {
                    writeln!(out, "    {line}").unwrap();
                }
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
