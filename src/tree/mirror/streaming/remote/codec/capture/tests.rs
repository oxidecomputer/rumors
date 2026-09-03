//! The capture renderer's pins.
//!
//! The framing the harness owns (item index, exact byte count,
//! protocol-phase label) is stated on each header line; every item body
//! is cbor-diag's diagnostic notation, verbatim, when the item is
//! canonical, and otherwise an explicit failure above its exact hex, so
//! two different byte strings never share a rendering and no body line
//! begins with `frame `; and the totality witness ([`assert_items_account_for`]) refuses
//! any gap between observed items and wire bytes.

use super::*;

use crate::Version;
use crate::message::{Message, PayloadDepthLimit};
use crate::tree::mirror::cbor::MAJOR_BSTR;

use super::super::encode::encode;
use super::super::frame::{Frame, LeafRun, Reaction};
use super::super::signal::Flow;

/// Encode one supply frame carrying `payload` as a single record on
/// stream 0, the harness-reachable shape a captured frame arrives in.
fn supply_frame(payload: &Message) -> (Stream, Vec<u8>) {
    let mut run = LeafRun::new();
    run.push(&Version::new(), payload)
        .expect("one record fits a fresh run");
    let stream = Stream::new(0).expect("stream 0 names a stream");
    let frame = (stream, Frame::Reaction(Reaction::Supply(run), Flow::End));
    let mut bytes = Vec::new();
    encode(Speaker::Initiator, &frame, &mut bytes).expect("a supply frame encodes");
    (stream, bytes)
}

/// A frame renders under a header line naming its index, its exact byte
/// count, and the signal its opener decodes to, with the whole frame as
/// one diagnostic-notation item beneath.
///
/// The supply run unfolds through its embedded-sequence tag down to the
/// record's payload.
#[test]
fn frames_render_their_label_and_diagnostic_body() {
    let (stream, bytes) = supply_frame(&Message::new(7_u64));
    let mut out = String::new();
    render_frame(Speaker::Initiator, stream, 3, &bytes, &mut out);
    let header = out.lines().next().unwrap_or_default();
    assert_eq!(
        header,
        format!("  frame 3 ({} bytes) / Supply(End) /", bytes.len()),
        "{out}"
    );
    assert!(
        out.contains("(h'e0'), 7>>"),
        "the run unfolds to the record's version atom and payload: {out}"
    );
}

/// Bytes that are not one parseable CBOR item render as an explicit
/// failure line carrying the parse error above the exact bytes.
///
/// The specimen is an array head promising more elements than follow.
#[test]
fn unparseable_items_fall_back_explicitly_to_hex() {
    let truncated = [0x82, 0x01];
    let mut out = String::new();
    render_item(&truncated, &mut out);
    assert!(
        out.starts_with("!! not rendered as CBOR ("),
        "the failure is explicit: {out}"
    );
    assert!(
        out.contains(&format!("h'{}'", hex::encode(truncated))),
        "the exact bytes stand: {out}"
    );
}

/// Render `bytes` as one item, returning the rendering's lines.
fn rendered(bytes: &[u8]) -> Vec<String> {
    let mut out = String::new();
    render_item(bytes, &mut out);
    out.lines().map(str::to_string).collect()
}

/// A length head wider than its value needs falls back to hex, so two
/// arrays differing only in which empty string has the wide head render
/// distinctly.
///
/// The notation spells a byte string's length without a width
/// indicator, which is why the canonical `[h'', h'']` alone renders in
/// notation. The same holds inside an embedded item.
#[test]
fn non_shortest_length_heads_fall_back() {
    let canonical = rendered(&[0x82, 0x40, 0x40]);
    let wide_second = rendered(&[0x82, 0x40, 0x58, 0x00]);
    let wide_first = rendered(&[0x82, 0x58, 0x00, 0x40]);
    assert_eq!(canonical, ["[h'', h'']"]);
    assert!(
        wide_second[0].contains("non-shortest byte string head"),
        "{wide_second:?}"
    );
    assert_eq!(wide_second[1], "h'82405800'");
    assert_eq!(wide_first[1], "h'82580040'");
    let embedded = rendered(&[0xd8, 0x18, 0x44, 0x82, 0x40, 0x58, 0x00]);
    assert!(embedded[0].contains("in an embedded item"), "{embedded:?}");
}

