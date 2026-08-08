//! Clock-level tests.

use proptest::prelude::*;

use crate::oracle;
use crate::testing::bridge::{
    from_oracle_clock, from_oracle_party, from_oracle_version, to_oracle_clock, to_oracle_party,
    to_oracle_version,
};
use crate::testing::generators::{
    arb_oracle_party_nonempty, arb_oracle_version, deep_left_spine_party,
};
use crate::testing::optrace::{run, step_impl, world_strategy, Op};
use crate::{error::Parse, Clock, Party, Version};

/// Build one organic history's clock population, deterministically
/// reproducible from the same ops.
fn world_clocks(ops: &[Op]) -> Vec<Clock> {
    let mut clocks = vec![Clock::seed()];
    for op in ops {
        step_impl(&mut clocks, op);
    }
    clocks
}

proptest! {
    /// The balanced `join_all` is the sequential fold on clocks.
    ///
    /// Over one organic history's pairwise-disjoint clocks, folding the rest
    /// into any member returns `Ok` with exactly the clock (party and version
    /// both) the sequential `join`-per-input reference produces, in both input
    /// orders.
    #[test]
    fn join_all_matches_the_sequential_fold(ops in world_strategy(), i in 0usize..64, reverse in any::<bool>()) {
        let mut reference_pool = world_clocks(&ops);
        let n = reference_pool.len();
        let mut reference = reference_pool.remove(i % n);
        if reverse {
            reference_pool.reverse();
        }
        for c in reference_pool {
            reference.join(c).expect("one world's clocks are pairwise disjoint");
        }

        let mut pool = world_clocks(&ops);
        let mut acc = pool.remove(i % n);
        if reverse {
            pool.reverse();
        }
        acc.join_all(pool).expect("one world's clocks are pairwise disjoint");
        prop_assert_eq!(acc, reference);
    }
}

// ───────────────────── the fold's up-front index, differentially ─────────────────────
//
// `Clock::join_all`'s up-front overlap test runs against a per-call index of
// the fixed accumulator's party; the index is a performance mechanism only, so
// every observable outcome — the hand-back vector (contents *and* order) and
// the accumulator's final party and version — must be exactly what the
// documented discipline decides. The recursive oracle's `join_all`
// (`oracle::Clock`) is that discipline's reference spelling. The id-level seam
// and the adversarial party mixes are pinned in `party/tests.rs`; these
// differentials pin the clock fold carrying versions through the same
// decisions.

/// On forked and aliased clock populations, the production fold and the
/// recursive oracle agree.
///
/// With no overlap anywhere — a forked clock population reuniting after
/// concurrent ticks — both return the same merged version and rebuild equal
/// accumulators; with the accumulator's own region duplicated among the inputs,
/// both hand back exactly the duplicate.
#[test]
fn join_all_agrees_with_oracle_on_forked_and_aliased_populations() {
    let population = |duplicate: bool| {
        let mut acc = Clock::seed();
        let mut children: Vec<Clock> = acc.forks(4).collect();
        for (n, child) in children.iter_mut().enumerate() {
            for _ in 0..n {
                child.tick();
            }
        }
        if duplicate {
            let dup = Clock::from_parts(acc.party().dangerously_alias(), acc.version().clone());
            children.insert(2, dup);
        }
        (acc, children)
    };
    let (acc, children) = population(false);
    assert_join_all_matches_recursive_oracle(acc, children);
    let (acc, children) = population(true);
    assert_join_all_matches_recursive_oracle(acc, children);
}

/// A group retained on the stack by a failed weight-1 combine — the over-full
/// counter slot — keeps coalescing with later inputs exactly as the recursive
/// oracle says, versions riding along.
///
/// The clock twin of the party suite's deterministic witness for the fold's
/// hand-back-retention arm (`fold.rs`, the failed-combine path whose newer
/// group has already coalesced): feed order [a, b, alias(a), c, d, e] over
/// pairwise-disjoint forks retains alias∪c on the stack at the failed weight-1
/// combine, then coalesces d∪e into it, so the hand-back is the four-input
/// group and the accumulator absorbs only a∪b — with each input carrying a
/// distinct ticked version, so the clock fold's version merges ride the same
/// decisions.
#[test]
fn join_all_agrees_with_oracle_on_aliased_coalesced_group() {
    let mut acc = Clock::seed();
    let mut children: Vec<Clock> = acc.forks(5).collect();
    for (n, child) in children.iter_mut().enumerate() {
        for _ in 0..=n {
            child.tick();
        }
    }
    let e = children.pop().expect("five forks");
    let d = children.pop().expect("five forks");
    let c = children.pop().expect("five forks");
    let b = children.pop().expect("five forks");
    let a = children.pop().expect("five forks");
    let alias = Clock::from_parts(a.party().dangerously_alias(), a.version().clone());
    assert_join_all_matches_recursive_oracle(acc, vec![a, b, alias, c, d, e]);
}

/// Run the production clock fold and the recursive oracle's `join_all` over one
/// input population and assert identical outcomes, compared over logical trees.
///
/// Identical outcomes: the same `Ok`/`Err` verdict — the returned version
/// lowering to the oracle accumulator's — the same hand-back vector (contents
/// *and* order, element-wise over `to_oracle_clock`), and accumulators (party
/// and version both) lowering to the same oracle trees.
fn assert_join_all_matches_recursive_oracle(mut acc: Clock, inputs: Vec<Clock>) {
    let lift = |c: &Clock| {
        let (p, v) = to_oracle_clock(c);
        oracle::Clock::from_parts(p, v)
    };
    let mut oracle_acc = lift(&acc);
    let oracle_inputs: Vec<oracle::Clock> = inputs.iter().map(lift).collect();
    let new = acc.join_all(inputs).cloned();
    let reference = oracle_acc.join_all(oracle_inputs);
    match (new, reference) {
        (Ok(version), Ok(())) => assert_eq!(
            to_oracle_version(&version),
            oracle_acc.version(),
            "the production fold and the oracle fold must return the same merged version"
        ),
        (Err(back), Err(oracle_back)) => {
            let back: Vec<_> = back.iter().map(to_oracle_clock).collect();
            let oracle_back: Vec<_> = oracle_back
                .into_iter()
                .map(oracle::Clock::into_parts)
                .collect();
            assert_eq!(
                back, oracle_back,
                "the production fold and the oracle fold must hand back the same clocks \
                 in the same order"
            );
        }
        (new, reference) => panic!(
            "the production fold and the oracle fold must agree on the verdict: \
             {new:?} vs {reference:?}"
        ),
    }
    assert_eq!(
        to_oracle_clock(&acc),
        (oracle_acc.party().clone(), oracle_acc.version()),
        "the production fold and the oracle fold must leave the same accumulator"
    );
}

proptest! {
    /// The production clock `join_all` decides exactly as the recursive
    /// oracle's `join_all` over arbitrary normal-form mixes.
    ///
    /// An arbitrary accumulator against clocks drawn with repetition from an
    /// arbitrary pool of party × version pairs — mixed sizes, duplicates, and
    /// every overlap disposition arise from the draws — with identical
    /// hand-backs (contents and order) and accumulators lowering to the same
    /// oracle trees.
    #[test]
    fn join_all_matches_the_recursive_oracle(
        oacc in (arb_oracle_party_nonempty(), arb_oracle_version()),
        (pool, picks) in proptest::collection::vec(
            (arb_oracle_party_nonempty(), arb_oracle_version()),
            1..5,
        )
        .prop_flat_map(|pool| {
            let len = pool.len();
            (Just(pool), proptest::collection::vec(0..len, 0..10))
        }),
    ) {
        let lower = |(p, v): &(oracle::Party, oracle::Version)| {
            Clock::from_parts(from_oracle_party(p), from_oracle_version(v))
        };
        let acc = lower(&oacc);
        let inputs: Vec<Clock> = picks.iter().map(|&i| lower(&pool[i])).collect();
        assert_join_all_matches_recursive_oracle(acc, inputs);
    }
}

