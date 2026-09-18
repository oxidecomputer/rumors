//! The operation axis: the board's row table.
//!
//! Each row declares the bundle slots its signature consumes and prepares its
//! cell from them alone — never from the shape's identity — so a row reaches
//! every shape that supplies its operands.

use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use num_bigint::BigUint;

use crate::error::Decode;
use crate::{causally, Clock, Party, Rank, Ranked, Span, Ticks, Version};

use super::ceilings::{COMB_SCATTER_PROJECTION_HEAP_BYTES_PER_IO_BYTE, TICKS_BOARD_COUNT};
use super::cell::Cell;
use super::currency::{Floors, Liveness};
use super::defect::{
    party_noncanonical_bytes, trailing_bytes, truncated_bytes, version_noncanonical_bytes,
};
use super::family::{decode_party, decode_version, FamilyData};
use super::floors::{
    clock_overlap_floors, comparison_floors, heap_materializes, id_rejection_floors,
    masked_cmp_floors, membership_floors, na, rejection_floors, scan_examines, scan_touch,
    seg_ceiling_only, sync_floors, tick_walk_floors, touch_delta_fold, touch_fold_first_merges,
    touch_pair_fold, touch_wide_stream, walk_floors, NA_HEAP_FORK_SHARES, NA_HEAP_IN_PLACE,
    NA_SCAN_BYTE_COPY, NA_SCAN_EQ_BYTES, NA_SCAN_NO_STREAM, NA_SCAN_RANK_BYTES, NA_SCAN_SEED_PARTY,
    NA_SCAN_SEED_PROJECTION, NA_TOUCH_GROW, NA_TOUCH_ID_TREE, NA_TOUCH_NOT_FORCED,
    NA_TOUCH_PLACEMENT, NA_TOUCH_PROJECTION, NA_TOUCH_RANK_ARITHMETIC, NA_TOUCH_SEED_RAISE,
    WHY_HEAP_FORK_HALF, WHY_SCAN_EXAMINES, WHY_SCAN_OVERLAP_END, WHY_SCAN_REJECT_CROSSED,
    WHY_SCAN_REJECT_END, WHY_TOUCH_RANK_SUM,
};
use super::operand::{stored_nonzero_deltas, version_output_bytes};
use crate::meter::registry::FamilyId;

/// Arity used to judge the consuming array conversions directly.
const SPLIT_ARRAY_ARITY: usize = 16;

/// A fork count whose stored width matches `input_bytes`.
///
/// The iterator need not drain this count. Constructing it exposes whether a
/// compact count can multiply the resident party bytes by its bit width.
fn wide_fork_count(input_bytes: usize) -> (Ticks, usize) {
    let shift = u64::try_from(input_bytes)
        .expect("a resident input byte length fits u64")
        .checked_mul(8)
        .expect("a resident byte buffer cannot exceed one eighth of u64::MAX");
    let count = Ticks(BigUint::from(1u8) << shift);
    let count_bytes = usize::try_from(count.0.bits().div_ceil(8))
        .expect("the constructed count width came from a usize byte length");
    (count, count_bytes)
}

/// One board row: a public operation and how to instantiate it per family.
pub(super) struct Op {
    /// The row label, `type_operation`.
    pub(super) name: &'static str,
    /// Build the cell for one shape, or `None` where the shape's bundle
    /// supplies no operand for this operation's signature.
    pub(super) prepare: fn(&FamilyData) -> Option<Cell>,
}

