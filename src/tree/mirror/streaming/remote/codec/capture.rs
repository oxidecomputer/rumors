//! Rendering of captured V2 traffic in CBOR diagnostic notation.
//!
//! The snapshot suites pin every wire byte of a captured session, and
//! this module is the form that pin takes: the harness's framing (each
//! direction, each control item and data frame with its index, exact
//! byte count, and protocol-phase label) over each item rendered by
//! `cbor-diag` in extended diagnostic notation with encoding
//! indicators, embedded CBOR (tags 24 and 63) unfolded, verbatim under
//! its header.
//!
//! # Why a rendering with no hexdump is still a byte pin
//!
//! The harness holds every item to canonical form before rendering it:
//! every head at its shortest width, definite lengths, well-formed
//! simple values, no NaN, text free of control characters, and a
//! re-encoding equal to the bytes, checked down through embedded CBOR
//! to the printer's depth limit. On a canonical item, diagnostic
//! notation with encoding indicators spells the value exactly, so two
//! different canonical byte streams cannot render identically; any other
//! item renders as an explicit failure line carrying the reason above
//! its exact hex. The rendering is therefore injective on wire bytes. A
//! NaN's sign and payload bits are what the notation cannot spell, which
//! is why every NaN is rejected; none occurs on the wire, since the
//! protocol emits no floats and the payload contract's `Eq` bound
//! excludes float fields (see
//! [choosing a payload type](crate#choosing-a-payload-type)); a
//! hand-written `Eq` admitting NaN has declared its NaNs equal. A control
//! item's or frame's byte count is the observed item's length and a
//! stream's is the transport's; the totality witness
//! ([`assert_items_account_for`]) holds the two accounts equal.
//!
//! # Where the bytes come from
//!
//! Capture enters through the observation hook ([`crate::observe`]):
//! the harness records each directed stream's items as a
//! [`HookCapture`]. The transport-level bytes ([`LinkCapture`]) are the
//! totality oracle: per directed stream, the on-wire open label followed
//! by the observed items must reproduce the transport bytes exactly
//! ([`assert_items_account_for`], [`stream_label`]), which is what lets
//! a rendering of items pin wire bytes. Structural violations of the
//! capture itself (a control item whose opening head is not canonical,
//! a frame whose opener is not the codec's or contradicts its stream, a
//! label that does not parse) are panics: the harness, not the peer, is
//! broken.

use std::{collections::BTreeMap, fmt::Write as _};

use crate::observe::Role;
use cbor_diag::{DataItem, IntegerWidth, Simple};

use crate::tree::mirror::cbor::{
    self, MAJOR_ARRAY, MAJOR_TAG, MAJOR_TEXT, MAJOR_UINT, TAG_CBOR_SEQUENCE, TAG_EMBEDDED_ITEM,
};

use super::{Speaker, Stream, signal::WireSignal};

#[cfg(test)]
mod tests;

/// Everything one endpoint sent during a captured session, at the
/// transport level.
///
/// The link keeps logical streams physically separate, so a capture is
/// already demultiplexed: the control stream's exact bytes, plus each
/// opened data stream's exact bytes (label included), in open order.
/// The rendering consumes the hook's [`HookCapture`]; this transport
/// form is the totality oracle beside it, and the wire-legibility
/// property's raw material.
pub struct LinkCapture {
    /// The control stream's outgoing bytes: preamble, the greeting item,
    /// and any trailing party hand-off and epilogue, in order.
    pub control: Vec<u8>,
    /// Each opened data stream's outgoing bytes: its label items, then
    /// its frames through the explicit end control.
    pub streams: Vec<Vec<u8>>,
}

/// Everything one endpoint sent during a captured session, as the
/// observation hook delivered it: one byte buffer per CBOR item.
pub struct HookCapture {
    /// The role this side was elected, if the session held an election.
    pub role: Option<Role>,
    /// The control stream's sent items, in order.
    pub control: Vec<Vec<u8>>,
    /// The sent data streams, in any order; rendering sorts by index.
    pub streams: Vec<HookStream>,
}

