//! The operation axis: the board's row table.
//!
//! Each row declares the bundle slots its signature consumes and prepares its
//! cell from them alone — never from the shape's identity — so a row reaches
//! every shape that supplies its operands.

use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::fmt::{self, Write};
use std::hash::{Hash, Hasher};

#[cfg(feature = "borsh")]
use borsh::BorshDeserialize;
use num_bigint::BigUint;
#[cfg(feature = "serde")]
use serde::de::{DeserializeOwned, Visitor};
#[cfg(feature = "serde")]
use serde::Deserializer;

use crate::causally::{self, Down, Query, Up};
use crate::error::Decode;
use crate::{shape, Clock, Party, Rank, Ranked, Span, Ticks, Version};

#[cfg(any(feature = "serde", feature = "borsh"))]
use super::ceilings::DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE;
use super::ceilings::{
    COMB_SCATTER_PROJECTION_HEAP_BYTES_PER_IO_BYTE, FOLD_SCAN_BITS_PER_INPUT_BYTE_PER_LEVEL,
    TICKS_BOARD_COUNT,
};
use super::cell::{Cell, ModelSpec};
use super::currency::{Currency, Floors, Liveness};
use super::defect::{
    party_noncanonical_bytes, trailing_bytes, truncated_bytes, version_noncanonical_bytes,
};
use super::family::{decode_party, decode_version, FamilyData};
use super::floors::{
    clock_overlap_floors, comparison_floors, heap_materializes, id_rejection_floors,
    masked_cmp_floors, membership_floors, na, rejection_floors, scan_examines, scan_touch,
    seg_ceiling_only, sync_floors, tick_walk_floors, touch_delta_fold, touch_fold_first_merges,
    touch_pair_fold, touch_wide_stream, walk_floors, NA_HEAP_FORK_SHARES, NA_HEAP_IN_PLACE,
    NA_HEAP_QUERY_CLONE, NA_SCAN_BYTE_COPY, NA_SCAN_EQ_BYTES, NA_SCAN_NO_STREAM,
    NA_SCAN_QUERY_CLONE, NA_SCAN_RANK_BYTES, NA_SCAN_SEED_PARTY, NA_SCAN_SEED_PROJECTION,
    NA_TOUCH_GROW, NA_TOUCH_ID_TREE, NA_TOUCH_NOT_FORCED, NA_TOUCH_PLACEMENT, NA_TOUCH_PROJECTION,
    NA_TOUCH_RANK_ARITHMETIC, NA_TOUCH_SEED_RAISE, WHY_HEAP_FORK_HALF, WHY_SCAN_EXAMINES,
    WHY_SCAN_OVERLAP_END, WHY_SCAN_REJECT_CROSSED, WHY_SCAN_REJECT_END, WHY_TOUCH_RANK_SUM,
};
use super::operand::{stored_nonzero_deltas, version_output_bytes};
use crate::meter::registry::FamilyId;

/// Arity used to exercise the public const-generic array operations.
const ARRAY_ARITY: usize = 16;

/// Why shape iteration has no representation-independent heap floor.
const NA_HEAP_SHAPE_WALK: &str =
    "paths and rises may share input storage or live inline; heap allocation is not required";

/// Why tick-count arithmetic does not drive the accumulator meter.
const NA_TOUCH_TICK_COUNT: &str =
    "tick counts use ordinary big-integer operations, not the skyline accumulator";

/// Why tick-count operations do not drive the encoded-stream meter.
const NA_SCAN_TICK_COUNT: &str = "tick counts have no encoded stream to walk";

/// Why formatting a tick count has no representation-independent heap floor.
const NA_HEAP_TICK_FORMAT: &str =
    "decimal digits may be streamed directly: heap allocation is not required";

/// Why constructing one query hole has no representation-independent heap floor.
const NA_HEAP_QUERY_CONSTRUCTION: &str =
    "one hole may live inline and its version buffer may be shared";

/// A formatting sink that counts UTF-8 bytes without storing them.
#[derive(Default)]
struct TextLen(usize);

/// Count the bytes written by a formatter.
impl fmt::Write for TextLen {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.0 += text.len();
        Ok(())
    }
}

/// Format into a counting sink and return the number of bytes written.
fn formatted_len(args: fmt::Arguments<'_>) -> usize {
    let mut len = TextLen::default();
    len.write_fmt(args)
        .expect("counting formatted bytes cannot fail");
    len.0
}

/// Drain a shape iterator while making every yielded item observable.
fn drain_shape(items: impl IntoIterator) {
    for item in items {
        std::hint::black_box(item);
    }
}

/// Resource floors shared by shape iterators that read every input bit.
fn shape_floors(input_bytes: usize, touch: Liveness) -> Floors {
    Floors {
        heap: na(NA_HEAP_SHAPE_WALK),
        segments: seg_ceiling_only(),
        scan: scan_examines(input_bytes),
        touch,
    }
}

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

