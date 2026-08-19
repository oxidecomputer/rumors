//! The mutable build-side form of a packed bit stream: [`BitsBuf`], the one
//! buffer every emitter, parser, and instrument writes into, and the sealing
//! step that hands a finished stream to the frozen storage form.
//!
//! # Representation
//!
//! A [`BitsBuf`] is the stream's bytes beside a `u64` live bit length, under
//! two invariants every mutation maintains:
//!
//! - **Exact bytes**: the byte vector holds exactly the live bits' bytes
//!   (`live.div_ceil(8)` of them), never a trailing byte of shed content.
//! - **Zeroed dead bits**: the bits of the final partial byte at and past
//!   the live length are zero. Truncation masks the new final byte in
//!   `O(1)`; appends write into space the invariant already zeroed.
//!
//! Together they make the byte image a function of the bit content alone:
//! two buffers holding equal bits are byte-for-byte equal (so [`PartialEq`]
//! is one length check and one `memcmp`), and sealing a finished stream
//! ([`seal_padding`]) only appends the marker bit — the padding it completes
//! is already canonical, so the freeze seam adopts the allocation whole,
//! without a repair pass or a copy.
//!
//! # Widths
//!
//! Lengths and positions are `u64` on every target: the buffer is correct up
//! to allocatable memory, and no `usize`-denominated bit count anywhere in
//! the build path can wrap or bind on a 32-bit target. (A 32-bit `usize`
//! spelling of `bytes.len() * 8` wraps from 512 MiB of buffer — sizes a
//! 4 GiB address space allocates comfortably.) The one storable bound is the
//! frozen form's, checked at its door (`Bits::freeze`), never here.
//!
//! Byte *indexes* stay `usize`: an index into an allocated buffer fits the
//! target's address width by construction.

use super::bits::BitsView;

/// The mutable build-side form of a packed bit stream: bytes beside a `u64`
/// live bit length, dead bits zeroed.
///
/// Every emitter and builder writes into one of these (the crate's
/// packed-stream builder wraps one with the metered move set); a finished
/// stream freezes into the at-rest `Bits` at the storage seam. The module doc
/// carries the representation invariants and the width discipline.
///
/// Equality is bit-content equality, decided bytewise: the zeroed-dead-bits
/// invariant makes the byte image injective on contents.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct BitsBuf {
    /// The live bits' bytes, most-significant bit first:
    /// `live.div_ceil(8)` bytes exactly, dead bits zero (the module doc's
    /// invariants).
    bytes: Vec<u8>,
    /// The live bit length.
    ///
    /// `u64` increments from an allocatable buffer cannot wrap: every
    /// stored bit occupies real memory, so the length is bounded by the
    /// address space times eight, orders of magnitude under `u64::MAX`.
    live: u64,
}

impl BitsBuf {
    /// An empty buffer: no bits, no bytes, no allocation.
    pub(crate) fn new() -> Self {
        BitsBuf::default()
    }

    /// An empty buffer with room for `bits` bits before reallocation.
    ///
    /// The capacity is a hint: a request past the target's address space
    /// allocates nothing up front, and the buffer still grows to whatever
    /// the pushes actually demand.
    pub(crate) fn with_capacity(bits: u64) -> Self {
        BitsBuf {
            bytes: Vec::with_capacity(usize::try_from(bits.div_ceil(8)).unwrap_or(0)),
            live: 0,
        }
    }

    /// A buffer of `len` copies of `bit`.
    pub(crate) fn repeat(bit: bool, len: u64) -> Self {
        let whole = usize::try_from(len.div_ceil(8)).expect("a repeated buffer is allocatable");
        let mut this = BitsBuf {
            bytes: vec![if bit { 0xFF } else { 0x00 }; whole],
            live: len,
        };
        this.mask_tail();
        this
    }

    /// Adopt bytes a builder produced under this type's own invariants:
    /// exactly `live.div_ceil(8)` bytes, dead bits zero. Debug-asserted.
    pub(super) fn from_raw_parts(bytes: Vec<u8>, live: u64) -> Self {
        let this = BitsBuf { bytes, live };
        debug_assert_eq!(
            this.bytes.len() as u64,
            live.div_ceil(8),
            "adopted bytes hold exactly the live bits"
        );
        debug_assert!(this.tail_is_zeroed(), "adopted dead bits are zero");
        this
    }

    /// The live bit length.
    pub fn len(&self) -> u64 {
        self.live
    }

    /// Whether the buffer holds no bits at all.
    pub fn is_empty(&self) -> bool {
        self.live == 0
    }

    /// The live bits' bytes: dead bits of the final partial byte read zero
    /// (the module doc's invariant), so the image is the content's one
    /// spelling.
    pub fn as_raw_slice(&self) -> &[u8] {
        &self.bytes
    }

