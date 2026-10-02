//! Canonical streams whose interesting coordinates do not fit in 32 bits.
//!
//! Building them inside wasm is essential: otherwise the 64-bit harness would
//! perform the very indexing arithmetic under test. Every size calculation is
//! checked in `u64`, then narrowed only when allocating or indexing the guest's
//! 32-bit address space. Synthesis failures are returned in-band, so a trap can
//! be attributed to the operation being checked.

use wasm32_pins_protocol::Failure;

/// A zero-filled stream with checked bit addressing.
struct Stream(Vec<u8>);

impl Stream {
    /// Allocates exactly `bytes` zeroed bytes or reports that wasm cannot hold it.
    fn zeroed(bytes: u64) -> Result<Self, Failure> {
        let len = usize::try_from(bytes).map_err(|_| Failure::Synthesis)?;
        let mut storage = Vec::new();
        storage
            .try_reserve_exact(len)
            .map_err(|_| Failure::Synthesis)?;
        storage.resize(len, 0);
        Ok(Self(storage))
    }

    /// Sets one MSB-first bit after checking that the byte is addressable.
    fn set(&mut self, bit: u64) -> Result<(), Failure> {
        let byte = usize::try_from(bit / 8).map_err(|_| Failure::Synthesis)?;
        let Some(slot) = self.0.get_mut(byte) else {
            return Err(Failure::Synthesis);
        };
        *slot |= 0x80 >> (bit % 8);
        Ok(())
    }

    /// Sets an inclusive run, filling its interior a byte at a time.
    fn fill(&mut self, lo: u64, hi: u64) -> Result<(), Failure> {
        if lo > hi {
            return Err(Failure::InvalidArguments);
        }
        let lo_byte = usize::try_from(lo / 8).map_err(|_| Failure::Synthesis)?;
        let hi_byte = usize::try_from(hi / 8).map_err(|_| Failure::Synthesis)?;
        if hi_byte >= self.0.len() {
            return Err(Failure::Synthesis);
        }
        let lo_mask = 0xFF >> (lo % 8);
        let hi_mask = 0xFF << (7 - hi % 8);
        if lo_byte == hi_byte {
            self.0[lo_byte] |= lo_mask & hi_mask;
        } else {
            self.0[lo_byte] |= lo_mask;
            self.0[lo_byte + 1..hi_byte].fill(0xFF);
            self.0[hi_byte] |= hi_mask;
        }
        Ok(())
    }

    /// Returns the completed byte stream.
    fn finish(self) -> Vec<u8> {
        self.0
    }
}

/// Encodes one leaf of height `2^(4n-5) - 1` in exactly `n` bytes.
///
/// A leaf flag is followed by gamma's zero prefix and lone mantissa bit; the
/// padding marker lands at bit `8n - 8`. Thus the encoding is canonical and
/// makes both the stored stream and its numeric payload grow with `n`.
pub fn version(bytes: u64) -> Result<Vec<u8>, Failure> {
    if bytes < 18 {
        return Err(Failure::InvalidArguments);
    }
    let width = bytes
        .checked_mul(4)
        .and_then(|n| n.checked_sub(5))
        .ok_or(Failure::Synthesis)?;
    let marker = width
        .checked_mul(2)
        .and_then(|n| n.checked_add(2))
        .ok_or(Failure::Synthesis)?;
    let mut stream = Stream::zeroed(bytes)?;
    stream.set(0)?;
    stream.set(width + 1)?;
    stream.set(marker)?;
    Ok(stream.finish())
}

/// Encodes `(2^(exp-65) + 1) / 2^exp` in canonical `Rank` form.
///
/// Fraction groups each contribute one continuation bit and eight expansion
/// bits. Reaching exponent `exp` therefore requires `9exp/64` input bytes;
/// the pointer-width boundary cannot be faked by a short declared length.
pub fn rank(exp: u64) -> Result<Vec<u8>, Failure> {
    if exp < 128 || !exp.is_multiple_of(8) {
        return Err(Failure::InvalidArguments);
    }
    fraction(exp, &[65, exp])
}

/// Encodes the same rank family with its final bit one position earlier.
///
/// This value exceeds [`rank`] by exactly `2^-exp`, giving the exponent-seam
/// check an exact algebraic witness for the final bit.
pub fn rank_with_penultimate(exp: u64) -> Result<Vec<u8>, Failure> {
    if exp < 128 || !exp.is_multiple_of(8) {
        return Err(Failure::InvalidArguments);
    }
    fraction(exp, &[65, exp - 1])
}

/// Encodes the exact rank `2^-exp`.
pub fn unit_fraction(exp: u64) -> Result<Vec<u8>, Failure> {
    if exp == 0 || !exp.is_multiple_of(8) {
        return Err(Failure::InvalidArguments);
    }
    fraction(exp, &[exp])
}

