//! The 32-bit boundary-pin guest: `before`'s public surface — the byte and
//! borsh decode doors, the semantic walks and emitters, and rank arithmetic —
//! driven at the sizes where 32-bit position arithmetic has boundaries,
//! compiled to wasm32-unknown-unknown so every pin executes under a genuinely
//! 32-bit `usize` with a full 4 GiB address space.
//!
//! The host (the `wasm32-pins-harness` crate) instantiates this module under
//! wasmtime and calls one export per pinned case. The contract:
//!
//! - Every interesting input is synthesized *inside* the guest (hundreds of
//!   megabytes at the deepest pins), so nothing bulk crosses the host
//!   boundary and the host-side test stays a one-call driver.
//! - An export returns a nonnegative observation on success — the decoded
//!   value's own bit length where one exists, `0` for a plain pass — and a
//!   negative code naming the first failed observation.
//! - A panic anywhere in `before` or its dependencies aborts the guest,
//!   which wasmtime surfaces as a trap: a first-class outcome the harness
//!   pins directly, so trap-versus-value is the red/green axis whenever a
//!   boundary misbehaves.
//!
//! The workspace builds this guest with `overflow-checks = true`: the 32-bit
//! failure class under audit includes silent release-mode wraps, and the
//! checks turn exactly those wraps into observable traps instead of wrong
//! values downstream code would have to detect after the fact.

use core::cmp::Ordering;

use before::{Rank, Ranked, Span, Version};
use borsh::BorshDeserialize;

/// The canonical encoding of a valid single-leaf `Version` padded to exactly
/// `n` bytes: one leaf flag, then the Elias-gamma code of the leaf height
/// `2^k - 1` with `k = 4n - 5`, then the marker byte.
///
/// The layout, bit by bit (positions are MSB-first over the buffer):
///
/// - bit `0`: `1`, the leaf flag ending an empty topology run;
/// - bits `1 ..= k`: the gamma code's `k`-zero prefix;
/// - bit `k + 1`: the mantissa's leading `1` (the value is `m = 2^k`);
/// - bits `k + 2 ..= 2k + 1`: the mantissa's remaining `k` zeros;
/// - bit `2k + 2 = 8n - 8`: the padding marker, alone in the final byte.
///
/// The stream is canonical (a lone leaf has no sibling to collapse with, and
/// its absolute height is a natural), so `Version::decode` must accept it at
/// any `n` the target's memory admits, and the decoded value's
/// `encoded_bits` is exactly `8n - 8`.
fn synth_version(n: usize) -> Vec<u8> {
    assert!(
        n >= 18,
        "the single-wide-leaf layout needs k = 4n - 5 >= 64"
    );
    let k = 4 * n - 5;
    let mut bytes = vec![0u8; n];
    bytes[0] |= 0x80; // the leaf flag
    bytes[(k + 1) / 8] |= 0x80 >> ((k + 1) % 8); // the mantissa's leading 1
    bytes[n - 1] |= 0x80; // the padding marker
    bytes
}

/// Sanity and liveness: a small synthesized version decodes to the expected
/// bit length, round-trips, and rejects two mutilations with typed errors.
///
/// Green at every commit; a red here means the harness or the synthesis is
/// broken, not that a boundary moved.
#[no_mangle]
pub extern "C" fn pin_version_small() -> i64 {
    let bytes = synth_version(64);
    let v = match Version::decode(&bytes[..]) {
        Ok(v) => v,
        Err(_) => return -1,
    };
    if v.encoded_bits() != 8 * 64 - 8 {
        return -2;
    }
    if v.as_bytes() != &bytes[..] {
        return -3;
    }
    // A truncated input must reject as a typed error.
    if Version::decode(&bytes[..63]).is_ok() {
        return -4;
    }
    // A zeroed final byte has no padding marker: typed reject.
    let mut unmarked = bytes;
    unmarked[63] = 0;
    if Version::decode(&unmarked[..]).is_ok() {
        return -5;
    }
    0
}