proptest! {
    /// The clock observers match the oracle's: `has_seen` is `msg <= version`,
    /// `happens_before` is the strict causal order, and `concurrent_with` is
    /// incomparability.
    #[test]
    fn clock_observers_match_oracle(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let (oa, ob) = (&cs[i % n], &cs[j % n]);

        let ia = from_oracle_clock(oa);
        let ib = from_oracle_clock(ob);
        let msg_oracle = ob.version();
        let msg = from_oracle_version(&msg_oracle);

        prop_assert_eq!(ia.version() >= msg, oa.version() >= msg_oracle);
        prop_assert_eq!(ia.version() < ib.version(), oa.version() < ob.version());
        prop_assert_eq!(ia.version().concurrent(ib.version()), oa.concurrent_with(ob));
    }
}

proptest! {
    /// `own_version` (`version() / party()`: the clock's history within its own
    /// region) matches the oracle on every clock in a generated population.
    #[test]
    fn own_version_matches_oracle(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let oc = &cs[i % n];
        let ic = from_oracle_clock(oc);
        prop_assert_eq!(to_oracle_version(&ic.own_version().to_version()), oc.own_version());
    }
}

// ───────────────────────────── master differential harness ─────────────────────────────

proptest! {
    /// A seed-derived op trace, applied in lockstep to the oracle and the impl,
    /// agrees structurally on every live clock after every step, and all live
    /// impl parties stay pairwise disjoint.
    ///
    /// Pairwise disjointness means `join`/`sync` never error in correct usage.
    /// Agreement is by structural lowering — `to_oracle_clock` rebuilds the
    /// oracle's tree shape from the impl's internal packed bits — not via the
    /// byte codec, which the per-trace round-trip below exercises separately.
    #[test]
    fn master_differential(ops in world_strategy()) {
        let mut ora: Vec<oracle::Clock> = vec![oracle::Clock::seed()];
        let mut imp: Vec<Clock> = vec![Clock::seed()];

        for op in &ops {
            let n = ora.len();
            match *op {
                Op::Tick(i) => {
                    let i = i % n;
                    ora[i].tick();
                    imp[i].tick();
                }
                Op::Ticks(i, k) => {
                    let i = i % n;
                    // The oracle iterates; the impl's one fused call must
                    // land on the same structure.
                    for _ in 0..k {
                        ora[i].tick();
                    }
                    imp[i].ticks(u64::from(k));
                }
                Op::Fork(i) => {
                    let i = i % n;
                    let oc = ora[i].fork();
                    let ic = imp[i].fork();
                    ora.push(oc);
                    imp.push(ic);
                }
                Op::Send(i, j) => {
                    let (i, j) = (i % n, j % n);
                    let om = ora[i].send();
                    let im = imp[i].send().clone();
                    ora[j].receive(om);
                    imp[j].recv(&im);
                }
                Op::Sync(i, j) => {
                    let (i, j) = (i % n, j % n);
                    if i != j {
                        let hi = i.max(j);
                        let lo = i.min(j);
                        {
                            let (a, b) = ora.split_at_mut(hi);
                            a[lo].sync(&mut b[0]).expect("seed-derived parties are disjoint");
                        }
                        {
                            let (a, b) = imp.split_at_mut(hi);
                            a[lo].sync(&mut b[0]).expect("seed-derived parties are disjoint");
                        }
                    }
                }
                Op::Join(i, j) => {
                    if n > 1 {
                        let (i, j) = (i % n, j % n);
                        if i != j {
                            let ov = ora.remove(j);
                            let iv = imp.remove(j);
                            let i2 = if j < i { i - 1 } else { i };
                            ora[i2].join(ov).expect("seed-derived parties are disjoint");
                            imp[i2].join(iv).expect("seed-derived parties are disjoint");
                        }
                    }
                }
            }

            // Structural agreement on every live clock.
            prop_assert_eq!(ora.len(), imp.len());
            for (o, m) in ora.iter().zip(imp.iter()) {
                let (op_tree, ov_tree) = o.trees();
                let (mp_tree, mv_tree) = to_oracle_clock(m);
                prop_assert_eq!(&mp_tree, op_tree);
                prop_assert_eq!(&mv_tree, ov_tree);
            }

            // Disjointness invariant: all live impl parties pairwise disjoint.
            for a in 0..imp.len() {
                for b in (a + 1)..imp.len() {
                    prop_assert!(imp[a].party().is_disjoint(imp[b].party()));
                }
            }
        }

        // Per-trace codec exercise: every live clock round-trips through
        // decode∘encode and stays structurally identical (also confirms each
        // encoding is canonical, since `decode` strictly rejects
        // non-normal-form input).
        for m in &imp {
            let back = Clock::decode(&m.encode()[..]).expect("impl encodings are canonical");
            prop_assert_eq!(to_oracle_clock(&back), to_oracle_clock(m));
        }
    }
}

proptest! {
    /// Every should-be-equivalent encoding view of a live `Party`/`Version`/
    /// `Clock` agrees, over an arbitrary *impl-driven* history.
    ///
    /// The oracle-lowered round-trip tests (and [`master_differential`] above)
    /// only ever compare `encode`d bytes; the `as_bytes_matches_encode` tests
    /// only build via the oracle. Neither combination drives the impl's own
    /// `fork`/`join`/`sync` *and* reads `as_bytes` — the exact seam where a
    /// normalizing `join` once left stale bits in the stored buffer, so that
    /// `as_bytes` (the borsh wire form) diverged from the canonical `encode`.
    ///
    /// For each clock reached by the trace this asserts the three packed views
    /// coincide — `as_bytes == encode`, and `decode` of *either* recovers the
    /// value — and the textual [`Display`]/[`FromStr`] view round-trips.
    #[test]
    fn encoding_views_agree_over_impl_history(ops in world_strategy()) {
        let mut imp = vec![Clock::seed()];
        for op in &ops {
            step_impl(&mut imp, op);
        }
        for c in &imp {
            let (p, v) = (c.party(), c.version());

            // The raw stored bytes are canonical: identical to the re-packed
            // encoding, and decodable as either.
            let (pe, ve) = (p.encode(), v.encode());
            prop_assert_eq!(p.as_bytes(), pe.as_slice());
            prop_assert_eq!(&Party::decode(p.as_bytes()).unwrap(), p);
            prop_assert_eq!(&Party::decode(&pe[..]).unwrap(), p);

            prop_assert_eq!(v.as_bytes(), ve.as_slice());
            prop_assert_eq!(&Version::decode(v.as_bytes()).unwrap(), v);
            prop_assert_eq!(&Version::decode(&ve[..]).unwrap(), v);

            // The textual view round-trips for both components and the pair.
            prop_assert_eq!(&p.to_string().parse::<Party>().unwrap(), p);
            prop_assert_eq!(&v.to_string().parse::<Version>().unwrap(), v);

            let back = Clock::decode(&c.encode()[..]).unwrap();
            prop_assert_eq!(back.party(), p);
            prop_assert_eq!(back.version(), v);
            let parsed: Clock = c.to_string().parse().unwrap();
            prop_assert_eq!(parsed.party(), p);
            prop_assert_eq!(parsed.version(), v);
        }
    }
}

// ───────────────────────────── protocol semantics ─────────────────────────────

// The protocol-shape laws (fork preserves the version, peeks are stable, an
// own-message receive is a bare tick, send/recv advance strictly) live in
// `crate::laws` and are driven by the algebraic-laws suite; this file keeps the
// oracle differentials.

