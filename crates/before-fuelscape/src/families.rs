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

use before::meter::registry::Shape;
use before::meter::Packed;
use before::Party;

use crate::ops::{Inputs, OpSpec, Operand};

/// One family's inputs at one ramp point: packed encodings in the op's
/// operand order.
pub struct FamilyInput {
    /// The generator's name (the overlay label).
    pub family: &'static str,
    /// Packed encodings, one per operand.
    pub inputs: Vec<Vec<u8>>,
    /// The arity handed to the row's measure function.
    ///
    /// [`ramp`] defaults it to the operand count, which every
    /// host-drawn row either equals or ignores; the guest-split
    /// party-fold row's families override it with their declared share
    /// count.
    pub arity: usize,
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
        let arity = inputs.len();
        out.push(FamilyInput {
            family,
            inputs,
            arity,
        });
        t *= 2;
    }
    out
}

/// The staggered fold population's version operands, encoded in the
/// committed bit-reversed feed order (the order is load-bearing: it is
/// what realizes the intermediate swell at every reduction level).
fn stagger_versions(n: usize, m: usize) -> Vec<Vec<u8>> {
    let (versions, _) = Shape::StaggerPopulation.population(n, m);
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
        Inputs::VersionSliceCapped(cap) => {
            // The capped rows ride the same committed slice families,
            // truncated to the arities their guest dispatch can take.
            return slice_overlays(op.name, max_bytes)
                .into_iter()
                .filter(|family| family.arity <= cap as usize)
                .collect();
        }
        Inputs::ClockSlice => return clock_fold_overlays(max_bytes),
        Inputs::PartyShares => return party_fold_overlays(max_bytes),
    };
    let mut out = Vec::new();
    match operands {
        [Operand::Version] => {
            out.extend(ramp("dense", max_bytes, |t| {
                Some(vec![version_bytes(&Shape::Dense.packed1(t))])
            }));
            out.extend(ramp("harmonic", max_bytes, |t| {
                Some(vec![version_bytes(&Shape::Harmonic.packed1(t))])
            }));
            out.extend(ramp("staircase", max_bytes, |t| {
                Some(vec![version_bytes(&Shape::Staircase.packed1(t))])
            }));
            out.extend(ramp("hugeleaf", max_bytes, |t| {
                Some(vec![version_bytes(&Shape::Hugeleaf.packed1(8 * t))])
            }));
            out.extend(ramp("wide_arming", max_bytes, |t| {
                Some(vec![version_bytes(&Shape::WideArming.packed2(10, t))])
            }));
            out.extend(ramp("dense_suffix", max_bytes, |t| {
                Some(vec![version_bytes(&Shape::DenseSuffix.packed2(t, t))])
            }));
            out.extend(ramp("weight_comb", max_bytes, |t| {
                Some(vec![version_bytes(&Shape::WeightComb.packed1(t))])
            }));
            out.extend(ramp("freeze_parade", max_bytes, |t| {
                Some(vec![version_bytes(&Shape::FreezeParade.packed1(t))])
            }));
            out.extend(ramp("plateau_puncture", max_bytes, |t| {
                Some(vec![version_bytes(&Shape::PlateauPuncture.packed2(10, t))])
            }));
        }
        [Operand::Version, Operand::Version] => {
            out.extend(ramp("jump_pair", max_bytes, |t| {
                let (a, b) = Shape::JumpPair.packed_pair3(8, t, 1);
                Some(vec![version_bytes(&a), version_bytes(&b)])
            }));
            out.extend(ramp("tooth_tail", max_bytes, |t| {
                let (a, b) = Shape::ToothTail.packed_pair(t, 2 * t.max(1));
                Some(vec![version_bytes(&a), version_bytes(&b)])
            }));
            out.extend(ramp("concurrent_pair", max_bytes, |t| {
                let (a, b) = Shape::ConcurrentPair.version_pair(2 * t);
                Some(vec![a.encode(), b.encode()])
            }));
            out.extend(ramp("dense × self", max_bytes, |t| {
                let v = version_bytes(&Shape::Dense.packed1(t));
                Some(vec![v.clone(), v])
            }));
            out.extend(ramp("hugeleaf × self", max_bytes, |t| {
                let v = version_bytes(&Shape::Hugeleaf.packed1(8 * t));
                Some(vec![v.clone(), v])
            }));
        }
        [Operand::Version, Operand::Party] => {
            out.extend(ramp("dense × scattered_id", max_bytes, |t| {
                Some(vec![
                    version_bytes(&Shape::Dense.packed1(t)),
                    party_bytes(&Shape::ScatteredId.packed1(t)),
                ])
            }));
            out.extend(ramp("hugeleaf × id_spine", max_bytes, |t| {
                Some(vec![
                    version_bytes(&Shape::Hugeleaf.packed1(8 * t)),
                    party_bytes(&Shape::IdSpine.packed_flagged(t, false)),
                ])
            }));
        }
        // The binary span-operator rows (two hulled pairs): one point
        // has the committed pair shapes hulled per side, the other
        // crosses a hulled pair with a coincident point (a hulled
        // self-pair), marking the operators' point-combine seam.
        [Operand::Version, Operand::Version, Operand::Version, Operand::Version] => {
            out.extend(ramp("jump_pair × concurrent_pair", max_bytes, |t| {
                let (a, b) = Shape::JumpPair.packed_pair3(8, t, 1);
                let (c, d) = Shape::ConcurrentPair.version_pair(2 * t);
                Some(vec![
                    version_bytes(&a),
                    version_bytes(&b),
                    c.encode(),
                    d.encode(),
                ])
            }));
            out.extend(ramp("tooth_tail × hugeleaf point", max_bytes, |t| {
                let (a, b) = Shape::ToothTail.packed_pair(t, 2 * t.max(1));
                let v = version_bytes(&Shape::Hugeleaf.packed1(8 * t));
                Some(vec![version_bytes(&a), version_bytes(&b), v.clone(), v])
            }));
        }
        // The span projection row (a hulled pair plus the projecting
        // party).
        [Operand::Version, Operand::Version, Operand::Party] => {
            out.extend(ramp("jump_pair × scattered_id", max_bytes, |t| {
                let (a, b) = Shape::JumpPair.packed_pair3(8, t, 1);
                Some(vec![
                    version_bytes(&a),
                    version_bytes(&b),
                    party_bytes(&Shape::ScatteredId.packed1(t)),
                ])
            }));
            out.extend(ramp("concurrent_pair × id_spine", max_bytes, |t| {
                let (a, b) = Shape::ConcurrentPair.version_pair(2 * t);
                Some(vec![
                    a.encode(),
                    b.encode(),
                    party_bytes(&Shape::IdSpine.packed_flagged(t, false)),
                ])
            }));
        }
        // The masked span placement rows (a hulled pair, the masking
        // party, and the probe).
        [Operand::Version, Operand::Version, Operand::Party, Operand::Version] => {
            out.extend(ramp("jump_pair × scattered_id × dense", max_bytes, |t| {
                let (a, b) = Shape::JumpPair.packed_pair3(8, t, 1);
                Some(vec![
                    version_bytes(&a),
                    version_bytes(&b),
                    party_bytes(&Shape::ScatteredId.packed1(t)),
                    version_bytes(&Shape::Dense.packed1(t)),
                ])
            }));
            out.extend(ramp(
                "concurrent_pair × id_spine × hugeleaf",
                max_bytes,
                |t| {
                    let (a, b) = Shape::ConcurrentPair.version_pair(2 * t);
                    Some(vec![
                        a.encode(),
                        b.encode(),
                        party_bytes(&Shape::IdSpine.packed_flagged(t, false)),
                        version_bytes(&Shape::Hugeleaf.packed1(8 * t)),
                    ])
                },
            ));
        }
        // The span placement rows (a hulled pair plus a probe): the
        // committed pair shapes hulled into the span, crossed with a
        // same-scale probe.
        [Operand::Version, Operand::Version, Operand::Version] => {
            out.extend(ramp("jump_pair × dense", max_bytes, |t| {
                let (a, b) = Shape::JumpPair.packed_pair3(8, t, 1);
                Some(vec![
                    version_bytes(&a),
                    version_bytes(&b),
                    version_bytes(&Shape::Dense.packed1(t)),
                ])
            }));
            out.extend(ramp("concurrent_pair × hugeleaf", max_bytes, |t| {
                let (a, b) = Shape::ConcurrentPair.version_pair(2 * t);
                Some(vec![
                    a.encode(),
                    b.encode(),
                    version_bytes(&Shape::Hugeleaf.packed1(8 * t)),
                ])
            }));
        }
        // The masked three-stream comparison row: the projected version
        // and its mask crossed with a compared version of the same shape.
        [Operand::Version, Operand::Party, Operand::Version] => {
            out.extend(ramp("dense × scattered_id × dense", max_bytes, |t| {
                Some(vec![
                    version_bytes(&Shape::Dense.packed1(t)),
                    party_bytes(&Shape::ScatteredId.packed1(t)),
                    version_bytes(&Shape::Dense.packed1(t)),
                ])
            }));
            out.extend(ramp("hugeleaf × id_spine × hugeleaf", max_bytes, |t| {
                Some(vec![
                    version_bytes(&Shape::Hugeleaf.packed1(8 * t)),
                    party_bytes(&Shape::IdSpine.packed_flagged(t, false)),
                    version_bytes(&Shape::Hugeleaf.packed1(8 * t)),
                ])
            }));
        }
        // The masked four-stream comparison row: two full views, each an
        // event family under an id-family mask.
        [Operand::Version, Operand::Party, Operand::Version, Operand::Party] => {
            out.extend(ramp("(dense / scattered_id) × self", max_bytes, |t| {
                Some(vec![
                    version_bytes(&Shape::Dense.packed1(t)),
                    party_bytes(&Shape::ScatteredId.packed1(t)),
                    version_bytes(&Shape::Dense.packed1(t)),
                    party_bytes(&Shape::ScatteredId.packed1(t)),
                ])
            }));
            out.extend(ramp("(hugeleaf / id_spine) × self", max_bytes, |t| {
                Some(vec![
                    version_bytes(&Shape::Hugeleaf.packed1(8 * t)),
                    party_bytes(&Shape::IdSpine.packed_flagged(t, false)),
                    version_bytes(&Shape::Hugeleaf.packed1(8 * t)),
                    party_bytes(&Shape::IdSpine.packed_flagged(t, false)),
                ])
            }));
        }
        [Operand::Party] => {
            out.extend(ramp("scattered_id", max_bytes, |t| {
                Some(vec![party_bytes(&Shape::ScatteredId.packed1(t))])
            }));
            out.extend(ramp("id_spine", max_bytes, |t| {
                Some(vec![party_bytes(&Shape::IdSpine.packed_flagged(t, false))])
            }));
            out.extend(ramp("memo_chain_id", max_bytes, |t| {
                Some(vec![party_bytes(&Shape::MemoChainId.packed1(t))])
            }));
            out.extend(ramp("nested_left_full_id", max_bytes, |t| {
                Some(vec![party_bytes(&Shape::NestedLeftFullId.packed1(t))])
            }));
        }
        // A distinct-pair row cannot take the self pairs (its draw
        // rejects byte-identical operands), so it gets committed crosses.
        [Operand::Party, Operand::Party] if distinct => {
            out.extend(ramp("scattered_id × id_spine", max_bytes, |t| {
                Some(vec![
                    party_bytes(&Shape::ScatteredId.packed1(t)),
                    party_bytes(&Shape::IdSpine.packed_flagged(t, false)),
                ])
            }));
            out.extend(ramp(
                "memo_chain_id × nested_left_full_id",
                max_bytes,
                |t| {
                    Some(vec![
                        party_bytes(&Shape::MemoChainId.packed1(t)),
                        party_bytes(&Shape::NestedLeftFullId.packed1(t)),
                    ])
                },
            ));
        }
        [Operand::Party, Operand::Party] => {
            out.extend(ramp("scattered_id × self", max_bytes, |t| {
                let p = party_bytes(&Shape::ScatteredId.packed1(t));
                Some(vec![p.clone(), p])
            }));
            out.extend(ramp("id_spine × self", max_bytes, |t| {
                let p = party_bytes(&Shape::IdSpine.packed_flagged(t, false));
                Some(vec![p.clone(), p])
            }));
        }
        // The composed-clock rows: an id family for the party half, an
        // event family for the version half.
        [Operand::Party, Operand::Version] => {
            out.extend(ramp("scattered_id × dense", max_bytes, |t| {
                Some(vec![
                    party_bytes(&Shape::ScatteredId.packed1(t)),
                    version_bytes(&Shape::Dense.packed1(t)),
                ])
            }));
            out.extend(ramp("id_spine × hugeleaf", max_bytes, |t| {
                Some(vec![
                    party_bytes(&Shape::IdSpine.packed_flagged(t, false)),
                    version_bytes(&Shape::Hugeleaf.packed1(8 * t)),
                ])
            }));
        }
        // The clock-plus-version rows (recv, and the fork-split pair
        // rows): the composed clock's families with a version rider.
        [Operand::Party, Operand::Version, Operand::Version] => {
            out.extend(ramp("scattered_id × dense × dense", max_bytes, |t| {
                Some(vec![
                    party_bytes(&Shape::ScatteredId.packed1(t)),
                    version_bytes(&Shape::Dense.packed1(t)),
                    version_bytes(&Shape::Dense.packed1(t)),
                ])
            }));
            out.extend(ramp("id_spine × hugeleaf × hugeleaf", max_bytes, |t| {
                Some(vec![
                    party_bytes(&Shape::IdSpine.packed_flagged(t, false)),
                    version_bytes(&Shape::Hugeleaf.packed1(8 * t)),
                    version_bytes(&Shape::Hugeleaf.packed1(8 * t)),
                ])
            }));
        }
        other => panic!("no overlay mapping for operand signature {other:?}"),
    }
    out
}

