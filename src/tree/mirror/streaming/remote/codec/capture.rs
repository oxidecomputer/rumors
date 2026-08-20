//! CBOR reflection rendering of captured V2 traffic.
//!
//! The snapshot suites pin every wire byte of a captured session, and
//! this module is the form that pin takes: each observed item — one
//! CBOR item per line of the hook's contract — renders as a fully
//! unfolded value tree in extended-diagnostic-style notation, with a
//! rumors naming layer as `/ comment /` annotations (signal names,
//! listing children, tagged-atom meanings). A reviewer reading a
//! re-accept diff sees the semantic field that moved; a generic CBOR
//! reader sees plain diagnostic notation.
//!
//! # Why a rendering with no hexdump is still a byte pin
//!
//! The wire is deterministic-encoding CBOR as a stated contract: one
//! spelling per value, shortest-form heads only. The renderer walks
//! each item with the codec's own canonical head grammar
//! ([`cbor::read_head`]) and shows the item's *complete* content —
//! every integer exactly, every byte string as full hex, every text
//! string escaped, every tag number, and structure in wire order. Under
//! the determinism contract a complete value tree has exactly one
//! encoding, so the rendering is injective on wire bytes: two different
//! byte streams cannot render identically. Wherever the walk cannot
//! vouch for that inversion — non-canonical heads, invalid UTF-8,
//! embedded content that does not fill its byte string, or nesting past
//! the renderer's depth bound (one budget spanning the whole walk:
//! structural descent and embedded-byte-string unfolds draw it down
//! together) — the subtree falls back to an explicit failure line above
//! its exact bytes as hex, which is injective trivially. Byte counts on
//! item and stream headers come from the transport capture, so totals
//! stay exact.
//!
//! # Where the bytes come from
//!
//! Capture enters through the public observation hook
//! ([`crate::observe`]): the harness records each directed stream's
//! items and hands them here as a [`HookCapture`]. The transport-level
//! byte capture ([`LinkCapture`]) remains the totality oracle: the
//! harness asserts, per directed stream, that the stream's on-wire open
//! label followed by the concatenated observed items reproduces the
//! transport bytes exactly ([`assert_items_account_for`],
//! [`stream_label`]) — that assertion is what licenses a rendering of
//! *items* as a pin of *wire bytes*. Structural violations of the
//! capture itself (an item that is not one canonical CBOR item where
//! the wire grammar requires one, a frame contradicting its stream, a
//! label that does not parse) are panics: they mean the capture
//! harness, not the peer, is broken. Application payload bytes are the
//! application's own CBOR and only ever fall back explicitly.

use std::{collections::BTreeMap, fmt::Write as _};

use crate::Version;
use crate::observe::Role;
use crate::tree::mirror::cbor::{
    self, MAJOR_ARRAY, MAJOR_BSTR, MAJOR_MAP, MAJOR_TAG, MAJOR_TEXT, MAJOR_UINT, TAG_CBOR_SEQUENCE,
    TAG_EMBEDDED_ITEM,
};

use super::{
    Speaker, Stream,
    signal::{Signal, WireSignal},
};

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
/// Data streams are keyed by their labeled stream index — exact items
/// and order within each stream, stream groups sorted — discarding the
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
        render_item(item, "  ", out);
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
            writeln!(out, "  frame {index} ({} bytes)", item.len()).unwrap();
            render_frame(speaker, wire_stream, item, out);
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

/// Render one data frame.
///
/// The codec's frame grammar (array head and signal) is held to
/// panics — a violation means the capture is broken — while the body
/// renders through the generic walk, falling back explicitly where it
/// cannot vouch for inversion.
fn render_frame(speaker: Speaker, stream: Stream, item: &[u8], out: &mut String) {
    let mut probe = item;
    let head = cbor::read_head(&mut probe).expect("captured frame head is canonical");
    assert_eq!(head.major, MAJOR_ARRAY, "captured frame is an array");
    let signal = cbor::read_head(&mut probe).expect("captured signal is canonical");
    assert_eq!(
        signal.major, MAJOR_UINT,
        "captured signal is an unsigned int"
    );
    let code = u8::try_from(signal.value).expect("captured signal is in the dense code space");
    let (framed, semantic) = WireSignal::from_byte(speaker, code)
        .expect("captured signal is valid")
        .into_parts();
    assert_eq!(framed, stream, "captured frame contradicts its label");

    writeln!(out, "    [").unwrap();
    writeln!(out, "      {code} / {semantic:?} /").unwrap();
    let naming = match semantic {
        Signal::Query(_) => Naming::Listing,
        Signal::Supply(_) => Naming::Run,
        _ => Naming::Plain,
    };
    let mut rest = probe;
    while !rest.is_empty() {
        let remaining = rest;
        match parse_node(&mut rest, 0) {
            Ok(node) => render_node(&node, naming, "      ", 0, out),
            Err(reason) => {
                fallback(remaining, &reason, "      ", out);
                rest = &[];
            }
        }
    }
    writeln!(out, "    ]").unwrap();
}

