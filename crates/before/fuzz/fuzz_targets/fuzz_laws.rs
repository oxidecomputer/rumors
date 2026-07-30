//! Law-driven fuzzing: every named law in `before::laws`, on decoded
//! hostile-but-canonical values.
//!
//! The in-tree proptests drive the law collection over *generated* inputs;
//! this target drives the identical slices over values `decode` accepts from
//! arbitrary bytes — adversarially-shaped trees, wide-gamma bases, and
//! party/version pairings no op sequence produces. A violated law is a panic
//! naming the law, so the fuzzer minimizes straight to the algebraic defect.
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

use before::{laws, Clock, Party, Version};

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

    drive!(laws::VERSION_SOLO, &a);
    drive!(laws::VERSION_PAIR, &a, &b);
    drive!(laws::VERSION_TRIPLE, &a, &b, &c);
    drive!(laws::PARTY_SOLO, &p);
    drive!(laws::PARTY_PAIR, &p, &q);
    drive!(laws::PARTY_TRIPLE, &p, &q, k.party());
    drive!(laws::VERSION_PARTY, &a, &p);
    drive!(laws::VERSION_PAIR_PARTY, &a, &b, &p);
    drive!(laws::VERSION_PARTY_PAIR, &a, &p, &q);
    drive!(laws::VERSION_PAIR_PARTY_PAIR, &a, &b, &p, &q);
    let (ra, rb, rc) = (a.rank(), b.rank(), a.distance(&b));
    drive!(laws::RANK_TRIPLE, &ra, &rb, &rc);
    drive!(laws::CLOCK_SOLO, &k);
    drive!(laws::CLOCK_VERSION, &k, &a);
    drive!(laws::VERSION_LIST, &versions);
    drive!(laws::VERSION_AND_LIST, &a, &versions);
    drive!(laws::PARTY_AND_LIST, &p, &parties);
    drive!(laws::CLOCK_AND_LIST, &k, &clocks);
}
