//! Process sharding: the operation × family grid split across child
//! processes, so the board parallelizes without changing what any cell
//! measures.
//!
//! The board's sweep is single-threaded by design: the peak-heap column
//! reads the process-global counting allocator, so cells measured on
//! concurrent threads would blend live sets and destroy the reading's
//! meaning. Parallelism therefore comes from *processes*: the runner
//! spawns copies of itself, each child owns its own global allocator and
//! measures one slice of the operation × family grid under exactly the
//! serial discipline — one cell at a time, reset-peak per cell, the
//! in-process determinism self-verification included ([`measure_cell`],
//! the same function the serial sweep drives) — and the parent merges
//! the measured samples back into board row order and judges and
//! renders them itself.
//!
//! # The seam
//!
//! A child emits raw [`Sample`]s, never verdicts: judgment
//! ([`evaluate`]) and rendering stay in the parent, over the one merged
//! result list, so the matrix, the worst-case fold, and the ranking pin
//! all run the same code over sharded and serial sweeps. The identity of
//! the two paths is pinned twice: in-process at smoke scale (the smoke
//! suite's round-trip test) and cross-process at both scales of record
//! (`just amp-board-shard-pin`, with the serial path as the reference).
//!
//! # The wire form
//!
//! One stamped header line (protocol version, shard index and count,
//! the scale's IEEE-754 bit pattern), one tab-separated `cell` line per
//! measured cell carrying both samples' counters, denominators, and
//! declarations (floats as bit patterns, so nothing rounds), and one
//! trailing `end` count line guarding truncation. The parent refuses any
//! mismatch — a header that is not byte-for-byte the one it commissioned,
//! an unknown operation or family name, a cell outside the child's
//! slice, a duplicate cell, or a count that disagrees with the lines
//! received. The protocol is internal to the runner (parent and children
//! are the same binary), not a stable format.
//!
//! # Slicing
//!
//! The shard unit is one cell of the operation × family grid: the
//! grid's family-outer linearization — the registry's board roster
//! ([`FamilyId::board`]) against the operation table — is dealt
//! round-robin over the shards ([`owns`]), so the shards partition the
//! grid by construction (coverage cannot drift with the shard count)
//! and, family cost being operand-size cost, the axis that dominates a
//! cell's price can never concentrate: one family's cells sit at
//! consecutive deal positions, so they spread evenly across the shards
//! whatever the tables' lengths share with the shard count. The
//! orientation is load-bearing — an op-outer deal aliases whenever the
//! family count and the shard count share a factor (at a roster length
//! the shard count divides, every cell of a family lands in one shard
//! and the deal degenerates to a family deal). The residual alias runs
//! the other way — a shard count dividing the operation count parks an
//! operation's column on few shards — and is accepted: operation
//! columns do not own operand size. The price of the per-cell deal is
//! that every child rebuilds each family it touches — cheap beside the
//! cells themselves, which measure every body four times per level pair
//! — and a child builds one family at a time, in roster order, so its
//! live set stays one operand-bundle pair wide.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Write};
use std::sync::{Mutex, OnceLock};

use super::currency::{ByCurrency, Liveness};
use super::judge::{evaluate, CellResult};
use super::measure::{HeapMeter, Sample};
use super::ops::ops;
use super::render::{assert_scale, build_pair, measure_cell, render_results, Summary};
use super::worst::{check_with, render_map};
use crate::meter::registry::FamilyId;

/// The wire header's protocol tag; bumped with any change to the cell
/// line's field order or encoding, or to the slice deal the ownership
/// check enforces, so a stale child binary can never be merged as
/// current.
const PROTOCOL: &str = "amp-board-shard v2";

/// Runs every shard child at one scale and returns their raw stdout
/// captures in shard-index order.
///
/// Supplied by the runner binary (spawning `current_exe` is per-binary
/// state the library cannot own, exactly like the [`HeapMeter`]); the
/// captures feed the merge, which validates each child's stamps.
pub type ShardSpawner<'a> = &'a dyn Fn(f64) -> io::Result<Vec<Vec<u8>>>;

