//! Relative interval widths, with large scales and empty digit ranges kept sparse.
//!
//! A width is nonnegative, but signed digits make its representation compact:
//! a run of base-2^32 digits all equal to `2^32 - 1` becomes a `-1` followed
//! by a carry beyond the run. The representation can therefore retain both
//! tiny deep intervals and long contiguous intervals without filling every
//! position between their endpoints.

use core::cmp::Ordering;

use num_bigint::{BigInt, BigUint, Sign};
use suanpan::Accumulator;

use crate::accumulator::{self, BigIntAccumulator as _};

/// A nonnegative interval width `magnitude * 2^shift`, with a digit-aligned shift.
pub struct ScaledWidth {
    /// Only the accumulator's written span, excluding its unwritten low prefix.
    magnitude: BigUint,
    /// Power-of-two scale, always a multiple of 32.
    shift: u64,
}

/// Keep an interval's unwritten low prefix separate through readout and arithmetic.
impl ScaledWidth {
    /// Read a sum of interval widths without materializing its zero prefix.
    ///
    /// Do not precede this with an accumulator sign read. Reading the sign can
    /// rewrite digits and lower the lowest-written position, making this read
    /// traverse a prefix it could otherwise skip.
    pub fn read(width: &Accumulator) -> Self {
        let (sign, magnitude, shift) = width.scaled_signed_magnitude();
        debug_assert_ne!(sign, Ordering::Less, "interval widths only accumulate");
        Self { magnitude, shift }
    }

    /// Whether no regions have contributed a width.
    pub fn is_zero(&self) -> bool {
        self.magnitude == BigUint::ZERO
    }

    /// Add this relative width to another sum, retaining its scale.
    pub fn add_to(&self, total: &mut Accumulator) {
        total.add_biguint_shl(&self.magnitude, self.shift);
    }

    /// Convert the width to balanced digits before multiplying by a height.
    pub fn add_product(&self, total: &mut Accumulator, height: &BigInt) {
        SparseWidth::from_scaled(self).add_product(total, height);
    }
}

/// An interval width in ascending, nonzero, balanced base-2^32 digits.
///
/// Each `(index, digit)` represents `digit * 2^(32 * index)`, with
/// `-2^31 <= digit < 2^31`. Indices are unique and strictly increasing.
/// Addition visits only present digits and carries, never gaps between them.
pub struct SparseWidth {
    /// The balanced digits, excluding all zero positions.
    digits: Vec<(u64, i64)>,
}

/// Preserve balanced sparse digits through addition and clustered multiplication.
impl SparseWidth {
    /// Convert one scaled width, balancing adjacent digits in a single pass.
    pub fn from_scaled(width: &ScaledWidth) -> Self {
        debug_assert_eq!(width.shift % 32, 0, "interval widths are digit-aligned");
        let start_index = width.shift / 32;
        let digits = width
            .magnitude
            .iter_u64_digits()
            .enumerate()
            .flat_map(|(limb_index, limb)| {
                [
                    (
                        start_index + 2 * limb_index as u64,
                        (limb & 0xFFFF_FFFF) as i64,
                    ),
                    (start_index + 2 * limb_index as u64 + 1, (limb >> 32) as i64),
                ]
            })
            .filter(|&(_, digit)| digit != 0);
        let mut width = Self { digits: Vec::new() };
        width.combine(digits);
        width
    }

    /// Number of stored digits: the work needed to merge this width.
    pub fn digit_count(&self) -> usize {
        self.digits.len()
    }

    /// Add another width in one pass over both sparse digit sequences.
    pub fn add(&mut self, other: Self) {
        self.combine(other.digits.into_iter());
    }

    /// Merge ascending digits while restoring the balanced range at each position.
    /// Incoming digits may be balanced digits or unsigned 32-bit digits.
    fn combine(&mut self, incoming: impl Iterator<Item = (u64, i64)>) {
        let mut old = core::mem::take(&mut self.digits).into_iter().peekable();
        let mut incoming = incoming.peekable();
        let mut digits = Vec::new();
        let mut carry = 0i64;
        let mut carry_index = 0u64;
        loop {
            // Select the next occupied position across both inputs and the
            // carry. No work is needed for any skipped zero positions.
            let mut index = if carry != 0 { carry_index } else { u64::MAX };
            if let Some(&(next_index, _)) = old.peek() {
                index = index.min(next_index);
            }
            if let Some(&(next_index, _)) = incoming.peek() {
                index = index.min(next_index);
            }
            if index == u64::MAX {
                break;
            }
            let mut sum = 0i64;
            if carry != 0 {
                // Inputs advance strictly beyond the position that produced
                // this carry, so neither can precede its immediate successor.
                debug_assert_eq!(carry_index, index, "consume a carry at its own index");
                sum = carry;
                carry = 0;
            }
            if let Some((_, digit)) = old.next_if(|&(at, _)| at == index) {
                sum += digit;
            }
            if let Some((_, digit)) = incoming.next_if(|&(at, _)| at == index) {
                sum += digit;
            }

            // sum = remainder + carry_out * 2^32. Biasing by half the base
            // makes the remainder lie in [-2^31, 2^31). In particular,
            // 2^32 - 1 becomes -1 with carry 1, compressing runs of ones.
            // The sum of two input digits and a carry is below 2^33 in
            // magnitude, so all this arithmetic fits comfortably in i64.
            let carry_out = (sum + (1 << 31)) >> 32;
            let remainder = sum - (carry_out << 32);
            if remainder != 0 {
                digits.push((index, remainder));
            }
            if carry_out != 0 {
                debug_assert_eq!(carry, 0, "the previous carry was consumed");
                carry = carry_out;
                carry_index = index + 1;
            }
        }
        self.digits = digits;
    }