/// The clock-fold rows' committed families (one party, then the version
/// riders each row composes into disjoint clocks, reconciles, or
/// receives).
///
/// The committed stagger population adapted to the fold's version
/// halves, ramped along both committed band axes as the version folds'
/// families are: operand size at a fixed labeled arity, and arity at a
/// fixed block count — every ramp point a fixed, labeled shape whose
/// reading compares across commits. The arity ramp is what lets the
/// marked frontier trace the fold's log-k factor; the size-only ramps
/// trace `D` at their fixed count. The composed clock families' cross
/// rides as the non-stagger shape.
fn clock_fold_overlays(max_bytes: usize) -> Vec<FamilyInput> {
    let mut out = Vec::new();
    out.extend(ramp("scattered_id × stagger (n=4)", max_bytes, |t| {
        let mut inputs = vec![party_bytes(&Shape::ScatteredId.packed1(t))];
        inputs.extend(stagger_versions(4, t));
        Some(inputs)
    }));
    out.extend(ramp(
        "scattered_id × stagger arity (m=4)",
        max_bytes,
        |t| {
            let mut inputs = vec![party_bytes(&Shape::ScatteredId.packed1(4))];
            inputs.extend(stagger_versions(2 * t, 4));
            Some(inputs)
        },
    ));
    out.extend(ramp("id_spine × hugeleaf⁴", max_bytes, |t| {
        let mut inputs = vec![party_bytes(&Shape::IdSpine.packed_flagged(t, false))];
        inputs.extend(
            std::iter::repeat_with(|| version_bytes(&Shape::Hugeleaf.packed1(8 * t))).take(4),
        );
        Some(inputs)
    }));
    out
}