/// Whether shard `index` of `count` owns the grid cell at
/// (`op_position`, `family_position`): the family-outer linearization
/// of the operation × family grid, dealt round-robin.
///
/// The one deal, shared by the emitter and the merge's ownership check
/// (`op_count` is the operation table's length, so both sides derive
/// the deal from the same tables). Family-outer keeps one family's
/// cells at consecutive deal positions — the module doc's aliasing
/// argument.
///
/// # Panics
///
/// Panics unless `index < count`.
fn owns(
    index: usize,
    count: usize,
    op_position: usize,
    family_position: usize,
    op_count: usize,
) -> bool {
    assert!(
        index < count,
        "amp-board shard: index {index} out of range for {count} shards"
    );
    (family_position * op_count + op_position) % count == index
}

/// The largest shard count that can still receive work: the size of the
/// operation × family grid (a larger count would only spawn children
/// with empty slices).
pub fn max_useful_shards() -> usize {
    ops().len() * FamilyId::board().count()
}

/// Child mode: measure shard `index` of `count`'s slice of the
/// operation × family grid at `scale` under the serial measurement
/// discipline and emit the measured samples to `out` in the shard wire
/// form.
///
/// Cells are grouped family-outer so each owned family's operand
/// bundles are built once and dropped before the next family's; the
/// parent restores board row order.
///
/// # Panics
///
/// Panics unless `index < count` and `scale` is strictly positive, and
/// on any counter disagreement between a cell's two in-process
/// measurements (the determinism self-verification, exactly as in the
/// serial sweep).
pub fn emit_shard(
    scale: f64,
    index: usize,
    count: usize,
    heap: &HeapMeter,
    out: &mut dyn Write,
) -> io::Result<()> {
    assert_scale(scale);
    let table = ops();
    let families: Vec<FamilyId> = FamilyId::board().collect();
    // The emission is staged in memory and written only after the last
    // cell is measured: `out` is typically a pipe the parent drains in
    // shard order, and a child that writes between measurements stalls
    // mid-sweep the moment the pipe's buffer fills, serializing the
    // shards against the parent's drain order.
    let mut staged = Vec::new();
    let mut emitted = 0usize;
    for (family_position, &family) in families.iter().enumerate() {
        let owned: Vec<usize> = (0..table.len())
            .filter(|&op_position| owns(index, count, op_position, family_position, table.len()))
            .collect();
        if owned.is_empty() {
            continue;
        }
        let pair = build_pair(family, scale);
        for op_position in owned {
            let Some(r) = measure_cell(heap, &table[op_position], &pair) else {
                continue;
            };
            write!(staged, "cell\t{op}\t{family}", op = r.op, family = r.family)?;
            emit_sample(&mut staged, &r.s1)?;
            emit_sample(&mut staged, &r.s2)?;
            writeln!(staged)?;
            emitted += 1;
        }
    }
    writeln!(
        out,
        "{PROTOCOL} shard {index}/{count} scale {bits:016x}",
        bits = scale.to_bits()
    )?;
    out.write_all(&staged)?;
    writeln!(out, "end {emitted}")
}

