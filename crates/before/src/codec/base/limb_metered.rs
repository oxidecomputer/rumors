use super::*;

/// Record a two-`Base` arithmetic operation's limb-scale work.
///
/// Compiles to nothing without the `limb-meter` feature, so every operation
/// below can call it unconditionally.
#[inline(always)]
pub(crate) fn meter_limbs2(a: &Base, b: &Base) {
    #[cfg(feature = "limb-meter")]
    limb_meter::record(a.limbs() + b.limbs());
    #[cfg(not(feature = "limb-meter"))]
    let _ = (a, b);
}

/// Record a `Base`-with-machine-scalar operation's limb-scale work (the
/// scalar counts as one limb).
///
/// Compiles to nothing without the `limb-meter` feature.
#[inline(always)]
pub(crate) fn meter_limbs1(a: &Base) {
    #[cfg(feature = "limb-meter")]
    limb_meter::record(a.limbs() + 1);
    #[cfg(not(feature = "limb-meter"))]
    let _ = a;
}

/// Record a single-operand `Base` operation's limb-scale work (hashing
/// walks every limb of its one operand).
///
/// Compiles to nothing without the `limb-meter` feature.
#[inline(always)]
pub(crate) fn meter_limbs_solo(a: &Base) {
    #[cfg(feature = "limb-meter")]
    limb_meter::record(a.limbs());
    #[cfg(not(feature = "limb-meter"))]
    let _ = a;
}

/// Record a widening left shift's limb-scale work: the operand's limbs
/// plus one per 64 shifted-in bits, plus one for the scalar.
///
/// The output spans `operand + rhs/64` limbs and the backend materializes
/// every one of them, so recording the operand alone would let a
/// shift-and-discard loop read near-zero while doing width-scale work.
/// The narrowing right shift stays on the operand-width recorder: its
/// output can only shrink, so the operand covers the cost. Compiles to
/// nothing without the `limb-meter` feature.
#[inline(always)]
pub(crate) fn meter_limbs_shl(a: &Base, rhs: u64) {
    #[cfg(feature = "limb-meter")]
    limb_meter::record(a.limbs() + rhs / 64 + 1);
    #[cfg(not(feature = "limb-meter"))]
    let _ = (a, rhs);
}