/// Decode a valid `n`-byte synthesized version and return its
/// `encoded_bits` (always `8n - 8` for the synthesized layout), checking
/// that the stored bytes round-trip the input exactly.
///
/// The harness aims this at the 32-bit boundaries: the sizes straddling
/// 2^29 bits (the 32-bit bit-vector length encoding's cap, which the
/// build side still carries) and the 512 MiB bit-length boundary of
/// `usize` position arithmetic.
#[no_mangle]
pub extern "C" fn pin_version_decode(n_bytes: u64) -> i64 {
    let n = match usize::try_from(n_bytes) {
        Ok(n) => n,
        Err(_) => return -100,
    };
    let bytes = synth_version(n);
    let v = match Version::decode(&bytes[..]) {
        Ok(v) => v,
        Err(_) => return -1,
    };
    if v.as_bytes() != &bytes[..] {
        return -2;
    }
    i64::try_from(v.encoded_bits()).unwrap_or(-3)
}

/// The canonical encoding of the valid rank `(2^(exp - 65) + 1) / 2^exp`:
/// integral part zero, a fraction exactly `exp` expansion bits deep whose
/// set bits are expansion positions `65` and `exp`.
///
/// `exp` must be a positive multiple of 8 (so the final fraction group is
/// flush and the pad is zero) and at least 128. The layout: one `0` header
/// bit (the inverted-delta code of integral `0`), then `exp / 8` fraction
/// groups of nine bits (a `1` continuation, then eight expansion bits),
/// then the closing `0` bit and zero padding to the byte boundary.
///
/// Every byte of this stream is input the decoder must actually read — the
/// fraction's depth is deliberately in-band (counted from bits read, never
/// from a header's claim), so ~`9/64` bytes per expansion bit is the
/// smallest honest trigger for any `exp`-boundary behavior: no crafted
/// stream can reach the exponent seam without materializing this length.
fn synth_rank(exp: u64) -> Vec<u8> {
    assert!(
        exp >= 128 && exp.is_multiple_of(8),
        "the layout wants flush groups"
    );
    let groups = exp / 8;
    let total_bits = 9 * groups + 2; // header + groups + close bit
    let total_bytes = usize::try_from(total_bits.div_ceil(8)).expect("the stream is addressable");

    // The group region is a 72-bit-periodic stream (eight 9-bit groups per
    // period) offset one bit by the header: emit it bytewise as the tile
    // shifted right one bit with carry, then clear everything at and past
    // the close bit and patch the two set expansion bits.
    const TILE: [u8; 9] = [0x80, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01, 0x00];
    let mut bytes = vec![0u8; total_bytes];
    for (b, byte) in bytes.iter_mut().enumerate() {
        let hi = if b == 0 { 0 } else { TILE[(b - 1) % 9] };
        *byte = (hi << 7) | (TILE[b % 9] >> 1);
    }
    // Clear the close bit and the padding after it (the tile pattern would
    // keep opening groups past the fraction's end).
    let close = 1 + 9 * groups;
    let close_byte = usize::try_from(close / 8).expect("the stream is addressable");
    let offset = (close % 8) as u32;
    bytes[close_byte] = if offset == 0 {
        0
    } else {
        bytes[close_byte] & (0xFFu8 << (8 - offset))
    };
    for byte in bytes.iter_mut().skip(close_byte + 1) {
        *byte = 0;
    }
    // Set expansion bits 65 and `exp` (1-based from the binary point):
    // expansion bit `e` lives at stream position `1 + 9·((e-1)/8) + 1 +
    // ((e-1)%8)` — its group's continuation bit, then its offset in the
    // group.
    for e in [65, exp] {
        let p = 1 + 9 * ((e - 1) / 8) + 1 + ((e - 1) % 8);
        let byte = usize::try_from(p / 8).expect("the stream is addressable");
        bytes[byte] |= 0x80 >> (p % 8);
    }
    bytes
}

