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
    meet_shade, memo_chain_id, nested_left_full_id, plateau_puncture, scattered_id,
    stagger_population, staircase, tooth_tail, weight_comb, wide_arming, Packed,
};
use before::Party;

use crate::ops::{Inputs, OpSpec, Operand};

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

/// The staggered fold population's version operands, encoded in the
/// committed bit-reversed feed order (the order is load-bearing: it is
/// what realizes the intermediate swell at every reduction level).
fn stagger_versions(n: usize, m: usize) -> Vec<Vec<u8>> {
    let (versions, _) = stagger_population(n, m);
    versions.iter().map(version_bytes).collect()
}

/// The overlay inputs for an operation, within the plotted span: keyed by
/// the row's operand signature, except the slice rows, whose committed
/// fold-cure families are per-operation ([`slice_overlays`]).
///
/// Binary rows pair a family with itself (declared by the label) unless a
/// committed pair generator exists — `jump_pair`, `tooth_tail`, and
/// `concurrent_pair` are the pair-shaped families and carry both
/// operands' design in one name. Rows that draw the same signature share
/// the same families: each overlay point rides the row's own `measure`,
/// so the committed shape is pushed through whatever preparation the row
/// declares (a fork split, a rank derivation, a render round-trip).
pub fn overlay_inputs(op: &OpSpec, max_bytes: usize) -> Vec<FamilyInput> {
    let (operands, distinct) = match op.inputs {
        Inputs::Packed(operands) => (operands, false),
        Inputs::PackedDistinct(operands) => (operands, true),
        Inputs::VersionSlice => return slice_overlays(op.name, max_bytes),
    };
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
        // The masked three-stream comparison row: the projected version
        // and its mask crossed with a compared version of the same shape.
        [Operand::Version, Operand::Party, Operand::Version] => {
            out.extend(ramp("dense × scattered_id × dense", max_bytes, |t| {
                Some(vec![
                    version_bytes(&dense(t)),
                    party_bytes(&scattered_id(t)),
                    version_bytes(&dense(t)),
                ])
            }));
            out.extend(ramp("hugeleaf × id_spine × hugeleaf", max_bytes, |t| {
                Some(vec![
                    version_bytes(&hugeleaf(8 * t)),
                    party_bytes(&id_spine(t, false)),
                    version_bytes(&hugeleaf(8 * t)),
                ])
            }));
        }
        // The masked four-stream comparison row: two full views, each an
        // event family under an id-family mask.
        [Operand::Version, Operand::Party, Operand::Version, Operand::Party] => {
            out.extend(ramp("(dense / scattered_id) × self", max_bytes, |t| {
                Some(vec![
                    version_bytes(&dense(t)),
                    party_bytes(&scattered_id(t)),
                    version_bytes(&dense(t)),
                    party_bytes(&scattered_id(t)),
                ])
            }));
            out.extend(ramp("(hugeleaf / id_spine) × self", max_bytes, |t| {
                Some(vec![
                    version_bytes(&hugeleaf(8 * t)),
                    party_bytes(&id_spine(t, false)),
                    version_bytes(&hugeleaf(8 * t)),
                    party_bytes(&id_spine(t, false)),
                ])
            }));
        }
        // The clock fold row (one party fork-split over four version
        // riders): the committed stagger population adapted to the fold's
        // version halves — arity fixed at the row's four clocks, feed
        // order preserved through the operand order — plus the composed
        // clock families' cross as the non-stagger shape.
        [Operand::Party, Operand::Version, Operand::Version, Operand::Version, Operand::Version] => {
            out.extend(ramp("scattered_id × stagger (n=4)", max_bytes, |t| {
                let mut inputs = vec![party_bytes(&scattered_id(t))];
                inputs.extend(stagger_versions(4, t));
                Some(inputs)
            }));
            out.extend(ramp("id_spine × hugeleaf⁴", max_bytes, |t| {
                let mut inputs = vec![party_bytes(&id_spine(t, false))];
                inputs.extend(std::iter::repeat_with(|| version_bytes(&hugeleaf(8 * t))).take(4));
                Some(inputs)
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
        // A distinct-pair row cannot take the self pairs (its draw
        // rejects byte-identical operands), so it gets committed crosses.
        [Operand::Party, Operand::Party] if distinct => {
            out.extend(ramp("scattered_id × id_spine", max_bytes, |t| {
                Some(vec![
                    party_bytes(&scattered_id(t)),
                    party_bytes(&id_spine(t, false)),
                ])
            }));
            out.extend(ramp(
                "memo_chain_id × nested_left_full_id",
                max_bytes,
                |t| {
                    Some(vec![
                        party_bytes(&memo_chain_id(t)),
                        party_bytes(&nested_left_full_id(t)),
                    ])
                },
            ));
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
        // The composed-clock rows: an id family for the party half, an
        // event family for the version half.
        [Operand::Party, Operand::Version] => {
            out.extend(ramp("scattered_id × dense", max_bytes, |t| {
                Some(vec![
                    party_bytes(&scattered_id(t)),
                    version_bytes(&dense(t)),
                ])
            }));
            out.extend(ramp("id_spine × hugeleaf", max_bytes, |t| {
                Some(vec![
                    party_bytes(&id_spine(t, false)),
                    version_bytes(&hugeleaf(8 * t)),
                ])
            }));
        }
        // The clock-plus-version rows (recv, and the fork-split pair
        // rows): the composed clock's families with a version rider.
        [Operand::Party, Operand::Version, Operand::Version] => {
            out.extend(ramp("scattered_id × dense × dense", max_bytes, |t| {
                Some(vec![
                    party_bytes(&scattered_id(t)),
                    version_bytes(&dense(t)),
                    version_bytes(&dense(t)),
                ])
            }));
            out.extend(ramp("id_spine × hugeleaf × hugeleaf", max_bytes, |t| {
                Some(vec![
                    party_bytes(&id_spine(t, false)),
                    version_bytes(&hugeleaf(8 * t)),
                    version_bytes(&hugeleaf(8 * t)),
                ])
            }));
        }
        other => panic!("no overlay mapping for operand signature {other:?}"),
    }
    out
}

/// The slice rows' committed fold-cure families, per operation: the
/// whole point of the fold panels is seeing the cured curves against the
/// uniform frontier.
///
/// - `version_join_all` gets the staggered fold populations
///   ([`stagger_population`], bit-reversed feed order preserved), ramped
///   along both committed band axes: arity at a fixed block count, and
///   operand size at a fixed arity.
/// - `version_meet_all` gets the meet-shade population ([`meet_shade`])
///   on the committed diagonal `d = k` (one dense carrier, `k − 1`
///   dominating plateau shades, carrier fed first).
fn slice_overlays(name: &str, max_bytes: usize) -> Vec<FamilyInput> {
    match name {
        "version_join_all" => {
            let mut out = Vec::new();
            // The ramp doubles t from 1, so n = 2t and m = t are always
            // the powers of two the generators require.
            out.extend(ramp("stagger arity (m=4)", max_bytes, |t| {
                Some(stagger_versions(2 * t, 4))
            }));
            out.extend(ramp("stagger size (n=4)", max_bytes, |t| {
                Some(stagger_versions(4, t))
            }));
            out
        }
        "version_meet_all" => ramp("meet_shade (d=k)", max_bytes, |t| {
            let d = 2 * t;
            Some(meet_shade(d, d).iter().map(|v| v.encode()).collect())
        }),
        other => panic!("no committed slice families for {other}"),
    }
}
