//! Probes of a join that re-anchors one wide payload at every level.
//!
//! The re-anchoring spine of depth `d` is a left spine whose right child at
//! every level is the leaf pair `(0, 1)`: height 1 on the right-right
//! quarter of each level's interval, zero elsewhere. Joining it with a flat
//! version high enough to dominate it emits one leaf, recovering the same
//! wide payload once at every level of the spine.

use before::testing::meter::registry::Shape;
use before::{Party, Version};

use crate::measured_per;

/// Builds the re-anchoring spine of `depth` levels.
///
/// Each level's quarter is one owned cell of its own party, so one tick
/// raises exactly that cell to height 1; the join of those versions holds
/// height 1 on every level's quarter.
fn spine(depth: usize) -> Version {
    (0..depth)
        .map(|level| {
            let mut version = Version::new();
            quarter(level).tick(&mut version);
            version
        })
        .fold(Version::new(), |spine, level| &spine | &level)
}

/// The party owning the right-right quarter of the interval `level` left
/// steps below the root.
///
/// Spelled in the party encoding (two presence bits per node, `00` for an
/// owned cell, a `1` marker, then zero padding) and decoded through the
/// validating `Party::decode`.
fn quarter(level: usize) -> Party {
    let mut bits: Vec<bool> = Vec::with_capacity(2 * level + 6);
    for _ in 0..level {
        bits.extend([true, false]);
    }
    bits.extend([false, true, false, true, false, false, true]);
    let mut bytes = vec![0u8; bits.len().div_ceil(8)];
    for (index, &bit) in bits.iter().enumerate() {
        if bit {
            bytes[index / 8] |= 0x80 >> (index % 8);
        }
    }
    Party::decode(&bytes[..]).expect("the constructed quarter is canonical")
}

/// Builds the spine of `depth` levels and the one-leaf version of height
/// `2^(10·depth) − 1` that dominates it, returning them with their encoded
/// bits.
fn operands(depth: usize) -> (Version, Version, u64) {
    let flat = Shape::Hugeleaf.build1(10 * depth).version();
    let spine = spine(depth);
    assert!(spine <= flat, "the flat version dominates the spine");
    let bits = flat.encoded_bits() + spine.encoded_bits();
    (flat, spine, bits)
}

/// Measures `flat | spine`, in units of the operands' encoded bits.
pub fn join_flat_first(depth: usize) -> u64 {
    let (flat, spine, bits) = operands(depth);
    measured_per(
        bits,
        || &flat | &spine,
        |join| assert_eq!(join, flat, "the dominating flat version is the join"),
    )
}

/// Measures `spine | flat`, in units of the operands' encoded bits.
pub fn join_spine_first(depth: usize) -> u64 {
    let (flat, spine, bits) = operands(depth);
    measured_per(
        bits,
        || &spine | &flat,
        |join| assert_eq!(join, flat, "the dominating flat version is the join"),
    )
}