/// Decode a valid synthesized rank whose fraction is exactly `exp`
/// expansion bits deep, then check exact-order observations.
///
/// The observations, against reference ranks: the value sits strictly
/// between zero and one, and equals its own clone.
///
/// The harness aims this at the `u64 -> usize` exponent seam: `exp` just
/// below `2^32` (in `usize` range on wasm32) and at `2^32` (past it). The
/// input is ~`9/64 · exp` bytes — about 604 MB at the seam — which is the
/// smallest honest trigger (`synth_rank` documents why no smaller stream
/// can reach it).
#[no_mangle]
pub extern "C" fn pin_rank_decode(exp: u64) -> i64 {
    let bytes = synth_rank(exp);
    let r = match Rank::decode(&bytes[..]) {
        Ok(r) => r,
        Err(_) => return -1,
    };
    drop(bytes);
    if r <= Rank::ZERO {
        return -2;
    }
    let one = match Version::try_from(1) {
        Ok(v) => v.rank(),
        Err(_) => return -3,
    };
    if r >= one {
        return -4;
    }
    if r != r.clone() {
        return -5;
    }
    0
}

/// Set the stream bit at `pos` (positions are MSB-first over the buffer).
fn set_bit(bytes: &mut [u8], pos: u64) {
    let byte = usize::try_from(pos / 8).expect("the stream is addressable");
    bytes[byte] |= 0x80 >> (pos % 8);
}

/// Set every stream bit in `lo ..= hi` (MSB-first positions).
///
/// The interior fills byte-at-a-time, so a run of hundreds of megabits is a
/// fill, not a per-bit loop.
fn fill_ones(bytes: &mut [u8], lo: u64, hi: u64) {
    assert!(lo <= hi, "a ones run has a nonempty range");
    let lo_byte = usize::try_from(lo / 8).expect("the stream is addressable");
    let hi_byte = usize::try_from(hi / 8).expect("the stream is addressable");
    let lo_mask = 0xFFu8 >> (lo % 8);
    let hi_mask = 0xFFu8 << (7 - (hi % 8));
    if lo_byte == hi_byte {
        bytes[lo_byte] |= lo_mask & hi_mask;
    } else {
        bytes[lo_byte] |= lo_mask;
        bytes[lo_byte + 1..hi_byte].fill(0xFF);
        bytes[hi_byte] |= hi_mask;
    }
}

/// The canonical encoding of the two-leaf version `node(leaf(2^k - 1),
/// leaf(0))`: a tall left plateau over `[0, 1/2)`, height zero over
/// `[1/2, 1)`.
///
/// The layout, bit by bit (MSB-first): the root's internal flag `0`; the left
/// leaf's flag `1` and its absolute height as gamma(`2^k - 1`) — `k` zeros,
/// then the `k + 1`-bit mantissa `2^k`; the right leaf's flag `1` and its
/// delta `-(2^k - 1)` as zigzag-gamma — the mapped value's successor is
/// `2^(k+1) - 2`, so `k` zeros, then the `k + 1`-bit mantissa `1…10`; the
/// padding marker. Live length `4k + 5` bits.
///
/// The stream is canonical (the right sibling's delta is nonzero, both
/// heights are naturals), so `Version::decode` accepts it, and joining it
/// with its right-tall dual concatenates rather than collapses.
fn synth_two_leaf_left(k: u64) -> Vec<u8> {
    assert!(k >= 2, "the layout wants a multi-bit height mantissa");
    let live = 4 * k + 5;
    let total_bytes = usize::try_from((live + 1).div_ceil(8)).expect("the stream is addressable");
    let mut bytes = vec![0u8; total_bytes];
    set_bit(&mut bytes, 1); // the left leaf flag
    set_bit(&mut bytes, k + 2); // gamma(2^k - 1)'s mantissa lead
    set_bit(&mut bytes, 2 * k + 3); // the right leaf flag
    fill_ones(&mut bytes, 3 * k + 4, 4 * k + 3); // the delta mantissa's k ones
    set_bit(&mut bytes, live); // the padding marker
    bytes
}

