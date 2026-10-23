use proptest::prelude::*;

use super::{Sampling, PAST_POWER_OFFSET};

proptest! {
    /// The past-power sampling places a knob `PAST_POWER_OFFSET` above a power
    /// of two and its double twice that above the next, for every base, with
    /// the knob at least the base and at most twice it plus the offset.
    ///
    /// A buffer doubling at either power has just reallocated, and the knob
    /// stays within about a factor of two of the base, so the sample's cost
    /// tracks the base's.
    #[test]
    fn past_power_knobs_sit_just_past_a_power_of_two(base in 1usize..1 << 40, min in 1usize..64) {
        let knob = Sampling::PastPowerOfTwo.knob(base, min);
        let floor = base.max(min);
        prop_assert!(knob >= floor && knob <= 2 * floor + PAST_POWER_OFFSET);
        for (sample, offset) in [(knob, PAST_POWER_OFFSET), (knob << 1, 2 * PAST_POWER_OFFSET)] {
            let power = sample - offset;
            prop_assert!(power.is_power_of_two(), "{sample} is {offset} past {power}");
        }
    }

    /// A sampling's protocol token, and its guest-export words, read back as
    /// the same sampling, scales bit for bit.
    #[test]
    fn encodings_round_trip(scale in proptest::num::f64::POSITIVE | proptest::num::f64::NORMAL) {
        for sampling in [Sampling::Scaled(scale), Sampling::PastPowerOfTwo] {
            let back = Sampling::from_token(&sampling.token());
            prop_assert_eq!(
                back.map(|back| back.token()),
                Some(sampling.token())
            );
            let (tag, word) = sampling.to_words();
            prop_assert_eq!(
                Sampling::from_words(tag, word).map(|back| back.token()),
                Some(sampling.token())
            );
        }
    }
}

/// A token or a pair of words of any other form is refused rather than read
/// as some sampling.
#[test]
fn malformed_encodings_are_refused() {
    assert_eq!(Sampling::from_words(1, 1), None);
    assert_eq!(Sampling::from_words(2, 0), None);
    for token in [
        "",
        "scale:",
        "scale:3ff",
        "scale:3ff00000000000000",
        "scale:zzzzzzzzzzzzzzzz",
        "past-power",
        "1.0",
    ] {
        assert_eq!(Sampling::from_token(token), None, "{token:?}");
    }
}