/// An `f64` as its IEEE-754 bit pattern, so the parent reconstructs the
/// child's value exactly (decimal round-trips are where identity dies).
fn bits(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

/// One optional field, `-` for absent.
fn opt<T: ToString>(value: Option<T>) -> String {
    value.map_or_else(|| "-".to_string(), |v| v.to_string())
}

/// A wire string field must not contain the framing bytes.
fn assert_unframed(text: &str) {
    assert!(
        !text.contains('\t') && !text.contains('\n'),
        "amp-board shard: a floor rationale may not contain a tab or newline: {text:?}"
    );
}

/// Emit one sample's fields, tab-prefixed, in the fixed wire order the
/// parser mirrors ([`parse_sample`]).
fn emit_sample(out: &mut dyn Write, s: &Sample) -> io::Result<()> {
    write!(
        out,
        "\t{denom}\t{exp_denom}\t{limb_denom}\t{text}",
        denom = s.denom_bytes,
        exp_denom = s.exp_denom_bytes,
        limb_denom = s.limb_denom,
        text = if s.text_row { "t" } else { "f" },
    )?;
    write!(
        out,
        "\t{arity}\t{search}\t{model}\t{declared_heap}\t{declared_limb}",
        arity = opt(s.fold_arity),
        search = s.fold_search_bits,
        model = opt(s.heap_model.map(bits)),
        declared_heap = opt(s.declared_heap.map(bits)),
        declared_limb = opt(s
            .declared_limb
            .map(|(e, k)| format!("{},{}", bits(e), bits(k)))),
    )?;
    for (_, reading) in s.readings.each() {
        write!(out, "\t{}", opt(*reading))?;
    }
    for (_, floor) in s.floors.each() {
        match *floor {
            Liveness::Floor { min, why } => {
                assert_unframed(why);
                write!(out, "\tF {min} {why}")?;
            }
            Liveness::NotApplicable { reason } => {
                assert_unframed(reason);
                write!(out, "\tN {reason}")?;
            }
        }
    }
    Ok(())
}

/// Intern one child-reported rationale string as `&'static str`.
///
/// A reconstructed [`Sample`] carries its floor rationales as
/// `&'static str` exactly like a locally measured one; each distinct
/// string is leaked once per process, and the set of distinct rationales
/// is bounded by the board's own legend.
fn intern(text: &str) -> &'static str {
    static CACHE: OnceLock<Mutex<BTreeSet<&'static str>>> = OnceLock::new();
    let mut cache = CACHE
        .get_or_init(|| Mutex::new(BTreeSet::new()))
        .lock()
        .expect("the intern cache lock is never poisoned: no panics under it");
    match cache.get(text) {
        Some(interned) => interned,
        None => {
            let interned: &'static str = Box::leak(text.to_owned().into_boxed_str());
            cache.insert(interned);
            interned
        }
    }
}

/// The next tab field of a cell line, or a refusal naming the line.
fn field<'a>(fields: &mut impl Iterator<Item = &'a str>, line: &str) -> &'a str {
    fields
        .next()
        .unwrap_or_else(|| panic!("amp-board shard merge: truncated cell line: {line:?}"))
}

/// A decimal wire integer.
fn number<T: std::str::FromStr>(text: &str, line: &str) -> T {
    text.parse()
        .unwrap_or_else(|_| panic!("amp-board shard merge: malformed number {text:?} in {line:?}"))
}

/// An optional decimal wire integer (`-` for absent).
fn opt_number<T: std::str::FromStr>(text: &str, line: &str) -> Option<T> {
    (text != "-").then(|| number(text, line))
}

/// An `f64` from its wire bit pattern.
fn from_bits(text: &str, line: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(text, 16).unwrap_or_else(|_| {
        panic!("amp-board shard merge: malformed float bits {text:?} in {line:?}")
    }))
}

/// One floor field back into its [`Liveness`] arm.
fn parse_liveness(text: &str, line: &str) -> Liveness {
    if let Some(rest) = text.strip_prefix("F ") {
        let (min, why) = rest.split_once(' ').unwrap_or_else(|| {
            panic!("amp-board shard merge: malformed floor field {text:?} in {line:?}")
        });
        Liveness::Floor {
            min: number(min, line),
            why: intern(why),
        }
    } else if let Some(reason) = text.strip_prefix("N ") {
        Liveness::NotApplicable {
            reason: intern(reason),
        }
    } else {
        panic!("amp-board shard merge: malformed floor field {text:?} in {line:?}")
    }
}

