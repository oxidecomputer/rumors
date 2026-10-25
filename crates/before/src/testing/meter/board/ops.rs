//! The operation axis: the board's row table.
//!
//! Each row declares the bundle slots its signature consumes and prepares its
//! cell from them alone, never from the shape's identity, so a row reaches
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
use crate::{shape, Clock, Count, Party, Rank, Ranked, Span, Version};

use super::ceilings::{
    COMB_SCATTER_PROJECTION_HEAP_BYTES_PER_IO_BYTE, MACHINE_WORD_MAGNITUDE_BITS,
    QUERY_EVALUATION_HEAP_BYTES_PER_INPUT_BYTE, TICKS_BOARD_COUNT,
};
#[cfg(any(feature = "serde", feature = "borsh"))]
use super::ceilings::{
    DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE, RANKED_DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE,
    RANK_DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE,
};
use super::cell::{Cell, ModelSpec};
use super::currency::{Currency, Floors, Liveness};
use super::defect::{
    party_noncanonical_bytes, trailing_bytes, truncated_bytes, version_noncanonical_bytes,
};
use super::family::{decode_party, decode_version, FamilyData};
use super::floors::{
    heap_materializes, in_place, na, rejection, NA_HEAP_FORK_SHARES, NA_HEAP_IN_PLACE,
    NA_HEAP_QUERY_CLONE, NA_HEAP_REJECTION, WHY_HEAP_FORK_HALF,
};
use super::operand::version_output_bytes;
use crate::testing::meter::registry::FamilyId;

/// Arity used to exercise the public const-generic array operations.
const ARRAY_ARITY: usize = 16;

/// Why shape iteration has no representation-independent heap floor.
const NA_HEAP_SHAPE_WALK: &str =
    "paths and rises may share input storage or live inline; heap allocation is not required";

/// Why formatting a [`Count`] has no representation-independent heap floor.
const NA_HEAP_COUNT_FORMAT: &str =
    "decimal digits may be streamed directly: heap allocation is not required";

/// Why cloning a rank has no representation-independent heap floor.
const NA_HEAP_RANK_CLONE: &str = "small numerators may remain inline: cloning need not allocate";

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

/// Read a hexadecimal formatting result's byte length.
fn hex_text_output_bytes(result: &dyn std::any::Any) -> usize {
    if let Some((text, _)) = result.downcast_ref::<(String, Party)>() {
        text.len()
    } else {
        result
            .downcast_ref::<(String, Version)>()
            .expect("a hexadecimal display cell keeps its text")
            .0
            .len()
    }
}

/// Drain a shape iterator while making every yielded item observable.
fn drain_shape(items: impl IntoIterator) {
    for item in items {
        std::hint::black_box(item);
    }
}

/// The floors shared by shape iterators.
fn shape_floors() -> Floors {
    Floors {
        heap: na(NA_HEAP_SHAPE_WALK),
    }
}

