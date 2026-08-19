//! The 32-bit boundary pins: each test drives one guest export at one
//! boundary size and pins its exact behavior on wasm32.
//!
//! Red-first discipline: a boundary found misbehaving is pinned AS FOUND —
//! the assertion names the trap or wrong value, and the doc comment names
//! the wrong behavior it stands for — and the commit that engineers the
//! seam around flips the same test to the correct-value assertion. A pinned
//! trap is therefore never an accepted behavior: it is a committed bad
//! baseline its cure must move. Each pin's own history of red and green
//! lives in this file's git log.

use wasm32_pins_harness::{call0, call1, call2, Outcome, Trap};

/// The largest buffer byte count whose whole-buffer bit count stays below
/// 2^29 bits — `usize::MAX >> 3`, the cap a 32-bit bit-vector length
/// encoding imposes — so 67108863 bytes.
///
/// This is the coordinate of the boundary class this suite exists to catch:
/// a `usize`-denominated bit count binding or wrapping on a 32-bit target.
/// Every surface is exact across it — the walks and doors read stored
/// streams through the crate-owned view, and the emitters build into the
/// crate-owned buffer, both `u64`-denominated — and the pins straddle it
/// (below, at, and past) on every surface class to hold that exactness
/// pinned.
const BUILD_CAP_BYTES: u64 = 67_108_863;

/// Liveness: a small valid version decodes on wasm32 with the exact bit
/// length, round-trips, and rejects mutilations with typed errors.
///
/// Green at every commit — a red here impeaches the leg itself, not a
/// boundary.
#[test]
fn version_small_roundtrips_and_rejects_typed() {
    assert_eq!(call0("pin_version_small"), Outcome::Value(0));
}

/// The largest input whose whole-buffer bit count stays below the 2^29-bit
/// straddle coordinate ([`BUILD_CAP_BYTES`]) decodes with its exact bit
/// length.
///
/// The boundary's lower adjacency witness: a failure at the sizes just
/// above is attributable to the boundary, never to general large-input
/// handling.
#[test]
fn version_decode_below_build_cap() {
    assert_eq!(
        call1("pin_version_decode", 67_108_863),
        Outcome::Value(8 * 67_108_863 - 8),
    );
}

/// A valid 64 MiB (67108864-byte) version encoding decodes correctly on
/// wasm32, returning its exact live bit length.
///
/// The size is exactly 2^29 bits: the first size past the straddle
/// coordinate, so this pin holds the doors to a walk on which no 32-bit
/// length encoding binds.
#[test]
fn version_decode_at_build_cap() {
    assert_eq!(
        call1("pin_version_decode", 67_108_864),
        Outcome::Value(8 * 67_108_864 - 8),
    );
}

/// A valid 67108865-byte version encoding decodes correctly on wasm32,
/// returning its exact live bit length.
///
/// One byte past the straddle coordinate's silent-wrap size; together
/// with the pin one byte below, this holds the doors clear of both failure
/// genres a 2^29-bit length encoding would produce (a silently empty view,
/// then an element-count guard panic).
#[test]
fn version_decode_past_build_cap() {
    assert_eq!(
        call1("pin_version_decode", 67_108_865),
        Outcome::Value(8 * 67_108_865 - 8),
    );
}

/// A valid 512 MiB (2^29-byte) version encoding — the largest stored
/// stream a 32-bit `usize` can denominate bit positions for — decodes
/// correctly on wasm32 with its exact live bit length.
///
/// The returned length is 2^32 - 8, within one marker byte of
/// `usize::MAX`. The size is where a `usize` spelling of the stored
/// length arithmetic (`bytes.len() * 8`) wraps a 32-bit target — a wrap
/// coincidentally correct at exactly 2^29 bytes and short by 2^32 for
/// anything larger — so this pin holds `Bits::len`'s wider arithmetic
/// exact at the boundary, under a build whose overflow checks would
/// surface any wrap as a trap.
#[test]
fn version_decode_at_bit_length_boundary() {
    assert_eq!(
        call1("pin_version_decode", 536_870_912),
        Outcome::Value(8 * 536_870_912 - 8),
    );
}

