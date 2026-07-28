//! The adversarial overlay: committed worst-case family generators from
//! `before::meter`, rendered as marked points on the same axes as the
//! uniform cloud.
//!
//! Uniform sampling audits the bulk; the engineered corners are
//! measure-zero to it. The atlas therefore marks the committed family
//! generators explicitly — the same generators the resource envelopes and
//! the amplification board price — so one canvas shows where the
//! adversarial frontier sits relative to the population. Families are
//! ramped by doubling their main knob and keeping the points whose packed
//! size lands inside the plotted span; the x-coordinate is the same
//! measure as the cloud's (total packed input bytes).

use before::meter::{
    concurrent_pair, dense, dense_suffix, freeze_parade, harmonic, hugeleaf, id_spine, jump_pair,
    memo_chain_id, nested_left_full_id, plateau_puncture, scattered_id, staircase, tooth_tail,
    weight_comb, wide_arming, Packed,
};
use before::Party;

use crate::ops::Operand;

/// One family's inputs at one ramp point: packed encodings in the op's
/// operand order.
pub struct FamilyInput {
    /// The generator's name (the overlay label).
    pub family: &'static str,
    /// Packed encodings, one per operand.
    pub inputs: Vec<Vec<u8>>,
}

/// A packed event shape's stored version encoding.
fn version_bytes(p: &Packed) -> Vec<u8> {
    p.version().encode()
}

/// A packed id shape's stored bytes, validated through the real decoder
/// so a generator drift fails here instead of inside the guest.
fn party_bytes(p: &Packed) -> Vec<u8> {
    Party::decode(&p.bytes[..]).expect("meter id families are canonical");
    p.bytes.clone()
}

/// Ramp `gen` over doubling knobs, keeping points whose total packed size
/// fits `max_bytes`. Generators whose smallest output already exceeds the
/// span contribute nothing (the caller's span, not the family, decides).
fn ramp(
    family: &'static str,
    max_bytes: usize,
    gen: impl Fn(usize) -> Option<Vec<Vec<u8>>>,
) -> Vec<FamilyInput> {
    let mut out = Vec::new();
    let mut t = 1usize;
    // Doubling knobs; every generator here grows at least linearly in its
    // knob, so the loop is bounded by the span.
    while t <= 1 << 20 {
        let Some(inputs) = gen(t) else {
            break;
        };
        let total: usize = inputs.iter().map(Vec::len).sum();
        if total > max_bytes {
            break;
        }
        out.push(FamilyInput { family, inputs });
        t *= 2;
    }
    out
}

/// The overlay inputs for an operand signature, within the plotted span.
///
/// Binary rows pair a family with itself (declared by the label) unless a
/// committed pair generator exists — `jump_pair`, `tooth_tail`, and
/// `concurrent_pair` are the pair-shaped families and carry both
/// operands' design in one name.
pub fn overlay_inputs(operands: &[Operand], max_bytes: usize) -> Vec<FamilyInput> {
    let mut out = Vec::new();
    match operands {
        [Operand::Version] => {
            out.extend(ramp("dense", max_bytes, |t| {
                Some(vec![version_bytes(&dense(t))])
            }));
            out.extend(ramp("harmonic", max_bytes, |t| {
                Some(vec![version_bytes(&harmonic(t))])
            }));
            out.extend(ramp("staircase", max_bytes, |t| {
                Some(vec![version_bytes(&staircase(t))])
            }));
            out.extend(ramp("hugeleaf", max_bytes, |t| {
                Some(vec![version_bytes(&hugeleaf(8 * t))])
            }));
            out.extend(ramp("wide_arming", max_bytes, |t| {
                Some(vec![version_bytes(&wide_arming(10, t))])
            }));
            out.extend(ramp("dense_suffix", max_bytes, |t| {
                Some(vec![version_bytes(&dense_suffix(t, t))])
            }));
            out.extend(ramp("weight_comb", max_bytes, |t| {
                Some(vec![version_bytes(&weight_comb(t))])
            }));
            out.extend(ramp("freeze_parade", max_bytes, |t| {
                Some(vec![version_bytes(&freeze_parade(t))])
            }));
            out.extend(ramp("plateau_puncture", max_bytes, |t| {
                Some(vec![version_bytes(&plateau_puncture(10, t))])
            }));
        }
        [Operand::Version, Operand::Version] => {
            out.extend(ramp("jump_pair", max_bytes, |t| {
                let (a, b) = jump_pair(8, t, 1);
                Some(vec![version_bytes(&a), version_bytes(&b)])
            }));
            out.extend(ramp("tooth_tail", max_bytes, |t| {
                let (a, b) = tooth_tail(t, 2 * t.max(1));
                Some(vec![version_bytes(&a), version_bytes(&b)])
            }));
            out.extend(ramp("concurrent_pair", max_bytes, |t| {
                let (a, b) = concurrent_pair(2 * t);
                Some(vec![a.encode(), b.encode()])
            }));
            out.extend(ramp("dense × self", max_bytes, |t| {
                let v = version_bytes(&dense(t));
                Some(vec![v.clone(), v])
            }));
            out.extend(ramp("hugeleaf × self", max_bytes, |t| {
                let v = version_bytes(&hugeleaf(8 * t));
                Some(vec![v.clone(), v])
            }));
        }
        [Operand::Version, Operand::Party] => {
            out.extend(ramp("dense × scattered_id", max_bytes, |t| {
                Some(vec![
                    version_bytes(&dense(t)),
                    party_bytes(&scattered_id(t)),
                ])
            }));
            out.extend(ramp("hugeleaf × id_spine", max_bytes, |t| {
                Some(vec![
                    version_bytes(&hugeleaf(8 * t)),
                    party_bytes(&id_spine(t, false)),
                ])
            }));
        }
        [Operand::Party] => {
            out.extend(ramp("scattered_id", max_bytes, |t| {
                Some(vec![party_bytes(&scattered_id(t))])
            }));
            out.extend(ramp("id_spine", max_bytes, |t| {
                Some(vec![party_bytes(&id_spine(t, false))])
            }));
            out.extend(ramp("memo_chain_id", max_bytes, |t| {
                Some(vec![party_bytes(&memo_chain_id(t))])
            }));
            out.extend(ramp("nested_left_full_id", max_bytes, |t| {
                Some(vec![party_bytes(&nested_left_full_id(t))])
            }));
        }
        [Operand::Party, Operand::Party] => {
            out.extend(ramp("scattered_id × self", max_bytes, |t| {
                let p = party_bytes(&scattered_id(t));
                Some(vec![p.clone(), p])
            }));
            out.extend(ramp("id_spine × self", max_bytes, |t| {
                let p = party_bytes(&id_spine(t, false));
                Some(vec![p.clone(), p])
            }));
        }
        other => panic!("no overlay mapping for operand signature {other:?}"),
    }
    out
}