/// The canonical encoding of the two-leaf version `node(leaf(0),
/// leaf(2^k - 1 + 2^(j-1)))`.
///
/// Height zero over `[0, 1/2)`, a tall right plateau over `[1/2, 1)`, its
/// height chosen so the join against [`synth_two_leaf_left`]'s value has
/// the exact delta `2^(j-1)`.
///
/// The layout: the root's `0`; the left leaf's `1` and gamma(0), the single
/// bit `1`; the right leaf's `1` and its delta `+h` as zigzag-gamma — the
/// mapped value's successor is `2^j + 2^(k+1) - 1` (`j >= k + 2` keeps its
/// bit length `j + 1`), so `j` zeros, then the mantissa: a leading `1`,
/// zeros, and `k + 1` trailing ones. Live length `2j + 5` bits.
fn synth_two_leaf_right(k: u64, j: u64) -> Vec<u8> {
    assert!(
        j >= k + 2,
        "the delta mantissa must dominate the height's low bits"
    );
    let live = 2 * j + 5;
    let total_bytes = usize::try_from((live + 1).div_ceil(8)).expect("the stream is addressable");
    let mut bytes = vec![0u8; total_bytes];
    set_bit(&mut bytes, 1); // the left leaf flag
    set_bit(&mut bytes, 2); // gamma(0): the single bit `1`
    set_bit(&mut bytes, 3); // the right leaf flag
    set_bit(&mut bytes, j + 4); // the delta mantissa's leading 1
    fill_ones(&mut bytes, 2 * j + 4 - k, 2 * j + 4); // its k + 1 trailing ones
    set_bit(&mut bytes, live); // the padding marker
    bytes
}

/// The canonical rank stream of the integral `2^k - 1` (exponent zero).
///
/// Exactly `Rank::encode` of [`synth_version`]'s decoded value: the lone
/// root leaf of height `2^k - 1` spans the whole unit interval, so its rank
/// — the area under the skyline — is that height itself.
///
/// The layout (the inverted-delta integral header, no fraction): for the
/// biased mantissa `m = 2^k`, width `w = k + 1`, and run `rho = bits(w) - 1`:
/// `rho` ones, the terminating zero, the `rho` bits of `w` below its leading
/// bit, the `k` zero mantissa bits below `m`'s leading bit, then the closing
/// `0` and zero padding to the byte boundary.
fn synth_integral_rank(k: u64) -> Vec<u8> {
    let w = k + 1;
    let rho = u64::from(63 - w.leading_zeros());
    let total_bits = 2 * rho + k + 2;
    let total_bytes = usize::try_from(total_bits.div_ceil(8)).expect("the stream is addressable");
    let mut bytes = vec![0u8; total_bytes];
    fill_ones(&mut bytes, 0, rho - 1); // the header's rho ones
    for i in 0..rho {
        // w's bits below its leading bit, MSB-first.
        if w >> i & 1 == 1 {
            set_bit(&mut bytes, 2 * rho - i);
        }
    }
    bytes
}

/// The canonical composite key `Ranked::encode` of [`synth_version`]'s
/// `n`-byte value.
///
/// The rank stream of its integral rank `2^(4n - 5) - 1`, then the
/// version's canonical bytes. Returns the composite and the rank stream's
/// byte length.
fn synth_ranked(n: usize) -> (Vec<u8>, usize) {
    let k = 4 * n as u64 - 5;
    let mut composite = synth_integral_rank(k);
    let rank_len = composite.len();
    composite.extend_from_slice(&synth_version(n));
    (composite, rank_len)
}

/// Decode a valid `n`-byte-version composite key through the byte door
/// `Ranked::decode`, checking the decoded version's bytes round-trip.
///
/// The door re-derives the version's rank to verify the key's rank
/// component — a whole-stream fold over the version's stored view — so this
/// export observes the composite door's walk surface at sizes the byte
/// doors admit (up to 512 MiB per stream).
#[no_mangle]
pub extern "C" fn pin_ranked_decode(n_bytes: u64) -> i64 {
    let n = match usize::try_from(n_bytes) {
        Ok(n) => n,
        Err(_) => return -100,
    };
    let (composite, rank_len) = synth_ranked(n);
    let ranked = match Ranked::decode(&composite[..]) {
        Ok(ranked) => ranked,
        Err(_) => return -1,
    };
    if ranked.version().as_bytes() != &composite[rank_len..] {
        return -2;
    }
    0
}

