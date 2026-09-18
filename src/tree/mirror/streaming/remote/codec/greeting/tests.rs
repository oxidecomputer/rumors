use super::*;

use proptest::prelude::*;

use crate::tree::typed::{Hash, hash::MERKLE_HASH_LEN};

/// A greeting with a real version and a caller-chosen root listing.
fn sample(listing: Vec<(u8, Hash)>) -> Greeting {
    let mut version = crate::Version::new();
    version.tick(&crate::tree::arb::nth_party(1));
    Greeting {
        version,
        set_len: 7,
        max_version_bytes: 4096,
        payload_depth_limit: 300,
        target_message_size: 1 << 20,
        listing,
    }
}

/// Generate integers across every canonical CBOR head width, including
/// exact width transitions.
fn arb_uint() -> impl Strategy<Value = u64> {
    prop_oneof![
        4 => any::<u64>(),
        1 => prop::sample::select(vec![
            0,
            23,
            24,
            u8::MAX.into(),
            u64::from(u8::MAX) + 1,
            u16::MAX.into(),
            u64::from(u16::MAX) + 1,
            u32::MAX.into(),
            u64::from(u32::MAX) + 1,
            u64::MAX,
        ]),
    ]
}

/// Extract the rejection from a deliberately malformed greeting.
fn rejection(bytes: &[u8]) -> GreetingError {
    match parse_greeting(bytes) {
        Err(error) => error,
        Ok(_) => panic!("the malformed greeting was accepted"),
    }
}

proptest::proptest! {
    /// Greeting encode and parse are inverses across versions, listings,
    /// and integer values at every head width.
    #[test]
    fn greetings_round_trip(
        version in crate::tree::arb::arb_version(),
        listing in proptest::collection::btree_map(any::<u8>(), any::<[u8; MERKLE_HASH_LEN]>(), 0..=256),
        set_len in arb_uint(),
        max_version_bytes in arb_uint(),
        payload_depth_limit in arb_uint(),
        target_message_size in arb_uint(),
    ) {
        let greeting = Greeting {
            version,
            listing: listing.into_iter().map(|(radix, hash)| (radix, Hash::from(hash))).collect(),
            set_len,
            max_version_bytes,
            payload_depth_limit,
            target_message_size,
        };
        let item = encode_greeting(&greeting);
        // Strip the embedded-item tag and byte-string head, the layer the
        // async reader consumes.
        let mut input = item.as_slice();
        let head = cbor::read_head(&mut input).expect("the item opens with a head");
        prop_assert_eq!((head.major, head.value), (MAJOR_TAG, TAG_EMBEDDED_ITEM));
        let head = cbor::read_head(&mut input).expect("the tag wraps a byte string");
        prop_assert_eq!(head.major, MAJOR_BSTR);
        prop_assert_eq!(head.value as usize, input.len());
        let parsed = parse_greeting(input).expect("a written greeting parses");
        prop_assert_eq!(parsed.version, greeting.version);
        prop_assert_eq!(parsed.set_len, greeting.set_len);
        prop_assert_eq!(parsed.max_version_bytes, greeting.max_version_bytes);
        prop_assert_eq!(parsed.payload_depth_limit, greeting.payload_depth_limit);
        prop_assert_eq!(parsed.target_message_size, greeting.target_message_size);
        prop_assert_eq!(parsed.listing, greeting.listing);
    }
}

/// Greeting keys are ordered by their complete canonical CBOR encodings.
#[test]
fn greeting_keys_are_in_deterministic_order() {
    let encoded: Vec<Vec<u8>> = FIELDS
        .iter()
        .map(|&field| {
            let mut bytes = Vec::new();
            super::write_key(&mut bytes, field);
            bytes
        })
        .collect();
    assert!(encoded.windows(2).all(|pair| pair[0] < pair[1]));
}