/// Builds a fractional rank with set bits at the requested expansions.
fn fraction(exp: u64, expansions: &[u64]) -> Result<Vec<u8>, Failure> {
    let groups = exp / 8;
    let close = groups
        .checked_mul(9)
        .and_then(|n| n.checked_add(1))
        .ok_or(Failure::Synthesis)?;
    let total_bytes = close
        .checked_add(1)
        .map(|bits| bits.div_ceil(8))
        .ok_or(Failure::Synthesis)?;
    let mut stream = Stream::zeroed(total_bytes)?;

    // Eight groups occupy nine bytes. This tile sets their continuation bits;
    // shifting it by the one-bit integral header produces the repeated body.
    const TILE: [u8; 9] = [0x80, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01, 0];
    for (index, byte) in stream.0.iter_mut().enumerate() {
        let carry = if index == 0 {
            0
        } else {
            TILE[(index - 1) % TILE.len()]
        };
        *byte = (carry << 7) | (TILE[index % TILE.len()] >> 1);
    }

    // The periodic body would continue past the chosen exponent. Clear the
    // closing bit and padding, then set the two expansion bits of the value.
    let close_byte = usize::try_from(close / 8).map_err(|_| Failure::Synthesis)?;
    let offset = (close % 8) as u32;
    stream.0[close_byte] = if offset == 0 {
        0
    } else {
        stream.0[close_byte] & (0xFF << (8 - offset))
    };
    stream.0[close_byte + 1..].fill(0);
    for &expansion in expansions {
        if expansion == 0 || expansion > exp {
            return Err(Failure::InvalidArguments);
        }
        let group = (expansion - 1) / 8;
        let within_group = (expansion - 1) % 8;
        // Skip the one-bit integral header and each preceding nine-bit group,
        // then this group's continuation bit and the earlier expansion bits.
        let bit = 1 + 9 * group + 1 + within_group;
        stream.set(bit)?;
    }
    Ok(stream.finish())
}

/// Encodes `node(leaf(2^k - 1), leaf(0))` in `4k + 5` live bits.
///
/// The left height needs a `2k + 1`-bit gamma code. The right leaf then stores
/// the same magnitude as a negative delta; its zigzag-gamma mantissa is `k`
/// ones followed by zero. The unequal siblings make the tree canonical.
pub fn two_leaf_left(k: u64) -> Result<Vec<u8>, Failure> {
    if k < 2 {
        return Err(Failure::InvalidArguments);
    }
    let live = k
        .checked_mul(4)
        .and_then(|n| n.checked_add(5))
        .ok_or(Failure::Synthesis)?;
    let mut stream = Stream::zeroed((live + 1).div_ceil(8))?;
    stream.set(1)?; // left leaf, after the root's zero internal flag
    stream.set(k + 2)?; // left height's mantissa lead
    stream.set(2 * k + 3)?; // right leaf
    stream.fill(3 * k + 4, 4 * k + 3)?; // right delta's mantissa
    stream.set(live)?;
    Ok(stream.finish())
}

/// Encodes `node(leaf(0), leaf(2^k - 1 + 2^(j-1)))` in `2j + 5` live bits.
///
/// Its right height is chosen to complement [`two_leaf_left`]. Joining the
/// pair keeps both leaves and emits a right delta whose width is controlled by
/// `j`; the output size can therefore cross a boundary independently of either
/// input size.
pub fn two_leaf_right(k: u64, j: u64) -> Result<Vec<u8>, Failure> {
    if j < k + 2 {
        return Err(Failure::InvalidArguments);
    }
    let live = j
        .checked_mul(2)
        .and_then(|n| n.checked_add(5))
        .ok_or(Failure::Synthesis)?;
    let mut stream = Stream::zeroed((live + 1).div_ceil(8))?;
    stream.set(1)?; // left leaf
    stream.set(2)?; // left height zero: gamma's single one
    stream.set(3)?; // right leaf
    stream.set(j + 4)?; // right delta's mantissa lead
    stream.fill(2 * j + 4 - k, 2 * j + 4)?; // its low k + 1 ones
    stream.set(live)?;
    Ok(stream.finish())
}

/// Encodes the exact join of [`two_leaf_left`] and [`two_leaf_right`].
///
/// The left leaf retains height `2^k - 1`. The right leaf retains height
/// `2^k - 1 + 2^(j-1)`, so its change from the left is `2^(j-1)`. The positive
/// change code consists of `j` zeros, that magnitude's `j` bits, and a set
/// sign bit. Together with the topology and left payload, the stream has
/// `2k + 2j + 5` live bits.
pub fn joined(k: u64, j: u64) -> Result<Vec<u8>, Failure> {
    if k < 2 || j < k + 2 {
        return Err(Failure::InvalidArguments);
    }
    let live = k
        .checked_mul(2)
        .and_then(|n| j.checked_mul(2).and_then(|m| n.checked_add(m)))
        .and_then(|n| n.checked_add(5))
        .ok_or(Failure::Synthesis)?;
    let mut stream = Stream::zeroed((live + 1).div_ceil(8))?;
    stream.set(1)?; // left leaf, after the root's zero internal flag
    stream.set(k + 2)?; // left absolute height's mantissa lead
    stream.set(2 * k + 3)?; // right leaf
    stream.set(2 * k + j + 4)?; // right change's magnitude lead
    stream.set(live - 1)?; // positive sign bit ending the change
    stream.set(live)?; // padding marker
    Ok(stream.finish())
}