    /// Group adjacent digits unless their separating zero gap exceeds `gap_limit`.
    fn clusters(&self, gap_limit: u64) -> impl Iterator<Item = &[(u64, i64)]> {
        let mut rest = self.digits.as_slice();
        core::iter::from_fn(move || {
            if rest.is_empty() {
                return None;
            }
            let mut end = 1;
            while end < rest.len() && rest[end].0 - rest[end - 1].0 - 1 <= gap_limit {
                end += 1;
            }
            let (head, tail) = rest.split_at(end);
            rest = tail;
            Some(head)
        })
    }

    /// Add `height * self` using one backend product per nonempty sign in a cluster.
    ///
    /// Multiplying every digit separately would repeatedly traverse a wide
    /// height. Multiplying the entire dense width would fill arbitrarily large
    /// gaps. Instead, bridge only gaps no wider than the height: filling a
    /// smaller gap costs less than another full-height product, while a larger
    /// gap stays absent. Each dense operand starts at its cluster's first digit;
    /// its absolute position appears only in the final shifted addition.
    pub fn add_product(&self, total: &mut Accumulator, height: &BigInt) {
        let sign = height.sign();
        let height = height.magnitude();
        let gap_limit = accumulator::digit_len(height) as u64;
        for cluster in self.clusters(gap_limit) {
            if let [(index, digit)] = *cluster {
                // One digit needs only multiplication by a word, with no
                // dense temporary. Its sign combines with the height's sign.
                let mut product = height.clone();
                product *=
                    u32::try_from(digit.unsigned_abs()).expect("balanced digits fit 32 bits");
                if (sign == Sign::Minus) != digit.is_negative() {
                    total.sub_biguint_shl(&product, 32 * index);
                } else {
                    total.add_biguint_shl(&product, 32 * index);
                }
                continue;
            }
            let floor_index = cluster[0].0;
            let span = usize::try_from(cluster[cluster.len() - 1].0 - floor_index + 1)
                .expect("cluster spans are bounded by the stream's depth");

            // Separate positive and negative digits. Both resulting operands
            // are unsigned, so constructing them needs no borrow propagation
            // across a cluster. Empty sign parts skip multiplication entirely.
            let mut parts = [DensePart::new(span), DensePart::new(span)];
            #[cfg(feature = "meter")]
            DENSIFIED_DIGITS.fetch_add(2 * span as u64, core::sync::atomic::Ordering::Relaxed);
            for &(index, digit) in cluster {
                debug_assert!(
                    digit != 0 && digit.unsigned_abs() <= 1 << 31,
                    "width digits are nonzero and balanced"
                );
                let offset = usize::try_from(index - floor_index).expect("inside the cluster span");
                parts[usize::from(digit.is_negative())].insert(offset, digit.unsigned_abs() as u32);
            }
            for (part_sign, part) in [Sign::Plus, Sign::Minus].into_iter().zip(&parts) {
                if !part.is_empty {
                    let product = height * &BigUint::from_bytes_le(&part.bytes);
                    if sign == part_sign {
                        total.add_limbs_shl(product.iter_u64_digits(), 32 * floor_index);
                    } else {
                        total.sub_limbs_shl(product.iter_u64_digits(), 32 * floor_index);
                    }
                }
            }
        }
    }
}

/// One unsigned sign-part of a dense cluster, stored as little-endian bytes.
struct DensePart {
    /// Four bytes per base-2^32 digit, including interior zeros.
    bytes: Vec<u8>,
    /// No multiplication is needed until a digit has been inserted.
    is_empty: bool,
}

/// Fill only the selected sign's digits within a cluster's relative span.
impl DensePart {
    /// Allocate zero-filled bytes for a cluster's digit span.
    fn new(span: usize) -> Self {
        let bytes = vec![0; span * 4];
        Self {
            bytes,
            is_empty: true,
        }
    }

    /// Place a nonzero digit. Sparse digit indices are unique, so slots never overlap.
    fn insert(&mut self, index: usize, digit: u32) {
        let offset = index * 4;
        self.bytes[offset..offset + 4].copy_from_slice(&digit.to_le_bytes());
        self.is_empty = false;
    }
}

/// Zero-filled base-2^32 digits allocated for multi-digit multiplication clusters.
/// This captures fill work that neither accumulator touches nor peak heap reveal.
#[cfg(feature = "meter")]
static DENSIFIED_DIGITS: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);

/// The process-wide count of dense cluster digits initialized since the last reset.
#[cfg(feature = "meter")]
pub fn densified_digits() -> u64 {
    DENSIFIED_DIGITS.load(core::sync::atomic::Ordering::Relaxed)
}

/// Start a new measurement of dense cluster initialization work.
#[cfg(feature = "meter")]
pub fn reset_densified_digits() {
    DENSIFIED_DIGITS.store(0, core::sync::atomic::Ordering::Relaxed);
}

#[cfg(test)]
mod tests;
