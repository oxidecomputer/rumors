//! Law-driven fuzzing: every named law in `before::laws`, on decoded
//! hostile-but-canonical values.
//!
//! The in-tree proptests drive the law collection over *generated* inputs;
//! this target drives the identical slices over values `decode` accepts from
//! arbitrary bytes — adversarially-shaped trees, wide-gamma bases, and
//! party/version pairings no op sequence produces. A violated law is a panic
//! naming the law, so the fuzzer minimizes straight to the algebraic defect.
//! The drive loop expands from the law-group roster
//! (`before::for_each_law_group!`), so a group added to the roster is
//! fuzzed here by construction, with no wiring in this file.
//! The contract: every law holds, and no law's computation passes the
//! harness heap cap (`before_fuzz::under_heap_cap`).
//!
//! The input is a run of length-prefixed chunks — `[len: u8][bytes: len,
//! capped at the remainder]` — decoded positionally: chunks 0–2 are
//! `Version`s, 3–4 are `Party`s, 5 is a `Clock`; a chunk that fails to
//! decode falls back to that type's canonical default (`Version::new()`,
//! `Party::seed()`, `Clock::seed()`), so every input drives every law group
//! and coverage feedback still rewards genuinely decoding chunks. Ranks are
//! derived from the decoded versions (their ranks and a distance). The
//! remainder is three list scripts — `[arity: u8][pool indices: one byte
//! per element]`, versions then parties then clocks, arity folded into the
//! fold-boundary band and indices into small pools of the decoded values
//! (missing bytes read as zero, so short inputs drive the empty edges) —
//! feeding the variadic law groups. This framing is a wire contract with
//! the committed seed corpus: `tests/support/fuzz_seed_set.rs` spells seeds
//! in exactly this shape, so a change here means regenerating the seeds
//! with it.

#![no_main]

use libfuzzer_sys::fuzz_target;

use before::{Clock, Party, Rank, Version};

fuzz_target!(|data: &[u8]| {
    before_fuzz::under_heap_cap(|| run(data));
});

/// Carve the next length-prefixed chunk off the front of the input.
///
/// The length is capped at the remainder, so a hostile prefix cannot index
/// out of bounds; an exhausted input yields empty chunks (which fail decode
/// and fall back).
fn chunk<'d>(data: &mut &'d [u8]) -> &'d [u8] {
    let Some((&len, rest)) = data.split_first() else {
        *data = &[];
        return &[];
    };
    let split = (len as usize).min(rest.len());
    let (bytes, tail) = rest.split_at(split);
    *data = tail;
    bytes
}

/// The list-arity band: a script's arity byte is folded into `0..ARITY_SPAN`.
///
/// The band `0..=17` sweeps every structural boundary of the balanced
/// counter the n-ary folds run on: every combine-arm genre (leaf,
/// merged–input, merged–merged; in-counter and drain) is reachable by
/// arity 6, and 8/9 and 15/16/17 cross two octave boundaries, so behavior
/// keyed to a particular counter weight rather than a genre still meets
/// two octaves of weights.
const ARITY_SPAN: usize = 18;

/// Read one script byte off the front of the input, or zero when the input
/// is exhausted — so short inputs still drive every variadic group, through
/// the empty edges.
fn byte(data: &mut &[u8]) -> u8 {
    let Some((&next, rest)) = data.split_first() else {
        return 0;
    };
    *data = rest;
    next
}

/// One list script: an arity byte folded into the band, then one pool-index
/// byte per element.
fn picks(data: &mut &[u8], pool: usize) -> Vec<usize> {
    let arity = usize::from(byte(data)) % ARITY_SPAN;
    (0..arity).map(|_| usize::from(byte(data)) % pool).collect()
}

/// Assert every law in a group, panicking with the violated law's name.
macro_rules! drive {
    ($group:expr, $($input:expr),+) => {
        for (name, law) in $group {
            assert!(law($($input),+), "law violated: {name}");
        }
    };
}