proptest! {
    /// After `a.sync(&mut b)`: both end at the oracle's result, their versions
    /// are equal, their parties are disjoint, and re-joining the two parties
    /// recovers the pre-sync merged party.
    #[test]
    fn sync(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        if n < 2 {
            return Ok(());
        }
        // Derive two distinct members directly rather than rejecting collisions
        // — small populations collide often, and `prop_assume` would blow the
        // reject cap under a high case count (see the oracle `sync` test).
        let i = i % n;
        let j = (i + 1 + j % (n - 1)) % n;

        // Oracle expectation, and the pre-sync merged party.
        let mut oa = cs[i].clone();
        let mut ob = cs[j].clone();
        oa.sync(&mut ob).expect("disjoint");
        let mut merged = cs[i].party().clone();
        merged.join(cs[j].party().clone()).expect("disjoint");

        // Impl sync.
        let mut ia = from_oracle_clock(&cs[i]);
        let mut ib = from_oracle_clock(&cs[j]);
        ia.sync(&mut ib).expect("disjoint");

        // Structural agreement with the oracle on both sides.
        let (oap, oav) = oa.trees();
        let (obp, obv) = ob.trees();
        prop_assert_eq!(to_oracle_clock(&ia), (oap.clone(), oav.clone()));
        prop_assert_eq!(to_oracle_clock(&ib), (obp.clone(), obv.clone()));

        // Versions equal, parties disjoint.
        prop_assert!(ia.version() == ib.version());
        prop_assert!(ia.party().is_disjoint(ib.party()));

        // Re-joining the re-split parties recovers the pre-sync merged party.
        let (pa, _) = ia.into_parts();
        let (pb, _) = ib.into_parts();
        let mut rejoined = pa;
        rejoined.join(pb).expect("disjoint after re-split");
        prop_assert!(rejoined == from_oracle_party(&merged));
    }

    /// The heterogeneous joins `Version|Version`, `Clock|Version`, and
    /// `Version|Clock` all match the oracle. The latter two encode the
    /// anonymous-as-party-0 identity: the version merges, the party is
    /// untouched.
    #[test]
    fn heterogeneous_joins(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let (i, j) = (i % n, j % n);
        let ov_i = cs[i].version();
        let ov_j = cs[j].version();

        // Version | Version.
        let exp_vv = ov_i.clone() | ov_j.clone();
        let got_vv = from_oracle_version(&ov_i) | from_oracle_version(&ov_j);
        prop_assert_eq!(to_oracle_version(&got_vv), exp_vv);

        // Clock | Version (party from the clock, versions joined).
        let exp_cv = cs[i].clone() | ov_j.clone();
        let got_cv = from_oracle_clock(&cs[i]) | from_oracle_version(&ov_j);
        let (cvp, cvv) = exp_cv.trees();
        prop_assert_eq!(to_oracle_clock(&got_cv), (cvp.clone(), cvv.clone()));

        // Version | Clock (party from the clock, versions joined).
        let exp_vc = ov_i.clone() | cs[j].clone();
        let got_vc = from_oracle_version(&ov_i) | from_oracle_clock(&cs[j]);
        let (vcp, vcv) = exp_vc.trees();
        prop_assert_eq!(to_oracle_clock(&got_vc), (vcp.clone(), vcv.clone()));
    }

    /// Assigning forms. The `Clock` assigning join surfaces merge the version
    /// and leave the party untouched, matching the oracle — complementing the
    /// by-value `Clock | Version` above.
    ///
    /// Covers `Clock |= Version` and `Clock |= &Version`.
    #[test]
    fn clock_assign_join_matches_oracle(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let (i, j) = (i % n, j % n);
        let msg_oracle = cs[j].version();

        // Oracle expectation: party unchanged, version merged.
        let expected = cs[i].clone() | msg_oracle.clone();
        let (ep, ev) = expected.trees();

        // `Clock |= Version`.
        let mut assign = from_oracle_clock(&cs[i]);
        assign |= from_oracle_version(&msg_oracle);
        prop_assert_eq!(to_oracle_clock(&assign), (ep.clone(), ev.clone()));

        // `Clock |= &Version`.
        let msg = from_oracle_version(&msg_oracle);
        let mut assign_ref = from_oracle_clock(&cs[i]);
        assign_ref |= &msg;
        prop_assert_eq!(to_oracle_clock(&assign_ref), (ep.clone(), ev.clone()));
    }
}

proptest! {
    /// `sync_all` reconciles one world's clocks: afterwards every participant
    /// holds the join of all the pre-sync versions, the re-shared parties are
    /// pairwise disjoint, and re-joining them recovers the pre-sync merged
    /// party.
    ///
    /// The byte-level pin to the composed spelling (`join_all` then the
    /// balanced re-share) is the `sync_all_is_join_all_then_forks` law; this
    /// differential samples the contract's invariants over organic
    /// populations, whose shares are not fork families of the receiver.
    #[test]
    fn sync_all_reconciles_one_world(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let i = i % n;

        // Pre-sync expectations on the oracle side: the version every
        // participant must end with, and the merged party the re-split shares
        // must recover.
        let expected = cs
            .iter()
            .map(|c| c.version())
            .reduce(|a, b| a | b)
            .expect("a world holds at least the seed clock");
        let mut merged = cs[0].party().clone();
        for c in &cs[1..] {
            merged
                .join(c.party().clone())
                .expect("one world's parties are pairwise disjoint");
        }

        // Impl sync_all: member `i` is the receiver, the rest the others.
        let mut others: Vec<Clock> = cs.iter().map(from_oracle_clock).collect();
        let mut receiver = others.remove(i);
        let returned = receiver
            .sync_all(others.iter_mut())
            .expect("one world's clocks are pairwise disjoint")
            .clone();

        // Every participant carries the merged version, the returned
        // reference included.
        prop_assert_eq!(to_oracle_version(&returned), expected.clone());
        for c in core::iter::once(&receiver).chain(&others) {
            prop_assert_eq!(to_oracle_version(c.version()), expected.clone());
        }

        // The re-shared parties are pairwise disjoint and re-join to the
        // pre-sync merged party.
        let all: Vec<&Clock> = core::iter::once(&receiver).chain(&others).collect();
        for (x, a) in all.iter().enumerate() {
            for b in &all[x + 1..] {
                prop_assert!(a.party().is_disjoint(b.party()));
            }
        }
        let mut rejoined = receiver.party().dangerously_alias();
        for c in &others {
            rejoined
                .join(c.party().dangerously_alias())
                .expect("re-split shares are disjoint");
        }
        prop_assert!(rejoined == from_oracle_party(&merged));
    }
}

// ───────────────────────── normal-form invariant ─────────────────────────

proptest! {
    /// Every value produced by every op is in canonical normal form, checked
    /// after every step of a seed-derived impl-only trace (lowered to oracle
    /// trees, which carry the `is_normal` predicate).
    #[test]
    fn ops_preserve_normal_form(ops in world_strategy()) {
        let mut imp = vec![Clock::seed()];
        for op in &ops {
            step_impl(&mut imp, op);
            for c in &imp {
                let (p, v) = to_oracle_clock(c);
                prop_assert!(p.is_normal(), "party not normal: {p:?}");
                prop_assert!(v.is_normal(), "version not normal: {v:?}");
            }
        }
    }
}

// ───────────────────────── encoded_bits ↔ encode ─────────────────────────

proptest! {
    /// `encoded_bits` is the pre-padding bit length of `encode`: for every
    /// live clock (and its party and version), `encode().len()` is
    /// `encoded_bits()` plus the final marker bit, rounded up to whole
    /// bytes.
    ///
    /// A `Clock` byte-concatenates its party and version (each
    /// marker-padded and byte-aligned), so its bit length is the
    /// *byte-aligned* party length plus the version's own bit length —
    /// the party's padding lies between the two parts, and only the
    /// version's final marker is left uncounted.
    #[test]
    fn encoded_bits_matches_encode_len(ops in world_strategy()) {
        for oc in &run(&ops) {
            let c = from_oracle_clock(oc);
            let (p, v) = (c.party(), c.version());
            prop_assert_eq!(c.encode().len(), (c.encoded_bits() + 1).div_ceil(8));
            prop_assert_eq!(p.encode().len(), (p.encoded_bits() + 1).div_ceil(8));
            prop_assert_eq!(v.encode().len(), (v.encoded_bits() + 1).div_ceil(8));
            prop_assert_eq!(c.encode().len(), p.encode().len() + v.encode().len());
        }
    }
}

// ───────────────────────────── robustness ─────────────────────────────