/// The operation table: every priced method or trait-family row with a
/// meaningful encoded operand. The `coverage` module accounts for the rest.
#[allow(clippy::too_many_lines)]
pub(super) fn ops() -> Vec<Op> {
    vec![
        // ── Version ────────────────────────────────────────────────────
        Op {
            name: "version_decode",
            prepare: |f| {
                let bytes = f.version.clone()?;
                let v = decode_version(&bytes);
                let floors = Floors {
                    heap: heap_materializes(bytes.len()),
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
            prepare: |f| {
                let (v, n) = f.version()?;
                let floors = Floors {
                    heap: heap_materializes(n),
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_BYTE_COPY),
                    touch: na(NA_TOUCH_NOT_FORCED),
                };
                Some(Cell::new(n, floors, move || (v.encode(), v)))
            },
        },
        Op {
            name: "version_cmp",
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
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_EQ_BYTES),
                    touch: na(NA_TOUCH_NOT_FORCED),
                };
                Some(Cell::new(n, floors, move || (v == w, v, w)))
            },
        },
        Op {
            name: "version_concurrent",
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                let floors = comparison_floors(&v, &w, n);
                Some(Cell::new(n, floors, move || (v.concurrent(&w), v, w)))
            },
        },
        Op {
            name: "version_join",
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                let touch = touch_pair_fold(&v, &w);
                Some(Cell::new(n, walk_floors(n, touch), move || (&v | &w, v, w)))
            },
        },
        Op {
            name: "version_join_assign",
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
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                let touch = touch_pair_fold(&v, &w);
                Some(Cell::new(n, walk_floors(n, touch), move || (&v & &w, v, w)))
            },
        },
        Op {
            name: "version_meet_assign",
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
            prepare: |f| {
                // The composite emission over the pair's hull (built at
                // prepare, outside measurement): one byte copy per
                // endpoint — the codec emission genre, denominated by
                // the span's own encoded size, which is exactly the
                // output.
                let (v, w, _) = f.version_pair()?;
                let span = v.span(&w);
                let n = span.encode().len();
                let floors = Floors {
                    heap: heap_materializes(n),
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_BYTE_COPY),
                    touch: na(NA_TOUCH_NOT_FORCED),
                };
                Some(Cell::new(n, floors, move || (span.encode(), span)))
            },
        },
        Op {
            name: "span_decode",
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
            prepare: |f| {
                // The tick-walk families carry their own (event, id)
                // pair; every other family ticks its version with the
                // seed.
                if let Some((mut v, party, n)) = f.cross() {
                    let floors = tick_walk_floors(&v, n);
                    return Some(Cell::new(n, floors, move || {
                        v.tick(&party);
                        (v, party)
                    }));
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
            prepare: |f| {
                // The fused multi-tick at a fixed count: the same walk
                // and splice as the tick rows, with the count's gamma
                // width the only n-dependence — so the cell must scale
                // exactly as the tick cell above it (the flatness rows
                // of tests/meter.rs pin the n axis; this cell pins the
                // input axis).
                if let Some((mut v, party, n)) = f.cross() {
                    let floors = tick_walk_floors(&v, n);
                    return Some(Cell::new(n, floors, move || {
                        v.ticks(&party, TICKS_BOARD_COUNT);
                        (v, party)
                    }));
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
            prepare: |f| {
                let (v, n) = f.version()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: touch_delta_fold(stored_nonzero_deltas(&v)),
                };
                Some(Cell::new(n, floors, move || (v.rank(), v)))
            },
        },
        Op {
            name: "rank_pair_ops",
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
            prepare: |f| {
                // The mixed fold: the family-derived rank (maximal exponent
                // on the spines) summed high-first with one small integer
                // rank per encoded byte of the family's measure operand, so
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
                        let mut version = Version::new();
                        version.ticks(&Party::seed(), i as u64 % 7 + 1);
                        version.rank()
                    })
                    .collect();
                let n = (a.content_bits().div_ceil(8) as usize)
                    + ones
                        .iter()
                        .map(|r| r.content_bits().div_ceil(8) as usize)
                        .sum::<usize>();
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
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
            prepare: |f| {
                // The canonical bytes of the family-derived rank,
                // encoded at prepare, outside measurement; the operand
                // is the byte string itself, input-denominated like
                // every codec row (the coding is canonical 1:1).
                let (a, _) = f.rank_pair.clone()?;
                let bytes = a.encode();
                let numerator_bytes = rank_numerator_bytes(&a);
                let floors = Floors {
                    heap: heap_materializes(numerator_bytes),
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
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: touch_pair_fold(&v, &w),
                };
                Some(Cell::new(n, floors, move || (v.distance(&w), v, w)))
            },
        },
        Op {
            name: "version_lag",
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: touch_pair_fold(&v, &w),
                };
                Some(Cell::new(n, floors, move || (v.lag(&w), v, w)))
            },
        },
        Op {
            name: "ranked_cmp",
            prepare: |f| {
                // The fused rank comparison: the distance/lag co-sweep
                // with constant orientation, so it takes their floors
                // (the walk decodes both streams, folds every nonzero
                // delta, and answers a word-scale verdict).
                let (v, w, n) = f.version_pair()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
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
            prepare: |f| {
                // The composite key emission: the fused rank fold's
                // floors plus the mandatory output (rank stream, then
                // one copy of the version's encoded bytes).
                // Input-denominated: the provenance pin bounds the
                // rank component within the encoded input plus one
                // byte, and the version tail is exactly the encoded
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
                     (within the encoded input plus one byte) plus the version's \
                     encoded bytes"
                );
                let floors = Floors {
                    heap: heap_materializes(encoded_len),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: touch_delta_fold(stored_nonzero_deltas(&v)),
                };
                Some(Cell::new(n, floors, move || (Ranked::from(&v).encode(), v)))
            },
        },
        Op {
            name: "ranked_encode_rank",
            prepare: |f| {
                // The fused rank-to-bytes emission: the rank fold's
                // floors plus the mandatory output. Input-denominated:
                // the provenance pin bounds the output within the
                // encoded input (asserted here, so the bound is
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
                     version's encoded bytes"
                );
                let floors = Floors {
                    heap: heap_materializes(encoded_len),
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
            prepare: |f| {
                // The exact fold walks the whole stream, decodes every
                // stored code, and folds heights and minima on
                // accumulators: the rank fold's floor spec exactly.
                let (v, n) = f.version()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: touch_delta_fold(stored_nonzero_deltas(&v)),
                };
                Some(Cell::new(n, floors, move || (v.min_ticks(), v)))
            },
        },
        Op {
            name: "version_join_all",
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
                    // Comb-scatter produces much more output than input, so it
                    // carries the tighter total-I/O heap ceiling derived for
                    // this materialization.
                    return Some(if matches!(f.kind, FamilyId::CombScatter) {
                        cell.with_declared_heap(COMB_SCATTER_PROJECTION_HEAP_BYTES_PER_IO_BYTE)
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
            name: "version_hash",
            prepare: |f| {
                let (v, n) = f.version()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
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
            prepare: |f| {
                let (a, b) = f.parties.clone()?;
                let n = a.len() + b.len();
                let floors = Floors {
                    heap: heap_materializes(n),
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
            prepare: |f| {
                let (a, _, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(a, _)| a.len())?;
                let floors = Floors {
                    heap: heap_materializes(n),
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_BYTE_COPY),
                    touch: na(NA_TOUCH_ID_TREE),
                };
                Some(Cell::new(n, floors, move || (a.encode(), a)))
            },
        },
        Op {
            name: "party_fork",
            prepare: |f| {
                let (mut a, _, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(a, _)| a.len())?;
                // Fork builds both halves, so the child's own encoded bytes
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
            name: "party_forks",
            prepare: |f| {
                let (mut party, _, _) = f.party_pair()?;
                let party_bytes = f.parties.as_ref().map(|(party, _)| party.len())?;
                let (count, count_bytes) = wide_fork_count(party_bytes);
                let child_bytes = {
                    let mut probe = party.dangerously_alias();
                    probe
                        .forks(count.clone())
                        .next()
                        .expect("the board requests a nonzero child count")
                        .as_bytes()
                        .len()
                };
                let floors = Floors {
                    heap: heap_materializes(child_bytes),
                    segments: seg_ceiling_only(),
                    scan: if party.is_seed() {
                        na(NA_SCAN_SEED_PARTY)
                    } else {
                        scan_touch()
                    },
                    touch: na(NA_TOUCH_ID_TREE),
                };
                Some(Cell::new(party_bytes + count_bytes, floors, move || {
                    let first = {
                        let mut forks = party.forks(count);
                        forks.next()
                    };
                    (party, first)
                }))
            },
        },
        Op {
            name: "party_split_array",
            prepare: |f| {
                let (party, _, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(party, _)| party.len())?;
                let output_bytes = {
                    let shares: [Party; SPLIT_ARRAY_ARITY] = party.dangerously_alias().into();
                    shares.iter().map(|share| share.as_bytes().len()).sum()
                };
                let floors = Floors {
                    heap: heap_materializes(output_bytes),
                    segments: seg_ceiling_only(),
                    scan: if party.is_seed() {
                        na(NA_SCAN_SEED_PARTY)
                    } else {
                        scan_touch()
                    },
                    touch: na(NA_TOUCH_ID_TREE),
                };
                Some(Cell::io(
                    n,
                    floors,
                    |result| {
                        result
                            .downcast_ref::<[Party; SPLIT_ARRAY_ARITY]>()
                            .expect("the party split cell yields its share array")
                            .iter()
                            .map(|share| share.as_bytes().len())
                            .sum()
                    },
                    move || <[Party; SPLIT_ARRAY_ARITY]>::from(party),
                ))
            },
        },
        Op {
            name: "party_join",
            prepare: |f| {
                let (mut a, b, n) = f.party_pair()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
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
            prepare: |f| {
                let (_, parties) = f.fold.as_ref()?;
                let n = parties.iter().map(Vec::len).sum();
                let arity = parties.len() as u64;
                let mut parties = parties.iter().map(|b| decode_party(b));
                let acc = parties.next().expect("the scatter population is nonempty");
                let rest: Vec<Party> = parties.collect();
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
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
                    .with_fold_arity(arity),
                )
            },
        },
        Op {
            name: "party_covers",
            prepare: |f| {
                let (a, b, n) = f.party_pair()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    segments: seg_ceiling_only(),
                    scan: scan_touch(),
                    touch: na(NA_TOUCH_ID_TREE),
                };
                Some(Cell::new(n, floors, move || (a.covers(&b), a, b)))
            },
        },
        Op {
            name: "party_disjoint",
            prepare: |f| {
                let (a, b, n) = f.party_pair()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: na(NA_TOUCH_ID_TREE),
                };
                Some(Cell::new(n, floors, move || (a.is_disjoint(&b), a, b)))
            },
        },
        Op {
            name: "party_without",
            prepare: |f| {
                let (_, b, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(_, b)| b.len())?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
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
            name: "party_hash",
            prepare: |f| {
                let (a, _, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(a, _)| a.len())?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
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
            prepare: |f| {
                let (clock, _) = f.clock()?;
                let bytes = clock.encode();
                let floors = Floors {
                    heap: heap_materializes(bytes.len()),
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
            prepare: |f| {
                let (clock, n) = f.clock()?;
                let floors = Floors {
                    heap: heap_materializes(n),
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_BYTE_COPY),
                    touch: na(NA_TOUCH_NOT_FORCED),
                };
                Some(Cell::new(n, floors, move || (clock.encode(), clock)))
            },
        },
        Op {
            name: "clock_tick",
            prepare: |f| {
                // The tick-walk families tick their own (id, event)
                // clock; they reach no other clock row.
                if let Some((v, p, n)) = f.cross() {
                    let floors = tick_walk_floors(&v, n);
                    let mut clock = Clock::from_parts(p, v);
                    return Some(Cell::new(n, floors, move || {
                        clock.tick();
                        clock
                    }));
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
            prepare: |f| {
                let (mut clock, n) = f.clock()?;
                let floors = Floors {
                    heap: na(NA_HEAP_FORK_SHARES),
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
            name: "clock_forks",
            prepare: |f| {
                let (mut clock, n) = f.clock()?;
                let (count, count_bytes) = wide_fork_count(n);
                let child_bytes = {
                    let mut probe = clock.dangerously_alias();
                    probe
                        .forks(count.clone())
                        .next()
                        .expect("the board requests a nonzero child count")
                        .party()
                        .as_bytes()
                        .len()
                };
                let floors = Floors {
                    heap: heap_materializes(child_bytes),
                    segments: seg_ceiling_only(),
                    scan: if clock.party().is_seed() {
                        na(NA_SCAN_SEED_PARTY)
                    } else {
                        scan_touch()
                    },
                    touch: na(NA_TOUCH_NOT_FORCED),
                };
                Some(Cell::new(n + count_bytes, floors, move || {
                    let first = {
                        let mut forks = clock.forks(count);
                        forks.next()
                    };
                    (clock, first)
                }))
            },
        },
        Op {
            name: "clock_split_array",
            prepare: |f| {
                let (clock, _) = f.clock()?;
                let party_bytes = clock.party().as_bytes().len();
                let output_party_bytes = {
                    let clocks: [Clock; SPLIT_ARRAY_ARITY] = clock.dangerously_alias().into();
                    clocks
                        .iter()
                        .map(|clock| clock.party().as_bytes().len())
                        .sum()
                };
                let floors = Floors {
                    heap: heap_materializes(output_party_bytes),
                    segments: seg_ceiling_only(),
                    scan: if clock.party().is_seed() {
                        na(NA_SCAN_SEED_PARTY)
                    } else {
                        scan_touch()
                    },
                    touch: na(NA_TOUCH_NOT_FORCED),
                };
                Some(Cell::io(
                    party_bytes,
                    floors,
                    |result| {
                        result
                            .downcast_ref::<[Clock; SPLIT_ARRAY_ARITY]>()
                            .expect("the clock split cell yields its clock array")
                            .iter()
                            .map(|clock| clock.party().as_bytes().len())
                            .sum()
                    },
                    move || <[Clock; SPLIT_ARRAY_ARITY]>::from(clock),
                ))
            },
        },
        Op {
            name: "clock_join",
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
                let mut msg = Version::new();
                msg.tick(&Party::seed());
                let touch = touch_delta_fold(stored_nonzero_deltas(&msg));
                Some(Cell::new(n + 2, walk_floors(n, touch), move || {
                    clock.recv(&msg);
                    (clock, msg)
                }))
            },
        },
        Op {
            name: "clock_own_version_to_version",
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
                    // The same total-I/O heap ceiling as the version spelling.
                    return Some(if matches!(f.kind, FamilyId::CombScatter) {
                        cell.with_declared_heap(COMB_SCATTER_PROJECTION_HEAP_BYTES_PER_IO_BYTE)
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
                        segments: seg_ceiling_only(),
                        scan,
                        touch: na(NA_TOUCH_PROJECTION),
                    },
                    move || (clock.own_version().to_version(), clock),
                ))
            },
        },
        Op {
            name: "clock_hash",
            prepare: |f| {
                let (clock, n) = f.clock()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
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
            name: "span_decode_truncated",
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
            name: "clock_decode_truncated",
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
            name: "party_join_overlap",
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
            name: "party_without_none",
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

/// The bytes needed to store a rank's numerator, rounded up.
fn rank_numerator_bytes(rank: &Rank) -> usize {
    rank.raw_parts().0.bits().div_ceil(8).max(1) as usize
}