    /// The buffer's bytes, surrendered whole: the freeze seam's `O(1)`
    /// hand-off into the frozen storage form.
    pub(crate) fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// The bit at `pos`.
    ///
    /// # Panics
    ///
    /// `pos` must be below the live length.
    pub(crate) fn get(&self, pos: u64) -> bool {
        assert!(pos < self.live, "bit read past the buffer's live length");
        self.bytes[(pos / 8) as usize] >> (7 - pos % 8) & 1 == 1
    }

    /// Overwrite the bit at `pos`.
    ///
    /// # Panics
    ///
    /// `pos` must be below the live length.
    pub(crate) fn set(&mut self, pos: u64, bit: bool) {
        assert!(pos < self.live, "bit write past the buffer's live length");
        let mask = 0x80 >> (pos % 8);
        if bit {
            self.bytes[(pos / 8) as usize] |= mask;
        } else {
            self.bytes[(pos / 8) as usize] &= !mask;
        }
    }

    /// The live bits, oldest first.
    pub(crate) fn iter(&self) -> impl Iterator<Item = bool> + '_ {
        (0..self.live).map(|pos| self.get(pos))
    }

    /// The number of set live bits: a bytewise popcount, exact because the
    /// dead bits are zero (the module doc's invariant).
    pub(crate) fn count_ones(&self) -> u64 {
        self.bytes.iter().map(|b| u64::from(b.count_ones())).sum()
    }

    /// Append one bit.
    pub(crate) fn push(&mut self, bit: bool) {
        let within = (self.live % 8) as u32;
        if within == 0 {
            self.bytes.push(if bit { 0x80 } else { 0x00 });
        } else if bit {
            // The target bit is zero (the invariant), so setting it is one OR.
            *self.bytes.last_mut().expect("a partial byte exists") |= 0x80 >> within;
        }
        self.live += 1;
    }

    /// Pop the newest bit.
    pub(crate) fn pop(&mut self) -> Option<bool> {
        let pos = self.live.checked_sub(1)?;
        let bit = self.get(pos);
        self.truncate(pos);
        Some(bit)
    }

    /// Append `len <= 64` bits, value-packed at the low end of `value` (bits
    /// above `len` must be zero), most-significant first.
    pub(crate) fn push_bits(&mut self, value: u64, len: u32) {
        debug_assert!(len <= 64, "an append stages at most one machine word");
        debug_assert!(
            len == 64 || value >> len == 0,
            "append value has bits above its stated width"
        );
        if len == 0 {
            return;
        }
        // Reload the partial tail byte's live bits, merge in a double-word
        // register, and write back whole bytes plus the new (zero-padded)
        // partial byte.
        let within = (self.live % 8) as u32;
        let staged = if within == 0 {
            0
        } else {
            u64::from(self.bytes.pop().expect("a partial byte exists") >> (8 - within))
        };
        let total = within + len;
        let acc = (u128::from(staged) << len) | u128::from(value);
        let aligned = (acc << (128 - total)).to_be_bytes();
        let whole = (total / 8) as usize;
        self.bytes.extend_from_slice(&aligned[..whole]);
        if !total.is_multiple_of(8) {
            // The next byte carries the remaining bits at its top and zeros
            // below: the dead-bits invariant by construction.
            self.bytes.push(aligned[whole]);
        }
        self.live += u64::from(len);
    }

    /// Append another buffer's bits, oldest first:
    /// [`extend_from_view`] over the other buffer's whole view.
    pub(crate) fn extend_from_buf(&mut self, other: &BitsBuf) {
        extend_from_view(self, built_view(other), 0, other.len());
    }

    /// Roll the buffer back to `len` bits, discarding everything after:
    /// the byte vector sheds the freed bytes and the new final partial
    /// byte's dead bits are zeroed, both `O(1)` past the deallocation.
    ///
    /// # Panics
    ///
    /// Panics if `len` exceeds the current length: truncation only ever
    /// shortens.
    pub(crate) fn truncate(&mut self, len: u64) {
        assert!(
            len <= self.live,
            "buffer truncation target {len} exceeds the {} bits held",
            self.live,
        );
        self.bytes.truncate(len.div_ceil(8) as usize);
        self.live = len;
        self.mask_tail();
    }

    /// Re-establish the zeroed-dead-bits invariant on the final partial
    /// byte, after a truncation exposed formerly live bits as dead.
    fn mask_tail(&mut self) {
        let within = (self.live % 8) as u32;
        if within != 0 {
            *self.bytes.last_mut().expect("a partial byte exists") &= 0xFF << (8 - within);
        }
    }

    /// Whether the final partial byte's dead bits are zero: the invariant,
    /// as a probe for the debug asserts.
    fn tail_is_zeroed(&self) -> bool {
        let within = (self.live % 8) as u32;
        within == 0 || self.bytes.last().is_some_and(|b| b & (0xFF >> within) == 0)
    }

    /// Append whole bytes: a `memcpy` when the live length is
    /// byte-aligned, a two-shift merge per byte otherwise.
    fn extend_bytes(&mut self, body: &[u8]) {
        let within = (self.live % 8) as u32;
        if within == 0 {
            self.bytes.extend_from_slice(body);
        } else {
            self.bytes.reserve(body.len());
            for &b in body {
                *self.bytes.last_mut().expect("a partial byte exists") |= b >> within;
                // The shift zero-fills below the carried bits: the dead-bits
                // invariant by construction.
                self.bytes.push(b << (8 - within));
            }
        }
        self.live += body.len() as u64 * 8;
    }
}

