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

use crate::Version;
use crate::tree::mirror::cbor::{self, MAJOR_BSTR, MAJOR_TAG, MAJOR_UINT, TAG_EMBEDDED_ITEM};
use crate::tree::mirror::handshake::V2_PREAMBLE_LEN;
use crate::tree::mirror::streaming::message::{Greeting, initiates};

use super::{
    End, Speaker, Stream,
    frame::{LeafRun, parse_listing_map},
    greeting::parse_greeting,
    signal::{Signal, WireSignal},
};

#[cfg(test)]
mod tests;

/// Everything one endpoint sent during a captured session.
///
/// The link keeps logical streams physically separate, so a capture is
/// already demultiplexed: the control stream's exact bytes, plus each opened
/// data stream's exact bytes (label included), in open order.
pub struct LinkCapture {
    /// The control stream's outgoing bytes: preamble, the greeting item,
    /// and any trailing party hand-off and epilogue, in order.
    pub control: Vec<u8>,
    /// Each opened data stream's outgoing bytes: its label items, then
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

    let (a_streams, b_streams) = match (&a_control.greeting, &b_control.greeting) {
        (None, None) => (None, None),
        (Some((_, a_greeting)), Some((_, b_greeting)))
            if a_greeting.version == b_greeting.version =>
        {
            assert!(
                a.streams.is_empty() && b.streams.is_empty(),
                "equal versions open no data streams",
            );
            (None, None)
        }
        (Some((_, a_greeting)), Some((_, b_greeting))) => {
            // Mirror the session's role election: the smaller advertised
            // set initiates, canonical version bytes break ties.
            let a_speaker = if initiates(
                a_greeting.set_len,
                &a_greeting.version,
                b_greeting.set_len,
                &b_greeting.version,
            ) {
                Speaker::Initiator
            } else {
                Speaker::Responder
            };
            (
                Some(Streams::parse(a_speaker, &a.streams)),
                Some(Streams::parse(a_speaker.other(), &b.streams)),
            )
        }
        _ => panic!("both directions must either carry or omit a greeting"),
    };

    let mut rendered = String::new();
    render_direction("A -> B", &a_control, a_streams.as_ref(), &mut rendered);
    rendered.push('\n');
    render_direction("B -> A", &b_control, b_streams.as_ref(), &mut rendered);
    rendered
}

/// The control stream's fixed prefix, optional greeting item, and trailing
/// session bytes.
struct Control {
    preamble: Vec<u8>,
    /// The greeting item's exact bytes and its decoded form.
    greeting: Option<(Vec<u8>, Greeting)>,
    trailing: Vec<u8>,
}

