//! Tests for generating network identifiers from caller-supplied randomness.

use std::panic::{AssertUnwindSafe, catch_unwind};

use rand::RngCore;

use super::{NETWORK_BYTES, NETWORK_DRAW_ATTEMPTS, Network};

/// An RNG that returns zero for a chosen number of draws, then a fixed value.
struct ZeroPrefix {
    /// Zero-valued draws remaining before the fixed value is returned.
    zeros: usize,
    /// Number of completed draws.
    draws: usize,
}

impl ZeroPrefix {
    /// Construct an RNG with `zeros` reserved draws before a valid one.
    fn new(zeros: usize) -> Self {
        Self { zeros, draws: 0 }
    }
}

impl RngCore for ZeroPrefix {
    /// Return the next four bytes of the scripted draw sequence.
    fn next_u32(&mut self) -> u32 {
        let mut bytes = [0; size_of::<u32>()];
        self.fill_bytes(&mut bytes);
        u32::from_le_bytes(bytes)
    }

    /// Return the next eight bytes of the scripted draw sequence.
    fn next_u64(&mut self) -> u64 {
        let mut bytes = [0; size_of::<u64>()];
        self.fill_bytes(&mut bytes);
        u64::from_le_bytes(bytes)
    }

    /// Fill one draw with zero or the fixed valid byte, following the script.
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.draws += 1;
        if self.zeros == 0 {
            dest.fill(1);
        } else {
            self.zeros -= 1;
            dest.fill(0);
        }
    }
}

/// Every permitted retry position accepts the first nonzero draw.
#[test]
fn each_allowed_draw_can_supply_the_network() {
    for zero_draws in 0..NETWORK_DRAW_ATTEMPTS {
        let mut rng = ZeroPrefix::new(zero_draws);

        let network = Network::from_rng(&mut rng);

        assert_eq!(network.to_bytes(), [1; NETWORK_BYTES]);
        assert_eq!(rng.draws, zero_draws + 1);
    }
}

/// Exhausting the bounded retries rejects the RNG instead of drawing again.
#[test]
fn exhausting_network_draws_panics() {
    let mut rng = ZeroPrefix::new(NETWORK_DRAW_ATTEMPTS);

    catch_unwind(AssertUnwindSafe(|| Network::from_rng(&mut rng)))
        .expect_err("two reserved draws must reject the random source");

    assert_eq!(rng.draws, NETWORK_DRAW_ATTEMPTS);
}
