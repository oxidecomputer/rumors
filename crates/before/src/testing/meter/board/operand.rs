//! Deterministic operand-content walks: the quantities the denominators are
//! stated in, derived from the inputs entirely outside any measurement.

use num_bigint::BigUint;

use crate::version::io::tree::{VersionNode, VersionTreeReader};
use crate::Version;

/// A version's value content in bytes: the summed bit widths of its absolute
/// leaf heights (one bit minimum per leaf), rounded to bytes.
///
/// This is the content that delta coding lets ride behind asymptotically fewer
/// wire bits, and the scaling denominator of the flat-denominator shape's
/// exponent fits: the boundary comb at fixed tooth magnitude doubles its value
/// content (and every operation's required per-tooth work) per level while its
/// size grows only by the unit delta codes over a fixed wide intercept.
/// Iterative over the encoded form, outside any measurement.
pub(super) fn value_content_bytes(v: &Version) -> usize {
    let mut reader = VersionTreeReader::new(v);
    let mut pending = 1usize;
    let mut last: Option<BigUint> = None;
    let mut content = 0u64;
    while pending > 0 {
        pending -= 1;
        match reader.node() {
            VersionNode::Branch => {
                pending += 2;
                continue;
            }
            VersionNode::Leaf => {}
        }
        let code = reader.payload();
        let value = match last {
            None => code,
            Some(prev) => {
                let odd = code.bit(0);
                let magnitude = if odd {
                    (code + 1u32) >> 1u32
                } else {
                    code >> 1u32
                };
                if odd {
                    prev - &magnitude
                } else {
                    prev + &magnitude
                }
            }
        };
        content += value.bits().max(1);
        last = Some(value);
    }
    (content.div_ceil(8)) as usize
}

/// The stored byte size of a version produced by a measured body.
pub(super) fn version_output_bytes(v: &Version) -> usize {
    v.as_bytes().len()
}