/// Decode a valid `n`-byte-version composite key through the borsh door
/// `Ranked::deserialize_reader`, checking full consumption and byte
/// round-trip.
///
/// Same composite and same rank re-derivation as [`pin_ranked_decode`], via
/// the streaming reader the borsh transport uses.
#[no_mangle]
pub extern "C" fn pin_ranked_borsh(n_bytes: u64) -> i64 {
    let n = match usize::try_from(n_bytes) {
        Ok(n) => n,
        Err(_) => return -100,
    };
    let (composite, rank_len) = synth_ranked(n);
    let mut reader = &composite[..];
    let ranked = match <Ranked<'static> as BorshDeserialize>::deserialize_reader(&mut reader) {
        Ok(ranked) => ranked,
        Err(_) => return -1,
    };
    if !reader.is_empty() {
        return -2;
    }
    if ranked.version().as_bytes() != &composite[rank_len..] {
        return -3;
    }
    0
}

/// Decode a valid coincident span (two byte-equal `n`-byte version streams)
/// through the borsh door `Span::deserialize_reader`, checking full
/// consumption and that both endpoints are the parsed version.
///
/// The door validates the second stream against the first component's
/// stored view in one fused admission walk, so this export observes that
/// walk on the `lo` component; the endpoint checks are byte compares,
/// exact at any storable size.
#[no_mangle]
pub extern "C" fn pin_span_borsh(n_bytes: u64) -> i64 {
    let n = match usize::try_from(n_bytes) {
        Ok(n) => n,
        Err(_) => return -100,
    };
    let lo = synth_version(n);
    let mut composite = lo.clone();
    composite.extend_from_slice(&lo);
    let mut reader = &composite[..];
    let span = match <Span<'static> as BorshDeserialize>::deserialize_reader(&mut reader) {
        Ok(span) => span,
        Err(_) => return -1,
    };
    if !reader.is_empty() {
        return -2;
    }
    if span.lo() != span.hi() {
        return -3;
    }
    if span.lo().as_bytes() != &lo[..] {
        return -4;
    }
    0
}

/// Causally compare a valid `n`-byte version against a small one (both lone
/// leaves, so they are comparable), expecting strictly greater both ways
/// around.
///
/// The comparison sweep reads each operand through its stored view, so
/// this export observes the walk surface on ordinary comparison — a
/// decode-free operation pair over already-stored values.
#[no_mangle]
pub extern "C" fn pin_version_cmp(n_bytes: u64) -> i64 {
    let n = match usize::try_from(n_bytes) {
        Ok(n) => n,
        Err(_) => return -100,
    };
    let big = match Version::decode(&synth_version(n)[..]) {
        Ok(v) => v,
        Err(_) => return -1,
    };
    let small = match Version::decode(&synth_version(18)[..]) {
        Ok(v) => v,
        Err(_) => return -2,
    };
    if big.partial_cmp(&small) != Some(Ordering::Greater) {
        return -3;
    }
    if small.partial_cmp(&big) != Some(Ordering::Less) {
        return -4;
    }
    0
}

/// Join a valid `n`-byte version with a small one it covers (both lone
/// leaves: pointwise max is the taller), expecting the join to equal the
/// big operand byte for byte.
///
/// The join walks both operands through their stored views before its
/// merge emission, so this export observes the emitting operation class:
/// the walk surface on its input side, and the build buffer on its output
/// side (a covered join's output reproduces the big operand whole); the
/// result check is a byte compare, exact at any storable size.
#[no_mangle]
pub extern "C" fn pin_version_join_covering(n_bytes: u64) -> i64 {
    let n = match usize::try_from(n_bytes) {
        Ok(n) => n,
        Err(_) => return -100,
    };
    let big = match Version::decode(&synth_version(n)[..]) {
        Ok(v) => v,
        Err(_) => return -1,
    };
    let small = match Version::decode(&synth_version(18)[..]) {
        Ok(v) => v,
        Err(_) => return -2,
    };
    let joined = big.join(&small);
    if joined != big {
        return -3;
    }
    0
}