/// Deep structures (a depth-100k id spine, and the deep event tree a tick
/// builds over it) survive every public op, the codec, and the `Debug` printer
/// with no stack overflow.
///
/// Every library walk is iterative — depth lives on explicit heap and bit
/// stacks, never the call stack — and this test is the depth-100k proof of that
/// claim (the hard rule in the crate's AGENTS.md names it as such). Beyond the
/// single-clock ops (tick, fork, join, partial_cmp, `|`, encode, decode,
/// Debug), this drives the composite ops on deep structures: `sync` between two
/// deep clocks, `send`/`recv` of a deep version, and version comparison and
/// concurrency at depth. Impl-only: the recursive oracle cannot build or even
/// drop a tree this deep (oracle agreement at bounded depth is the master
/// differential harness's job).
#[test]
fn deep_tree_stack_safety() {
    const DEPTH: usize = 100_000;
    let party = deep_left_spine_party(DEPTH);
    let mut clock = Clock::from_parts(party, Version::new());

    // Codec over a deep id round-trips to canonical bytes.
    let bytes = clock.encode();
    let decoded = Clock::decode(&bytes[..]).expect("deep id encodes to canonical bytes");
    assert_eq!(decoded.encode(), bytes);

    // Ticks build, then refine, a deep event tree (fill, then the grow fallback,
    // on the stored stream).
    clock.tick();
    clock.tick();

    // Snapshot this deep version before the clock advances further. Used below
    // to drive the skyline join/meet sweep against a *distinct* deep version:
    // equal operands short-circuit on `codec::canonical_eq`'s byte compare at
    // the top of join/meet, so only distinct ones reach the full-length sweep.
    let early = clock.version().clone();

    // Observing ops over the deep version do not overflow.
    let v = clock.version();
    assert_eq!(v.partial_cmp(v), Some(core::cmp::Ordering::Equal));
    assert_eq!(v.clone() | v.clone(), *v);

    // Codec over a deep id + deep event tree round-trips.
    let bytes = clock.encode();
    assert_eq!(
        Clock::decode(&bytes[..])
            .expect("deep clock encodes canonically")
            .encode(),
        bytes
    );

    // `send`/`receive` over the deep clock: `send` extracts a deep version (and
    // ticks the clock's event tree), and a self-`receive` (the sent message is
    // `<= self`) merges a deep version into a deep clock without overflow.
    let msg = clock.send().clone();
    clock.recv(&msg);

    // A join and a meet of two *distinct* deep versions drive the skyline
    // join/meet sweep (the `|`/`&` walk) over the full deep encoding. `early`
    // predates the `send`/`recv` ticks, so `early < clock.version()`: their
    // meet (GLB) is the older `early`, their join (LUB) the newer current
    // version. The operands are distinct and non-empty, so neither the
    // `codec::canonical_eq` nor the empty-operand fast path fires — the
    // full-length sweep is genuinely exercised, not skipped.
    let current = clock.version().clone();
    assert!(early.clone() & current.clone() == early);
    assert!(early | current.clone() == current);

    // Observers over a deep clock and a deep message do not overflow:
    // `has_seen` lowers to a deep `causal_cmp` against the version, and the
    // clock-vs-clock observers compare two deep versions.
    let sent = clock.send().clone();
    assert!(clock.version() >= sent);
    assert_ne!(
        clock.version().partial_cmp(clock.version()),
        Some(core::cmp::Ordering::Greater)
    );
    assert!(!(clock.version().concurrent(clock.version())));

    // Fork (deep split + snapshot) yields a disjoint child; both halves stay deep.
    let mut child = clock.fork();
    assert!(clock.party().is_disjoint(child.party()));

    // `sync` between two deep clocks is the most complex composite (fork + join
    // + merge of deep structures). Drive it and assert it reconciles without
    // overflow: post-sync the two versions are equal, the parties stay
    // disjoint, and the observers agree they are neither strictly ordered nor
    // concurrent.
    clock.sync(&mut child).expect("fork halves are disjoint");
    assert!(clock.version() == child.version());
    assert!(clock.party().is_disjoint(child.party()));
    assert_ne!(
        clock.version().partial_cmp(child.version()),
        Some(core::cmp::Ordering::Less)
    );
    assert!(!(clock.version().concurrent(child.version())));

    // join restores the whole from the two deep halves.
    clock.join(child).expect("fork halves are disjoint");

    // The recursive Debug pretty-printer must not overflow either.
    assert!(!format!("{clock:?}").is_empty());
}

/// The query folds and causal-interval walks survive depth 100k.
///
/// Driven here: rank, distance, lag, `Ranked` ordering, the `Rank` wire
/// round-trip at a 100k exponent, span hulls (pair and n-ary), `Span`
/// validation, decode, placement, and dominance, the span algebra (all four
/// operators, the n-ary door, the quotient view), query membership and
/// coverage, and projection through a deep id.
///
/// `deep_tree_stack_safety` above proves the clock ops at this depth; this is
/// the same proof for the surfaces it does not drive — every one an iterative
/// walk whose depth lives on explicit heap or bit stacks, exercised here at a
/// depth no program stack could carry.
#[test]
fn deep_tree_query_and_causal_stack_safety() {
    use crate::causally;
    use crate::Rank;

    const DEPTH: usize = 100_000;
    let party = deep_left_spine_party(DEPTH);
    let mut clock = Clock::from_parts(party, Version::new());
    clock.tick();
    let early = clock.version().clone();
    clock.tick();
    let late = clock.version().clone();

    // Query folds at depth: the single-stream rank integral and the
    // fused pair co-sweeps (distance, lag, the Ranked signed compare).
    let r_early = early.rank();
    let r_late = late.rank();
    assert!(r_early < r_late);
    let d = early.distance(&late);
    assert_eq!(early.lag(&late) + late.lag(&early), d);
    assert!(early.ranked() < late.ranked());

    // The Rank wire form round-trips at a 100k-deep exponent.
    let bytes = r_late.encode();
    assert_eq!(Rank::decode(&bytes[..]).expect("canonical rank"), r_late);

    // Span hulls (pair and n-ary), the fused decode/admit walk, and the
    // 3-stream placement walks.
    let span = early.span(&late);
    assert_eq!(span.lo(), &early);
    assert_eq!(span.hi(), &late);
    let span_bytes = span.encode();
    let decoded = causally::Span::decode(&span_bytes[..]).expect("canonical span");
    assert_eq!(decoded.lo(), &early);
    let validated = causally::Span::new(&early, &late).expect("early <= late");
    let _ = validated.place(&early);
    let _ = validated.dominance(&late);
    let hull = early.span_all([late.clone()]);
    assert_eq!(hull.hi(), &late);

    // The span algebra at depth: each operator's legs run the join and meet
    // kernels over the deep endpoints, the n-ary door drives the balanced
    // fold's combine arms, and the quotient view runs the masked co-walks —
    // every constituent iterative, pinned here at the door.
    let head = causally::Span::new(&early, &early).expect("coincident");
    assert_eq!(&head + &span, span);
    assert_eq!(&head * &span, Some(head.clone()));
    assert_eq!(&head | &span, span);
    assert_eq!(&head & &span, head);
    assert_eq!(head.union_all([&span, &hull]), span);
    let view = &span / clock.party();
    let _ = view.place(&early);
    assert_eq!(view.to_span(), span);

    // Query classification: the fused multi-bound filter walks, and the
    // coverage clamp's lattice legs, over the deep streams.
    let query = causally::since(&early) & causally::before(&late);
    assert!(query.contains(&late));
    let _ = query.coverage(span.reborrow());

    // Projection through the deep id (the masked walk), materialized.
    let own = &late / clock.party();
    assert_eq!(own.to_version(), late);
}