/// Renders the live bits most-significant-first as `0`/`1`, the test
/// suites' failure-message spelling.
impl core::fmt::Debug for BitsBuf {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("BitsBuf[")?;
        for pos in 0..self.live {
            f.write_str(if self.get(pos) { "1" } else { "0" })?;
        }
        f.write_str("]")
    }
}

/// Collect bits into a buffer, oldest first: the test generators'
/// construction form.
impl FromIterator<bool> for BitsBuf {
    fn from_iter<I: IntoIterator<Item = bool>>(iter: I) -> Self {
        let mut out = BitsBuf::new();
        out.extend(iter);
        out
    }
}

/// Append bits oldest-first: [`FromIterator`]'s in-place form.
impl Extend<bool> for BitsBuf {
    fn extend<I: IntoIterator<Item = bool>>(&mut self, iter: I) {
        for bit in iter {
            self.push(bit);
        }
    }
}

/// A [`BitsBuf`] literal for the test suites: `bits_buf![1, 0, 1]` builds
/// from listed bits, `bits_buf![1; 8]` repeats one.
#[cfg(test)]
macro_rules! bits_buf {
    ($bit:literal; $n:expr) => {
        $crate::codec::BitsBuf::repeat($bit != 0, $n)
    };
    ($($bit:literal),+ $(,)?) => {
        [$($bit != 0),+]
            .into_iter()
            .collect::<$crate::codec::BitsBuf>()
    };
}
#[cfg(test)]
pub(crate) use bits_buf;

/// A build buffer's contents as a [`BitsView`]: the underlying bytes beside
/// the live bit length.
///
/// A [`BitsBuf`]'s bits always start on byte 0 of its storage and its raw
/// bytes travel with it, so the view is a plain destructuring; the final
/// partial byte's dead bits read zero (the buffer's invariant) and sit
/// behind the view exactly as a frozen stream's padding does.
pub(crate) fn built_view(bits: &BitsBuf) -> BitsView<'_> {
    BitsView::new(bits.as_raw_slice(), bits.len())
}

/// Extend a build buffer with the bit range `start..end` copied verbatim
/// from a stored stream's view.
///
/// The build-side copy seam of the operations that assemble their output
/// from input subtrees (the party split/sum families). Byte-parallel past
/// the source's alignment: at most seven leading bits go one at a time,
/// then whole source bytes land as a `memcpy` (aligned output) or a
/// two-shift merge per byte, then the trailing partial byte.
///
/// # Panics
///
/// `start..end` must be a range within the view's live length.
pub(crate) fn extend_from_view(out: &mut BitsBuf, src: BitsView<'_>, start: u64, end: u64) {
    assert!(
        start <= end && end <= src.len(),
        "copied range within the view's live length"
    );
    let mut pos = start;
    // Head bits to the source's byte boundary (at most 7).
    while pos < end && !pos.is_multiple_of(8) {
        out.push(src.bit(pos));
        pos += 1;
    }
    // Whole source bytes.
    let whole = ((end - pos) / 8) as usize;
    if whole > 0 {
        let at = (pos / 8) as usize;
        out.extend_bytes(&src.bytes()[at..at + whole]);
        pos += whole as u64 * 8;
    }
    // Tail bits (fewer than 8).
    while pos < end {
        out.push(src.bit(pos));
        pos += 1;
    }
}

/// Seal a built stream's canonical padding: one `1` marker bit appended
/// after the live bits.
///
/// Sealing makes the buffer's bytes ([`BitsBuf::as_raw_slice`]) the canonical
/// wire spelling — injective, byte-equal if and only if the bit content is
/// equal. The dead bits after the marker are already zero (the buffer's own
/// invariant, held through every truncation), so the marker completes the
/// canonical `1 0*` padding without a zeroing pass; the debug assert holds
/// the invariant at the seam. The empty stream seals to itself: no marker,
/// no bytes. `Bits::freeze` applies this at the storage seam; the standalone
/// form seals buffers that stay build-side — all of them meter/test
/// instruments producing decodable bytes.
pub(crate) fn seal_padding(bits: &mut BitsBuf) {
    if !bits.is_empty() {
        bits.push(true);
    }
    debug_assert!(
        bits.tail_is_zeroed(),
        "a sealed stream's dead bits are zero: the build buffer's invariant"
    );
}
