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