/// The declared share count of the party-fold row's committed overlay
/// points, stated in every family label.
///
/// The bulk cloud carries the drawn-arity axis, and these points keep
/// one fixed arity so their readings compare across commits.
const PARTY_FOLD_OVERLAY_SHARES: usize = 8;

/// The party-fold row's committed families: the committed id shapes,
/// each split into and re-merged from [`PARTY_FOLD_OVERLAY_SHARES`]
/// balanced shares by the row's measure.
fn party_fold_overlays(max_bytes: usize) -> Vec<FamilyInput> {
    let mut out = Vec::new();
    out.extend(ramp("scattered_id (k=8)", max_bytes, |t| {
        Some(vec![party_bytes(&Shape::ScatteredId.packed1(t))])
    }));
    out.extend(ramp("id_spine (k=8)", max_bytes, |t| {
        Some(vec![party_bytes(&Shape::IdSpine.packed_flagged(t, false))])
    }));
    out.extend(ramp("memo_chain_id (k=8)", max_bytes, |t| {
        Some(vec![party_bytes(&Shape::MemoChainId.packed1(t))])
    }));
    out.extend(ramp("nested_left_full_id (k=8)", max_bytes, |t| {
        Some(vec![party_bytes(&Shape::NestedLeftFullId.packed1(t))])
    }));
    for family in &mut out {
        family.arity = PARTY_FOLD_OVERLAY_SHARES;
    }
    out
}

