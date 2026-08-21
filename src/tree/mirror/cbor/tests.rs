use proptest::prelude::*;

use super::*;

/// Every head a writer emits reads back as the same `(major, value)` pair,
/// occupies exactly `head_len` bytes, and leaves trailing input untouched:
/// the writer and the canonical reader are inverses.
#[test]
fn heads_round_trip_at_their_stated_width() {
    proptest!(|(major in 0u8..7, value: u64, trailing: Vec<u8>)| {
        let mut bytes = Vec::new();
        write_head(&mut bytes, major, value);
        prop_assert_eq!(bytes.len(), head_len(value));
        bytes.extend_from_slice(&trailing);
        let mut input = bytes.as_slice();
        let head = read_head(&mut input).expect("a written head is canonical");
        prop_assert_eq!(head, Head { major, value });
        prop_assert_eq!(input, trailing.as_slice());
    });
}

/// A head whose argument is wider than its value requires is rejected as
/// non-shortest-form: the deterministic contract admits one spelling per
/// value.
#[test]
fn widened_heads_are_rejected() {
    proptest!(|(major in 0u8..7, value: u64)| {
        let widths: &[(u8, usize)] = &[(24, 1), (25, 2), (26, 4), (27, 8)];
        for &(info, width) in widths {
            // Only widths strictly larger than the shortest form are
            // non-canonical spellings of this value.
            if width < head_len(value) || value >= 1u64 << (8 * width as u32).min(63) {
                continue;
            }
            let mut bytes = vec![(major << 5) | info];
            bytes.extend_from_slice(&value.to_be_bytes()[8 - width..]);
            let mut input = bytes.as_slice();
            prop_assert_eq!(read_head(&mut input), Err(HeadError::NotShortest));
        }
    });
}

/// Indefinite-length and reserved additional-information heads are
/// rejected: the wire is definite-length, deterministic CBOR only.
#[test]
fn indefinite_and_reserved_heads_are_rejected() {
    for major in 0u8..8 {
        for (info, expected) in [
            (28, HeadError::Reserved),
            (29, HeadError::Reserved),
            (30, HeadError::Reserved),
            (31, HeadError::Indefinite),
        ] {
            let bytes = [(major << 5) | info];
            let mut input = bytes.as_slice();
            assert_eq!(read_head(&mut input), Err(expected));
        }
    }
}

/// A head cut anywhere before its final byte is a truncation, and the
/// input is left unconsumed.
#[test]
fn truncated_heads_are_rejected() {
    proptest!(|(major in 0u8..7, value: u64)| {
        let mut bytes = Vec::new();
        write_head(&mut bytes, major, value);
        for cut in 0..bytes.len() {
            let mut input = &bytes[..cut];
            let before = input;
            prop_assert_eq!(read_head(&mut input), Err(HeadError::Truncated));
            prop_assert_eq!(input, before);
        }
    });
}

/// The async head reader agrees with the slice reader on every written
/// head — the two ingress paths cannot drift — and reports a clean
/// end-of-stream before the first byte as `None`.
#[test]
fn async_heads_match_the_slice_reader() {
    proptest!(|(major in 0u8..7, value: u64)| {
        let mut bytes = Vec::new();
        write_head(&mut bytes, major, value);
        let head = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("runtime builds")
            .block_on(async {
                let mut read = bytes.as_slice();
                read_head_async(&mut read).await
            })
            .expect("a written head is canonical")
            .expect("a nonempty stream yields a head");
        prop_assert_eq!(head, Head { major, value });
    });
    let none = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("runtime builds")
        .block_on(async {
            let mut read: &[u8] = &[];
            read_head_async(&mut read).await
        })
        .expect("an empty stream is a clean close");
    assert!(none.is_none());
}
