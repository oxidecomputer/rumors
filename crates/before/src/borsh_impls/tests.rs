use borsh::BorshDeserialize;
use proptest::prelude::*;

use crate::testing::optrace::{step_impl, world_strategy};
use crate::{Clock, Party, Version};

/// Regression: a `Party` grown by [`join`](Party::join) — the operation
/// [`reclaim`](crate::bookmark) drives on a reboot — must survive the borsh
/// wire round-trip.
///
/// `fork; fork; join` reunites two quarter-regions into `(0, 1)`, normalizing
/// the build buffer's tree to a smaller one. The buffer once kept the bits it
/// shed, so [`as_bytes`](Party::as_bytes) — which borsh serializes — carried
/// trailing garbage that the peer's [`Party::decode`] rejected as
/// `TrailingBits`. On the wire that silently dropped a donated identity; here
/// it is a one-shot witness that the dead bits are now zeroed.
#[test]
fn joined_party_roundtrips_through_borsh() {
    let mut left = Party::seed();
    let right = left.fork(); // left = (1, 0), right = (0, 1)
    let mut right = right;
    let rb = right.fork(); // right = (0, (1, 0)), rb = (0, (0, 1))
    right.join(rb).expect("the two quarters are disjoint");

    assert_eq!(
        right,
        "(0, 1)".parse().unwrap(),
        "the quarters reunite to (0, 1)"
    );
    assert_eq!(
        right.as_bytes(),
        right.encode().as_slice(),
        "stored bytes must be canonical after a normalizing join",
    );
    let bytes = borsh::to_vec(&right).expect("serialize");
    assert_eq!(
        Party::try_from_slice(&bytes).expect("a joined party must decode"),
        right,
    );
}

proptest! {
    /// Every `Party`/`Version`/`Clock` reachable by an arbitrary impl-driven
    /// history (`fork`/`join`/`sync`/`tick` via [`step_impl`]) round-trips
    /// through borsh — the on-wire form the gossip protocol ships.
    ///
    /// Drives the impl's own operations, not oracle-lowered values, so it
    /// covers the reused-buffer path that the per-view equivalence test also
    /// guards.
    #[test]
    fn borsh_roundtrips_over_impl_history(ops in world_strategy()) {
        let mut imp = vec![Clock::seed()];
        for op in &ops {
            step_impl(&mut imp, op);
        }
        for c in &imp {
            let (p, v) = (c.party(), c.version());
            let party_bytes = borsh::to_vec(p).unwrap();
            let version_bytes = borsh::to_vec(v).unwrap();
            let clock_bytes = borsh::to_vec(c).unwrap();

            prop_assert_eq!(party_bytes.as_slice(), p.as_bytes());
            prop_assert_eq!(version_bytes.as_slice(), v.as_bytes());
            prop_assert_eq!(&clock_bytes, &c.encode());

            prop_assert_eq!(&Party::try_from_slice(&party_bytes).unwrap(), p);
            prop_assert_eq!(&Version::try_from_slice(&version_bytes).unwrap(), v);
            let back = Clock::try_from_slice(&clock_bytes).unwrap();
            prop_assert_eq!(back.party(), p);
            prop_assert_eq!(back.version(), v);
        }
    }
}