/// The text mirror and the tick-floor fold survive depth 100k: a deep clock
/// renders to paper notation and parses back equal, and `min_ticks` runs its
/// epoch-ledger/min-web walk over the deep event tree.
///
/// `codec::tests::deep_id_text_roundtrip` proves the *id* text parser at this
/// depth; the event tree's text walk is a separate parser (its parked-stack
/// frames live in `version::skyline::text`), and nothing else drives it or
/// `min_ticks` past proptest depths. Both are iterative walks on explicit heap
/// stacks, exercised here at a depth no program stack could carry.
#[test]
fn deep_tree_text_and_min_ticks_stack_safety() {
    const DEPTH: usize = 100_000;
    let party = deep_left_spine_party(DEPTH);
    let mut clock = Clock::from_parts(party, Version::new());
    clock.tick();
    let version = clock.version().clone();

    // The version text mirror: render the deep event tree, parse it
    // back, and land on the same version.
    let text = version.to_string();
    let parsed: Version = text.parse().expect("a deep rendered version parses");
    assert_eq!(parsed, version);

    // The clock text mirror carries both deep components at once.
    let text = clock.to_string();
    let parsed: Clock = text.parse().expect("a deep rendered clock parses");
    assert_eq!(parsed.version(), &version);

    // The tick floor: one tick raised one leaf, so the minimal
    // construction is that single tick.
    assert_eq!(version.min_ticks(), crate::Ticks::from(1u64));
}

proptest! {
    /// `decode` of arbitrary bytes never panics; it returns `Ok` or `Err`.
    ///
    /// Any accepted value satisfies the keystone invariant `decode(b) == Ok(x)
    /// ⟹ is_normal(x)`: lowering it to the oracle yields a normal-form tree.
    /// This — not the re-encode round-trip alone — is what makes the
    /// byte-equality `Eq`/`Hash` sound: a non-normal accept would give two
    /// distinct byte strings for one logical value. The re-encode-then-decode
    /// round-trip is also asserted (canonical encoding is stable).
    #[test]
    fn decode_never_panics(bytes in prop::collection::vec(any::<u8>(), 0..512)) {
        if let Ok(p) = Party::decode(&bytes[..]) {
            prop_assert!(to_oracle_party(&p).is_normal(), "accepted a non-normal Party");
            prop_assert_eq!(Party::decode(&p.encode()[..]).ok(), Some(p));
        }
        if let Ok(v) = Version::decode(&bytes[..]) {
            prop_assert!(to_oracle_version(&v).is_normal(), "accepted a non-normal Version");
            prop_assert_eq!(Version::decode(&v.encode()[..]).ok(), Some(v));
        }
        if let Ok(c) = Clock::decode(&bytes[..]) {
            let (p, v) = to_oracle_clock(&c);
            prop_assert!(p.is_normal() && v.is_normal(), "accepted a non-normal Clock");
            let re = Clock::decode(&c.encode()[..]).expect("re-encode of an accepted clock is canonical");
            prop_assert_eq!(re.encode(), c.encode());
        }
    }
}

// ─────────────────────── decoded-component canonicity (regression) ───────────────────────
//
// `Clock::encode` lays the id directly before the event, so the event begins at
// a generally non-byte-aligned bit offset. A `decode` that extracts the event
// with `slice.to_bitvec()` keeps that head offset (rather than shifting to bit
// 0), leaving the recovered `Version`'s packed stream non-canonical:
// `version().encode()` mis-packs it and `Version::decode` then disagrees.
// Whole-clock round-trips hide this, because `Clock::encode` re-aligns each
// component via `extend_from_bitslice`; the bug only shows when a component
// extracted from a decoded clock is encoded on its own.

/// The seed's id is two bits, so its event starts at a non-byte-aligned offset.
///
/// Decoding the seed and re-encoding the recovered version must reproduce the
/// canonical encoding (and survive its own `decode`), not an offset-shifted
/// one.
#[test]
fn decoded_seed_version_encodes_canonically() {
    let seed = Clock::seed();
    let decoded = Clock::decode(&seed.encode()[..]).unwrap();
    assert_eq!(decoded.version().encode(), seed.version().encode());
    assert_eq!(
        &Version::decode(&decoded.version().encode()[..]).unwrap(),
        seed.version(),
    );
}

proptest! {
    /// For any seed-derived clock, decoding it preserves each component's
    /// canonical byte encoding, and the extracted party and version each
    /// round-trip through their own `decode`.
    ///
    /// Guards the whole class of
    /// non-byte-aligned offset extraction.
    #[test]
    fn decode_preserves_component_canonicity(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let original = from_oracle_clock(&cs[i % n]);
        let decoded = Clock::decode(&original.encode()[..]).expect("re-decode canonical clock");

        prop_assert_eq!(decoded.party().encode(), original.party().encode());
        prop_assert_eq!(decoded.version().encode(), original.version().encode());

        let v = decoded.version();
        prop_assert_eq!(&Version::decode(&v.encode()[..]).unwrap(), v);
        let p_bytes = decoded.party().encode();
        prop_assert_eq!(Party::decode(&p_bytes[..]).unwrap().encode(), p_bytes);
    }
}

// ───────────────────────────── worked example ─────────────────────────────

/// Paper §5.1's example run, step by step.
///
/// Seed forks to two; one ticks then forks; the other ticks twice; one of three
/// ticks while the other two sync; finally all rejoin to the whole space and a
/// tick collapses the event tree to a single integer. Mirrors the oracle's
/// `worked_example` on the impl.
#[test]
fn worked_example() {
    // Whole-space region check, computed structurally (parties are not `Clone`).
    let region = |clocks: &[&Clock]| {
        let mut acc = oracle::Party::Leaf(false);
        for c in clocks {
            acc.join(to_oracle_party(c.party()))
                .expect("participants own disjoint regions");
        }
        acc
    };

    // seed -> fork into two.
    let mut p1 = Clock::seed();
    let mut p2 = p1.fork();

    // p1 suffers one event, then forks.
    p1.tick();
    let mut p1a = p1.fork();
    let mut p1b = p1;

    // p2 suffers two events.
    p2.tick();
    p2.tick();

    // Three participants covering the whole space.
    assert_eq!(region(&[&p1a, &p1b, &p2]), oracle::Party::seed());

    // One participant ticks; the other two sync.
    let before = p1a.version().clone();
    p1a.tick();
    assert!(p1a.version() > before);

    let merged_region = {
        let mut acc = to_oracle_party(p1b.party());
        acc.join(to_oracle_party(p2.party())).expect("disjoint");
        acc
    };
    p1b.sync(&mut p2).expect("disjoint");

    // Sync reconciled histories and preserved total ownership of the two halves.
    assert!(p1b.version() == p2.version());
    let mut rejoined = to_oracle_party(p1b.party());
    rejoined
        .join(to_oracle_party(p2.party()))
        .expect("disjoint");
    assert_eq!(rejoined, merged_region);
    assert_eq!(region(&[&p1a, &p1b, &p2]), oracle::Party::seed());

    // Rejoin all three (recovering id = 1) and tick: the id owns the whole
    // space, so the event tree collapses to a single integer.
    let mut whole = p1a;
    whole.join(p1b).expect("disjoint");
    whole.join(p2).expect("disjoint");
    assert_eq!(to_oracle_party(whole.party()), oracle::Party::seed());
    whole.tick();
    assert!(
        matches!(to_oracle_version(whole.version()), oracle::Version::Leaf(_)),
        "post-join event should collapse to a single integer, got {:?}",
        whole.version()
    );
}

// ───────────────────── Display / FromStr / TryFrom (paper notation) ─────────────────────

proptest! {
    /// `Display` then `FromStr` round-trips for every type, and the printed
    /// form is the canonical paper notation (re-parsing yields the same value).
    #[test]
    fn display_fromstr_roundtrip(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let p = from_oracle_party(cs[i % n].party());
        let v = from_oracle_version(&cs[i % n].version());
        let c = from_oracle_clock(&cs[i % n]);

        let ps = p.to_string();
        prop_assert_eq!(ps.parse::<Party>().expect("Display is valid paper notation"), p);

        let vs = v.to_string();
        prop_assert_eq!(vs.parse::<Version>().expect("Display is valid paper notation"), v);

        let cstr = c.to_string();
        let cparsed: Clock = cstr.parse().expect("Display is valid paper notation");
        prop_assert_eq!(cparsed.encode(), c.encode());
    }
}