/// Render one whole captured item (a control item) as a value tree.
fn render_item(item: &[u8], indent: &str, out: &mut String) {
    let mut rest = item;
    match parse_node(&mut rest, 0) {
        Ok(node) if rest.is_empty() => render_node(&node, Naming::Plain, indent, 0, out),
        Ok(_) => panic!("captured control item carries trailing bytes"),
        Err(reason) => panic!("captured control item is not canonical CBOR: {reason}"),
    }
}

/// The naming context a subtree renders under.
///
/// `Listing` annotates a map as a `{radix => digest}` listing (hex
/// radix keys, `/ digest /` value comments, an order check in the
/// block comment); `Run` names a supply body's embedded sequence a
/// *supply run* and `Record` names the run's items *records*, so a
/// re-accept diff speaks the protocol's own vocabulary.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Naming {
    Plain,
    Listing,
    Run,
    Record,
}

/// One parsed CBOR value, canonical-head-checked, structure preserved
/// in wire order.
#[derive(Debug)]
enum Node {
    Uint(u64),
    /// A major-1 negative integer holding `n`, meaning `-(n + 1)`.
    Nint(u64),
    Bytes(Vec<u8>),
    Text(String),
    Array(Vec<Node>),
    Map(Vec<(Node, Node)>),
    Tag(u64, Box<Node>),
    /// A major-7 simple value.
    Simple(u8),
    /// A major-7 float: its width byte (25, 26, or 27) and raw bits.
    Float(u8, u64),
}

/// Nesting past this bound falls back to exact hex: the walk never
/// recurses on unbounded input-controlled depth.
///
/// One budget spans the whole walk — an embedded byte string's content
/// re-parses at the depth already consumed above it, never at a fresh
/// zero — so structural descent and embedded unfolds are bounded
/// together.
const MAX_DEPTH: usize = 64;

/// Parse one canonical item off the front of `input`.
///
/// Head canonicality comes from the codec's own grammar; major-7 items
/// are handled here because float widths are semantic, not
/// shortest-form arithmetic. Any violation is a typed reason for the
/// caller's explicit fallback.
fn parse_node(input: &mut &[u8], depth: usize) -> Result<Node, String> {
    if depth >= MAX_DEPTH {
        return Err(format!("nested deeper than {MAX_DEPTH}"));
    }
    let Some(&initial) = input.first() else {
        return Err("input ends before an item".into());
    };
    if initial >> 5 == 7 {
        return parse_major_seven(input);
    }
    let head = cbor::read_head(input).map_err(|e| e.to_string())?;
    match head.major {
        MAJOR_UINT => Ok(Node::Uint(head.value)),
        1 => Ok(Node::Nint(head.value)),
        MAJOR_BSTR => {
            let bytes = take(input, head.value)?;
            Ok(Node::Bytes(bytes.to_vec()))
        }
        MAJOR_TEXT => {
            let bytes = take(input, head.value)?;
            let text = std::str::from_utf8(bytes).map_err(|_| "invalid UTF-8".to_string())?;
            Ok(Node::Text(text.to_string()))
        }
        MAJOR_ARRAY => {
            let mut items = Vec::new();
            for _ in 0..head.value {
                items.push(parse_node(input, depth + 1)?);
            }
            Ok(Node::Array(items))
        }
        MAJOR_MAP => {
            let mut entries = Vec::new();
            for _ in 0..head.value {
                let key = parse_node(input, depth + 1)?;
                let value = parse_node(input, depth + 1)?;
                entries.push((key, value));
            }
            Ok(Node::Map(entries))
        }
        MAJOR_TAG => Ok(Node::Tag(
            head.value,
            Box::new(parse_node(input, depth + 1)?),
        )),
        _ => unreachable!("majors 0 through 6 handled; 7 split off above"),
    }
}

