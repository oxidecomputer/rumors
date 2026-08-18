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

/// The largest buffer byte count whose whole-stream borrowed bit view is
/// constructible on wasm32: `bitvec`'s view length caps at `usize::MAX >> 3`
/// = 2^29 - 1 bits, so 67108863 bytes.
///
/// The walk surface — comparisons, joins, the composite doors' rank
/// re-derivation and dominance re-walk — reads stored streams through that
/// borrowed view, so this is every walk pin's lower adjacency witness; the
/// byte doors themselves admit streams up to 512 MiB.
const VIEW_CAP_BYTES: u64 = 67_108_863;

/// Liveness: a small valid version decodes on wasm32 with the exact bit
/// length, round-trips, and rejects mutilations with typed errors.
///
/// Green at every commit — a red here impeaches the leg itself, not a
/// boundary.
#[test]
fn version_small_roundtrips_and_rejects_typed() {
    assert_eq!(call0("pin_version_small"), Outcome::Value(0));
}

/// The largest input whose whole-buffer bit view still fits `bitvec`'s
/// borrowed-view length cap on wasm32 decodes with its exact bit length.
///
/// The cap is `usize::MAX >> 3` = 2^29 - 1 bits, so 67108863 bytes: the
/// boundary's lower adjacency witness, so a failure at the sizes just
/// above is attributable to the cap, never to general large-input
/// handling.
#[test]
fn version_decode_below_view_cap() {
    assert_eq!(
        call1("pin_version_decode", 67_108_863),
        Outcome::Value(8 * 67_108_863 - 8),
    );
}

/// A valid 64 MiB (67108864-byte) version encoding decodes correctly on
/// wasm32, returning its exact live bit length.
///
/// The size is exactly 2^29 bits: a whole-buffer borrowed bit view of it
/// is unconstructible — `bitvec`'s span encoding silently produces an
/// EMPTY view here, whose walk would reject a valid input as `Truncated`
/// with no alarm — so this pin holds the doors to their byte-backed walk,
/// on which no such view exists.
#[test]
fn version_decode_at_view_cap() {
    assert_eq!(
        call1("pin_version_decode", 67_108_864),
        Outcome::Value(8 * 67_108_864 - 8),
    );
}

