//! The 32-bit boundary pins: each test drives one guest export at one
//! boundary size and pins its behavior on wasm32.
//!
//! Red-first discipline: a boundary found misbehaving is first pinned AS
//! FOUND — the assertion names the trap and the doc comment names the wrong
//! behavior it stands for — and the commit that engineers the seam around
//! flips the same test to the correct-value assertion. A pinned trap is
//! therefore never an accepted behavior: it is the committed bad baseline
//! the cure must move.

use wasm32_pins_harness::{call0, call1, Outcome};
use wasmtime::Trap;

/// A guest panic surfaces as the `unreachable` trap (`panic = abort` on
/// wasm32-unknown-unknown).
const PANICKED: Outcome = Outcome::Trapped(Trap::UnreachableCodeReached);

/// Liveness: a small valid synthesized version decodes on wasm32 with the
/// exact expected bit length, its stored bytes round-trip the input, and
/// truncated or marker-stripped mutations reject with typed errors, never a
/// panic. Green at every commit — a red here impeaches the leg itself, not
/// a boundary.
#[test]
fn version_small_roundtrips_and_rejects_typed() {
    assert_eq!(call0("pin_version_small"), Outcome::Value(0));
}

/// Adjacency, green side: the largest input whose whole-buffer bit view
/// still fits `bitvec`'s borrowed-view length cap on wasm32
/// (`usize::MAX >> 3` = 2^29 - 1 bits, so 67108863 bytes) decodes
/// correctly, returning its exact live bit length. This witnesses that the
/// red pin one byte up is the cap itself, not general large-input
/// breakage.
#[test]
fn version_decode_below_view_cap() {
    assert_eq!(
        call1("pin_version_decode", 67_108_863),
        Outcome::Value(8 * 67_108_863 - 8),
    );
}

/// BAD BASELINE, pinned red: a valid 64 MiB (67108864-byte) version
/// encoding is SILENTLY rejected as a typed decode error on wasm32 — the
/// worst failure class, a wrong result with no alarm. The decode door
/// views the whole buffer as one bit slice; at exactly 2^29 bits,
/// `bitvec`'s element-count guard still admits the slice, and the span
/// encoding's `len << 3` then shifts the length's one set bit clean out
/// of a 32-bit `usize`, constructing an EMPTY view. The validator walks
/// zero bits and reports `Truncated` — a typed rejection, reserved for
/// invalid inputs, delivered for a valid one. The guest observes it as
/// its `-1` (decode returned `Err`) code. The door engineering flips
/// this to `Value(8 * 67_108_864 - 8)`.
#[test]
fn version_decode_at_view_cap() {
    assert_eq!(call1("pin_version_decode", 67_108_864), Outcome::Value(-1));
}

/// BAD BASELINE, pinned red: one byte past the silent-empty boundary
/// (67108865 bytes, 2^29 + 8 bits), `bitvec`'s element-count guard fires
/// and the decode door panics — a valid encoding that fits the target's
/// memory, refused by panic. Together with the silent-empty pin one byte
/// below, this witnesses both failure genres the borrowed-view cap
/// produces on wasm32. The door engineering flips this to
/// `Value(8 * 67_108_865 - 8)`.
#[test]
fn version_decode_past_view_cap() {
    assert_eq!(call1("pin_version_decode", 67_108_865), PANICKED);
}

/// BAD BASELINE, pinned red: a valid 512 MiB (2^29-byte) version encoding
/// panics on wasm32. Two seams stack here: today the trap fires in
/// `bitvec`'s view cap (the 64 MiB seam masks everything above it), and
/// once that is engineered around, `Bits::len`'s `bytes.len() * 8` — a
/// `usize` multiply — overflows at exactly this size (2^32 bits). This
/// build runs with overflow checks, so the wrap is this same trap; in an
/// unchecked build it wraps silently, coincidentally correct at exactly
/// 2^29 bytes and wrong (a slice length 2^32 short) for anything larger.
/// The len fix flips this to `Value(8 * 536_870_912 - 8)` — the largest
/// stored stream a 32-bit `usize` can denominate bit positions for.
#[test]
fn version_decode_at_bit_length_boundary() {
    assert_eq!(call1("pin_version_decode", 536_870_912), PANICKED);
}

/// Adjacency, green side: a valid rank whose fraction is 2^32 - 32
/// expansion bits deep — the deepest flush-group exponent whose fraction
/// bytes still fit the big-integer backend's 32-bit buffer capacity —
/// decodes correctly today, and the decoded value sits strictly between
/// zero and one. ~604 MB of input: the smallest honest trigger, since the
/// fraction's depth is deliberately counted from bits actually read,
/// never from a header's claim.
#[test]
fn rank_decode_below_backend_capacity() {
    assert_eq!(
        call1("pin_rank_decode", (1u64 << 32) - 32),
        Outcome::Value(0),
    );
}

/// BAD BASELINE, pinned red: a valid rank whose fraction is 2^32 - 8
/// expansion bits deep panics on wasm32 INSIDE the big-integer backend —
/// `dashu`'s `from_be_bytes` sizes its buffer from the input's byte count
/// (leading zero bytes included), and 536870911 fraction bytes need one
/// word more than the backend's 32-bit `MAX_CAPACITY`, even though the
/// numerator VALUE (which starts with 64 zero expansion bits) fits the
/// backend comfortably. This seam sits below the decoder's own
/// `usize::try_from` shift seam and partially masks it. The byte-assembly
/// fix strips leading zero bytes before materializing, flipping this to
/// `Value(0)`.
#[test]
fn rank_decode_at_backend_byte_capacity() {
    assert_eq!(call1("pin_rank_decode", (1u64 << 32) - 8), PANICKED);
}

/// BAD BASELINE, pinned red: a valid rank whose fraction is exactly 2^32
/// expansion bits deep panics on wasm32 at `Base`'s `Shl<u64>` — the
/// decoder rebuilds the numerator as `integral << exp`, and the shift
/// amount's `usize::try_from` fails for `exp >= 2^32` on a 32-bit target
/// even though the input (~604 MB) and the decoded numerator (~512 MiB)
/// both fit the 4 GiB address space. Under the ruled contract this valid
/// encoding must decode; the byte-assembly fix (concatenate the
/// integral's bytes with the fraction groups instead of shifting, leading
/// zeros stripped) flips this to `Value(0)`.
#[test]
fn rank_decode_at_usize_exp_boundary() {
    assert_eq!(call1("pin_rank_decode", 1u64 << 32), PANICKED);
}