/// Join the first half of a fold population into one party to repartition.
///
/// The fold populations preserve one universe and adversarial ordering. In
/// particular, the scatter population lists even leaves before odd leaves, so
/// its first half joins to an alternating-leaf region rather than the seed.
/// The returned arity scales with populations that scale their operand count.
fn joined_fold_prefix(f: &FamilyData) -> Option<(Party, usize)> {
    let (_, parties) = f.population.as_ref()?;
    let arity = parties.len() / 2;
    let mut parties = parties.iter().take(arity).map(|bytes| decode_party(bytes));
    let mut joined = parties
        .next()
        .expect("a board fold population has at least two parties");
    joined
        .join_all(parties)
        .expect("one fold population contains disjoint parties");
    Some((joined, arity))
}

/// Stored bytes needed for a positive machine-sized fork count.
fn fork_count_bytes(count: usize) -> usize {
    usize::try_from((usize::BITS - count.leading_zeros()).div_ceil(8))
        .expect("a machine-word bit count fits usize")
}

/// Apply the balanced fold's `D log k` work model.
///
/// Scan has a measured constant per reduction level. Touch keeps its ordinary
/// per-input-byte ceiling, but fits growth against the same logarithmic work
/// axis so increasing arity is not mistaken for super-linearity.
fn fold_model(cell: Cell, arity: u64) -> Cell {
    let levels = (2.0 * arity as f64).log2();
    cell.with_model(
        Currency::Scan,
        ModelSpec::scaled(levels, FOLD_SCAN_BITS_PER_INPUT_BYTE_PER_LEVEL),
    )
    .with_model(Currency::Touch, ModelSpec::scaled_trend(levels))
}

/// The four balanced span folds measured by the board.
#[derive(Clone, Copy)]
enum SpanFold {
    /// Containment union.
    Union,
    /// Containment intersection.
    Intersect,
    /// Pointwise join.
    Join,
    /// Pointwise meet.
    Meet,
}

/// Two ordered spans derived from a family's version operands.
///
/// Population families contribute four independent versions, so both endpoint
/// pairs require real lattice operations. Other version families contribute
/// two spans with a shared meet, retaining their difficult shape on the upper
/// endpoint pair.
fn span_pair(f: &FamilyData) -> Option<(Span<'static>, Span<'static>, usize)> {
    let pair = if let Some((versions, _)) = &f.population {
        let mut versions = versions.iter().take(4).map(|bytes| decode_version(bytes));
        let (a, a_extra, b, b_extra) = (
            versions.next()?,
            versions.next()?,
            versions.next()?,
            versions.next()?,
        );
        let a_hi = &a | &a_extra;
        let b_hi = &b | &b_extra;
        (
            Span::new(a, a_hi).expect("a version precedes its join"),
            Span::new(b, b_hi).expect("a version precedes its join"),
        )
    } else {
        let (a, b, _) = f.version_pair()?;
        let shared_lo = &a & &b;
        (
            Span::new(shared_lo.clone(), a).expect("a meet precedes its operand"),
            Span::new(shared_lo, b).expect("a meet precedes its operand"),
        )
    };
    let n = span_bytes(&pair.0) + span_bytes(&pair.1);
    Some((pair.0, pair.1, n))
}

/// A population of ordered spans with independent lower and upper endpoints.
fn span_population(f: &FamilyData) -> Option<Vec<Span<'static>>> {
    let (versions, _) = f.population.as_ref()?;
    let mut versions = versions.iter().map(|bytes| decode_version(bytes));
    let mut spans = Vec::with_capacity(versions.len().div_ceil(2));
    while let Some(lo) = versions.next() {
        let Some(extra) = versions.next() else {
            spans.push(Span::at(lo));
            break;
        };
        let hi = &lo | &extra;
        spans.push(Span::new(lo, hi).expect("a version precedes its join"));
    }
    (spans.len() >= 2).then_some(spans)
}

/// Measure one balanced span fold over a population-derived operand set.
fn span_fold(f: &FamilyData, kind: SpanFold) -> Option<Cell> {
    let mut spans = span_population(f)?;
    let n = spans.iter().map(span_bytes).sum();
    let arity = spans.len() as u64;
    let receiver = spans.pop().expect("a span population is nonempty");
    let cell = match kind {
        SpanFold::Union => Cell::new(n, span_algebra_floors(), move || {
            (receiver.union_all(&spans), receiver, spans)
        }),
        SpanFold::Intersect => Cell::new(n, span_algebra_floors(), move || {
            (receiver.intersect_all(&spans), receiver, spans)
        }),
        SpanFold::Join => Cell::new(n, span_algebra_floors(), move || {
            (receiver.join_all(&spans), receiver, spans)
        }),
        SpanFold::Meet => Cell::new(n, span_algebra_floors(), move || {
            (receiver.meet_all(&spans), receiver, spans)
        }),
    };
    Some(fold_model(cell, arity))
}

/// Stored endpoint bytes in a span operand or result.
fn span_bytes(span: &Span<'_>) -> usize {
    span.lo().as_bytes().len() + span.hi().as_bytes().len()
}

/// Resource floors for span algebra, whose endpoint kernels may exit early.
fn span_algebra_floors() -> Floors {
    Floors {
        heap: na(NA_HEAP_IN_PLACE),
        segments: seg_ceiling_only(),
        scan: scan_touch(),
        touch: na(NA_TOUCH_NOT_FORCED),
    }
}