/// Join the two-leaf pair [`synth_two_leaf_left`]`(k)` and
/// [`synth_two_leaf_right`]`(k, j)`, returning the joined stream's exact
/// live bit length, `2k + 2j + 5`.
///
/// Each operand sits well under the walk surface's per-operand bound.
/// The join is `node(leaf(2^k - 1), leaf(2^k - 1 + 2^(j-1)))`: each half of
/// the unit interval takes its taller side, so the output concatenates the
/// left operand's tall code with a fresh `2j + 1`-bit delta code instead of
/// collapsing — the output outgrows both inputs. The harness aims `(k, j)`
/// at the emitter's output-side boundary: the build buffer's own length
/// encoding, not any per-operand bound, is what this export observes.
#[no_mangle]
pub extern "C" fn pin_version_join_emit(k: u64, j: u64) -> i64 {
    let a = match Version::decode(&synth_two_leaf_left(k)[..]) {
        Ok(v) => v,
        Err(_) => return -1,
    };
    let b = match Version::decode(&synth_two_leaf_right(k, j)[..]) {
        Ok(v) => v,
        Err(_) => return -2,
    };
    let joined = a.join(&b);
    let expected = 2 * k + 2 * j + 5;
    if joined.encoded_bits() as u64 != expected {
        return -3;
    }
    i64::try_from(expected).unwrap_or(-4)
}

/// The small second operand of the rank-arithmetic pins, keyed by its
/// exponent: `0` is the integral rank `1`, `1` is the rank `1/2`, and any
/// larger value is [`synth_rank`]'s fraction at that depth.
fn small_rank(exp: u64) -> Result<Rank, i64> {
    match exp {
        0 => match Version::try_from(1u64) {
            Ok(v) => Ok(v.rank()),
            Err(_) => Err(-101),
        },
        1 => match "(0, 1, 0)".parse::<Version>() {
            Ok(v) => Ok(v.rank()),
            Err(_) => Err(-102),
        },
        exp => Rank::decode(&synth_rank(exp)[..]).map_err(|_| -103),
    }
}

/// Add a rank whose fraction is exactly `2^32` expansion bits deep to a
/// small rank of exponent `small_exp`, checking the sum strictly exceeds
/// both summands.
///
/// Addition aligns the two numerators to the larger exponent by a left
/// shift of the exponent gap, so the harness aims `small_exp` at the gap
/// boundaries of a 32-bit target: the width the shift amount must fit, and
/// the width the shifted numerator must fit.
#[no_mangle]
pub extern "C" fn pin_rank_add(small_exp: u64) -> i64 {
    let bytes = synth_rank(1u64 << 32);
    let big = match Rank::decode(&bytes[..]) {
        Ok(r) => r,
        Err(_) => return -1,
    };
    drop(bytes);
    let small = match small_rank(small_exp) {
        Ok(r) => r,
        Err(code) => return code,
    };
    let sum = &big + &small;
    if sum <= big {
        return -2;
    }
    if sum <= small {
        return -3;
    }
    0
}

/// Subtract a rank whose fraction is exactly `2^32` expansion bits deep
/// from a strictly larger small rank of exponent `small_exp`, checking the
/// difference sits strictly between zero and the minuend.
///
/// A strictly positive difference aligns both numerators to the larger
/// exponent exactly as addition does, so the harness aims `small_exp` at
/// the same gap boundaries through the subtraction arm.
#[no_mangle]
pub extern "C" fn pin_rank_checked_sub(small_exp: u64) -> i64 {
    let bytes = synth_rank(1u64 << 32);
    let big = match Rank::decode(&bytes[..]) {
        Ok(r) => r,
        Err(_) => return -1,
    };
    drop(bytes);
    let small = match small_rank(small_exp) {
        Ok(r) => r,
        Err(code) => return code,
    };
    // Both small operands exceed `big` (about `2^-65 + 2^-(2^32)`): the
    // integral `1` outright, and a synthesized fraction by its earlier
    // second set expansion bit at equal magnitude class.
    let diff = match small.checked_sub(&big) {
        Some(diff) => diff,
        None => return -2,
    };
    if diff <= Rank::ZERO {
        return -3;
    }
    if diff >= small {
        return -4;
    }
    0
}
