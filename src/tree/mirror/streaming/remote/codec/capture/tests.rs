//! The capture renderer's payload-decoding pins.
//!
//! Two commitments: the decoded parse tree names the semantic field a
//! snapshot re-accept moved (the committed fixture pair below differs in
//! exactly one field and exactly one rendered line), and payload bytes
//! that do not decode render as an explicit failure line, never as
//! silent hex.

use super::*;

use crate::message::Message;
use crate::tree::typed::hash::MERKLE_HASH_LEN;

use super::super::frame::LeafRun;

/// The committed fixture pair: two supply runs identical except one
/// record's version decode to renderings differing at exactly the line
/// naming that record.
///
/// That one-line diff is the field-level account an insta re-accept
/// shows beside the hex.

/// Encode a listing as its wire form: raw radix-hash records.
fn encode_listing(children: &[(u8, Hash)]) -> Vec<u8> {
    let mut body = Vec::new();
    for (radix, hash) in children {
        body.push(*radix);
        body.extend_from_slice(hash.as_bytes());
    }
    body
}

/// The capture renderer decodes each supply record structurally.
///
/// Two runs differing only in the record's version render record lines
/// that differ exactly at the named version field, with the record count
/// and the (identical) payload accounting unchanged.
#[test]
fn supply_decode_names_the_field_that_moved() {
    let party = before::Party::seed();
    let mut low = Version::new();
    low.tick(&party);
    let mut high = low.clone();
    high.tick(&party);

    let render = |version: &Version| {
        let mut run = LeafRun::new();
        run.push(version, &Message::new(7_u64))
            .expect("one small record fits any run");
        supply_lines(run.as_bytes().to_vec())
    };
    let a = render(&low);
    let b = render(&high);
    assert_eq!(a.len(), 2, "one header line and one record line");
    assert_eq!(a[0], b[0], "the record count did not move");
    assert_ne!(a[1], b[1], "the record line names the moved field");
    assert!(a[1].contains(&format!("version {low}")));
    assert!(b[1].contains(&format!("version {high}")));
    // The message is identical on both sides, so both record lines
    // account it identically: one CBOR byte for the small u64.
    assert!(a[1].ends_with("message 1 byte(s)"));
    assert!(b[1].ends_with("message 1 byte(s)"));
}

/// Unparseable supply payloads render an explicit decode failure, never
/// silent hex.
///
/// A run with broken record framing convicts the whole run, and a
/// structurally framed record whose version bytes do not decode
/// convicts that record by index.
#[test]
fn undecodable_supply_renders_failure_not_silent_hex() {
    // A record header promising more bytes than the run carries.
    let torn = vec![0, 0, 0, 9, 1, 2, 3];
    let lines = supply_lines(torn);
    assert_eq!(lines.len(), 1);
    assert!(
        lines[0].contains("supply run undecodable"),
        "torn framing must convict the run: {lines:?}"
    );

    // Valid record framing around bytes that are no version encoding.
    let mut framed = vec![0, 0, 0, 3];
    framed.extend_from_slice(&[0xff, 0xff, 0xff]);
    let lines = supply_lines(framed);
    assert_eq!(lines[0], "supply run: 1 record(s)");
    assert!(
        lines[1].contains("record 0 undecodable"),
        "a garbage version must convict its record: {lines:?}"
    );
}

/// The greeting's root-fan listing decodes to one line per child naming
/// its radix and full hash; bytes that are not a canonical listing
/// render the explicit failure instead.
#[test]
fn listing_decodes_children_and_convicts_garbage() {
    let children = vec![
        (0x3_u8, Hash([0xab; MERKLE_HASH_LEN])),
        (0xc_u8, Hash([0x01; MERKLE_HASH_LEN])),
    ];
    let body = encode_listing(&children);
    let lines = listing_lines(&body);
    assert_eq!(lines[0], "listing: 2 child(ren)");
    assert_eq!(
        lines[1],
        format!("  child 0x3: {}", "ab".repeat(MERKLE_HASH_LEN))
    );
    assert_eq!(
        lines[2],
        format!("  child 0xc: {}", "01".repeat(MERKLE_HASH_LEN))
    );

    let truncated = &body[..body.len() - 1];
    let lines = listing_lines(truncated);
    assert_eq!(lines.len(), 1);
    assert!(
        lines[0].contains("listing undecodable"),
        "a truncated listing must convict itself: {lines:?}"
    );
}

/// A nonempty query's children decode to one line per child naming its
/// radix and full hash, in wire order.
#[test]
fn query_children_decode_to_radix_and_hash() {
    let mut children = Vec::new();
    children.push(0x0_u8);
    children.extend_from_slice(&[0x22; MERKLE_HASH_LEN]);
    children.push(0xf_u8);
    children.extend_from_slice(&[0x9d; MERKLE_HASH_LEN]);
    let lines = query_lines(&children);
    assert_eq!(lines[0], "query: 2 child(ren)");
    assert_eq!(
        lines[1],
        format!("  child 0x0: {}", "22".repeat(MERKLE_HASH_LEN))
    );
    assert_eq!(
        lines[2],
        format!("  child 0xf: {}", "9d".repeat(MERKLE_HASH_LEN))
    );
}

/// The renderer decodes queries through the codec's own path, canonical
/// child order included: descending radixes convict the frame with an
/// explicit failure line, never silent hex.
#[test]
fn non_canonical_query_renders_failure_not_silent_hex() {
    let mut children = Vec::new();
    children.push(0xf_u8);
    children.extend_from_slice(&[0x9d; MERKLE_HASH_LEN]);
    children.push(0x0_u8);
    children.extend_from_slice(&[0x22; MERKLE_HASH_LEN]);
    let lines = query_lines(&children);
    assert_eq!(lines.len(), 1);
    assert!(
        lines[0].contains("query undecodable"),
        "descending children must convict the query: {lines:?}"
    );
}

/// The listing is held to the same canonical child order the handshake
/// enforces before building scope from it: out-of-order children render
/// an explicit conviction, never a quietly decoded tree.
#[test]
fn non_canonical_listing_renders_failure_not_silent_hex() {
    let children = vec![
        (0xc_u8, Hash([0x01; MERKLE_HASH_LEN])),
        (0x3_u8, Hash([0xab; MERKLE_HASH_LEN])),
    ];
    let body = encode_listing(&children);
    let lines = listing_lines(&body);
    assert_eq!(lines.len(), 1);
    assert!(
        lines[0].contains("listing not canonical"),
        "descending children must convict the listing: {lines:?}"
    );
}