/// The staggered fold populations (bit-reversed feed order preserved),
/// ramped along both committed band axes: arity at a fixed block
/// count, and operand size at a fixed arity.
///
/// The ramp doubles `t` from 1, so `n = 2t` and `m = t` are always the
/// powers of two the generators require.
fn stagger_ramps(max_bytes: usize) -> Vec<FamilyInput> {
    let mut out = Vec::new();
    out.extend(ramp("stagger arity (m=4)", max_bytes, |t| {
        Some(stagger_versions(2 * t, 4))
    }));
    out.extend(ramp("stagger size (n=4)", max_bytes, |t| {
        Some(stagger_versions(4, t))
    }));
    out
}

/// The meet-shade population on the committed diagonal `d = k` (one
/// dense carrier, `k − 1` dominating plateau shades, carrier fed
/// first).
fn meet_shade_ramp(max_bytes: usize) -> Vec<FamilyInput> {
    ramp("meet_shade (d=k)", max_bytes, |t| {
        let d = 2 * t;
        Some(
            Shape::MeetShade
                .versions(d, d)
                .iter()
                .map(|v| v.encode())
                .collect(),
        )
    })
}

/// The slice rows' committed fold-cure families, per operation: the
/// whole point of the fold panels is seeing the cured curves against the
/// uniform frontier.
///
/// - `version_join_all` gets the staggered fold populations
///   ([`stagger_ramps`]): the join fold's committed swell shapes.
/// - `version_meet_all` gets the meet-shade population
///   ([`meet_shade_ramp`]): the meet fold's committed diagonal.
/// - `version_span_all` carries both hull endpoints through one fold,
///   so it gets both sides' committed shapes — and the span fold rows
///   composed from drawn versions follow the same assignment by the
///   lattice directions their legs fold: the containment doors
///   (`span_union_all`, `span_intersect_all`) fold one leg each way
///   and get both, the pointwise join (`span_join_all`) folds join legs
///   only and gets the staggers, the pointwise meet
///   (`span_meet_all`) folds meet legs only and gets the shade.
fn slice_overlays(name: &str, max_bytes: usize) -> Vec<FamilyInput> {
    match name {
        "version_join_all" | "span_join_all" => stagger_ramps(max_bytes),
        // The combiner walks whatever version family it is handed; the
        // committed staggered fold population is the n-ary family with
        // committed generators, so its ramp marks the panel.
        "shape_combine" => stagger_ramps(max_bytes),
        "version_meet_all" | "span_meet_all" => meet_shade_ramp(max_bytes),
        "version_span_all" | "span_union_all" | "span_intersect_all" => {
            let mut out = stagger_ramps(max_bytes);
            out.extend(meet_shade_ramp(max_bytes));
            out
        }
        other => panic!("no committed slice families for {other}"),
    }
}