/// A valid 67108865-byte version encoding decodes correctly on wasm32,
/// returning its exact live bit length.
///
/// One byte past the borrowed view's silent-empty boundary, this is the
/// first size whose view construction `bitvec`'s element-count guard
/// refuses by panic. Together with the pin one byte below, this holds the
/// doors clear of both failure genres the borrowed-view cap produces (the
/// silent empty view, then the panic).
#[test]
fn version_decode_past_view_cap() {
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

/// A valid rank whose numerator is exactly the big-integer backend's
/// 32-bit capacity — 2^32 - 32 value bits, from a fraction 2^32 + 32
/// expansion bits deep opening with 64 zero bits — decodes correctly and
/// orders exactly against reference ranks.
///
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
/// version size whose whole-stream bit view is constructible on wasm32.
///
/// The door re-derives the version's rank to verify the key, and that fold
/// walks the version through its borrowed view: this is the composite
/// door's lower adjacency witness, one byte below the view cap.
#[test]
fn ranked_decode_below_view_cap() {
    assert_eq!(
        call1("pin_ranked_decode", VIEW_CAP_BYTES),
        Outcome::Value(0)
    );
}

/// PINNED AS FOUND: a valid composite key whose version component is
/// 67108864 bytes — exactly 2^29 bits, the first size past the borrowed
/// view's encoding — traps in the byte door `Ranked::decode` on wasm32
/// instead of decoding.
///
/// The rank and version components both parse (the version door's own walk
/// is byte-backed and admits streams to 512 MiB); the door then re-derives
/// the version's rank through its borrowed bit view, which is
/// unconstructible at this size, and the walk panics. Unconditional 32-bit
/// correctness requires every storable key to decode; the cure flips this
/// pin to the correct-value assertion.
#[test]
fn ranked_decode_at_view_cap_traps() {
    assert_eq!(
        call1("pin_ranked_decode", VIEW_CAP_BYTES + 1),
        Outcome::Trapped(Trap::UnreachableCodeReached),
    );
}

/// PINNED AS FOUND: a valid composite key whose version component is
/// 67108865 bytes — one byte past the view-cap boundary, where the view
/// construction fails by the element-count guard rather than the length
/// encoding — traps in `Ranked::decode` on wasm32 instead of decoding.
///
/// Together with the pin one byte below, this holds both failure genres of
/// the borrowed-view cap to the same committed baseline the cure must move.
#[test]
fn ranked_decode_past_view_cap_traps() {
    assert_eq!(
        call1("pin_ranked_decode", VIEW_CAP_BYTES + 2),
        Outcome::Trapped(Trap::UnreachableCodeReached),
    );
}

/// A valid composite key decodes through the borsh door
/// `Ranked::deserialize_reader` at the largest version size whose
/// whole-stream bit view is constructible on wasm32, consuming exactly its
/// own bytes.
///
/// The streaming door runs the same rank re-derivation as the byte door:
/// this is its lower adjacency witness.
#[test]
fn ranked_borsh_below_view_cap() {
    assert_eq!(call1("pin_ranked_borsh", VIEW_CAP_BYTES), Outcome::Value(0));
}

/// PINNED AS FOUND: a valid composite key whose version component is
/// 67108864 bytes — exactly 2^29 bits — traps in the borsh door
/// `Ranked::deserialize_reader` on wasm32 instead of deserializing.
///
/// The streaming reader parses both components byte-backed; the door's
/// rank re-derivation then walks the version through its borrowed bit
/// view, unconstructible at this size. The cure flips this pin to the
/// correct-value assertion.
#[test]
fn ranked_borsh_at_view_cap_traps() {
    assert_eq!(
        call1("pin_ranked_borsh", VIEW_CAP_BYTES + 1),
        Outcome::Trapped(Trap::UnreachableCodeReached),
    );
}

/// PINNED AS FOUND: a valid composite key whose version component is
/// 67108865 bytes — the view cap's element-count-guard genre — traps in
/// the borsh door `Ranked::deserialize_reader` on wasm32.
///
/// The second genre's committed baseline beside the pin one byte below.
#[test]
fn ranked_borsh_past_view_cap_traps() {
    assert_eq!(
        call1("pin_ranked_borsh", VIEW_CAP_BYTES + 2),
        Outcome::Trapped(Trap::UnreachableCodeReached),
    );
}

/// A valid coincident span — two byte-equal version streams — decodes
/// through the borsh door `Span::deserialize_reader` at the largest `lo`
/// size whose whole-stream bit view is constructible on wasm32, consuming
/// exactly its own bytes.
///
/// The door validates the second stream against `lo`'s borrowed view in
/// one fused admission walk: this is that walk's lower adjacency witness.
/// (The byte door `Span::decode` runs the same admission on raw bytes and
/// is exact to 512 MiB per component; only the streaming door reads
/// through the view.)
#[test]
fn span_borsh_below_view_cap() {
    assert_eq!(call1("pin_span_borsh", VIEW_CAP_BYTES), Outcome::Value(0));
}

/// PINNED AS FOUND: a valid coincident span whose `lo` component is
/// 67108864 bytes — exactly 2^29 bits — traps in the borsh door
/// `Span::deserialize_reader` on wasm32 instead of deserializing.
///
/// `lo` itself parses byte-backed; the door then opens `lo`'s borrowed bit
/// view for the dominance re-walk, unconstructible at this size. The byte
/// door decodes this same span exactly, so the boundary is the streaming
/// door's view, not the value. The cure flips this pin to the
/// correct-value assertion.
#[test]
fn span_borsh_at_view_cap_traps() {
    assert_eq!(
        call1("pin_span_borsh", VIEW_CAP_BYTES + 1),
        Outcome::Trapped(Trap::UnreachableCodeReached),
    );
}

/// PINNED AS FOUND: a valid coincident span whose `lo` component is
/// 67108865 bytes — the view cap's element-count-guard genre — traps in
/// the borsh door `Span::deserialize_reader` on wasm32.
///
/// The second genre's committed baseline beside the pin one byte below.
#[test]
fn span_borsh_past_view_cap_traps() {
    assert_eq!(
        call1("pin_span_borsh", VIEW_CAP_BYTES + 2),
        Outcome::Trapped(Trap::UnreachableCodeReached),
    );
}

/// Causal comparison decides a valid stored pair exactly at the largest
/// operand size whose whole-stream bit view is constructible on wasm32:
/// the taller lone leaf reads strictly greater both ways around.
///
/// The comparison-class walk's lower adjacency witness: ordering reads
/// each operand through its borrowed view, with no decode door in front.
#[test]
fn version_cmp_below_view_cap() {
    assert_eq!(call1("pin_version_cmp", VIEW_CAP_BYTES), Outcome::Value(0));
}

/// PINNED AS FOUND: causally comparing a valid stored 67108864-byte
/// version — exactly 2^29 bits, the first size past the borrowed view's
/// encoding — traps on wasm32 instead of ordering.
///
/// The byte doors admit and store this value exactly (its decode is pinned
/// green above), so the boundary is the walk surface's borrowed view, not
/// the value: a stored version this size cannot be compared at all.
/// Unconditional 32-bit correctness requires ordering to be exact at every
/// storable size; the cure flips this pin to the correct-value assertion.
#[test]
fn version_cmp_at_view_cap_traps() {
    assert_eq!(
        call1("pin_version_cmp", VIEW_CAP_BYTES + 1),
        Outcome::Trapped(Trap::UnreachableCodeReached),
    );
}

/// PINNED AS FOUND: causally comparing a valid stored 67108865-byte
/// version — the view cap's element-count-guard genre — traps on wasm32.
///
/// The second genre's committed baseline beside the pin one byte below.
#[test]
fn version_cmp_past_view_cap_traps() {
    assert_eq!(
        call1("pin_version_cmp", VIEW_CAP_BYTES + 2),
        Outcome::Trapped(Trap::UnreachableCodeReached),
    );
}

/// Join emits a covered pair exactly at the largest operand size whose
/// whole-stream bit view is constructible on wasm32: the taller lone leaf
/// joined with a short one reproduces the taller, byte for byte.
///
/// The join-class walk's lower adjacency witness — and its emission
/// rebuilds the full-size output, so the pin also witnesses the build
/// buffer just below its own cap.
#[test]
fn version_join_below_view_cap() {
    assert_eq!(
        call1("pin_version_join_covering", VIEW_CAP_BYTES),
        Outcome::Value(0),
    );
}

/// PINNED AS FOUND: joining a valid stored 67108864-byte version —
/// exactly 2^29 bits — with a small one traps on wasm32 instead of
/// emitting the covered result.
///
/// The join walks both operands through their borrowed views before its
/// merge emission; the big operand's view is unconstructible at this
/// size. The cure flips this pin to the correct-value assertion.
#[test]
fn version_join_at_view_cap_traps() {
    assert_eq!(
        call1("pin_version_join_covering", VIEW_CAP_BYTES + 1),
        Outcome::Trapped(Trap::UnreachableCodeReached),
    );
}

/// PINNED AS FOUND: joining a valid stored 67108865-byte version — the
/// view cap's element-count-guard genre — with a small one traps on
/// wasm32.
///
/// The second genre's committed baseline beside the pin one byte below.
#[test]
fn version_join_past_view_cap_traps() {
    assert_eq!(
        call1("pin_version_join_covering", VIEW_CAP_BYTES + 2),
        Outcome::Trapped(Trap::UnreachableCodeReached),
    );
}

/// Join emits the largest output this operand family can pass through the
/// emitter's finish seam on wasm32 — 536870903 live bits, one bit under
/// the 67108863 whole output bytes the finished stream's bit-vector
/// adoption can represent — from two operands each comfortably under the
/// walk surface's per-operand bound.
///
/// The operands are complementary two-leaf skylines (~50 MB and ~42 MB)
/// whose join concatenates: the output outgrows both inputs, so this
/// witnesses the emitter's output-side boundary from below, independent of
/// any per-operand cap. The returned observation is the output's exact
/// live bit length.
#[test]
fn version_join_emit_below_build_cap() {
    assert_eq!(
        call2("pin_version_join_emit", 100_000_000, 168_435_449),
        Outcome::Value(536_870_903),
    );
}

/// PINNED AS FOUND: a join of two valid operands, each comfortably under
/// the walk surface's per-operand bound (~50 MB and ~42 MB), traps on
/// wasm32 when its output's finished byte length reaches 67108864 —
/// 536870905 live bits, the first length whose whole-byte buffer exceeds
/// the bit vector's `usize::MAX >> 3`-bit length encoding when the
/// emitter's byte-backed builder hands the finished stream over — instead
/// of emitting.
///
/// The boundary is the build-side buffer's length encoding, documented at
/// no public operation, and it sits below the byte doors' 512 MiB storage
/// bound: valid operand pairs admitted and walkable on this target have no
/// emittable join. Unconditional 32-bit correctness requires every
/// storable join to emit; the cure flips this pin to the exact-length
/// assertion beside its witness `version_join_emit_below_build_cap`.
#[test]
fn version_join_emit_at_build_cap_traps() {
    assert_eq!(
        call2("pin_version_join_emit", 100_000_000, 168_435_450),
        Outcome::Trapped(Trap::UnreachableCodeReached),
    );
}

/// Rank addition is exact just below the 32-bit alignment-gap boundary: a
/// fraction 2^32 expansion bits deep plus one 128 bits deep — an exponent
/// gap of 2^32 - 128, whose aligned numerator still fits the backend —
/// sums to a value strictly above both summands.
///
/// The gap boundary's lower adjacency witness on the addition arm.
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
/// deep traps on wasm32: the exponent gap, 2^32 - 1, fits the shift
/// amount, but the aligned numerator — 2^32 value bits — exceeds the
/// backend's 2^32 - 32-bit capacity.
///
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

/// Rank subtraction is exact just below the 32-bit alignment-gap
/// boundary: a fraction 128 expansion bits deep minus a smaller one 2^32
/// bits deep — the same 2^32 - 128 gap as the addition witness — yields a
/// difference strictly between zero and the minuend.
///
/// The gap boundary's lower adjacency witness on the subtraction arm.
#[test]
fn rank_checked_sub_below_gap_boundary() {
    assert_eq!(call1("pin_rank_checked_sub", 128), Outcome::Value(0));
}

/// PINNED AS FOUND: subtracting a fraction 2^32 expansion bits deep from
/// the integral rank 1 traps on wasm32: the strictly positive difference
/// aligns the minuend by a shift whose exponent gap is exactly 2^32, one
/// past the widest amount a 32-bit target can name.
///
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