/// The ill-formed two-byte spelling of a simple value (`f8 14` for
/// `false`) falls back to hex, so arrays differing only in which `false`
/// is ill-formed render distinctly.
///
/// The re-encoding check catches it: re-encoding uses the one-byte form.
#[test]
fn ill_formed_simple_values_fall_back() {
    assert_eq!(rendered(&[0x82, 0xf4, 0xf4]), ["[false, false]"]);
    let second = rendered(&[0x82, 0xf4, 0xf8, 0x14]);
    let first = rendered(&[0x82, 0xf8, 0x14, 0xf4]);
    assert!(second[0].contains("re-encodes differently"), "{second:?}");
    assert_eq!(second[1], "h'82f4f814'");
    assert_eq!(first[1], "h'82f814f4'");
}

/// Every NaN falls back to hex, so two NaNs differing only in the sign
/// bit render distinctly, while a finite float renders in notation.
///
/// The parser and encoder are bit-exact on floats, so the re-encoding
/// check alone would pass both.
#[test]
fn nans_fall_back() {
    let positive = rendered(&[0xfb, 0x7f, 0xf8, 0, 0, 0, 0, 0, 0]);
    let negative = rendered(&[0xfb, 0xff, 0xf8, 0, 0, 0, 0, 0, 0]);
    assert!(positive[0].contains("NaN"), "{positive:?}");
    assert_eq!(positive[1], "h'fb7ff8000000000000'");
    assert_eq!(negative[1], "h'fbfff8000000000000'");
    assert_eq!(rendered(&[0xf9, 0x3c, 0x00]), ["1.0_1"]);
}

/// The two-byte spelling of a simple value in 24 through 31 (`f8 18`)
/// is not well-formed CBOR and falls back to hex; the parser accepts it
/// and re-encodes it verbatim, so the walk rejects it by value.
#[test]
fn simple_values_without_a_well_formed_spelling_fall_back() {
    let out = rendered(&[0xf8, 0x18]);
    assert!(out[0].contains("ill-formed simple value"), "{out:?}");
    assert_eq!(out[1], "h'f818'");
    assert_eq!(rendered(&[0xf8, 0x20]), ["simple(32)"]);
}

/// A text string holding a control character falls back to hex, so a
/// payload cannot forge a header.
///
/// A string carrying a newline followed by a frame header's text renders
/// under its real header as hex, and the only line beginning with
/// `frame ` is the header.
#[test]
fn text_with_control_characters_falls_back() {
    let forged = "a\nframe 9 (1 bytes) / Supply(End) /";
    let (stream, bytes) = supply_frame(&Message::new(forged.to_string()));
    let mut out = String::new();
    render_frame(Speaker::Initiator, stream, 0, &bytes, &mut out);
    let headers: Vec<_> = out
        .lines()
        .filter(|line| line.trim_start().starts_with("frame "))
        .collect();
    assert_eq!(headers.len(), 1, "{out}");
    assert!(out.contains("control character in a text string"), "{out}");
}

/// An item followed by trailing bytes is not one item: the whole buffer
/// falls back explicitly rather than rendering the item and dropping the
/// tail.
#[test]
fn trailing_bytes_fall_back_explicitly() {
    let two_items = [0x01, 0x02];
    let mut out = String::new();
    render_item(&two_items, &mut out);
    assert!(out.starts_with("!! not rendered as CBOR ("), "{out}");
    assert!(out.contains("h'0102'"), "{out}");
}

/// The totality witness accepts exactly the wire it was given, split at
/// any item boundaries.
#[test]
fn items_accounting_accepts_the_exact_wire() {
    let wire = [1_u8, 2, 3, 4, 5];
    assert_items_account_for(&[vec![1, 2], vec![3], vec![4, 5]], &wire);
    assert_items_account_for(&[vec![1, 2, 3, 4, 5]], &wire);
    assert_items_account_for(&[], &[]);
}

/// A wire byte no observed item accounts for is refused.
#[test]
#[should_panic(expected = "beyond the last observed item")]
fn items_accounting_refuses_unobserved_bytes() {
    assert_items_account_for(&[vec![1, 2]], &[1, 2, 3]);
}

/// An observed item the wire does not carry is refused.
#[test]
#[should_panic(expected = "does not match the wire")]
fn items_accounting_refuses_diverging_items() {
    assert_items_account_for(&[vec![1, 9]], &[1, 2]);
}

/// The stream label parses to its epoch and index, and reports its
/// exact byte length.
#[test]
fn stream_label_parses_epoch_and_index() {
    let mut bytes = Vec::new();
    cbor::write_head(&mut bytes, MAJOR_UINT, 1);
    cbor::write_head(&mut bytes, MAJOR_UINT, 200);
    bytes.push(0xee);
    let ((epoch, index), len) = stream_label(&bytes);
    assert_eq!((epoch, index), (1, 200));
    assert_eq!(len, 3, "one short head and one byte-argument head");
}