/// A valid rank whose fraction is 2^32 - 32 expansion bits deep decodes
/// correctly, and the decoded value sits strictly between zero and one.
///
/// The deepest flush-group exponent whose whole fraction image fits the
/// big-integer backend's 32-bit buffer capacity without any stripping:
/// the backend-capacity boundary's lower adjacency witness. ~604 MB of
/// input is the smallest honest trigger, since the fraction's depth is
/// deliberately counted from bits actually read, never from a header's
/// claim.
#[test]
fn rank_decode_below_backend_capacity() {
    assert_eq!(
        call1("pin_rank_decode", (1u64 << 32) - 32),
        Outcome::Value(0),
    );
}

/// A valid rank whose fraction is 2^32 - 8 expansion bits deep decodes
/// correctly on wasm32 and orders exactly against reference ranks.
///
/// At this size an unstripped fraction image overruns the big-integer
/// backend's 32-bit buffer capacity by one word — `dashu` sizes buffers
/// from an image's byte count, leading zero bytes included — while the
/// numerator's value (its fraction opens with 64 zero bits) fits the
/// backend comfortably, so this pin holds the decoder to materializing
/// value, never zeros.
#[test]
fn rank_decode_at_backend_byte_capacity() {
    assert_eq!(
        call1("pin_rank_decode", (1u64 << 32) - 8),
        Outcome::Value(0),
    );
}

/// A valid rank whose fraction is exactly 2^32 expansion bits deep —
/// the exponent one past wasm32's `usize` — decodes correctly and orders
/// exactly against reference ranks.
///
/// The input (~604 MB) and its decoded numerator (~512 MiB) fit the
/// 4 GiB address space, while an exponent this size fits no backend
/// shift amount on a 32-bit target — so this pin holds the decode path
/// to its byte-assembled numerator, on which no value-width shift exists
/// at all.
#[test]
fn rank_decode_at_usize_exp_boundary() {
    assert_eq!(call1("pin_rank_decode", 1u64 << 32), Outcome::Value(0));
}

/// A valid rank whose numerator exactly fills the big-integer backend's
/// 32-bit capacity decodes correctly and orders exactly against reference
/// ranks.
///
/// The numerator is 2^32 - 32 value bits, from a fraction 2^32 + 32
/// expansion bits deep opening with 64 zero bits.
/// The backend caps a magnitude at `usize::MAX / 32` words so bit counts
/// fit `usize`; a numerator of exactly that many bits fills the buffer to
/// its last word. This is the backend-capacity boundary's lower adjacency
/// witness on the numerator's own width (the byte-capacity witness above
/// covers the unstripped image's width).
#[test]
fn rank_decode_at_backend_bit_capacity() {
    assert_eq!(
        call1("pin_rank_decode", (1u64 << 32) + 32),
        Outcome::Value(0),
    );
}

/// PINNED AS FOUND: a valid rank whose numerator is 2^32 - 24 value bits —
/// one flush group past the backend's 2^32 - 32-bit capacity — traps on
/// wasm32 instead of decoding or rejecting with a typed error.
///
/// The ~604 MB input and its ~512 MiB numerator both fit the 4 GiB address
/// space with the decode's whole working set beside them, so the boundary
/// is the backend's structural word cap, not memory: the backend clamps
/// the buffer's capacity to its maximum and the fill's release assert
/// panics on the word past it. Unconditional 32-bit correctness requires
/// this input to decode; the cure flips this pin to the correct-value
/// assertion beside its lower witness `rank_decode_at_backend_bit_capacity`.
#[test]
fn rank_decode_past_backend_bit_capacity_traps() {
    assert_eq!(
        call1("pin_rank_decode", (1u64 << 32) + 40),
        Outcome::Trapped(Trap::UnreachableCodeReached),
    );
}

/// A valid composite key — the rank stream, then the version whose rank it
/// is — decodes through the byte door `Ranked::decode` at the largest
/// version size below the straddle coordinate.
///
/// The door re-derives the version's rank to verify the key, and that fold
/// walks the version through the crate-owned view: the lower adjacency
/// witness of the boundary straddle the two pins above it complete.
#[test]
fn ranked_decode_below_build_cap() {
    assert_eq!(
        call1("pin_ranked_decode", BUILD_CAP_BYTES),
        Outcome::Value(0)
    );
}

