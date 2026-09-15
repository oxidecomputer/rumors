use super::*;
use crate::message::{PayloadCodec, PayloadDepthLimit};

use crate::tree::mirror::cbor::HeadError;

/// The record heads ahead of a record's content: the embedded-sequence
/// tag plus the byte-string head for `content` bytes.
fn record_heads(content: usize) -> usize {
    RECORD_TAG_LEN + cbor::head_len(content as u64)
}

/// Push's capacity check is eager and charges the record's whole item.
///
/// A record is admitted exactly when its heads plus its content fit the
/// wire's run byte cap, so a record item with length past `u32::MAX`
/// fails at record level rather than later at the run head.
#[test]
fn record_capacity_charges_the_whole_item() {
    assert!(checked_run_len(u32::MAX as usize).is_ok());
    for unshippable in [u32::MAX as usize + 1, u32::MAX as usize + 2] {
        let error = checked_run_len(unshippable)
            .expect_err("a record item past the run byte cap must fail");
        assert_eq!(error.len, unshippable);
    }
}

proptest::proptest! {
    /// Record sizing matches the bytes actually written for varied versions
    /// and payload lengths; the version's framing also matches serde encoding.
    #[test]
    fn record_len_matches_an_actual_push(
        ticks in proptest::collection::vec(0u32..1024, 0..=128),
        payload in proptest::collection::vec(proptest::num::u8::ANY, 0..1024),
    ) {
        use proptest::prelude::*;
        // Independent parties with different counts produce varied clock
        // shapes, including versions whose byte-string head needs widening.
        let mut version = crate::Version::new();
        for (index, ticks) in ticks.into_iter().enumerate() {
            version.ticks(&crate::tree::arb::nth_party(index), ticks);
        }
        let message = Message::new(payload);
        let mut run = LeafRun::new();
        run.push(&version, &message).expect("test records fit");
        prop_assert_eq!(run.encoded_len(), LeafRun::record_len(&version, &message));

        // Skip the record heads and version tag to compare the version's
        // hand-written byte-string framing with an independent serde writer.
        let mut serde_form = Vec::new();
        ciborium::ser::into_writer(&version, &mut serde_form).unwrap();
        let content = LeafRun::record_body_len(&version, &message);
        let at = record_heads(content) + VERSION_TAG_LEN;
        prop_assert_eq!(&run.as_bytes()[at..at + serde_form.len()], serde_form.as_slice());
    }
}

/// A pushed run round-trips through `from_encoded` and yields the same
/// records: the writer's record heads are exactly what the validator
/// chains over, one record at a time.
#[test]
fn pushed_runs_validate_and_iterate() {
    let mut version = crate::Version::new();
    version.tick(&crate::tree::arb::nth_party(1));
    let mut run = LeafRun::new();
    for payload in [1u64, 2, 3] {
        run.push(&version, &Message::new(payload))
            .expect("test records fit");
    }
    let decoded =
        LeafRun::from_encoded(run.as_bytes().to_vec()).expect("a pushed run is structurally valid");
    let payloads: Vec<u64> = decoded
        .records(PayloadCodec::new::<u64>(PayloadDepthLimit::default()))
        .map(|record| *record.expect("a pushed record decodes").1.arc::<u64>())
        .collect();
    assert_eq!(payloads, vec![1, 2, 3]);
    assert_eq!(decoded.record_count(), 3);
}

/// A run whose record opens with anything but the embedded-sequence tag,
/// or whose head is widened past shortest form, is rejected typed: the
/// deterministic contract holds inside runs too.
#[test]
fn malformed_record_heads_are_typed() {
    // A bare byte string where a tagged record belongs.
    let mut bytes = Vec::new();
    cbor::write_head(&mut bytes, MAJOR_BSTR, 1);
    bytes.push(0);
    assert!(matches!(
        LeafRun::from_encoded(bytes),
        Err(LeafRunError::NotARecord { .. })
    ));
    // A widened (non-shortest) byte-string head behind a valid tag.
    let mut bytes = Vec::new();
    cbor::write_tag(&mut bytes, TAG_CBOR_SEQUENCE);
    bytes.extend_from_slice(&[0x58, 0x01, 0x00]); // 1 spelled wide
    assert!(matches!(
        LeafRun::from_encoded(bytes),
        Err(LeafRunError::Head {
            source: HeadError::NotShortest,
            ..
        })
    ));
    // A record whose declared content overruns the run.
    let mut bytes = Vec::new();
    cbor::write_tag(&mut bytes, TAG_CBOR_SEQUENCE);
    cbor::write_head(&mut bytes, MAJOR_BSTR, 4);
    bytes.push(0);
    assert!(matches!(
        LeafRun::from_encoded(bytes),
        Err(LeafRunError::TruncatedRecord {
            len: 4,
            remaining: 1
        })
    ));
    // The empty run.
    assert!(matches!(
        LeafRun::from_encoded(Vec::new()),
        Err(LeafRunError::Empty)
    ));
}

/// The listing writer and the listing parser are inverses on every
/// canonical listing, and the parser holds keys strictly ascending:
/// the map's deterministic key order and the wire's canonical child
/// order are one rule.
#[test]
fn listings_round_trip_and_hold_canonical_order() {
    use proptest::prelude::*;
    proptest!(|(radixes in proptest::collection::btree_set(any::<u8>(), 0..=64))| {
        let children: Vec<(u8, Hash)> = radixes
            .iter()
            .map(|&radix| (radix, Hash([radix; MERKLE_HASH_LEN])))
            .collect();
        let mut bytes = Vec::new();
        write_listing(&mut bytes, &children);
        prop_assert_eq!(bytes.len(), listing_len(&children));
        let mut input = bytes.as_slice();
        let parsed = parse_listing_map(&mut input).expect("a written listing is canonical");
        prop_assert_eq!(parsed, children);
        prop_assert!(input.is_empty());
    });
}

/// A listing with a descending or repeated key is rejected with the
/// order violation, exactly like a wire query: an equal adjacent pair is
/// as non-canonical as a descent.
#[test]
fn unordered_listings_are_rejected() {
    for (previous, radix) in [(3u8, 3u8), (5, 2)] {
        let children = [
            (previous, Hash([0; MERKLE_HASH_LEN])),
            (radix, Hash([1; MERKLE_HASH_LEN])),
        ];
        let mut bytes = Vec::new();
        write_listing(&mut bytes, &children);
        let mut input = bytes.as_slice();
        assert_eq!(
            parse_listing_map(&mut input),
            Err(ListingIssue::Order(QueryOrderError { previous, radix })),
        );
    }
}