/// Display renders the paper's notation exactly (id `0/1/(l, r)`, event `n/(n,
/// e1, e2)`, stamp `(i, e)`), matching the paper's §5 examples.
#[test]
fn display_matches_paper_notation() {
    assert_eq!(Party::seed().to_string(), "1");
    assert_eq!(Version::new().to_string(), "0");
    assert_eq!(Clock::seed().to_string(), "(1, 0)");

    let id: Party = "((0, (1, 0)), (1, 0))".parse().unwrap();
    assert_eq!(id.to_string(), "((0, (1, 0)), (1, 0))");

    let ev: Version = "(1, 2, (0, (1, 0, 2), 0))".parse().unwrap();
    assert_eq!(ev.to_string(), "(1, 2, (0, (1, 0, 2), 0))");

    // Arbitrary-precision bases round-trip: a base past `u64::MAX` (2^64)
    // parses, re-renders, and decodes unchanged — there is no integer-width
    // cap.
    let wide: Version = "(18446744073709551616, 0, 1)".parse().unwrap();
    assert_eq!(wide.to_string(), "(18446744073709551616, 0, 1)");
    assert_eq!(Version::decode(&wide.encode()[..]).unwrap(), wide);

    // Debug is the same as Display.
    assert_eq!(format!("{id:?}"), "((0, (1, 0)), (1, 0))");
    assert_eq!(format!("{ev:?}"), "(1, 2, (0, (1, 0, 2), 0))");
    assert_eq!(
        format!("{:?}", Clock::seed()),
        "Clock { party: 1, version: 0 }"
    );
}

/// `TryFrom` literals build the same values as the equivalent paper-notation
/// strings, grounding out in the `bool`/`u8`/`u64` base cases.
#[test]
fn tryfrom_literals_build_values() {
    let p = Party::try_from((1, (0, 1))).unwrap();
    assert_eq!(p, "(1, (0, 1))".parse::<Party>().unwrap());

    let p = Party::try_from((true, false)).unwrap();
    assert_eq!(p, "(1, 0)".parse::<Party>().unwrap());

    let v = Version::try_from((1u64, 0u64, (2u64, 0u64, 1u64))).unwrap();
    assert_eq!(v, "(1, 0, (2, 0, 1))".parse::<Version>().unwrap());

    let c = Clock::try_from(((1u8, 0u8), 5u64)).unwrap();
    assert_eq!(c.encode(), "((1, 0), 5)".parse::<Clock>().unwrap().encode());

    // Base cases. `1` is a valid party; `0` is anonymous on its own but fine as
    // a sub-tree (see the `(0, 1)` cases above).
    assert_eq!(Party::try_from(1u8).unwrap().to_string(), "1");
    assert_eq!(Party::try_from(0u8), Err(Parse::Anonymous));
    assert_eq!(Party::try_from(false), Err(Parse::Anonymous));
    assert_eq!(Version::try_from(7u64).unwrap().to_string(), "7");
}

/// `FromStr` and `TryFrom` reject both malformed input and
/// well-formed-but-denormal input, mirroring `decode`'s strictness.
#[test]
fn fromstr_tryfrom_reject_denormal_and_syntax() {
    // Denormal (well-formed but not canonical).
    assert_eq!("(1, 1)".parse::<Party>(), Err(Parse::NotCanonical));
    assert_eq!(Party::try_from((1u8, 1u8)), Err(Parse::NotCanonical));
    assert_eq!("(5, 3, 3)".parse::<Version>(), Err(Parse::NotCanonical));
    assert_eq!("(1, 2, 3)".parse::<Version>(), Err(Parse::NotCanonical));
    assert_eq!(
        Version::try_from((1u64, 2u64, 3u64)),
        Err(Parse::NotCanonical)
    );

    // Syntax (malformed).
    assert_eq!("(1, 2".parse::<Party>(), Err(Parse::Syntax)); // unbalanced
    assert_eq!("2".parse::<Party>(), Err(Parse::Syntax)); // id leaves are only 0/1
    assert_eq!(Party::try_from(2u8), Err(Parse::Syntax));
    assert_eq!("".parse::<Version>(), Err(Parse::Syntax)); // empty
    assert_eq!("(1, 0)".parse::<Version>(), Err(Parse::Syntax)); // event needs 3 parts
    assert_eq!("(café, 0)".parse::<Clock>().err(), Some(Parse::Syntax)); // non-ASCII byte

    // Anonymous identity `0` is rejected as a standalone party (but allowed as
    // a sub-tree, exercised in `tryfrom_literals_build_values`).
    assert_eq!("0".parse::<Party>(), Err(Parse::Anonymous));
    assert_eq!(Party::try_from(0u8), Err(Parse::Anonymous));
    assert_eq!("(0, 1)".parse::<Party>().unwrap().to_string(), "(0, 1)"); // 0 as sub-tree: ok

    // Clock has no `PartialEq`, so compare the error directly.
    assert_eq!("(0, 5)".parse::<Clock>().err(), Some(Parse::Anonymous)); // anonymous party
    assert_eq!(Clock::try_from((0u8, 5u64)).err(), Some(Parse::Anonymous));

    // Whitespace is tolerated.
    assert_eq!(
        " ( 1 , ( 0 , 1 ) ) ".parse::<Party>().unwrap().to_string(),
        "(1, (0, 1))"
    );
}

// ───────────────────────────── serde (feature-gated) ─────────────────────────────

