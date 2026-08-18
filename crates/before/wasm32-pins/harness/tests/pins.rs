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

use wasm32_pins_harness::{call0, call1, Outcome};

/// Liveness: a small valid synthesized version decodes on wasm32 with the
/// exact expected bit length, its stored bytes round-trip the input, and
/// truncated or marker-stripped mutations reject with typed errors, never a
/// panic. Green at every commit — a red here impeaches the leg itself, not
/// a boundary.
#[test]
fn version_small_roundtrips_and_rejects_typed() {
    assert_eq!(call0("pin_version_small"), Outcome::Value(0));
}

/// The largest input whose whole-buffer bit view still fits `bitvec`'s
/// borrowed-view length cap on wasm32 (`usize::MAX >> 3` = 2^29 - 1 bits,
/// so 67108863 bytes) decodes correctly, returning its exact live bit
/// length: the boundary's lower adjacency witness, so a failure at the
/// sizes just above is attributable to the cap, never to general
/// large-input handling.
#[test]
fn version_decode_below_view_cap() {
    assert_eq!(
        call1("pin_version_decode", 67_108_863),
        Outcome::Value(8 * 67_108_863 - 8),
    );
}

/// A valid 64 MiB (67108864-byte) version encoding decodes correctly on
/// wasm32, returning its exact live bit length. The size is exactly 2^29
/// bits: a whole-buffer borrowed bit view of it is unconstructible —
/// `bitvec`'s span encoding silently produces an EMPTY view here, whose
/// walk would reject a valid input as `Truncated` with no alarm — so this
/// pin holds the doors to their byte-backed walk, on which no such view
/// exists.
#[test]
fn version_decode_at_view_cap() {
    assert_eq!(
        call1("pin_version_decode", 67_108_864),
        Outcome::Value(8 * 67_108_864 - 8),
    );
}

/// A valid 67108865-byte version encoding — one byte past the borrowed
/// view's silent-empty boundary, the first size whose view construction
/// `bitvec`'s element-count guard refuses by panic — decodes correctly on
/// wasm32, returning its exact live bit length. Together with the pin one
/// byte below, this holds the doors clear of both failure genres the
/// borrowed-view cap produces (the silent empty view, then the panic).
#[test]
fn version_decode_past_view_cap() {
    assert_eq!(
        call1("pin_version_decode", 67_108_865),
        Outcome::Value(8 * 67_108_865 - 8),
    );
}

/// A valid 512 MiB (2^29-byte) version encoding — the largest stored
/// stream a 32-bit `usize` can denominate bit positions for — decodes
/// correctly on wasm32, returning its exact live bit length (2^32 - 8,
/// within one marker byte of `usize::MAX`). The size is where a `usize`
/// spelling of the stored length arithmetic (`bytes.len() * 8`) wraps a
/// 32-bit target — a wrap that is coincidentally correct at exactly 2^29
/// bytes and short by 2^32 for anything larger — so this pin holds
/// `Bits::len`'s wider arithmetic exact at the boundary, under a build
/// whose overflow checks would surface any wrap as a trap.
#[test]
fn version_decode_at_bit_length_boundary() {
    assert_eq!(
        call1("pin_version_decode", 536_870_912),
        Outcome::Value(8 * 536_870_912 - 8),
    );
}

/// A valid rank whose fraction is 2^32 - 32 expansion bits deep — the
/// deepest flush-group exponent whose whole fraction image fits the
/// big-integer backend's 32-bit buffer capacity without any stripping —
/// decodes correctly, and the decoded value sits strictly between zero
/// and one: the backend-capacity boundary's lower adjacency witness.
/// ~604 MB of input: the smallest honest trigger, since the fraction's
/// depth is deliberately counted from bits actually read, never from a
/// header's claim.
#[test]
fn rank_decode_below_backend_capacity() {
    assert_eq!(
        call1("pin_rank_decode", (1u64 << 32) - 32),
        Outcome::Value(0),
    );
}

/// A valid rank whose fraction is 2^32 - 8 expansion bits deep decodes
/// correctly on wasm32 and orders exactly against reference ranks. At
/// this size an unstripped fraction image overruns the big-integer
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

/// A valid rank whose fraction is exactly 2^32 expansion bits deep — the
/// exponent one past wasm32's `usize`, from an input (~604 MB) whose
/// decoded numerator (~512 MiB) fits the 4 GiB address space — decodes
/// correctly and orders exactly against reference ranks. An exponent this
/// size does not fit any backend shift amount on a 32-bit target, so this
/// pin holds the decode path to its byte-assembled numerator, on which no
/// value-width shift exists at all.
#[test]
fn rank_decode_at_usize_exp_boundary() {
    assert_eq!(call1("pin_rank_decode", 1u64 << 32), Outcome::Value(0));
}
