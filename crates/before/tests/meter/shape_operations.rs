//! Core version operations across wide-value and alternating-shape families.

use super::*;

// ─── bigroot scenarios ──────────────────────────────────────────────────────

/// Decoding bigroot stays within its envelope (one big-integer base plus the
/// parse stack).
#[test]
fn decode_bigroot_envelope() {
    let p = Shape::Bigroot.build2(BIGROOT_MAGNITUDE_BITS, BIGROOT_DEPTH);
    let wire = version_of(&p).encode();
    let v = metered(
        "decode_bigroot",
        wire.len(),
        &envelope::DECODE_BIGROOT,
        || Version::decode(&wire[..]).expect("a stored version's wire bytes decode"),
    );
    drop(v);
}

/// Comparing bigroot against the empty version stays within its envelope:
/// the difference accumulator absorbs the wide first height once, paid by
/// its own code, and every later delta is small.
#[test]
fn cmp_bigroot_envelope() {
    let p = Shape::Bigroot.build2(BIGROOT_MAGNITUDE_BITS, BIGROOT_DEPTH);
    let v = version_of(&p);
    let r = metered("cmp_bigroot", p.bytes.len(), &envelope::CMP_BIGROOT, || {
        v.partial_cmp(&Version::new())
    });
    assert_eq!(
        r,
        Some(Ordering::Greater),
        "bigroot strictly dominates the empty version"
    );
}

/// Joining bigroot with a one-tick version stays within its envelope: the
/// wide first height is absorbed once, paid by its own code, and every
/// later delta is small. The join is the emit kernel's stream, byte for
/// byte.
#[test]
fn join_bigroot_envelope() {
    let p = Shape::Bigroot.build2(BIGROOT_MAGNITUDE_BITS, BIGROOT_DEPTH);
    let v = version_of(&p);
    let one = uniform_version(1u8);
    let joined = metered(
        "join_bigroot",
        p.bytes.len(),
        &envelope::JOIN_BIGROOT,
        || &v | &one,
    );
    assert_eq!(
        joined,
        emitted(meter::version::join, &v, &one),
        "the public join must be the emit kernel's stream"
    );
}

// ─── hugeleaf scenarios ─────────────────────────────────────────────────────

/// Decoding hugeleaf stays within its envelope: one gamma code occupies almost
/// the whole input, so the row isolates wide-value decoding.
#[test]
fn decode_hugeleaf_envelope() {
    let p = Shape::Hugeleaf.build1(HUGELEAF_MAGNITUDE_BITS);
    let wire = version_of(&p).encode();
    let v = metered(
        "decode_hugeleaf",
        wire.len(),
        &envelope::DECODE_HUGELEAF,
        || Version::decode(&wire[..]).expect("a stored version's wire bytes decode"),
    );
    drop(v);
}

/// Joining hugeleaf with a one-tick version stays within its envelope.
///
/// The emit path grows by push, so the peak tracks the result's node count.
#[test]
fn join_hugeleaf_envelope() {
    let p = Shape::Hugeleaf.build1(HUGELEAF_MAGNITUDE_BITS);
    let v = version_of(&p);
    let one = uniform_version(1u8);
    let joined = metered(
        "join_hugeleaf",
        p.bytes.len(),
        &envelope::JOIN_HUGELEAF,
        || &v | &one,
    );
    drop(joined);
}

/// Joining the dense spine with a dominating flat operand stays within its
/// envelope.
///
/// The whole output collapses to one leaf through 125k absorb steps
/// around a held 125k-bit code, linear only because absorb never moves
/// the held code. The join is the flat operand, byte for byte.
#[test]
fn join_absorb_envelope() {
    let p = Shape::Dense.build1(DENSE_DEPTH);
    let q = Shape::Hugeleaf.build1(HUGELEAF_MAGNITUDE_BITS);
    let v = version_of(&p);
    let flat = version_of(&q);
    let joined = metered(
        "join_absorb",
        p.bytes.len() + q.bytes.len(),
        &envelope::JOIN_ABSORB,
        || &v | &flat,
    );
    assert_eq!(joined, flat, "a dominating flat operand is the whole join");
}

// ─── boundary comb scenarios ────────────────────────────────────────────────

/// Decoding the boundary comb stays within its envelope. Every carry-cliff
/// crossing is funded by a `2k + 1`-bit stored code.
#[test]
fn decode_cliff_envelope() {
    let p = Shape::CliffComb.build2(CLIFF_SCALE, CLIFF_SCALE);
    let wire = version_of(&p).encode();
    let v = metered("decode_cliff", wire.len(), &envelope::DECODE_CLIFF, || {
        Version::decode(&wire[..]).expect("a stored version's wire bytes decode")
    });
    drop(v);
}

/// Comparing the boundary comb against the empty version stays within its
/// envelope.
///
/// Each tooth's cliff excursion costs `Θ(k)` word operations bought by its own
/// `2k + 1`-bit stored magnitude, so the walk stays linear per input bit —
/// the property the comb exists to separate from codings that store 3-bit
/// deltas per crossing.
#[test]
fn cmp_cliff_envelope() {
    let p = Shape::CliffComb.build2(CLIFF_SCALE, CLIFF_SCALE);
    let v = version_of(&p);
    let r = metered("cmp_cliff", p.bytes.len(), &envelope::CMP_CLIFF, || {
        v.partial_cmp(&Version::new())
    });
    assert_eq!(
        r,
        Some(Ordering::Greater),
        "the comb strictly dominates the empty version"
    );
}