#[cfg(feature = "serde")]
proptest! {
    /// Every value round-trips through serde (here `serde_json`), since it
    /// serializes as its canonical encoding and deserializes back through the
    /// strict validator.
    #[test]
    fn serde_roundtrip(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let p = from_oracle_party(cs[i % n].party());
        let v = from_oracle_version(&cs[i % n].version());
        let c = from_oracle_clock(&cs[i % n]);

        let p2: Party = serde_json::from_slice(&serde_json::to_vec(&p).unwrap()).unwrap();
        let v2: Version = serde_json::from_slice(&serde_json::to_vec(&v).unwrap()).unwrap();
        let c2: Clock = serde_json::from_slice(&serde_json::to_vec(&c).unwrap()).unwrap();

        prop_assert_eq!(p2, p);
        prop_assert_eq!(v2, v);
        prop_assert_eq!(c2.encode(), c.encode());
    }

    /// `serde_json` represents `serialize_bytes` as a JSON number-array,
    /// decoded back via `visit_seq` — so it never exercises the binary
    /// `serialize_bytes`/`visit_bytes` path.
    ///
    /// Pin that path through two non-JSON formats: `postcard`
    /// (non-self-describing, length-prefixed bytes) and `ciborium`
    /// (self-describing CBOR, which emits a *typed* byte-string — CBOR major
    /// type 2). Every type must round-trip through both: the serialized form is
    /// the canonical encoding, deserialization re-validates it, and the CBOR
    /// typed-bytes path is the one `serde_json` alone can never reach.
    #[test]
    fn serde_roundtrip_binary_formats(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let p = from_oracle_party(cs[i % n].party());
        let v = from_oracle_version(&cs[i % n].version());
        let c = from_oracle_clock(&cs[i % n]);

        // postcard: non-self-describing binary format.
        let p2: Party = postcard::from_bytes(&postcard::to_allocvec(&p).unwrap()).unwrap();
        let v2: Version = postcard::from_bytes(&postcard::to_allocvec(&v).unwrap()).unwrap();
        let c2: Clock = postcard::from_bytes(&postcard::to_allocvec(&c).unwrap()).unwrap();
        prop_assert_eq!(&p2, &p);
        prop_assert_eq!(&v2, &v);
        prop_assert_eq!(c2.encode(), c.encode());

        // ciborium: self-describing CBOR. Each value must serialize as a byte
        // string (major type 2) and deserialize back through `Vec<u8>`'s
        // `visit_bytes`.
        let cbor = |bytes: &[u8]| -> u8 { bytes[0] >> 5 };

        let mut b = Vec::new();
        ciborium::ser::into_writer(&p, &mut b).unwrap();
        prop_assert_eq!(cbor(&b), 2, "Party did not serialize as a CBOR byte string");
        let p3: Party = ciborium::de::from_reader(&b[..]).unwrap();
        prop_assert_eq!(&p3, &p);

        let mut b = Vec::new();
        ciborium::ser::into_writer(&v, &mut b).unwrap();
        prop_assert_eq!(cbor(&b), 2, "Version did not serialize as a CBOR byte string");
        let v3: Version = ciborium::de::from_reader(&b[..]).unwrap();
        prop_assert_eq!(&v3, &v);

        let mut b = Vec::new();
        ciborium::ser::into_writer(&c, &mut b).unwrap();
        prop_assert_eq!(cbor(&b), 2, "Clock did not serialize as a CBOR byte string");
        let c3: Clock = ciborium::de::from_reader(&b[..]).unwrap();
        prop_assert_eq!(c3.encode(), c.encode());
    }

    /// The serde byte payload is exactly the canonical encoding.
    ///
    /// Each type serializes to the same stream as its own `encode()` bytes
    /// handed to the format as a plain byte sequence, so the wire form is
    /// `encode()` with nothing added, reordered, or wrapped — the serde mirror
    /// of the borsh `bytes == as_bytes` pin, witnessed through postcard, whose
    /// byte-sequence framing is a length prefix plus the raw bytes.
    #[test]
    fn serde_bytes_pin_the_canonical_encoding(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let p = from_oracle_party(cs[i % n].party());
        let v = from_oracle_version(&cs[i % n].version());
        let c = from_oracle_clock(&cs[i % n]);

        prop_assert_eq!(
            postcard::to_allocvec(&p).unwrap(),
            postcard::to_allocvec(&p.encode()).unwrap(),
        );
        prop_assert_eq!(
            postcard::to_allocvec(&v).unwrap(),
            postcard::to_allocvec(&v.encode()).unwrap(),
        );
        prop_assert_eq!(
            postcard::to_allocvec(&c).unwrap(),
            postcard::to_allocvec(&c.encode()).unwrap(),
        );
    }

    /// Serde deserialization runs the strict `decode` validator: a
    /// non-canonical payload is rejected, never silently accepted.
    ///
    /// The serde mirror of the borsh strict-reject leg, for all three types and
    /// through both deserialization paths.
    #[test]
    fn serde_rejects_non_canonical(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let p = from_oracle_party(cs[i % n].party());
        let v = from_oracle_version(&cs[i % n].version());
        let c = from_oracle_clock(&cs[i % n]);

        // Append a spurious whole zero byte: canonical padding is < 8 bits, so
        // `decode` rejects it, and serde must surface that rejection through
        // both the binary (typed-bytes) and the self-describing (number-array)
        // paths.
        let mut party_body = p.encode();
        party_body.push(0x00);
        prop_assume!(Party::decode(&party_body[..]).is_err());
        let mut version_body = v.encode();
        version_body.push(0x00);
        prop_assume!(Version::decode(&version_body[..]).is_err());
        let mut clock_body = c.encode();
        clock_body.push(0x00);
        prop_assume!(Clock::decode(&clock_body[..]).is_err());

        let postcard_frame =
            |body: &Vec<u8>| postcard::to_allocvec(body).expect("byte vectors serialize");
        prop_assert!(postcard::from_bytes::<Party>(&postcard_frame(&party_body)).is_err());
        prop_assert!(postcard::from_bytes::<Version>(&postcard_frame(&version_body)).is_err());
        prop_assert!(postcard::from_bytes::<Clock>(&postcard_frame(&clock_body)).is_err());

        let json_frame =
            |body: &Vec<u8>| serde_json::to_vec(body).expect("byte vectors serialize");
        prop_assert!(serde_json::from_slice::<Party>(&json_frame(&party_body)).is_err());
        prop_assert!(serde_json::from_slice::<Version>(&json_frame(&version_body)).is_err());
        prop_assert!(serde_json::from_slice::<Clock>(&json_frame(&clock_body)).is_err());
    }
}

// ───────────────────────────── borsh (feature-gated) ─────────────────────────────

#[cfg(feature = "borsh")]
proptest! {
    /// Every value round-trips through its raw canonical borsh encoding.
    #[test]
    fn borsh_roundtrip(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let p = from_oracle_party(cs[i % n].party());
        let v = from_oracle_version(&cs[i % n].version());
        let c = from_oracle_clock(&cs[i % n]);

        let p2: Party = borsh::from_slice(&borsh::to_vec(&p).unwrap()).unwrap();
        let v2: Version = borsh::from_slice(&borsh::to_vec(&v).unwrap()).unwrap();
        let c2: Clock = borsh::from_slice(&borsh::to_vec(&c).unwrap()).unwrap();

        prop_assert_eq!(p2, p);
        prop_assert_eq!(v2, v);
        prop_assert_eq!(c2.encode(), c.encode());
    }

    /// The borsh payload is exactly the canonical in-memory representation, and
    /// the representation is self-delimiting, so two concatenated values decode
    /// back in order.
    #[test]
    fn borsh_frames_as_canonical_bytes(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let v = from_oracle_version(&cs[i % n].version());

        let encoded = borsh::to_vec(&v).unwrap();
        prop_assert_eq!(encoded.as_slice(), v.as_bytes());

        // Two concatenated versions decode back in order: proof of self-delimitation.
        let mut buf = encoded.clone();
        buf.extend_from_slice(&encoded);
        let mut reader = &buf[..];
        let a = <Version as borsh::BorshDeserialize>::deserialize_reader(&mut reader).unwrap();
        let b = <Version as borsh::BorshDeserialize>::deserialize_reader(&mut reader).unwrap();
        prop_assert_eq!(a, v.clone());
        prop_assert_eq!(b, v);
    }

    /// Deserialization runs the strict `decode` validator: a frame whose body
    /// is not a canonical encoding is rejected rather than silently accepted.
    #[test]
    fn borsh_rejects_non_canonical(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let v = from_oracle_version(&cs[i % n].version());

        // Append a spurious whole zero byte: canonical padding is < 8 bits, so
        // `decode` rejects this, and borsh must surface that rejection.
        let mut body = v.encode();
        body.push(0x00);
        prop_assume!(Version::decode(&body[..]).is_err());
        prop_assert!(borsh::from_slice::<Version>(&body).is_err());
    }
}

// ───────────────────── orbit pins: iterated-operation size trajectories ─────────────────────
//
// A per-call cost bound does not preclude compounding: an operation linear in
// its input can feed itself an ever-larger input, so the size trajectory of
// *iterated* operation is its own pin surface. These orbits are fully
// deterministic (fixed populations, fixed arithmetic schedules, no randomness
// in any operation), so every trajectory below is pinned by exact measured
// numbers, asserted across the whole orbit — shape over point: tuning any one
// round cannot pass. The two scenario orbits transcribe the ITC 2008 paper's §6
// experiment (also reproduced statistically by
// `examples/space_consumption.rs`): the paper's observed shape — rapid early
// growth, then stabilization with a minor logarithmic component — is here a
// committed criterion, not a chart.

/// Build the scenario orbits' fixed population: `n` clocks balanced- forked
/// from one seed, deterministically.
fn orbit_population(n: usize) -> Vec<Clock> {
    let mut clocks = vec![Clock::seed()];
    let children: Vec<Clock> = clocks[0].forks(n as u64 - 1).collect();
    clocks.extend(children);
    clocks
}

/// Max over each octave `[2^i, 2^(i+1))` of a per-round trajectory (`traj[k -
/// 1]` is the reading after round `k`), starting at octave `[4, 8)`: the
/// resolution the scenario orbits' bands are pinned at.
fn octave_maxima(traj: &[usize]) -> Vec<usize> {
    let mut maxima = Vec::new();
    let mut hi = 8usize;
    while hi <= traj.len() {
        maxima.push(*traj[hi / 2..hi].iter().max().expect("octaves are nonempty"));
        hi *= 2;
    }
    maxima
}

