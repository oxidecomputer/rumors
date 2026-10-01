//! Properties of the canonical public text forms.

use proptest::prelude::*;

use crate::error::ParseValue;
use crate::testing::bridge::{from_oracle_party, from_oracle_version};
use crate::testing::generators::{arb_magnitude, arb_oracle_party_nonempty, arb_oracle_version};
use crate::{Count, Party, Version};

/// Apply both hexadecimal letter cases within one spelling when possible.
fn mixed_case(hex: &str) -> String {
    hex.bytes()
        .enumerate()
        .map(|(index, byte)| {
            if index.is_multiple_of(2) {
                byte.to_ascii_uppercase()
            } else {
                byte.to_ascii_lowercase()
            }
        })
        .map(char::from)
        .collect()
}

proptest! {
    /// Party text is canonical lowercase hex; parsing accepts either letter
    /// case and recovers the same party.
    #[test]
    fn party_text_round_trips(oracle in arb_oracle_party_nonempty()) {
        let party = from_oracle_party(&oracle);
        let canonical = party.to_string();
        prop_assert_eq!(canonical.parse::<Party>().unwrap(), party.dangerously_alias());
        prop_assert_eq!(mixed_case(&canonical).parse::<Party>().unwrap(), party.dangerously_alias());
        prop_assert_eq!(party.to_string(), canonical.to_ascii_lowercase());
    }

    /// Version text is canonical lowercase hex; parsing accepts either letter
    /// case and recovers the same version.
    #[test]
    fn version_text_round_trips(oracle in arb_oracle_version()) {
        let version = from_oracle_version(&oracle);
        let canonical = version.to_string();
        prop_assert!(canonical.parse::<Version>().unwrap() == version);
        prop_assert!(mixed_case(&canonical).parse::<Version>().unwrap() == version);
        prop_assert_eq!(version.to_string(), canonical.to_ascii_lowercase());
    }

    /// Count text is canonical unsigned decimal at arbitrary widths.
    #[test]
    fn count_text_round_trips(value in arb_magnitude()) {
        let count = Count(value);
        let text = count.to_string();
        prop_assert_eq!(text.parse::<Count>().unwrap(), count);
        prop_assert!(text == "0" || !text.starts_with('0'));
    }

    /// Prefixes and surrounding whitespace are rejected for every text form,
    /// even when the enclosed value is otherwise valid.
    #[test]
    fn decorations_are_rejected(oracle in arb_oracle_version()) {
        let version = from_oracle_version(&oracle);
        let hex = version.to_string();
        for decorated in [
            format!("0x{hex}"),
            format!(" {hex}"),
            format!("{hex} "),
        ] {
            let result = decorated.parse::<Version>().map(|_| ());
            prop_assert!(matches!(result, Err(ParseValue::InvalidSyntax)));
        }
    }
}

/// A syntactically valid hexadecimal spelling whose bytes are not a canonical
/// version is distinguished from malformed text.
#[test]
fn invalid_canonical_bytes_are_an_encoding_error() {
    assert!(matches!(
        "ff".parse::<Version>(),
        Err(ParseValue::InvalidEncoding(_))
    ));
}

/// Count syntax rejects empty text, signs, leading zeroes, and whitespace.
#[test]
fn count_rejects_noncanonical_decimal() {
    for text in ["", "+1", "-1", "00", "01", " 1", "1 ", "1_0"] {
        assert!(matches!(
            text.parse::<Count>(),
            Err(ParseValue::InvalidSyntax)
        ));
    }
}