/// Joining the boundary comb with a one-tick version stays within its
/// envelope.
///
/// The emit path re-codes every tooth magnitude, each paid for by a
/// comparably-wide input code, and every 3-bit `±1` delta re-emits
/// across the `2^k` carry boundary at amortized O(1). The join is the
/// emit kernel's stream, byte for byte.
#[test]
fn join_cliff_envelope() {
    let p = Shape::CliffComb.build2(CLIFF_SCALE, CLIFF_SCALE);
    let v = version_of(&p);
    let one = uniform_version(1u8);
    let joined = metered("join_cliff", p.bytes.len(), &envelope::JOIN_CLIFF, || {
        &v | &one
    });
    assert_eq!(
        joined,
        emitted(meter::version::join, &v, &one),
        "the public join must be the emit kernel's stream"
    );
}

/// Meeting the boundary comb with a one-tick version stays within its
/// envelope.
///
/// The pointwise minimum clamps every tooth to the flat operand's height
/// while every comb delta still crosses the carry boundary in the
/// accumulator. The meet is the emit kernel's stream, byte for byte.
#[test]
fn meet_cliff_envelope() {
    let p = Shape::CliffComb.build2(CLIFF_SCALE, CLIFF_SCALE);
    let v = version_of(&p);
    let one = uniform_version(1u8);
    let met = metered("meet_cliff", p.bytes.len(), &envelope::MEET_CLIFF, || {
        &v & &one
    });
    assert_eq!(
        met,
        emitted(meter::version::meet, &v, &one),
        "the public meet must be the emit kernel's stream"
    );
}

// ─── wide-tooth comb scenarios ────────────────────────────────────────────
//
// The wide-tooth comb: every Version delta is a `±2^w` operand wider than
// any machine word, still oscillating across the `2^k` cliff. Accumulator
// work must stay linear per input bit at every tooth width.

/// Decoding the wide-tooth comb stays within its envelope: each wide delta is
/// paid by its own zigzag code, and the adopted buffer prices the payloads.
#[test]
fn decode_wide_tooth_envelope() {
    let p = Shape::WideToothComb.build3(CLIFF_SCALE, WIDE_TOOTH_WIDTH_BITS, CLIFF_SCALE);
    let wire = version_of(&p).encode();
    let v = metered(
        "decode_wide_tooth",
        wire.len(),
        &envelope::DECODE_WIDE_TOOTH,
        || Version::decode(&wire[..]).expect("a stored version's wire bytes decode"),
    );
    drop(v);
}

/// Comparing the wide-tooth comb against the empty version stays within
/// its envelope: each `±2^w` delta is a wide operand paid by its own
/// zigzag code.
#[test]
fn cmp_wide_tooth_envelope() {
    let p = Shape::WideToothComb.build3(CLIFF_SCALE, WIDE_TOOTH_WIDTH_BITS, CLIFF_SCALE);
    let v = version_of(&p);
    let r = metered(
        "cmp_wide_tooth",
        p.bytes.len(),
        &envelope::CMP_WIDE_TOOTH,
        || v.partial_cmp(&Version::new()),
    );
    assert_eq!(
        r,
        Some(Ordering::Greater),
        "the wide-tooth comb strictly dominates the empty version"
    );
}

/// Joining the wide-tooth comb with a one-tick version stays within its
/// envelope.
///
/// Each `±2^w` delta is a wide operand re-coded into the output, paid by
/// its own zigzag code. The join is the emit kernel's stream, byte for
/// byte.
#[test]
fn join_wide_tooth_envelope() {
    let p = Shape::WideToothComb.build3(CLIFF_SCALE, WIDE_TOOTH_WIDTH_BITS, CLIFF_SCALE);
    let v = version_of(&p);
    let one = uniform_version(1u8);
    let joined = metered(
        "join_wide_tooth",
        p.bytes.len(),
        &envelope::JOIN_WIDE_TOOTH,
        || &v | &one,
    );
    assert_eq!(
        joined,
        emitted(meter::version::join, &v, &one),
        "the public join must be the emit kernel's stream"
    );
}

/// Meeting the wide-tooth comb with a one-tick version stays within its
/// envelope.
///
/// Wide deltas are folded but never re-emitted (the flat side wins
/// everywhere), so the collapse discipline runs at spilled operand
/// widths. The meet is the emit kernel's stream, byte for byte.
#[test]
fn meet_wide_tooth_envelope() {
    let p = Shape::WideToothComb.build3(CLIFF_SCALE, WIDE_TOOTH_WIDTH_BITS, CLIFF_SCALE);
    let v = version_of(&p);
    let one = uniform_version(1u8);
    let met = metered(
        "meet_wide_tooth",
        p.bytes.len(),
        &envelope::MEET_WIDE_TOOTH,
        || &v & &one,
    );
    assert_eq!(
        met,
        emitted(meter::version::meet, &v, &one),
        "the public meet must be the emit kernel's stream"
    );
}

// ─── alternating spine scenarios ──────────────────────────────────────────

/// Decoding the alternating-binary spine stays within its envelope: the
/// direction of descent flips every level, and per-level state stays two
/// bits, not a frame.
#[test]
fn decode_alt_spine_envelope() {
    let p = Shape::AltSpine.build1(DENSE_DEPTH);
    let wire = version_of(&p).encode();
    let v = metered(
        "decode_alt_spine",
        wire.len(),
        &envelope::DECODE_ALT_SPINE,
        || Version::decode(&wire[..]).expect("a stored version's wire bytes decode"),
    );
    drop(v);
}
