//! Resource envelopes for balanced folds and their identity fast path.

use super::*;

// ─── the cheap-clone demonstration cell ──────────────────────────────────────

/// Peak-heap ceiling for the equal-operands join fold: the fold's own
/// machinery (the counter's group vec, the dedup adapter's held clone),
/// none of it proportional to the operands.
// The equality rung's hand-back is an O(1) refcount bump, so the fold's
// peak is its size-independent machinery alone — the flatness leg below
// is the proof. Ceiling 1.25x the measurement of record (the reading
// lives in the pin commit).
const JOIN_EQUAL_OPERANDS_PEAK: usize = 440;

/// The cheap-clone demonstration: `join_all` over two byte-equal
/// versions answers through the equality rung and hands back a clone,
/// an `O(1)` refcount bump.
///
/// The fold's peak heap is therefore the counter machinery alone,
/// *byte-identical across a 4x operand growth* (the flatness leg no
/// operand-copying clone arm can pass). The semantic leg pins the
/// verdict: the join IS the operand, byte for byte.
#[test]
fn join_all_equal_operands_is_clone_cheap() {
    let run = |depth: usize| {
        let p = Shape::Dense.build1(depth);
        let a = version_of(&p);
        let b = version_of(&p); // byte-equal, buffer-distinct
        let input_bytes = a.encode().len() + b.encode().len();
        HEAP.reset_peak_usage();
        let baseline = HEAP.current_usage();
        let out = a.join_all([&b]);
        let peak_heap = HEAP.peak_usage().saturating_sub(baseline);
        eprintln!(
            "MEASURED join_all_equal_operands/{depth}: input_bytes={input_bytes} \
             peak_heap={peak_heap}"
        );
        assert_eq!(out, a, "a ∨ a = a: the fold's verdict is the operand");
        assert_eq!(out.as_bytes(), a.as_bytes());
        peak_heap
    };
    // The witness of record: two byte-equal dense versions (depth 1,000).
    let small = run(1000);
    let large = run(4000);
    assert!(
        small <= JOIN_EQUAL_OPERANDS_PEAK,
        "join_all over two equal operands peaked {small} B (ceiling \
         {JOIN_EQUAL_OPERANDS_PEAK} B): the clone arm must not copy the \
         operand: {ISOLATION_NOTE}"
    );
    assert_eq!(
        small, large,
        "the equal-operands fold's peak must not move with operand size: \
         the clone arm is O(1): {ISOLATION_NOTE}"
    );
}

// ─── join-fold scenarios ────────────────────────────────────────────────────
//
// The public join folds on the scatter population: 1,024 balanced-forked
// parties, one tick each, ordered evens before odds so a left fold's
// accumulator would hold every other leaf and never coalesce — the shape
// on which a sequential fold reads quadratic (the board's `scatter`
// cells exist to catch exactly that). The balanced binary-counter
// reduction gives every input O(log n) joins against similarly-sized
// partners. These rows pin the balanced fold's scan and touch work. A
// sequential left fold reads an order of magnitude more at this arity, with a
// widening gap as the population grows.

/// The board's scatter population at the enforced-suite scale.
const FOLD_SCATTER_CLOCKS: usize = 1_024;

/// Build the scatter fold population: balanced-forked parties, one tick
/// each, evens before odds (the board's `scatter` family recipe).
fn scatter_population() -> (Vec<Version>, Vec<before::Party>) {
    let mut parties = vec![before::Party::seed()];
    while parties.len() < FOLD_SCATTER_CLOCKS {
        let mut next = Vec::with_capacity(parties.len() * 2);
        for mut p in parties {
            let q = p.fork();
            next.push(p);
            next.push(q);
        }
        parties = next;
    }
    let versions: Vec<Version> = parties
        .iter()
        .map(|p| {
            let mut v = Version::new();
            v.tick(p);
            v
        })
        .collect();
    let scatter = |len: usize| (0..len).step_by(2).chain((1..len).step_by(2));
    let versions = scatter(versions.len())
        .map(|i| versions[i].clone())
        .collect();
    let mut scattered_parties = Vec::with_capacity(parties.len());
    let mut slots: Vec<Option<before::Party>> = parties.into_iter().map(Some).collect();
    for i in scatter(slots.len()) {
        scattered_parties.push(slots[i].take().expect("each index is visited once"));
    }
    (versions, scattered_parties)
}

