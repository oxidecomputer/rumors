//! Checks the snapshot packing the history probes' parameters use.

use super::{pack, unpack, Snapshot};

/// Every snapshot, in number order.
const ALL: [Snapshot; 5] = [
    Snapshot::Start,
    Snapshot::Middle,
    Snapshot::End,
    Snapshot::Divergent,
    Snapshot::Top,
];

/// Unpacking inverts packing for every pair and triple of snapshots, and a
/// parameter packed at one arity never unpacks at the other, so a probe
/// cannot read a parameter meant for a probe of another arity.
#[test]
fn unpack_inverts_pack_at_its_own_arity_alone() {
    for x in ALL {
        for y in ALL {
            let pair = pack([x, y]);
            assert_eq!(unpack::<2>(pair), Some([x, y]));
            assert_eq!(unpack::<3>(pair), None, "{x:?} {y:?}");
            for z in ALL {
                let triple = pack([x, y, z]);
                assert_eq!(unpack::<3>(triple), Some([x, y, z]));
                assert_eq!(unpack::<2>(triple), None, "{x:?} {y:?} {z:?}");
            }
        }
    }
}

/// Every four-bit number that names no snapshot unpacks to nothing.
#[test]
fn unpack_rejects_an_unknown_snapshot() {
    for low in (0..16).filter(|number| ALL.iter().all(|&s| s as u64 != *number)) {
        assert_eq!(unpack::<1>(low), None, "snapshot number {low}");
    }
}