/// One sent data stream, as observed through the hook plus the wire
/// facts the hook deliberately does not carry (the label's epoch and
/// the exact transport byte count).
pub struct HookStream {
    /// The stream's wire index, from the hook's stream identity.
    pub index: u8,
    /// The elected role that speaks this stream's frames.
    pub speaker: Role,
    /// The epoch carried by the stream's on-wire open label.
    pub epoch: u8,
    /// The stream's exact transport byte count, label included.
    pub wire_len: usize,
    /// The stream's frames, one CBOR item each, in stream order.
    pub items: Vec<Vec<u8>>,
}

/// Parse one data stream's on-wire open label: two canonical unsigned
/// int items, `(epoch, stream index)`.
///
/// Returns the label values and the label's byte length. Panics if the
/// label is not two canonical byte-ranged uints: the capture harness,
/// not the peer, is broken.
pub fn stream_label(bytes: &[u8]) -> ((u8, u8), usize) {
    let mut rest = bytes;
    let epoch = label_item(&mut rest, "epoch");
    let index = label_item(&mut rest, "stream index");
    ((epoch, index), bytes.len() - rest.len())
}

/// Read one label item: a canonical byte-ranged unsigned int.
fn label_item(rest: &mut &[u8], what: &str) -> u8 {
    let head = cbor::read_head(rest)
        .unwrap_or_else(|e| panic!("captured stream label {what} is canonical: {e}"));
    assert_eq!(
        head.major, MAJOR_UINT,
        "captured label {what} is an unsigned int"
    );
    u8::try_from(head.value).unwrap_or_else(|_| panic!("captured label {what} is byte-ranged"))
}

/// Assert that the concatenation of `items` reproduces `wire` exactly.
///
/// The totality witness that licenses rendering hook items as a pin of
/// wire bytes: every transport byte is some observed item's byte, once,
/// in order. Panics on any mismatch, naming the first divergence.
pub fn assert_items_account_for(items: &[Vec<u8>], wire: &[u8]) {
    let mut rest = wire;
    for (index, item) in items.iter().enumerate() {
        assert!(
            rest.len() >= item.len() && &rest[..item.len()] == item.as_slice(),
            "observed item {index} does not match the wire at offset {}",
            wire.len() - rest.len(),
        );
        rest = &rest[item.len()..];
    }
    assert!(
        rest.is_empty(),
        "{} wire byte(s) beyond the last observed item",
        rest.len(),
    );
}

/// Render both endpoints' hook captures without retaining cross-stream
/// order.
///
/// Data streams are keyed by their labeled stream index (exact items
/// and order within each stream, stream groups sorted), discarding the
/// incidental order in which independent streams were opened.
pub fn render_hook_capture(a: &HookCapture, b: &HookCapture) -> String {
    let mut rendered = String::new();
    render_direction("A -> B", a, &mut rendered);
    rendered.push('\n');
    render_direction("B -> A", b, &mut rendered);
    rendered
}

/// Render one direction: its control items, then its data streams in
/// stream-index order.
fn render_direction(label: &str, capture: &HookCapture, out: &mut String) {
    writeln!(out, "direction {label}").unwrap();
    if let Some(role) = capture.role {
        writeln!(out, "role: {role:?}").unwrap();
    }
    for (index, item) in capture.control.iter().enumerate() {
        let name = control_item_name(item);
        writeln!(
            out,
            "control item {index} ({} bytes) / {name} /",
            item.len()
        )
        .unwrap();
        render_item(item, out);
    }

    let mut streams = BTreeMap::new();
    for stream in &capture.streams {
        let previous = streams.insert(stream.index, stream);
        assert!(previous.is_none(), "duplicate captured stream index");
    }
    for stream in streams.values() {
        let speaker = speaker(stream.speaker);
        let wire_stream = Stream::new(stream.index).expect("hook stream index names a stream");
        writeln!(
            out,
            "{:?} stream {} (height {}), epoch {}, {} wire bytes",
            speaker,
            stream.index,
            wire_stream.height(speaker),
            stream.epoch,
            stream.wire_len,
        )
        .unwrap();
        for (index, item) in stream.items.iter().enumerate() {
            render_frame(speaker, wire_stream, index, item, out);
        }
    }
}

/// The elected role, in the codec's speaker vocabulary.
fn speaker(role: Role) -> Speaker {
    match role {
        Role::Initiator => Speaker::Initiator,
        Role::Responder => Speaker::Responder,
    }
}