/// One sample's fields back into a [`Sample`], mirroring
/// [`emit_sample`]'s order.
fn parse_sample<'a>(fields: &mut impl Iterator<Item = &'a str>, line: &str) -> Sample {
    let denom_bytes = number(field(fields, line), line);
    let exp_denom_bytes = number(field(fields, line), line);
    let limb_denom = number(field(fields, line), line);
    let text_row = match field(fields, line) {
        "t" => true,
        "f" => false,
        other => panic!("amp-board shard merge: malformed text-row flag {other:?} in {line:?}"),
    };
    let fold_arity = opt_number(field(fields, line), line);
    let fold_search_bits = number(field(fields, line), line);
    let heap_model = {
        let text = field(fields, line);
        (text != "-").then(|| from_bits(text, line))
    };
    let declared_heap = {
        let text = field(fields, line);
        (text != "-").then(|| from_bits(text, line))
    };
    let declared_limb = {
        let text = field(fields, line);
        (text != "-").then(|| {
            let (e, k) = text.split_once(',').unwrap_or_else(|| {
                panic!("amp-board shard merge: malformed limb model {text:?} in {line:?}")
            });
            (from_bits(e, line), from_bits(k, line))
        })
    };
    let mut reading = || opt_number(field(fields, line), line);
    let readings = ByCurrency {
        heap: reading(),
        segments: reading(),
        limb: reading(),
        scan: reading(),
        touch: reading(),
    };
    let mut floor = || parse_liveness(field(fields, line), line);
    let floors = ByCurrency {
        heap: floor(),
        segments: floor(),
        limb: floor(),
        scan: floor(),
        touch: floor(),
    };
    Sample {
        denom_bytes,
        exp_denom_bytes,
        limb_denom,
        text_row,
        floors,
        fold_arity,
        fold_search_bits,
        heap_model,
        declared_heap,
        declared_limb,
        readings,
    }
}

/// Merge `count` children's captures into one whole board's judged cells
/// in board row order.
///
/// Validates every stamp, parses the samples, reorders by the parent's
/// own operation table and family roster, and judges each cell
/// ([`evaluate`]) exactly as the serial sweep would.
///
/// # Panics
///
/// Panics on any protocol violation — the module doc's refusal list —
/// and unless `scale` is strictly positive (the same guard as the serial
/// sweep, before any child capture is trusted).
fn merge(scale: f64, count: usize, captures: &[Vec<u8>]) -> Vec<CellResult> {
    assert!(
        scale > 0.0 && scale.is_finite(),
        "amp-board: scale must be a positive finite number"
    );
    assert_eq!(
        captures.len(),
        count,
        "amp-board shard merge: expected {count} child captures, got {}",
        captures.len()
    );
    let op_order: BTreeMap<&'static str, usize> = ops()
        .iter()
        .enumerate()
        .map(|(position, op)| (op.name, position))
        .collect();
    let family_order: BTreeMap<&'static str, usize> = FamilyId::board()
        .enumerate()
        .map(|(position, family)| (family.name(), position))
        .collect();
    let mut cells: BTreeMap<(usize, usize), (&'static str, &'static str, Sample, Sample)> =
        BTreeMap::new();
    for (index, capture) in captures.iter().enumerate() {
        let text = std::str::from_utf8(capture).unwrap_or_else(|_| {
            panic!("amp-board shard merge: shard {index}/{count} emitted non-UTF-8 output")
        });
        let mut lines = text.lines();
        let expected_header = format!(
            "{PROTOCOL} shard {index}/{count} scale {bits:016x}",
            bits = scale.to_bits()
        );
        let header = lines.next().unwrap_or_default();
        assert_eq!(
            header, expected_header,
            "amp-board shard merge: shard {index}/{count} stamp mismatch: refused"
        );
        let mut emitted = 0usize;
        let mut ended = false;
        for line in lines {
            assert!(
                !ended,
                "amp-board shard merge: shard {index}/{count} emitted past its end line: {line:?}"
            );
            if let Some(cell_count) = line.strip_prefix("end ") {
                let declared: usize = number(cell_count, line);
                assert_eq!(
                    declared, emitted,
                    "amp-board shard merge: shard {index}/{count} declared {declared} cells but \
                     emitted {emitted}"
                );
                ended = true;
                continue;
            }
            let mut fields = line
                .strip_prefix("cell\t")
                .unwrap_or_else(|| {
                    panic!("amp-board shard merge: shard {index}/{count} unknown line: {line:?}")
                })
                .split('\t');
            let op_name = field(&mut fields, line);
            let family_name = field(&mut fields, line);
            let (&op, &op_position) = op_order.get_key_value(op_name).unwrap_or_else(|| {
                panic!("amp-board shard merge: unknown operation {op_name:?} in {line:?}")
            });
            let (&family, &family_position) =
                family_order.get_key_value(family_name).unwrap_or_else(|| {
                    panic!("amp-board shard merge: unknown family {family_name:?} in {line:?}")
                });
            assert!(
                owns(index, count, op_position, family_position, op_order.len()),
                "amp-board shard merge: shard {index}/{count} emitted {op} x {family}, a cell \
                 outside its slice"
            );
            let s1 = parse_sample(&mut fields, line);
            let s2 = parse_sample(&mut fields, line);
            assert!(
                fields.next().is_none(),
                "amp-board shard merge: trailing fields on cell line: {line:?}"
            );
            let duplicate = cells
                .insert((op_position, family_position), (op, family, s1, s2))
                .is_some();
            assert!(
                !duplicate,
                "amp-board shard merge: duplicate cell {op} x {family}"
            );
            emitted += 1;
        }
        assert!(
            ended,
            "amp-board shard merge: shard {index}/{count} capture is truncated: no end line"
        );
    }
    cells
        .into_values()
        .map(|(op, family, s1, s2)| evaluate(op, family, s1, s2))
        .collect()
}

