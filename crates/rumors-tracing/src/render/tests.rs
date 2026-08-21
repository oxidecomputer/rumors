use ciborium::value::Value;
use rumors::tags::{CLOCK_TAG, PARTY_TAG, VERSION_TAG};

use super::{LENGTH_BUDGET, SHOWN_BYTES, UNFOLD_BUDGET, item, render};

/// Encodes one value to CBOR bytes, the shape `item` consumes.
fn encoded(value: &Value) -> Vec<u8> {
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(value, &mut bytes).expect("test values encode");
    bytes
}

/// Scalars render in diagnostic notation: integers as decimals, text
/// quoted, booleans and null literally.
#[test]
fn scalars_render_in_diagnostic_notation() {
    assert_eq!(item(&encoded(&Value::Integer(170.into()))), "170");
    assert_eq!(item(&encoded(&Value::Text("rumors".into()))), "\"rumors\"");
    assert_eq!(item(&encoded(&Value::Bool(true))), "true");
    assert_eq!(item(&encoded(&Value::Null)), "null");
}

/// Short byte strings render as full hex; long ones elide to the shown
/// prefix plus their true length, so no input can inflate the output.
#[test]
fn byte_strings_render_as_capped_hex() {
    assert_eq!(item(&encoded(&Value::Bytes(vec![0xab, 0xcd]))), "h'abcd'");
    let long = vec![0x5a; SHOWN_BYTES + 9];
    let rendered = item(&encoded(&Value::Bytes(long.clone())));
    assert!(rendered.starts_with("h'"));
    assert!(rendered.ends_with(&format!("…({} B)", long.len())));
}

/// Arrays and maps unfold with their elements rendered in place, the
/// shape a wire frame or greeting map arrives in.
#[test]
fn containers_unfold() {
    let frame = Value::Array(vec![Value::Integer(162.into()), Value::Bytes(vec![0x01])]);
    assert_eq!(item(&encoded(&frame)), "[162, h'01']");
    let map = Value::Map(vec![(
        Value::Text("protocol".into()),
        Value::Text("rumors".into()),
    )]);
    assert_eq!(item(&encoded(&map)), "{\"protocol\": \"rumors\"}");
}

/// The registered atom tags render by name, the thin naming layer over
/// the otherwise rumors-blind skeleton.
#[test]
fn atom_tags_render_by_name() {
    for (tag, name) in [
        (PARTY_TAG, "party"),
        (VERSION_TAG, "version"),
        (CLOCK_TAG, "clock"),
    ] {
        let atom = Value::Tag(tag, Box::new(Value::Bytes(vec![0x42])));
        assert_eq!(item(&encoded(&atom)), format!("{name}(h'42')"));
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
    assert_eq!(item(&encoded(&one)), "24(<<7>>)");

    let mut sequence = encoded(&Value::Integer(1.into()));
    sequence.extend(encoded(&Value::Text("x".into())));
    let run = Value::Tag(63, Box::new(Value::Bytes(sequence)));
    assert_eq!(item(&encoded(&run)), "63(<<1, \"x\">>)");
}

/// A tag-24 byte string that does not hold exactly one item (or does
/// not parse) falls back to the honest raw byte-string form instead of
/// guessing.
#[test]
fn malformed_embeddings_fall_back_to_bytes() {
    let two_items = {
        let mut bytes = encoded(&Value::Integer(1.into()));
        bytes.extend(encoded(&Value::Integer(2.into())));
        bytes
    };
    let tagged = Value::Tag(24, Box::new(Value::Bytes(two_items)));
    assert_eq!(item(&encoded(&tagged)), "24(h'0102')");

    let garbage = Value::Tag(63, Box::new(Value::Bytes(vec![0xff])));
    assert_eq!(item(&encoded(&garbage)), "63(h'ff')");
}

/// Tag-24 embedded CBOR nested one level past the unfold budget stops
/// unfolding at the boundary: the innermost embedded byte string
/// renders as raw hex (`h'…'`), not as its decoded contents.
///
/// Every level the budget covers unfolds as `<<…>>`. This pins that
/// the budget spans embedded-CBOR boundaries — each re-parse draws
/// down the one shared budget rather than starting a fresh one.
#[test]
fn unfold_budget_spans_embedded_boundaries() {
    // One more tag-24 level than the budget can unfold.
    let mut value = Value::Integer(7.into());
    for _ in 0..=UNFOLD_BUDGET {
        value = Value::Tag(24, Box::new(Value::Bytes(encoded(&value))));
    }
    // The innermost tag's byte string (the encoding of 7) stays raw;
    // every level above it unfolds.
    let mut expected = "24(h'07')".to_string();
    for _ in 0..UNFOLD_BUDGET {
        expected = format!("24(<<{expected}>>)");
    }
    assert_eq!(item(&encoded(&value)), expected);
}

/// Unknown tags render by number with their content unfolded: the
/// renderer stays total over foreign vocabulary.
#[test]
fn unknown_tags_render_by_number() {
    let foreign = Value::Tag(55799, Box::new(Value::Text("rumors".into())));
    assert_eq!(item(&encoded(&foreign)), "55799(\"rumors\")");
}

/// Undecodable bytes and trailing garbage render as explicit defect
/// notes: the renderer never panics on wire input, whatever arrives.
#[test]
fn defects_render_as_notes_not_panics() {
    assert!(item(&[0xff]).starts_with("!undecodable "));
    let mut trailing = encoded(&Value::Integer(1.into()));
    trailing.push(0x00);
    assert_eq!(item(&trailing), "1 !trailing(1 B)");
}

/// Nesting past the depth budget elides instead of recursing: the
/// budget parameter, not the input's shape, bounds the renderer's
/// stack.
///
/// Driven through `render` directly with a small budget so the
/// property is the renderer's own, independent of the decoder's
/// nesting limits.
#[test]
fn depth_is_bounded_by_the_budget() {
    let mut value = Value::Integer(0.into());
    for _ in 0..8 {
        value = Value::Array(vec![value]);
    }
    let mut shallow = String::new();
    render(&value, &mut shallow, UNFOLD_BUDGET, 4);
    assert!(shallow.contains('…'));
    let mut deep = String::new();
    render(&value, &mut deep, UNFOLD_BUDGET, 16);
    assert_eq!(deep, "[[[[[[[[0]]]]]]]]");
}

/// A multibyte character straddling the length budget truncates to the
/// preceding character boundary instead of panicking.
#[test]
fn truncation_respects_character_boundaries() {
    let text: String = "é".repeat(LENGTH_BUDGET);
    let rendered = item(&encoded(&Value::Text(text)));
    assert!(rendered.ends_with('…'));
    assert!(rendered.len() <= LENGTH_BUDGET + '…'.len_utf8());
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
    let rendered = item(&encoded(&wide));
    assert!(rendered.len() <= LENGTH_BUDGET + '…'.len_utf8());
    assert!(rendered.ends_with('…'));
}