/// Parse one major-7 item: simple values inline, one-byte simples with
/// their canonical floor, floats by width with exact bits.
fn parse_major_seven(input: &mut &[u8]) -> Result<Node, String> {
    let (&initial, rest) = input.split_first().expect("caller peeked the initial byte");
    let info = initial & 0x1f;
    match info {
        0..=23 => {
            *input = rest;
            Ok(Node::Simple(info))
        }
        24 => {
            let (&value, rest) = rest
                .split_first()
                .ok_or("input ends inside a simple value")?;
            if value < 32 {
                return Err("one-byte simple value below 32 is not canonical".into());
            }
            *input = rest;
            Ok(Node::Simple(value))
        }
        25..=27 => {
            let width = 1usize << (info - 24);
            if rest.len() < width {
                return Err("input ends inside a float".into());
            }
            let (bytes, rest) = rest.split_at(width);
            let mut bits = 0u64;
            for &byte in bytes {
                bits = bits << 8 | u64::from(byte);
            }
            *input = rest;
            Ok(Node::Float(info, bits))
        }
        28..=30 => Err("reserved additional-information value".into()),
        _ => Err("indefinite-length CBOR is not canonical".into()),
    }
}

/// Split `len` payload bytes off `input`.
fn take<'a>(input: &mut &'a [u8], len: u64) -> Result<&'a [u8], String> {
    let len = usize::try_from(len).map_err(|_| "length exceeds memory".to_string())?;
    if input.len() < len {
        return Err("input ends inside a string".into());
    }
    let (bytes, rest) = input.split_at(len);
    *input = rest;
    Ok(bytes)
}

/// Render one node at `indent`, one line per scalar or bracket.
///
/// `depth` is the walk's one nesting budget, shared with
/// [`parse_node`]: it counts structural levels descended since the
/// walk's entry point, and an embedded byte string's content re-parses
/// at the depth already consumed above it, so structural descent and
/// embedded unfolds are bounded by [`MAX_DEPTH`] together. Invariant
/// every `render_*` call site preserves: the `depth` passed is no
/// greater than the depth its node was parsed at — so a node in hand
/// always fits the remaining budget, and only [`parse_node`] need
/// check the bound.
fn render_node(node: &Node, naming: Naming, indent: &str, depth: usize, out: &mut String) {
    match node {
        Node::Map(entries) if naming == Naming::Listing => {
            render_listing(entries, indent, depth, out);
        }
        Node::Map(entries) => {
            writeln!(out, "{indent}{{").unwrap();
            for (key, value) in entries {
                // The one context-sensitive key: a map value under the
                // text key "listing" is a `{radix => digest}` listing.
                let value_naming = match key {
                    Node::Text(text) if text == "listing" => Naming::Listing,
                    _ => Naming::Plain,
                };
                let key = scalar(key).unwrap_or_else(|| "…".into());
                match scalar(value) {
                    Some(value) => writeln!(out, "{indent}  {key} => {value}").unwrap(),
                    None => {
                        writeln!(out, "{indent}  {key} =>").unwrap();
                        let deeper = format!("{indent}    ");
                        render_node(value, value_naming, &deeper, depth + 1, out);
                    }
                }
            }
            writeln!(out, "{indent}}}").unwrap();
        }
        Node::Array(items) => {
            writeln!(out, "{indent}[").unwrap();
            let deeper = format!("{indent}  ");
            for item in items {
                render_node(item, Naming::Plain, &deeper, depth + 1, out);
            }
            writeln!(out, "{indent}]").unwrap();
        }
        Node::Tag(number, content) => render_tag(*number, content, naming, indent, depth + 1, out),
        scalar_node => {
            let text = scalar(scalar_node).expect("non-container nodes render inline");
            writeln!(out, "{indent}{text}").unwrap();
        }
    }
}