/// A valid composite key whose version component is 67108864 bytes —
/// exactly 2^29 bits — decodes through the byte door `Ranked::decode` on
/// wasm32.
///
/// The door's rank re-derivation walks the version through the crate-owned
/// view, whose `u64` live length is exact at every storable size, so the
/// composite door admits every storable key: the boundary straddle's
/// middle witness, beside `ranked_decode_below_build_cap`.
#[test]
fn ranked_decode_at_build_cap() {
    assert_eq!(
        call1("pin_ranked_decode", BUILD_CAP_BYTES + 1),
        Outcome::Value(0),
    );
}

/// A valid composite key whose version component is 67108865 bytes decodes
/// through `Ranked::decode` on wasm32: the boundary straddle's upper
/// witness.
#[test]
fn ranked_decode_past_build_cap() {
    assert_eq!(
        call1("pin_ranked_decode", BUILD_CAP_BYTES + 2),
        Outcome::Value(0),
    );
}

/// A valid composite key decodes through the byte door `Ranked::decode`
/// with a 256 MiB version component, checking the decoded version's bytes
/// round-trip.
///
/// The upward exactness spot-check on the composite door's rank
/// re-derivation: the fold's numerator (~128 MiB of value bits) sits well
/// inside the big-integer backend's 32-bit capacity, so the whole key is
/// priced by its own size, hundreds of megabytes into the storable range.
#[test]
fn ranked_decode_deep_in_storable_range() {
    assert_eq!(call1("pin_ranked_decode", 268_435_456), Outcome::Value(0),);
}

/// A valid composite key decodes through the borsh door
/// `Ranked::deserialize_reader` at the largest version size below the
/// straddle coordinate, consuming exactly its own bytes.
///
/// The streaming door runs the same rank re-derivation as the byte door:
/// the boundary straddle's lower witness.
#[test]
fn ranked_borsh_below_build_cap() {
    assert_eq!(
        call1("pin_ranked_borsh", BUILD_CAP_BYTES),
        Outcome::Value(0)
    );
}

/// A valid composite key whose version component is 67108864 bytes —
/// exactly 2^29 bits — deserializes through the borsh door
/// `Ranked::deserialize_reader` on wasm32, consuming exactly its own
/// bytes.
///
/// The streaming reader parses both components byte-backed and the rank
/// re-derivation walks the version through the crate-owned view: the
/// boundary straddle's middle witness.
#[test]
fn ranked_borsh_at_build_cap() {
    assert_eq!(
        call1("pin_ranked_borsh", BUILD_CAP_BYTES + 1),
        Outcome::Value(0),
    );
}

/// A valid composite key whose version component is 67108865 bytes
/// deserializes through the borsh door on wasm32: the boundary straddle's
/// upper witness.
#[test]
fn ranked_borsh_past_build_cap() {
    assert_eq!(
        call1("pin_ranked_borsh", BUILD_CAP_BYTES + 2),
        Outcome::Value(0),
    );
}

/// A valid composite key deserializes through the borsh door with a
/// 128 MiB version component: the streaming door's upward exactness
/// spot-check, deep in the storable range.
#[test]
fn ranked_borsh_deep_in_storable_range() {
    assert_eq!(call1("pin_ranked_borsh", 134_217_728), Outcome::Value(0),);
}

/// A valid coincident span — two byte-equal version streams — decodes
/// through the borsh door `Span::deserialize_reader` at the largest `lo`
/// size below the straddle coordinate.
///
/// The door consumes exactly its own bytes.
/// It validates the second stream against `lo`'s view in one fused
/// admission walk: the boundary straddle's lower witness. (The byte door
/// `Span::decode` runs the same admission and is exact to 512 MiB per
/// component.)
#[test]
fn span_borsh_below_build_cap() {
    assert_eq!(call1("pin_span_borsh", BUILD_CAP_BYTES), Outcome::Value(0));
}

/// A valid coincident span whose `lo` component is 67108864 bytes —
/// exactly 2^29 bits — deserializes through the borsh door
/// `Span::deserialize_reader` on wasm32, consuming exactly its own bytes.
///
/// The dominance re-walk reads `lo` through the crate-owned view, exact at
/// every storable size: the boundary straddle's middle witness.
#[test]
fn span_borsh_at_build_cap() {
    assert_eq!(
        call1("pin_span_borsh", BUILD_CAP_BYTES + 1),
        Outcome::Value(0),
    );
}

