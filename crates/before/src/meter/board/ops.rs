//! The operation axis: the board's row table.
//!
//! Each row declares the bundle slots its signature consumes and prepares its
//! cell from them alone — never from the shape's identity — so a row reaches
//! every shape that supplies its operands.

use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::error::{Decode, Parse};
use crate::{causally, Clock, Party, Rank, Ranked, Span, Version};

use super::ceilings::{
    both_present_nodes, ASCEND_CLIFF_MIN_TICKS_HEAP_BYTES_PER_INPUT_BYTE,
    ASCEND_CLIFF_TICK_HEAP_BYTES_PER_INPUT_BYTE, INDEX_PROBE_SCAN_BITS,
    MACHINE_WORD_MAGNITUDE_BITS, MIRROR_WIDE_RENDER_LIMB_EXPONENT_CEILING,
    MIRROR_WIDE_RENDER_LIMB_OPS_PER_RADIX_UNIT, TICKS_BOARD_COUNT,
};
use super::cell::{assert_honest_text, Cell, TextSpec};
use super::currency::{Floors, Liveness};
use super::defect::{
    clock_trailing_text, party_noncanonical_bytes, party_noncanonical_text, trailing_bytes,
    trailing_text, truncated_bytes, version_noncanonical_bytes, version_noncanonical_text,
};
use super::family::{
    decode_party, decode_version, overlap_fold_probe, FamilyData, MIN_SIZE_PARAM,
    OVERLAP_FOLD_INPUT_DIVISOR,
};
use super::floors::{
    clock_overlap_floors, comparison_floors, heap_materializes, id_rejection_floors, limb_stream,
    limb_wide, masked_cmp_floors, membership_floors, na, rejection_floors, scan_examines,
    scan_touch, seg_ceiling_only, sync_floors, text_rejection_floors, tick_walk_floors,
    touch_delta_fold, touch_fold_first_merges, touch_pair_fold, touch_wide_stream, walk_floors,
    NA_HEAP_FORK_SHARES, NA_HEAP_IN_PLACE, NA_LIMB_DEPENDENCY, NA_LIMB_ID_TREE, NA_LIMB_NARROW,
    NA_LIMB_NOT_FORCED, NA_LIMB_REJECTION, NA_SCAN_BYTE_COPY, NA_SCAN_EQ_BYTES, NA_SCAN_NO_STREAM,
    NA_SCAN_RANK_BYTES, NA_SCAN_SEED_PARTY, NA_SCAN_SEED_PROJECTION, NA_TOUCH_GROW,
    NA_TOUCH_ID_TREE, NA_TOUCH_NOT_FORCED, NA_TOUCH_PLACEMENT, NA_TOUCH_PROJECTION,
    NA_TOUCH_RANK_ARITHMETIC, NA_TOUCH_REJECTION, NA_TOUCH_RENDER_SUMMARIES, NA_TOUCH_SEED_RAISE,
    WHY_HEAP_FORK_HALF, WHY_LIMB_RANK_DECODE, WHY_LIMB_RANK_ENCODE, WHY_LIMB_RANK_PAIR,
    WHY_LIMB_RANK_SUM, WHY_SCAN_EXAMINES, WHY_SCAN_OVERLAP_END, WHY_SCAN_REJECT_CROSSED,
    WHY_SCAN_REJECT_END, WHY_TOUCH_RANK_SUM,
};
use super::operand::{
    mandatory_limbs_stream, mandatory_limbs_version, radix_units_clock, radix_units_party,
    radix_units_version, stored_bases, stored_nonzero_deltas, version_output_bytes,
};
use crate::meter::registry::FamilyId;

/// One board row: a public operation and how to instantiate it per family.
pub(super) struct Op {
    /// The row label, `type_operation`.
    pub(super) name: &'static str,
    /// The signature group the row belongs to on the operation axis.
    pub(super) group: OpGroup,
    /// Build the cell for one shape, or `None` where the shape's bundle
    /// supplies no operand for this operation's signature.
    pub(super) prepare: fn(&FamilyData) -> Option<Cell>,
}

/// The operation axis's signature groups.
///
/// A group names the operand signature a row consumes; the bench mirror's
/// pinned subset pairs each shape with the groups it was designed to
/// stress ([`designed`]), so the subset is a rule over the same two axes
/// the board's product runs on, never a second cell list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OpGroup {
    /// Rows over a shape's version operands (codec, comparison, merge,
    /// text, hash rows).
    Version,
    /// The linear-functional query rows: `rank`, `distance`, `lag`,
    /// `min_ticks`.
    Measure,
    /// The rank-value rows: `rank_pair_ops`, `rank_sum`.
    Rank,
    /// The tick rows, driven through a cross shape's designated pairing.
    Tick,
    /// The projection rows: the explicit materializations and the fused
    /// lazy comparisons.
    ///
    /// `own_version_to_version` and `clock_own_version_to_version` price
    /// the explicit materialization; `own_version_cmp` and
    /// `own_version_pair_cmp` price the fused comparisons, which stay
    /// input-denominated on every shape — a comparison never
    /// materializes the projection.
    Projection,
    /// The fold rows: `version_join_all`, `version_meet_all`,
    /// `version_span_all`, `party_join_all`, and
    /// `party_join_all_overlap`.
    Fold,
    /// Rows over a shape's disjoint party pair.
    Party,
    /// Rows over a shape's clock (the tick and projection clock rows
    /// carry their own groups above).
    Clock,
}

/// The shape × operation-group pairings each shape was designed to
/// stress: the bench mirror's diagonal.
///
/// Declared per shape, on the shape axis, so the pinned bench subset is
/// derived — a shape added to the axis must answer which groups it was
/// built against (the exhaustive match), and the subset follows. The
/// deterministic board itself never consults this: it runs the whole
/// product.
pub(super) fn designed(kind: FamilyId, group: OpGroup) -> bool {
    match kind {
        // The original full-surface adversaries and the organic control,
        // plus the two population shapes (whose bundles already narrow
        // them to their party/fold rows).
        FamilyId::Dense | FamilyId::Benign | FamilyId::IdPair | FamilyId::Scatter => true,
        // The magnitude shapes predate the rank rows' mismatch pair and
        // were never its designed adversary.
        FamilyId::Bigroot | FamilyId::Hugeleaf | FamilyId::Cliff => group != OpGroup::Rank,
        // The rank fold's wide-numerator adversary.
        FamilyId::Harmonic => matches!(group, OpGroup::Measure | OpGroup::Rank),
        // The output-domination cross.
        FamilyId::CombScatter => group == OpGroup::Projection,
        // The correlated fold populations, built against the fold rows:
        // weave loads the up-front overlap test, stagger the balanced
        // reduction's intermediate swell.
        FamilyId::Weave | FamilyId::Stagger => group == OpGroup::Fold,
        // The tick-walk crosses.
        FamilyId::NestedFull
        | FamilyId::NestedWide
        | FamilyId::MirrorWide
        | FamilyId::MirrorNarrow
        | FamilyId::Staircase
        | FamilyId::RevealComb
        | FamilyId::RevealHifloor
        | FamilyId::PureComb
        | FamilyId::AscendCliff
        | FamilyId::AscendPlateau
        | FamilyId::DominatedUndercut => group == OpGroup::Tick,
        // The query-fold adversaries, built against the
        // linear-functional rows: wide difference crests over a
        // dense-position spine, the many-freezes spine, the
        // many-armings spine, the accumulator skip families (the
        // many-jumps and deep-segment-freeze spines), the settle
        // sentinels (the many-armings re-arm spine over its unit mate,
        // the single wide arming over the same spine, the
        // answer-embedded product, and the first-freeze gate
        // straddle), and the switch-density population.
        // The tooth-tail pair rides the same designation: its genre is
        // the fused pair sweep, which the distance/lag rows drive (the
        // cmp row runs the identical walk). Wide-arming's text-parse
        // cells ride the deterministic board columns and the
        // `parse_wide_arming` envelope band, as tooth-tail's parse
        // cell does.
        FamilyId::JumpPair
        | FamilyId::FreezePos
        | FamilyId::PromoRearm
        | FamilyId::WeightComb
        | FamilyId::FreezeParade
        | FamilyId::DenseSuffix
        | FamilyId::WideArming
        | FamilyId::PlateauPuncture
        | FamilyId::LoneFreeze
        | FamilyId::ConcurrentPair
        | FamilyId::ToothTail => group == OpGroup::Measure,
        // Envelope-only families never reach the board's product, so
        // they have no designed diagonal.
        FamilyId::WideToothComb
        | FamilyId::JumpComb
        | FamilyId::CliffFan
        | FamilyId::CancellingChain
        | FamilyId::AltSpine
        | FamilyId::MemoChain
        | FamilyId::MemoComb
        | FamilyId::MemoFanout
        | FamilyId::MemoOscillating
        | FamilyId::MemoChurn
        | FamilyId::DescendingRaises
        | FamilyId::MaskDrift
        | FamilyId::MeetShade
        | FamilyId::ArmingTrain
        | FamilyId::ScanHole
        | FamilyId::MaskedHole
        | FamilyId::HoistedWindow
        | FamilyId::PropagateSeam
        | FamilyId::LatentLadder => unreachable!(
            "{kind:?} is envelope-only in the registry: the bench mirror derives its \
             subset from the board roster alone"
        ),
    }
}