/// Render a `{radix => digest}` listing map: hex radix keys, digest
/// annotations, and an explicit order verdict when the wire's
/// strictly-ascending canonical form is violated.
fn render_listing(entries: &[(Node, Node)], indent: &str, depth: usize, out: &mut String) {
    let ascending = entries
        .windows(2)
        .all(|pair| match (&pair[0].0, &pair[1].0) {
            (Node::Uint(a), Node::Uint(b)) => a < b,
            _ => false,
        })
        || entries.len() < 2;
    let order = if ascending {
        ""
    } else {
        ", NON-CANONICAL ORDER"
    };
    writeln!(
        out,
        "{indent}{{ / listing: {} child(ren){order} /",
        entries.len()
    )
    .unwrap();
    for (key, value) in entries {
        let key = match key {
            Node::Uint(radix) => format!("0x{radix:x}"),
            other => scalar(other).unwrap_or_else(|| "…".into()),
        };
        match value {
            Node::Bytes(bytes) => {
                writeln!(
                    out,
                    "{indent}  {key} => h'{}' / digest /",
                    hex::encode(bytes)
                )
                .unwrap();
            }
            other => match scalar(other) {
                Some(text) => writeln!(out, "{indent}  {key} => {text}").unwrap(),
                None => {
                    writeln!(out, "{indent}  {key} =>").unwrap();
                    let deeper = format!("{indent}    ");
                    render_node(other, Naming::Plain, &deeper, depth + 1, out);
                }
            },
        }
    }
    writeln!(out, "{indent}}}").unwrap();
}

/// Render one tagged node, unfolding embedded byte strings and
/// annotating the tags the protocol names.
///
/// `depth` is the tag's *content* depth — the caller already counted
/// the tag's own structural level — and passes through unchanged.
fn render_tag(
    number: u64,
    content: &Node,
    naming: Naming,
    indent: &str,
    depth: usize,
    out: &mut String,
) {
    match (number, content) {
        (TAG_CBOR_SEQUENCE, Node::Bytes(bytes)) => {
            let (name, inner) = match naming {
                Naming::Run => ("supply run", Naming::Record),
                Naming::Record => ("record", Naming::Plain),
                _ => ("embedded sequence", Naming::Plain),
            };
            render_embedded_as(number, name, inner, bytes, indent, depth, out);
        }
        (TAG_EMBEDDED_ITEM, Node::Bytes(bytes)) => {
            render_embedded(number, "embedded item", bytes, indent, depth, out);
        }
        (crate::tags::VERSION_TAG, Node::Bytes(bytes)) => {
            let meaning = match Version::decode(&bytes[..]) {
                Ok(version) => format!("version {version}"),
                Err(e) => format!("version undecodable: {e}"),
            };
            writeln!(
                out,
                "{indent}{number}(h'{}') / {meaning} /",
                hex::encode(bytes)
            )
            .unwrap();
        }
        (crate::tags::PARTY_TAG, Node::Bytes(bytes)) => {
            writeln!(out, "{indent}{number}(h'{}') / party /", hex::encode(bytes)).unwrap();
        }
        (crate::tags::CLOCK_TAG, Node::Bytes(bytes)) => {
            writeln!(out, "{indent}{number}(h'{}') / clock /", hex::encode(bytes)).unwrap();
        }
        (cbor::TAG_SELF_DESCRIBED, _) => {
            writeln!(out, "{indent}{number}( / self-described CBOR /").unwrap();
            let deeper = format!("{indent}  ");
            render_node(content, naming, &deeper, depth, out);
            writeln!(out, "{indent})").unwrap();
        }
        (_, scalar_content) if scalar(scalar_content).is_some() => {
            let text = scalar(scalar_content).expect("checked by the guard");
            writeln!(out, "{indent}{number}({text})").unwrap();
        }
        _ => {
            writeln!(out, "{indent}{number}(").unwrap();
            let deeper = format!("{indent}  ");
            render_node(content, naming, &deeper, depth, out);
            writeln!(out, "{indent})").unwrap();
        }
    }
}

/// Unfold one embedded byte string (tag 24 or 63) as its parsed
/// item sequence, falling back to exact hex when the content is not
/// wholly canonical CBOR or when the walk's depth budget is spent.
fn render_embedded(
    number: u64,
    name: &str,
    bytes: &[u8],
    indent: &str,
    depth: usize,
    out: &mut String,
) {
    render_embedded_as(number, name, Naming::Plain, bytes, indent, depth, out);
}