/// Name one control item by its shape.
///
/// The control stream's items are position- and shape-determined: the
/// self-described tag opens the preamble, the embedded-item tag wraps
/// the greeting, a tagged party atom is the identity hand-off, and the
/// dot text item is the epilogue.
fn control_item_name(item: &[u8]) -> &'static str {
    let mut probe = item;
    let Ok(head) = cbor::read_head(&mut probe) else {
        panic!("captured control item opens with a canonical head");
    };
    match (head.major, head.value) {
        (MAJOR_TAG, cbor::TAG_SELF_DESCRIBED) => "preamble",
        (MAJOR_TAG, TAG_EMBEDDED_ITEM) => "greeting",
        (MAJOR_TAG, crate::tags::PARTY_TAG) => "party hand-off",
        (MAJOR_TEXT, _) => "epilogue",
        _ => panic!("captured control item has no known shape"),
    }
}

/// Render one data frame: a header line with the frame's index, exact
/// byte count, and the signal its opener names, then the frame item in
/// diagnostic notation.
///
/// The frame grammar (the array head, the opener's stream and state
/// items) is held to panics: a violation means the capture is broken.
fn render_frame(speaker: Speaker, stream: Stream, index: usize, item: &[u8], out: &mut String) {
    let mut probe = item;
    let head = cbor::read_head(&mut probe).expect("captured frame head is canonical");
    assert_eq!(head.major, MAJOR_ARRAY, "captured frame is an array");
    let stream_item = cbor::read_head(&mut probe).expect("captured stream item is canonical");
    assert_eq!(
        stream_item.major, MAJOR_UINT,
        "captured stream item is an unsigned int"
    );
    let state = cbor::read_head(&mut probe).expect("captured state item is canonical");
    assert_eq!(
        state.major, MAJOR_UINT,
        "captured state item is an unsigned int"
    );
    let (framed, semantic) = WireSignal::decode(speaker, stream_item.value, state.value)
        .expect("captured frame opener is valid")
        .into_parts();
    assert_eq!(framed, stream, "captured frame contradicts its label");

    writeln!(
        out,
        "  frame {index} ({} bytes) / {semantic:?} /",
        item.len()
    )
    .unwrap();
    render_item(item, out);
}

/// Render one captured item under its header: cbor-diag's pretty
/// diagnostic notation, verbatim, when the item is canonical, or the
/// explicit fallback with the reason it is not.
fn render_item(item: &[u8], out: &mut String) {
    let parsed = match cbor_diag::parse_bytes(item) {
        Ok(parsed) => parsed,
        Err(error) => return fallback(item, &error.to_string(), out),
    };
    if parsed.to_bytes() != item {
        return fallback(item, "re-encodes differently", out);
    }
    match canonical(&parsed, cbor_diag::DEFAULT_DEPTH_LIMIT) {
        Ok(()) => writeln!(out, "{}", parsed.to_diag_pretty()).unwrap(),
        Err(reason) => fallback(item, &reason, out),
    }
}