proptest::proptest! {
    /// Removing, renaming, duplicating, or exchanging fields is rejected;
    /// a valid greeting also rejects every nonempty trailing byte sequence.
    #[test]
    fn greeting_key_roster_is_exact(
        index in 0usize..FIELDS.len(),
        distance in 1usize..FIELDS.len(),
        suffix in proptest::collection::vec(proptest::num::u8::ANY, 1..32),
    ) {
        use ciborium::Value;
        let map = greeting_map(&sample(Vec::new()));
        let Value::Map(fields) = ciborium::de::from_reader(map.as_slice()).unwrap() else {
            panic!("a greeting is a map");
        };
        let mut removed = fields.clone();
        removed.remove(index);
        let mut renamed = fields.clone();
        renamed[index].0 = Value::Text("unknown".into());
        let mut duplicated = fields.clone();
        duplicated.insert(index, fields[index].clone());
        let mut swapped = fields.clone();
        swapped.swap(index, (index + distance) % fields.len());

        // Ciborium retains map entry order. Re-encoding the unmodified map
        // must preserve the fixture, so rejection is attributable to each edit.
        let encode = |fields| {
            let mut bytes = Vec::new();
            ciborium::ser::into_writer(&Value::Map(fields), &mut bytes).unwrap();
            bytes
        };
        prop_assert_eq!(encode(fields), map.clone());
        let removed_error = rejection(&encode(removed));
        let removed_is_precise = matches!(
            removed_error,
            GreetingError::Structure(GreetingStructureError::Map { expected, actual })
                if expected == FIELDS.len()
                    && actual.major == cbor::MAJOR_MAP
                    && actual.value == (FIELDS.len() - 1) as u64
        );
        prop_assert!(removed_is_precise, "wrong removal diagnostic: {removed_error:?}");

        let renamed_error = rejection(&encode(renamed));
        let renamed_is_precise = matches!(
            renamed_error,
            GreetingError::Structure(GreetingStructureError::Key { expected, .. })
                if expected == FIELDS[index]
        );
        prop_assert!(renamed_is_precise, "wrong renamed-key diagnostic: {renamed_error:?}");

        let duplicated_error = rejection(&encode(duplicated));
        let duplicated_is_precise = matches!(
            duplicated_error,
            GreetingError::Structure(GreetingStructureError::Map { expected, actual })
                if expected == FIELDS.len()
                    && actual.major == cbor::MAJOR_MAP
                    && actual.value == (FIELDS.len() + 1) as u64
        );
        prop_assert!(duplicated_is_precise, "wrong duplicate diagnostic: {duplicated_error:?}");

        let first_changed = index.min((index + distance) % FIELDS.len());
        let swapped_error = rejection(&encode(swapped));
        let swapped_is_precise = matches!(
            swapped_error,
            GreetingError::Structure(GreetingStructureError::Key { expected, .. })
                if expected == FIELDS[first_changed]
        );
        prop_assert!(swapped_is_precise, "wrong swapped-key diagnostic: {swapped_error:?}");

        let suffix_len = suffix.len();
        let trailing = [map, suffix].concat();
        let trailing_error = rejection(&trailing);
        let trailing_is_precise = matches!(
            trailing_error,
            GreetingError::Structure(GreetingStructureError::Trailing { remaining })
                if remaining == suffix_len
        );
        prop_assert!(trailing_is_precise, "wrong trailing-byte diagnostic: {trailing_error:?}");
    }
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
    let greeting = sample(vec![(4, Hash::from([7; MERKLE_HASH_LEN]))]);
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

/// A greeting preserves the listing parser's typed ordering defect.
#[test]
fn greeting_listing_order_is_enforced() {
    // The encoder trusts its caller, so an unsorted listing synthesizes
    // the wire violation directly.
    let greeting = sample(vec![
        (9, Hash::from([1; MERKLE_HASH_LEN])),
        (5, Hash::from([2; MERKLE_HASH_LEN])),
    ]);
    let item = encode_greeting(&greeting);
    let mut input = item.as_slice();
    cbor::read_head(&mut input).expect("tag head");
    cbor::read_head(&mut input).expect("bstr head");
    assert!(matches!(
        parse_greeting(input),
        Err(GreetingError::Listing(ListingIssue::Order(
            QueryOrderError {
                previous: 9,
                radix: 5
            }
        )))
    ));
}

// Defensive-variant exemption: `Structure(ItemTooLarge)` in the greeting
// reader and `Structure(VersionTooLarge)` in the map parser deliberately
// have no construction tests. Each guards a
// u64-to-usize length conversion that cannot fail on a 64-bit host; only
// a 32-bit target (e.g. wasm32) can present a declarable length past
// `usize::MAX`, and this suite has no 32-bit test host.

/// Find the `skip`-th occurrence of `needle` in `haystack`.
fn find(haystack: &[u8], needle: &[u8], skip: usize) -> Option<usize> {
    haystack
        .windows(needle.len())
        .enumerate()
        .filter(|(_, window)| *window == needle)
        .map(|(at, _)| at)
        .nth(skip)
}