/// The operation table: every public operation with a meaningful packed
/// operand ([`BOARD_NOT_APPLICABLE`](super::coverage::BOARD_NOT_APPLICABLE)
/// and the `coverage` module doc list the rest).
#[allow(clippy::too_many_lines)]
pub(super) fn ops() -> Vec<Op> {
    vec![
        // ── Version ────────────────────────────────────────────────────
        Op {
            name: "version_decode",
            group: OpGroup::Version,
            prepare: |f| {
                let bytes = f.version.clone()?;
                let v = decode_version(&bytes);
                let floors = Floors {
                    heap: heap_materializes(bytes.len()),
                    limb: limb_stream(mandatory_limbs_stream(&v)),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(bytes.len()),
                    touch: touch_wide_stream(&v),
                };
                Some(Cell::new(bytes.len(), floors, move || {
                    decode_version(&bytes)
                }))
            },
        },
        Op {
            name: "version_encode",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, n) = f.version()?;
                let floors = Floors {
                    heap: heap_materializes(n),
                    limb: na(NA_LIMB_NOT_FORCED),
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_BYTE_COPY),
                    touch: na(NA_TOUCH_NOT_FORCED),
                };
                Some(Cell::new(n, floors, move || (v.encode(), v)))
            },
        },
        Op {
            name: "version_cmp",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                let floors = comparison_floors(&v, &w, n);
                Some(Cell::new(n, floors, move || {
                    let ord: Option<Ordering> = v.partial_cmp(&w);
                    (ord, v, w)
                }))
            },
        },
        Op {
            name: "version_eq",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: na(NA_LIMB_NOT_FORCED),
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_EQ_BYTES),
                    touch: na(NA_TOUCH_NOT_FORCED),
                };
                Some(Cell::new(n, floors, move || (v == w, v, w)))
            },
        },
        Op {
            name: "version_concurrent",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                let floors = comparison_floors(&v, &w, n);
                Some(Cell::new(n, floors, move || (v.concurrent(&w), v, w)))
            },
        },
        Op {
            name: "version_join",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                let touch = touch_pair_fold(&v, &w);
                Some(Cell::new(n, walk_floors(n, touch), move || (&v | &w, v, w)))
            },
        },
        Op {
            name: "version_join_assign",
            group: OpGroup::Version,
            prepare: |f| {
                let (mut v, w, n) = f.version_pair()?;
                let touch = touch_pair_fold(&v, &w);
                Some(Cell::new(n, walk_floors(n, touch), move || {
                    v |= &w;
                    (v, w)
                }))
            },
        },
        Op {
            name: "version_meet",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                let touch = touch_pair_fold(&v, &w);
                Some(Cell::new(n, walk_floors(n, touch), move || (&v & &w, v, w)))
            },
        },
        Op {
            name: "version_meet_assign",
            group: OpGroup::Version,
            prepare: |f| {
                let (mut v, w, n) = f.version_pair()?;
                let touch = touch_pair_fold(&v, &w);
                Some(Cell::new(n, walk_floors(n, touch), move || {
                    v &= &w;
                    (v, w)
                }))
            },
        },
        Op {
            name: "version_span",
            group: OpGroup::Version,
            prepare: |f| {
                // The fused pair hull: one sweep feeds both endpoints,
                // so the cell carries the same walk floors as the
                // single-op join/meet rows it undercuts.
                let (v, w, n) = f.version_pair()?;
                let touch = touch_pair_fold(&v, &w);
                Some(Cell::new(n, walk_floors(n, touch), move || {
                    (v.span(&w), v, w)
                }))
            },
        },
        Op {
            name: "span_encode",
            group: OpGroup::Version,
            prepare: |f| {
                // The composite emission over the pair's hull (built at
                // prepare, outside measurement): one byte copy per
                // endpoint — the codec emission genre, denominated by
                // the span's own packed size, which is exactly the
                // output.
                let (v, w, _) = f.version_pair()?;
                let span = v.span(&w);
                let n = span.encode().len();
                let floors = Floors {
                    heap: heap_materializes(n),
                    limb: na(NA_LIMB_NOT_FORCED),
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_BYTE_COPY),
                    touch: na(NA_TOUCH_NOT_FORCED),
                };
                Some(Cell::new(n, floors, move || (span.encode(), span)))
            },
        },
        Op {
            name: "span_decode",
            group: OpGroup::Version,
            prepare: |f| {
                // The hull's composite, encoded at prepare, outside
                // measurement. The fused decode parses the first
                // component, then one admission walk parses the second
                // while validating dominance against the first — so the
                // floors are the first component's parse plus the pair
                // comparison's: both endpoints materialize, every
                // stored payload of both streams decodes once, the
                // whole composite is examined, and the walk folds the
                // pair's nonzero deltas. The second component's
                // standalone validation accumulator is what the fusion
                // deletes, so no floor may demand it.
                let (v, w, _) = f.version_pair()?;
                let span = v.span(&w);
                let bytes = span.encode();
                let n = bytes.len();
                let (lo, hi) = span.into_parts();
                let floors = Floors {
                    heap: heap_materializes(n),
                    limb: limb_stream(mandatory_limbs_stream(&lo) + mandatory_limbs_stream(&hi)),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: touch_pair_fold(&lo, &hi),
                };
                Some(Cell::new(n, floors, move || {
                    Span::decode(&bytes[..]).expect("a canonical composite decodes")
                }))
            },
        },
        Op {
            name: "version_tick",
            group: OpGroup::Tick,
            prepare: |f| {
                // The tick-walk families carry their own (event, id)
                // pair; every other family ticks its version with the
                // seed.
                if let Some((mut v, party, n)) = f.cross() {
                    let floors = tick_walk_floors(&v, n);
                    let cell = Cell::new(n, floors, move || {
                        v.tick(&party);
                        (v, party)
                    });
                    // The ascending cliff defeats certificate consumption,
                    // so its tick cells carry the ratified family-stated
                    // heap ceiling (the constant's derivation).
                    return Some(if matches!(f.kind, FamilyId::AscendCliff) {
                        cell.with_declared_heap(ASCEND_CLIFF_TICK_HEAP_BYTES_PER_INPUT_BYTE)
                    } else {
                        cell
                    });
                }
                let (mut v, n) = f.version()?;
                let party = Party::seed();
                Some(Cell::new(
                    n + 1,
                    walk_floors(n, na(NA_TOUCH_SEED_RAISE)),
                    move || {
                        v.tick(&party);
                        v
                    },
                ))
            },
        },
        Op {
            name: "version_ticks",
            group: OpGroup::Tick,
            prepare: |f| {
                // The fused multi-tick at a fixed count: the same walk
                // and splice as the tick rows, with the count's gamma
                // width the only n-dependence — so the cell must scale
                // exactly as the tick cell above it (the flatness rows
                // of tests/meter.rs pin the n axis; this cell pins the
                // input axis).
                if let Some((mut v, party, n)) = f.cross() {
                    let floors = tick_walk_floors(&v, n);
                    let cell = Cell::new(n, floors, move || {
                        v.ticks(&party, TICKS_BOARD_COUNT);
                        (v, party)
                    });
                    // As the tick cell above: the ascending cliff's
                    // certificate memory is family-stated.
                    return Some(if matches!(f.kind, FamilyId::AscendCliff) {
                        cell.with_declared_heap(ASCEND_CLIFF_TICK_HEAP_BYTES_PER_INPUT_BYTE)
                    } else {
                        cell
                    });
                }
                let (mut v, n) = f.version()?;
                let party = Party::seed();
                Some(Cell::new(
                    n + 1,
                    walk_floors(n, na(NA_TOUCH_SEED_RAISE)),
                    move || {
                        v.ticks(&party, TICKS_BOARD_COUNT);
                        v
                    },
                ))
            },
        },
        Op {
            name: "version_tick_adv_party",
            group: OpGroup::Party,
            prepare: |f| {
                let (a, _, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(a, _)| a.len())?;
                let mut v = Version::new();
                Some(Cell::new(
                    n + 1,
                    walk_floors(n, na(NA_TOUCH_GROW)),
                    move || {
                        v.tick(&a);
                        (v, a)
                    },
                ))
            },
        },
        Op {
            name: "version_rank",
            group: OpGroup::Measure,
            prepare: |f| {
                let (v, n) = f.version()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: limb_stream(mandatory_limbs_stream(&v)),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: touch_delta_fold(stored_nonzero_deltas(&v)),
                };
                Some(Cell::new(n, floors, move || (v.rank(), v)))
            },
        },
        Op {
            name: "rank_pair_ops",
            group: OpGroup::Rank,
            prepare: |f| {
                // The mismatched pair: a family-derived rank (maximal
                // exponent on the spines) against a small integer rank, on
                // the spine families that maximize the mismatch plus the
                // benign control. Ranks are built at family construction,
                // outside measurement; the denominator is the pair's value
                // content (the `cell` module doc's rank denomination).
                let (a, b) = f.rank_pair.clone()?;
                let n = (a.content_bits() + b.content_bits()).div_ceil(8) as usize;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: Liveness::Floor {
                        min: a.content_bits().max(b.content_bits()).div_ceil(64),
                        why: WHY_LIMB_RANK_PAIR,
                    },
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_NO_STREAM),
                    touch: na(NA_TOUCH_RANK_ARITHMETIC),
                };
                Some(Cell::new(n, floors, move || {
                    let ord = a.cmp(&b);
                    // One direction of the pair dominates; keep whichever
                    // difference exists so the subtraction always runs.
                    let diff = a.checked_sub(&b).or_else(|| b.checked_sub(&a));
                    let sum = &a + &b;
                    (ord, diff, sum, a, b)
                }))
            },
        },
        Op {
            name: "rank_sum",
            group: OpGroup::Rank,
            prepare: |f| {
                // The mixed fold: the family-derived rank (maximal exponent
                // on the spines) summed high-first with one small integer
                // rank per packed byte of the family's measure operand, so
                // both sides of the value content scale together. High-first
                // is the committed adversarial order: `Sum` accepts arbitrary
                // order, and under a fold that re-normalizes per element it
                // is the order that makes every later add a full-width
                // operation. The denominator is the summands' total value
                // content (the `cell` module doc's rank denomination).
                let (a, _) = f.rank_pair.clone()?;
                let (_, k) = f.version()?;
                let ones: Vec<Rank> = (0..k)
                    .map(|i| {
                        Version::try_from(i as u64 % 7 + 1)
                            .expect("a small integer version is valid")
                            .rank()
                    })
                    .collect();
                let n = (a.content_bits().div_ceil(8) as usize)
                    + ones
                        .iter()
                        .map(|r| r.content_bits().div_ceil(8) as usize)
                        .sum::<usize>();
                let wide = a.content_bits();
                let limb = if wide > MACHINE_WORD_MAGNITUDE_BITS {
                    Liveness::Floor {
                        min: wide.div_ceil(64),
                        why: WHY_LIMB_RANK_SUM,
                    }
                } else {
                    na(NA_LIMB_NARROW)
                };
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb,
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_NO_STREAM),
                    touch: Liveness::Floor {
                        min: ones.len() as u64 + 1,
                        why: WHY_TOUCH_RANK_SUM,
                    },
                };
                Some(Cell::new(n, floors, move || {
                    std::iter::once(a).chain(ones).sum::<Rank>()
                }))
            },
        },
        Op {
            name: "rank_encode",
            group: OpGroup::Rank,
            prepare: |f| {
                // The family-derived rank (maximal exponent on the
                // spines), built at family construction, outside
                // measurement. I/O-denominated: value content in, the
                // actual canonical bytes out (the `cell` module doc's
                // rank denomination), with the emission's honesty
                // asserted here — a canonical encoding is at most
                // `9⁄8 · ‖r‖ + O(log ‖r‖)` bits, so padding the output
                // side of the denominator trips the run instead of
                // greening the cell. Derivation of the bound: the
                // stream is `2ρ + w` header/mantissa bits plus
                // `9 · ⌈exp/8⌉ + 1` fraction bits; `w ≤ bits(⌊r⌋) + 1`
                // and `exp` each sit inside `content = bits(num) + exp`
                // (so the 9⁄8 shows up as `content/8`), and `2ρ + 11`
                // stays under 64 for any integral part narrower than
                // 2²⁶ bits — orders beyond every committed family.
                let (a, _) = f.rank_pair.clone()?;
                let content = a.content_bits();
                let encoded_len = a.encode().len();
                assert!(
                    (encoded_len as u64) * 8 <= content + content / 8 + 64,
                    "output honesty: a canonical rank encoding is at most \
                     9/8 content + O(log) bits"
                );
                let floors = Floors {
                    heap: heap_materializes(encoded_len),
                    limb: Liveness::Floor {
                        min: rank_numerator_limbs(&a),
                        why: WHY_LIMB_RANK_ENCODE,
                    },
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_NO_STREAM),
                    touch: na(NA_TOUCH_RANK_ARITHMETIC),
                };
                Some(Cell::io(
                    content.div_ceil(8) as usize,
                    floors,
                    |result| {
                        result
                            .downcast_ref::<(Vec<u8>, Rank)>()
                            .expect("the rank_encode cell keeps its bytes")
                            .0
                            .len()
                    },
                    move || (a.encode(), a),
                ))
            },
        },
        Op {
            name: "rank_decode",
            group: OpGroup::Rank,
            prepare: |f| {
                // The canonical bytes of the family-derived rank,
                // encoded at prepare, outside measurement; the operand
                // is the byte string itself, input-denominated like
                // every codec row (the coding is canonical 1:1).
                let (a, _) = f.rank_pair.clone()?;
                let bytes = a.encode();
                let numerator_bytes = (rank_numerator_limbs(&a) * 8) as usize;
                let floors = Floors {
                    heap: heap_materializes(numerator_bytes),
                    limb: Liveness::Floor {
                        min: rank_numerator_limbs(&a),
                        why: WHY_LIMB_RANK_DECODE,
                    },
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_RANK_BYTES),
                    touch: na(NA_TOUCH_RANK_ARITHMETIC),
                };
                Some(Cell::new(bytes.len(), floors, move || {
                    Rank::decode(&bytes[..]).expect("a canonical rank encoding decodes")
                }))
            },
        },
        Op {
            name: "version_distance",
            group: OpGroup::Measure,
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: limb_stream(mandatory_limbs_stream(&v) + mandatory_limbs_stream(&w)),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: touch_pair_fold(&v, &w),
                };
                Some(Cell::new(n, floors, move || (v.distance(&w), v, w)))
            },
        },
        Op {
            name: "version_lag",
            group: OpGroup::Measure,
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: limb_stream(mandatory_limbs_stream(&v) + mandatory_limbs_stream(&w)),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: touch_pair_fold(&v, &w),
                };
                Some(Cell::new(n, floors, move || (v.lag(&w), v, w)))
            },
        },
        Op {
            name: "ranked_cmp",
            group: OpGroup::Measure,
            prepare: |f| {
                // The fused rank comparison: the distance/lag co-sweep
                // with constant orientation, so it takes their floors
                // (the walk decodes both streams, folds every nonzero
                // delta, and answers a word-scale verdict).
                let (v, w, n) = f.version_pair()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: limb_stream(mandatory_limbs_stream(&v) + mandatory_limbs_stream(&w)),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: touch_pair_fold(&v, &w),
                };
                Some(Cell::new(n, floors, move || {
                    let ord = Ranked::from(&v).cmp(&Ranked::from(&w));
                    (ord, v, w)
                }))
            },
        },
        Op {
            name: "ranked_encode",
            group: OpGroup::Measure,
            prepare: |f| {
                // The composite key emission: the fused rank fold's
                // floors plus the mandatory output (rank stream, then
                // one copy of the version's packed bytes).
                // Input-denominated: the provenance pin bounds the
                // rank component within the packed input plus one
                // byte, and the version tail is exactly the packed
                // input again (asserted here, so the bound is enforced
                // at every family and scale), which makes input bytes
                // the honest, harder denominator — the codec rows'
                // rule — and lets the flat-denominator shape's content
                // exponent govern exactly as on the version_rank cell
                // this row extends.
                let (v, n) = f.version()?;
                let encoded_len = Ranked::from(&v).encode().len();
                assert!(
                    encoded_len <= 2 * n + 1,
                    "output honesty: a composite ranked key is the rank emission \
                     (within the packed input plus one byte) plus the version's \
                     packed bytes"
                );
                let floors = Floors {
                    heap: heap_materializes(encoded_len),
                    limb: limb_stream(mandatory_limbs_stream(&v)),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: touch_delta_fold(stored_nonzero_deltas(&v)),
                };
                Some(Cell::new(n, floors, move || (Ranked::from(&v).encode(), v)))
            },
        },
        Op {
            name: "ranked_encode_rank",
            group: OpGroup::Measure,
            prepare: |f| {
                // The fused rank-to-bytes emission: the rank fold's
                // floors plus the mandatory output. Input-denominated:
                // the provenance pin bounds the output within the
                // packed input (asserted here, so the bound is
                // enforced at every family and scale), which makes
                // input bytes the honest, harder denominator — the
                // codec rows' rule — and lets the flat-denominator
                // shape's content exponent govern exactly as on the
                // version_rank cell this row extends.
                let (v, n) = f.version()?;
                let encoded_len = Ranked::from(&v).encode_rank().len();
                assert!(
                    encoded_len <= n + 1,
                    "output honesty: a version-derived rank encodes within its \
                     version's packed bytes"
                );
                let floors = Floors {
                    heap: heap_materializes(encoded_len),
                    limb: limb_stream(mandatory_limbs_stream(&v)),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: touch_delta_fold(stored_nonzero_deltas(&v)),
                };
                Some(Cell::new(n, floors, move || {
                    (Ranked::from(&v).encode_rank(), v)
                }))
            },
        },
        Op {
            name: "ranked_decode",
            group: OpGroup::Measure,
            prepare: |f| {
                // The composite key of the family's version, encoded at
                // prepare, outside measurement; the operand is the byte
                // string itself, input-denominated like every codec row
                // (the coding is canonical 1:1). The decode's dominant
                // term is the verifying rank fold over the decoded
                // version, so the row takes the fold's floors (the
                // walk decodes the stream and folds every nonzero
                // delta) plus the materialized owned version.
                let (v, n) = f.version()?;
                let bytes = Ranked::from(&v).encode();
                let floors = Floors {
                    heap: heap_materializes(n),
                    limb: limb_stream(mandatory_limbs_stream(&v)),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: touch_delta_fold(stored_nonzero_deltas(&v)),
                };
                Some(Cell::new(bytes.len(), floors, move || {
                    Ranked::decode(&bytes[..]).expect("a canonical composite key decodes")
                }))
            },
        },
        Op {
            name: "version_min_ticks",
            group: OpGroup::Measure,
            prepare: |f| {
                // The exact fold walks the whole stream, decodes every
                // stored code, and folds heights and minima on
                // accumulators: the rank fold's floor spec exactly.
                let (v, n) = f.version()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: limb_stream(mandatory_limbs_stream(&v)),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: touch_delta_fold(stored_nonzero_deltas(&v)),
                };
                let cell = Cell::new(n, floors, move || (v.min_ticks(), v));
                // The ascending cliff defeats reign batching, so this
                // cell carries the ratified family-stated heap ceiling
                // (the constant's derivation).
                Some(if matches!(f.kind, FamilyId::AscendCliff) {
                    cell.with_declared_heap(ASCEND_CLIFF_MIN_TICKS_HEAP_BYTES_PER_INPUT_BYTE)
                } else {
                    cell
                })
            },
        },
        Op {
            name: "version_join_all",
            group: OpGroup::Fold,
            prepare: |f| {
                let (versions, _) = f.fold.as_ref()?;
                let n = versions.iter().map(Vec::len).sum();
                let mut versions: Vec<Version> =
                    versions.iter().map(|b| decode_version(b)).collect();
                let arity = versions.len() as u64;
                let touch = touch_fold_first_merges(&versions);
                let rest = versions.split_off(1);
                let receiver = versions.pop()?;
                Some(
                    Cell::new(n, walk_floors(n, touch), move || receiver.join_all(rest))
                        .with_fold_arity(arity),
                )
            },
        },
        Op {
            name: "version_meet_all",
            group: OpGroup::Fold,
            prepare: |f| {
                // The meet fold: the join fold's balanced reduction over
                // the meet emitter, so the same declared fold model and
                // the same first-level touch floor (the fused pair sweep
                // walks both operands of every first-level merge; later
                // levels' groups shrink toward the population's meet and
                // canonical identity answers equal groups before any
                // sweep).
                let (versions, _) = f.fold.as_ref()?;
                let n = versions.iter().map(Vec::len).sum();
                let mut versions: Vec<Version> =
                    versions.iter().map(|b| decode_version(b)).collect();
                let arity = versions.len() as u64;
                let touch = touch_fold_first_merges(&versions);
                let rest = versions.split_off(1);
                let receiver = versions.pop()?;
                Some(
                    Cell::new(n, walk_floors(n, touch), move || receiver.meet_all(rest))
                        .with_fold_arity(arity),
                )
            },
        },
        Op {
            name: "version_span_all",
            group: OpGroup::Fold,
            prepare: |f| {
                // The hull fold: one balanced reduction carrying both
                // endpoints, its leaf combines fused (each first-level
                // pair decoded once for both directions), so the same
                // declared fold model and first-level touch floor as
                // the single-direction fold rows apply.
                let (versions, _) = f.fold.as_ref()?;
                let n = versions.iter().map(Vec::len).sum();
                let versions: Vec<Version> = versions.iter().map(|b| decode_version(b)).collect();
                let arity = versions.len() as u64;
                let touch = touch_fold_first_merges(&versions);
                Some(
                    Cell::new(n, walk_floors(n, touch), move || {
                        let hull = versions[0].span_all(&versions[1..]);
                        (hull, versions)
                    })
                    .with_fold_arity(arity),
                )
            },
        },
        Op {
            name: "own_version_to_version",
            group: OpGroup::Projection,
            prepare: |f| {
                // The explicit materialization `(&v / &p).to_version()`:
                // the one projection spelling that pays the product-growth
                // output. Adversarial × adversarial with mandatory
                // dominating output: the declared output-domination cross,
                // I/O-denominated.
                if f.output_dominated {
                    let (v_bytes, p_bytes) = f.cross.as_ref()?;
                    let n = v_bytes.len() + p_bytes.len();
                    let v = decode_version(v_bytes);
                    let p = decode_party(p_bytes);
                    let cell = Cell::io(
                        n,
                        walk_floors(n, na(NA_TOUCH_PROJECTION)),
                        |r| {
                            let (out, _, _) = r
                                .downcast_ref::<(Version, Version, Party)>()
                                .expect("the cross projection body yields (out, v, p)");
                            version_output_bytes(out)
                        },
                        move || ((&v / &p).to_version(), v, p),
                    );
                    // The comb-scatter cross's builder runs the ratified
                    // capacity chain (the ceilings module's declared-models
                    // section); the
                    // plateau-comb crosses stay flat-judged and green.
                    return Some(if matches!(f.kind, FamilyId::CombScatter) {
                        cell.with_capacity_model()
                    } else {
                        cell
                    });
                }
                // A cross shape without output domination materializes its
                // event side through its id side, input-denominated (the
                // `cell` module doc's do-not-re-denominate list).
                if let Some((v, p, n)) = f.cross() {
                    return Some(Cell::new(
                        n,
                        walk_floors(n, na(NA_TOUCH_PROJECTION)),
                        move || ((&v / &p).to_version(), v, p),
                    ));
                }
                // Small (half-interval) party × adversarial version.
                if f.version.is_some() {
                    let (v, n) = f.version()?;
                    let half = Party::seed().fork();
                    return Some(Cell::new(
                        n + 1,
                        walk_floors(n, na(NA_TOUCH_PROJECTION)),
                        move || ((&v / &half).to_version(), v, half),
                    ));
                }
                // Adversarial party × small version.
                let (a, _, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(a, _)| a.len())?;
                let mut v = Version::new();
                v.tick(&a);
                let input = n + v.encode().len();
                Some(Cell::new(
                    input,
                    walk_floors(input, na(NA_TOUCH_PROJECTION)),
                    move || ((&v / &a).to_version(), v, a),
                ))
            },
        },
        Op {
            name: "own_version_cmp",
            group: OpGroup::Projection,
            prepare: |f| {
                // The fused three-stream comparison `(v / p) ⋚ w`: lazy at
                // every spelling, so the cell stays input-denominated on
                // every shape — the output-domination crosses included,
                // which is the point: comparing a projection never pays
                // its materialization.
                let (v, p, w, n) = if let Some((v, p, np)) = f.cross() {
                    let (_, w, _) = f.version_pair()?;
                    let nw = f.version2.as_ref()?.len();
                    (v, p, w, np + nw)
                } else if f.version.is_some() {
                    // Half-interval party × the shape's version pair.
                    let (v, w, n) = f.version_pair()?;
                    (v, Party::seed().fork(), w, n + 1)
                } else {
                    // Adversarial party × small versions ticked on it.
                    let (a, _, _) = f.party_pair()?;
                    let np = f.parties.as_ref().map(|(a, _)| a.len())?;
                    let mut v = Version::new();
                    v.tick(&a);
                    let mut w = v.clone();
                    w.tick(&Party::seed());
                    let n = np + v.encode().len() + w.encode().len();
                    (v, a, w, n)
                };
                let floors = masked_cmp_floors(&(&v / &p).partial_cmp(&w), &v, &w, n);
                Some(Cell::new(n, floors, move || {
                    let ord = (&v / &p).partial_cmp(&w);
                    (ord, v, p, w)
                }))
            },
        },
        Op {
            name: "own_version_pair_cmp",
            group: OpGroup::Projection,
            prepare: |f| {
                // The fused four-stream comparison `(v/a) ⋚ (w/b)`:
                // input-denominated everywhere, as the three-stream row.
                let (v, a, w, b, n) = match (f.parties.is_some(), f.version.is_some()) {
                    (true, true) => {
                        let (a, b, np) = f.party_pair()?;
                        let (v, w, nv) = f.version_pair()?;
                        (v, a, w, b, np + nv)
                    }
                    (false, true) => {
                        // Seed fork halves around the shape's version pair.
                        let (v, w, nv) = f.version_pair()?;
                        let mut a = Party::seed();
                        let b = a.fork();
                        (v, a, w, b, nv + 2)
                    }
                    (true, false) => {
                        // The party pair's own single-tick histories.
                        let (a, b, np) = f.party_pair()?;
                        let mut v = Version::new();
                        v.tick(&a);
                        let mut w = Version::new();
                        w.tick(&b);
                        let n = np + v.encode().len() + w.encode().len();
                        (v, a, w, b, n)
                    }
                    (false, false) => return None,
                };
                let floors = masked_cmp_floors(&(&v / &a).partial_cmp(&(&w / &b)), &v, &w, n);
                Some(Cell::new(n, floors, move || {
                    let ord = (&v / &a).partial_cmp(&(&w / &b));
                    (ord, v, a, w, b)
                }))
            },
        },
        Op {
            name: "version_display",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, n) = f.version()?;
                let spec = TextSpec {
                    radix_units: radix_units_version(&v),
                    spelled_values: stored_bases(&v).len() as u64,
                    output_is_text: true,
                };
                let floors = Floors {
                    heap: heap_materializes(n),
                    limb: na(NA_LIMB_DEPENDENCY),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: na(NA_TOUCH_RENDER_SUMMARIES),
                };
                let cell = Cell::text(
                    n,
                    floors,
                    |r| {
                        r.downcast_ref::<(String, Version)>()
                            .expect("the display body yields (text, v)")
                            .0
                            .len()
                    },
                    spec,
                    move || (v.to_string(), v),
                );
                // The mirror-wide cross realizes the render's documented
                // superlinear summary-merge class on the limb column, so
                // that cell is judged under the ratified family-stated
                // limb model (the constants' derivations).
                Some(if matches!(f.kind, FamilyId::MirrorWide) {
                    cell.with_declared_limb(
                        MIRROR_WIDE_RENDER_LIMB_EXPONENT_CEILING,
                        MIRROR_WIDE_RENDER_LIMB_OPS_PER_RADIX_UNIT,
                    )
                } else {
                    cell
                })
            },
        },
        Op {
            name: "version_from_str",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, _) = f.version()?;
                let s = v.to_string();
                let spec = TextSpec {
                    radix_units: radix_units_version(&v),
                    spelled_values: stored_bases(&v).len() as u64,
                    output_is_text: false,
                };
                assert_honest_text("version_from_str input", s.len(), spec.radix_units);
                let packed = version_output_bytes(&v);
                let floors = Floors {
                    heap: heap_materializes(packed),
                    limb: limb_wide(mandatory_limbs_version(&v)),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(packed),
                    touch: touch_delta_fold(stored_nonzero_deltas(&v)),
                };
                Some(Cell::text(
                    s.len(),
                    floors,
                    |r| {
                        version_output_bytes(
                            r.downcast_ref::<Version>()
                                .expect("the parse body yields a version"),
                        )
                    },
                    spec,
                    move || {
                        s.parse::<Version>()
                            .expect("a displayed version parses back")
                    },
                ))
            },
        },
        Op {
            name: "version_hash",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, n) = f.version()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: na(NA_LIMB_NOT_FORCED),
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_BYTE_COPY),
                    touch: na(NA_TOUCH_NOT_FORCED),
                };
                Some(Cell::new(n, floors, move || {
                    let mut hasher = DefaultHasher::new();
                    v.hash(&mut hasher);
                    (hasher.finish(), v)
                }))
            },
        },
        Op {
            name: "causally_contains",
            group: OpGroup::Version,
            prepare: |f| {
                // Membership is one-directional, so the floors fork on
                // the verdict, not on comparability (the constructor's
                // doc carries the derivation).
                let (v, w, n) = f.version_pair()?;
                let floors = membership_floors(&v, &w, n);
                Some(Cell::new(n, floors, move || {
                    let hit = causally::since(&v).contains(&w);
                    (hit, v, w)
                }))
            },
        },
        // The placement rows: the arity-3 fused walk (one probe against
        // two bounds) on the pair's own hull, probed by a buffer-distinct
        // re-decode of one input. Clone identity between the probe and an
        // endpoint is impossible while byte equality holds, so the
        // coincidence rung cannot collapse the walk; and the probe always
        // lies within its own pair's hull, so the verdict is confirming
        // — full examination of all three streams is forced, and the
        // full-examination scan floor is honest on every family. The
        // precedence row's probe re-decodes the meet instead: its watched
        // directions are the mirror pair (probe at-or-below each bound),
        // so only a probe at the span's own floor confirms both and
        // forces the full sweep — an in-hull probe above the meet refutes
        // `probe <= lo` and legitimately stops the start stream's scan.
        Op {
            name: "span_place",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, w, _) = f.version_pair()?;
                let span = v.span(&w);
                let probe = decode_version(&v.encode());
                let n = span.lo().encode().len() + span.hi().encode().len() + probe.encode().len();
                Some(Cell::new(
                    n,
                    walk_floors(n, na(NA_TOUCH_PLACEMENT)),
                    move || {
                        let verdict = span.place(&probe);
                        (verdict, span, probe)
                    },
                ))
            },
        },
        Op {
            name: "span_dominance",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, w, _) = f.version_pair()?;
                let span = v.span(&w);
                let probe = decode_version(&v.encode());
                let n = span.lo().encode().len() + span.hi().encode().len() + probe.encode().len();
                Some(Cell::new(
                    n,
                    walk_floors(n, na(NA_TOUCH_PLACEMENT)),
                    move || {
                        let verdict = span.dominance(&probe);
                        (verdict, span, probe)
                    },
                ))
            },
        },
        Op {
            name: "span_precedence",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, w, _) = f.version_pair()?;
                let span = v.span(&w);
                let probe = decode_version(&span.lo().encode());
                let n = span.lo().encode().len() + span.hi().encode().len() + probe.encode().len();
                Some(Cell::new(
                    n,
                    walk_floors(n, na(NA_TOUCH_PLACEMENT)),
                    move || {
                        let verdict = span.precedence(&probe);
                        (verdict, span, probe)
                    },
                ))
            },
        },
        Op {
            name: "span_contains",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, w, _) = f.version_pair()?;
                let span = v.span(&w);
                let probe = decode_version(&v.encode());
                let n = span.lo().encode().len() + span.hi().encode().len() + probe.encode().len();
                Some(Cell::new(
                    n,
                    walk_floors(n, na(NA_TOUCH_PLACEMENT)),
                    move || {
                        let verdict = span.contains(&probe);
                        (verdict, span, probe)
                    },
                ))
            },
        },
        Op {
            name: "query_contains",
            group: OpGroup::Version,
            prepare: |f| {
                // The two-bounded segment query over the same operands.
                // The query borrows its bounds, so the measured body
                // builds it in place: the cross-side conjunction
                // performs no comparison, so the cell prices exactly
                // the fused two-bound membership walk.
                let (v, w, _) = f.version_pair()?;
                let (lo, hi) = v.span(&w).into_parts();
                let probe = decode_version(&v.encode());
                let n = lo.encode().len() + hi.encode().len() + probe.encode().len();
                Some(Cell::new(
                    n,
                    walk_floors(n, na(NA_TOUCH_PLACEMENT)),
                    move || {
                        let verdict = {
                            let query = causally::after(&lo) & causally::before(&hi);
                            query.contains(&probe)
                        };
                        (verdict, lo, hi, probe)
                    },
                ))
            },
        },
        Op {
            name: "query_coverage",
            group: OpGroup::Version,
            prepare: |f| {
                // The anti-entropy delta classified against the pair's
                // hull: the fused two-probe walk over hole and ceiling,
                // plus the clamp legs on verdicts the walk alone cannot
                // close — the tree walk's per-subtree verdict.
                let (v, w, _) = f.version_pair()?;
                let lo = &v & &w;
                let hi = &v | &w;
                let span = lo.span(&hi);
                let n = v.encode().len()
                    + w.encode().len()
                    + span.lo().encode().len()
                    + span.hi().encode().len();
                Some(Cell::new(
                    n,
                    walk_floors(n, na(NA_TOUCH_PLACEMENT)),
                    move || {
                        let verdict = {
                            let query = causally::delta(&v, &w);
                            query.coverage(span.reborrow())
                        };
                        (verdict, span, v, w)
                    },
                ))
            },
        },
        // ── Party ──────────────────────────────────────────────────────
        Op {
            name: "party_decode",
            group: OpGroup::Party,
            prepare: |f| {
                let (a, b) = f.parties.clone()?;
                let n = a.len() + b.len();
                let floors = Floors {
                    heap: heap_materializes(n),
                    limb: na(NA_LIMB_ID_TREE),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: na(NA_TOUCH_ID_TREE),
                };
                Some(Cell::new(n, floors, move || {
                    (decode_party(&a), decode_party(&b))
                }))
            },
        },
        Op {
            name: "party_encode",
            group: OpGroup::Party,
            prepare: |f| {
                let (a, _, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(a, _)| a.len())?;
                let floors = Floors {
                    heap: heap_materializes(n),
                    limb: na(NA_LIMB_ID_TREE),
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_BYTE_COPY),
                    touch: na(NA_TOUCH_ID_TREE),
                };
                Some(Cell::new(n, floors, move || (a.encode(), a)))
            },
        },
        Op {
            name: "party_fork",
            group: OpGroup::Party,
            prepare: |f| {
                let (mut a, _, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(a, _)| a.len())?;
                // Fork builds both halves, so the child's own packed bytes
                // floor the heap (probed on a fresh decode, outside
                // measurement); the generic in-place NA would misstate
                // what fork does.
                let child_bytes = {
                    let bytes = f.parties.as_ref().map(|(a, _)| a.clone())?;
                    let mut probe = decode_party(&bytes);
                    probe.fork().encoded_bits() / 8
                };
                let floors = Floors {
                    heap: if child_bytes == 0 {
                        na(NA_HEAP_IN_PLACE)
                    } else {
                        Liveness::Floor {
                            min: child_bytes,
                            why: WHY_HEAP_FORK_HALF,
                        }
                    },
                    limb: na(NA_LIMB_ID_TREE),
                    segments: seg_ceiling_only(),
                    scan: if a.is_seed() {
                        na(NA_SCAN_SEED_PARTY)
                    } else {
                        scan_touch()
                    },
                    touch: na(NA_TOUCH_ID_TREE),
                };
                Some(Cell::new(n, floors, move || {
                    let child = a.fork();
                    (a, child)
                }))
            },
        },
        Op {
            name: "party_join",
            group: OpGroup::Party,
            prepare: |f| {
                let (mut a, b, n) = f.party_pair()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: na(NA_LIMB_ID_TREE),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: na(NA_TOUCH_ID_TREE),
                };
                Some(Cell::new(n, floors, move || {
                    let joined = a.join(b).is_ok();
                    (joined, a)
                }))
            },
        },
        Op {
            name: "party_join_all",
            group: OpGroup::Fold,
            prepare: |f| {
                let (_, parties) = f.fold.as_ref()?;
                let n = parties.iter().map(Vec::len).sum();
                let arity = parties.len() as u64;
                let mut parties = parties.iter().map(|b| decode_party(b));
                let acc = parties.next().expect("the scatter population is nonempty");
                let rest: Vec<Party> = parties.collect();
                // The declared search allowance: the accumulator's table
                // size prices each tested input's both-present nodes
                // (INDEX_PROBE_SCAN_BITS carries the derivation).
                let table = both_present_nodes(&acc);
                let probes_per_node = u64::from((table + 1).next_power_of_two().trailing_zeros());
                let search_bits = INDEX_PROBE_SCAN_BITS
                    * probes_per_node
                    * rest.iter().map(both_present_nodes).sum::<u64>();
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: na(NA_LIMB_ID_TREE),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: na(NA_TOUCH_ID_TREE),
                };
                Some(
                    Cell::new(n, floors, move || {
                        let mut acc = acc;
                        acc.join_all(rest)
                            .expect("fold operands are forked parties, pairwise disjoint");
                        acc
                    })
                    .with_fold_arity(arity)
                    .with_fold_search(search_bits),
                )
            },
        },
        Op {
            name: "party_covers",
            group: OpGroup::Party,
            prepare: |f| {
                let (a, b, n) = f.party_pair()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: na(NA_LIMB_ID_TREE),
                    segments: seg_ceiling_only(),
                    scan: scan_touch(),
                    touch: na(NA_TOUCH_ID_TREE),
                };
                Some(Cell::new(n, floors, move || (a.covers(&b), a, b)))
            },
        },
        Op {
            name: "party_disjoint",
            group: OpGroup::Party,
            prepare: |f| {
                let (a, b, n) = f.party_pair()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: na(NA_LIMB_ID_TREE),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: na(NA_TOUCH_ID_TREE),
                };
                Some(Cell::new(n, floors, move || (a.is_disjoint(&b), a, b)))
            },
        },
        Op {
            name: "party_without",
            group: OpGroup::Party,
            prepare: |f| {
                let (_, b, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(_, b)| b.len())?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: na(NA_LIMB_ID_TREE),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: na(NA_TOUCH_ID_TREE),
                };
                Some(Cell::new(n + 1, floors, move || {
                    (Party::seed().without(&b), b)
                }))
            },
        },
        Op {
            name: "party_display",
            group: OpGroup::Party,
            prepare: |f| {
                let (a, _, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(a, _)| a.len())?;
                let spec = TextSpec {
                    radix_units: radix_units_party(&a),
                    spelled_values: 0,
                    output_is_text: true,
                };
                let floors = Floors {
                    heap: heap_materializes(n),
                    limb: na(NA_LIMB_ID_TREE),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: na(NA_TOUCH_ID_TREE),
                };
                Some(Cell::text(
                    n,
                    floors,
                    |r| {
                        r.downcast_ref::<(String, Party)>()
                            .expect("the display body yields (text, party)")
                            .0
                            .len()
                    },
                    spec,
                    move || (a.to_string(), a),
                ))
            },
        },
        Op {
            name: "party_from_str",
            group: OpGroup::Party,
            prepare: |f| {
                let (a, _, _) = f.party_pair()?;
                let s = a.to_string();
                let spec = TextSpec {
                    radix_units: radix_units_party(&a),
                    spelled_values: 0,
                    output_is_text: false,
                };
                assert_honest_text("party_from_str input", s.len(), spec.radix_units);
                // The operand's stored buffer is allocated on this host, so
                // its byte count fits `usize`.
                let packed = usize::try_from(a.encoded_bits().div_ceil(8))
                    .expect("an allocated buffer's byte count");
                let floors = Floors {
                    heap: heap_materializes(packed),
                    limb: na(NA_LIMB_ID_TREE),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(packed),
                    touch: na(NA_TOUCH_ID_TREE),
                };
                Some(Cell::text(
                    s.len(),
                    floors,
                    |r| {
                        let bits = r
                            .downcast_ref::<Party>()
                            .expect("the parse body yields a party")
                            .encoded_bits();
                        // An output the body materialized: its byte count
                        // fits this host's `usize`.
                        usize::try_from(bits.div_ceil(8)).expect("an allocated buffer's byte count")
                    },
                    spec,
                    move || s.parse::<Party>().expect("a displayed party parses back"),
                ))
            },
        },
        Op {
            name: "party_hash",
            group: OpGroup::Party,
            prepare: |f| {
                let (a, _, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(a, _)| a.len())?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: na(NA_LIMB_ID_TREE),
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_BYTE_COPY),
                    touch: na(NA_TOUCH_ID_TREE),
                };
                Some(Cell::new(n, floors, move || {
                    let mut hasher = DefaultHasher::new();
                    a.hash(&mut hasher);
                    (hasher.finish(), a)
                }))
            },
        },
        // ── Clock ──────────────────────────────────────────────────────
        Op {
            name: "clock_decode",
            group: OpGroup::Clock,
            prepare: |f| {
                let (clock, _) = f.clock()?;
                let bytes = clock.encode();
                let floors = Floors {
                    heap: heap_materializes(bytes.len()),
                    limb: limb_stream(mandatory_limbs_stream(clock.version())),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(bytes.len()),
                    touch: touch_wide_stream(clock.version()),
                };
                Some(Cell::new(bytes.len(), floors, move || {
                    Clock::decode(&bytes[..]).expect("an encoded clock decodes back")
                }))
            },
        },
        Op {
            name: "clock_encode",
            group: OpGroup::Clock,
            prepare: |f| {
                let (clock, n) = f.clock()?;
                let floors = Floors {
                    heap: heap_materializes(n),
                    limb: na(NA_LIMB_NOT_FORCED),
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_BYTE_COPY),
                    touch: na(NA_TOUCH_NOT_FORCED),
                };
                Some(Cell::new(n, floors, move || (clock.encode(), clock)))
            },
        },
        Op {
            name: "clock_tick",
            group: OpGroup::Tick,
            prepare: |f| {
                // The tick-walk families tick their own (id, event)
                // clock; they reach no other clock row.
                if let Some((v, p, n)) = f.cross() {
                    let floors = tick_walk_floors(&v, n);
                    let mut clock = Clock::from_parts(p, v);
                    let cell = Cell::new(n, floors, move || {
                        clock.tick();
                        clock
                    });
                    // As version_tick: the ascending cliff's certificate
                    // memory is family-stated.
                    return Some(if matches!(f.kind, FamilyId::AscendCliff) {
                        cell.with_declared_heap(ASCEND_CLIFF_TICK_HEAP_BYTES_PER_INPUT_BYTE)
                    } else {
                        cell
                    });
                }
                let (mut clock, n) = f.clock()?;
                // A version-bearing shape's clock ticks its seed party (an
                // in-place raise); the id pair's clock ticks an empty
                // version (pure growth). Neither runs the accumulator.
                let touch = if clock.version().is_empty() {
                    na(NA_TOUCH_GROW)
                } else {
                    na(NA_TOUCH_SEED_RAISE)
                };
                Some(Cell::new(n, walk_floors(n, touch), move || {
                    clock.tick();
                    clock
                }))
            },
        },
        Op {
            name: "clock_fork",
            group: OpGroup::Clock,
            prepare: |f| {
                let (mut clock, n) = f.clock()?;
                let floors = Floors {
                    heap: na(NA_HEAP_FORK_SHARES),
                    limb: na(NA_LIMB_NOT_FORCED),
                    segments: seg_ceiling_only(),
                    scan: if clock.party().is_seed() {
                        na(NA_SCAN_SEED_PARTY)
                    } else {
                        scan_touch()
                    },
                    touch: na(NA_TOUCH_NOT_FORCED),
                };
                Some(Cell::new(n, floors, move || {
                    let child = clock.fork();
                    (clock, child)
                }))
            },
        },
        Op {
            name: "clock_join",
            group: OpGroup::Clock,
            prepare: |f| {
                let (mut a, b, n) = f.clock_pair()?;
                let touch = touch_pair_fold(a.version(), b.version());
                Some(Cell::new(n, walk_floors(n, touch), move || {
                    let joined = a.join(b).is_ok();
                    (joined, a)
                }))
            },
        },
        Op {
            name: "clock_sync",
            group: OpGroup::Clock,
            prepare: |f| {
                let (mut a, mut b, n) = f.clock_pair()?;
                // Not the shared full-examination premise: the fused
                // sync's party leg splices subtrees owned by one side
                // alone without reading them, so its floors derive from
                // the version join alone (`sync_floors`).
                let floors = sync_floors(a.version(), b.version());
                Some(Cell::new(n, floors, move || {
                    let synced = a.sync(&mut b).is_ok();
                    (synced, a, b)
                }))
            },
        },
        Op {
            name: "clock_recv",
            group: OpGroup::Clock,
            prepare: |f| {
                // Small clock × adversarial received version.
                if let Some((v, n)) = f.version() {
                    let mut clock = Clock::seed();
                    let touch = touch_delta_fold(stored_nonzero_deltas(&v));
                    return Some(Cell::new(n + 2, walk_floors(n, touch), move || {
                        clock.recv(&v);
                        (clock, v)
                    }));
                }
                // Adversarial party × small received version.
                let (a, _, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(a, _)| a.len())?;
                let mut clock = Clock::from_parts(a, Version::new());
                let msg = Version::try_from(1u64).expect("a one-tick version is valid");
                let touch = touch_delta_fold(stored_nonzero_deltas(&msg));
                Some(Cell::new(n + 2, walk_floors(n, touch), move || {
                    clock.recv(&msg);
                    (clock, msg)
                }))
            },
        },
        Op {
            name: "clock_own_version_to_version",
            group: OpGroup::Projection,
            prepare: |f| {
                // The clock spelling of the explicit materialization:
                // `clock.own_version()` is an O(1) view (no cell of its
                // own — nothing scales), and this row prices its
                // `.to_version()`. Adversarial × adversarial with
                // mandatory dominating output: a clock holding the cross's
                // event side whose party is its id side, I/O-denominated
                // (the `cell` module doc's output-domination cross).
                if f.output_dominated {
                    let (v_bytes, p_bytes) = f.cross.as_ref()?;
                    let n = v_bytes.len() + p_bytes.len();
                    let clock = Clock::from_parts(decode_party(p_bytes), decode_version(v_bytes));
                    let cell = Cell::io(
                        n,
                        walk_floors(n, na(NA_TOUCH_PROJECTION)),
                        |r| {
                            let (out, _) = r
                                .downcast_ref::<(Version, Clock)>()
                                .expect("the own_version body yields (out, clock)");
                            version_output_bytes(out)
                        },
                        move || (clock.own_version().to_version(), clock),
                    );
                    // The same ratified capacity chain as the version
                    // spelling of this materialization.
                    return Some(if matches!(f.kind, FamilyId::CombScatter) {
                        cell.with_capacity_model()
                    } else {
                        cell
                    });
                }
                let (clock, n) = f.clock()?;
                // The whole-interval party's projection is the version
                // itself, handed back as a buffer-sharing clone, so no
                // stream walk is forced for a seed clock; any other
                // party runs the projection walk whole.
                let scan = if clock.party().is_seed() {
                    na(NA_SCAN_SEED_PROJECTION)
                } else {
                    scan_examines(n)
                };
                Some(Cell::new(
                    n,
                    Floors {
                        heap: na(NA_HEAP_IN_PLACE),
                        limb: na(NA_LIMB_NOT_FORCED),
                        segments: seg_ceiling_only(),
                        scan,
                        touch: na(NA_TOUCH_PROJECTION),
                    },
                    move || (clock.own_version().to_version(), clock),
                ))
            },
        },
        Op {
            name: "clock_display",
            group: OpGroup::Clock,
            prepare: |f| {
                let (clock, n) = f.clock()?;
                let spec = TextSpec {
                    radix_units: radix_units_clock(&clock),
                    spelled_values: stored_bases(clock.version()).len() as u64,
                    output_is_text: true,
                };
                let floors = Floors {
                    heap: heap_materializes(n),
                    limb: na(NA_LIMB_DEPENDENCY),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: na(NA_TOUCH_RENDER_SUMMARIES),
                };
                let cell = Cell::text(
                    n,
                    floors,
                    |r| {
                        r.downcast_ref::<(String, Clock)>()
                            .expect("the display body yields (text, clock)")
                            .0
                            .len()
                    },
                    spec,
                    move || (clock.to_string(), clock),
                );
                // As version_display: the mirror-wide render-merge cell
                // is judged under the family-stated limb model.
                Some(if matches!(f.kind, FamilyId::MirrorWide) {
                    cell.with_declared_limb(
                        MIRROR_WIDE_RENDER_LIMB_EXPONENT_CEILING,
                        MIRROR_WIDE_RENDER_LIMB_OPS_PER_RADIX_UNIT,
                    )
                } else {
                    cell
                })
            },
        },
        Op {
            name: "clock_from_str",
            group: OpGroup::Clock,
            prepare: |f| {
                let (clock, _) = f.clock()?;
                let s = clock.to_string();
                let spec = TextSpec {
                    radix_units: radix_units_clock(&clock),
                    spelled_values: stored_bases(clock.version()).len() as u64,
                    output_is_text: false,
                };
                assert_honest_text("clock_from_str input", s.len(), spec.radix_units);
                // The operand's stored buffers are allocated on this host,
                // so their byte count fits `usize`.
                let packed = usize::try_from(clock.encoded_bits().div_ceil(8))
                    .expect("an allocated buffer's byte count");
                let floors = Floors {
                    heap: heap_materializes(packed),
                    limb: limb_wide(mandatory_limbs_version(clock.version())),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(packed),
                    touch: touch_delta_fold(stored_nonzero_deltas(clock.version())),
                };
                Some(Cell::text(
                    s.len(),
                    floors,
                    |r| {
                        let bits = r
                            .downcast_ref::<Clock>()
                            .expect("the parse body yields a clock")
                            .encoded_bits();
                        // An output the body materialized: its byte count
                        // fits this host's `usize`.
                        usize::try_from(bits.div_ceil(8)).expect("an allocated buffer's byte count")
                    },
                    spec,
                    move || s.parse::<Clock>().expect("a displayed clock parses back"),
                ))
            },
        },
        Op {
            name: "clock_hash",
            group: OpGroup::Clock,
            prepare: |f| {
                let (clock, n) = f.clock()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: na(NA_LIMB_NOT_FORCED),
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_BYTE_COPY),
                    touch: na(NA_TOUCH_NOT_FORCED),
                };
                Some(Cell::new(n, floors, move || {
                    let mut hasher = DefaultHasher::new();
                    clock.hash(&mut hasher);
                    (hasher.finish(), clock)
                }))
            },
        },
        // ── the rejection surface (the `defect` module doc) ────────────
        Op {
            name: "version_decode_truncated",
            group: OpGroup::Version,
            prepare: |f| {
                let bytes = f.version.clone()?;
                let fed = truncated_bytes(&bytes);
                let n = fed.len();
                let floors = rejection_floors(n, WHY_SCAN_REJECT_END);
                Some(Cell::new(n, floors, move || {
                    let err =
                        Version::decode(&fed[..]).expect_err("a truncated stream is rejected");
                    assert!(
                        matches!(err, Decode::Truncated),
                        "the placed defect is the cut, not {err:?}"
                    );
                    (err, fed)
                }))
            },
        },
        Op {
            name: "version_decode_trailing",
            group: OpGroup::Version,
            prepare: |f| {
                let bytes = f.version.clone()?;
                let fed = trailing_bytes(&bytes);
                let n = fed.len();
                let floors = rejection_floors(n, WHY_SCAN_REJECT_END);
                Some(Cell::new(n, floors, move || {
                    let err =
                        Version::decode(&fed[..]).expect_err("a trailing-bits stream is rejected");
                    assert!(
                        matches!(err, Decode::TrailingBits),
                        "the placed defect is the appended tail, not {err:?}"
                    );
                    (err, fed)
                }))
            },
        },
        Op {
            name: "version_decode_noncanon",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, _) = f.version()?;
                let fed = version_noncanonical_bytes(&v);
                let n = fed.len();
                let floors = rejection_floors(n, WHY_SCAN_REJECT_END);
                Some(Cell::new(n, floors, move || {
                    let err =
                        Version::decode(&fed[..]).expect_err("a non-canonical tail is rejected");
                    assert!(
                        matches!(err, Decode::NotCanonical),
                        "the placed defect is the equal-sibling tail, not {err:?}"
                    );
                    (err, fed)
                }))
            },
        },
        Op {
            name: "version_parse_trailing",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, _) = f.version()?;
                let fed = trailing_text(&v.to_string());
                let n = fed.len();
                let floors = text_rejection_floors(na(NA_LIMB_REJECTION), na(NA_TOUCH_REJECTION));
                Some(Cell::new(n, floors, move || {
                    let err = fed
                        .parse::<Version>()
                        .expect_err("trailing junk after valid text is rejected");
                    assert!(
                        matches!(err, Parse::Syntax),
                        "the placed defect is the trailing junk, not {err:?}"
                    );
                    (err, fed)
                }))
            },
        },
        Op {
            name: "version_parse_noncanon",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, _) = f.version()?;
                let fed = version_noncanonical_text(&v.to_string());
                let n = fed.len();
                let floors = text_rejection_floors(na(NA_LIMB_REJECTION), na(NA_TOUCH_REJECTION));
                Some(Cell::new(n, floors, move || {
                    let err = fed
                        .parse::<Version>()
                        .expect_err("a non-canonical tail is rejected");
                    assert!(
                        matches!(err, Parse::NotCanonical),
                        "the placed defect is the equal-sibling tail, not {err:?}"
                    );
                    (err, fed)
                }))
            },
        },
        Op {
            name: "span_decode_truncated",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, w, _) = f.version_pair()?;
                let bytes = v.span(&w).encode();
                let fed = truncated_bytes(&bytes);
                let n = fed.len();
                let floors = rejection_floors(n, WHY_SCAN_REJECT_END);
                Some(Cell::new(n, floors, move || {
                    let err =
                        Span::decode(&fed[..]).expect_err("a truncated composite is rejected");
                    assert!(
                        matches!(err, Decode::Truncated),
                        "the placed defect is the cut, not {err:?}"
                    );
                    (err, fed)
                }))
            },
        },
        Op {
            name: "span_decode_trailing",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, w, _) = f.version_pair()?;
                let bytes = v.span(&w).encode();
                let fed = trailing_bytes(&bytes);
                let n = fed.len();
                let floors = rejection_floors(n, WHY_SCAN_REJECT_END);
                Some(Cell::new(n, floors, move || {
                    let err =
                        Span::decode(&fed[..]).expect_err("a trailing-bits composite is rejected");
                    assert!(
                        matches!(err, Decode::TrailingBits),
                        "the placed defect is the appended tail, not {err:?}"
                    );
                    (err, fed)
                }))
            },
        },
        Op {
            name: "span_decode_crossed",
            group: OpGroup::Version,
            prepare: |f| {
                // The genre the span decode mints: the reversed
                // composite — join first — is well-formed
                // component-wise but the canonical spelling of no
                // span. The hull of a distinct pair is strictly
                // ordered, so the reversal is genuinely crossed.
                let (v, w, _) = f.version_pair()?;
                let (lo, hi) = v.span(&w).into_parts();
                assert_ne!(lo, hi, "a crossed witness needs a strictly ordered hull");
                let fed = [hi.encode(), lo.encode()].concat();
                let n = fed.len();
                let floors = rejection_floors(n, WHY_SCAN_REJECT_CROSSED);
                Some(Cell::new(n, floors, move || {
                    let err = Span::decode(&fed[..]).expect_err("a crossed composite is rejected");
                    assert!(
                        matches!(err, Decode::NotCanonical),
                        "the placed defect is the reversal, not {err:?}"
                    );
                    (err, fed)
                }))
            },
        },
        Op {
            name: "party_decode_truncated",
            group: OpGroup::Party,
            prepare: |f| {
                let bytes = f.parties.as_ref().map(|(a, _)| a.clone())?;
                let fed = truncated_bytes(&bytes);
                let n = fed.len();
                let floors = id_rejection_floors(n, WHY_SCAN_REJECT_END);
                Some(Cell::new(n, floors, move || {
                    let err = Party::decode(&fed[..]).expect_err("a truncated stream is rejected");
                    assert!(
                        matches!(err, Decode::Truncated),
                        "the placed defect is the cut, not {err:?}"
                    );
                    (err, fed)
                }))
            },
        },
        Op {
            name: "party_decode_trailing",
            group: OpGroup::Party,
            prepare: |f| {
                let bytes = f.parties.as_ref().map(|(a, _)| a.clone())?;
                let fed = trailing_bytes(&bytes);
                let n = fed.len();
                let floors = id_rejection_floors(n, WHY_SCAN_REJECT_END);
                Some(Cell::new(n, floors, move || {
                    let err =
                        Party::decode(&fed[..]).expect_err("a trailing-bits stream is rejected");
                    assert!(
                        matches!(err, Decode::TrailingBits),
                        "the placed defect is the appended tail, not {err:?}"
                    );
                    (err, fed)
                }))
            },
        },
        Op {
            name: "party_decode_noncanon",
            group: OpGroup::Party,
            prepare: |f| {
                let (a, _, _) = f.party_pair()?;
                let fed = party_noncanonical_bytes(&a);
                let n = fed.len();
                let floors = id_rejection_floors(n, WHY_SCAN_REJECT_END);
                Some(Cell::new(n, floors, move || {
                    let err =
                        Party::decode(&fed[..]).expect_err("a non-canonical tail is rejected");
                    assert!(
                        matches!(err, Decode::NotCanonical),
                        "the placed defect is the collapsible (1, 1) tail, not {err:?}"
                    );
                    (err, fed)
                }))
            },
        },
        Op {
            name: "party_parse_trailing",
            group: OpGroup::Party,
            prepare: |f| {
                let (a, _, _) = f.party_pair()?;
                let fed = trailing_text(&a.to_string());
                let n = fed.len();
                let floors = text_rejection_floors(na(NA_LIMB_ID_TREE), na(NA_TOUCH_ID_TREE));
                Some(Cell::new(n, floors, move || {
                    let err = fed
                        .parse::<Party>()
                        .expect_err("trailing junk after valid text is rejected");
                    assert!(
                        matches!(err, Parse::Syntax),
                        "the placed defect is the trailing junk, not {err:?}"
                    );
                    (err, fed)
                }))
            },
        },
        Op {
            name: "party_parse_noncanon",
            group: OpGroup::Party,
            prepare: |f| {
                let (a, _, _) = f.party_pair()?;
                let fed = party_noncanonical_text(&a.to_string());
                let n = fed.len();
                let floors = text_rejection_floors(na(NA_LIMB_ID_TREE), na(NA_TOUCH_ID_TREE));
                Some(Cell::new(n, floors, move || {
                    let err = fed
                        .parse::<Party>()
                        .expect_err("a non-canonical tail is rejected");
                    assert!(
                        matches!(err, Parse::NotCanonical),
                        "the placed defect is the collapsible (1, 1) tail, not {err:?}"
                    );
                    (err, fed)
                }))
            },
        },
        Op {
            name: "clock_decode_truncated",
            group: OpGroup::Clock,
            prepare: |f| {
                let (clock, _) = f.clock()?;
                let fed = truncated_bytes(&clock.encode());
                let n = fed.len();
                let floors = rejection_floors(n, WHY_SCAN_REJECT_END);
                Some(Cell::new(n, floors, move || {
                    let err = Clock::decode(&fed[..]).expect_err("a truncated stream is rejected");
                    assert!(
                        matches!(err, Decode::Truncated),
                        "the placed defect is the cut, not {err:?}"
                    );
                    (err, fed)
                }))
            },
        },
        Op {
            name: "clock_decode_trailing",
            group: OpGroup::Clock,
            prepare: |f| {
                let (clock, _) = f.clock()?;
                let fed = trailing_bytes(&clock.encode());
                let n = fed.len();
                let floors = rejection_floors(n, WHY_SCAN_REJECT_END);
                Some(Cell::new(n, floors, move || {
                    let err =
                        Clock::decode(&fed[..]).expect_err("a trailing-bits stream is rejected");
                    assert!(
                        matches!(err, Decode::TrailingBits),
                        "the placed defect is the appended tail, not {err:?}"
                    );
                    (err, fed)
                }))
            },
        },
        Op {
            name: "clock_parse_trailing",
            group: OpGroup::Clock,
            prepare: |f| {
                let (clock, _) = f.clock()?;
                let fed = clock_trailing_text(&clock.to_string());
                let n = fed.len();
                let floors = text_rejection_floors(na(NA_LIMB_REJECTION), na(NA_TOUCH_REJECTION));
                Some(Cell::new(n, floors, move || {
                    let err = fed
                        .parse::<Clock>()
                        .expect_err("junk inside the stamp's parens is rejected");
                    assert!(
                        matches!(err, Parse::Syntax),
                        "the placed defect is the inner junk, not {err:?}"
                    );
                    (err, fed)
                }))
            },
        },
        Op {
            name: "party_join_overlap",
            group: OpGroup::Party,
            prepare: |f| {
                let (a_bytes, b_bytes) = f.overlap.clone()?;
                let n = a_bytes.len() + b_bytes.len();
                let mut a = decode_party(&a_bytes);
                let b = decode_party(&b_bytes);
                let floors = id_rejection_floors(n, WHY_SCAN_OVERLAP_END);
                Some(Cell::new(n, floors, move || {
                    let back = a
                        .join(b)
                        .expect_err("the overlap-mounted pair must be rejected");
                    (back, a)
                }))
            },
        },
        Op {
            name: "clock_join_overlap",
            group: OpGroup::Clock,
            prepare: |f| {
                let (a_bytes, b_bytes) = f.overlap.clone()?;
                let id_bytes = a_bytes.len() + b_bytes.len();
                // Versions ride along where the bundle has them (empty
                // otherwise); rejection does no version work — the party
                // join is the gate — so the scan floor covers the ids.
                let (v, w, nv) = match f.version_pair() {
                    Some(pair) => pair,
                    None => (Version::new(), Version::new(), 2),
                };
                let n = id_bytes + nv;
                let mut a = Clock::from_parts(decode_party(&a_bytes), v);
                let b = Clock::from_parts(decode_party(&b_bytes), w);
                Some(Cell::new(n, clock_overlap_floors(id_bytes), move || {
                    let back = a
                        .join(b)
                        .expect_err("the overlap-mounted pair must be rejected");
                    (back, a)
                }))
            },
        },
        Op {
            name: "clock_sync_overlap",
            group: OpGroup::Clock,
            prepare: |f| {
                let (a_bytes, b_bytes) = f.overlap.clone()?;
                let id_bytes = a_bytes.len() + b_bytes.len();
                let (v, w, nv) = match f.version_pair() {
                    Some(pair) => pair,
                    None => (Version::new(), Version::new(), 2),
                };
                let n = id_bytes + nv;
                let mut a = Clock::from_parts(decode_party(&a_bytes), v);
                let mut b = Clock::from_parts(decode_party(&b_bytes), w);
                Some(Cell::new(n, clock_overlap_floors(id_bytes), move || {
                    let err = a
                        .sync(&mut b)
                        .expect_err("the overlap-mounted pair must be rejected");
                    (err, a, b)
                }))
            },
        },
        Op {
            name: "party_join_all_overlap",
            group: OpGroup::Fold,
            prepare: |f| {
                // One large accumulator, many one-byte probes each
                // overlapping its right half behind the whole left shape:
                // every probe is tested against the fixed accumulator and
                // handed back, and the probe count scales with the
                // accumulator (the divisor's rustdoc), so any per-input
                // work scaling with the accumulator reads quadratic here
                // while the indexed test's O(probe) checks read linear.
                let (a_bytes, _) = f.overlap.clone()?;
                let acc = decode_party(&a_bytes);
                let probe = overlap_fold_probe();
                assert!(
                    !acc.is_disjoint(&decode_party(&probe)),
                    "the fold probe overlaps the a-mount's right half"
                );
                let count = (a_bytes.len() / OVERLAP_FOLD_INPUT_DIVISOR).max(MIN_SIZE_PARAM);
                let inputs: Vec<Party> = (0..count).map(|_| decode_party(&probe)).collect();
                let n = a_bytes.len() + count * probe.len();
                let floors = id_rejection_floors(n, WHY_SCAN_EXAMINES);
                Some(Cell::new(n, floors, move || {
                    let mut acc = acc;
                    let back = acc
                        .join_all(inputs)
                        .expect_err("every probe overlaps the accumulator");
                    assert_eq!(back.len(), count, "every probe is handed back");
                    (back, acc)
                }))
            },
        },
        Op {
            name: "party_without_none",
            group: OpGroup::Party,
            prepare: |f| {
                // Identical-region operands: the diff walks both streams in
                // full, and the empty remainder is known only at the end.
                let bytes = f.parties.as_ref().map(|(a, _)| a.clone())?;
                let n = bytes.len() * 2;
                let a = decode_party(&bytes);
                let b = decode_party(&bytes);
                let floors = id_rejection_floors(n, WHY_SCAN_EXAMINES);
                Some(Cell::new(n, floors, move || {
                    let gone = a.without(&b);
                    assert!(gone.is_none(), "removing a covering region leaves nothing");
                    (gone, b)
                }))
            },
        },
    ]
}

/// A rank's numerator width in 64-bit limbs (minimum 1).
///
/// The limb floors of the wire rows, which materialize the numerator
/// whatever the exponent (a spine rank's exponent is wide while its
/// numerator is one word — the exponent costs fraction *bits* on the
/// wire, never limbs).
fn rank_numerator_limbs(rank: &Rank) -> u64 {
    rank.raw_parts().0.bits().div_ceil(64).max(1)
}
