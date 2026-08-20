use super::*;

use crate::tree::typed::{Hash, hash::MERKLE_HASH_LEN};

fn sample(listing: Vec<(u8, Hash)>) -> Greeting {
    let mut version = crate::Version::new();
    version.tick(&crate::tree::arb::nth_party(1));
    Greeting {
        version,
        set_len: 7,
        max_version_bytes: 4096,
        target_message_size: 1 << 20,
        listing,
    }
}

/// Greeting encode and parse are inverses, listing shapes included:
/// empty, small-radix, and large-radix listings all round-trip through
/// the one wire spelling.
#[test]
fn greetings_round_trip() {
    for listing in [
        Vec::new(),
        vec![(0, Hash([1; MERKLE_HASH_LEN]))],
        vec![
            (3, Hash([1; MERKLE_HASH_LEN])),
            (24, Hash([2; MERKLE_HASH_LEN])),
            (255, Hash([3; MERKLE_HASH_LEN])),
        ],
    ] {
        let greeting = sample(listing);
        let item = encode_greeting(&greeting);
        // Strip the embedded-item tag and byte-string head, the layer the
        // async reader consumes.
        let mut input = item.as_slice();
        let head = cbor::read_head(&mut input).expect("the item opens with a head");
        assert_eq!((head.major, head.value), (MAJOR_TAG, TAG_EMBEDDED_ITEM));
        let head = cbor::read_head(&mut input).expect("the tag wraps a byte string");
        assert_eq!(head.major, MAJOR_BSTR);
        assert_eq!(head.value as usize, input.len());
        let parsed = parse_greeting(input).expect("a written greeting parses");
        assert_eq!(parsed.version, greeting.version);
        assert_eq!(parsed.set_len, greeting.set_len);
        assert_eq!(parsed.max_version_bytes, greeting.max_version_bytes);
        assert_eq!(parsed.target_message_size, greeting.target_message_size);
        assert_eq!(parsed.listing, greeting.listing);
    }
}

/// The greeting's map admits exactly one spelling: a missing or
/// out-of-order key, a wrong protocol magic, or trailing bytes are each
/// rejected — one spelling per greeting is the deterministic contract.
#[test]
fn greeting_key_roster_is_exact() {
    let greeting = sample(Vec::new());
    let item = encode_greeting(&greeting);
    let mut input = item.as_slice();
    cbor::read_head(&mut input).expect("tag head");
    cbor::read_head(&mut input).expect("bstr head");
    let map = input.to_vec();

    // Renaming the protocol magic's value breaks the greeting.
    let mut wrong_magic = map.clone();
    let at = find(&wrong_magic, b"rumors", 0).expect("the magic value is present");
    wrong_magic[at] = b'x';
    assert!(matches!(
        parse_greeting(&wrong_magic),
        Err(GreetingError::Shape(_))
    ));

    // Renaming a key breaks the roster.
    let mut wrong_key = map.clone();
    let at = find(&wrong_key, b"set_len", 0).expect("the key is present");
    wrong_key[at] = b'x';
    assert!(matches!(
        parse_greeting(&wrong_key),
        Err(GreetingError::Shape(_))
    ));

    // Trailing bytes are rejected.
    let mut trailing = map.clone();
    trailing.push(0);
    assert!(matches!(
        parse_greeting(&trailing),
        Err(GreetingError::Shape(_))
    ));
}

/// A listing whose content ends inside a hash is rejected as the typed
/// listing issue.
///
/// The map declares its listing entries up front; content that runs out
/// inside an entry's digest bytes must surface
/// [`GreetingError::Listing`] with the listing's own truncation, never a
/// panic and never a partial listing.
#[test]
fn truncated_listing_hash_is_a_typed_listing_issue() {
    let greeting = sample(vec![(4, Hash([7; MERKLE_HASH_LEN]))]);
    let item = encode_greeting(&greeting);
    let mut input = item.as_slice();
    cbor::read_head(&mut input).expect("tag head");
    cbor::read_head(&mut input).expect("bstr head");
    // Cut one byte deeper than the listing's end (the next key's one-byte
    // text head sits just before the key text): the kept bytes stop
    // inside the entry's digest.
    let at = find(input, b"set_len", 0).expect("the key is present");
    let cut = &input[..at - 2];
    assert!(matches!(
        parse_greeting(cut),
        Err(GreetingError::Listing(ListingIssue::Truncated))
    ));
}

