//! The reflection renderer's pins.
//!
//! Three commitments: the rendered value tree names the semantic field a
//! snapshot re-accept moved (the committed fixture pair below differs in
//! exactly one rendered line), bytes the walk cannot vouch for render as
//! an explicit failure above their exact hex (never as a silently pretty
//! tree, never as silent omission), and the totality witness
//! ([`assert_items_account_for`]) refuses any gap between observed items
//! and wire bytes.

use super::*;

use crate::message::Message;
use crate::tree::typed::Hash;
use crate::tree::typed::hash::MERKLE_HASH_LEN;

use super::super::frame::{LeafRun, write_listing};

/// Render one embedded run (the supply body's tag-63 content) to lines.
fn run_lines(run: &LeafRun) -> Vec<String> {
    let mut out = String::new();
    render_embedded(
        TAG_CBOR_SEQUENCE,
        "embedded sequence",
        run.as_bytes(),
        "",
        &mut out,
    );
    out.lines().map(str::to_string).collect()
}

/// Two supply runs differing only in one record's version render line
/// sets that differ in exactly the line annotating that version: the
/// field-level account an insta re-accept diff shows.
#[test]
fn supply_reflection_names_the_field_that_moved() {
    let party = before::Party::seed();
    let mut low = Version::new();
    low.tick(&party);
    let mut high = low.clone();
    high.tick(&party);

    let render = |version: &Version| {
        let mut run = LeafRun::new();
        run.push(version, &Message::new(7_u64))
            .expect("one small record fits any run");
        run_lines(&run)
    };
    let a = render(&low);
    let b = render(&high);
    assert_eq!(a.len(), b.len(), "one field moved, no line appeared");
    let diffs: Vec<_> = a.iter().zip(&b).filter(|(a, b)| a != b).collect();
    assert_eq!(diffs.len(), 1, "exactly one rendered line moved: {diffs:?}");
    let (a_line, b_line) = diffs[0];
    assert!(a_line.contains(&format!("version {low}")), "{a_line}");
    assert!(b_line.contains(&format!("version {high}")), "{b_line}");
    // The identical payload renders identically as its own line: the
    // small u64 is a bare CBOR int, visible directly.
    assert!(a.iter().any(|line| line.trim() == "7"), "{a:?}");
}

/// Embedded content the walk cannot vouch for renders as an explicit
/// failure line above the exact bytes, never as silence or a partial
/// tree presented as whole.
#[test]
fn undecodable_embedded_content_falls_back_explicitly_to_hex() {
    // 0xf8 0x05: a one-byte simple value below 32 is not canonical.
    let garbage = [0xf8, 0x05];
    let mut out = String::new();
    render_embedded(
        TAG_CBOR_SEQUENCE,
        "embedded sequence",
        &garbage,
        "",
        &mut out,
    );
    assert!(
        out.contains("!! not rendered as CBOR"),
        "the failure is explicit: {out}"
    );
    assert!(
        out.contains(&format!("h'{}'", hex::encode(garbage))),
        "the exact bytes stand: {out}"
    );
}

/// A tag-24 embedded item holding anything but exactly one item falls
/// back explicitly: an embedded *item* is one item by definition.
#[test]
fn embedded_item_with_two_items_falls_back() {
    let mut bytes = Vec::new();
    cbor::write_head(&mut bytes, MAJOR_UINT, 1);
    cbor::write_head(&mut bytes, MAJOR_UINT, 2);
    let mut out = String::new();
    render_embedded(TAG_EMBEDDED_ITEM, "embedded item", &bytes, "", &mut out);
    assert!(out.contains("holds 2 items"), "{out}");
    assert!(out.contains("!! not rendered as CBOR"), "{out}");
}

/// A listing map renders each child as a hex radix and an annotated
/// digest, and the block comment carries the child count.
#[test]
fn listing_renders_children_with_digest_annotations() {
    let mut bytes = Vec::new();
    write_listing(
        &mut bytes,
        &[
            (0x3_u8, Hash([0xab; MERKLE_HASH_LEN])),
            (0xc_u8, Hash([0x01; MERKLE_HASH_LEN])),
        ],
    );
    let mut input = bytes.as_slice();
    let node = parse_node(&mut input, 0).expect("the codec writes canonical listings");
    let mut out = String::new();
    render_node(&node, Naming::Listing, "", &mut out);
    assert!(out.contains("/ listing: 2 child(ren) /"), "{out}");
    assert!(
        out.contains(&format!(
            "0x3 => h'{}' / digest /",
            "ab".repeat(MERKLE_HASH_LEN)
        )),
        "{out}"
    );
    assert!(
        out.contains(&format!(
            "0xc => h'{}' / digest /",
            "01".repeat(MERKLE_HASH_LEN)
        )),
        "{out}"
    );
    assert!(!out.contains("NON-CANONICAL"), "{out}");
}

/// A listing whose radixes are not strictly ascending renders with an
/// explicit order verdict: the violation is visible in the transcript,
/// and the entries still render completely, in wire order.
#[test]
fn descending_listing_renders_an_order_verdict() {
    let mut bytes = Vec::new();
    cbor::write_head(&mut bytes, MAJOR_MAP, 2);
    for radix in [0xf_u8, 0x0] {
        cbor::write_head(&mut bytes, MAJOR_UINT, u64::from(radix));
        cbor::write_head(&mut bytes, MAJOR_BSTR, MERKLE_HASH_LEN as u64);
        bytes.extend_from_slice(&[radix; MERKLE_HASH_LEN]);
    }
    let mut input = bytes.as_slice();
    let node = parse_node(&mut input, 0).expect("heads are canonical; order is not");
    let mut out = String::new();
    render_node(&node, Naming::Listing, "", &mut out);
    assert!(out.contains("NON-CANONICAL ORDER"), "{out}");
    assert!(out.contains("0xf =>"), "first entry renders: {out}");
    assert!(out.contains("0x0 =>"), "second entry renders: {out}");
}

/// A version-tagged byte string whose bytes are no version encoding
/// annotates the failure rather than inventing a meaning.
#[test]
fn garbage_version_atom_annotates_undecodable() {
    let mut bytes = Vec::new();
    cbor::write_tag(&mut bytes, crate::tags::VERSION_TAG);
    cbor::write_head(&mut bytes, MAJOR_BSTR, 3);
    bytes.extend_from_slice(&[0xff, 0xff, 0xff]);
    let mut input = bytes.as_slice();
    let node = parse_node(&mut input, 0).expect("the wrapper is canonical");
    let mut out = String::new();
    render_node(&node, Naming::Plain, "", &mut out);
    assert!(out.contains("version undecodable"), "{out}");
    assert!(out.contains("h'ffffff'"), "the atom bytes stand: {out}");
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

/// Control items are named by their shape; an unknown shape is a broken
/// capture, not a renderable one.
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

/// Nesting past the walk's depth bound falls back explicitly instead of
/// recursing without bound on input-controlled depth.
#[test]
fn nesting_past_the_depth_bound_falls_back() {
    let mut bytes = Vec::new();
    for _ in 0..=MAX_DEPTH {
        cbor::write_head(&mut bytes, MAJOR_ARRAY, 1);
    }
    cbor::write_head(&mut bytes, MAJOR_UINT, 0);
    let mut input = bytes.as_slice();
    let error = parse_node(&mut input, 0).expect_err("too deep to vouch for");
    assert!(error.contains("deeper than"), "{error}");
}