/// One decoded input's environment: the pools the roster-derived drive
/// list selects each law group's inputs from.
struct Env<'x> {
    /// The three positionally decoded versions.
    v: [&'x Version; 3],
    /// The two decoded parties, then the decoded clock's party.
    p: [&'x Party; 3],
    /// Ranks derived from the decoded versions: two own ranks and a
    /// genuine distance.
    r: [&'x Rank; 3],
    /// The decoded clock.
    k: &'x Clock,
    /// The list scripts' pool-indexed variadic inputs.
    versions: &'x [Version],
    parties: &'x [Party],
    clocks: &'x [Clock],
}

/// Expands the law-group roster into the drive loop: one `drive!` per
/// group, keyed on the group's input signature, selecting that
/// signature's inputs from an [`Env`].
///
/// The group list lives only in `before::for_each_law_group!`, so a
/// group added to the roster is fuzzed here by construction; a roster
/// signature without an arm refuses to compile.
macro_rules! drive_groups {
    (args: ($env:expr); $(($group:ident, $driver:ident, $shape:tt)),* $(,)?) => {
        $( drive_groups!(@one $env, $group, $shape); )*
    };
    (@one $env:expr, $group:ident, (version)) => {
        drive!(before::laws::$group, $env.v[0]);
    };
    (@one $env:expr, $group:ident, (version, version)) => {
        drive!(before::laws::$group, $env.v[0], $env.v[1]);
    };
    (@one $env:expr, $group:ident, (version, version, version)) => {
        drive!(before::laws::$group, $env.v[0], $env.v[1], $env.v[2]);
    };
    (@one $env:expr, $group:ident, (party)) => {
        drive!(before::laws::$group, $env.p[0]);
    };
    (@one $env:expr, $group:ident, (party, party)) => {
        drive!(before::laws::$group, $env.p[0], $env.p[1]);
    };
    (@one $env:expr, $group:ident, (party, party, party)) => {
        drive!(before::laws::$group, $env.p[0], $env.p[1], $env.p[2]);
    };
    (@one $env:expr, $group:ident, (version, party)) => {
        drive!(before::laws::$group, $env.v[0], $env.p[0]);
    };
    (@one $env:expr, $group:ident, (version, version, party)) => {
        drive!(before::laws::$group, $env.v[0], $env.v[1], $env.p[0]);
    };
    (@one $env:expr, $group:ident, (version, party, party)) => {
        drive!(before::laws::$group, $env.v[0], $env.p[0], $env.p[1]);
    };
    (@one $env:expr, $group:ident, (version, version, party, party)) => {
        drive!(before::laws::$group, $env.v[0], $env.v[1], $env.p[0], $env.p[1]);
    };
    (@one $env:expr, $group:ident, (rank, rank, rank)) => {
        drive!(before::laws::$group, $env.r[0], $env.r[1], $env.r[2]);
    };
    (@one $env:expr, $group:ident, (clock)) => {
        drive!(before::laws::$group, $env.k);
    };
    (@one $env:expr, $group:ident, (clock, version)) => {
        drive!(before::laws::$group, $env.k, $env.v[0]);
    };
    (@one $env:expr, $group:ident, (versions)) => {
        drive!(before::laws::$group, $env.versions);
    };
    (@one $env:expr, $group:ident, (version, versions)) => {
        drive!(before::laws::$group, $env.v[0], $env.versions);
    };
    (@one $env:expr, $group:ident, (party, parties)) => {
        drive!(before::laws::$group, $env.p[0], $env.parties);
    };
    (@one $env:expr, $group:ident, (clock, clocks)) => {
        drive!(before::laws::$group, $env.k, $env.clocks);
    };
}

/// One input's body: decode the chunks positionally and drive every group.
fn run(mut data: &[u8]) {
    let data = &mut data;
    let a = Version::decode(chunk(data)).unwrap_or_default();
    let b = Version::decode(chunk(data)).unwrap_or_default();
    let c = Version::decode(chunk(data)).unwrap_or_default();
    let p = Party::decode(chunk(data)).unwrap_or_else(|_| Party::seed());
    let q = Party::decode(chunk(data)).unwrap_or_else(|_| Party::seed());
    let k = Clock::decode(chunk(data)).unwrap_or_else(|_| Clock::seed());

    // The list scripts: one per variadic group, each `[arity][indices…]`
    // over a small pool of the values above — repeats (and, for parties
    // and clocks, aliases) arise at every arity, the input classes the
    // variadic laws' fold and refusal arms exist for.
    let zero = Version::new();
    let vpool = [&a, &b, &c, &zero];
    let versions: Vec<Version> = picks(data, vpool.len())
        .into_iter()
        .map(|i| vpool[i].clone())
        .collect();
    let ppool = [&p, &q, k.party()];
    let parties: Vec<Party> = picks(data, ppool.len())
        .into_iter()
        .map(|i| ppool[i].dangerously_alias())
        .collect();
    let ka = Clock::from_parts(p.dangerously_alias(), a.clone());
    let kb = Clock::from_parts(q.dangerously_alias(), b.clone());
    let cpool = [&k, &ka, &kb];
    let clocks: Vec<Clock> = picks(data, cpool.len())
        .into_iter()
        .map(|i| cpool[i].dangerously_alias())
        .collect();

    let (ra, rb, rc) = (a.rank(), b.rank(), a.distance(&b));
    let env = Env {
        v: [&a, &b, &c],
        p: [&p, &q, k.party()],
        r: [&ra, &rb, &rc],
        k: &k,
        versions: &versions,
        parties: &parties,
        clocks: &clocks,
    };
    before::for_each_law_group!(drive_groups(&env));
}