/// The fork+join round-trip orbit is byte-stationary.
///
/// Forking a child off a clock and immediately joining it back returns the
/// clock byte-identical to its resting encoding, every round — iterated
/// re-partitioning of an idle region mints nothing, with no transient and no
/// ratchet [measured: identity at all 256 rounds].
///
/// Liveness floor: mid-round the encoding must differ from the resting one (the
/// fork really split the party), so the identity is a round trip, not a no-op.
/// Budget: 256 rounds, microseconds.
#[test]
fn fork_join_round_trip_orbit_is_byte_stationary() {
    let mut c = Clock::seed();
    let resting = c.encode();
    for k in 1u32..=256 {
        let child = c.fork();
        assert_ne!(
            c.encode(),
            resting,
            "round {k}: the fork must split the resting party"
        );
        c.join(child).expect("a clock's own fork is disjoint");
        assert_eq!(
            c.encode(),
            resting,
            "round {k}: the round trip must return the clock byte-identical"
        );
    }
}

/// The fork+tick+join round-trip orbit grows only the counter's code width.
///
/// Forking a child, ticking it once, and joining it back leaves the party
/// byte-identical to the seed every round, and the version — a fixed two-leaf
/// scaffold holding one counter at the child's leaf, `(0, 0, k)` exactly —
/// reads exactly `7 + 2·⌊log2 k⌋` encoded bits after round k: the k accumulated
/// events cost one gamma code's width (2 bits per doubling), never a ratcheting
/// tree [measured: exact at all 512 rounds].
///
/// Liveness floor: the exact form at `k = 512` is the floor — a round trip that
/// dropped events would read a smaller counter. Budget: 512 rounds,
/// milliseconds.
#[test]
fn fork_tick_join_orbit_returns_party_and_grows_gamma() {
    let mut c = Clock::seed();
    let seed_party = c.party().encode();
    for k in 1usize..=512 {
        let mut child = c.fork();
        child.tick();
        c.join(child).expect("a clock's own fork is disjoint");
        assert_eq!(
            c.party().encode(),
            seed_party,
            "round {k}: the party must return to the seed"
        );
        assert_eq!(
            c.version().encoded_bits(),
            7 + 2 * k.ilog2() as usize,
            "version bits after round {k}"
        );
    }
    let expected: Version = "(0, 0, 512)".parse().expect("test literals parse");
    assert_eq!(
        c.version(),
        &expected,
        "the orbit's whole history is one counter at the child's leaf"
    );
}

/// The paper's dynamic (churn) scenario reaches a bounded steady state.
///
/// Over a population held at 8 by one fork, one tick, one anonymous version
/// exchange, and one retiring join per round on a fixed arithmetic schedule,
/// the population's maximum id size plateaus in a fixed band — the octave
/// maxima climb through a transient and then sit flat at 132–134 bits, tail
/// octaves no higher than the plateau's first — and its maximum version size
/// grows only with the counters' code widths: per-octave growth bounded by a
/// constant that is itself shrinking (65 → 48 → 42 bits per doubling over the
/// tail), logarithmic in the round count, never per round. Both trajectories
/// are pinned exactly at octave resolution [measured: the two arrays below,
/// 4096 rounds].
///
/// Liveness floor: the population count is asserted every round and the pinned
/// arrays are strictly positive and rising through the transient — a scenario
/// that stopped forking, ticking, or joining would flatten them. Budget: 4096
/// rounds over ≤ 470-bit values, well under a second.
#[test]
fn churn_orbit_sizes_reach_a_bounded_band() {
    const N: usize = 8;
    const ROUNDS: usize = 4096;
    let mut clocks = orbit_population(N);
    let mut party_max = Vec::with_capacity(ROUNDS);
    let mut version_max = Vec::with_capacity(ROUNDS);
    for r in 0..ROUNDS {
        // fork: a new peer joins, population N -> N + 1.
        let parent = r % clocks.len();
        let child = clocks[parent].fork();
        clocks.push(child);
        // event: one peer records an internal event.
        let who = (r * 3 + 1) % clocks.len();
        clocks[who].tick();
        // anonymous exchange: one peer's version joined by another.
        let s = (r * 5 + 2) % clocks.len();
        let mut t = (r * 7 + 3) % clocks.len();
        if t == s {
            t = (t + 1) % clocks.len();
        }
        let peeked = clocks[s].version().clone();
        clocks[t] |= peeked;
        // retire: a donor's id is joined back into a survivor, N + 1 -> N.
        let donor = clocks.swap_remove((r * 11 + 5) % clocks.len());
        let survivor = (r * 13 + 7) % clocks.len();
        clocks[survivor]
            .join(donor)
            .expect("clocks forked from one seed are disjoint");
        assert_eq!(clocks.len(), N, "round {r}: churn holds the population");
        party_max.push(
            clocks
                .iter()
                .map(|c| c.party().encoded_bits())
                .max()
                .expect("the population is nonempty"),
        );
        version_max.push(
            clocks
                .iter()
                .map(|c| c.version().encoded_bits())
                .max()
                .expect("the population is nonempty"),
        );
    }

    let party_octaves = octave_maxima(&party_max);
    assert_eq!(
        party_octaves,
        [20, 26, 42, 70, 92, 100, 122, 134, 132, 132],
        "max id bits per octave: transient, then a flat band"
    );
    let plateau = party_octaves[7];
    assert!(
        party_octaves[8..].iter().all(|&m| m <= plateau),
        "id sizes must not creep past the plateau's first octave"
    );

    let version_octaves = octave_maxima(&version_max);
    assert_eq!(
        version_octaves,
        [24, 58, 102, 126, 176, 226, 270, 335, 383, 425],
        "max version bits per octave: growth per doubling, not per round"
    );
    for w in version_octaves[6..].windows(2) {
        assert!(
            w[1] - w[0] <= 65,
            "tail version growth must stay a bounded step per doubling"
        );
    }
}

/// The paper's static scenario stabilizes at the causal-history bound.
///
/// A fixed set of 8 peers recording one internal event and one anonymous
/// version exchange per round on a fixed arithmetic schedule keeps every id
/// byte-identical forever (messages carry no id), and the population's maximum
/// version size is monotone nondecreasing and grows exactly 8 bits per doubling
/// of the round count — the eight counters' gamma widths, 1 bit each per
/// doubling — reading exactly `8·i − 4` bits over the octave ending at round
/// `2^i`, flat per round, logarithmic in total [measured: exact at octave
/// resolution, 4096 rounds].
///
/// Liveness floor: the closed form's equality at every octave is the floor —
/// peers that stopped ticking or exchanging would read short. Budget: 4096
/// rounds over ≤ 100-bit versions, well under a second.
#[test]
fn static_orbit_ids_freeze_and_versions_grow_log() {
    const N: usize = 8;
    const ROUNDS: usize = 4096;
    let mut clocks = orbit_population(N);
    let resting_ids: Vec<Vec<u8>> = clocks.iter().map(|c| c.party().encode()).collect();
    let mut version_max = Vec::with_capacity(ROUNDS);
    for r in 0..ROUNDS {
        // internal event on one peer.
        clocks[r % N].tick();
        // anonymous exchange: one peer's version joined by another.
        let s = (r * 3 + 1) % N;
        let mut t = (r * 5 + 2) % N;
        if t == s {
            t = (t + 1) % N;
        }
        let peeked = clocks[s].version().clone();
        clocks[t] |= peeked;
        for (c, resting) in clocks.iter().zip(&resting_ids) {
            assert_eq!(
                &c.party().encode(),
                resting,
                "round {r}: a static peer's id must stay byte-identical"
            );
        }
        version_max.push(
            clocks
                .iter()
                .map(|c| c.version().encoded_bits())
                .max()
                .expect("the population is nonempty"),
        );
    }

    assert!(
        version_max.windows(2).all(|w| w[1] >= w[0]),
        "the shared causal history only accumulates"
    );
    let octaves = octave_maxima(&version_max);
    assert_eq!(octaves[0], 22, "the transient octave [4, 8)");
    for (j, &m) in octaves.iter().enumerate().skip(1) {
        let i = j + 3; // octave j ends at round 2^(j + 3)
        assert_eq!(
            m,
            8 * i - 4,
            "max version bits over the octave ending at 2^{i}"
        );
    }
}