/// A valid coincident span whose `lo` component is 67108865 bytes
/// deserializes through the borsh door on wasm32: the boundary straddle's
/// upper witness.
#[test]
fn span_borsh_past_build_cap() {
    assert_eq!(
        call1("pin_span_borsh", BUILD_CAP_BYTES + 2),
        Outcome::Value(0),
    );
}

/// A valid coincident span deserializes through the borsh door with
/// 128 MiB components: the admission walk's upward exactness spot-check,
/// a quarter-gigabyte composite deep in the storable range.
#[test]
fn span_borsh_deep_in_storable_range() {
    assert_eq!(call1("pin_span_borsh", 134_217_728), Outcome::Value(0),);
}

/// Causal comparison decides a valid stored pair exactly at the largest
/// operand size below the straddle coordinate: the taller lone leaf reads
/// strictly greater both ways around.
///
/// The comparison-class walk's boundary straddle, lower witness: ordering
/// reads each operand through the crate-owned view, with no decode door in
/// front.
#[test]
fn version_cmp_below_build_cap() {
    assert_eq!(call1("pin_version_cmp", BUILD_CAP_BYTES), Outcome::Value(0));
}

/// Causal comparison decides a valid stored pair exactly at 67108864
/// bytes — exactly 2^29 bits — on wasm32: the taller lone leaf reads
/// strictly greater both ways around.
///
/// The crate-owned view carries a `u64` live length, so the comparison
/// sweep is exact at every storable size: the boundary straddle's middle
/// witness, beside `version_cmp_below_build_cap`.
#[test]
fn version_cmp_at_build_cap() {
    assert_eq!(
        call1("pin_version_cmp", BUILD_CAP_BYTES + 1),
        Outcome::Value(0),
    );
}

/// Causal comparison decides a valid stored 67108865-byte pair exactly on
/// wasm32: the boundary straddle's upper witness.
#[test]
fn version_cmp_past_build_cap() {
    assert_eq!(
        call1("pin_version_cmp", BUILD_CAP_BYTES + 2),
        Outcome::Value(0),
    );
}

/// Causal comparison decides a valid stored pair exactly at 512 MiB — the
/// largest storable stream on a 32-bit target — both ways around.
///
/// The comparison class's upward exactness spot-check at the storage
/// bound itself: the operand's live length (2^32 - 8 bits) is within one
/// marker byte of `usize::MAX`, so this also exercises the view's `u64`
/// length arithmetic at the very top of the storable range.
#[test]
fn version_cmp_at_storage_bound() {
    assert_eq!(call1("pin_version_cmp", 536_870_912), Outcome::Value(0),);
}

/// Join emits a covered pair exactly at the largest operand size below the
/// straddle coordinate: the taller lone leaf joined with a short one
/// reproduces the taller, byte for byte.
///
/// The join-class walk's lower adjacency witness — and its emission
/// rebuilds the full-size output, so the pin also witnesses the build
/// buffer just below the coordinate.
#[test]
fn version_join_below_build_cap() {
    assert_eq!(
        call1("pin_version_join_covering", BUILD_CAP_BYTES),
        Outcome::Value(0),
    );
}

/// Joining a valid stored 67108864-byte version — exactly 2^29 bits — with
/// a small one it covers emits the covered result on wasm32, byte for
/// byte.
///
/// The emitting operation class's middle straddle witness, on the output
/// side: the covered join rebuilds the big operand whole, a finished
/// stream whose bit count sits exactly at the straddle coordinate when it
/// crosses the freeze seam — which the crate-owned build buffer carries at
/// `u64` width on every target.
#[test]
fn version_join_at_build_cap() {
    assert_eq!(
        call1("pin_version_join_covering", BUILD_CAP_BYTES + 1),
        Outcome::Value(0),
    );
}

/// Joining a valid stored 67108865-byte version with a small one it covers
/// emits the covered result on wasm32: the emitting class's upper straddle
/// witness, one byte past the coordinate, beside
/// `version_join_at_build_cap`.
#[test]
fn version_join_past_build_cap() {
    assert_eq!(
        call1("pin_version_join_covering", BUILD_CAP_BYTES + 2),
        Outcome::Value(0),
    );
}

