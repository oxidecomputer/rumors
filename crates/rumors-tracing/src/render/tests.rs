use ciborium::value::Value;
use rumors::tags::{CLOCK_TAG, PARTY_TAG, VERSION_TAG};

use super::{DiagCbor, LENGTH_BUDGET};

/// Encodes one value to CBOR bytes, the shape `DiagCbor` consumes.
fn encoded(value: &Value) -> Vec<u8> {
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(value, &mut bytes).expect("test values encode");
    bytes
}

/// Renders one item the way the event field does.
fn rendered(bytes: &[u8]) -> String {
    DiagCbor(bytes).to_string()
}

/// Scalars render in diagnostic notation: integers as decimals (with
/// the notation's encoding indicator where the head is not immediate),
/// text quoted, booleans and null literally.
#[test]
fn scalars_render_in_diagnostic_notation() {
    assert_eq!(rendered(&encoded(&Value::Integer(7.into()))), "7");
    assert_eq!(rendered(&encoded(&Value::Integer(170.into()))), "170_0");
    assert_eq!(
        rendered(&encoded(&Value::Text("rumors".into()))),
        "\"rumors\""
    );
    assert_eq!(rendered(&encoded(&Value::Bool(true))), "true");
    assert_eq!(rendered(&encoded(&Value::Null)), "null");
}

/// Byte strings render as full hex; the length budget, not a per-string
/// cap, is what bounds a rendering built from large strings.
#[test]
fn byte_strings_render_as_hex() {
    assert_eq!(
        rendered(&encoded(&Value::Bytes(vec![0xab, 0xcd]))),
        "h'abcd'"
    );
}

/// Arrays and maps unfold with their elements rendered in place, the
/// shape a wire frame or greeting map arrives in.
#[test]
fn containers_unfold() {
    let frame = Value::Array(vec![Value::Integer(162.into()), Value::Bytes(vec![0x01])]);
    assert_eq!(rendered(&encoded(&frame)), "[162_0,h'01']");
    let map = Value::Map(vec![(
        Value::Text("protocol".into()),
        Value::Text("rumors".into()),
    )]);
    assert_eq!(rendered(&encoded(&map)), "{\"protocol\":\"rumors\"}");
}

/// The registered atom tags render by number, like any other tag: the
/// rendering is rumors-blind, and the tag registry in [`rumors::tags`]
/// is the reader's decoder ring.
#[test]
fn atom_tags_render_by_number() {
    for tag in [PARTY_TAG, VERSION_TAG, CLOCK_TAG] {
        let atom = Value::Tag(tag, Box::new(Value::Bytes(vec![0x42])));
        assert_eq!(rendered(&encoded(&atom)), format!("{tag}_1(h'42')"));
    }
}

/// Embedded-CBOR byte strings unfold as `<<…>>`: tag 24 to its one
/// item, tag 63 to its whole sequence — the deep-inspection payoff for
/// supply runs and their records.
#[test]
fn embedded_cbor_tags_unfold() {
    let one = Value::Tag(
        24,
        Box::new(Value::Bytes(encoded(&Value::Integer(7.into())))),
    );
    assert_eq!(rendered(&encoded(&one)), "24_0(<<7>>)");

    let mut sequence = encoded(&Value::Integer(1.into()));
    sequence.extend(encoded(&Value::Text("x".into())));
    let run = Value::Tag(63, Box::new(Value::Bytes(sequence)));
    assert_eq!(rendered(&encoded(&run)), "63_0(<<1,\"x\">>)");
}

/// A tag-24 byte string that does not hold exactly one item falls back
/// to the honest raw byte-string form instead of guessing.
#[test]
fn malformed_embedded_items_fall_back_to_bytes() {
    let two_items = {
        let mut bytes = encoded(&Value::Integer(1.into()));
        bytes.extend(encoded(&Value::Integer(2.into())));
        bytes
    };
    let tagged = Value::Tag(24, Box::new(Value::Bytes(two_items)));
    assert_eq!(rendered(&encoded(&tagged)), "24_0(h'0102')");
}

/// A chain of embedded byte strings longer than the depth limit stops
/// unfolding at the budget and falls back to the raw byte-string form:
/// rendering terminates with bounded recursion however deep the chain.
#[test]
fn unfolding_stops_at_the_depth_budget() {
    let mut value = Value::Integer(7.into());
    for _ in 0..300 {
        value = Value::Tag(24, Box::new(Value::Bytes(encoded(&value))));
    }
    let out = rendered(&encoded(&value));
    assert!(out.starts_with("24_0(<<"));
    assert!(out.contains("24_0(h'"));
}

/// An item nested structurally past the depth limit renders as the
/// explicit unrenderable note instead of recursing without bound.
#[test]
fn deep_structure_renders_as_a_note() {
    let mut value = Value::Integer(0.into());
    for _ in 0..300 {
        value = Value::Array(vec![value]);
    }
    assert!(rendered(&encoded(&value)).starts_with("unrenderable CBOR h'"));
}

/// Undecodable bytes and trailing garbage render as the explicit note
/// plus capped hex: the renderer never panics on wire input, whatever
/// arrives, and a large garbage buffer buys only a bounded rendering.
#[test]
fn defects_render_as_notes_not_panics() {
    assert_eq!(rendered(&[0xff]), "unrenderable CBOR h'ff'");
    let mut trailing = encoded(&Value::Integer(1.into()));
    trailing.push(0x00);
    assert_eq!(rendered(&trailing), "unrenderable CBOR h'0100'");
    let garbage = vec![0xffu8; 100_000];
    let out = rendered(&garbage);
    assert!(out.len() <= LENGTH_BUDGET + '…'.len_utf8());
    assert!(out.ends_with(&format!("…({} B)", garbage.len())));
}

/// A multibyte character straddling the length budget truncates to the
/// preceding character boundary instead of panicking.
#[test]
fn truncation_respects_character_boundaries() {
    let text: String = "é".repeat(LENGTH_BUDGET);
    let out = rendered(&encoded(&Value::Text(text)));
    assert!(out.ends_with('…'));
    assert!(out.len() <= LENGTH_BUDGET + '…'.len_utf8());
}

/// A rendering that crosses the length budget truncates with an
/// elision marker: events stay cheap under megabyte supply runs.
#[test]
fn length_is_bounded_by_the_budget() {
    let wide = Value::Array(
        (0..LENGTH_BUDGET)
            .map(|i| Value::Integer((i as i64 % 10).into()))
            .collect(),
    );
    let out = rendered(&encoded(&wide));
    assert!(out.len() <= LENGTH_BUDGET + '…'.len_utf8());
    assert!(out.ends_with('…'));
}
