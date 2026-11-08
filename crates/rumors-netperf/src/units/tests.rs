use proptest::prelude::*;

use super::*;

proptest! {
    /// Every rate prints as text that parses back to the same rate, so a
    /// value written to a results file reads back into the grid unchanged.
    #[test]
    fn rate_text_round_trips(bits in 1u64..=u64::MAX / 2, uncapped in any::<bool>()) {
        let rate = if uncapped { Rate::Uncapped } else { Rate::Bits(bits) };
        prop_assert_eq!(rate.to_string().parse::<Rate>().unwrap(), rate);
    }

    /// Every link rate, symmetric or not, round-trips through its text.
    #[test]
    fn link_rate_text_round_trips(a in 1u64..1 << 40, b in 1u64..1 << 40) {
        let link = LinkRate { a_to_b: Rate::Bits(a), b_to_a: Rate::Bits(b) };
        prop_assert_eq!(link.to_string().parse::<LinkRate>().unwrap(), link);
    }

    /// Every round-trip time in whole microseconds round-trips through its
    /// text.
    #[test]
    fn rtt_text_round_trips(micros in 0u64..1 << 40) {
        let rtt = Rtt::from_duration(Duration::from_micros(micros));
        prop_assert_eq!(rtt.to_string().parse::<Rtt>().unwrap(), rtt);
    }

    /// Every byte count round-trips through `format_bytes`.
    #[test]
    fn byte_text_round_trips(bytes in any::<u64>()) {
        prop_assert_eq!(parse_bytes(&format_bytes(bytes)).unwrap(), bytes);
    }
}

/// The units scale as their names say: decimal for rates and the `kB`
/// family, binary for the `KiB` family, and `0.1ms` is a hundred
/// microseconds.
#[test]
fn units_scale_as_named() {
    assert_eq!("100Mbit".parse::<Rate>().unwrap(), Rate::Bits(100_000_000));
    assert_eq!(
        "1.5gbit".parse::<Rate>().unwrap(),
        Rate::Bits(1_500_000_000)
    );
    assert_eq!(
        "0.1ms".parse::<Rtt>().unwrap().duration(),
        Duration::from_micros(100)
    );
    assert_eq!(parse_bytes("32MiB").unwrap(), 32 << 20);
    assert_eq!(parse_bytes("4MB").unwrap(), 4_000_000);
    assert_eq!(parse_count("1e6").unwrap(), 1_000_000);
    assert!(parse_count("1.5").is_err());
    assert!("fast".parse::<Rate>().is_err());
}
