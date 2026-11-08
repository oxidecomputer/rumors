use proptest::prelude::*;

use super::*;

proptest! {
    /// A rate-limited queue holds its delay line plus one bandwidth-delay
    /// product of full-sized packets, and grows with both rate and round
    /// trip; its delay is exactly half the round trip.
    #[test]
    fn shaped_queue_covers_delay_line_and_buffer(
        bits in 1_000u64..100_000_000_000,
        micros in 0u64..1_000_000,
        factor in 1u64..10,
    ) {
        let rtt = Rtt::from_duration(Duration::from_micros(micros));
        let netem = Netem::for_direction(Rate::Bits(bits), rtt, 0);
        let bdp_packets = bits as f64 / 8.0 * rtt.seconds() / PACKET_BYTES;
        prop_assert!(netem.limit_packets as f64 >= 1.5 * bdp_packets + QUEUE_FLOOR_PACKETS as f64);
        prop_assert_eq!(netem.delay * 2, rtt.duration());
        let faster = Netem::for_direction(Rate::Bits(bits * factor), rtt, 0);
        prop_assert!(faster.limit_packets >= netem.limit_packets);
    }

    /// An uncapped queue holds a full window of segments and an
    /// acknowledgement for each, so it never overflows while the window
    /// bounds what is in flight.
    #[test]
    fn uncapped_queue_covers_window(window in 0u64..1 << 32) {
        let netem = Netem::for_direction(Rate::Uncapped, "10ms".parse().unwrap(), window);
        prop_assert!(netem.limit_packets as f64 >= 2.0 * window as f64 / SEGMENT_BYTES);
    }
}

/// The `tc` arguments name the delay in microseconds, the limit, and the
/// rate in bits, and leave the rate out on an uncapped direction.
#[test]
fn tc_arguments_name_every_parameter() {
    let shaped = Netem {
        delay: Duration::from_micros(50),
        rate: Rate::Bits(10_000_000),
        limit_packets: 40,
    };
    assert_eq!(
        shaped.to_string(),
        "root netem delay 50us limit 40 rate 10000000bit"
    );
    let uncapped = Netem {
        rate: Rate::Uncapped,
        ..shaped
    };
    assert_eq!(uncapped.to_string(), "root netem delay 50us limit 40");
}
