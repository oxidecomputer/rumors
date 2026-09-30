//! Resource envelopes for core operations on the dense-spine family.

use super::*;

// ─── dense spine scenarios ──────────────────────────────────────────────────

/// Decoding the dense spine stays within its envelope: wire decode is
/// validate plus the wrap, and the payloads ride the word-valued form.
#[test]
fn decode_dense_envelope() {
    let p = Shape::Dense.build1(DENSE_DEPTH);
    let wire = version_of(&p).encode();
    let v = metered("decode_dense", wire.len(), &envelope::DECODE_DENSE, || {
        Version::decode(&wire[..]).expect("a stored version's wire bytes decode")
    });
    drop(v);
}

/// Comparing the dense spine against the empty version stays within its
/// envelope: the iterative sweep over the at-rest form, the whole deep
/// side consumed against one depth-0 plateau.
#[test]
fn cmp_dense_envelope() {
    let p = Shape::Dense.build1(DENSE_DEPTH);
    let v = version_of(&p);
    let r = metered("cmp_dense", p.bytes.len(), &envelope::CMP_DENSE, || {
        v.partial_cmp(&Version::new())
    });
    assert_eq!(
        r,
        Some(Ordering::Greater),
        "the dense spine strictly dominates the empty version"
    );
}

/// Comparing the dense spine against a byte-equal, buffer-distinct copy
/// stays within its envelope.
///
/// Every boundary is an aligned tie, both cursors advance in lockstep to
/// full depth, and the verdict is Equal only after both streams are
/// wholly consumed.
#[test]
fn cmp_dense_self_envelope() {
    let p = Shape::Dense.build1(DENSE_DEPTH);
    let v = version_of(&p);
    let w = version_of(&p);
    let r = metered(
        "cmp_dense_self",
        2 * p.bytes.len(),
        &envelope::CMP_DENSE_SELF,
        || v.partial_cmp(&w),
    );
    assert_eq!(r, Some(Ordering::Equal), "identical streams read equal");
}

/// Joining the dense spine with a one-tick version stays within its
/// envelope.
///
/// The 125k-level walk emits and collapses on path-bit stacks and one
/// accumulator, with the peak in the emitted stream itself. The join is
/// the emit kernel's stream, byte for byte.
#[test]
fn join_dense_envelope() {
    let p = Shape::Dense.build1(DENSE_DEPTH);
    let v = version_of(&p);
    let one = uniform_version(1u8);
    let joined = metered("join_dense", p.bytes.len(), &envelope::JOIN_DENSE, || {
        &v | &one
    });
    assert_eq!(
        joined,
        emitted(meter::version::join, &v, &one),
        "the public join must be the emit kernel's stream"
    );
}