/// Control items are named by their shape.
#[test]
fn control_items_are_named_by_shape() {
    let mut preamble = Vec::new();
    cbor::write_tag(&mut preamble, cbor::TAG_SELF_DESCRIBED);
    assert_eq!(control_item_name(&preamble), "preamble");

    let mut greeting = Vec::new();
    cbor::write_tag(&mut greeting, TAG_EMBEDDED_ITEM);
    assert_eq!(control_item_name(&greeting), "greeting");

    let mut party = Vec::new();
    cbor::write_tag(&mut party, crate::tags::PARTY_TAG);
    assert_eq!(control_item_name(&party), "party hand-off");

    let mut epilogue = Vec::new();
    cbor::write_head(&mut epilogue, cbor::MAJOR_TEXT, 1);
    epilogue.push(b'.');
    assert_eq!(control_item_name(&epilogue), "epilogue");
}

/// A control item of no known shape is a broken capture, not a
/// renderable one.
#[test]
#[should_panic(expected = "no known shape")]
fn unknown_control_item_shapes_are_refused() {
    let mut bare = Vec::new();
    cbor::write_head(&mut bare, MAJOR_UINT, 0);
    control_item_name(&bare);
}

/// Build a chain of `levels` nested embedded byte strings, each level
/// one tag-24 item wrapping the next level's encoding as a byte string,
/// bottoming out at a single `0x00` uint.
///
/// Written outside-in: encoded lengths follow the recurrence
/// `len[0] = 1` (the innermost uint) and
/// `len[i + 1] = tag head (2 bytes) + byte-string head + len[i]`, so
/// every head is computed before any byte is emitted and the build is
/// linear in the output size.
fn embedded_chain(levels: usize) -> Vec<u8> {
    let mut lens = vec![1_usize];
    for _ in 0..levels {
        let inner = *lens.last().expect("the list starts nonempty");
        lens.push(2 + cbor::head_len(inner as u64) + inner);
    }
    let mut bytes = Vec::with_capacity(lens[levels]);
    for &len in lens[..levels].iter().rev() {
        cbor::write_tag(&mut bytes, TAG_EMBEDDED_ITEM);
        cbor::write_head(&mut bytes, MAJOR_BSTR, len as u64);
    }
    cbor::write_head(&mut bytes, MAJOR_UINT, 0);
    bytes
}

/// A chain of embedded byte strings nested far past the depth limit
/// renders with bounded recursion.
///
/// Unfolding stops at the limit and the remainder stands as a plain byte
/// string; the walk returns rather than overflowing the stack.
#[test]
fn deep_embedded_chain_stops_unfolding_instead_of_recursing() {
    let bytes = embedded_chain(10 * cbor_diag::DEFAULT_DEPTH_LIMIT);
    let mut out = String::new();
    render_item(&bytes, &mut out);
    assert!(
        out.starts_with("24_0(<<"),
        "the chain unfolds from the top: {out:.80}"
    );
    assert!(out.contains("24_0(h'"), "and stops at the limit: {out:.80}");
}

/// The same nesting arriving as a supply record's payload (a captured
/// frame rendered whole) renders with the same bounded unfold.
///
/// An application payload of legal CBOR can nest arbitrarily, and the
/// frame walk must return, never overflow.
#[test]
fn deep_payload_through_the_frame_path_renders() {
    let payload = Message::from_slice::<ciborium::Value>(
        &embedded_chain(10 * cbor_diag::DEFAULT_DEPTH_LIMIT),
        PayloadDepthLimit::default(),
    )
    .expect("the chain is exactly one CBOR item");
    let (stream, bytes) = supply_frame(&payload);
    let mut out = String::new();
    render_frame(Speaker::Initiator, stream, 0, &bytes, &mut out);
    assert!(
        out.contains("24_0(h'"),
        "unfolding stopped at the limit: {out:.120}"
    );
}

/// Structure nested past cbor-diag's depth limit is not one parseable
/// item: the whole buffer falls back explicitly, naming the depth, rather
/// than recursing without bound on input-controlled depth.
#[test]
fn structure_past_the_depth_limit_falls_back_explicitly() {
    let mut bytes = Vec::new();
    for _ in 0..=cbor_diag::DEFAULT_DEPTH_LIMIT {
        cbor::write_head(&mut bytes, MAJOR_ARRAY, 1);
    }
    cbor::write_head(&mut bytes, MAJOR_UINT, 0);
    let mut out = String::new();
    render_item(&bytes, &mut out);
    assert!(out.starts_with("!! not rendered as CBOR ("), "{out:.120}");
    assert!(
        out.contains(&format!("h'{}'", hex::encode(&bytes))),
        "the exact bytes stand"
    );
}