impl Control {
    /// Split one captured control direction at its exact item boundaries.
    fn parse(bytes: &[u8]) -> Self {
        assert!(
            bytes.len() >= V2_PREAMBLE_LEN,
            "capture omitted the preamble"
        );
        let (preamble, rest) = bytes.split_at(V2_PREAMBLE_LEN);
        // A greeting item opens with the embedded-item tag; anything else
        // after the preamble (a session ending before its greeting — a
        // mutual retire declining — closes with just the epilogue item) is
        // trailing.
        let mut probe = rest;
        let is_greeting = matches!(
            cbor::read_head(&mut probe),
            Ok(cbor::Head {
                major: MAJOR_TAG,
                value: TAG_EMBEDDED_ITEM,
            })
        );
        if !is_greeting {
            return Self {
                preamble: preamble.to_vec(),
                greeting: None,
                trailing: rest.to_vec(),
            };
        }
        let head = cbor::read_head(&mut probe).expect("captured greeting has a byte-string head");
        assert_eq!(
            head.major, MAJOR_BSTR,
            "captured greeting wraps a byte string"
        );
        let len = usize::try_from(head.value).expect("captured greeting fits in memory");
        let consumed = rest.len() - probe.len() + len;
        assert!(rest.len() >= consumed, "truncated greeting item");
        let (item, trailing) = rest.split_at(consumed);
        let greeting = parse_greeting(&probe[..len]).expect("captured greeting is canonical");
        Self {
            preamble: preamble.to_vec(),
            greeting: Some((item.to_vec(), greeting)),
            trailing: trailing.to_vec(),
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
            let mut rest = bytes.as_slice();
            let epoch = label_item(&mut rest, "epoch");
            let epoch = u8::try_from(epoch).expect("captured epoch is a byte-ranged counter");
            let index = label_item(&mut rest, "stream index");
            let labeled = Stream::new(u8::try_from(index).expect("captured label is byte-ranged"))
                .expect("captured label names a logical stream");

            let mut frames = Vec::new();
            let mut ended = false;
            while !ended {
                let (stream, signal, consumed) = raw_frame(speaker, rest);
                assert_eq!(stream, labeled, "captured frame contradicts its label");
                ended = matches!(signal, Signal::End(End::Stream));
                frames.push(CapturedFrame {
                    semantic: format!("{signal:?}"),
                    payload: payload_lines(speaker, &rest[..consumed]),
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

/// Read one label item: a canonical unsigned int.
fn label_item(rest: &mut &[u8], what: &str) -> u64 {
    let head = cbor::read_head(rest)
        .unwrap_or_else(|e| panic!("captured stream label {what} is canonical: {e}"));
    assert_eq!(
        head.major, MAJOR_UINT,
        "captured label {what} is an unsigned int"
    );
    head.value
}

/// Parse one honest frame's boundary without decoding its supplied payload.
fn raw_frame(speaker: Speaker, bytes: &[u8]) -> (Stream, Signal, usize) {
    let mut rest = bytes;
    let head = cbor::read_head(&mut rest).expect("captured frame head is canonical");
    assert_eq!(head.major, cbor::MAJOR_ARRAY, "captured frame is an array");
    let head = cbor::read_head(&mut rest).expect("captured signal is canonical");
    assert_eq!(head.major, MAJOR_UINT, "captured signal is an unsigned int");
    let code = u8::try_from(head.value).expect("captured signal is in the dense code space");
    let (stream, signal) = WireSignal::from_byte(speaker, code)
        .expect("captured signal is valid")
        .into_parts();
    match signal {
        Signal::Match(_) | Signal::QueryEmpty(_) | Signal::End(_) => {}
        Signal::Query(_) => {
            // Walking the listing map through the codec's own parser both
            // finds the frame boundary and validates canonical form.
            parse_listing_map(&mut rest).expect("captured query listing is canonical");
        }
        Signal::Supply(_) => {
            let head = cbor::read_head(&mut rest).expect("captured run tag is canonical");
            assert_eq!(
                (head.major, head.value),
                (MAJOR_TAG, cbor::TAG_CBOR_SEQUENCE),
                "captured supply opens with the embedded-sequence tag"
            );
            let head = cbor::read_head(&mut rest).expect("captured run head is canonical");
            assert_eq!(head.major, MAJOR_BSTR, "captured run is a byte string");
            let len = usize::try_from(head.value).expect("captured run fits in memory");
            assert!(rest.len() >= len, "captured frame is truncated");
            rest = &rest[len..];
        }
    }
    let consumed = bytes.len() - rest.len();
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

/// Decode one captured frame's peer-supplied payload into rendered lines.
///
/// Match, empty-query, and end frames carry no payload. A query decodes
/// to its `(radix, hash)` children and a supply to its leaf-record run;
/// payload bytes that do not decode render as an explicit failure line —
/// the hex below them then stands as the witness, never as the only
/// account.
fn payload_lines(speaker: Speaker, frame: &[u8]) -> Vec<String> {
    // Skip the frame's array and signal heads, re-reading them through
    // the head grammar so the offsets cannot drift from `raw_frame`.
    let mut rest = frame;
    cbor::read_head(&mut rest).expect("frame head validated by raw_frame");
    let signal = cbor::read_head(&mut rest).expect("signal validated by raw_frame");
    let code = u8::try_from(signal.value).expect("signal range validated by raw_frame");
    let (_, signal) = WireSignal::from_byte(speaker, code)
        .expect("signal validated by raw_frame")
        .into_parts();
    match signal {
        Signal::Match(_) | Signal::QueryEmpty(_) | Signal::End(_) => Vec::new(),
        Signal::Query(_) => query_lines(rest),
        Signal::Supply(_) => {
            let mut heads = rest;
            cbor::read_head(&mut heads).expect("run tag validated by raw_frame");
            cbor::read_head(&mut heads).expect("run head validated by raw_frame");
            supply_lines(heads.to_vec())
        }
    }
}

/// Render a nonempty query's children: each child's radix and hash,
/// decoded through the codec's own listing parser (canonical child order
/// included), so the renderer cannot drift from what the decoder
/// accepts.
fn query_lines(mut children: &[u8]) -> Vec<String> {
    let children = match parse_listing_map(&mut children) {
        Ok(children) => children,
        Err(err) => {
            return vec![format!(
                "query undecodable ({err}); the exact bytes stand below"
            )];
        }
    };
    let mut lines = vec![format!("query: {} child(ren)", children.len())];
    for (radix, hash) in &children {
        lines.push(format!("  child 0x{radix:x}: {}", hex::encode(hash.0)));
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
    let run = match LeafRun::from_encoded(run) {
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
        let tagged = matches!(
            cbor::read_head(&mut input),
            Ok(cbor::Head {
                major: MAJOR_TAG,
                value,
            }) if value == crate::tags::VERSION_TAG
        );
        if !tagged {
            lines.push(format!(
                "  record {index} does not open with the version-atom tag; the exact bytes stand below"
            ));
            continue;
        }
        match ciborium::de::from_reader::<Version, _>(&mut input) {
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

/// Render one root-fan listing's children.
///
/// The canonical child order was already held by the codec's own listing
/// parser when the greeting decoded; these lines render the result.
fn listing_lines(listing: &[(u8, crate::tree::typed::Hash)]) -> Vec<String> {
    let mut lines = vec![format!("listing: {} child(ren)", listing.len())];
    for (radix, hash) in listing {
        lines.push(format!("  child 0x{radix:x}: {}", hex::encode(hash.0)));
    }
    lines
}

/// Render one physical direction in stable logical order.
fn render_direction(label: &str, control: &Control, streams: Option<&Streams>, out: &mut String) {
    writeln!(out, "direction {label}").unwrap();
    render_block("preamble", &control.preamble, out);
    if let Some((item, greeting)) = &control.greeting {
        writeln!(out, "version: {}", greeting.version).unwrap();
        writeln!(
            out,
            "greeting words: set len {}, version-size bound {}, message-size target {}",
            greeting.set_len, greeting.max_version_bytes, greeting.target_message_size,
        )
        .unwrap();
        for line in listing_lines(&greeting.listing) {
            writeln!(out, "{line}").unwrap();
        }
        render_block("greeting frame", item, out);
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