/// Multi-hole query operands derived from a population.
///
/// The down-query holes retain each population version's tree shape. Its high
/// endpoint advances each whole party, then one fork half by a varying count,
/// so it dominates every hole without flattening the population's topology.
///
/// The up-query probe contains every hole and advances the opposite fork
/// halves by varying counts. No hole can be dismissed before exhaustion, so
/// the walk exercises its work per live hole over a growing probe.
pub(super) struct QueryOperands {
    pub(super) down_holes: Vec<Version>,
    pub(super) up_holes: Vec<Version>,
    pub(super) down_hi: Version,
    pub(super) up_probe: Version,
}

impl QueryOperands {
    /// Build both query fixtures from any population bundle.
    pub(super) fn build(f: &FamilyData) -> Option<QueryOperands> {
        let (versions, parties) = f.population.as_ref()?;
        assert_eq!(
            versions.len(),
            parties.len(),
            "a population pairs every version with its party"
        );
        let mut down_holes = Vec::with_capacity(parties.len());
        let mut up_holes = Vec::with_capacity(parties.len());
        let mut down_parts = Vec::with_capacity(parties.len());
        let mut up_parts = Vec::with_capacity(parties.len());

        for (i, (version, party)) in versions.iter().zip(parties).enumerate() {
            let mut whole_party = decode_party(party);
            let version = decode_version(version);
            let mut down_hole = version.project(&whole_party).to_version();
            down_hole.tick(&whole_party);

            let count = u64::try_from(i % 7 + 1).expect("the count is at most seven");
            let mut down = down_hole.clone();
            down.tick(&whole_party);
            let probe_party = whole_party.fork();
            down.ticks(&probe_party, count);

            let mut up_hole = Version::new();
            up_hole.tick(&whole_party);
            let mut up = up_hole.clone();
            up.ticks(&probe_party, count);

            down_holes.push(down_hole);
            up_holes.push(up_hole);
            down_parts.push(down);
            up_parts.push(up);
        }

        let down_hi: Version = down_parts.into_iter().sum();
        let up_probe: Version = up_parts.into_iter().sum();
        Some(QueryOperands {
            down_holes,
            up_holes,
            down_hi,
            up_probe,
        })
    }
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
    let operations = vec![
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
            name: "span_union",
            prepare: |f| {
                let (left, right, n) = span_pair(f)?;
                Some(Cell::new(n, span_algebra_floors(), move || {
                    (left.union(&right), left, right)
                }))
            },
        },
        Op {
            name: "span_intersect",
            prepare: |f| {
                let (left, right, n) = span_pair(f)?;
                Some(Cell::new(n, span_algebra_floors(), move || {
                    (left.intersect(&right), left, right)
                }))
            },
        },
        Op {
            name: "span_join",
            prepare: |f| {
                let (left, right, n) = span_pair(f)?;
                Some(Cell::new(n, span_algebra_floors(), move || {
                    (left.join(&right), left, right)
                }))
            },
        },
        Op {
            name: "span_meet",
            prepare: |f| {
                let (left, right, n) = span_pair(f)?;
                Some(Cell::new(n, span_algebra_floors(), move || {
                    (left.meet(&right), left, right)
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
            name: "rank_cmp",
            prepare: |f| {
                let (a, b) = f.rank_pair.clone()?;
                let n = rank_pair_bytes(&a, &b);
                let floors = rank_floors(na(NA_HEAP_IN_PLACE));
                Some(Cell::new(n, floors, move || (a.cmp(&b), a, b)))
            },
        },
        Op {
            name: "rank_add",
            prepare: |f| {
                let (a, b) = f.rank_pair.clone()?;
                let n = rank_pair_bytes(&a, &b);
                let floors = rank_floors(na(NA_HEAP_IN_PLACE));
                Some(Cell::new(n, floors, move || (&a + &b, a, b)))
            },
        },
        Op {
            name: "rank_checked_sub",
            prepare: |f| {
                let (a, b) = f.rank_pair.clone()?;
                let (larger, smaller) = if a >= b { (a, b) } else { (b, a) };
                let n = rank_pair_bytes(&larger, &smaller);
                let floors = rank_floors(na(NA_HEAP_IN_PLACE));
                Some(Cell::new(n, floors, move || {
                    let difference = larger
                        .checked_sub(&smaller)
                        .expect("the operands are ordered before measurement");
                    (difference, larger, smaller)
                }))
            },
        },
        Op {
            name: "rank_sum",
            prepare: |f| {
                // Sum the family-derived rank with one small integer per
                // encoded byte of its measure operand, so both the wide value
                // and the list length grow with the family. The denominator
                // is the summands' total value content.
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
            name: "rank_display",
            prepare: |f| {
                let (rank, _) = f.rank_pair.clone()?;
                let input_bytes = rank_content_bytes(&rank);
                let output_bytes = rank.to_string().len();
                let floors = rank_floors(heap_materializes(output_bytes));
                Some(Cell::io(
                    input_bytes,
                    floors,
                    rank_text_output_bytes,
                    move || (rank.to_string(), rank),
                ))
            },
        },
        Op {
            name: "rank_display_precision",
            prepare: |f| {
                let (rank, _) = f.rank_pair.clone()?;
                let input_bytes = rank_content_bytes(&rank);
                // Fixed precision exercises truncation while leaving rank
                // width as the only scaling input.
                let output_bytes = format!("{rank:.1}").len();
                let floors = rank_floors(heap_materializes(output_bytes));
                Some(Cell::io(
                    input_bytes,
                    floors,
                    rank_text_output_bytes,
                    move || (format!("{rank:.1}"), rank),
                ))
            },
        },
        Op {
            name: "rank_parse",
            prepare: |f| {
                let (rank, _) = f.rank_pair.clone()?;
                let text = rank.to_string();
                let floors = rank_floors(heap_materializes(rank_numerator_bytes(&rank)));
                Some(Cell::new(text.len(), floors, move || {
                    text.parse::<Rank>()
                        .expect("a displayed rank is canonical text")
                }))
            },
        },
        Op {
            name: "ticks_clone",
            prepare: |f| {
                let count = tick_counts(f)?
                    .into_iter()
                    .max_by_key(|count| count.0.bits())?;
                let n = tick_count_bytes(&count);
                Some(Cell::new(
                    n,
                    tick_count_floors(heap_materializes(tick_count_value_bytes(&count))),
                    move || (count.clone(), count),
                ))
            },
        },
        Op {
            name: "ticks_add",
            prepare: |f| {
                let mut counts = tick_counts(f)?.into_iter();
                let (a, b) = (counts.next()?, counts.next()?);
                let n = tick_count_bytes(&a) + tick_count_bytes(&b);
                let output_bytes = tick_count_value_bytes(&a).max(tick_count_value_bytes(&b));
                Some(Cell::new(
                    n,
                    tick_count_floors(heap_materializes(output_bytes)),
                    move || (&a + &b, a, b),
                ))
            },
        },
        Op {
            name: "ticks_sum",
            prepare: |f| {
                let counts = tick_counts(f)?;
                let n = counts.iter().map(tick_count_bytes).sum();
                let output_bytes = counts.iter().map(tick_count_value_bytes).max()?;
                Some(Cell::new(
                    n,
                    tick_count_floors(heap_materializes(output_bytes)),
                    move || {
                        let sum: Ticks = counts.iter().sum();
                        (sum, counts)
                    },
                ))
            },
        },
        Op {
            name: "ticks_display",
            prepare: |f| {
                let count = tick_counts(f)?
                    .into_iter()
                    .max_by_key(|count| count.0.bits())?;
                let n = tick_count_bytes(&count);
                Some(Cell::io(
                    n,
                    tick_count_floors(na(NA_HEAP_TICK_FORMAT)),
                    tick_count_text_output_bytes,
                    move || (formatted_len(format_args!("{count}")), count),
                ))
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
                let (versions, _) = f.population.as_ref()?;
                let n = versions.iter().map(Vec::len).sum();
                let mut versions: Vec<Version> =
                    versions.iter().map(|b| decode_version(b)).collect();
                let arity = versions.len() as u64;
                let touch = touch_fold_first_merges(&versions);
                let rest = versions.split_off(1);
                let receiver = versions.pop()?;
                Some(fold_model(
                    Cell::new(n, walk_floors(n, touch), move || receiver.join_all(rest)),
                    arity,
                ))
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
                let (versions, _) = f.population.as_ref()?;
                let n = versions.iter().map(Vec::len).sum();
                let mut versions: Vec<Version> =
                    versions.iter().map(|b| decode_version(b)).collect();
                let arity = versions.len() as u64;
                let touch = touch_fold_first_merges(&versions);
                let rest = versions.split_off(1);
                let receiver = versions.pop()?;
                Some(fold_model(
                    Cell::new(n, walk_floors(n, touch), move || receiver.meet_all(rest)),
                    arity,
                ))
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
                let (versions, _) = f.population.as_ref()?;
                let n = versions.iter().map(Vec::len).sum();
                let versions: Vec<Version> = versions.iter().map(|b| decode_version(b)).collect();
                let arity = versions.len() as u64;
                let touch = touch_fold_first_merges(&versions);
                Some(fold_model(
                    Cell::new(n, walk_floors(n, touch), move || {
                        let hull = versions[0].span_all(&versions[1..]);
                        (hull, versions)
                    }),
                    arity,
                ))
            },
        },
        Op {
            name: "span_union_all",
            prepare: |f| span_fold(f, SpanFold::Union),
        },
        Op {
            name: "span_intersect_all",
            prepare: |f| span_fold(f, SpanFold::Intersect),
        },
        Op {
            name: "span_join_all",
            prepare: |f| span_fold(f, SpanFold::Join),
        },
        Op {
            name: "span_meet_all",
            prepare: |f| span_fold(f, SpanFold::Meet),
        },
        Op {
            name: "own_version_to_version",
            prepare: |f| {
                // The explicit materialization `(&v / &p).to_version()`:
                // the one projection spelling that pays the product-growth
                // output. Adversarial × adversarial with mandatory
                // dominating output: a declared output-domination cross,
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
                        cell.with_model(
                            Currency::Heap,
                            ModelSpec::ceiling(COMB_SCATTER_PROJECTION_HEAP_BYTES_PER_IO_BYTE),
                        )
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
            name: "version_shape",
            prepare: |f| {
                let (version, n) = f.version()?;
                let floors = shape_floors(n, na(NA_TOUCH_NOT_FORCED));
                Some(Cell::new(n, floors, move || {
                    drain_shape(version.shape());
                }))
            },
        },
        Op {
            name: "shape_combine_pair",
            prepare: |f| {
                let (left, right, n) = f.version_pair()?;
                let floors = shape_floors(n, na(NA_TOUCH_NOT_FORCED));
                Some(Cell::new(n, floors, move || {
                    drain_shape(shape::combine([&left, &right]));
                }))
            },
        },
        Op {
            name: "shape_combine_many",
            prepare: |f| {
                let (versions, _) = f.population.as_ref()?;
                if versions.is_empty() {
                    return None;
                }
                let versions: [Version; ARRAY_ARITY] = versions
                    .iter()
                    .cycle()
                    .take(ARRAY_ARITY)
                    .map(|bytes| decode_version(bytes))
                    .collect::<Vec<_>>()
                    .try_into()
                    .ok()?;
                let n = versions
                    .iter()
                    .map(|version| version.as_bytes().len())
                    .sum();
                let floors = shape_floors(n, na(NA_TOUCH_NOT_FORCED));
                Some(Cell::new(n, floors, move || {
                    drain_shape(shape::combine(versions.each_ref()));
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
        Op {
            name: "query_single_hole",
            prepare: |f| {
                let (version, n) = f.version()?;
                let floors = Floors {
                    heap: na(NA_HEAP_QUERY_CONSTRUCTION),
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_QUERY_CLONE),
                    touch: na(NA_TOUCH_NOT_FORCED),
                };
                Some(Cell::new(n, floors, move || {
                    (causally::strictly_after(version.clone()), version)
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
        Op {
            name: "query_contains_many",
            prepare: |f| {
                let operands = QueryOperands::build(f)?;
                let n = operands
                    .up_holes
                    .iter()
                    .map(|hole| hole.encode().len())
                    .sum::<usize>()
                    + operands.up_probe.encode().len();
                let work = n
                    .checked_mul(operands.up_holes.len())
                    .expect("allocated query operands have a representable work bound");
                let query = Query::<Up>::from_inclusive_holes(operands.up_holes);
                let probe = operands.up_probe;
                Some(
                    Cell::new(n, walk_floors(n, na(NA_TOUCH_PLACEMENT)), move || {
                        (query.contains(&probe), query, probe)
                    })
                    .with_model(Currency::Touch, ModelSpec::work(work)),
                )
            },
        },
        Op {
            name: "query_coverage_many",
            prepare: |f| {
                let operands = QueryOperands::build(f)?;
                let lo = Version::new();
                let n = operands
                    .down_holes
                    .iter()
                    .map(|hole| hole.encode().len())
                    .sum::<usize>()
                    + lo.encode().len()
                    + operands.down_hi.encode().len();
                let work = n
                    .checked_mul(operands.down_holes.len())
                    .expect("allocated query operands have a representable work bound");
                let query = Query::<Down>::from_inclusive_holes(operands.down_holes);
                let span = lo.span(&operands.down_hi);
                Some(
                    Cell::new(n, walk_floors(n, na(NA_TOUCH_PLACEMENT)), move || {
                        (query.coverage(span.reborrow()), query, span)
                    })
                    .with_model(Currency::Touch, ModelSpec::work(work)),
                )
            },
        },
        Op {
            name: "query_conjoin_many",
            prepare: |f| {
                let operands = QueryOperands::build(f)?;
                let mut left_holes = operands.down_holes;
                let right_holes = left_holes.split_off(left_holes.len() / 2);
                if left_holes.is_empty() || right_holes.is_empty() {
                    return None;
                }
                let left_bytes = left_holes
                    .iter()
                    .map(|hole| hole.as_bytes().len())
                    .sum::<usize>();
                let right_bytes = right_holes
                    .iter()
                    .map(|hole| hole.as_bytes().len())
                    .sum::<usize>();
                let n = left_bytes
                    .checked_add(right_bytes)
                    .expect("allocated query operands have a representable size");
                // Every cross-side pair is concurrent. Comparing them reads
                // each left hole once per right hole and vice versa.
                let work = left_bytes
                    .checked_mul(right_holes.len())
                    .and_then(|left| {
                        right_bytes
                            .checked_mul(left_holes.len())
                            .and_then(|right| left.checked_add(right))
                    })
                    .expect("allocated query operands have a representable work bound");
                let left = Query::<Down>::from_inclusive_holes(left_holes);
                let right = Query::<Down>::from_inclusive_holes(right_holes);
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    segments: seg_ceiling_only(),
                    scan: scan_touch(),
                    touch: na(NA_TOUCH_PLACEMENT),
                };
                Some(
                    Cell::new(n, floors, move || left & right)
                        .with_model(Currency::Scan, ModelSpec::work(work))
                        .with_model(Currency::Touch, ModelSpec::work(work)),
                )
            },
        },
        Op {
            name: "query_clone_many",
            prepare: |f| {
                let operands = QueryOperands::build(f)?;
                let holes = operands.down_holes;
                let n = holes
                    .iter()
                    .map(|hole| hole.as_bytes().len())
                    .sum::<usize>();
                let query = Query::<Down>::from_inclusive_holes(holes);
                let floors = Floors {
                    heap: na(NA_HEAP_QUERY_CLONE),
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_QUERY_CLONE),
                    touch: na(NA_TOUCH_NOT_FORCED),
                };
                Some(Cell::new(n, floors, move || (query.clone(), query)))
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
            name: "party_forks_full",
            prepare: |f| {
                let (mut party, arity) = joined_fold_prefix(f)?;
                let child_count = arity - 1;
                let party_bytes = party.as_bytes().len();
                let count_bytes = fork_count_bytes(child_count);
                let input_bytes = party_bytes + count_bytes;
                // A borrowing drain may scan the stored party once per child;
                // count bookkeeping is bounded by the count's own width. This
                // is the public `k (D + log k)` bound expressed in byte units.
                let scan_work = child_count
                    .checked_mul(input_bytes)
                    .expect("the board's fork work model fits usize");
                let floors = Floors {
                    // The result retains every share. Even when canonical
                    // collapses shorten them, at least one encoded byte per
                    // share must be materialized.
                    heap: heap_materializes(arity),
                    segments: seg_ceiling_only(),
                    scan: scan_touch(),
                    touch: na(NA_TOUCH_ID_TREE),
                };
                Some(
                    Cell::io(
                        input_bytes,
                        floors,
                        |result| {
                            let (residual, children) = result
                                .downcast_ref::<(Party, Vec<Party>)>()
                                .expect("the full party-forks cell retains every share");
                            residual.as_bytes().len()
                                + children
                                    .iter()
                                    .map(|party| party.as_bytes().len())
                                    .sum::<usize>()
                        },
                        move || {
                            let children: Vec<Party> = party.forks(child_count).collect();
                            (party, children)
                        },
                    )
                    .with_model(Currency::Scan, ModelSpec::work(scan_work)),
                )
            },
        },
        Op {
            name: "party_split_array",
            prepare: |f| {
                let (party, _, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(party, _)| party.len())?;
                let output_bytes = {
                    let shares: [Party; ARRAY_ARITY] = party.dangerously_alias().into();
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
                            .downcast_ref::<[Party; ARRAY_ARITY]>()
                            .expect("the party split cell yields its share array")
                            .iter()
                            .map(|share| share.as_bytes().len())
                            .sum()
                    },
                    move || <[Party; ARRAY_ARITY]>::from(party),
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
                let (_, parties) = f.population.as_ref()?;
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
                Some(fold_model(
                    Cell::new(n, floors, move || {
                        let mut acc = acc;
                        acc.join_all(rest)
                            .expect("fold operands are forked parties, pairwise disjoint");
                        acc
                    }),
                    arity,
                ))
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
        Op {
            name: "party_shape",
            prepare: |f| {
                let (party, _, _) = f.party_pair()?;
                let n = party.as_bytes().len();
                let floors = shape_floors(n, na(NA_TOUCH_ID_TREE));
                Some(Cell::new(n, floors, move || {
                    drain_shape(party.shape());
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
            name: "clock_forks_full",
            prepare: |f| {
                let (party, arity) = joined_fold_prefix(f)?;
                let child_count = arity - 1;
                let party_bytes = party.as_bytes().len();
                let count_bytes = fork_count_bytes(child_count);
                let input_bytes = party_bytes + 1 + count_bytes;
                // Clock delegates partitioning to Party. Its shared empty
                // Version is neither scanned nor copied, so the scan model is
                // the same `k (D + log k)` bound over party and count bytes.
                let scan_work = child_count
                    .checked_mul(party_bytes + count_bytes)
                    .expect("the board's fork work model fits usize");
                let mut clock = Clock::from_parts(party, Version::new());
                let floors = Floors {
                    heap: heap_materializes(arity),
                    segments: seg_ceiling_only(),
                    scan: scan_touch(),
                    touch: na(NA_TOUCH_NOT_FORCED),
                };
                Some(
                    Cell::io(
                        input_bytes,
                        floors,
                        |result| {
                            let (residual, children) = result
                                .downcast_ref::<(Clock, Vec<Clock>)>()
                                .expect("the full clock-forks cell retains every share");
                            residual.party().as_bytes().len()
                                + children
                                    .iter()
                                    .map(|clock| clock.party().as_bytes().len())
                                    .sum::<usize>()
                        },
                        move || {
                            let children: Vec<Clock> = clock.forks(child_count).collect();
                            (clock, children)
                        },
                    )
                    .with_model(Currency::Scan, ModelSpec::work(scan_work)),
                )
            },
        },
        Op {
            name: "clock_split_array",
            prepare: |f| {
                let (clock, _) = f.clock()?;
                let party_bytes = clock.party().as_bytes().len();
                let output_party_bytes = {
                    let clocks: [Clock; ARRAY_ARITY] = clock.dangerously_alias().into();
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
                            .downcast_ref::<[Clock; ARRAY_ARITY]>()
                            .expect("the clock split cell yields its clock array")
                            .iter()
                            .map(|clock| clock.party().as_bytes().len())
                            .sum()
                    },
                    move || <[Clock; ARRAY_ARITY]>::from(clock),
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
            name: "clock_sync_all",
            prepare: |f| {
                let (versions, parties) = f.population.as_ref()?;
                let arity = versions.len() / 2;
                let versions: Vec<Version> = versions
                    .iter()
                    .take(arity)
                    .map(|bytes| decode_version(bytes))
                    .collect();
                let version_bytes: usize = versions
                    .iter()
                    .map(|version| version.as_bytes().len())
                    .sum();
                let touch = touch_fold_first_merges(&versions);
                let mut clocks: Vec<Clock> = versions
                    .into_iter()
                    .zip(parties.iter().take(arity).map(|bytes| decode_party(bytes)))
                    .map(|(version, party)| Clock::from_parts(party, version))
                    .collect();
                let input_bytes: usize = clocks
                    .iter()
                    .map(|clock| clock.party().as_bytes().len() + clock.version().as_bytes().len())
                    .sum();
                let mut receiver = clocks.remove(0);
                let floors = Floors {
                    // `sync_all` collects one mutable reference per peer before
                    // reducing them, so the heap meter must observe arity even
                    // when the canonical result becomes small.
                    heap: heap_materializes(arity),
                    segments: seg_ceiling_only(),
                    // Every input version participates in the version fold;
                    // party joins may splice exclusive subtrees without
                    // reading their interiors.
                    scan: scan_examines(version_bytes),
                    touch,
                };
                Some(fold_model(
                    Cell::io(
                        input_bytes,
                        floors,
                        |result| {
                            let (receiver, others) = result
                                .downcast_ref::<(Clock, Vec<Clock>)>()
                                .expect("the sync-all cell retains every participant");
                            receiver.version().as_bytes().len()
                                + receiver.party().as_bytes().len()
                                + others
                                    .iter()
                                    .map(|clock| clock.party().as_bytes().len())
                                    .sum::<usize>()
                        },
                        move || {
                            receiver
                                .sync_all(clocks.iter_mut())
                                .expect("one fold population contains disjoint parties");
                            (receiver, clocks)
                        },
                    ),
                    arity as u64,
                ))
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
                        cell.with_model(
                            Currency::Heap,
                            ModelSpec::ceiling(COMB_SCATTER_PROJECTION_HEAP_BYTES_PER_IO_BYTE),
                        )
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
        Op {
            name: "clock_shape",
            prepare: |f| {
                let (clock, n) = f.clock()?;
                let floors = shape_floors(n, na(NA_TOUCH_NOT_FORCED));
                Some(Cell::new(n, floors, move || {
                    drain_shape(clock.shape());
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
    ];
    #[cfg(any(feature = "serde", feature = "borsh"))]
    let mut operations = operations;
    #[cfg(feature = "serde")]
    operations.extend(serde_decode_ops());
    #[cfg(feature = "borsh")]
    operations.extend(borsh_decode_ops());
    operations
}

/// The bytes needed to store a rank's numerator, rounded up.
fn rank_numerator_bytes(rank: &Rank) -> usize {
    rank.raw_parts().0.bits().div_ceil(8).max(1) as usize
}

/// A rank's logical value width, rounded up to bytes.
fn rank_content_bytes(rank: &Rank) -> usize {
    rank.content_bits().div_ceil(8) as usize
}

/// The logical value width of two rank operands, rounded up to bytes.
fn rank_pair_bytes(a: &Rank, b: &Rank) -> usize {
    (a.content_bits() + b.content_bits()).div_ceil(8) as usize
}

/// Resource floors shared by operations over decoded ranks.
fn rank_floors(heap: Liveness) -> Floors {
    Floors {
        heap,
        segments: seg_ceiling_only(),
        scan: na(NA_SCAN_NO_STREAM),
        touch: na(NA_TOUCH_RANK_ARITHMETIC),
    }
}

/// Read the rendered text length from a rank-formatting result.
fn rank_text_output_bytes(result: &dyn std::any::Any) -> usize {
    result
        .downcast_ref::<(String, Rank)>()
        .expect("a rank display cell keeps its text")
        .0
        .len()
}

/// Tick counts derived from a family's rank or population values.
fn tick_counts(f: &FamilyData) -> Option<Vec<Ticks>> {
    if let Some((versions, _)) = &f.population {
        let counts = versions
            .iter()
            .map(|bytes| {
                let version = decode_version(bytes);
                Ticks(version.rank().raw_parts().0.clone())
            })
            .collect::<Vec<_>>();
        return (!counts.is_empty()).then_some(counts);
    }
    let (a, b) = f.rank_pair.as_ref()?;
    Some(vec![
        Ticks(a.raw_parts().0.clone()),
        Ticks(b.raw_parts().0.clone()),
    ])
}

/// A tick count's numeric width, rounded up to bytes.
fn tick_count_bytes(count: &Ticks) -> usize {
    tick_count_value_bytes(count).max(1)
}

/// Bytes needed to store a tick count's significant bits.
fn tick_count_value_bytes(count: &Ticks) -> usize {
    count.0.bits().div_ceil(8) as usize
}

/// Resource floors shared by operations over decoded tick counts.
fn tick_count_floors(heap: Liveness) -> Floors {
    Floors {
        heap,
        segments: seg_ceiling_only(),
        scan: na(NA_SCAN_TICK_COUNT),
        touch: na(NA_TOUCH_TICK_COUNT),
    }
}

/// Read the rendered text length from a tick-count formatting result.
fn tick_count_text_output_bytes(result: &dyn std::any::Any) -> usize {
    result
        .downcast_ref::<(usize, Ticks)>()
        .expect("a tick-count display cell keeps its byte count")
        .0
}

/// A binary serde input which gives its byte allocation to the visitor.
///
/// This is the ownership case the serde adapters are designed to preserve: a
/// format has already accumulated one field in a `Vec`, then transfers that
/// allocation through `visit_byte_buf`. Reporting the format as binary also
/// keeps [`Rank`]'s text representation out of this byte-oriented path.
#[cfg(feature = "serde")]
struct OwnedBytes(Vec<u8>);

#[cfg(feature = "serde")]
impl<'de> Deserializer<'de> for OwnedBytes {
    type Error = serde::de::value::Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_byte_buf(self.0)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_byte_buf(self.0)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_byte_buf(self.0)
    }

    fn is_human_readable(&self) -> bool {
        false
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        option unit unit_struct newtype_struct seq tuple tuple_struct map struct enum
        identifier ignored_any
    }
}

/// Deserialize one binary serde byte field while transferring its allocation.
#[cfg(feature = "serde")]
fn serde_from_owned_bytes<T: DeserializeOwned>(bytes: Vec<u8>) -> T {
    T::deserialize(OwnedBytes(bytes))
        .unwrap_or_else(|error| panic!("board-generated serde bytes must decode: {error}"))
}

/// Direct serde deserialization rows for the two encoded tree grammars.
#[cfg(feature = "serde")]
fn serde_decode_ops() -> [Op; 2] {
    const OWNED_INPUT: &str = "serde transfers the caller's byte allocation into the decoded \
        value; only validation state may allocate";
    [
        Op {
            name: "party_serde_deserialize",
            prepare: |f| {
                let (bytes, _) = f.parties.clone()?;
                let n = bytes.len();
                let floors = Floors {
                    heap: na(OWNED_INPUT),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: na(NA_TOUCH_ID_TREE),
                };
                Some(
                    Cell::new(n, floors, move || serde_from_owned_bytes::<Party>(bytes))
                        .with_model(
                            Currency::Heap,
                            ModelSpec::ceiling(DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE),
                        ),
                )
            },
        },
        Op {
            name: "version_serde_deserialize",
            prepare: |f| {
                let bytes = f.version.clone()?;
                let n = bytes.len();
                let version = decode_version(&bytes);
                let floors = Floors {
                    heap: na(OWNED_INPUT),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: touch_wide_stream(&version),
                };
                Some(
                    Cell::new(n, floors, move || serde_from_owned_bytes::<Version>(bytes))
                        .with_model(
                            Currency::Heap,
                            ModelSpec::ceiling(DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE),
                        ),
                )
            },
        },
    ]
}

/// Direct borsh deserialization rows for the two encoded tree grammars.
#[cfg(feature = "borsh")]
fn borsh_decode_ops() -> [Op; 2] {
    [
        Op {
            name: "party_borsh_deserialize",
            prepare: |f| {
                let (bytes, _) = f.parties.clone()?;
                let n = bytes.len();
                let floors = Floors {
                    heap: heap_materializes(n),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: na(NA_TOUCH_ID_TREE),
                };
                Some(
                    Cell::new(n, floors, move || {
                        Party::try_from_slice(&bytes)
                            .expect("board-generated borsh party bytes are canonical")
                    })
                    .with_model(
                        Currency::Heap,
                        ModelSpec::ceiling(DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE),
                    ),
                )
            },
        },
        Op {
            name: "version_borsh_deserialize",
            prepare: |f| {
                let bytes = f.version.clone()?;
                let n = bytes.len();
                let version = decode_version(&bytes);
                let floors = Floors {
                    heap: heap_materializes(n),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                    touch: touch_wide_stream(&version),
                };
                Some(
                    Cell::new(n, floors, move || {
                        Version::try_from_slice(&bytes)
                            .expect("board-generated borsh version bytes are canonical")
                    })
                    .with_model(
                        Currency::Heap,
                        ModelSpec::ceiling(DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE),
                    ),
                )
            },
        },
    ]
}
