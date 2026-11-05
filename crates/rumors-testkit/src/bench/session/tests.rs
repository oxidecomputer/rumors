//! Fixture messages' encoded sizes, against an independent CBOR encoder.

use proptest::prelude::*;

use super::{message, text_chars};

/// The bytes `text` takes encoded as a CBOR text string.
fn encoded(text: &str) -> usize {
    let mut out = Vec::new();
    ciborium::into_writer(text, &mut out).expect("a string encodes");
    out.len()
}

proptest! {
    /// A message size names the characters of the one text string whose
    /// CBOR encoding takes exactly that many bytes, or none where no
    /// string's does, and a fixture message built at that size encodes in
    /// exactly that many bytes.
    ///
    /// A CBOR head takes one, two, three, five, or nine bytes, so the
    /// strings that could encode in `bytes` bytes are those of `bytes`
    /// less each head's length; the encoder judges each candidate.
    #[test]
    fn messages_encode_in_their_shape_s_size(bytes in 1..70_000_usize, index in 0..1_000_000_u64) {
        let fitting: Vec<usize> = [1, 2, 3, 5, 9]
            .into_iter()
            .filter_map(|head| bytes.checked_sub(head))
            .filter(|&chars| encoded(&"x".repeat(chars)) == bytes)
            .collect();
        prop_assert!(fitting.len() <= 1, "one string length per size: {fitting:?}");
        prop_assert_eq!(text_chars(bytes), fitting.first().copied());
        if let Some(chars) = fitting.first()
            && *chars > 7
        {
            prop_assert_eq!(encoded(&message('a', index, bytes)), bytes);
        }
    }
}