/// Join emits an output of 536870903 live bits — one bit under 67108863
/// whole output bytes, the straddle coordinate — from two operands each
/// comfortably under 64 MiB.
///
/// The operands are complementary two-leaf skylines (~50 MB and ~42 MB)
/// whose join concatenates: the output outgrows both inputs, so this
/// witnesses the emitter's output side just below the coordinate,
/// independent of any operand size. The returned observation is the
/// output's exact live bit length.
#[test]
fn version_join_emit_below_build_cap() {
    assert_eq!(
        call2("pin_version_join_emit", 100_000_000, 168_435_449),
        Outcome::Value(536_870_903),
    );
}

/// A join of two valid operands, each under every per-operand bound, emits
/// on wasm32 with an output of 536870905 live bits — 67108864 finished
/// bytes, the first byte length past the straddle coordinate at the freeze
/// seam.
///
/// The operands are complementary two-leaf skylines, ~50 MB and ~42 MB,
/// whose join concatenates: the emitting class's upper output-side
/// straddle witness, beside `version_join_emit_below_build_cap` — every
/// storable join emits, whatever its output size, because the build
/// buffer and the freeze hand-off carry `u64` bit counts on every target.
#[test]
fn version_join_emit_at_build_cap() {
    assert_eq!(
        call2("pin_version_join_emit", 100_000_000, 168_435_450),
        Outcome::Value(536_870_905),
    );
}

/// Rank addition is exact just below the 32-bit alignment-gap boundary.
///
/// A fraction 2^32 expansion bits deep plus one 128 bits deep — an
/// exponent gap of 2^32 - 128, whose aligned numerator still fits the
/// backend — sums to a value strictly above both summands. The gap
/// boundary's lower adjacency witness on the addition arm.
#[test]
fn rank_add_below_gap_boundary() {
    assert_eq!(call1("pin_rank_add", 128), Outcome::Value(0));
}

/// PINNED AS FOUND: adding the integral rank 1 to a fraction 2^32
/// expansion bits deep traps on wasm32: the alignment shift's exponent
/// gap is exactly 2^32, one past the widest shift amount a 32-bit target
/// can name.
///
/// Both operands are honest decoded/derived values and the exact sum is
/// representable in memory; the boundary is the alignment shift's `usize`
/// conversion (loud by construction, documented at the shift). The cure
/// flips this pin to the correct-value assertion.
#[test]
fn rank_add_at_gap_boundary_traps() {
    assert_eq!(
        call1("pin_rank_add", 0),
        Outcome::Trapped(Trap::UnreachableCodeReached),
    );
}

/// PINNED AS FOUND: adding the rank 1/2 to a fraction 2^32 expansion bits
/// deep traps on wasm32.
///
/// The exponent gap, 2^32 - 1, fits the shift amount, but the aligned
/// numerator — 2^32 value bits — exceeds the backend's 2^32 - 32-bit
/// capacity.
/// The gap boundary's other genre: just below the shift-amount seam the
/// terminal moves from the shift's conversion to the backend's capacity
/// assert, so no sub-boundary gap is safe unless the aligned width also
/// fits. The cure flips this pin to the correct-value assertion.
#[test]
fn rank_add_below_gap_wide_result_traps() {
    assert_eq!(
        call1("pin_rank_add", 1),
        Outcome::Trapped(Trap::UnreachableCodeReached),
    );
}

/// Rank subtraction is exact just below the 32-bit alignment-gap boundary.
///
/// A fraction 128 expansion bits deep minus a smaller one 2^32 bits deep —
/// the same 2^32 - 128 gap as the addition witness — yields a difference
/// strictly between zero and the minuend. The gap boundary's lower
/// adjacency witness on the subtraction arm.
#[test]
fn rank_checked_sub_below_gap_boundary() {
    assert_eq!(call1("pin_rank_checked_sub", 128), Outcome::Value(0));
}

/// PINNED AS FOUND: subtracting a fraction 2^32 expansion bits deep from
/// the integral rank 1 traps on wasm32.
///
/// The strictly positive difference aligns the minuend by a shift whose
/// exponent gap is exactly 2^32, one past the widest amount a 32-bit
/// target can name.
/// The ordering pre-check settles sign without alignment, so `None` and
/// zero results stay exact at any gap; only the positive arm reaches the
/// shift. The cure flips this pin to the correct-value assertion.
#[test]
fn rank_checked_sub_at_gap_boundary_traps() {
    assert_eq!(
        call1("pin_rank_checked_sub", 0),
        Outcome::Trapped(Trap::UnreachableCodeReached),
    );
}