/// `Version::join_all` over the scatter population stays within its
/// envelope.
///
/// The balanced reduction keeps every join's operands comparably sized,
/// so the fold is near-linear in the population's input bytes where the
/// left fold re-scanned its whole accumulator per input.
#[test]
fn fold_version_scatter_envelope() {
    let (mut versions, _) = scatter_population();
    let input_bytes: usize = versions.iter().map(|v| v.encode().len()).sum();
    let reference = versions.iter().fold(Version::new(), |acc, v| acc | v);
    let rest = versions.split_off(1);
    let receiver = versions.pop().expect("the population is nonempty");
    let out = metered(
        "fold_version_scatter",
        input_bytes,
        &query_env::FOLD_VERSION_SCATTER,
        || receiver.join_all(rest),
    );
    assert_eq!(out, reference, "the balanced fold equals the left fold");
}

/// `Party::join_all` over the scatter population stays within its
/// envelope.
///
/// The id-side fold's work is pure stream scanning, and the balanced
/// reduction keeps the scanned bits near-linear in the population's
/// input bytes where the left fold re-walked its whole accumulated
/// region per input.
#[test]
fn fold_party_scatter_envelope() {
    let (_, mut parties) = scatter_population();
    let input_bytes: usize = parties.iter().map(|p| p.encode().len()).sum();
    let rest = parties.split_off(1);
    let mut acc = parties.remove(0);
    let acc = metered(
        "fold_party_scatter",
        input_bytes,
        &query_env::FOLD_PARTY_SCATTER,
        move || {
            acc.join_all(rest)
                .expect("balanced forks are pairwise disjoint");
            acc
        },
    );
    assert!(acc.is_seed(), "the scattered forks reunite the seed region");
}

// ─── the n-ary fold's correlated-population bands (the stagger pins) ─────────
//
// The staggered fold population loads the balanced binary-counter
// reduction itself: `n` operands of `m` unit teeth each, every operand's
// teeth landing in the gaps of every other's, fed in bit-reversed order
// so every internal merge — at every level — joins region sets that
// interleave maximally and swell to near the sum of their sizes
// (`meter::stagger_population` carries the construction and the feed
// order's derivation). The scatter population scales arity at
// single-leaf operands and weave scales operand size at fixed arity;
// this family drives both axes jointly against the reduction, so the
// bands below hold the fold's declared `O(D log 2k)` model in EACH
// direction independently — arity doubling at fixed operand size, and
// operand size doubling at fixed arity — under absolute pinned ceilings:
// a single diagonal scaling could hide which factor drives growth.
// Every counter is normalized by the model's own level count
// `log2(2n)` before the ×1.25 flatness bound, so the bands enforce the
// model's *constant*, not a flat reading the documented log factor
// would forbid (on the arity axis the raw per-byte cost legitimately
// grows exactly one level's worth per doubling).
//
// A sequential left version fold (`fold(Version::new(), |acc, v| acc | v)`)
// re-walks
// its never-coalescing accumulator per input, and the sequential party
// fold (one `join` per input) re-walks its accumulated region the same
// way — the growing-accumulator case the balanced reduction exists to
// foreclose, quadratic in arity where the reduction is log-linear, so
// its readings sit several times over these bands with the gap widening
// with arity (the demonstration readings live in the pin commit); the
// per-entry point `*_log_factor_is_alive` pins (the asymptotics suite) keep
// the model's log factor itself explicit.
mod fold_stagger;