/// Check that a parsed item is canonical wherever the notation would
/// not show a difference.
///
/// Every head must be at its shortest width (only string and container
/// heads need it for injectivity, since the printer spells the others
/// with a width indicator), every container definite, every simple
/// value well-formed, no float a NaN, text free of control characters,
/// and every embedded item (tags 24 and 63) canonical and re-encoding
/// to its bytes.
///
/// The walk mirrors the printer's depth budget: `remaining` counts down
/// one per level, embedded content is parsed with what is left, and a
/// level past the budget is accepted unchecked because the printer shows
/// it as hex. Recursion is therefore bounded by the parser's depth limit.
fn canonical(item: &DataItem, remaining: usize) -> Result<(), String> {
    let Some(remaining) = remaining.checked_sub(1) else {
        return Ok(());
    };
    match item {
        DataItem::Integer { value, bitwidth } | DataItem::Negative { value, bitwidth } => {
            shortest(*value, *bitwidth, "integer")
        }
        DataItem::ByteString(string) => {
            shortest(string.data.len() as u64, string.bitwidth, "byte string")
        }
        DataItem::TextString(string) => {
            shortest(string.data.len() as u64, string.bitwidth, "text string")?;
            if string.data.chars().any(char::is_control) {
                return Err("control character in a text string".into());
            }
            Ok(())
        }
        DataItem::IndefiniteByteString(_) | DataItem::IndefiniteTextString(_) => {
            Err("indefinite-length string".into())
        }
        DataItem::Array { data, bitwidth } => {
            definite(*bitwidth, data.len(), "array")?;
            data.iter().try_for_each(|item| canonical(item, remaining))
        }
        DataItem::Map { data, bitwidth } => {
            definite(*bitwidth, data.len(), "map")?;
            data.iter().try_for_each(|(key, value)| {
                canonical(key, remaining)?;
                canonical(value, remaining)
            })
        }
        DataItem::Tag {
            tag,
            bitwidth,
            value,
        } => {
            shortest(tag.0, *bitwidth, "tag")?;
            if let DataItem::ByteString(string) = &**value {
                match tag.0 {
                    TAG_EMBEDDED_ITEM => embedded_item(&string.data, remaining)?,
                    TAG_CBOR_SEQUENCE => embedded_sequence(&string.data, remaining)?,
                    _ => {}
                }
            }
            canonical(value, remaining)
        }
        // The parser and encoder are bit-exact on floats, so a NaN's
        // sign and payload bits survive re-encoding while the printer
        // writes a bare `NaN`; every NaN is rejected outright.
        DataItem::Float { value, .. } if value.is_nan() => {
            Err("NaN, whose sign and payload bits the notation cannot spell".into())
        }
        DataItem::Float { .. } => Ok(()),
        // Simple values 24 through 31 have no well-formed spelling; the
        // parser accepts their two-byte form and re-encodes it verbatim.
        DataItem::Simple(Simple(24..=31)) => Err("ill-formed simple value".into()),
        DataItem::Simple(_) => Ok(()),
    }
}

/// The embedded item the printer unfolds under tag 24 must be canonical
/// and fill its byte string exactly; content that does not parse stays a
/// byte string, which the printer shows as hex.
fn embedded_item(data: &[u8], remaining: usize) -> Result<(), String> {
    match cbor_diag::parse_bytes_with_limit(data, remaining) {
        Ok(item) if item.to_bytes() != data => Err("embedded item re-encodes differently".into()),
        Ok(item) => {
            canonical(&item, remaining).map_err(|why| format!("in an embedded item, {why}"))
        }
        Err(_) => Ok(()),
    }
}

/// Every item the printer unfolds from a tag-63 sequence must be
/// canonical and re-encode to its span; the unparsed remainder, if any,
/// stays a byte string.
fn embedded_sequence(data: &[u8], remaining: usize) -> Result<(), String> {
    let mut rest = data;
    while let Ok(Some((item, len))) = cbor_diag::parse_bytes_partial_with_limit(rest, remaining) {
        if item.to_bytes() != rest[..len] {
            return Err("an embedded sequence item re-encodes differently".into());
        }
        canonical(&item, remaining).map_err(|why| format!("in an embedded sequence, {why}"))?;
        rest = &rest[len..];
    }
    Ok(())
}

/// The recorded width of a head must be the shortest that holds `value`.
fn shortest(value: u64, width: IntegerWidth, what: &str) -> Result<(), String> {
    let expected = match value {
        0..=23 => IntegerWidth::Zero,
        24..=0xff => IntegerWidth::Eight,
        0x100..=0xffff => IntegerWidth::Sixteen,
        0x1_0000..=0xffff_ffff => IntegerWidth::ThirtyTwo,
        _ => IntegerWidth::SixtyFour,
    };
    if width == expected {
        Ok(())
    } else {
        Err(format!("non-shortest {what} head"))
    }
}

/// A container must be definite-length, with its count at the shortest
/// width.
fn definite(width: Option<IntegerWidth>, len: usize, what: &str) -> Result<(), String> {
    match width {
        Some(width) => shortest(len as u64, width, what),
        None => Err(format!("indefinite-length {what}")),
    }
}

/// Render a failure and its reason above the exact bytes.
fn fallback(bytes: &[u8], reason: &str, out: &mut String) {
    writeln!(
        out,
        "!! not rendered as CBOR ({reason}); the exact bytes stand here:"
    )
    .unwrap();
    writeln!(out, "h'{}'", hex::encode(bytes)).unwrap();
}