/// [`render_embedded`], with the naming context the unfolded items
/// render under (a supply run's items are records).
///
/// The content re-parses at `depth` — the budget already consumed
/// above this byte string — so a chain of embedded byte strings draws
/// down the same [`MAX_DEPTH`] bound as structural nesting, and spends
/// it here as the too-deep fallback.
fn render_embedded_as(
    number: u64,
    name: &str,
    inner: Naming,
    bytes: &[u8],
    indent: &str,
    depth: usize,
    out: &mut String,
) {
    let mut items = Vec::new();
    let mut rest = bytes;
    let mut failure = None;
    while !rest.is_empty() {
        match parse_node(&mut rest, depth) {
            Ok(node) => items.push(node),
            Err(reason) => {
                failure = Some(reason);
                break;
            }
        }
    }
    if let Some(reason) = failure {
        writeln!(out, "{indent}{number}( / {name}, {} bytes /", bytes.len()).unwrap();
        fallback(bytes, &reason, &format!("{indent}  "), out);
        writeln!(out, "{indent})").unwrap();
        return;
    }
    if number == TAG_EMBEDDED_ITEM && items.len() != 1 {
        writeln!(out, "{indent}{number}( / {name}, {} bytes /", bytes.len()).unwrap();
        fallback(
            bytes,
            &format!("embedded item holds {} items", items.len()),
            &format!("{indent}  "),
            out,
        );
        writeln!(out, "{indent})").unwrap();
        return;
    }
    let count = match (number, inner) {
        (TAG_CBOR_SEQUENCE, Naming::Record) => format!(", {} record(s)", items.len()),
        (TAG_CBOR_SEQUENCE, _) => format!(", {} item(s)", items.len()),
        _ => String::new(),
    };
    writeln!(
        out,
        "{indent}{number}(<< / {name}{count}, {} bytes /",
        bytes.len()
    )
    .unwrap();
    let deeper = format!("{indent}  ");
    for item in &items {
        render_node(item, inner, &deeper, depth, out);
    }
    writeln!(out, "{indent}>>)").unwrap();
}

/// Render one scalar node inline, or `None` for containers.
fn scalar(node: &Node) -> Option<String> {
    Some(match node {
        Node::Uint(value) => format!("{value}"),
        Node::Nint(value) => format!("-{}", u128::from(*value) + 1),
        Node::Bytes(bytes) => format!("h'{}'", hex::encode(bytes)),
        Node::Text(text) => format!("{text:?}"),
        Node::Simple(20) => "false".into(),
        Node::Simple(21) => "true".into(),
        Node::Simple(22) => "null".into(),
        Node::Simple(23) => "undefined".into(),
        Node::Simple(value) => format!("simple({value})"),
        Node::Float(25, bits) => format!("float16'{bits:04x}'"),
        Node::Float(26, bits) => format!("float32'{bits:08x}'"),
        Node::Float(_, bits) => format!("float64'{bits:016x}'"),
        Node::Array(_) | Node::Map(_) => return None,
        // Tags the protocol names always render through the block path,
        // so their annotations cannot be skipped by an inline rendering.
        Node::Tag(
            TAG_CBOR_SEQUENCE
            | TAG_EMBEDDED_ITEM
            | cbor::TAG_SELF_DESCRIBED
            | crate::tags::PARTY_TAG
            | crate::tags::VERSION_TAG
            | crate::tags::CLOCK_TAG,
            _,
        ) => return None,
        Node::Tag(number, content) => format!("{number}({})", scalar(content)?),
    })
}

/// Render an explicit walk failure above the exact bytes it convicts:
/// the fallback that keeps the rendering injective where the generic
/// walk cannot vouch for inversion.
fn fallback(bytes: &[u8], reason: &str, indent: &str, out: &mut String) {
    writeln!(
        out,
        "{indent}!! not rendered as CBOR ({reason}); the exact bytes stand here:"
    )
    .unwrap();
    writeln!(out, "{indent}h'{}'", hex::encode(bytes)).unwrap();
}