/// Run the whole board across `shards` child processes and render the
/// matrix to `out`: [`run`](super::render::run)'s process-sharded form,
/// byte-identical to it by pin (`just amp-board-shard-pin`).
///
/// `spawn` is invoked once, at `scale`.
///
/// # Panics
///
/// Panics unless `scale` is strictly positive, and on any protocol
/// violation in a child capture (the module doc's refusal list).
pub fn run_sharded(
    scale: f64,
    shards: usize,
    spawn: ShardSpawner<'_>,
    out: &mut dyn Write,
) -> io::Result<Summary> {
    render_results(&merge(scale, shards, &spawn(scale)?), out)
}

/// Render the worst-case map at `scale` from a sweep run across `shards`
/// child processes: [`worst_map`](super::worst::worst_map)'s
/// process-sharded form.
///
/// `spawn` is invoked once, at `scale`.
///
/// # Panics
///
/// As [`run_sharded`].
pub fn worst_map_sharded(
    label: &str,
    scale: f64,
    shards: usize,
    spawn: ShardSpawner<'_>,
    out: &mut dyn Write,
) -> io::Result<()> {
    render_map(label, scale, &merge(scale, shards, &spawn(scale)?), out)
}

/// Entry-compare the live worst-case fold against the committed ranking
/// pin from sweeps run across `shards` child processes:
/// [`check_worst_map`](super::worst::check_worst_map)'s process-sharded
/// form.
///
/// `spawn` is invoked once per scale of record.
///
/// # Panics
///
/// As [`check_worst_map`](super::worst::check_worst_map), plus any
/// protocol violation in a child capture.
pub fn check_worst_map_sharded(
    shards: usize,
    spawn: ShardSpawner<'_>,
    out: &mut dyn Write,
) -> io::Result<bool> {
    check_with(&mut |scale| Ok(merge(scale, shards, &spawn(scale)?)), out)
}