/// A fork count whose stored width matches `input_bytes`.
///
/// The iterator need not drain this count. Constructing it exposes whether a
/// compact count can multiply the resident party bytes by its bit width.
fn wide_fork_count(input_bytes: usize) -> (Count, usize) {
    let shift = u64::try_from(input_bytes)
        .expect("a resident input byte length fits u64")
        .checked_mul(8)
        .expect("a resident byte buffer cannot exceed one eighth of u64::MAX");
    let count = Count(BigUint::from(1u8) << shift);
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

/// The growth units of multiplying two `bytes`-byte integers: `bytes^log2 3`,
/// an asymptotic envelope of `num-bigint`'s multiplication cost `M(n)`.
///
/// `num-bigint` multiplies by long multiplication (`n²`) while the shorter
/// factor has at most 32 digits, by Karatsuba (`n^log2 3`) up to 256 digits,
/// and by Toom-3 (`n^1.465`) beyond; a factor at least twice its partner's
/// length is split by Half-Karatsuba. Its digits are 32 bits wide on wasm32
/// and 64 bits wide on 64-bit targets, so the thresholds fall at 128 bytes
/// and 1 KiB in the fuel guest. Above the long-multiplication threshold,
/// these units grow at least as fast as `M(n)`; below it, `M(n)` grows faster
/// than its envelope, so a model built on these units holds only for operands
/// past that threshold.
fn multiplication_units(bytes: usize) -> u64 {
    let units = (bytes as f64).powf(3f64.log2()).ceil();
    // A float-to-integer cast saturates silently, so the range is checked
    // before it.
    assert!(units < u64::MAX as f64, "multiplication units fit in a u64");
    units as u64
}

/// Apply the balanced fold's `D log k` work model.
///
/// Fuel keeps its per-cell ceiling, but fits growth against the logarithmic
/// work axis so increasing arity is not mistaken for super-linearity.
fn fold_model(cell: Cell, arity: u64) -> Cell {
    let levels = (2.0 * arity as f64).log2();
    cell.with_model(Currency::Fuel, ModelSpec::scaled_trend(levels))
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

/// Read the materialized span's stored bytes from a projection result.
fn projected_span_output_bytes(result: &dyn std::any::Any) -> usize {
    let (projected, _, _) = result
        .downcast_ref::<(Span<'static>, Span<'static>, Party)>()
        .expect("the span projection cell retains its output and operands");
    span_bytes(projected)
}

/// The floors for span algebra: no allocation is forced.
fn span_algebra_floors() -> Floors {
    Floors {
        heap: na(NA_HEAP_IN_PLACE),
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

/// A prepared many-hole operation and the work its common-overlay walk permits.
struct MeasuredQuery {
    holes: Vec<Version>,
    lower: Option<Version>,
    endpoint: Version,
    input_bytes: usize,
    /// Model units of work, which count steps rather than bytes held, so
    /// they are `u64` on every target.
    work: u64,
}

impl MeasuredQuery {
    /// Package a query fixture and calculate its work allowance: a step per
    /// input byte, and one per hole at each overlay interval.
    ///
    /// `overlay_intervals` is exact for a purpose-built fixture. The organic
    /// populations use their coarser, always-valid `i = O(n)` bound.
    fn new(
        holes: Vec<Version>,
        lower: Option<Version>,
        endpoint: Version,
        overlay_intervals: Option<usize>,
    ) -> MeasuredQuery {
        let input_bytes = holes
            .iter()
            .map(|hole| hole.as_bytes().len())
            .sum::<usize>()
            .checked_add(endpoint.as_bytes().len())
            .and_then(|bytes| bytes.checked_add(lower.as_ref().map_or(0, |lo| lo.as_bytes().len())))
            .expect("allocated query operands have a representable size");
        let intervals = overlay_intervals.unwrap_or(input_bytes) as u64;
        let work = (holes.len() as u64)
            .checked_mul(intervals)
            .and_then(|updates| (input_bytes as u64).checked_add(updates))
            .expect("a work bound over allocated query operands fits in a u64");
        MeasuredQuery {
            holes,
            lower,
            endpoint,
            input_bytes,
            work,
        }
    }
}

/// Prepare the organic population's membership witness.
fn query_membership_operands(f: &FamilyData) -> Option<MeasuredQuery> {
    let operands = QueryOperands::build(f)?;
    Some(MeasuredQuery::new(
        operands.up_holes,
        None,
        operands.up_probe,
        None,
    ))
}

/// Prepare the organic population's coverage witness.
fn query_coverage_operands(f: &FamilyData) -> Option<MeasuredQuery> {
    let operands = QueryOperands::build(f)?;
    Some(MeasuredQuery::new(
        operands.down_holes,
        Some(Version::new()),
        operands.down_hi,
        None,
    ))
}

/// Decode the correlated fixture's holes for one independently measured row.
fn wide_query_holes(f: &FamilyData) -> Option<Vec<Version>> {
    Some(
        f.query_probe
            .as_ref()?
            .holes
            .iter()
            .map(|bytes| decode_version(bytes))
            .collect(),
    )
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
/// meaningful operand. The `coverage` module accounts for the rest.
#[allow(clippy::too_many_lines)]
pub(super) fn ops() -> Vec<Op> {
    let operations = vec![
        // ── Version ────────────────────────────────────────────────────
        Op {
            name: "version_decode",
            prepare: |f| {
                let bytes = f.version.clone()?;
                let floors = Floors {
                    heap: heap_materializes(bytes.len()),
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
                };
                Some(Cell::new(n, floors, move || (v.encode(), v)))
            },
        },
        Op {
            name: "version_display",
            prepare: |f| {
                let (version, n) = f.version()?;
                Some(Cell::io(
                    n,
                    Floors {
                        heap: heap_materializes(n * 2),
                    },
                    hex_text_output_bytes,
                    move || (version.to_string(), version),
                ))
            },
        },
        Op {
            name: "version_parse",
            prepare: |f| {
                let (version, n) = f.version()?;
                let text = version.to_string();
                Some(Cell::new(
                    text.len(),
                    Floors {
                        heap: heap_materializes(n),
                    },
                    move || {
                        text.parse::<Version>()
                            .expect("displayed version text is canonical")
                    },
                ))
            },
        },
        Op {
            name: "version_cmp",
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                let floors = in_place();
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
                };
                Some(Cell::new(n, floors, move || (v == w, v, w)))
            },
        },
        Op {
            name: "version_concurrent",
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                let floors = in_place();
                Some(Cell::new(n, floors, move || (v.concurrent(&w), v, w)))
            },
        },
        Op {
            name: "version_join",
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                Some(Cell::new(n, in_place(), move || (&v | &w, v, w)))
            },
        },
        Op {
            name: "version_join_assign",
            prepare: |f| {
                let (mut v, w, n) = f.version_pair()?;
                Some(Cell::new(n, in_place(), move || {
                    v |= &w;
                    (v, w)
                }))
            },
        },
        Op {
            name: "version_meet",
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                Some(Cell::new(n, in_place(), move || (&v & &w, v, w)))
            },
        },
        Op {
            name: "version_meet_assign",
            prepare: |f| {
                let (mut v, w, n) = f.version_pair()?;
                Some(Cell::new(n, in_place(), move || {
                    v &= &w;
                    (v, w)
                }))
            },
        },
        Op {
            name: "version_span",
            prepare: |f| {
                // The fused pair hull: one sweep feeds both endpoints.
                let (v, w, n) = f.version_pair()?;
                Some(Cell::new(n, in_place(), move || (v.span(&w), v, w)))
            },
        },
        Op {
            name: "span_new",
            prepare: |f| {
                let (v, w, _) = f.version_pair()?;
                let lo = &v & &w;
                let hi = &v | &w;
                let n = lo.as_bytes().len() + hi.as_bytes().len();
                let floors = in_place();
                Some(Cell::new(n, floors, move || {
                    Span::new(lo, hi).expect("a meet precedes the corresponding join")
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
                // endpoint, denominated by
                // the span's own size, which is exactly the output.
                let (v, w, _) = f.version_pair()?;
                let span = v.span(&w);
                let n = span.encode().len();
                let floors = Floors {
                    heap: heap_materializes(n),
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
                // while validating dominance against the first; both
                // endpoints materialize.
                let (v, w, _) = f.version_pair()?;
                let span = v.span(&w);
                let bytes = span.encode();
                let n = bytes.len();
                let floors = Floors {
                    heap: heap_materializes(n),
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
                    let floors = in_place();
                    return Some(Cell::new(n, floors, move || {
                        v.tick(&party);
                        (v, party)
                    }));
                }
                let (mut v, n) = f.version()?;
                let party = Party::seed();
                Some(Cell::new(n + 1, in_place(), move || {
                    v.tick(&party);
                    v
                }))
            },
        },
        Op {
            name: "version_ticks",
            prepare: |f| {
                // The fused multi-tick at a fixed count: the same walk
                // and splice as the tick rows, with the count's gamma
                // width the only n-dependence, so the cell must scale
                // exactly as the tick cell above it (the flatness rows
                // of tests/meter.rs pin the n axis; this cell pins the
                // input axis).
                if let Some((mut v, party, n)) = f.cross() {
                    let floors = in_place();
                    return Some(Cell::new(n, floors, move || {
                        v.ticks(&party, TICKS_BOARD_COUNT);
                        (v, party)
                    }));
                }
                let (mut v, n) = f.version()?;
                let party = Party::seed();
                Some(Cell::new(n + 1, in_place(), move || {
                    v.ticks(&party, TICKS_BOARD_COUNT);
                    v
                }))
            },
        },
        Op {
            name: "version_tick_adv_party",
            prepare: |f| {
                let (a, _, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(a, _)| a.len())?;
                let mut v = Version::new();
                Some(Cell::new(n + 1, in_place(), move || {
                    v.tick(&a);
                    (v, a)
                }))
            },
        },
        Op {
            name: "version_rank",
            prepare: |f| {
                let (v, n) = f.version()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                };
                Some(Cell::new(n, floors, move || (v.rank(), v)))
            },
        },
        Op {
            name: "rank_clone",
            prepare: |f| {
                let (rank, _) = f.rank_pair.clone()?;
                let n = rank_content_bytes(&rank);
                let floors = Floors {
                    heap: na(NA_HEAP_RANK_CLONE),
                };
                Some(Cell::new(n, floors, move || (rank.clone(), rank)))
            },
        },
        Op {
            name: "rank_cmp",
            prepare: |f| {
                let (a, b) = f.rank_pair.clone()?;
                let n = rank_pair_bytes(&a, &b);
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                };
                Some(Cell::new(n, floors, move || (a.cmp(&b), a, b)))
            },
        },
        Op {
            name: "rank_add",
            prepare: |f| {
                let (a, b) = f.rank_pair.clone()?;
                let n = rank_pair_bytes(&a, &b);
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                };
                Some(Cell::new(n, floors, move || (&a + &b, a, b)))
            },
        },
        Op {
            name: "rank_checked_sub",
            prepare: |f| {
                let (a, b) = f.rank_pair.clone()?;
                let (larger, smaller) = if a >= b { (a, b) } else { (b, a) };
                let n = rank_pair_bytes(&larger, &smaller);
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                };
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
                // stored byte of its measure operand, so both the wide value
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
                };
                Some(Cell::new(n, floors, move || {
                    std::iter::once(a).chain(ones).sum::<Rank>()
                }))
            },
        },
        Op {
            name: "rank_sum_ascending",
            prepare: |f| {
                // The family-derived rank, then the unit fractions 2^-1
                // through 2^-k, `k = 2(⌊√w⌋ + 1)` for the rank's content
                // width `w`. Each fraction's exponent exceeds the one before.
                // Where the family rank's exponent is smaller than the
                // first fraction's (an integer rank), that fraction makes the
                // running sum shift by its whole held span, and every later
                // fraction then lands below the run the shift left: the
                // shift costs the held width once, where a shift by the
                // exponent gap alone would move it once per fraction, `k · w`
                // work on `w + k²/2` bits of input. Where the family rank's
                // exponent is larger, the fractions land inside its span. The
                // denominator is the summands' total value content.
                let (a, _) = f.rank_pair.clone()?;
                let k = 2 * (a.content_bits().isqrt() + 1);
                let fractions: Vec<Rank> =
                    (1..=k).map(|j| Rank::from_raw(BigUint::ONE, j)).collect();
                let n = rank_content_bytes(&a)
                    + fractions.iter().map(rank_content_bytes).sum::<usize>();
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                };
                Some(Cell::new(n, floors, move || {
                    std::iter::once(a).chain(fractions).sum::<Rank>()
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
                // rank denomination), with the output bound
                // asserted here: a canonical encoding is at most
                // `9⁄8 · ‖r‖ + O(log ‖r‖)` bits, so padding the output
                // side of the denominator trips the run instead of
                // greening the cell. Derivation of the bound: the
                // stream is `2ρ + w` header/mantissa bits plus
                // `9 · ⌈exp/8⌉ + 1` fraction bits; `w ≤ bits(⌊r⌋) + 1`
                // and `exp` each sit inside `content = bits(num) + exp`
                // (so the 9⁄8 shows up as `content/8`), and `2ρ + 11`
                // stays under 64 for any integral part narrower than
                // 2²⁶ bits, orders beyond every committed family.
                let (a, _) = f.rank_pair.clone()?;
                let content = a.content_bits();
                let encoded_len = a.encode().len();
                assert!(
                    (encoded_len as u64) * 8 <= content + content / 8 + 64,
                    "output bound: a canonical rank encoding is at most \
                     9/8 content + O(log) bits"
                );
                let floors = Floors {
                    heap: heap_materializes(encoded_len),
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
                let floors = Floors {
                    heap: heap_materializes(output_bytes),
                };
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
                let floors = Floors {
                    heap: heap_materializes(output_bytes),
                };
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
                let floors = Floors {
                    heap: heap_materializes(rank_numerator_bytes(&rank)),
                };
                Some(Cell::new(text.len(), floors, move || {
                    text.parse::<Rank>()
                        .expect("a displayed rank is canonical text")
                }))
            },
        },
        Op {
            name: "count_clone",
            prepare: |f| {
                let count = counts_for_family(f)?
                    .into_iter()
                    .max_by_key(|count| count.0.bits())?;
                let n = count_input_bytes(&count);
                Some(Cell::new(
                    n,
                    Floors {
                        heap: count_heap_floor(count_value_bytes(&count)),
                    },
                    move || (count.clone(), count),
                ))
            },
        },
        Op {
            name: "count_add",
            prepare: |f| {
                let mut counts = counts_for_family(f)?.into_iter();
                let (a, b) = (counts.next()?, counts.next()?);
                let n = count_input_bytes(&a) + count_input_bytes(&b);
                let output_bytes = count_value_bytes(&a).max(count_value_bytes(&b));
                Some(Cell::new(
                    n,
                    Floors {
                        heap: count_heap_floor(output_bytes),
                    },
                    move || (&a + &b, a, b),
                ))
            },
        },
        Op {
            name: "count_sum",
            prepare: |f| {
                let counts = counts_for_family(f)?;
                let n = counts.iter().map(count_input_bytes).sum();
                let output_bytes = counts.iter().map(count_value_bytes).max()?;
                Some(Cell::new(
                    n,
                    Floors {
                        heap: count_heap_floor(output_bytes),
                    },
                    move || {
                        let sum: Count = counts.iter().sum();
                        (sum, counts)
                    },
                ))
            },
        },
        Op {
            name: "count_display",
            prepare: |f| {
                let count = counts_for_family(f)?
                    .into_iter()
                    .max_by_key(|count| count.0.bits())?;
                let n = count_input_bytes(&count);
                // Decimal rendering is documented superlinear but
                // subquadratic: `num-bigint` renders by divide and conquer
                // over Burnikel-Ziegler division, `O(M(n) log n)`. Fuel fits
                // growth against `M(n)`'s envelope, so a quadratic regression
                // reads red.
                Some(
                    Cell::io(
                        n,
                        Floors {
                            heap: na(NA_HEAP_COUNT_FORMAT),
                        },
                        count_text_output_bytes,
                        move || (formatted_len(format_args!("{count}")), count),
                    )
                    .with_model(Currency::Fuel, ModelSpec::trend(multiplication_units(n))),
                )
            },
        },
        Op {
            name: "count_parse",
            prepare: |f| {
                let count = counts_for_family(f)?
                    .into_iter()
                    .max_by_key(|count| count.0.bits())?;
                let text = count.to_string();
                // Parsing `d` digits takes the documented `O(d²)` time, so
                // fuel fits growth against `d²`.
                let digits = text.len() as u64;
                Some(
                    Cell::new(
                        text.len(),
                        Floors {
                            heap: count_heap_floor(count_value_bytes(&count)),
                        },
                        move || {
                            text.parse::<Count>()
                                .expect("displayed count text is canonical")
                        },
                    )
                    .with_model(Currency::Fuel, ModelSpec::trend(digits * digits)),
                )
            },
        },
        Op {
            name: "version_distance",
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
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
                };
                Some(Cell::new(n, floors, move || (v.lag(&w), v, w)))
            },
        },
        Op {
            name: "ranked_cmp",
            prepare: |f| {
                // The fused rank comparison: the distance/lag co-sweep
                // with constant orientation, answering a word-scale verdict.
                let (v, w, n) = f.version_pair()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
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
                // The composite key emission: the rank fold plus the
                // mandatory output (rank stream, then one copy of the
                // version's encoded bytes).
                // Input-denominated: the provenance pin bounds the
                // rank component within the encoded input plus one
                // byte, and the version tail is exactly the encoded
                // input again (asserted here, so the bound is enforced
                // at every family and scale), which makes input bytes
                // the stricter denominator (the codec rows'
                // rule) and lets the flat-denominator shape's content
                // exponent govern exactly as on the version_rank cell
                // this row extends.
                let (v, n) = f.version()?;
                let encoded_len = Ranked::from(&v).encode().len();
                assert!(
                    encoded_len <= 2 * n + 1,
                    "output bound: a composite ranked key is the rank emission \
                     (within the encoded input plus one byte) plus the version's \
                     encoded bytes"
                );
                let floors = Floors {
                    heap: heap_materializes(encoded_len),
                };
                Some(Cell::new(n, floors, move || (Ranked::from(&v).encode(), v)))
            },
        },
        Op {
            name: "ranked_encode_rank",
            prepare: |f| {
                // The rank-to-bytes emission: the rank fold plus the
                // mandatory output. Input-denominated:
                // the provenance pin bounds the output within the
                // encoded input (asserted here, so the bound is
                // enforced at every family and scale), which makes
                // input bytes the stricter denominator (the
                // codec rows' rule) and lets the flat-denominator
                // shape's content exponent govern exactly as on the
                // version_rank cell this row extends.
                let (v, n) = f.version()?;
                let encoded_len = Ranked::from(&v).encode_rank().len();
                assert!(
                    encoded_len <= n + 1,
                    "output bound: a version-derived rank encodes within its \
                     version's encoded bytes"
                );
                let floors = Floors {
                    heap: heap_materializes(encoded_len),
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
                // version, and the owned version materializes.
                let (v, n) = f.version()?;
                let bytes = Ranked::from(&v).encode();
                let floors = Floors {
                    heap: heap_materializes(n),
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
                // accumulators, computing in place.
                let (v, n) = f.version()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
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
                let rest = versions.split_off(1);
                let receiver = versions.pop()?;
                Some(fold_model(
                    Cell::new(n, in_place(), move || receiver.join_all(rest)),
                    arity,
                ))
            },
        },
        Op {
            name: "version_meet_all",
            prepare: |f| {
                // The meet fold: the join fold's balanced reduction over
                // the meet emitter, so the same declared fold model applies.
                let (versions, _) = f.population.as_ref()?;
                let n = versions.iter().map(Vec::len).sum();
                let mut versions: Vec<Version> =
                    versions.iter().map(|b| decode_version(b)).collect();
                let arity = versions.len() as u64;
                let rest = versions.split_off(1);
                let receiver = versions.pop()?;
                Some(fold_model(
                    Cell::new(n, in_place(), move || receiver.meet_all(rest)),
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
                // declared fold model as the single-direction fold rows
                // applies.
                let (versions, _) = f.population.as_ref()?;
                let n = versions.iter().map(Vec::len).sum();
                let versions: Vec<Version> = versions.iter().map(|b| decode_version(b)).collect();
                let arity = versions.len() as u64;
                Some(fold_model(
                    Cell::new(n, in_place(), move || {
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
                        in_place(),
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
                    return Some(Cell::new(n, in_place(), move || {
                        ((&v / &p).to_version(), v, p)
                    }));
                }
                // Small (half-interval) party × adversarial version.
                if f.version.is_some() {
                    let (v, n) = f.version()?;
                    let half = Party::seed().fork();
                    return Some(Cell::new(n + 1, in_place(), move || {
                        ((&v / &half).to_version(), v, half)
                    }));
                }
                // Adversarial party × small version.
                let (a, _, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(a, _)| a.len())?;
                let mut v = Version::new();
                v.tick(&a);
                let input = n + v.encode().len();
                Some(Cell::new(input, in_place(), move || {
                    ((&v / &a).to_version(), v, a)
                }))
            },
        },
        Op {
            name: "own_span_to_span",
            prepare: |f| {
                let (span, party, input_bytes, output_dominated) =
                    if let Some((version, party, n)) = f.cross() {
                        (Span::at(version), party, n, f.output_dominated)
                    } else if f.version.is_some() {
                        let (v, w, n) = f.version_pair()?;
                        let span = v.span(&w);
                        let party = Party::seed().fork();
                        let input_bytes = n + party.as_bytes().len();
                        (span, party, input_bytes, false)
                    } else {
                        let (party, _, n) = f.party_pair()?;
                        let mut version = Version::new();
                        version.tick(&party);
                        let span = Span::at(version);
                        let input_bytes = n + span.lo().as_bytes().len();
                        (span, party, input_bytes, false)
                    };
                let cell = Cell::io(
                    input_bytes,
                    in_place(),
                    projected_span_output_bytes,
                    move || {
                        let projected = (&span / &party).to_span();
                        (projected, span, party)
                    },
                );
                Some(
                    if output_dominated && matches!(f.kind, FamilyId::CombScatter) {
                        cell.with_model(
                            Currency::Heap,
                            ModelSpec::ceiling(COMB_SCATTER_PROJECTION_HEAP_BYTES_PER_IO_BYTE),
                        )
                    } else {
                        cell
                    },
                )
            },
        },
        Op {
            name: "own_version_cmp",
            prepare: |f| {
                // The fused three-stream comparison `(v / p) ⋚ w`: lazy at
                // every spelling, so the cell stays input-denominated on
                // every shape, the output-domination crosses included,
                // which is the point: comparing a projection never pays
                // its materialization.
                let (v, p, w, n) = if let Some(masked) = &f.masked_against {
                    (
                        Version::decode(&masked.projected.version[..]).unwrap_or_else(|error| {
                            panic!("{} generated an invalid masked version: {error}", f.name)
                        }),
                        Party::decode(&masked.projected.mask[..]).unwrap_or_else(|error| {
                            panic!("{} generated an invalid mask: {error}", f.name)
                        }),
                        Version::decode(&masked.other[..]).unwrap_or_else(|error| {
                            panic!(
                                "{} generated an invalid comparison version: {error}",
                                f.name
                            )
                        }),
                        masked.projected.version.len()
                            + masked.projected.mask.len()
                            + masked.other.len(),
                    )
                } else if let Some((v, p, np)) = f.cross() {
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
                Some(Cell::new(n, in_place(), move || {
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
                let (v, a, w, b, n) = if let Some(masked) = &f.masked_pair {
                    (
                        Version::decode(&masked.left.version[..]).unwrap_or_else(|error| {
                            panic!(
                                "{} generated an invalid first masked version: {error}",
                                f.name
                            )
                        }),
                        Party::decode(&masked.left.mask[..]).unwrap_or_else(|error| {
                            panic!("{} generated an invalid first mask: {error}", f.name)
                        }),
                        Version::decode(&masked.right.version[..]).unwrap_or_else(|error| {
                            panic!(
                                "{} generated an invalid second masked version: {error}",
                                f.name
                            )
                        }),
                        Party::decode(&masked.right.mask[..]).unwrap_or_else(|error| {
                            panic!("{} generated an invalid second mask: {error}", f.name)
                        }),
                        masked.left.version.len()
                            + masked.left.mask.len()
                            + masked.right.version.len()
                            + masked.right.mask.len(),
                    )
                } else {
                    match (f.parties.is_some(), f.version.is_some()) {
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
                    }
                };
                let floors = in_place();
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
                let floors = shape_floors();
                Some(Cell::new(n, floors, move || {
                    drain_shape(version.shape());
                }))
            },
        },
        Op {
            name: "shape_combine_pair",
            prepare: |f| {
                let (left, right, n) = f.version_pair()?;
                let floors = shape_floors();
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
                let floors = shape_floors();
                Some(Cell::new(n, floors, move || {
                    drain_shape(shape::combine(versions.each_ref()));
                }))
            },
        },
        Op {
            name: "causally_contains",
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                let floors = in_place();
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
        // and forces full examination of all three streams. The
        // precedence row's probe re-decodes the meet instead: its watched
        // directions are the mirror pair (probe at-or-below each bound),
        // so only a probe at the span's own floor confirms both and
        // forces the full sweep; an in-hull probe above the meet refutes
        // `probe <= lo` and legitimately stops reading the start stream.
        Op {
            name: "span_place",
            prepare: |f| {
                let (v, w, _) = f.version_pair()?;
                let span = v.span(&w);
                let probe = decode_version(&v.encode());
                let n = span.lo().encode().len() + span.hi().encode().len() + probe.encode().len();
                Some(Cell::new(n, in_place(), move || {
                    let verdict = span.place(&probe);
                    (verdict, span, probe)
                }))
            },
        },
        Op {
            name: "span_dominance",
            prepare: |f| {
                let (v, w, _) = f.version_pair()?;
                let span = v.span(&w);
                let probe = decode_version(&v.encode());
                let n = span.lo().encode().len() + span.hi().encode().len() + probe.encode().len();
                Some(Cell::new(n, in_place(), move || {
                    let verdict = span.dominance(&probe);
                    (verdict, span, probe)
                }))
            },
        },
        Op {
            name: "span_precedence",
            prepare: |f| {
                let (v, w, _) = f.version_pair()?;
                let span = v.span(&w);
                let probe = decode_version(&span.lo().encode());
                let n = span.lo().encode().len() + span.hi().encode().len() + probe.encode().len();
                Some(Cell::new(n, in_place(), move || {
                    let verdict = span.precedence(&probe);
                    (verdict, span, probe)
                }))
            },
        },
        Op {
            name: "span_contains",
            prepare: |f| {
                let (v, w, _) = f.version_pair()?;
                let span = v.span(&w);
                let probe = decode_version(&v.encode());
                let n = span.lo().encode().len() + span.hi().encode().len() + probe.encode().len();
                Some(Cell::new(n, in_place(), move || {
                    let verdict = span.contains(&probe);
                    (verdict, span, probe)
                }))
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
                Some(
                    Cell::new(n, in_place(), move || {
                        let verdict = {
                            let query = causally::after(&lo) & causally::before(&hi);
                            query.contains(&probe)
                        };
                        (verdict, lo, hi, probe)
                    })
                    .with_model(
                        Currency::Heap,
                        ModelSpec::ceiling(QUERY_EVALUATION_HEAP_BYTES_PER_INPUT_BYTE),
                    ),
                )
            },
        },
        Op {
            name: "query_coverage",
            prepare: |f| {
                // The anti-entropy delta classified against the pair's
                // hull: the fused two-probe walk over hole and ceiling,
                // plus the clamp legs on verdicts the walk alone cannot
                // close: the tree walk's per-subtree verdict.
                let (v, w, _) = f.version_pair()?;
                let lo = &v & &w;
                let hi = &v | &w;
                let span = lo.span(&hi);
                let n = v.encode().len()
                    + w.encode().len()
                    + span.lo().encode().len()
                    + span.hi().encode().len();
                Some(
                    Cell::new(n, in_place(), move || {
                        let verdict = {
                            let query = causally::delta(&v, &w);
                            query.coverage(span.reborrow())
                        };
                        (verdict, span, v, w)
                    })
                    .with_model(
                        Currency::Heap,
                        ModelSpec::ceiling(QUERY_EVALUATION_HEAP_BYTES_PER_INPUT_BYTE),
                    ),
                )
            },
        },
        Op {
            name: "query_coverage_neutral",
            prepare: |f| {
                // The crossed segment query `after(w) & before(v)` over the
                // pair's hull `[v ∧ w, v ∨ w]`. The floor admits the hull's top
                // and the ceiling admits its bottom, so the fused walk never
                // returns `Empty`; it returns `Full` exactly when `w <= v`,
                // where the hull is the query's own segment. On every other
                // pair, `v` and its ticked successor included, the walk returns
                // `Partial` and the clamp refinement finds the clamp crossed.
                // The query holds no holes, so the refinement decides from its
                // bounds and needs no clamped endpoint. Building one would cost
                // a new version here, because the top clamp `(v ∨ w) ∧ v` has
                // distinct, nonempty operands; the `query_coverage` row, whose
                // hole needs a clamped endpoint, cannot show that cost.
                let (v, w, _) = f.version_pair()?;
                let span = v.span(&w);
                let n = v.encode().len()
                    + w.encode().len()
                    + span.lo().encode().len()
                    + span.hi().encode().len();
                Some(
                    Cell::new(n, in_place(), move || {
                        let verdict = {
                            let query = causally::after(&w) & causally::before(&v);
                            query.coverage(span.reborrow())
                        };
                        (verdict, span, v, w)
                    })
                    .with_model(
                        Currency::Heap,
                        ModelSpec::ceiling(QUERY_EVALUATION_HEAP_BYTES_PER_INPUT_BYTE),
                    ),
                )
            },
        },
        Op {
            name: "query_contains_many",
            prepare: |f| {
                let operands = query_membership_operands(f)?;
                let n = operands.input_bytes;
                let work = operands.work;
                let query = Query::<Up>::from_inclusive_holes(operands.holes);
                let probe = operands.endpoint;
                Some(
                    Cell::new(n, in_place(), move || {
                        (query.contains(&probe), query, probe)
                    })
                    .with_model(
                        Currency::Heap,
                        ModelSpec::ceiling(QUERY_EVALUATION_HEAP_BYTES_PER_INPUT_BYTE),
                    )
                    // The work units lie within the documented `O(k·n)` time
                    // bound.
                    .with_model(Currency::Fuel, ModelSpec::work(work)),
                )
            },
        },
        Op {
            name: "query_coverage_many",
            prepare: |f| {
                let operands = query_coverage_operands(f)?;
                let lo = operands
                    .lower
                    .expect("coverage operands include a lower endpoint");
                let n = operands.input_bytes;
                let work = operands.work;
                let query = Query::<Down>::from_inclusive_holes(operands.holes);
                let span = lo.span(&operands.endpoint);
                Some(
                    Cell::new(n, in_place(), move || {
                        (query.coverage(span.reborrow()), query, span)
                    })
                    .with_model(
                        Currency::Heap,
                        ModelSpec::ceiling(QUERY_EVALUATION_HEAP_BYTES_PER_INPUT_BYTE),
                    )
                    // The work units lie within the documented `O(k·n)` time
                    // bound.
                    .with_model(Currency::Fuel, ModelSpec::work(work)),
                )
            },
        },
        Op {
            name: "query_contains_wide_up",
            prepare: |f| {
                let fixture = f.query_probe.as_ref()?;
                let operands = MeasuredQuery::new(
                    wide_query_holes(f)?,
                    None,
                    decode_version(&fixture.membership),
                    Some(3),
                );
                let n = operands.input_bytes;
                let query = Query::<Up>::from_inclusive_holes(operands.holes);
                let probe = operands.endpoint;
                Some(
                    Cell::new(n, in_place(), move || {
                        (query.contains(&probe), query, probe)
                    })
                    .with_model(
                        Currency::Heap,
                        ModelSpec::ceiling(QUERY_EVALUATION_HEAP_BYTES_PER_INPUT_BYTE),
                    ),
                )
            },
        },
        Op {
            name: "query_contains_wide_down",
            prepare: |f| {
                let fixture = f.query_probe.as_ref()?;
                let operands = MeasuredQuery::new(
                    wide_query_holes(f)?,
                    None,
                    decode_version(&fixture.coverage.0),
                    Some(3),
                );
                let n = operands.input_bytes;
                let query = Query::<Down>::from_inclusive_holes(operands.holes);
                let probe = operands.endpoint;
                Some(
                    Cell::new(n, in_place(), move || {
                        (query.contains(&probe), query, probe)
                    })
                    .with_model(
                        Currency::Heap,
                        ModelSpec::ceiling(QUERY_EVALUATION_HEAP_BYTES_PER_INPUT_BYTE),
                    ),
                )
            },
        },
        Op {
            name: "query_coverage_wide_down",
            prepare: |f| {
                let fixture = f.query_probe.as_ref()?;
                let operands = MeasuredQuery::new(
                    wide_query_holes(f)?,
                    Some(decode_version(&fixture.coverage.0)),
                    decode_version(&fixture.coverage.1),
                    Some(3),
                );
                let lo = operands
                    .lower
                    .expect("coverage operands include a lower endpoint");
                let n = operands.input_bytes;
                let query = Query::<Down>::from_inclusive_holes(operands.holes);
                let span = lo.span(&operands.endpoint);
                Some(
                    Cell::new(n, in_place(), move || {
                        (query.coverage(span.reborrow()), query, span)
                    })
                    .with_model(
                        Currency::Heap,
                        ModelSpec::ceiling(QUERY_EVALUATION_HEAP_BYTES_PER_INPUT_BYTE),
                    ),
                )
            },
        },
        Op {
            name: "query_coverage_wide_up",
            prepare: |f| {
                let fixture = f.query_probe.as_ref()?;
                let operands = MeasuredQuery::new(
                    wide_query_holes(f)?,
                    Some(Version::new()),
                    decode_version(&fixture.membership),
                    Some(3),
                );
                let lo = operands
                    .lower
                    .expect("coverage operands include a lower endpoint");
                let n = operands.input_bytes;
                let query = Query::<Up>::from_inclusive_holes(operands.holes);
                let span = lo.span(&operands.endpoint);
                Some(
                    Cell::new(n, in_place(), move || {
                        (query.coverage(span.reborrow()), query, span)
                    })
                    .with_model(
                        Currency::Heap,
                        ModelSpec::ceiling(QUERY_EVALUATION_HEAP_BYTES_PER_INPUT_BYTE),
                    ),
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
                let work = (left_bytes as u64)
                    .checked_mul(right_holes.len() as u64)
                    .and_then(|left| {
                        (right_bytes as u64)
                            .checked_mul(left_holes.len() as u64)
                            .and_then(|right| left.checked_add(right))
                    })
                    .expect("a work bound over allocated query operands fits in a u64");
                let left = Query::<Down>::from_inclusive_holes(left_holes);
                let right = Query::<Down>::from_inclusive_holes(right_holes);
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                };
                Some(
                    Cell::new(n, floors, move || left & right)
                        // The work units lie within the documented
                        // `O(n(k + m + 1))` time bound.
                        .with_model(Currency::Fuel, ModelSpec::work(work)),
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
                };
                Some(Cell::new(n, floors, move || (a.encode(), a)))
            },
        },
        Op {
            name: "party_display",
            prepare: |f| {
                let (party, _, _) = f.party_pair()?;
                let n = party.as_bytes().len();
                Some(Cell::io(
                    n,
                    Floors {
                        heap: heap_materializes(n * 2),
                    },
                    hex_text_output_bytes,
                    move || (party.to_string(), party),
                ))
            },
        },
        Op {
            name: "party_parse",
            prepare: |f| {
                let (party, _, _) = f.party_pair()?;
                let n = party.as_bytes().len();
                let text = party.to_string();
                Some(Cell::new(
                    text.len(),
                    Floors {
                        heap: heap_materializes(n),
                    },
                    move || {
                        text.parse::<Party>()
                            .expect("displayed party text is canonical")
                    },
                ))
            },
        },
        Op {
            name: "party_fork",
            prepare: |f| {
                let (mut a, _, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(a, _)| a.len())?;
                // Fork builds both halves, so the child's own stored bytes
                // floor the heap (probed on a fresh decode, outside
                // measurement); the generic in-place NA would misstate
                // what fork does.
                let child_bytes = {
                    let bytes = f.parties.as_ref().map(|(a, _)| a)?;
                    let mut probe = decode_party(bytes);
                    u64::try_from(probe.fork().as_bytes().len())
                        .expect("a resident party's byte count fits u64")
                };
                let floors = Floors {
                    heap: Liveness::Floor {
                        min: child_bytes,
                        why: WHY_HEAP_FORK_HALF,
                    },
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
                // The caller chooses whether to collect the iterator. Reserve
                // the board's retained-result slots outside measurement.
                let mut children = Vec::with_capacity(child_count);
                // A borrowing drain may read the stored party once per child;
                // count bookkeeping is bounded by the count's own width. This
                // is the public `k (D + log k)` bound expressed in byte units,
                // and it bounds the drain's time, which fuel fits against.
                let work = (child_count as u64)
                    .checked_mul(input_bytes as u64)
                    .expect("the board's fork work model fits in a u64");
                let floors = Floors {
                    // The result retains every share. Even when canonical
                    // collapses shorten them, at least one encoded byte per
                    // share must be materialized.
                    heap: heap_materializes(arity),
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
                            children.extend(party.forks(child_count));
                            (party, children)
                        },
                    )
                    .with_model(Currency::Fuel, ModelSpec::work(work)),
                )
            },
        },
        Op {
            name: "party_forks_count",
            prepare: |f| {
                // The family's party forked into a count as wide as its
                // version's encoded bits: the count's width follows the
                // version side rather than the party's own size, so the fork
                // plan's arithmetic on a wide count meets parties of shapes
                // independent of it. The count is a power of two, so the
                // plan's extra-share test compares every bit of its index
                // before it decides.
                let (mut party, _, _) = f.party_pair()?;
                let party_bytes = f.parties.as_ref().map(|(party, _)| party.len())?;
                let (_, version_bytes) = f.version()?;
                let (count, count_bytes) = wide_fork_count(version_bytes);
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
                };
                // A count `w` bits wide gives the first share a path about
                // `w` levels deep, and the residual party keeps the sibling
                // of each level, so the required output is several times the
                // count's own bytes: the row is I/O-denominated. Charged to
                // its input alone, it would read about 17.6 heap bytes per
                // input byte, above the board-wide 15. The `party_forks` row
                // meters the same mechanism input-denominated, with a count
                // only as wide as the party, so its output stays near its
                // input.
                Some(Cell::io(
                    party_bytes + count_bytes,
                    floors,
                    |result| {
                        let (residual, first) = result
                            .downcast_ref::<(Party, Option<Party>)>()
                            .expect("the counted party-forks cell keeps its residual and share");
                        residual.as_bytes().len()
                            + first.as_ref().map_or(0, |share| share.as_bytes().len())
                    },
                    move || {
                        let first = {
                            let mut forks = party.forks(count);
                            forks.next()
                        };
                        (party, first)
                    },
                ))
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
                let floors = shape_floors();
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
                    let floors = in_place();
                    let mut clock = Clock::from_parts(p, v);
                    return Some(Cell::new(n, floors, move || {
                        clock.tick();
                        clock
                    }));
                }
                let (mut clock, n) = f.clock()?;
                Some(Cell::new(n, in_place(), move || {
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
                // The collection is instrument storage, not an allocation
                // made by Clock::forks. Retain the yielded clocks in slots
                // reserved before the heap baseline.
                let mut children = Vec::with_capacity(child_count);
                // Clock delegates partitioning to Party. Its shared empty
                // Version is neither read nor copied, so the work model is
                // the same `k (D + log k)` bound over party and count bytes,
                // which bounds the time fuel fits against.
                let work = (child_count as u64)
                    .checked_mul((party_bytes + count_bytes) as u64)
                    .expect("the board's fork work model fits in a u64");
                let mut clock = Clock::from_parts(party, Version::new());
                let floors = Floors {
                    heap: heap_materializes(arity),
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
                            children.extend(clock.forks(child_count));
                            (clock, children)
                        },
                    )
                    .with_model(Currency::Fuel, ModelSpec::work(work)),
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
                Some(Cell::new(n, in_place(), move || {
                    let joined = a.join(b).is_ok();
                    (joined, a)
                }))
            },
        },
        Op {
            name: "clock_sync",
            prepare: |f| {
                let (mut a, mut b, n) = f.clock_pair()?;
                let floors = in_place();
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
                    return Some(Cell::new(n + 2, in_place(), move || {
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
                Some(Cell::new(n + 2, in_place(), move || {
                    clock.recv(&msg);
                    (clock, msg)
                }))
            },
        },
        Op {
            name: "clock_recv_all",
            prepare: |f| {
                let (version_bytes, party_bytes) = f.population.as_ref()?;
                let versions: Vec<Version> = version_bytes
                    .iter()
                    .map(|bytes| decode_version(bytes))
                    .collect();
                let arity = versions.len() as u64;
                let mut versions = versions.into_iter();
                let receiver_version = versions.next()?;
                let messages: Vec<Version> = versions.collect();
                let party = decode_party(party_bytes.first()?);
                let input_bytes = receiver_version.as_bytes().len()
                    + messages
                        .iter()
                        .map(|version| version.as_bytes().len())
                        .sum::<usize>()
                    + party.as_bytes().len();
                let mut clock = Clock::from_parts(party, receiver_version);
                Some(fold_model(
                    Cell::new(input_bytes, in_place(), move || {
                        clock.recv_all(&messages);
                        (clock, messages)
                    }),
                    arity,
                ))
            },
        },
        Op {
            name: "clock_own_version_to_version",
            prepare: |f| {
                // The clock spelling of the explicit materialization:
                // `clock.own_version()` is an O(1) view (no cell of its
                // own; nothing scales), and this row prices its
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
                        in_place(),
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
                Some(Cell::new(
                    n,
                    Floors {
                        heap: na(NA_HEAP_IN_PLACE),
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
                let floors = shape_floors();
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
                let floors = rejection();
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
                let floors = rejection();
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
                let floors = rejection();
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
                let floors = rejection();
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
                let floors = rejection();
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
                // Reverse the canonical endpoints. The resulting composite is
                // well-formed component-wise but does not encode a span. The
                // hull of a distinct pair is strictly ordered, so reversing it
                // crosses the endpoints.
                let (v, w, _) = f.version_pair()?;
                let (lo, hi) = v.span(&w).into_parts();
                assert_ne!(lo, hi, "a crossed witness needs a strictly ordered hull");
                let fed = [hi.encode(), lo.encode()].concat();
                let n = fed.len();
                let floors = rejection();
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
                let floors = rejection();
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
                let floors = rejection();
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
                let floors = rejection();
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
                let floors = rejection();
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
                let floors = rejection();
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
            name: "rank_decode_truncated",
            prepare: |f| {
                // The family-derived rank's canonical bytes without their
                // last byte. Minimal packing ends the byte string in the byte
                // holding the stream's closing bit, so the cut removes it and
                // the decoder reads to the cut before it can reject.
                let (a, _) = f.rank_pair.clone()?;
                let mut fed = a.encode();
                fed.pop();
                let floors = Floors {
                    heap: na(NA_HEAP_REJECTION),
                };
                Some(Cell::new(fed.len(), floors, move || {
                    let err = Rank::decode(&fed[..]).expect_err("a truncated rank is rejected");
                    assert!(
                        matches!(err, Decode::Truncated),
                        "the placed defect is the cut, not {err:?}"
                    );
                    (err, fed)
                }))
            },
        },
        Op {
            name: "ranked_decode_truncated",
            prepare: |f| {
                // The family version's composite key cut short in its
                // version component, the key's tail: the decoder parses the
                // rank prefix and the version to the cut.
                let (v, _) = f.version()?;
                let ranked = Ranked::from(&v);
                let fed = truncated_bytes(&ranked.encode());
                let floors = rejection();
                Some(Cell::new(fed.len(), floors, move || {
                    let err =
                        Ranked::decode(&fed[..]).expect_err("a truncated composite is rejected");
                    assert!(
                        matches!(err, Decode::Truncated),
                        "the placed defect is the cut, not {err:?}"
                    );
                    (err, fed)
                }))
            },
        },
        Op {
            name: "ranked_decode_mismatch",
            prepare: |f| {
                // A composite key whose rank prefix is the rank of the
                // family's version ticked once by the seed, over the
                // version's own bytes. A tick raises the height on a region
                // of positive width, so the two ranks differ, and the
                // decoder can reject only after it has parsed both
                // components and ranked the version: the latest-detected
                // rejection the format has.
                let (v, _) = f.version()?;
                let mut ticked = v.clone();
                ticked.tick(&Party::seed());
                let fed = [ticked.rank().encode(), v.encode()].concat();
                let floors = Floors {
                    heap: na(NA_HEAP_REJECTION),
                };
                Some(Cell::new(fed.len(), floors, move || {
                    let err = Ranked::decode(&fed[..])
                        .expect_err("a rank prefix the version does not measure is rejected");
                    assert!(
                        matches!(err, Decode::NotCanonical),
                        "the placed defect is the mismatched rank, not {err:?}"
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
                let floors = rejection();
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
                // otherwise); rejection does no version work, since the
                // party join is the gate.
                let (v, w, nv) = match f.version_pair() {
                    Some(pair) => pair,
                    None => (Version::new(), Version::new(), 2),
                };
                let n = id_bytes + nv;
                let mut a = Clock::from_parts(decode_party(&a_bytes), v);
                let b = Clock::from_parts(decode_party(&b_bytes), w);
                Some(Cell::new(n, rejection(), move || {
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
                Some(Cell::new(n, rejection(), move || {
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
                let floors = rejection();
                Some(Cell::new(n, floors, move || {
                    let gone = a.without(&b);
                    assert!(gone.is_none(), "removing a covering region leaves nothing");
                    (gone, b)
                }))
            },
        },
        Op {
            name: "party_without_overlap",
            prepare: |f| {
                // A deep receiver: the overlap-mounted pair, whose one shared
                // region sits at both operands' preorder ends. The
                // disjointness check that `without` runs first reads both
                // streams to that region, and the difference walk then reads
                // both again and writes the receiver's remainder.
                let (a_bytes, b_bytes) = f.overlap.clone()?;
                let n = a_bytes.len() + b_bytes.len();
                let a = decode_party(&a_bytes);
                let b = decode_party(&b_bytes);
                let remainder_bytes = a
                    .dangerously_alias()
                    .without(&b)
                    .expect("the receiver keeps its left subtree, which the other operand lacks")
                    .as_bytes()
                    .len();
                let floors = Floors {
                    heap: heap_materializes(remainder_bytes),
                };
                Some(Cell::new(n, floors, move || (a.without(&b), b)))
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

/// Read the rendered text length from a rank-formatting result.
fn rank_text_output_bytes(result: &dyn std::any::Any) -> usize {
    result
        .downcast_ref::<(String, Rank)>()
        .expect("a rank display cell keeps its text")
        .0
        .len()
}

/// Representative [`Count`] values derived from a family's ranks.
fn counts_for_family(f: &FamilyData) -> Option<Vec<Count>> {
    if let Some((versions, _)) = &f.population {
        let counts = versions
            .iter()
            .map(|bytes| {
                let version = decode_version(bytes);
                Count(version.rank().raw_parts().0.clone())
            })
            .collect::<Vec<_>>();
        return (!counts.is_empty()).then_some(counts);
    }
    let (a, b) = f.rank_pair.as_ref()?;
    Some(vec![
        Count(a.raw_parts().0.clone()),
        Count(b.raw_parts().0.clone()),
    ])
}

/// A count's numeric width, rounded up so zero still has an input byte.
fn count_input_bytes(count: &Count) -> usize {
    count_value_bytes(count).max(1)
}

/// Bytes needed to store a count's significant bits.
fn count_value_bytes(count: &Count) -> usize {
    count.0.bits().div_ceil(8) as usize
}

/// Heap floor for a count result of `output_bytes` significant bytes.
fn count_heap_floor(output_bytes: usize) -> Liveness {
    if output_bytes <= (MACHINE_WORD_MAGNITUDE_BITS / 8) as usize {
        na("word-sized counts may use inline storage")
    } else {
        heap_materializes(output_bytes)
    }
}

/// Read the rendered text length from a count-formatting result.
fn count_text_output_bytes(result: &dyn std::any::Any) -> usize {
    result
        .downcast_ref::<(usize, Count)>()
        .expect("a count display cell keeps its byte count")
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

/// A binary serde sequence backed by owned `u64` limbs.
#[cfg(feature = "serde")]
struct OwnedLimbs(Vec<u64>);

#[cfg(feature = "serde")]
impl<'de> Deserializer<'de> for OwnedLimbs {
    type Error = serde::de::value::Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(serde::de::value::SeqDeserializer::new(self.0.into_iter()))
    }

    fn is_human_readable(&self) -> bool {
        false
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct tuple tuple_struct map struct enum
        identifier ignored_any
    }
}

/// Deserialize one binary serde byte field while transferring its allocation.
#[cfg(feature = "serde")]
fn serde_from_owned_bytes<T: DeserializeOwned>(bytes: Vec<u8>) -> T {
    T::deserialize(OwnedBytes(bytes))
        .unwrap_or_else(|error| panic!("board-generated serde bytes must decode: {error}"))
}

/// Deserialize a binary serde Count field from canonical limbs.
#[cfg(feature = "serde")]
fn serde_count_from_limbs(limbs: Vec<u64>) -> Count {
    <Count as serde::Deserialize>::deserialize(OwnedLimbs(limbs))
        .unwrap_or_else(|error| panic!("board-generated Count limbs must decode: {error}"))
}

/// Direct serde deserialization rows for each distinct allocation lifetime.
#[cfg(feature = "serde")]
fn serde_decode_ops() -> [Op; 7] {
    const OWNED_INPUT: &str = "serde transfers the caller's byte allocation into the decoded \
        value; only validation state may allocate";
    const SPLIT_INPUT: &str = "serde transfers the caller's byte allocation to the decoder, \
        which splits it so that each part owns storage sized to its encoding; only that split \
        and validation state may allocate";
    [
        Op {
            name: "party_serde_deserialize",
            prepare: |f| {
                let (bytes, _) = f.parties.clone()?;
                let n = bytes.len();
                let floors = Floors {
                    heap: na(OWNED_INPUT),
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
                let floors = Floors {
                    heap: na(OWNED_INPUT),
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
        Op {
            name: "clock_serde_deserialize",
            prepare: |f| {
                let (clock, _) = f.clock()?;
                let bytes = clock.encode();
                let n = bytes.len();
                let floors = Floors {
                    heap: na(SPLIT_INPUT),
                };
                Some(
                    Cell::new(n, floors, move || serde_from_owned_bytes::<Clock>(bytes))
                        .with_model(
                            Currency::Heap,
                            ModelSpec::ceiling(DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE),
                        ),
                )
            },
        },
        Op {
            name: "rank_serde_deserialize",
            prepare: |f| {
                let (rank, _) = f.rank_pair.clone()?;
                let bytes = rank.encode();
                let n = bytes.len();
                let floors = Floors {
                    heap: heap_materializes(rank_numerator_bytes(&rank)),
                };
                Some(
                    Cell::new(n, floors, move || serde_from_owned_bytes::<Rank>(bytes)).with_model(
                        Currency::Heap,
                        ModelSpec::ceiling(RANK_DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE),
                    ),
                )
            },
        },
        Op {
            name: "ranked_serde_deserialize",
            prepare: |f| {
                let (version, _) = f.version()?;
                let bytes = Ranked::from(&version).encode();
                let floors = Floors {
                    heap: na(SPLIT_INPUT),
                };
                Some(
                    Cell::new(bytes.len(), floors, move || {
                        serde_from_owned_bytes::<Ranked<'static>>(bytes)
                    })
                    .with_model(
                        Currency::Heap,
                        ModelSpec::ceiling(RANKED_DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE),
                    ),
                )
            },
        },
        Op {
            name: "span_serde_deserialize",
            prepare: |f| {
                let (v, w, _) = f.version_pair()?;
                let span = v.span(&w);
                let bytes = span.encode();
                let n = bytes.len();
                let floors = Floors {
                    heap: na(SPLIT_INPUT),
                };
                Some(
                    Cell::new(n, floors, move || {
                        serde_from_owned_bytes::<Span<'static>>(bytes)
                    })
                    .with_model(
                        Currency::Heap,
                        ModelSpec::ceiling(DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE),
                    ),
                )
            },
        },
        Op {
            name: "count_serde_deserialize",
            prepare: |f| {
                let count = counts_for_family(f)?
                    .into_iter()
                    .max_by_key(|count| count.0.bits())?;
                let limbs = count.limbs().collect::<Vec<_>>();
                let n = count_input_bytes(&count);
                Some(Cell::new(
                    n,
                    Floors {
                        heap: count_heap_floor(count_value_bytes(&count)),
                    },
                    move || serde_count_from_limbs(limbs),
                ))
            },
        },
    ]
}

/// Direct borsh deserialization rows for each distinct allocation lifetime.
#[cfg(feature = "borsh")]
fn borsh_decode_ops() -> [Op; 7] {
    [
        Op {
            name: "party_borsh_deserialize",
            prepare: |f| {
                let (bytes, _) = f.parties.clone()?;
                let n = bytes.len();
                let floors = Floors {
                    heap: heap_materializes(n),
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
                let floors = Floors {
                    heap: heap_materializes(n),
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
        Op {
            name: "clock_borsh_deserialize",
            prepare: |f| {
                let (clock, _) = f.clock()?;
                let bytes = clock.encode();
                let n = bytes.len();
                let floors = Floors {
                    heap: heap_materializes(n),
                };
                Some(
                    Cell::new(n, floors, move || {
                        Clock::try_from_slice(&bytes)
                            .expect("board-generated borsh clock bytes are canonical")
                    })
                    .with_model(
                        Currency::Heap,
                        ModelSpec::ceiling(DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE),
                    ),
                )
            },
        },
        Op {
            name: "rank_borsh_deserialize",
            prepare: |f| {
                let (rank, _) = f.rank_pair.clone()?;
                let bytes = rank.encode();
                let n = bytes.len();
                let floors = Floors {
                    heap: heap_materializes(rank_numerator_bytes(&rank)),
                };
                Some(
                    Cell::new(n, floors, move || {
                        Rank::try_from_slice(&bytes)
                            .expect("board-generated borsh rank bytes are canonical")
                    })
                    .with_model(
                        Currency::Heap,
                        ModelSpec::ceiling(RANK_DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE),
                    ),
                )
            },
        },
        Op {
            name: "ranked_borsh_deserialize",
            prepare: |f| {
                let (version, n) = f.version()?;
                let bytes = Ranked::from(&version).encode();
                let floors = Floors {
                    heap: heap_materializes(n),
                };
                Some(
                    Cell::new(bytes.len(), floors, move || {
                        Ranked::try_from_slice(&bytes)
                            .expect("board-generated borsh ranked bytes are canonical")
                    })
                    .with_model(
                        Currency::Heap,
                        ModelSpec::ceiling(RANKED_DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE),
                    ),
                )
            },
        },
        Op {
            name: "span_borsh_deserialize",
            prepare: |f| {
                let (v, w, _) = f.version_pair()?;
                let span = v.span(&w);
                let bytes = span.encode();
                let n = bytes.len();
                let floors = Floors {
                    heap: heap_materializes(n),
                };
                Some(
                    Cell::new(n, floors, move || {
                        Span::try_from_slice(&bytes)
                            .expect("board-generated borsh span bytes are canonical")
                    })
                    .with_model(
                        Currency::Heap,
                        ModelSpec::ceiling(DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE),
                    ),
                )
            },
        },
        Op {
            name: "count_borsh_deserialize",
            prepare: |f| {
                let count = counts_for_family(f)?
                    .into_iter()
                    .max_by_key(|count| count.0.bits())?;
                let bytes =
                    borsh::to_vec(&count).expect("a board-generated Count serializes to memory");
                let n = bytes.len();
                Some(Cell::new(
                    n,
                    Floors {
                        heap: count_heap_floor(count_value_bytes(&count)),
                    },
                    move || {
                        Count::try_from_slice(&bytes)
                            .expect("board-generated Count limbs are canonical")
                    },
                ))
            },
        },
    ]
}