/// A version atom whose bytes are not one canonical version encoding is
/// rejected as the typed version defect.
///
/// The atom's tag and byte string parse, so the failure is the content's
/// own: [`GreetingError::Version`] carrying the decoder's verdict.
#[test]
fn undecodable_version_atom_is_a_typed_version_defect() {
    let greeting = sample(Vec::new());
    let item = encode_greeting(&greeting);
    let mut input = item.as_slice();
    cbor::read_head(&mut input).expect("tag head");
    cbor::read_head(&mut input).expect("bstr head");
    let mut map = input.to_vec();

    // Locate the version atom's content: after the "version" key text
    // ride the version tag's head, the byte string's head, and then the
    // encoded version itself; saturate those bytes.
    let at = find(&map, b"version", 0).expect("the key is present");
    let mut cursor = &map[at + b"version".len()..];
    let before_heads = cursor.len();
    cbor::read_head(&mut cursor).expect("the version tag's head");
    let head = cbor::read_head(&mut cursor).expect("the version string's head");
    let content_at = at + b"version".len() + (before_heads - cursor.len());
    assert!(head.value > 0, "a ticked version encodes to content bytes");
    for byte in &mut map[content_at..content_at + head.value as usize] {
        *byte = 0xFF;
    }
    assert!(matches!(
        parse_greeting(&map),
        Err(GreetingError::Version(_))
    ));
}

/// A widened spelling of a greeting value head is rejected as the
/// codec's own shortest-form violation.
///
/// The greeting is deterministic-encoding CBOR, so a head wider than
/// its value requires is a spelling the encoder never writes, even
/// though the value it carries is the right one.
#[test]
fn widened_value_spelling_is_rejected() {
    // Build the malformed map directly: copy the canonical map bytes
    // and re-spell the one-byte `set_len` value as the widened
    // two-byte `0x18 <v>` form. Operating on the bare map (the layer
    // `parse_greeting` consumes) needs no fix-up of an embedding
    // byte-string head.
    let greeting = sample(Vec::new());
    let map = greeting_map(&greeting);
    let at = find(&map, b"set_len", 0).expect("the key is present");
    // The value head follows the key's text bytes.
    let value_at = at + b"set_len".len();
    assert_eq!(
        map[value_at],
        u8::try_from(greeting.set_len).expect("the fixture's set_len is small"),
        "the fixture's set_len spells as one canonical head byte"
    );
    let mut widened = Vec::with_capacity(map.len() + 1);
    widened.extend_from_slice(&map[..value_at]);
    widened.extend_from_slice(&[0x18, map[value_at]]);
    widened.extend_from_slice(&map[value_at + 1..]);
    assert!(matches!(
        parse_greeting(&widened),
        Err(GreetingError::Head(HeadError::NotShortest))
    ));
}

/// A greeting listing violating strictly ascending radix order is
/// rejected as the codec's own order violation, the same class a wire
/// query reports.
#[test]
fn greeting_listing_order_is_enforced() {
    // The encoder trusts its caller, so an unsorted listing synthesizes
    // the wire violation directly.
    let greeting = sample(vec![
        (9, Hash([1; MERKLE_HASH_LEN])),
        (5, Hash([2; MERKLE_HASH_LEN])),
    ]);
    let item = encode_greeting(&greeting);
    let mut input = item.as_slice();
    cbor::read_head(&mut input).expect("tag head");
    cbor::read_head(&mut input).expect("bstr head");
    assert!(matches!(
        parse_greeting(input),
        Err(GreetingError::Order(QueryOrderError {
            previous: 9,
            radix: 5
        }))
    ));
}

/// Find the `skip`-th occurrence of `needle` in `haystack`.
fn find(haystack: &[u8], needle: &[u8], skip: usize) -> Option<usize> {
    haystack
        .windows(needle.len())
        .enumerate()
        .filter(|(_, window)| *window == needle)
        .map(|(at, _)| at)
        .nth(skip)
}
