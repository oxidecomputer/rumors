# Witness constructions

## party-1

Outcome: demonstrated

Witness agent: 1

Test code:

```rust
#[global_allocator]
static HEAP: PeakAlloc = PeakAlloc;

fn party_of(p: &meter::Packed) -> Party {
    Party::decode(&p.bytes[..]).expect("generated shape decodes")
}

/// Peak heap delta over `f`, in bytes, plus `f`'s result (kept alive).
fn peak_of<R>(f: impl FnOnce() -> R) -> (usize, R) {
    HEAP.reset_peak_usage();
    let base = HEAP.current_usage();
    let r = f();
    (HEAP.peak_usage().saturating_sub(base), r)
}

#[test]
fn w_party_1_covers_and_is_disjoint_allocate_on_the_divert_pair() {
    let a = party_of(&Shape::IdSpine.packed_flagged(1_000, false));
    let b = party_of(&Shape::IdSpine.packed_flagged(1_000, true));
    let a2 = party_of(&Shape::IdSpine.packed_flagged(1_000, false));
    let (covers_peak, covers) = peak_of(|| black_box(a.covers(&b)));
    let (disjoint_peak, disjoint) = peak_of(|| black_box(a.is_disjoint(&b)));
    let (control_covers_peak, control_covers) = peak_of(|| black_box(a.covers(&a2)));
    let (control_disjoint_peak, control_disjoint) = peak_of(|| black_box(a.is_disjoint(&a2)));
    eprintln!(
        "MEASURED party-1: divert pair covers_peak_heap={covers_peak}B is_disjoint_peak_heap={disjoint_peak}B (covers={covers} disjoint={disjoint}); \
         unary control covers_peak_heap={control_covers_peak}B is_disjoint_peak_heap={control_disjoint_peak}B (covers={control_covers} disjoint={control_disjoint})"
    );
    assert!(!covers && disjoint, "the divert arms are disjoint");
    assert!(control_covers && !control_disjoint, "a spine covers its re-decoded twin");
    assert!(covers_peak > 0, "party-1 refuted: covers allocated nothing on the divert pair");
    assert!(disjoint_peak > 0, "party-1 refuted: is_disjoint allocated nothing on the divert pair");
}
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --no-capture -E 'test(/^w_/) | test(zz_witness_40) | test(inverted_verdict_tripwires_match_the_committed_roster) | test(band_tests_and_registry_citations_stay_paired) | test(superlinear_tripwires_match_the_committed_roster) | test(skyline_validate_dense_envelope) | test(skyline_validate_alt_spine_envelope) | test(skyline_decode_dense_envelope) | test(eq_exit_cost_is_tail_independent_on_first_interval_refuted_pair) | test(span_place_scans_each_stream_once) | test(query_single_bound_matches_the_pair_sweep_limbs)'
```

Output:

```text
MEASURED party-1: divert pair covers_peak_heap=8B is_disjoint_peak_heap=8B (covers=false disjoint=true); unary control covers_peak_heap=0B is_disjoint_peak_heap=0B (covers=true disjoint=false)
        PASS [   0.004s] (18/25) before::zz_witness_1 w_party_1_covers_and_is_disjoint_allocate_on_the_divert_pair
```

Explanation:

Verified by run (integration test with PeakAlloc as the binary's global allocator, in crates/before/tests/zz_witness_1.rs). On the diverted id-spine pair I(1000,false)/I(1000,true) both `covers` and `is_disjoint` raise the peak heap by 8 bytes (the lockstep BitsBuf's byte vector at the one both-present node pair, compare.rs:161-165), matching the committed ID_COVERS/ID_DISJOINT envelope basis (10 = 8 x 1.25). The control pair (an identical spine re-decoded, no both-present node) allocates 0 bytes, isolating the allocation to the queued right pair. The contract text `O(|self| + |other|), no allocation` is at before-fuelscape/src/ops.rs:821 (party_is_disjoint) and :835 (read by grep; the second is the covers island).

## party-13

Outcome: inconclusive

Witness agent: 1

Test code:

```rust
// No test constructed: the panic needs usize::MAX < 2^32 + 1, i.e. a 32-bit target.
// On this 64-bit host `Party::seed().forks(u64::from(u32::MAX) + 1).len()` cannot reach
// the (usize::MAX, None) size_hint arm.
```

Command:

```text
rustc --print sysroot; grep -n 'fn len\|assert_eq!(upper, Some(lower))' <sysroot>/lib/rustlib/src/rust/library/core/src/iter/traits/exact_size.rs
```

Output:

```text
   116:    fn len(&self) -> usize {
   122:        assert_eq!(upper, Some(lower));
```

Explanation:

Platform-untestable here (64-bit host; wasm32 runs are outside my permitted commands). The mechanism is confirmed by reading: forks.rs:65-73 returns `(usize::try_from(self.remaining).unwrap_or(usize::MAX), usize::try_from(self.remaining).ok())`, forks.rs:76 and :132 are bare `impl ExactSizeIterator for Split {}` / `for Forks<'_> {}`, and the installed toolchain's core source (exact_size.rs:116-122) shows the default `len` is `assert_eq!(upper, Some(lower))`. So on any 32-bit target `len()` panics once `remaining > usize::MAX`; tests/forks_max.rs:26 and :49 sidestep it with `.expect("64-bit test host")` (read). Not run.

## version-core-5

Outcome: inconclusive

Witness agent: 1

Test code:

```rust
#[test]
fn w_version_5_subadditivity_on_meter_shapes() {
    let shapes = [
        Shape::Dense.packed1(200).version(),
        Shape::Harmonic.packed1(200).version(),
        Shape::AltSpine.packed1(200).version(),
        Shape::CliffComb.packed2(20, 32).version(),
        Shape::Hugeleaf.packed1(300).version(),
        Shape::Bigroot.packed2(100, 50).version(),
        spine_of_pairs(50),
        Version::new(),
    ];
    let mut checked = 0;
    for a in &shapes {
        for b in &shapes {
            let (na, nb) = (a.encode().len(), b.encode().len());
            let j = a.join(b).encode().len();
            let m = a.meet(b).encode().len();
            assert!(j <= na + nb, "join not subadditive: {j} > {na} + {nb}");
            assert!(m <= na + nb, "meet not subadditive: {m} > {na} + {nb}");
            checked += 2;
        }
    }
    eprintln!("MEASURED version-5: {checked} join/meet pairs subadditive");
}
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --no-capture --no-fail-fast -E 'test(w_rank_33) | test(w_clock_22) | test(w_board_10a) | test(w_skyline_9) | test(w_surface_28) | test(w_version_5) | test(w_watermark_21) | test(zz_witness_40)'
```

Output:

```text
MEASURED version-5: 128 join/meet pairs subadditive
        PASS [   0.020s] ( 7/10) before::zz_witness_1 w_version_5_subadditivity_on_meter_shapes
```

Explanation:

This finding is a documentation gap (the subadditivity lemma lives only in test prose while join_all/meet_all's `O(|self| + |iter|)` auxiliary-space claim at version.rs:475 rests on it); no run can demonstrate a missing sentence. As a sanity check, join and meet were subadditive in encoded bytes on all 128 ordered pairs of eight adversarial shapes (no counterexample). The gap itself I assessed by reading: version.rs:438-445 and :471-475 state time only via the fuelscape island plus the auxiliary-space line, with no subadditivity statement.

## rank-33

Outcome: demonstrated

Witness agent: 1

Test code:

```rust
fn ranked_cmp_costs(w: usize, d: usize) -> [u64; 5] {
    let a = Shape::PlateauPuncture.packed2(w, d).version();
    let b = Version::new();
    let bits = ((a.encode().len() + b.encode().len()) * 8) as u64;
    meter::reset_limb_ops();
    black_box(Ranked::from(&a).cmp(&Ranked::from(&b)));
    let cmp_limbs = meter::limb_ops();
    meter::reset_limb_ops();
    black_box(a.rank());
    let rank_limbs = meter::limb_ops();
    meter::reset_limb_ops();
    black_box(a.partial_cmp(&b));
    let pcmp_limbs = meter::limb_ops();
    let one = Version::try_from(1u64).expect("leaf");
    meter::reset_limb_ops();
    black_box(Ranked::from(&a).cmp(&Ranked::from(&one)));
    let cmp_one_limbs = meter::limb_ops();
    [bits, cmp_limbs, rank_limbs, pcmp_limbs, cmp_one_limbs]
}

#[test]
fn w_rank_33_ranked_cmp_runs_the_settle_product() {
    let rows = [(32usize, 24usize), (64, 48), (128, 96)];
    let mut per_bit = Vec::new();
    for (w, d) in rows {
        let [bits, cmp, rank, pcmp, cmp_one] = ranked_cmp_costs(w, d);
        eprintln!(
            "MEASURED rank-33 PP({w},{d}): stored_bits={bits} ranked_cmp_vs_empty_limbs={cmp} rank_limbs={rank} partial_cmp_limbs={pcmp} ranked_cmp_vs_leaf1_limbs={cmp_one} cmp/bit={:.3} rank/bit={:.3}",
            cmp as f64 / bits as f64,
            rank as f64 / bits as f64
        );
        per_bit.push((bits, cmp, rank));
    }
    let (b1, c1, r1) = per_bit[1];
    let (b2, c2, r2) = per_bit[2];
    let exponent = ((c2 as f64) / (c1 as f64)).ln() / ((b2 as f64) / (b1 as f64)).ln();
    let rank_exponent = ((r2 as f64) / (r1 as f64)).ln() / ((b2 as f64) / (b1 as f64)).ln();
    eprintln!("MEASURED rank-33: fitted limb exponent over stored bits: ranked_cmp={exponent:.3} rank={rank_exponent:.3}");
    // The limb meter prices one multiplication by its operand limbs, so the
    // settle's product is linear in that denomination by construction; the
    // wall-time rows below are the (noisy) supplement: min of three runs.
    let mut prev: Option<(u64, f64)> = None;
    for (w, d) in [(128usize, 96usize), (256, 192), (512, 384), (1024, 768), (2048, 1536)] {
        let a = Shape::PlateauPuncture.packed2(w, d).version();
        let b = Version::new();
        let bits = (a.encode().len() * 8) as u64;
        let mut best_cmp = f64::INFINITY;
        let mut best_rank = f64::INFINITY;
        let mut best_pcmp = f64::INFINITY;
        for _ in 0..3 {
            let t = std::time::Instant::now();
            black_box(Ranked::from(&a).cmp(&Ranked::from(&b)));
            best_cmp = best_cmp.min(t.elapsed().as_secs_f64());
            let t = std::time::Instant::now();
            black_box(a.rank());
            best_rank = best_rank.min(t.elapsed().as_secs_f64());
            let t = std::time::Instant::now();
            black_box(a.partial_cmp(&b));
            best_pcmp = best_pcmp.min(t.elapsed().as_secs_f64());
        }
        let growth = prev.map(|(pb, pc)| {
            let e = (best_cmp / pc).ln() / ((bits as f64) / (pb as f64)).ln();
            format!(" ranked_cmp time exponent vs previous row={e:.2}")
        });
        eprintln!(
            "MEASURED rank-33 wall PP({w},{d}): stored_bits={bits} ranked_cmp_min={:.3}ms rank_min={:.3}ms partial_cmp_min={:.3}ms{}",
            best_cmp * 1e3,
            best_rank * 1e3,
            best_pcmp * 1e3,
            growth.unwrap_or_default()
        );
        prev = Some((bits, best_cmp));
    }
}
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --no-capture --no-fail-fast -E 'test(skyline_validate_dense_envelope) | test(skyline_validate_alt_spine_envelope) | test(skyline_decode_dense_envelope) | test(skyline_validate_hugeleaf_envelope) | test(eq_exit_cost_is_tail_independent_on_first_interval_refuted_pair) | test(w_rank_33)'
```

Output:

```text
MEASURED rank-33 PP(32,24): stored_bits=8864 ranked_cmp_vs_empty_limbs=281 rank_limbs=364 partial_cmp_limbs=69 ranked_cmp_vs_leaf1_limbs=281 cmp/bit=0.032 rank/bit=0.041
MEASURED rank-33 PP(64,48): stored_bits=17712 ranked_cmp_vs_empty_limbs=561 rank_limbs=726 partial_cmp_limbs=133 ranked_cmp_vs_leaf1_limbs=561 cmp/bit=0.032 rank/bit=0.041
MEASURED rank-33 PP(128,96): stored_bits=35408 ranked_cmp_vs_empty_limbs=1113 rank_limbs=1440 partial_cmp_limbs=261 ranked_cmp_vs_leaf1_limbs=1113 cmp/bit=0.031 rank/bit=0.041
MEASURED rank-33: fitted limb exponent over stored bits: ranked_cmp=0.989 rank=0.989
MEASURED rank-33 wall PP(128,96): stored_bits=35400 ranked_cmp_min=0.345ms rank_min=0.312ms partial_cmp_min=0.165ms
MEASURED rank-33 wall PP(256,192): stored_bits=70792 ranked_cmp_min=0.761ms rank_min=0.685ms partial_cmp_min=0.331ms ranked_cmp time exponent vs previous row=1.14
MEASURED rank-33 wall PP(512,384): stored_bits=141576 ranked_cmp_min=1.742ms rank_min=1.598ms partial_cmp_min=0.663ms ranked_cmp time exponent vs previous row=1.20
MEASURED rank-33 wall PP(1024,768): stored_bits=283144 ranked_cmp_min=4.234ms rank_min=3.919ms partial_cmp_min=1.334ms ranked_cmp time exponent vs previous row=1.28
MEASURED rank-33 wall PP(2048,1536): stored_bits=566280 ranked_cmp_min=10.085ms rank_min=9.412ms partial_cmp_min=2.648ms ranked_cmp time exponent vs previous row=1.25
        PASS [   2.417s] (6/6) before::zz_witness_1 w_rank_33_ranked_cmp_runs_the_settle_product
```

Explanation:

Demonstrated for the claim's core (Ranked::cmp pays the rank fold's settle while its island says linear), with a caveat on the order. Verified by run: at every scale Ranked::cmp costs 1.08x rank in wall time (0.345 vs 0.312 ms ... 10.085 vs 9.412 ms) and 0.77x rank in limb ops, i.e. the co-sweep does the rank fold's work; its wall exponent over stored bits is 1.14, 1.20, 1.28, 1.25 across four doublings while the same-run `partial_cmp` control (no settle) grows at exactly 1.00 (0.165, 0.331, 0.663, 1.334, 2.648 ms). The construction's limb-counter leg did NOT show superlinearity (exponent 0.989): meter.rs:3565-3566 (read) prices each Base operation by its operands' limb counts, so one big multiplication counts linearly by denomination, and the limb meter is structurally blind to the M(n) term this finding is about; that instrument limitation is itself worth recording. Structural claim confirmed by reading query.rs:286-289 (`rank_cmp` = `pair_fold(a, b, |_| 1).0`), query.rs:399 (`integral.finish`), integral.rs:1142-1143 (`finish` calls `self.settle()`). The wall-time rows are min-of-3 in a dev build on a possibly loaded machine (noisy, supplementary); the asymptotic exponent of the M-bound was not pinned at reachable scales. Run A's first version of this test asserted limb exponent > 1.2 and failed; iteration 2 replaced that with the wall rows (two iterations used).

## skyline-coding-9

Outcome: demonstrated

Witness agent: 1

Test code:

```rust
fn scanned(f: impl FnOnce()) -> u64 {
    meter::reset_scan_bits();
    f();
    meter::scan_bits()
}

fn spine_of_pairs(d: usize) -> Version {
    let mut t = String::from("0");
    for _ in 0..d {
        t = format!("(0, {t}, (0, 0, 1))");
    }
    t.parse().expect("canonical text")
}

fn join_scan(d: usize) -> (u64, u64, u64) {
    let a = Shape::Hugeleaf.packed1(10 * d).version();
    let b = spine_of_pairs(d);
    let bits = ((a.encode().len() + b.encode().len()) * 8) as u64;
    let ab = scanned(|| {
        let j = black_box(a.join(&b));
        assert_eq!(j, a, "the flat wide leaf dominates every leaf of the spine");
    });
    let ba = scanned(|| {
        let j = black_box(b.join(&a));
        assert_eq!(j, a);
    });
    (bits, ab, ba)
}

#[test]
fn w_skyline_9_join_reanchor_cascade_is_superlinear() {
    let mut ratios = Vec::new();
    for d in [32usize, 64, 128, 256] {
        let (bits, ab, ba) = join_scan(d);
        eprintln!(
            "MEASURED skyline-9 d={d}: input_bits={bits} scan(a|b)={ab} scan(b|a)={ba} per_bit(a|b)={:.2} per_bit(b|a)={:.2}",
            ab as f64 / bits as f64,
            ba as f64 / bits as f64
        );
        ratios.push(ab as f64 / bits as f64);
    }
    assert!(
        ratios[3] > 1.5 * ratios[2] && ratios[2] > 1.5 * ratios[1],
        "skyline-9 refuted: scan per input bit stays flat across the doublings: {ratios:?}"
    );
}
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --no-capture --no-fail-fast -E 'test(w_rank_33) | test(w_clock_22) | test(w_board_10a) | test(w_skyline_9) | test(w_surface_28) | test(w_version_5) | test(w_watermark_21) | test(zz_witness_40)'
```

Output:

```text
MEASURED skyline-9 d=32: input_bits=976 scan(a|b)=42724 scan(b|a)=42724 per_bit(a|b)=43.77 per_bit(b|a)=43.77
MEASURED skyline-9 d=64: input_bits=1936 scan(a|b)=167364 scan(b|a)=167364 per_bit(a|b)=86.45 per_bit(b|a)=86.45
MEASURED skyline-9 d=128: input_bits=3856 scan(a|b)=662404 scan(b|a)=662404 per_bit(a|b)=171.79 per_bit(b|a)=171.79
MEASURED skyline-9 d=256: input_bits=7696 scan(a|b)=2635524 scan(b|a)=2635524 per_bit(a|b)=342.45 per_bit(b|a)=342.45
        PASS [   0.018s] ( 5/10) before::zz_witness_1 w_skyline_9_join_reanchor_cascade_is_superlinear
```

Explanation:

Verified by run under scan-meter via the public `Version::join` (both orientations). Joining the flat wide leaf 2^(10d)-1 against the depth-d left spine of two-leaf pairs (built through the text door, `(0, t, (0, 0, 1))` nested d times) scans 43.8, 86.5, 171.8, 342.5 bits per input bit as d doubles: the per-bit cost doubles every doubling, i.e. total scan is Theta(d * W) on Theta(d + W) input (scan/bits^2 is flat: 42724/976^2 ~ 0.045, 2635524/7696^2 ~ 0.044). The result equals the flat leaf, so the whole cost is the re-anchor cascade, exactly as the finding traces (build.rs:154-155 flush then :333-335 extract per level; read). The version_join island contract `O(|self| + |other|)` is at before-fuelscape/src/ops.rs:571 (read).

## skyline-sweep-place-masked-19

Outcome: demonstrated

Witness agent: 1

Test code:

```rust
// Mutation (crates/before/src/version/skyline/place.rs:535), applied for Run C only:
//   -        [Self::PROBE, Self::START, Self::END].into_iter()
//   +        [Self::START, Self::END, Self::PROBE].into_iter()

fn touches(f: impl FnOnce()) -> u64 {
    meter::reset_touch_ops();
    f();
    meter::touch_ops()
}

fn clock_chain_fixture() -> (Version, Version, Version) {
    let mut main = Clock::seed();
    let mut others: Vec<Clock> = (0..6).map(|_| main.fork()).collect();
    let mut rounds = |main: &mut Clock, n: usize| {
        let k = others.len();
        for i in 0..n {
            main.tick();
            let msg = others[i % k].send().clone();
            main.recv(&msg);
        }
    };
    rounds(&mut main, 24);
    let s = main.version().clone();
    main.ticks("1208925819614629174706176".parse::<before::Ticks>().expect("2^80 parses"));
    rounds(&mut main, 24);
    let v = main.version().clone();
    rounds(&mut main, 24);
    let e = main.version().clone();
    assert!(s < v && v < e);
    (s, v, e)
}

fn cliff_fixture() -> (Version, Version, Version) {
    let v = Shape::CliffComb.packed2(20, 64).version();
    let w = Shape::CliffComb.packed2(18, 64).version();
    let s = v.meet(&w);
    let mut e = v.clone();
    e.tick(&Party::seed());
    assert!(s < v && v < e, "the cliff chain is strict");
    (s, v, e)
}

#[test]
fn w_masked_19_placement_touch_readings() {
    for (name, (s, v, e)) in [("clock_chain", clock_chain_fixture()), ("cliff_comb", cliff_fixture())] {
        let span = Span::new(&s, &e).expect("comparable bounds");
        let place = touches(|| {
            black_box(span.place(&v));
        });
        let contains = touches(|| {
            black_box(span.contains(&v));
        });
        let query = touches(|| {
            black_box((causally::since(&s) & causally::before(&e)).contains(&v));
        });
        let cmp_s = touches(|| {
            black_box(v.partial_cmp(&s));
        });
        let cmp_e = touches(|| {
            black_box(v.partial_cmp(&e));
        });
        let scan_place = scanned(|| {
            black_box(span.place(&v));
        });
        eprintln!(
            "MEASURED masked-19 {name}: place_touches={place} contains_touches={contains} query_touches={query} pair_sweep_touches v~s={cmp_s} v~e={cmp_e} sum={} place_scan_bits={scan_place}",
            cmp_s + cmp_e
        );
    }
}
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --no-capture --no-fail-fast -E 'test(/placement::/) | test(w_masked_19) | test(w_watermark_21)'
```

Output:

```text
Baseline (Run A, unmutated):
MEASURED masked-19 clock_chain: place_touches=43 contains_touches=43 query_touches=31 pair_sweep_touches v~s=18 v~e=25 sum=43 place_scan_bits=726
MEASURED masked-19 cliff_comb: place_touches=646 contains_touches=646 query_touches=262 pair_sweep_touches v~s=387 v~e=259 sum=646 place_scan_bits=1478
Run C (priority swapped to [START, END, PROBE]):
        PASS [   0.006s] ( 1/11) before::meter placement::contains_bails_at_either_refuted_side
        PASS [   0.006s] ( 2/11) before::meter placement::dominance_bails_at_the_refuted_start
        PASS [   0.006s] ( 3/11) before::meter placement::precedence_bails_at_the_refuted_end
        PASS [   0.006s] ( 4/11) before::meter placement::query_early_exits_survive_the_fusion
        PASS [   0.006s] ( 5/11) before::meter placement::query_fused_walk_scans_each_stream_once
        PASS [   0.006s] ( 6/11) before::meter placement::query_single_bound_matches_the_pair_sweep
MEASURED query_single_bound_limbs: raw=20 ceiling=20
        PASS [   0.006s] ( 7/11) before::meter placement::query_single_bound_matches_the_pair_sweep_limbs
        PASS [   0.006s] ( 8/11) before::meter placement::span_concurrent_exits_survive_the_fusion
MEASURED span_one_pass: fused=756 dominance=568 composed=1113 probe_scan=357
        PASS [   0.006s] ( 9/11) before::meter placement::span_place_scans_each_stream_once
MEASURED masked-19 clock_chain: place_touches=43 contains_touches=43 query_touches=31 pair_sweep_touches v~s=18 v~e=25 sum=43 place_scan_bits=726
MEASURED masked-19 cliff_comb: place_touches=646 contains_touches=646 query_touches=262 pair_sweep_touches v~s=387 v~e=259 sum=646 place_scan_bits=1478
        PASS [   0.006s] (10/11) before::zz_witness_1 w_masked_19_placement_touch_readings
```

Explanation:

Verified by run: with PROBE moved last in the placement priority, all nine `placement::` rows in tests/meter.rs pass (they read verdicts, scan bits, and limb ops: none reads the touch meter), and both named identities (`span_place_scans_each_stream_once`, `query_single_bound_matches_the_pair_sweep_limbs`) stay green, as the finding predicts. Caveat, stated plainly: the swap changed NO touch reading on either of my fixtures (the 2^80-plateau clock chain and a 2^20/2^18 cliff-comb chain read identically before and after; the touch identity place == cmp_s + cmp_e holds in both builds). So the rows' blindness is demonstrated, but I could not exhibit a fixture where the reorder changes carry work; with QUICK_MAX = 2^96 (accumulator.rs:61, read) a word-scale tie cannot spill the quick register unless a wide value already forced the digit engine, and engineering a digit-carry-sensitive tie was beyond the remaining budget. The finding's premise that a reorder changes the committed write sequence remains unverified by measurement.

## skyline-watermark-21

Outcome: demonstrated

Witness agent: 1

Test code:

```rust
// Mutation (crates/before/src/version/skyline/watermark.rs:541-545), applied for Run C only:
//         if let Some(latent) = self.latent.take() {
//   -            // The annihilation: residue = (A − v) − Λ = m − v > 0.
//   -            residue.sub_accum(&latent);
//   +            // witness:1 mutant: the annihilation `residue.sub_accum(&latent)` skipped.
//             self.retire(latent);
//         }

const WATERMARK_TREE: &str = "(0, 0, (1, (17179869183, 0, (1, 1, 0)), 0))";

#[test]
fn w_watermark_21_min_ticks_reads_the_annihilation_residue() {
    let v: Version = WATERMARK_TREE.parse().expect("normal form");
    assert_eq!(v.to_string(), WATERMARK_TREE);
    let t = v.min_ticks().to_string();
    eprintln!("MEASURED watermark-21: min_ticks={t}");
    assert_eq!(t, "17179869186", "expected 2^34 + 2");
}
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --no-capture --no-fail-fast -E 'test(/placement::/) | test(w_masked_19) | test(w_watermark_21)'
```

Output:

```text
Baseline (Run A2, unmutated):
MEASURED watermark-21: min_ticks=17179869186
        PASS [   0.011s] ( 8/10) before::zz_witness_1 w_watermark_21_min_ticks_reads_the_annihilation_residue
Run C (annihilation skipped):
MEASURED watermark-21: min_ticks=17179869185
thread 'w_watermark_21_min_ticks_reads_the_annihilation_residue' (241442125) panicked at crates/before/tests/zz_witness_1.rs:408:5:
  left: "17179869185"
 right: "17179869186"
        FAIL [   0.006s] (11/11) before::zz_witness_1 w_watermark_21_min_ticks_reads_the_annihilation_residue
```

Explanation:

Verified by run. The finding's tree, written in the crate's normal-form text (leaf heights in stream order 0, 2^34, 2^34+2, 2^34+1, 1), round-trips through the public text door and `min_ticks` reads 2^34 + 2 = 17179869186 in production; with `drop_below`'s latent annihilation removed (watermark.rs:543) the public answer changes to 17179869185. A public-API input therefore executes the arm that watermark/tests.rs:1-10 says no packed-stream walk reaches, exactly as the finding's trace predicts (with the annihilation skipped the residue meets Word(2^34) and the answer drops by one).

## clock-22

Outcome: demonstrated

Witness agent: 1

Test code:

```rust
fn drive_deep_clock(name: &str, party: Party) {
    let original = party.dangerously_alias();
    let party_bits = party.encoded_bits();
    let mut c = Clock::from_parts(party, Version::new());
    c.ticks(1u64 << 40);
    c.tick();
    let kids: Vec<Clock> = c.forks(3).collect();
    assert_eq!(kids.len(), 3);
    c.join_all(kids).expect("balanced shares rejoin");
    assert_eq!(*c.party(), original, "forks + join_all restore the deep party");
    // Every whole-party op runs while `c` still holds the deep party.
    let plateaus = c.shape().count();
    let bytes = c.encode();
    let back = Clock::decode(&bytes[..]).expect("deep clock decodes");
    assert_eq!(back.encode(), bytes);
    let bits = c.encoded_bits();
    let alias = c.dangerously_alias();
    assert!(alias.party().covers(c.party()));
    assert!(!alias.party().is_disjoint(c.party()));
    let own = c.own_version().to_version();
    assert!(own <= *c.version());
    let debug_len = format!("{:?}", c.party()).len();
    let mut p_ticks = c.version().clone();
    c.party().ticks(&mut p_ticks, 7u64);
    assert!(p_ticks > *c.version());
    // Then the multi-clock ops, which re-fork the party among the shares.
    let mut others: Vec<Clock> = c.forks(2).collect();
    c.sync_all(others.iter_mut()).expect("disjoint clocks sync");
    let msgs: Vec<Version> = others.iter().map(|o| o.version().clone()).collect();
    c.recv_all(&msgs);
    c.absorb_all(&msgs);
    let residual_debug_len = format!("{:?}", c.party()).len();
    c.join_all(others).expect("rejoin the synced shares");
    assert_eq!(*c.party(), original, "sync_all shares rejoin to the deep party");
    eprintln!(
        "MEASURED clock-22 {name}: party_bits={party_bits} plateaus={plateaus} clock_encoded_bits={bits} party_debug_len={debug_len} residual_after_sync_all_debug_len={residual_debug_len}"
    );
}

#[test]
fn w_clock_22_nested_full_comb_depth_100k() {
    drive_deep_clock("nested_full", party_of(&Shape::NestedFullId.packed1(100_000)));
}

#[test]
fn w_clock_22_nested_left_full_comb_depth_100k() {
    drive_deep_clock("nested_left_full", party_of(&Shape::NestedLeftFullId.packed1(100_000)));
}
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --no-capture --no-fail-fast -E 'test(w_rank_33) | test(w_clock_22) | test(w_board_10a) | test(w_skyline_9) | test(w_surface_28) | test(w_version_5) | test(w_watermark_21) | test(zz_witness_40)'
```

Output:

```text
MEASURED clock-22 nested_full: party_bits=400004 plateaus=100002 clock_encoded_bits=400095 party_debug_len=500006 residual_after_sync_all_debug_len=500006
        PASS [   0.030s] ( 2/10) before::zz_witness_1 w_clock_22_nested_full_comb_depth_100k
MEASURED clock-22 nested_left_full: party_bits=400004 plateaus=100002 clock_encoded_bits=400175 party_debug_len=500006 residual_after_sync_all_debug_len=11
        PASS [   0.026s] ( 3/10) before::zz_witness_1 w_clock_22_nested_left_full_comb_depth_100k
```

Explanation:

The gap is real and the constructed witness finds no bug behind it. Verified by reading: generators.rs:274-283 builds only `10` tags, so deep_tree_stack_safety has no both-present frame at depth. Verified by run: two 100k-level both-present id combs (N(100k) descending left and its mirror M(100k) descending right, 400004 bits each, built through the meter registry and the public `Party::decode`) survive ticks(2^40), tick, forks(3), join_all, shape().count(), encode/decode, encoded_bits, dangerously_alias, covers/is_disjoint, own_version, Party::ticks, Debug, forks(2)+sync_all, recv_all, absorb_all, and a final join_all, with the party restored byte-for-byte, and no stack overflow in the 2 MiB test thread. Note on the printed residual: after sync_all re-forks M(d) the receiver keeps the shallow share `((1, 0), 0)` (11 Debug chars), which is legitimate balanced-split behavior, not a collapse; that observation cost one revision of the driver (two iterations used). The finding's exact both-present chain built by hand-packing bits was not used because Party::from_bits is pub(crate); the registry's N/M combs are its equivalent.

## codec-base-text-tree-13

Outcome: demonstrated

Witness agent: 1

Test code:

```rust
#[test]
fn w_codec_13_party_literal_scan_is_quadratic_in_depth() {
    let s32 = scanned(|| {
        let p = Party::try_from(NESTED32).expect("normal form");
        black_box(p);
    });
    let s64 = scanned(|| {
        let p = Party::try_from(NESTED64).expect("normal form");
        black_box(p);
    });
    let n32 = Party::try_from(NESTED32).unwrap().encode().len();
    let n64 = Party::try_from(NESTED64).unwrap().encode().len();
    eprintln!(
        "MEASURED codec-13: depth32 scan_bits={s32} packed_bytes={n32}; depth64 scan_bits={s64} packed_bytes={n64}; scan ratio={:.2} byte ratio={:.2}",
        s64 as f64 / s32 as f64,
        n64 as f64 / n32 as f64
    );
    assert!(
        s64 as f64 > 3.0 * s32 as f64,
        "codec-13 refuted: scan grew near-linearly with depth"
    );
}

// NESTED32 / NESTED64 are `const`s of the left-spine tuple type
// ((((u8, u8), u8), u8) ... ) at depth 32 and 64, holding
// ((((1u8, 0u8), 0u8), 0u8) ... ) (generated; verbatim in the archived file).
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --no-capture -E 'test(/^w_/) | ...' (Run A; full filter under party-1)
```

Output:

```text
MEASURED codec-13: depth32 scan_bits=1120 packed_bytes=9; depth64 scan_bits=4288 packed_bytes=17; scan ratio=3.83 byte ratio=1.89
        PASS [   0.004s] (15/25) before::zz_witness_1 w_codec_13_party_literal_scan_is_quadratic_in_depth
```

Explanation:

Verified by run under scan-meter through the public `Party::try_from` tuple door. Doubling the left-spine literal's depth from 32 to 64 grows the packed party 1.89x (9 -> 17 bytes) but the scanned/written bits 3.83x (1120 -> 4288): quadratic in the claimed byte denomination, matching the trace that `id_node` copies both children and re-runs `validate_id` over the assembled subtree at every level (literal.rs:59-64, read; the per-bit records are cursor.rs:114 for parse_id's SliceCursor and build.rs:83/113 for the pushes). The rustdoc claim `O(n), n the built party's size in bytes` is at party.rs:899 (read).

## crate-root-34

Outcome: demonstrated

Witness agent: 1

Test code:

```rust
mod bytes_de {
    use serde::de::{self, Visitor};

    /// A Deserializer holding one typed byte string, which answers every
    /// request through `deserialize_any` with `visit_bytes` (what
    /// `serde_test` does for a `Token::Bytes`).
    pub struct BytesDe<'a>(pub &'a [u8]);

    impl<'de, 'a> de::Deserializer<'de> for BytesDe<'a> {
        type Error = de::value::Error;
        fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
            visitor.visit_bytes(self.0)
        }
        serde::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
}

#[test]
fn w_crate_root_34_serde_deserialize_rejects_typed_bytes() {
    use serde::Deserialize;
    let p = Party::seed();
    let bytes = p.encode();
    let r = Party::deserialize(bytes_de::BytesDe(&bytes));
    let v = Version::try_from(3u64).unwrap();
    let rv = Version::deserialize(bytes_de::BytesDe(&v.encode()));
    let rc = Clock::deserialize(bytes_de::BytesDe(&Clock::seed().encode()));
    match (&r, &rv, &rc) {
        (Err(e), Err(ev), Err(ec)) => {
            eprintln!("MEASURED crate-root-34: Party: {e}; Version: {ev}; Clock: {ec}");
            assert!(e.to_string().contains("expected a sequence"), "unexpected error text: {e}");
        }
        _ => panic!("crate-root-34 refuted: a typed byte string deserialized: {r:?} {rv:?} {rc:?}"),
    }
}
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --no-capture -E 'test(/^w_/) | ...' (Run A; full filter under party-1)
```

Output:

```text
MEASURED crate-root-34: Party: invalid type: byte array, expected a sequence; Version: invalid type: byte array, expected a sequence; Clock: invalid type: byte array, expected a sequence
        PASS [   0.004s] (16/25) before::zz_witness_1 w_crate_root_34_serde_deserialize_rejects_typed_bytes
```

Explanation:

Verified by run (serde feature). Instead of adding serde_test as a dependency, a 20-line Deserializer that answers every request with `visit_bytes` of one typed byte string (the data-model behavior serde_test exhibits for Token::Bytes) makes `Party`, `Version`, and `Clock` deserialization fail with `invalid type: byte array, expected a sequence`, while the Serialize side emits exactly that byte string (`serialize_bytes`, serde_impls.rs:22/35/48, read). The Deserialize impls go through `<Vec<u8>>::deserialize` (serde_impls.rs:28/41/54, read), whose visitor implements only visit_seq. The serialize leg was verified by reading, not by a recording Serializer.

## suanpan-40

Outcome: demonstrated

Witness agent: 1

Test code:

```rust
// Mutations (crates/suanpan/src/accumulator.rs), applied for Run E only:
//   lines 1112 and 1121:  high >>= DIGIT_BITS;  ->  high <<= DIGIT_BITS; // witness:1 mutant of `>>=`
//   line 607:  if shift == 0 || self.is_literally_zero() {  ->  if shift == 0 && self.is_literally_zero() { // witness:1 mutant of `||`

// Appended to crates/suanpan/src/accumulator/tests.rs (uses the file's existing
// `park_extreme_negative_digit` helper at lines 110-116):

/// witness:1 scratch (suanpan-40): the touch count of `sign_magnitude` on
/// an accumulator holding one parked extreme negative digit; the `<<=`
/// mutant of `read_digits` drains the high part in four steps instead of one.
#[test]
fn zz_witness_40_sign_magnitude_touches_after_extreme_park() {
    let mut a = Accumulator::new();
    park_extreme_negative_digit(&mut a, 0);
    crate::touch_meter::reset();
    let (sign, mag) = a.sign_magnitude();
    let t = crate::touch_meter::touches();
    eprintln!("MEASURED suanpan-40 sign_magnitude: touches={t} sign={sign:?} magnitude={mag}");
    assert_eq!(sign, Ordering::Less);
    assert_eq!(mag, UBig::from((1u64 << 33) - 1));
}

/// witness:1 scratch (suanpan-40): `shl(0)` on a digit-engine value costs
/// no touches through the identity guard; the `&&` mutant rebuilds the value.
#[test]
fn zz_witness_40_shl_zero_touches_on_digit_engine_value() {
    let mut a = Accumulator::new();
    a.add_wide(&(UBig::ONE << 2048usize));
    crate::touch_meter::reset();
    a.shl(0);
    let t = crate::touch_meter::touches();
    eprintln!("MEASURED suanpan-40 shl(0): touches={t}");
    let (sign, mag) = a.sign_magnitude();
    assert_eq!(sign, Ordering::Greater);
    assert_eq!(mag, UBig::ONE << 2048usize);
}
```

Command:

```text
cargo nextest run -p suanpan --all-features --no-capture --no-fail-fast
```

Output:

```text
Baseline (Run A2, unmutated):
MEASURED suanpan-40 shl(0): touches=0
MEASURED suanpan-40 sign_magnitude: touches=3 sign=Less magnitude=8589934591
Run E (both mutants applied):
MEASURED suanpan-40 shl(0): touches=66
MEASURED suanpan-40 sign_magnitude: touches=6 sign=Less magnitude=8589934591
     Summary [   4.197s] 62 tests run: 62 passed, 0 skipped
```

Explanation:

Verified by run. Production reads 3 touches for `sign_magnitude` after the extreme park and 0 for `shl(0)` on a 2048-bit digit-engine value; with the two excluded mutants applied the same calls read 6 and 66 while every value assertion still holds (value-equivalent, touch-observable), and suanpan's entire committed suite (62 tests, all of the crate's tests) stays green under both mutants. So neither exclusion premise in .cargo/mutants.toml:85-95 holds: the `<<=` mutant has a nonempty discriminating class in the touch denomination (+3 touches, three extra drain steps), and the `&&` mutant turns a free zero shift into a full take-and-redeposit. The finding's estimate of ~130 touches for the shl mutant was high by 2x (66 measured: 64 digits plus two), the direction and mechanism as stated.

## meter-core-8

Outcome: demonstrated

Witness agent: 1

Test code:

```rust
// Tap added to crates/before/src/version/skyline/query/integral.rs (cfg(test) only), Run B:
//   in the existing `#[cfg(test)] thread_local!` block after FREEZE_HITS:
//     /// witness:1 scratch tap: calls of [`Integrator::promote`].
//     pub(super) static PROMOTE_CALLS: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
//     /// witness:1 scratch tap: ledger armings pushed by [`Integrator::promote`].
//     pub(super) static ARMINGS: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
//   in `fn promote(&mut self)`:
//     #[cfg(test)]
//     PROMOTE_CALLS.with(|hits| hits.set(hits.get() + 1));
//     ... inside `if parked_magnitude != UBig::ZERO {`:
//     #[cfg(test)]
//     ARMINGS.with(|hits| hits.set(hits.get() + 1));

// Added to crates/before/src/version/skyline/query/tests.rs:
#[test]
fn zz_witness_8_wide_arming_promotions_by_width() {
    use super::integral::{ARMINGS, FREEZE_HITS, PROMOTE_CALLS};
    let counts = |v: &Version| {
        let f0 = FREEZE_HITS.with(|h| h.get());
        let p0 = PROMOTE_CALLS.with(|h| h.get());
        let a0 = ARMINGS.with(|h| h.get());
        let _ = v.rank();
        (
            FREEZE_HITS.with(|h| h.get()) - f0,
            PROMOTE_CALLS.with(|h| h.get()) - p0,
            ARMINGS.with(|h| h.get()) - a0,
        )
    };
    for (w, d) in [(10usize, 1usize), (12, 5), (17, 5), (18, 5), (19, 5), (24, 5)] {
        let v = version_of(&Shape::WideArming.packed2(w, d));
        let (f, p, a) = counts(&v);
        eprintln!("MEASURED meter-core-8 WA({w},{d}): freezes={f} promote_calls={p} armings={a}");
    }
    let hw = version_of(&Shape::HoistedWindow.packed3(12, 40, 1024));
    let (f, p, a) = counts(&hw);
    eprintln!("MEASURED meter-core-8 HW(12,40,1024): freezes={f} promote_calls={p} armings={a}");
}
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --no-capture --no-fail-fast -E 'test(roster_is_total_over_the_public_fn_surface) | test(every_cited_binding_test_exists) | test(exclusion_payload_citations_resolve) | test(version_party_laws) | test(dependency_reexports_match_the_committed_roster) | test(zz_witness_8) | test(zz_witness_9)'
```

Output:

```text
MEASURED meter-core-8 WA(10,1): freezes=3 promote_calls=0 armings=0
MEASURED meter-core-8 WA(12,5): freezes=3 promote_calls=0 armings=0
MEASURED meter-core-8 WA(17,5): freezes=3 promote_calls=0 armings=0
MEASURED meter-core-8 WA(18,5): freezes=3 promote_calls=1 armings=1
MEASURED meter-core-8 WA(19,5): freezes=3 promote_calls=1 armings=1
MEASURED meter-core-8 WA(24,5): freezes=3 promote_calls=1 armings=1
MEASURED meter-core-8 HW(12,40,1024): freezes=3 promote_calls=0 armings=0
        PASS [   0.006s] (6/7) before version::skyline::query::tests::zz_witness_8_wide_arming_promotions_by_width
```

Explanation:

Verified by run with a cfg(test) counter tap on `Integrator::promote`. `rank` on wide_arming(w, d) never calls promote for w = 10, 12, 17 and calls it exactly once (one ledger arming) from w = 18 on, matching integral.rs:880's `parked.digit_count() > base_digits(&drift) + FREEZE_ALLOWANCE_DIGITS` with a ten-digit drift and FREEZE_ALLOWANCE_DIGITS = 8 (integral.rs:274, read): promotion needs w + 1 > 18. The guard `w >= 10` (meter.rs:1909-1912, read) therefore admits eight widths at which the documented 'one ledger arming' (meter.rs:1887-1889) cannot fire, and the committed hoisted_window band width 12 (tests/meter.rs:5332) runs with zero promotions (HW(12,40,1024): promote_calls=0). One deviation from the finding's prose: FREEZE_HITS counts 3 freezes per run, not two; the tap sits on the drift-parking arm (integral.rs:874-875) and I did not trace the third.

## board-families-floors-judge-10

Outcome: demonstrated

Witness agent: 1

Test code:

```rust
#[test]
fn w_board_10a_version_decode_touches_digits_on_word_scale_deltas() {
    let v = Shape::Dense.packed1(1_000).version();
    let bytes = v.encode();
    let t = touches(|| {
        let d = Version::decode(&bytes[..]).expect("round trip");
        assert_eq!(d, v);
    });
    eprintln!("MEASURED board-10a: dense(1000) version_decode touches={t} packed_bytes={}", bytes.len());
    assert!(t > 0, "board-10a refuted: decode of word-scale deltas forced no digit touches");
    let comb = Shape::CliffComb.packed2(20, 64).version();
    let comb_bytes = comb.encode();
    let tc = touches(|| {
        let d = Version::decode(&comb_bytes[..]).expect("round trip");
        assert_eq!(d, comb);
    });
    eprintln!("MEASURED board-10a: cliff_comb(20,64) version_decode touches={tc} packed_bytes={}", comb_bytes.len());
    assert!(tc > 0);
}

#[test]
fn w_board_10b_seed_fork_scans_the_seed_tag() {
    let seed_bytes = Party::seed().encode();
    let mut p = Party::seed();
    let bits = scanned(|| {
        let _q = black_box(p.fork());
    });
    eprintln!("MEASURED board-10b: seed_fork scan_bits={bits} seed_packed_bytes={} seed_bytes={seed_bytes:?}", seed_bytes.len());
    assert!(bits >= 2, "board-10b refuted: forking the seed scanned no bits");
}

#[test]
fn w_board_10c_clock_fork_allocates_the_child_party() {
    let v = Shape::Dense.packed1(100).version();
    let mut c = Clock::from_parts(Party::seed(), v);
    let (peak, child) = peak_of(|| c.fork());
    eprintln!(
        "MEASURED board-10c: clock_fork peak_heap={peak}B child_party_bytes={} parent_party_bytes={}",
        child.party().encode().len(),
        c.party().encode().len()
    );
    assert!(peak > 0, "board-10c refuted: Clock::fork allocated nothing");
}
```

Command:

```text
Run A (filter under party-1) for 10b/10c; Run A2 (filter under version-core-5) for 10a
```

Output:

```text
MEASURED board-10a: dense(1000) version_decode touches=3 packed_bytes=376
MEASURED board-10a: cliff_comb(20,64) version_decode touches=193 packed_bytes=91
        PASS [   0.007s] ( 1/10) before::zz_witness_1 w_board_10a_version_decode_touches_digits_on_word_scale_deltas
MEASURED board-10b: seed_fork scan_bits=2 seed_packed_bytes=1 seed_bytes=[32]
        PASS [   0.004s] (11/25) before::zz_witness_1 w_board_10b_seed_fork_scans_the_seed_tag
MEASURED board-10c: clock_fork peak_heap=2B child_party_bytes=1 parent_party_bytes=1
        PASS [   0.004s] (12/25) before::zz_witness_1 w_board_10c_clock_fork_allocates_the_child_party
```

Explanation:

All three clauses verified by run through public API. (a) `Version::decode` of word-scale streams touches accumulator digits: 3 touches on dense(1000) and 193 on cliff_comb(20,64), contradicting NA_TOUCH_LAZY_BATCH's 'force no digit touches' (floors.rs:295-296, read; the decode rows' NA route is touch_wide_stream at floors.rs:309-313 when mandatory limbs are 0). I could not call the pub(super) `stored_nonzero_deltas` from the integration binary, so the >= comparison in the construction is not reproduced; the nonzero touch count is what contradicts the rendered reason. (b) `Party::seed().fork()` records 2 scan bits, and the seed's packed form is one byte `[32]` (0b0010_0000), so NA_SCAN_SEED_PARTY's 'its packed form is empty' (floors.rs:144-145, read) is false while scan_touch() is met exactly. (c) `Clock::fork` on a version-only clock raises the peak heap by 2 bytes (the child's one-byte party plus its Bytes header), while ops.rs:1613 declares clock_fork's heap NA (read).

## board-ops-render-9

Outcome: demonstrated

Witness agent: 1

Test code:

```rust
// Added to crates/before/src/meter/board/tests.rs (lib test; needs the pub(super) family API):
#[test]
fn zz_witness_9_membership_and_covers_verdicts_per_family() {
    use super::family::FamilyData;
    use crate::causally;
    use crate::meter::registry::FamilyId;
    let mut pairs = 0usize;
    let mut contains_false: Vec<&str> = Vec::new();
    let mut parties = 0usize;
    let mut covers_true: Vec<&str> = Vec::new();
    for kind in FamilyId::board() {
        let data = FamilyData::build(kind, 1.0, 0);
        if let Some((v, w, _)) = data.version_pair() {
            pairs += 1;
            let hit = causally::since(&v).contains(&w);
            let order = v.partial_cmp(&w);
            if !hit {
                contains_false.push(data.name);
            }
            eprintln!("MEASURED board-9 {}: since(v).contains(w)={hit} v.partial_cmp(w)={order:?}", data.name);
        }
        if let Some((a, b, _)) = data.party_pair() {
            parties += 1;
            if a.covers(&b) {
                covers_true.push(data.name);
            }
        }
    }
    eprintln!(
        "MEASURED board-9: version pairs={pairs} contains_false_on={contains_false:?}; party pairs={parties} covers_true_on={covers_true:?}"
    );
}
```

Command:

```text
Run B (filter under meter-core-8)
```

Output:

```text
MEASURED board-9 dense: since(v).contains(w)=true v.partial_cmp(w)=Some(Less)
MEASURED board-9 jump-pair: since(v).contains(w)=true v.partial_cmp(w)=None
MEASURED board-9 dense-suffix: since(v).contains(w)=false v.partial_cmp(w)=Some(Greater)
MEASURED board-9 concurrent-pair: since(v).contains(w)=true v.partial_cmp(w)=None
MEASURED board-9: version pairs=29 contains_false_on=["dense-suffix"]; party pairs=14 covers_true_on=[]
        PASS [   0.215s] (1/7) before meter::board::tests::zz_witness_9_membership_and_covers_verdicts_per_family
```

Explanation:

Verified by run over the board's own bundles (FamilyId::board(), scale 1.0, level 0). `since(&v).contains(&w)` returns true on 28 of 29 families with a version pair (26 of them the post-pass's `w = v + seed tick` with v < w; the two pair shapes jump-pair and concurrent-pair are incomparable and still admit) and false only on dense-suffix, the one family whose own pairing has v > w; `a.covers(&b)` is false on all 14 families with a party pair. So the causally_contains row measures the one-witness early exit everywhere but dense-suffix and party_covers measures only the refusing walk, as the finding states. The membership_floors fork on `v.partial_cmp(w) == Some(Greater)` (floors.rs:718-730) and the post-pass at family.rs:797-803 were read, not exercised separately; the refusal-walk plant in the construction was not attempted.

## surface-roster-4

Outcome: demonstrated

Witness agent: 1

Test code:

```rust
// Mutation (crates/before/src/party.rs:180-182), applied for Run B only:
//       pub fn tick(&self, version: &mut Version) {
//   -        version.tick(self)
//   +        let _ = version;
//       }
// Tests run: the three existing roster tests in src/testing/surface_coverage/tests.rs and the
// existing law driver `version_party_laws` (src/testing/algebraic_laws/tests.rs, via for_each_law_group!).
```

Command:

```text
Run B (filter under meter-core-8)
```

Output:

```text
thread 'testing::algebraic_laws::tests::version_party_laws' (241426746) panicked at crates/before/src/testing/algebraic_laws/tests.rs:304:1:
Test failed: law violated: party_tick_matches_version_tick at crates/before/src/testing/algebraic_laws/tests.rs:304.
        FAIL [   0.091s] (2/7) before testing::algebraic_laws::tests::version_party_laws
        PASS [   0.012s] (3/7) before testing::surface_coverage::tests::every_cited_binding_test_exists
        PASS [   0.014s] (4/7) before testing::surface_coverage::tests::exclusion_payload_citations_resolve
        PASS [   0.005s] (5/7) before testing::surface_coverage::tests::roster_is_total_over_the_public_fn_surface
```

Explanation:

Verified by run: with `Party::tick`'s body replaced by `let _ = version;`, all three roster tests stay green while the algebraic-laws driver fails naming `party_tick_matches_version_tick`, so the roster's three Trans citations for the Party::tick row (surface.rs:340-345, read) bind nothing that exercises Party::tick, and the law that does is what catches it. Verified mechanically (grep over crates/before/src and tests): the string `party_tick_matches_version_tick` occurs only at laws.rs:2482, its definition, so deleting the law would leave every roster citation resolving; that deletion was not run (the mechanical result makes its outcome determinate).

## surface-roster-28

Outcome: demonstrated

Witness agent: 1

Test code:

```rust
#[test]
fn w_surface_28_extractor_drops_pub_const_fn_silently() {
    let root = PathBuf::from(SCRATCH).join("surface28");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("const_fn.rs"),
        "impl Thing {\n    pub const fn zero() -> u8 {\n        0\n    }\n    pub async fn later() {}\n    pub unsafe fn raw() {}\n    pub fn one() -> u8 {\n        1\n    }\n}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("nested.rs"),
        "mod inner {\n    impl Thing {\n        pub fn hidden() -> u8 {\n            2\n        }\n    }\n}\n",
    )
    .unwrap();
    let got = surface_scan::extract_public_fns(
        &root,
        &[SourceSpec { path: "const_fn.rs", module_prefix: None, type_overrides: &[] }],
    );
    let got_nested = surface_scan::extract_public_fns(
        &root,
        &[SourceSpec { path: "nested.rs", module_prefix: None, type_overrides: &[] }],
    );
    eprintln!("MEASURED surface-28: const_fn fixture rows={got:?}; nested fixture rows={got_nested:?}");
    assert_eq!(got.into_iter().collect::<Vec<_>>(), vec!["Thing::one".to_string()]);
    assert!(got_nested.is_empty());
}
```

Command:

```text
Run A2 (filter under version-core-5)
```

Output:

```text
MEASURED surface-28: const_fn fixture rows={"Thing::one"}; nested fixture rows={}
        PASS [   0.016s] ( 6/10) before::zz_witness_1 w_surface_28_extractor_drops_pub_const_fn_silently
```

Explanation:

Verified by run against the real `surface_scan::extract_public_fns`. In one inherent impl holding `pub const fn zero`, `pub async fn later`, `pub unsafe fn raw`, and `pub fn one`, the extractor returns exactly {Thing::one}: the three qualified shapes vanish with no panic. A `pub fn` at an 8-space indent inside a private `mod inner { impl Thing { ... } }` yields no row and no panic either. This contradicts lib.rs:23-25's 'A pub fn at an unexpected position panics rather than silently vanishing' (read); the extractor's only positive shapes are `    pub fn ` and `pub fn ` (lib.rs:111, :123, read). The suanpan roster consequence (adding a `pub const fn` to Accumulator leaves claims_are_total green) was not run; it follows from the extractor result.

## testing-diff-gen-23

Outcome: demonstrated

Witness agent: 1

Test code:

```rust
// No cargo test; mechanical roster comparison only.
// grep -n 'log k' crates/before-fuelscape/src/ops.rs ; grep -n 'name: "[a-z_]*_all"' crates/before-fuelscape/src/ops.rs
// grep -n '^fn .*log_factor_is_alive' crates/before/src/testing/asymptotics.rs
```

Command:

```text
grep -n "log k" crates/before-fuelscape/src/ops.rs; grep -n 'name: "[a-z_]*_all"' crates/before-fuelscape/src/ops.rs; grep -n "^fn .*log_factor_is_alive" crates/before/src/testing/asymptotics.rs
```

Output:

```text
625:        name: "version_join_all",
638:        name: "version_meet_all",
668:        name: "version_span_all",
796:        name: "party_join_all",
1034:        name: "clock_join_all",
1078:        name: "clock_sync_all",
1106:        name: "clock_recv_all",
1563:        name: "span_union_all",
1579:        name: "span_intersect_all",
1597:        name: "span_join_all",
1613:        name: "span_meet_all",
(each followed by a contract line containing `log k` at 630, 643, 675, 806, 1043, 1088, 1115, 1568, 1584, 1602, 1618)
259:fn version_join_all_log_factor_is_alive() {
285:fn version_meet_all_log_factor_is_alive() {
311:fn version_span_all_log_factor_is_alive() {
341:fn party_join_all_log_factor_is_alive() {
368:fn clock_join_all_log_factor_is_alive() {
```

Explanation:

Mechanically verified, not run as a mutation: eleven fold-door islands carry a `log k` contract in before-fuelscape/src/ops.rs (the eleven names the finding lists), and asymptotics.rs has exactly five `*_log_factor_is_alive` pins (version_join_all, version_meet_all, version_span_all, party_join_all, clock_join_all); nothing mechanical ties the two lists, so clock_sync_all, clock_recv_all, and the four Span *_all doors have contracts with no pin. The construction's left-fold replacement behind Span::join_all was not run (it would require the whole suite to show nothing reads red); the roster gap it rests on is confirmed by the counts.

## envelopes-a-6

Outcome: demonstrated

Witness agent: 1

Test code:

```rust
// Mutation (crates/before/src/codec/dsi.rs:189-194), applied for Run D only:
//       fn read_unary(&mut self) -> Result<u64, Truncated> {
//           let k = self.unary_raw()?;
//   -        super::scan::record_bits_u64(k + 1);
//   +        // witness:1 mutant: `super::scan::record_bits_u64(k + 1)` removed.
//           self.position += k + 1;
//           Ok(k)
//       }
// Tests run: the existing tests/meter.rs rows skyline_validate_dense_envelope,
// skyline_validate_alt_spine_envelope, skyline_decode_dense_envelope,
// skyline_validate_hugeleaf_envelope, and eq_early_exit::eq_exit_cost_is_tail_independent_on_first_interval_refuted_pair.
```

Command:

```text
Run D (filter under rank-33)
```

Output:

```text
Baseline (Run A):
MEASURED skyline_validate_dense: input_bytes=46876 peak_heap=49152 segments=0 limb_ops=0 scan_bits=375006
MEASURED skyline_validate_alt_spine: input_bytes=46876 peak_heap=49152 segments=0 limb_ops=0 scan_bits=375006
MEASURED skyline_decode_dense: input_bytes=46876 peak_heap=49152 segments=0 limb_ops=0 scan_bits=375006
MEASURED eq_exit: small touches=49/1154B scan=2054 large touches=49/1794B scan=2054
Run D (unary-run scan tap removed):
MEASURED eq_exit: small touches=49/1154B scan=2050 large touches=49/1794B scan=2050
        PASS [   0.014s] (1/6) before::meter eq_early_exit::eq_exit_cost_is_tail_independent_on_first_interval_refuted_pair
MEASURED skyline_decode_dense: input_bytes=46876 peak_heap=49152 segments=0 limb_ops=0 scan_bits=125005
        PASS [   0.030s] (2/6) before::meter skyline_decode_dense_envelope
MEASURED skyline_validate_alt_spine: input_bytes=46876 peak_heap=49152 segments=0 limb_ops=0 scan_bits=125005
        PASS [   0.020s] (3/6) before::meter skyline_validate_alt_spine_envelope
MEASURED skyline_validate_dense: input_bytes=46876 peak_heap=49152 segments=0 limb_ops=0 scan_bits=125005
        PASS [   0.020s] (4/6) before::meter skyline_validate_dense_envelope
MEASURED skyline_validate_hugeleaf: input_bytes=31251 peak_heap=66752 segments=0 limb_ops=1954 scan_bits=250001
        PASS [   0.006s] (5/6) before::meter skyline_validate_hugeleaf_envelope
```

Explanation:

Verified by run, exactly as predicted. Removing the validator cursor's topology (unary-run) scan record drops the SKYLINE_VALIDATE_DENSE / ALT_SPINE / DECODE_DENSE scan readings from 375006 to 125005 bits (two thirds of the walk's scan work vanishes), every envelope stays green (the SweepEnvelope has a scan ceiling and no scan floor; sweep_metered at tests/meter.rs:1716-1720, read, asserts only `scan_bits <= env.scan_bits`; limb 0 >= 0 holds trivially), and eq_exit's scan pin moves 2054 -> 2050, in band. So these rows have no live lower bound on the scan column; the DECODE_DENSE comment at tests/meter.rs:262 names 'touch and scan floors' its four-column Envelope does not have (read). The validate-to-Ok(()) stub alternative was not run.

## envelopes-b-22

Outcome: demonstrated

Witness agent: 1

Test code:

```rust
// Edit (crates/before/tests/meter.rs:8487-8504), applied for Runs A/A2 and left in place through Run D:
// deleted the whole `memo_comb_resolution_reads_linear` test (doc comment, #[test], fn body).
// Tests run: the existing parity scanners
//   amp_board_smoke::band_tests_and_registry_citations_stay_paired
//   superlinear_tripwires::superlinear_tripwires_match_the_committed_roster
```

Command:

```text
Run A (filter under party-1); citation grep: grep -rn memo_comb_resolution_reads_linear crates/before/src .cargo tools crates/before/tests/amp_board_smoke.rs crates/before/tests/superlinear_tripwires.rs
```

Output:

```text
        PASS [   0.005s] ( 1/25) before::amp_board_smoke band_tests_and_registry_citations_stay_paired
        PASS [   0.013s] ( 8/25) before::superlinear_tripwires superlinear_tripwires_match_the_committed_roster
(citation grep: no matches)
```

Explanation:

Verified by run: with `memo_comb_resolution_reads_linear` deleted from tests/meter.rs, both parity scanners pass, because band_test_names keeps only `_is_flat_per_unit`/`_band` names (amp_board_smoke.rs:331, read) and the tripwire scan keeps only `_reads_superlinear` (superlinear_tripwires.rs:87, read); the name is cited nowhere under crates/before/src, .cargo, or tools (grep). `just gate` itself was not run (not permitted), so the claim that every gate leg is green is inferred from those two scanners plus the absence of any other citation; the registry prose mischaracterization (registry.rs:1724-1725 vs the x2.5 ratio at tests/meter.rs:8426) was checked by reading only.

## tests-other-16

Outcome: demonstrated

Witness agent: 1

Test code:

```rust
// Edit (crates/before/src/lib.rs, after line 431), applied for Run B only:
//   +/// witness:1 scratch: a whole-crate foreign re-export (tests-other-16).
//   +pub use bytes;
// Tests run: the existing tests/foreign_reexport.rs::dependency_reexports_match_the_committed_roster
// and src/testing/surface_coverage/tests.rs::roster_is_total_over_the_public_fn_surface.
```

Command:

```text
Run B (filter under meter-core-8)
```

Output:

```text
        PASS [   0.005s] (5/7) before testing::surface_coverage::tests::roster_is_total_over_the_public_fn_surface
        PASS [   0.010s] (7/7) before::foreign_reexport dependency_reexports_match_the_committed_roster
```

Explanation:

Verified by run: with `pub use bytes;` at the crate root (the whole `bytes` dependency crate re-exported), `dependency_reexports_match_the_committed_roster` passes against its empty roster, because the pathed arm requires `bytes::` or `::bytes` on the line (foreign_reexport.rs:83-88, read) and the whole-crate arm requires the text `pub extern crate`; the in-tree pub-fn roster test passes too. The `pub type Blob = Bytes;` spelling and the `[dependencies.<name>]`-table hole in dependency_names (lines 39-41) were assessed by reading only; `just surface-totality` (the surfacecheck jaw) was not run.

## tests-other-24

Outcome: demonstrated

Witness agent: 1

Test code:

```rust
// Edit (crates/before/tests/verdict_matrix.rs:1369-1370), applied for Runs A/A2 and left in place through Run D:
//   #[test]
//   +#[ignore]
//   fn polarity_flipped_sweep_reads_inverted_through_the_matrix() {
// Test run: the existing verdict_matrix::inverted_verdict_tripwires_match_the_committed_roster.
```

Command:

```text
Run A (filter under party-1)
```

Output:

```text
        PASS [   0.016s] ( 9/25) before::verdict_matrix inverted_verdict_tripwires_match_the_committed_roster
```

Explanation:

Verified by run: with `#[ignore]` on the polarity-flipped twin, the roster test still finds the name through `fn_name` (verdict_matrix.rs:1232-1257, which reads `fn` lines only, read) and passes, while the twin itself would never execute in a normal run. The sibling scanners' equivalent blindness (superlinear_tripwires.rs:80-87 scans `fn` lines; amp_board_smoke.rs:323-327 skips every `#[` line while armed) was verified by reading; tools/citecheck's exclusion of these rosters was read from the finding's quote, not from the tool.

## benches-examples-12

Outcome: inconclusive

Witness agent: 1

Test code:

```rust
// No test: the construction is a `cargo bench` invocation, which is outside my permitted commands.
```

Command:

```text
git log --oneline -S projection_shrink -- crates/before; git log --oneline -1 cd171c29
```

Output:

```text
b28c35ad6 bench: the allocation-strategy record, per site — presize and stacks legs
----
cd171c29 smallvec retired: the parser stacks are plain Vec, priced and adopted
```

Explanation:

Not constructed (benches are forbidden). Read-only git corroborates the history claim: `git log -S projection_shrink` over crates/before finds only the introducing commit b28c35ad6, and cd171c29 is the sibling stacks leg's closing commit. Whether the presize-resident lines move under `--cfg before_alloc_ab="projection_growth"` and whether anything asserts them was assessed only from the finding's quoted bench doc; I did not read benches/presize.rs myself.

## fuzz-guests-pins-26

Outcome: inconclusive

Witness agent: 1

Test code:

```rust
// No test: `just wasm32-pins` (a wasmtime guest run) is outside my permitted commands.
```

Command:

```text
(none run)
```

Output:

```text
(none)
```

Explanation:

Not constructed. I did not read wasm32-pins/Cargo.toml or pins.rs myself (my one grep attempt failed on shell globbing and I did not retry), so I neither corroborate nor dispute the quoted profile lines; the liveness-self-test absence needs the guest run the construction names.

## fuzz-guests-pins-37

Outcome: demonstrated

Witness agent: 1

Test code:

```rust
// No cargo test; mechanical roster comparison:
// git ls-files '*Cargo.lock'; awk 'NR>=345 && NR<=352' justfile; grep -A1 'name = "wasmtime"' <the three lockfiles> | grep version; grep -n supply-chain .github/workflows/ci.yml
```

Command:

```text
git ls-files '*Cargo.lock'; awk 'NR>=345 && NR<=352 {printf "%6d\t%s\n", NR, $0}' justfile; grep -A1 'name = "wasmtime"' crates/before/wasm32-pins/Cargo.lock crates/before/fuzzfit/Cargo.lock crates/before-fuelscape/Cargo.lock | grep version; grep -n supply-chain .github/workflows/ci.yml
```

Output:

```text
Cargo.lock
crates/before-fuelscape/Cargo.lock
crates/before/fuzz/Cargo.lock
crates/before/fuzzfit/Cargo.lock
crates/before/surfacecheck/Cargo.lock
crates/before/wasm32-pins/Cargo.lock
   345	# Audit advisories on every lockfile and hold the workspace to single crate versions.
   346	supply-chain:
   347	    cargo audit
   348	    cargo audit --file crates/before/fuzz/Cargo.lock
   349	    cargo audit --file crates/before/fuzzfit/Cargo.lock
   350	    cargo audit --file crates/before-fuelscape/Cargo.lock
   351	    cargo audit --file crates/before/surfacecheck/Cargo.lock
   352	    cargo deny --workspace check bans
crates/before/wasm32-pins/Cargo.lock-version = "47.0.3"
crates/before/fuzzfit/Cargo.lock-version = "47.0.4"
crates/before-fuelscape/Cargo.lock-version = "47.0.4"
171:      - name: just supply-chain
172:        run: just supply-chain
```

Explanation:

Mechanically verified (no cargo audit run, which is outside my permitted commands): six lockfiles are committed, the supply-chain recipe audits five, wasm32-pins/Cargo.lock is the one omitted and pins wasmtime 47.0.3 where fuzzfit and fuelscape lock 47.0.4 (a distinct resolution), and CI runs `just supply-chain` (ci.yml:171-172). Because the recipe never names that file, an advisory confined to it cannot fail the leg; the planted-advisory construction was not run.

## fuzzfit-bands-5

Outcome: inconclusive

Witness agent: 1

Test code:

```rust
// No test: the construction patches the fuzzfit harness's wasm.rs and runs `just fuzzfit`, outside my permitted commands.
```

Command:

```text
grep -n REFIT_TOLERANCE crates/before/fuzzfit/harness/src/bands.rs
```

Output:

```text
52://!               10^(REFIT_TOLERANCE −/+ the key's honest prefix divergence) )
218:pub const REFIT_TOLERANCE: f64 = 0.7;
984:/// within [`REFIT_TOLERANCE`] — so coverage decay, a classification flip
```

Explanation:

Not constructed (needs the detached fuzzfit workspace and a wasmtime run). Grep confirms one global `REFIT_TOLERANCE: f64 = 0.7` at bands.rs:218; whether a uniform 2x fuel undercount on one key passes every leg was not tested.

## fuzzfit-bands-27

Outcome: demonstrated

Witness agent: 1

Test code:

```rust
// No cargo test; mechanical vocabulary comparison:
// guest exports: grep -rn 'extern "C" fn ff_' crates/before/fuzzfit/guest | sed 's/.*fn \(ff_[a-z0-9_]*\).*/\1/' | sort
// band kernels:  grep -o 'ff_[a-z0-9_]*' crates/before/fuzzfit/harness/src/bands.rs | sort -u
// comm -23 exports kernels | wc -l ; grep -c ff_span_join kernels
```

Command:

```text
grep -rn 'extern "C" fn ff_' crates/before/fuzzfit/guest | wc -l; grep -o 'ff_[a-z0-9_]*' crates/before/fuzzfit/harness/src/bands.rs | sort -u | wc -l; comm -23 guest_exports.txt band_kernels.txt | wc -l; grep -c ff_span_join band_kernels.txt
```

Output:

```text
107
44
63
0
```

Explanation:

Mechanically verified, not run through `just fuzzfit`: bands.rs names 44 distinct `ff_*` kernels while the guest exports 107 `extern "C" fn ff_*` entry points, 63 of which appear nowhere in bands.rs; `ff_span_join` (the construction's example) is not among the 44. The 63 includes any self-test or non-public-op exports (I did not classify them), so 'about fifty-six further measured kernels for public operations' is consistent with but not exactly confirmed by the count. The finding's quoted claim text at enforce.rs:441-443 was not independently read.

## fuelscape-render-19

Outcome: inconclusive

Witness agent: 1

Test code:

```rust
// No test: the construction runs the fuelscape binary (`cargo run --bin fuelscape`) or `cargo build -p before` with an edited committed dataset, outside my permitted commands.
```

Command:

```text
awk 'NR>=154 && NR<=160' crates/before-fuelscape/src/bin/fuelscape.rs; awk 'NR>=62 && NR<=68' crates/before/build.rs; grep -n FUELSCAPE_TIP justfile
```

Output:

```text
   155	    let meta = RenderMeta {
   156	        commit: std::env::var("FUELSCAPE_TIP").unwrap_or_else(|_| "untracked".into()),
   157	        base_seed: plan.base_seed,
   158	        samples_per_column: plan.samples_per_column,
   159	    };
    64	        assert!(
    65	            doc["meta"]["commit"].is_string(),
    66	            "{file}: the measurement commit is missing"
    67	        );
678:    FUELSCAPE_TIP=$(git rev-parse HEAD) FUZZFIT_GUEST_WASM=... cargo run --release --bin fuelscape -- --out ... {{ args }}
```

Explanation:

Not constructed. Reading confirms the three quoted sites: the stamp falls back to the literal `"untracked"` (fuelscape.rs:156), build.rs's only shape check is `is_string()` (build.rs:64-67), and the recipe stamps `git rev-parse HEAD` with no dirty-tree check (justfile:678). That `"untracked"` passes `is_string()` follows from the code, but no run was made.

## gate-legs-3

Outcome: demonstrated

Witness agent: 1

Test code:

```rust
// No cargo test; mechanical roster comparison (shared with fuzz-guests-pins-37) plus the hand-enumerated rosters:
// grep -n 'fuzz, fuzzfit, fuelscape' deny.toml rust-toolchain.toml .github/workflows/ci.yml justfile
```

Command:

```text
grep -n "fuzz, fuzzfit, fuelscape" deny.toml rust-toolchain.toml .github/workflows/ci.yml justfile; git ls-files '*Cargo.lock'; grep -n supply-chain .github/workflows/ci.yml
```

Output:

```text
deny.toml:4:# detached workspaces (fuzz, fuzzfit, fuelscape, surfacecheck) are
rust-toolchain.toml:3:# directory, so the fuzz, fuzzfit, fuelscape, and surfacecheck
justfile:330:# and each detached workspace (fuzz, fuzzfit, fuelscape, surfacecheck) —
(six lockfiles listed, wasm32-pins/Cargo.lock among them; recipe audits five: see fuzz-guests-pins-37)
171:      - name: just supply-chain
172:        run: just supply-chain
```

Explanation:

Mechanically verified (no cargo audit run): the four-name detached-workspace enumeration recurs in deny.toml:4, rust-toolchain.toml:3, and justfile:330 without wasm32-pins, while a sixth lockfile exists at crates/before/wasm32-pins/Cargo.lock that the supply-chain recipe (justfile:347-351) never audits; the leg's own comment (justfile:329-330) claims every lockfile. The ci.yml:117 quote and the run-33429688343 history were not independently checked. The planted-advisory construction was not run.

## module-graph-1

Outcome: demonstrated

Witness agent: 1

Test code:

```rust
// No cargo test (a test referencing before::recurse::grow from an integration binary would fail the whole build);
// mechanical cfg check:
// awk 'NR>=100 && NR<=109' crates/before/src/recurse.rs ; awk 'NR>=116 && NR<=122' crates/before/src/recurse.rs
// grep -n MAX_GROWN_STACK_SEGMENTS crates/before/src/meter/board/ceilings.rs
// grep '= envelope(' crates/before/tests/meter.rs | grep -v 'envelope( *[0-9_]*, *0,'
```

Command:

```text
awk 'NR>=116 && NR<=122 {printf "%6d\t%s\n", NR, $0}' crates/before/src/recurse.rs; grep -n MAX_GROWN_STACK_SEGMENTS crates/before/src/meter/board/ceilings.rs; grep -c '= envelope(' crates/before/tests/meter.rs; grep '= envelope(' crates/before/tests/meter.rs | grep -v 'envelope( *[0-9_]*, *0,'
```

Output:

```text
   118	#[cfg(test)]
   119	macro_rules! descend {
   120	    ($depth:expr, $call:expr) => {
   121	        if $crate::recurse::should_grow($depth) {
   122	            $crate::recurse::grow(|| $call)
83:pub const MAX_GROWN_STACK_SEGMENTS: u64 = 1;
15
(no envelope row with a nonzero segments argument)
```

Explanation:

Mechanically verified by reading the cfg attributes: `grow` (recurse.rs:100-109, the counter's only writer via `SEGMENTS_GROWN.fetch_add`) and the `descend!` macro that routes through it (recurse.rs:118) are both `#[cfg(test)]`, while the counter and its readers are `cfg(any(test, feature = "meter"))` (recurse.rs:68-86) and `meter::stack_segments` (meter.rs:3552-3554) just loads it; so every integration binary, example, and bench reads a counter nothing in that build can increment. The 15 `= envelope(` rows in tests/meter.rs all pin segments 0 and ceilings.rs:83 pins MAX_GROWN_STACK_SEGMENTS = 1 (read). The lib-test ratchet at meter/tests.rs:407-454 (read) is the one live writer. Neither the compile-failure construction nor the amp_board example was run.

## suite-economics-1

Outcome: inconclusive

Witness agent: 1

Test code:

```rust
// No test: the construction is `cargo test -p before --all-features --test meter` (libtest's shared process), which my brief restricts to nextest and which would run the whole meter binary.
```

Command:

```text
(none run)
```

Output:

```text
(none)
```

Explanation:

Not constructed. Reading confirms the premise: tests/meter.rs:17-21 states process-per-test isolation as a requirement, ISOLATION_NOTE at :354-355 only decorates failures, and `metered` at :363-367 resets the process-global meters with no runtime check that a sibling scenario is not concurrently running. Whether readings actually flip under a shared-process runner was not observed.

## party-9

Outcome: demonstrated

Witness agent: 2

Test code:

```rust
/// party-9: `Party::shape` says "nothing allocates"; a party 200 levels
/// deep drained through `Regions` should then leave peak heap at zero.
#[test]
fn party_9_shape_drain_allocates_nothing() {
    let packed = Shape::IdSpine.packed_flagged(200, false);
    let party = Party::decode(&packed.bytes[..]).expect("generated id is canonical");
    HEAP.reset_peak_usage();
    let baseline = HEAP.current_usage();
    let regions = std::hint::black_box(party.shape().count());
    let peak = HEAP.peak_usage().saturating_sub(baseline);
    eprintln!("MEASURED party_9: depth=200 regions={regions} peak_heap_bytes={peak}");
    assert_eq!(peak, 0, "shape drain allocated {peak} bytes on a 200-level id spine");
}
// (crates/before/tests/zz_witness_1.rs; `#[global_allocator] static HEAP: PeakAlloc = PeakAlloc;` at file top; features meter+limb-meter+scan-meter)
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --no-fail-fast -E 'binary(zz_witness_1) | test(/zz_witness/) | test(rank_pair_mismatch_envelope)'
```

Output:

```text
MEASURED party_9: depth=200 regions=201 peak_heap_bytes=64
thread 'party_9_shape_drain_allocates_nothing' panicked at crates/before/tests/zz_witness_1.rs:25:5:
assertion `left == right` failed: shape drain allocated 64 bytes on a 200-level id spine
  left: 64
 right: 0
```

Explanation:

Ran (execution 1). Draining `Party::shape()` on the registry's 200-level id spine (Shape::IdSpine.packed_flagged(200, false), decoded through the public `Party::decode`) allocates 64 bytes of peak heap under the same PeakAlloc meter tests/meter.rs uses: the two BitStacks (path, right_present) behind Regions -> PartyWalk -> IdLeafCursor spill words past 64 levels. The rustdoc sentence at party.rs:480-481 ("nothing allocates") is false for parties deeper than 64 levels. I verified by reading that shape.rs:206 opens `PartyWalk`, skyline/shape.rs:77-78 wraps `IdLeafCursor`, and overlay.rs:474-477 holds two `BitStack`s whose `push` (codec/stack.rs:49-50) does `self.words.push(self.top)` at 64 bits.

## party-22

Outcome: demonstrated

Witness agent: 2

Test code:

```rust
// crates/before/src/party/ops/index.rs (scratch accessor added inside `impl<'a> IdIndex<'a>`):
    /// witness:2 scratch accessor: the table's entry count (one `u32` each).
    #[cfg(test)]
    pub(crate) fn zz_witness_table_len(&self) -> Option<usize> {
        self.rights.as_ref().map(Vec::len)
    }

// crates/before/src/party/tests.rs (appended):
/// witness:2 (party-22): `IdIndex`'s doc says its table is "strictly
/// smaller than the operand itself"; one `u32` per both-present node must
/// then be fewer bits than the operand on every input, the one-node id and
/// the left comb `L_d` included.
#[test]
fn zz_witness_party_22_index_table_is_strictly_smaller_than_operand() {
    let p = Party::try_from((1u8, (1u8, 0u8))).unwrap();
    let table = IdIndex::build(p.as_bits())
        .zz_witness_table_len()
        .expect("small stream indexes");
    let operand_bits = p.encoded_bits();
    eprintln!(
        "MEASURED party_22: party={p} operand_bits={operand_bits} table_entries={table} table_bits={}",
        32 * table
    );
    let mut worst = 0.0f64;
    for d in [1usize, 4, 16, 64, 256] {
        // L_0 = (1, 0), L_{k+1} = (L_k, 1): d both-present nodes, 4d + 4 bits.
        let mut s = String::from("(1, 0)");
        for _ in 0..d {
            s = format!("({s}, 1)");
        }
        let comb: Party = s.parse().unwrap();
        let entries = IdIndex::build(comb.as_bits())
            .zz_witness_table_len()
            .expect("small stream indexes");
        let ratio = (32 * entries) as f64 / comb.encoded_bits() as f64;
        eprintln!(
            "MEASURED party_22: comb d={d} operand_bits={} table_bits={} ratio={ratio:.3}",
            comb.encoded_bits(),
            32 * entries
        );
        worst = worst.max(ratio);
    }
    let table_bits = (32 * table) as u64;
    assert!(
        table_bits < operand_bits,
        "table {table_bits} bits is not strictly smaller than the {operand_bits}-bit operand"
    );
    assert!(worst < 1.0, "the comb's table reaches {worst:.3}x the operand's bits");
}
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --no-fail-fast -E 'binary(zz_witness_1) | test(/zz_witness/) | test(rank_pair_mismatch_envelope)'
```

Output:

```text
MEASURED party_22: party=(1, (1, 0)) operand_bits=8 table_entries=1 table_bits=32
MEASURED party_22: comb d=1 operand_bits=8 table_bits=32 ratio=4.000
MEASURED party_22: comb d=4 operand_bits=20 table_bits=128 ratio=6.400
MEASURED party_22: comb d=16 operand_bits=68 table_bits=512 ratio=7.529
MEASURED party_22: comb d=64 operand_bits=260 table_bits=2048 ratio=7.877
MEASURED party_22: comb d=256 operand_bits=1028 table_bits=8192 ratio=7.969
thread 'party::tests::zz_witness_party_22_index_table_is_strictly_smaller_than_operand' panicked at crates/before/src/party/tests.rs:1199:5
```

Explanation:

Ran (execution 1). `IdIndex::build` on `(1, (1, 0))` (8 encoded bits) allocates a one-entry u32 table (32 bits); on the left comb L_d the table is 32d bits against 4d + 4 operand bits, ratio 7.969 at d = 256 and approaching 8. The doc sentence at index.rs:43-44 ("strictly smaller than the operand itself") is false on every input with a both-present node. The table length was read through a cfg(test) accessor I added and then removed; the rest is public API (`Party::try_from`, `FromStr`, `encoded_bits`).

## rank-4

Outcome: demonstrated

Witness agent: 2

Test code:

```rust
/// rank-4: "A rank's representation never is larger than the version it
/// measures" — checked at the committed goldens' scale, bytes and bits.
#[test]
fn rank_4_rank_never_larger_than_version_at_small_scale() {
    let v: Version = "(0, 1, 0)".parse().unwrap();
    let r = v.rank();
    eprintln!(
        "MEASURED rank_4: version_bytes={} version_bits={} rank_bytes={} rank={:?}",
        v.encode().len(),
        v.encoded_bits(),
        r.encode().len(),
        r.encode()
    );
    assert_eq!(r.encode(), vec![0x60, 0x00], "the committed golden for 1/2");
    let one = Version::try_from(1u64).unwrap();
    eprintln!(
        "MEASURED rank_4: one_bytes={} one_bits={} one_rank_bytes={} one_rank={:?}",
        one.encode().len(),
        one.encoded_bits(),
        one.rank().encode().len(),
        one.rank().encode()
    );
    let mut violations = Vec::new();
    for (name, ver, rank) in [("(0, 1, 0)", &v, &r), ("1", &one, &one.rank())] {
        if rank.encode().len() > ver.encode().len() {
            violations.push(format!(
                "{name}: rank {} bytes > version {} bytes",
                rank.encode().len(),
                ver.encode().len()
            ));
        }
        if (rank.encode().len() as u64) * 8 > ver.encoded_bits() {
            violations.push(format!(
                "{name}: rank {} bits (bytes x 8) > version {} live bits",
                rank.encode().len() * 8,
                ver.encoded_bits()
            ));
        }
    }
    eprintln!("MEASURED rank_4: violations={violations:?}");
    assert!(violations.is_empty(), "{violations:?}");
}
```

Command:

```text
cargo nextest run -p before --all-features --no-fail-fast --success-output immediate -E 'test(rank_4_rank_never_larger_than_version_at_small_scale) | test(span_causally_24_k_hole_query_touches_do_not_scale_with_k) | test(crate_root_24_space_ratio_against_naive_transcriptions) | test(zz_witness_testing_diff_gen_26_fold_door_floors_sit_above_linear_reference)'
```

Output:

```text
MEASURED rank_4: version_bytes=2 version_bits=9 rank_bytes=2 rank=[96, 0]
MEASURED rank_4: one_bytes=1 one_bits=4 one_rank_bytes=1 one_rank=[128]
MEASURED rank_4: violations=["(0, 1, 0): rank 16 bits (bytes x 8) > version 9 live bits", "1: rank 8 bits (bytes x 8) > version 4 live bits"]
thread 'rank_4_rank_never_larger_than_version_at_small_scale' panicked at crates/before/tests/zz_witness_1.rs:68:5
```

Explanation:

Ran (executions 1 and 3; the first run included a wrong premise from the finding, fixed for run 3). The bit-denominated claim holds: "(0, 1, 0)" is 9 live bits and its rank golden [0x60, 0x00] is 16 bits; Version 1 is 4 live bits and its rank golden [0x80] is 8 bits, so in the provenance pin's own denomination the rank is larger at both scales. The finding's byte-level premise is wrong, though: "(0, 1, 0)" encodes to 2 bytes (9 bits plus marker), not "7 live bits, one byte", so rank bytes equal version bytes (2 = 2 and 1 = 1) and the byte inequality "rank bytes 2 > version bytes 1" is refuted. The rustdoc at rank.rs:198-202 is false in bits, tied in bytes, at small scale.

## span-causally-24

Outcome: demonstrated

Witness agent: 2

Test code:

```rust
/// span-causally-24: a k-hole `until` conjunction on a fixed probe `v`
/// should read touches linear in the input; if the fused walk folds every
/// live pair per elementary interval, halving k roughly halves the reading.
#[test]
fn span_causally_24_k_hole_query_touches_do_not_scale_with_k() {
    const K: usize = 64;
    const FRESH: u64 = 10_000;
    let mut root = Clock::seed();
    let mut hs: Vec<Clock> = root.forks(K as u64).collect();
    // Distinct tick counts per hole so their join keeps every plateau
    // (uniform ticks would join flat).
    for (i, h) in hs.iter_mut().enumerate() {
        for _ in 0..=i {
            h.tick();
        }
    }
    let mut v = Version::new();
    for h in &hs {
        v = v | h.version();
    }
    let fresh: Vec<Clock> = root.forks(FRESH).collect();
    for (j, mut f) in fresh.into_iter().enumerate() {
        for _ in 0..=(j % 3) {
            f.tick();
        }
        v = v | f.version();
    }
    let v_bits = v.encoded_bits();
    for h in &hs {
        assert!(h.version() <= &v, "every hole bound sits at or below the probe");
    }
    let measure = |k: usize| -> (bool, u64, u64) {
        let mut q = until(hs[0].version());
        for h in &hs[1..k] {
            q = q & until(h.version());
        }
        suanpan::touch_meter::reset();
        meter::reset_scan_bits();
        let r = q.contains(&v);
        (r, suanpan::touch_meter::touches(), meter::scan_bits())
    };
    let (r64, t64, s64) = measure(64);
    let (r32, t32, s32) = measure(32);
    let (r16, t16, s16) = measure(16);
    eprintln!(
        "MEASURED span_causally_24: v_bits={v_bits} k=64 contains={r64} touches={t64} scan_bits={s64}"
    );
    eprintln!(
        "MEASURED span_causally_24: v_bits={v_bits} k=32 contains={r32} touches={t32} scan_bits={s32}"
    );
    eprintln!(
        "MEASURED span_causally_24: v_bits={v_bits} k=16 contains={r16} touches={t16} scan_bits={s16}"
    );
    assert!(
        (t64 as f64) < 1.25 * (t32 as f64),
        "touches at k=64 ({t64}) are not within 1.25x of k=32 ({t32}) on the same probe"
    );
}
```

Command:

```text
cargo nextest run -p before --all-features --no-fail-fast --success-output immediate -E 'test(rank_4_rank_never_larger_than_version_at_small_scale) | test(span_causally_24_k_hole_query_touches_do_not_scale_with_k) | test(crate_root_24_space_ratio_against_naive_transcriptions) | test(zz_witness_testing_diff_gen_26_fold_door_floors_sit_above_linear_reference)'
```

Output:

```text
MEASURED span_causally_24: v_bits=56986 k=64 contains=false touches=1288319 scan_bits=59567
MEASURED span_causally_24: v_bits=56986 k=32 contains=false touches=644160 scan_bits=58169
MEASURED span_causally_24: v_bits=56986 k=16 contains=false touches=322080 scan_bits=57525
thread 'span_causally_24_k_hole_query_touches_do_not_scale_with_k' panicked at crates/before/tests/zz_witness_1.rs:124:5
```

Explanation:

Ran (execution 3; execution 1's first attempt used uniform ticks, which joined flat to a 67-bit probe, so I rebuilt the probe with distinct tick counts). On one fixed 56,986-bit probe, `until(h_1) & ... & until(h_k)` reads 1,288,319 accumulator touches at k = 64, 644,160 at k = 32, 322,080 at k = 16: halving k halves the touch reading while the scan-bits reading stays ~58k (each stream read once). Touches are ~21.6 per scanned bit at k = 64, ~5.6 at k = 16. This is the Θ(k · #intervals) fold count the finding predicts; the "linear time" wording at causally.rs:85-86, query.rs:28-30 and polarity.rs:211-212 holds only for the bits-decoded currency.

## skyline-coding-29

Outcome: demonstrated

Witness agent: 2

Test code:

```rust
/// skyline-coding-29: the render merge's limb work on WideTail(s, s) should
/// stay below the quadratic term `s * ceil(s/64)` if the class is n log n,
/// and its doubling ratio should stay near 2.
#[test]
fn skyline_coding_29_render_limb_ops_are_not_quadratic() {
    let render_limb_ops = |s: usize| -> u64 {
        let version = Shape::WideTail.packed2(s, s).version();
        let bits = version.encoded_bits();
        meter::reset_limb_ops();
        std::hint::black_box(version.to_string());
        let ops = meter::limb_ops();
        eprintln!(
            "MEASURED skyline_coding_29: s={s} input_bits={bits} limb_ops={ops} s*ceil(s/64)={}",
            s * s.div_ceil(64)
        );
        ops
    };
    let o1 = render_limb_ops(1024);
    let o2 = render_limb_ops(2048);
    let o4 = render_limb_ops(4096);
    eprintln!(
        "MEASURED skyline_coding_29: ratio(2048/1024)={:.3} ratio(4096/2048)={:.3}",
        o2 as f64 / o1 as f64,
        o4 as f64 / o2 as f64
    );
    for (s, o) in [(1024usize, o1), (2048, o2), (4096, o4)] {
        assert!(
            (o as usize) < s * s.div_ceil(64),
            "render limb ops {o} at s={s} reach the quadratic term {}",
            s * s.div_ceil(64)
        );
    }
    assert!(
        o4 as f64 / o2 as f64 <= 3.5,
        "doubling ratio {:.3} exceeds 3.5 at s=4096",
        o4 as f64 / o2 as f64
    );
}
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --no-fail-fast -E 'binary(zz_witness_1) | test(/zz_witness/) | test(rank_pair_mismatch_envelope)'
```

Output:

```text
MEASURED skyline_coding_29: s=1024 input_bits=5122 limb_ops=25716 s*ceil(s/64)=16384
MEASURED skyline_coding_29: s=2048 input_bits=10242 limb_ops=84196 s*ceil(s/64)=65536
MEASURED skyline_coding_29: s=4096 input_bits=20482 limb_ops=299460 s*ceil(s/64)=262144
MEASURED skyline_coding_29: ratio(2048/1024)=3.274 ratio(4096/2048)=3.557
thread 'skyline_coding_29_render_limb_ops_are_not_quadratic' panicked at crates/before/tests/zz_witness_1.rs:140:9
```

Explanation:

Ran (execution 1). `Display` on WideTail(s, s) reads 25,716 / 84,196 / 299,460 limb ops at s = 1024 / 2048 / 4096, each above the quadratic term s·⌈s/64⌉ (16,384 / 65,536 / 262,144) and growing at doubling ratios 3.274 then 3.557 (an n log n mechanism would sit near 2.2; a quadratic one tends to 4). The input is 5s + 2 bits, so the class is quadratic in the input as the finding says; the fuelscape contract text "superlinear, subquadratic time"/"n log n" (ops.rs:331-332) is not what the counter reads.

## skyline-sweep-place-masked-20

Outcome: demonstrated

Witness agent: 2

Test code:

```rust
// Mutant A (execution 4), exactly the finding's construction, in crates/before/src/version/skyline/place/filter.rs:
-            if !side.lo.live && !side.hi.live {
-                *slot = None;
-                live -= 1;
-            }
+            // witness:2 MUTANT (skyline-sweep-place-masked-20): the settled-side
+            // drop (lines 427-430) and the endpoint-liveness updates (439-440)
+            // deleted, exactly as the finding's construction states.
         }
         // A walk left holding only settled holes is decided: every hole was
         // refuted both ways, so nothing subtracts from the segment and nothing
         // more can change the verdict.
         if live == 0 {
             break;
         }
-        // A probe endpoint whose every pair is settled stops being scanned.
-        walk.lo_live = walk.lo_live && walk.sides.iter().flatten().any(|side| side.lo.live);
-        walk.hi_live = walk.hi_live && walk.sides.iter().flatten().any(|side| side.hi.live);
         debug_assert!(

// Mutant B (execution 5), the softer form that keeps the slot drop so the per-interval debug_assert stays true:
-        // A walk left holding only settled holes is decided: every hole was
-        // refuted both ways, so nothing subtracts from the segment and nothing
-        // more can change the verdict.
-        if live == 0 {
-            break;
-        }
-        // A probe endpoint whose every pair is settled stops being scanned.
-        walk.lo_live = walk.lo_live && walk.sides.iter().flatten().any(|side| side.lo.live);
-        walk.hi_live = walk.hi_live && walk.sides.iter().flatten().any(|side| side.hi.live);
+        // witness:2 MUTANT (skyline-sweep-place-masked-20, softer form): the
+        // `live == 0` early return and the endpoint-liveness updates deleted;
+        // the settled-side drop above is kept so the per-interval assert
+        // stays true and the walk runs every stream to exhaustion.
+        let _ = live;
         debug_assert!(
```

Command:

```text
# execution 4 (mutant A, batched with three unrelated mutants):
cargo nextest run -p before --all-features --no-fail-fast -E 'test(/skyline::place::tests/) | test(coverage_is_exact_on_the_two_party_grid) | test(/causally::tests/) | binary(verdict_matrix) | test(/laws::tests/) | test(/skyline::watermark::tests/) | test(/span_borsh/) | test(/serde_impls::tests/) | test(/span::tests/) | binary(fuzz_seeds) | test(exhaustive_small) | test(/skyline::fill::tests/) | test(grow_matches_brute_force) | test(grow_minimal)'
# execution 5 (mutant B, batched with watermark mutant 2):
cargo nextest run -p before --all-features --no-fail-fast -E 'test(/skyline::place::tests/) | test(/causally::tests/) | binary(verdict_matrix) | test(/laws::tests/) | test(/algebraic_laws/) | test(/skyline::watermark::tests/) | test(/skyline::fill::tests/)'
```

Output:

```text
# execution 4, mutant A:
thread 'causally::tests::coverage_witness_matrix' panicked at crates/before/src/version/skyline/place/filter.rs:437:9:
a surviving hole keeps its dominated endpoint's pair live: settling it forces the dominating pair's settle at the same interval, and the joint settle drops the side
(same panic in causally::tests::coverage_is_exact_on_the_two_party_grid, testing::algebraic_laws::tests::laws_hold_on_organic_populations, version::skyline::place::tests::filter_coverage_organic_witnesses, version::skyline::place::tests::filter_coverage_matches_the_composed_sweeps)
# execution 5, mutant B:
Summary [  13.746s] 93 tests run: 87 passed, 6 failed, 808 skipped
PASS before version::skyline::place::tests::filter_coverage_organic_witnesses
PASS before version::skyline::place::tests::filter_coverage_matches_the_composed_sweeps
PASS before causally::tests::coverage_is_exact_on_the_two_party_grid
PASS before testing::algebraic_laws::tests::laws_hold_on_organic_populations
PASS before::verdict_matrix production_verdicts_agree_across_the_surfaces
(all 7 verdict_matrix tests, all 5 place organic-witness tests, all 3 place proptests, all causally tests PASS; the 6 failures are fill::tests killed by the unrelated watermark mutant 2 in the same batch)
```

Explanation:

Ran (executions 4 and 5). The literal construction (delete 427-430 and 439-440) is caught, but only by the committed `debug_assert!` at filter.rs:441-450 (line 437 after the deletion), which fires in five tests because settled holes with both pairs dead now stay in their slots; no verdict changes. Deleting only the two exits the debug_assert cannot see (the `live == 0` early `Full` return and the endpoint-cursor drops) passes every place, causally, laws and verdict-matrix test: those two exits are unobserved by any committed verdict or meter, as the finding claims, and the third (the slot drop) is observed by an invariant assert in dev builds only, never by a resource pin. The finding's headline stands in this modified form; its "passes every committed test" sentence is too strong for the literal deletion.

## skyline-watermark-24

Outcome: demonstrated

Witness agent: 2

Test code:

```rust
// Mutant 1 (executions 4 and 6), crates/before/src/version/skyline/watermark.rs drop_below:
     fn drop_below(&mut self, mut residue: Accumulator, on_die: impl FnMut(P)) {
-        for slot in 0..self.followers.len() {
-            if let Some(follower) = &mut self.followers[slot] {
-                debug_assert_eq!(
-                    self.anchor_relative[slot],
-                    self.latent.is_some(),
-                    "a live latent keeps every active follower tagged"
-                );
-                follower.sub_accum(&residue);
-                self.anchor_relative[slot] = false;
-            }
-        }
+        // witness:2 MUTANT 1 (skyline-watermark-24): the latent annihilation
+        // moved ahead of the follower fold, so a live latent changes what
+        // every follower absorbs.
+        let had_latent = self.latent.is_some();
         if let Some(latent) = self.latent.take() {
             // The annihilation: residue = (A − v) − Λ = m − v > 0.
             residue.sub_accum(&latent);
             self.retire(latent);
         }
+        for slot in 0..self.followers.len() {
+            if let Some(follower) = &mut self.followers[slot] {
+                debug_assert_eq!(
+                    self.anchor_relative[slot],
+                    had_latent,
+                    "a live latent keeps every active follower tagged"
+                );
+                follower.sub_accum(&residue);
+                self.anchor_relative[slot] = false;
+            }
+        }
         self.propagate(residue, on_die);
     }

// Mutant 2 (execution 5), line 537:
-                follower.sub_accum(&residue);
+                // witness:2 MUTANT 2 (skyline-watermark-24): polarity flipped.
+                follower.add_accum(&residue);
```

Command:

```text
# execution 6 (mutant 1 alone):
cargo nextest run -p before --all-features --no-fail-fast -E 'test(/skyline::watermark::tests/) | test(/skyline::fill::tests/) | test(/version::tests::tick/) | test(grow_matches_brute_force) | test(grow_minimal)'
# execution 5 (mutant 2, batched with the softer filter mutant):
cargo nextest run -p before --all-features --no-fail-fast -E 'test(/skyline::place::tests/) | test(/causally::tests/) | binary(verdict_matrix) | test(/laws::tests/) | test(/algebraic_laws/) | test(/skyline::watermark::tests/) | test(/skyline::fill::tests/)'
```

Output:

```text
# execution 6, mutant 1 alone:
Summary [   4.735s] 51 tests run: 51 passed, 850 skipped
# execution 5, mutant 2:
PASS before version::skyline::watermark::tests::a_dominated_undercut_subtracts_its_residue_from_live_followers
PASS before version::skyline::watermark::tests::a_drop_inside_the_latent_never_moves_the_minimum
(all seven watermark::tests PASS)
FAIL before version::skyline::fill::tests::arbitrary_pairs_tick_and_flag_identically
FAIL before version::skyline::fill::tests::narrow_undercut_moves_the_live_ledger_relation
FAIL before version::skyline::fill::tests::organic_histories_tick_and_flag_identically
FAIL before version::skyline::fill::tests::tick_is_ticks_one
FAIL before version::skyline::fill::tests::undercut_under_a_live_relation_family_ticks_identically
FAIL before version::skyline::fill::tests::family_pairs_tick_and_flag_identically
thread 'version::skyline::fill::tests::arbitrary_pairs_tick_and_flag_identically' panicked at crates/before/src/version/skyline/fill/tests.rs:83:13:
assertion `left == right` failed: the changed branch must be the oracle's fill
```

Explanation:

Ran (executions 5 and 6; execution 4 also ran mutant 1 but was confounded by the oracle mutation in the same batch, so mutant 1 was rerun alone). Both mutants pass all seven tests in watermark/tests.rs, confirming the finding's sentence. Beyond that file the two differ: mutant 1 (the latent annihilation reordered ahead of the follower fold, i.e. the tagged-follower-with-live-latent arm) survives every fill, watermark, tick and grow test run, so `drop_below`'s value flow under a live latent is exercised by no test I ran; mutant 2 (the polarity flip at line 537) is killed by six fill::tests through the fill walk's follower client. The `a_dominated_undercut_subtracts_its_residue_from_live_followers` proptest passes under the polarity flip, so its undercut must reach the followers through a path other than `drop_below`'s loop.

## codec-bits-15

Outcome: demonstrated

Witness agent: 2

Test code:

```rust
// Three single-site mutants in crates/before/src/codec/build.rs, one per execution.
// truncate (line 218), execution 7:
-                u64::from(self.bytes[whole as usize] >> (8 - rem))
+                // witness:2 MUTANT (codec-bits-15): the re-derived staged
+                // register takes the byte's low bits instead of its high ones.
+                u64::from(self.bytes[whole as usize] & ((1u8 << rem) - 1))
// read_bits chunk (line 297), execution 8:
-                let chunk = u64::from(byte >> (8 - within - take)) & ((1u64 << take) - 1);
+                // witness:2 MUTANT (codec-bits-15, read_bits chunk): `>>` -> `<<`.
+                let chunk = u64::from(byte << (8 - within - take)) & ((1u64 << take) - 1);
// append_bytes carry (line 278), execution 9:
-            self.bytes.push((carry << (8 - r)) | (b >> r));
+            // witness:2 MUTANT (codec-bits-15, append_bytes carry): the body
+            // byte's high bits are shifted the wrong way into the merge.
+            self.bytes.push((carry << (8 - r)) | (b << r));
```

Command:

```text
cargo nextest run -p before --all-features --no-fail-fast -E 'test(/codec::/) | test(/party::tests/) | test(/party::ops/) | test(/skyline::tests/) | test(/skyline::build/) | test(/skyline::encode/) | test(/skyline::text/) | test(/version::tests/) | test(/borsh_impls::tests/) | binary(coincident_span) | binary(fuzz_seeds)'   # run once per mutant (executions 7, 8, 9)
```

Output:

```text
# execution 7 (truncate): Summary [   4.278s] 248 tests run: 214 passed, 34 failed, 653 skipped
  killed by e.g. party::tests::sum_split_is_sum_then_split, party::tests::join_all_matches_the_recursive_oracle, version::skyline::build::tests::merged_right_subtree_reanchors_over_left_leaf, version::skyline::tests::op_paths_yield_identical_bytes, version::tests::join_matrix_matches_oracle, borsh_impls::tests::span_borsh_roundtrips
  panic sites (count): 1014 crates/before/src/version/skyline/overlay.rs:447:25 ; 918 crates/before/src/borsh_impls/tests.rs:1070:54 ; 212 crates/before/src/codec/base.rs:421:9 ; 178 crates/before/src/testing/bridge.rs:157:60
# execution 8 (read_bits chunk): Summary [   3.866s] 248 tests run: 247 passed, 1 failed, 653 skipped
  FAIL before party::tests::zz_witness_party_22_index_table_is_strictly_smaller_than_operand   (my own witness test, unrelated; every committed test passed)
# execution 9 (append_bytes carry): Summary [   4.297s] 248 tests run: 211 passed, 37 failed, 653 skipped
  killed by e.g. party::tests::sum_split_constructed::deep_subtree_splices_verbatim, version::skyline::text::tests::arbitrary_trees_render_and_parse_identically, version::tests::min_ticks_floors_every_history, before::fuzz_seeds committed_seeds_match_the_live_derivation, before::meter span_codec::span_decode_scans_the_second_component_once
```

Explanation:

Ran (executions 7-9). The gap is real but narrower than stated: two of the three builder sites are killed heavily by the downstream suites (truncate: 34 tests; append_bytes: 36 tests), always with panics surfacing in overlay.rs, base.rs, bridge.rs or the borsh tests rather than at the builder, which confirms the "failure would point at the wrong layer" half. The read_bits committed-byte chunk arm (line 297) is different: a non-equivalent operator swap (`>>` to `<<`; e.g. byte 0xB0, pos 0, n 2 reads 2 originally and 0 under the mutant) survives all 247 committed tests in the codec/party/skyline/version/borsh selection plus coincident_span and fuzz_seeds. No test names PackedBuilder or load_be (grep verified: only codec.rs, party/ops/build.rs, codec/{code,build,dsi,bits}.rs and skyline/build.rs mention them).

## crate-root-24

Outcome: demonstrated

Witness agent: 2

Test code:

```rust
/// crate-root-24: "approximately 100x more space-efficient than a naive
/// transcription" — measured on the paper's data scenario at 128 entities
/// (short run), against a boxed-tree transcription and the paper's
/// Appendix A bit coding, both computed from the clocks' paper notation.
#[test]
fn crate_root_24_space_ratio_against_naive_transcriptions() {
    use rand::{Rng, SeedableRng};
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(0x9E37_79B9_7F4A_7C15);
    let n = 128usize;
    let mut clocks = vec![Clock::seed()];
    while clocks.len() < n {
        let round = clocks.len();
        for i in 0..round {
            if clocks.len() >= n {
                break;
            }
            let child = clocks[i].fork();
            clocks.push(child);
        }
    }
    for _ in 0..10_000u32 {
        let parent = rng.gen_range(0..clocks.len());
        let child = clocks[parent].fork();
        clocks.push(child);
        let who = rng.gen_range(0..clocks.len());
        clocks[who].tick();
        let donor = clocks.swap_remove(rng.gen_range(0..clocks.len()));
        let target = rng.gen_range(0..clocks.len());
        clocks[target].join(donor).expect("disjoint");
    }
    // Paper-notation parser: id = "0" | "1" | "(l, r)"; event = n | "(n, l, r)".
    #[derive(Debug)]
    enum T {
        Leaf(u128),
        Node(Vec<T>),
    }
    fn parse(s: &[u8], i: &mut usize) -> T { /* recursive descent over the Display string; see scratchpad copy */ }
    // Boxed transcription: 32 bytes per event node, 16 per event leaf, 24 per id node, 8 per id leaf.
    fn boxed_id(t: &T) -> usize { match t { T::Leaf(_) => 8, T::Node(k) => 24 + k.iter().map(boxed_id).sum::<usize>() } }
    fn boxed_ev(t: &T) -> usize { match t { T::Leaf(_) => 16, T::Node(k) => 32 + k[1..].iter().map(boxed_ev).sum::<usize>() } }
    // Paper Appendix A (as recalled by the witness; label it as such).
    fn encn(n: u128, b: u32) -> usize { if n < (1u128 << b) { 1 + b as usize } else { 1 + encn(n - (1u128 << b), b + 1) } }
    fn enci(t: &T) -> usize { /* 000/001 leaves; 01/10/11 + children */ }
    fn ence(t: &T) -> usize { /* 1 encn(n,2) leaves; 0 00/01/10 children, 0 11 0 xx encn(n,2) children */ }
    let (mut packed, mut boxed, mut paper_bits) = (0usize, 0usize, 0usize);
    for c in &clocks {
        packed += c.encode().len();
        let id = c.party().to_string();
        let ev = c.version().to_string();
        let (mut i, mut j) = (0usize, 0usize);
        let it = parse(id.as_bytes(), &mut i);
        let et = parse(ev.as_bytes(), &mut j);
        boxed += boxed_id(&it) + boxed_ev(&et);
        paper_bits += enci(&it) + ence(&et);
    }
    let k = clocks.len() as f64;
    let paper_bytes = paper_bits as f64 / 8.0;
    eprintln!(
        "MEASURED crate_root_24: entities={n} iters=10000 mean_packed_bytes={:.1} mean_boxed_bytes={:.1} mean_paperA_bytes={:.1} ratio_boxed/packed={:.2} ratio_paperA/packed={:.2}",
        packed as f64 / k, boxed as f64 / k, paper_bytes / k, boxed as f64 / packed as f64, paper_bytes / packed as f64
    );
    let ratio_boxed = boxed as f64 / packed as f64;
    assert!((50.0..=200.0).contains(&ratio_boxed), "boxed-transcription ratio {ratio_boxed:.2} is not approximately 100x");
}
// (full verbatim source, including the elided parser bodies, is in the scratchpad copy before_tests_zz_witness_1.rs)
```

Command:

```text
cargo nextest run -p before --all-features --no-fail-fast --success-output immediate -E 'test(rank_4_rank_never_larger_than_version_at_small_scale) | test(span_causally_24_k_hole_query_touches_do_not_scale_with_k) | test(crate_root_24_space_ratio_against_naive_transcriptions) | test(zz_witness_testing_diff_gen_26_fold_door_floors_sit_above_linear_reference)'
```

Output:

```text
MEASURED crate_root_24: entities=128 iters=10000 mean_packed_bytes=1791.7 mean_boxed_bytes=111631.0 mean_paperA_bytes=1776.0 ratio_boxed/packed=62.31 ratio_paperA/packed=0.99
PASS [   1.746s] (2/4) before::zz_witness_1 crate_root_24_space_ratio_against_naive_transcriptions
```

Explanation:

Ran (execution 3) plus reading. Nothing in the tree produces the 100x figure (verified: results/space_consumption/README.md:29-34 compares only against the paper's Appendix A encoding and reports parity). Measured on the paper's data scenario (128 entities, 10,000 of the paper's 100,000 iterations, seeded ChaCha8), this crate's mean stamp is 1,791.7 bytes; a boxed-tree transcription (32/16/24/8-byte nodes) is 111,631 bytes (62x); the paper's own Appendix A bit coding, as I reconstructed it from memory, is 1,776 bytes (0.99x). So the sentence at lib.rs:3-4 is true only if "naive transcription" means heap-boxed trees (and then ~60x at this run, the same order as 100x), and false if it means the paper's own encoding; the referent is undefined in the doc, exactly as the finding says. The Appendix A tally is my recollection of the paper's coding, not a committed artifact.

## crate-root-40

Outcome: inconclusive

Witness agent: 2

Test code:

```rust
(none: textual finding; the settling instrument `just amp-board-acceptance` is the board, which my brief forbids running)
```

Command:

```text
awk 'NR>=1 && NR<=12' crates/before/tests/meter.rs ; awk 'NR>=350 && NR<=358' crates/before/src/lib.rs
```

Output:

```text
tests/meter.rs:7-11:
//! on value magnitude, tree depth, or encoded size. Today's implementation
//! is far from that — several operations amplify their input by large
//! constants or worse — so every scenario here pins the *current* measured
//! cost, with ×1.25 slack, as a ceiling. A regression fails loudly now; each
//! improvement tightens a committed number.
lib.rs:350-353:
//! The operations in this crate have been very carefully hardened against
//! pathological input shapes. Any asymptotic claim is a hard guarantee that the
//! operation will perform in time proportionate to that bound, for all input
//! sizes, no matter how unlikely and contorted the shape of the input.
```

Explanation:

Not constructed: the only committed instrument that adjudicates which text is stale is the amplification board, which I was told never to run. By reading I confirmed both present-tense texts exist verbatim and contradict each other; which side the board currently supports I did not verify.

## suanpan-tests-16

Outcome: demonstrated

Witness agent: 2

Test code:

```rust
// crates/suanpan/tests/zz_witness_1.rs
#![cfg(feature = "touch-meter")]

use suanpan::{touch_meter, Accumulator, UBig};

/// A two-digit receiver merged with a 64-digit operand costs the narrow
/// side: 4 touches. Without the swap the fold walks the operand's 64
/// digits (128 touches).
#[test]
fn suanpan_tests_16_merge_into_wider_swaps_when_receiver_is_narrower() {
    let mut receiver = Accumulator::new();
    receiver.add_wide(&UBig::from((1u64 << 32) | 1));
    let mut operand = Accumulator::new();
    operand.add_wide(&((UBig::from(1u8) << 2_048usize) - 1u8));
    touch_meter::reset();
    let spare = receiver.merge_into_wider(operand);
    let touches = touch_meter::touches();
    eprintln!("MEASURED suanpan_tests_16: touches={touches}");
    drop(spare);
    assert_eq!(
        touches, 4,
        "merge_into_wider read the wide side: {touches} touches"
    );
}

// Mutant in crates/suanpan/src/accumulator.rs (execution 2):
-    pub fn merge_into_wider(&mut self, mut other: Accumulator) -> Accumulator {
-        if other.digit_count() > self.digit_count() {
-            core::mem::swap(self, &mut other);
-        }
+    pub fn merge_into_wider(&mut self, other: Accumulator) -> Accumulator {
+        // witness:2 MUTANT (suanpan-tests-16): the width swap deleted.
         self.add_accum(&other);
         other
     }
```

Command:

```text
# execution 1 (unmutated): cargo nextest run -p before -p suanpan --all-features --no-fail-fast -E 'binary(zz_witness_1) | test(/zz_witness/) | test(rank_pair_mismatch_envelope)'
# execution 2 (swap deleted): cargo nextest run -p suanpan --all-features --no-fail-fast --success-output immediate -E 'package(suanpan) & (test(/merge/) | test(/metered/) | test(/differential/) | binary(zz_witness_1))'
```

Output:

```text
# execution 1: PASS [   0.007s] ( 1/16) suanpan::zz_witness_1 suanpan_tests_16_merge_into_wider_swaps_when_receiver_is_narrower
# execution 2:
Summary [   0.593s] 40 tests run: 39 passed, 1 failed, 21 skipped
PASS before... suanpan accumulator::tests::metered::accumulator_operand_rows_cost_the_operand
PASS suanpan accumulator::tests::metered::merge_tie_reads_the_operand
PASS suanpan accumulator::tests::differential::width_ordered_merges_match_the_oracle
FAIL [   0.006s] (30/40) suanpan::zz_witness_1 suanpan_tests_16_merge_into_wider_swaps_when_receiver_is_narrower
MEASURED suanpan_tests_16: touches=128
assertion `left == right` failed: merge_into_wider read the wide side: 128 touches
  left: 128
 right: 4
```

Explanation:

Ran (executions 1 and 2). Today the narrow-receiver direction costs 4 touches (the swap is live). With the swap at accumulator.rs:1170-1172 deleted, all 39 committed suanpan tests in the metered, differential and merge selection stay green (including the three the finding names) while the narrow-receiver merge reads 128 touches. The 21 skipped suanpan tests are the claims-roster and scanner tests, which do not exercise the accumulator; I did not run them.

## meter-registry-tier2-7

Outcome: inconclusive

Witness agent: 2

Test code:

```rust
(not constructed)
```

Command:

```text
awk 'NR>=4504 && NR<=4519' crates/before/tests/meter.rs
```

Output:

```text
  4506	        let v = Shape::WeightComb.packed1(n).version();
  4507	        let bytes = v.encode().len() as u64;
  4508	        // Σ stored bases: the spine's 32n − 1 unit leaves plus the
  4509	        // block's n twos.
  4510	        let expected = UBig::from((34 * n - 1) as u64);
  4511	        assert_eq!(
  4512	            v.min_ticks(),
  4513	            expected
  4514	                .to_string()
  4515	                .parse::<before::Ticks>()
  4516	                .expect("the closed form parses"),
  4517	            "the family's base sum disagrees with min_ticks: the generator \
  4518	             does not build the tree this band reasons about"
  4519	        );
```

Explanation:

Not constructed within the cap, because reading shows the finding's specific construction cannot pass as written: the weight-comb band's semantic leg at tests/meter.rs:4511-4519 pins min_ticks to the closed form 34n − 1, and dropping the parked-unit spine changes the base sum, so `skyline_rank_weight_comb_is_flat_per_unit` would fail on the semantic leg, not stay green. A degradation that preserves the base sum while removing the never-written gap would need a redesigned generator, which I did not attempt. The textual half (registry.rs:770-771 and 784-785, tests/meter.rs:4551 cite uncommitted probe builds; no committed known-bad kernel exists for these three bands) I verified by reading those lines.

## board-families-floors-judge-20

Outcome: demonstrated

Witness agent: 2

Test code:

```rust
// crates/before/src/meter/board/tests.rs (appended)
/// witness:2 scratch: a probe sample with every floor NA and the given
/// denominators and readings.
fn zz_witness_sample(
    denom: usize,
    exp_denom: usize,
    floors: super::Floors,
    readings: super::ByCurrency<Option<u64>>,
) -> super::measure::Sample {
    super::measure::Sample {
        denom_bytes: denom,
        exp_denom_bytes: exp_denom,
        limb_denom: denom as u64,
        text_row: false,
        floors,
        fold_arity: None,
        fold_search_bits: 0,
        heap_model: None,
        declared_heap: None,
        declared_limb: None,
        readings,
    }
}

/// witness:2 (board-families-floors-judge-20): a cell whose two samples
/// share one exponent denominator (a generator that stopped scaling) must
/// not read GREEN with its exponent legs silently unjudged; the same
/// readings over a scaling denominator are the control.
#[test]
fn zz_witness_board_judge_20_unscaled_denominator_is_not_a_silent_green() {
    use super::floors::na;
    use super::judge::evaluate;
    use super::{ByCurrency, Floors};
    const PROBE_NA: &str = "probe: the exponent legs alone are under test";
    let floors = || Floors {
        heap: na(PROBE_NA),
        segments: na(PROBE_NA),
        limb: na(PROBE_NA),
        scan: na(PROBE_NA),
        touch: na(PROBE_NA),
    };
    // A blatantly superlinear scan reading across the two samples.
    let readings = |scan: u64| ByCurrency {
        heap: Some(0),
        segments: Some(0),
        limb: None,
        scan: Some(scan),
        touch: None,
    };
    let control = evaluate(
        "zz_probe",
        "scaling",
        zz_witness_sample(1_000, 1_000, floors(), readings(8_000)),
        zz_witness_sample(2_000, 2_000, floors(), readings(64_000)),
    );
    eprintln!(
        "MEASURED board_judge_20 control: red={:?} scan_exp={:?} exp_judged={}",
        control.red, control.scores.scan.exp, control.scores.scan.exp_judged
    );
    assert!(
        !control.red.is_empty(),
        "the control's cubic scan trend must read red"
    );
    let stalled = evaluate(
        "zz_probe",
        "stalled",
        zz_witness_sample(1_000, 1_000, floors(), readings(8_000)),
        zz_witness_sample(2_000, 1_000, floors(), readings(64_000)),
    );
    eprintln!(
        "MEASURED board_judge_20 stalled: red={:?} scan_exp={:?} exp_judged={}",
        stalled.red, stalled.scores.scan.exp, stalled.scores.scan.exp_judged
    );
    assert!(
        !stalled.red.is_empty() || stalled.scores.scan.exp_judged,
        "a stalled exponent denominator leaves the cell GREEN with the exponent leg unjudged"
    );
}
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --no-fail-fast -E 'binary(zz_witness_1) | test(/zz_witness/) | test(rank_pair_mismatch_envelope)'
```

Output:

```text
MEASURED board_judge_20 control: red=["scan exponent"] scan_exp=Some(2.9999999999999996) exp_judged=true
MEASURED board_judge_20 stalled: red=[] scan_exp=Some(0.0) exp_judged=false
thread 'meter::board::tests::zz_witness_board_judge_20_unscaled_denominator_is_not_a_silent_green' panicked at crates/before/src/meter/board/tests.rs:1358:5
```

Explanation:

Ran (execution 1) through the board's own `evaluate` rather than the amp_board_smoke capture loop (the smoke needs a board run, which I was told not to launch). An 8x scan reading across a 2x denominator reads red ("scan exponent", trend 3.0) when the exponent denominator scales, and red=[] with exp_judged=false when the second sample's exp_denom equals the first: the guard at judge.rs:120-124 silently unjudges the leg and the cell is GREEN, with nothing pinning which cells may be unjudged.

## board-ops-render-10

Outcome: demonstrated

Witness agent: 2

Test code:

```rust
// crates/before/src/meter/board/tests.rs (appended; uses the zz_witness_sample helper quoted under board-families-floors-judge-20)
/// witness:2 (board-ops-render-10): the placement rows' committed touch
/// declaration (`walk_floors(n, na(NA_TOUCH_PLACEMENT))`) accepts a dead
/// touch meter (reading 0) on a span_place-shaped cell, while the pair-fold
/// floor the placement kernel's premise supports would trip it.
#[test]
fn zz_witness_board_ops_10_placement_touch_na_accepts_a_dead_meter() {
    use super::floors::{na, touch_pair_fold, walk_floors, NA_TOUCH_PLACEMENT};
    use super::judge::{evaluate, TOUCH_FLOOR_TRIP};
    use super::ByCurrency;
    let v = version_of(&crate::meter::registry::Shape::Staircase.packed1(64));
    let w = version_of(&crate::meter::registry::Shape::Dense.packed1(64));
    let v2 = version_of(&crate::meter::registry::Shape::Staircase.packed1(128));
    let w2 = version_of(&crate::meter::registry::Shape::Dense.packed1(128));
    let n1 = v.encode().len() + w.encode().len() + v.encode().len();
    let n2 = v2.encode().len() + w2.encode().len() + v2.encode().len();
    let readings = |scan: u64| ByCurrency {
        heap: Some(0),
        segments: Some(0),
        limb: Some(0),
        scan: Some(scan),
        touch: Some(0),
    };
    let committed = evaluate(
        "span_place",
        "zz_staircase",
        zz_witness_sample(n1, n1, walk_floors(n1, na(NA_TOUCH_PLACEMENT)), readings(8 * n1 as u64)),
        zz_witness_sample(n2, n2, walk_floors(n2, na(NA_TOUCH_PLACEMENT)), readings(8 * n2 as u64)),
    );
    eprintln!("MEASURED board_ops_10 committed declaration: red={:?}", committed.red);
    let floored = evaluate(
        "span_place",
        "zz_staircase",
        zz_witness_sample(n1, n1, walk_floors(n1, touch_pair_fold(&v, &w)), readings(8 * n1 as u64)),
        zz_witness_sample(n2, n2, walk_floors(n2, touch_pair_fold(&v2, &w2)), readings(8 * n2 as u64)),
    );
    eprintln!(
        "MEASURED board_ops_10 pair-fold floor: red={:?} floor={:?}",
        floored.red,
        touch_pair_fold(&v, &w)
    );
    assert_eq!(
        floored.red,
        vec![TOUCH_FLOOR_TRIP],
        "the pair-fold floor trips on a dead touch meter"
    );
    assert!(
        !committed.red.is_empty(),
        "the committed placement declaration reads GREEN on a dead touch meter"
    );
}

// crates/before/tests/zz_witness_1.rs (live kernel half)
/// board-ops-render-10 (live kernel half): the fused placement walk on a
/// staircase pair folds deltas into accumulators, so the touch meter reads
/// nonzero; a not-applicable declaration says no fold count is forced.
#[test]
fn board_ops_render_10_placement_walk_touches_are_nonzero() {
    let v = Shape::Staircase.packed1(64).version();
    let w = Shape::Dense.packed1(64).version();
    let span = v.span(&w);
    let probe = Version::decode(&v.encode()[..]).unwrap();
    suanpan::touch_meter::reset();
    let verdict = std::hint::black_box(span.place(&probe));
    let touches = suanpan::touch_meter::touches();
    eprintln!("MEASURED board_ops_render_10: verdict={verdict:?} touches={touches}");
    assert_eq!(touches, 0, "the placement walk touched {touches} digits: a floor is forced");
}
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --no-fail-fast -E 'binary(zz_witness_1) | test(/zz_witness/) | test(rank_pair_mismatch_envelope)'
```

Output:

```text
MEASURED board_ops_10 committed declaration: red=[]
MEASURED board_ops_10 pair-fold floor: red=["touch floor: counter reads below floor: the meter is not watching this work"] floor=Floor { min: 64, why: "deterministic-liveness: the fused sweep lands each nonzero stored delta of either operand in the one running difference accumulator, at least one digit touch per boundary the larger nonzero-delta count's operand steps (a zero delta folds nothing, and a sign re-read where nothing folded is not forced); digit state moving to an unmetered representation lowers this floor deliberately" }
thread 'meter::board::tests::zz_witness_board_ops_10_placement_touch_na_accepts_a_dead_meter' panicked at crates/before/src/meter/board/tests.rs:1409:5

MEASURED board_ops_render_10: verdict=At(End) touches=327
assertion `left == right` failed: the placement walk touched 327 digits: a floor is forced
  left: 327
 right: 0
```

Explanation:

Ran (execution 1). Through `evaluate`, a span_place-shaped cell with the committed `walk_floors(n, na(NA_TOUCH_PLACEMENT))` and touch reading 0 is red=[] (a dead touch meter passes), while the same samples under `touch_pair_fold` read [TOUCH_FLOOR_TRIP] with a floor of 64 on the staircase(64)/dense(64) pair. On the live kernel, `span.place(&probe)` on that pair reads 327 touches, so the walk does fold into accumulators and a positive floor is derivable, as the finding argues from place.rs:26-29.

## surface-roster-6

Outcome: refuted

Witness agent: 2

Test code:

```rust
// crates/before/src/span/wire.rs (execution 4):
     pub fn encode_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
-        self.lo.encode_to(writer)?;
-        self.hi.encode_to(writer)
+        // witness:2 MUTANT (surface-roster-6): endpoint order swapped.
+        self.hi.encode_to(writer)?;
+        self.lo.encode_to(writer)
     }
```

Command:

```text
cargo nextest run -p before --all-features --no-fail-fast -E 'test(/skyline::place::tests/) | test(coverage_is_exact_on_the_two_party_grid) | test(/causally::tests/) | binary(verdict_matrix) | test(/laws::tests/) | test(/skyline::watermark::tests/) | test(/span_borsh/) | test(/serde_impls::tests/) | test(/span::tests/) | binary(fuzz_seeds) | test(exhaustive_small) | test(/skyline::fill::tests/) | test(grow_matches_brute_force) | test(grow_minimal)'
```

Output:

```text
FAIL [   0.009s] (  8/145) before borsh_impls::tests::span_borsh_composes_and_keeps_its_genres
thread 'borsh_impls::tests::span_borsh_composes_and_keeps_its_genres' panicked at crates/before/src/borsh_impls/tests.rs:1011:54
FAIL [   0.031s] ( 37/145) before borsh_impls::tests::span_borsh_roundtrips
thread 'borsh_impls::tests::span_borsh_roundtrips' panicked at crates/before/src/borsh_impls/tests.rs:1058:1:
Test failed: assertion failed: `(left == right)` ... : borsh is the one wire form, unframed at crates/before/src/borsh_impls/tests.rs:1069.
(serde_impls::tests, fuzz_seeds, verdict_matrix all PASS)
```

Explanation:

Ran (execution 4). Swapping the two writes in Span::encode_to is killed under --all-features by two borsh tests: `span_borsh_composes_and_keeps_its_genres` (borsh_impls/tests.rs:998-1056 builds `Span::new(&older, &newer)` with distinct endpoints, serializes through `BorshSerialize::serialize`, which borsh_impls.rs:262-266 routes to `encode_to`, and round-trips at line 1011) and the `span_borsh_roundtrips` proptest (line 1069 compares `borsh::to_vec(&span)` to `span.encode()`). So the claim "swapping the two writes passes the whole gate" is false: the gate's all-features test leg would catch it. The doctest at wire.rs:62-70 does use `Span::new(&v, &v)` (read, not run) and the five roster rows do carry `pins: &[]` (surface.rs:987-992 read), so the name-resolvability half stands as a reading; only the exposure claim is refuted.

## testing-oracles-22

Outcome: demonstrated

Witness agent: 2

Test code:

```rust
// crates/before/src/oracle/version.rs (execution 4):
     #[cfg(test)]
     pub(crate) fn fill_for_test(&self, id: &Party) -> Version {
-        self.fill(id)
+        // witness:2 MUTANT (testing-oracles-22): an always-changing fill.
+        let mut filled = self.fill(id);
+        filled.tick(&Party::Leaf(true));
+        filled
     }
```

Command:

```text
cargo nextest run -p before --all-features --no-fail-fast -E 'test(/skyline::place::tests/) | test(coverage_is_exact_on_the_two_party_grid) | test(/causally::tests/) | binary(verdict_matrix) | test(/laws::tests/) | test(/skyline::watermark::tests/) | test(/span_borsh/) | test(/serde_impls::tests/) | test(/span::tests/) | binary(fuzz_seeds) | test(exhaustive_small) | test(/skyline::fill::tests/) | test(grow_matches_brute_force) | test(grow_minimal)'
```

Output:

```text
PASS [   8.214s] (142/145) before testing::exhaustive::tests::exhaustive_small
FAIL [   1.005s] (129/145) before version::tests::grow_matches_brute_force
FAIL [   0.989s] (131/145) before version::tests::grow_minimal
FAIL before version::skyline::fill::tests::worked_examples_tick_exactly
(about 25 version::skyline::fill::tests::* FAIL; some of those failures may also owe to the watermark mutant 1 that shared this batch)
```

Explanation:

Ran (execution 4). With `fill_for_test` always returning a changed tree, `exhaustive_small` passes in 8.2 s: every pair takes the `continue` at exhaustive/tests.rs:314-316, `best_inflation` and `all_inflations` never run, and no floor counts how many pairs reached the grow pin, exactly as the finding says (exhaustive_deep was not run). The mutant is not invisible crate-wide, though: `version::tests::grow_matches_brute_force` and `grow_minimal` fail (their `prop_assume!(ov.fill_for_test(&op) == ov)` at version/tests.rs:560 and 588 now rejects every case) and the fill suite's `assert_tick` helper (fill/tests.rs:75) calls `fill_for_test` directly. The pin's vacuity is demonstrated; the specific mutant would be noticed by other suites.

## testing-diff-gen-26

Outcome: demonstrated

Witness agent: 2

Test code:

```rust
// crates/before/src/testing/asymptotics.rs (appended)
/// witness:2 (testing-diff-gen-26): each fold-door floor must sit above
/// its population's own byte growth across the n = 256 -> 1,024
/// quadrupling (the linear reference the pins describe but never assert);
/// the scan readings are printed beside it.
#[cfg(feature = "scan-meter")]
#[test]
fn zz_witness_testing_diff_gen_26_fold_door_floors_sit_above_linear_reference() {
    let version_bytes = |n: usize| -> usize { stagger_versions(n).iter().map(|v| v.encode().len()).sum() };
    let notch_bytes =
        |n: usize| -> usize { stagger_notch_versions(n).iter().map(|v| v.encode().len()).sum() };
    let party_bytes = |n: usize| -> usize { stagger_parties(n).iter().map(|p| p.encode().len()).sum() };
    let clock_bytes = |n: usize| -> usize {
        stagger_parties(n)
            .into_iter()
            .zip(stagger_versions(n))
            .map(|(p, v)| crate::Clock::from_parts(p, v).encode().len())
            .sum()
    };
    let doors: [(&str, f64, usize, usize, u64, u64); 5] = [
        ("Version::join_all", 5.29, version_bytes(256), version_bytes(1024), join_all_scan_bits(256), join_all_scan_bits(1024)),
        ("Version::meet_all", 5.29, notch_bytes(256), notch_bytes(1024), meet_all_scan_bits(256), meet_all_scan_bits(1024)),
        ("Version::span_all", 5.26, version_bytes(256), version_bytes(1024), span_all_scan_bits(256), span_all_scan_bits(1024)),
        ("Party::join_all", 4.89, party_bytes(256), party_bytes(1024), party_join_all_scan_bits(256), party_join_all_scan_bits(1024)),
        ("Clock::join_all", 5.05, clock_bytes(256), clock_bytes(1024), clock_join_all_scan_bits(256), clock_join_all_scan_bits(1024)),
    ];
    let mut vacuous = Vec::new();
    for (door, min_growth, b_lo, b_hi, s_lo, s_hi) in doors {
        let byte_growth = b_hi as f64 / b_lo as f64;
        let scan_growth = s_hi as f64 / s_lo as f64;
        eprintln!(
            "MEASURED testing_diff_gen_26: {door} bytes {b_lo}->{b_hi} (x{byte_growth:.3}) scan {s_lo}->{s_hi} (x{scan_growth:.3}) floor={min_growth} midpoint={:.3}",
            (byte_growth + scan_growth) / 2.0
        );
        if byte_growth >= min_growth {
            vacuous.push(door);
        }
    }
    assert!(
        vacuous.is_empty(),
        "floors at or below their linear reference today: {vacuous:?}"
    );
}
```

Command:

```text
cargo nextest run -p before --all-features --no-fail-fast --success-output immediate -E 'test(rank_4_rank_never_larger_than_version_at_small_scale) | test(span_causally_24_k_hole_query_touches_do_not_scale_with_k) | test(crate_root_24_space_ratio_against_naive_transcriptions) | test(zz_witness_testing_diff_gen_26_fold_door_floors_sit_above_linear_reference)'
```

Output:

```text
MEASURED testing_diff_gen_26: Version::join_all bytes 63488->303104 (x4.774) scan 4906266->28537882 (x5.817) floor=5.29 midpoint=5.295
MEASURED testing_diff_gen_26: Version::meet_all bytes 63742->304126 (x4.771) scan 4907680->28543880 (x5.816) floor=5.29 midpoint=5.294
MEASURED testing_diff_gen_26: Version::span_all bytes 63488->303104 (x4.774) scan 5241754->30212634 (x5.764) floor=5.26 midpoint=5.269
MEASURED testing_diff_gen_26: Party::join_all bytes 40960->196608 (x4.800) scan 7260488->36191048 (x4.985) floor=4.89 midpoint=4.892
MEASURED testing_diff_gen_26: Clock::join_all bytes 104448->499712 (x4.784) scan 12166758->64770022 (x5.324) floor=5.05 midpoint=5.054
PASS [   2.006s] (3/4) before testing::asymptotics::zz_witness_testing_diff_gen_26_fold_door_floors_sit_above_linear_reference
```

Explanation:

Ran (executions 1 and 3). The recomputed linear references are bytes x4.771-4.800 and the scan readings x5.817/5.816/5.764/4.985/5.324, so the five committed MIN_GROWTH constants (5.29, 5.29, 5.26, 4.89, 5.05) are exactly the transcribed midpoints (5.295, 5.294, 5.269, 4.892, 5.054) the finding names, and today no floor sits at or below its byte reference (my added assertion passes). The gap is the absence of that assertion from the committed tree: `door_scan_bits` (asymptotics.rs:117-123) only prints input_bytes and `assert_log_factor_alive` (230-239) never sees a byte ratio, verified by reading.

## envelopes-a-14

Outcome: demonstrated

Witness agent: 2

Test code:

```rust
/// envelopes-a-14: the rank_pair_mismatch row prices cmp, checked_sub and
/// `+` jointly; its ceiling should leave less headroom than a linear
/// class-mismatch compare would cost (one limb op per 64 exponent bits).
#[test]
fn envelopes_a_14_rank_pair_row_headroom_excludes_a_linear_cmp() {
    const RANK_PAIR_DEPTH: usize = 500_000;
    const CEILING: u64 = 87_910;
    let a = Shape::Dense.packed1(RANK_PAIR_DEPTH).version().rank();
    let b = Version::try_from(3u64).unwrap().rank();
    meter::reset_limb_ops();
    let ord = std::hint::black_box(a.cmp(&b));
    let cmp_only = meter::limb_ops();
    meter::reset_limb_ops();
    let diff = std::hint::black_box(b.checked_sub(&a));
    let sum = std::hint::black_box(&a + &b);
    let rest = meter::limb_ops();
    let basis = cmp_only + rest;
    let linear_cmp = (RANK_PAIR_DEPTH / 64) as u64;
    eprintln!(
        "MEASURED envelopes_a_14: ord={ord:?} diff_some={} cmp_only_limb_ops={cmp_only} checked_sub_plus_add_limb_ops={rest} basis={basis} ceiling={CEILING} headroom={} linear_cmp_limbs={linear_cmp}",
        diff.is_some(),
        CEILING.saturating_sub(basis)
    );
    drop(sum);
    assert!(
        CEILING.saturating_sub(basis) < linear_cmp,
        "headroom {} admits a linear class-mismatch cmp of {linear_cmp} limb ops",
        CEILING.saturating_sub(basis)
    );
}
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --no-fail-fast -E 'binary(zz_witness_1) | test(/zz_witness/) | test(rank_pair_mismatch_envelope)'
```

Output:

```text
MEASURED envelopes_a_14: ord=Less diff_some=true cmp_only_limb_ops=0 checked_sub_plus_add_limb_ops=70328 basis=70328 ceiling=87910 headroom=17582 linear_cmp_limbs=7812
thread 'envelopes_a_14_rank_pair_row_headroom_excludes_a_linear_cmp' panicked at crates/before/tests/zz_witness_1.rs:278:5
LEAK [   0.483s] (14/16) before::meter rank_pair_mismatch_envelope   (the committed row itself passes; nextest flagged a leaked child handle only)
```

Explanation:

Ran (execution 1), by arithmetic on the committed row rather than the finding's Ord-replacement mutant (Num has no shift or Ord API to write a metered value-identical cmp cheaply). On the row's own operands `Rank::cmp` reads 0 limb ops (the class-first O(1) leg), `checked_sub` plus `+` read 70,328, matching the row's basis; the ceiling 87,910 leaves 17,582 limb ops of headroom, more than twice the 7,812 a limb-linear class-mismatch compare over the 500,000-bit exponent would cost. So the joint row cannot exclude a linear cmp and the O(1) claim at rank.rs:879 is priced by no committed instrument, as the finding says.

## tests-other-6

Outcome: demonstrated

Witness agent: 2

Test code:

```rust
(none: mechanical grep, as the finding's construction specifies)
```

Command:

```text
grep -n "answer_embedded\|wide_base\|wide ladder\|skeleton" crates/before/src/meter/registry.rs crates/before/tests/meter.rs
```

Output:

```text
crates/before/src/meter/registry.rs:615:    /// shared upper skeleton is both-present in every operand pair, so the
crates/before/tests/meter.rs:5677:mod answer_embedded_product {
crates/before/tests/meter.rs:5826:// (`ledger_wide_arming`, `answer_embedded_product`); the probes here
```

Explanation:

Verified mechanically by grep (not a test run). The only hits for the WT/WL/skeleton vocabulary in registry.rs and tests/meter.rs are the unrelated `answer_embedded_product` band (a different family) and a `skeleton` word in the weave family's prose; no FamilyId, Shape, or `_reads_superlinear` kernel corresponds to tests/answer_embedded.rs's WT(n, w), WL(n, w) grid or tests/fold_skeleton.rs's deep skeleton, so their mixed-second-difference criteria have no committed known-bad demonstration and sit outside the registry's roster, as claimed.

## tests-other-17

Outcome: demonstrated

Witness agent: 2

Test code:

```rust
/// tests-other-17: `Clock::forks`'s rustdoc says the iterator yields `n`
/// children; at `u64::MAX` it should therefore report `u64::MAX`.
#[test]
fn tests_other_17_clock_forks_yields_n_children_at_saturation() {
    let mut c = Clock::seed();
    let len = c.forks(u64::MAX).len() as u64;
    eprintln!("MEASURED tests_other_17: forks(u64::MAX).len()={len}");
    assert_eq!(len, u64::MAX, "the doc's `n` children; observed {len}");
}
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --no-fail-fast -E 'binary(zz_witness_1) | test(/zz_witness/) | test(rank_pair_mismatch_envelope)'
```

Output:

```text
MEASURED tests_other_17: forks(u64::MAX).len()=18446744073709551614
assertion `left == right` failed: the doc's `n` children; observed 18446744073709551614
  left: 18446744073709551614
 right: 18446744073709551615
```

Explanation:

Ran (execution 1). `Clock::seed().forks(u64::MAX).len()` is u64::MAX − 1, the saturated reading forks_max.rs:49-50 pins, while clock.rs:166 says "The iterator yields `n` children" and clock.rs:192 names the parameter `k`; the saturation is stated only in the private comment at party/forks.rs:111-112 (read). The 'both profiles' wording and the dissolved cfg-split I assessed by reading only.

## tests-other-26

Outcome: demonstrated

Witness agent: 2

Test code:

```rust
/// tests-other-26: after `clock.sync(&mut sibling)` the seed comment calls
/// the sibling's version "concurrent to the clock's own history".
#[test]
fn tests_other_26_synced_sibling_version_is_concurrent() {
    let mut clock = Clock::seed();
    let mut sibling = clock.fork();
    clock.tick();
    sibling.tick();
    clock.sync(&mut sibling).expect("forked clocks are disjoint");
    eprintln!(
        "MEASURED tests_other_26: clock={} sibling_version={} equal={}",
        clock,
        sibling.version(),
        clock.version() == sibling.version()
    );
    assert!(
        clock.version().concurrent(sibling.version()),
        "the synced sibling's version is not concurrent to the clock's"
    );
}
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --no-fail-fast -E 'binary(zz_witness_1) | test(/zz_witness/) | test(rank_pair_mismatch_envelope)'
```

Output:

```text
MEASURED tests_other_26: clock=((1, 0), 1) sibling_version=1 equal=true
thread 'tests_other_26_synced_sibling_version_is_concurrent' panicked at crates/before/tests/zz_witness_1.rs:178:5
```

Explanation:

Ran (execution 1), replicating fuzz_seed_set.rs:269-275 exactly. After `sync` both clocks hold the single version `1` (clock prints `((1, 0), 1)`, sibling version `1`, equal), so the comments at fuzz_seed_set.rs:290-291 ("concurrent to the clock's own history") and 335-336 describe a value that does not exist: the clock_then_msg payload is the clock's own version and laws_family's first two versions are one value.

## benches-examples-25

Outcome: demonstrated

Witness agent: 2

Test code:

```rust
(none: benches are forbidden; verified by reading and read-only git)
```

Command:

```text
git log -1 --format='%h %ad %s' --date=short 786fd8e4 ; git log -1 --format='%h %ad %s' --date=short 7139904b ; grep -n partial_cmp crates/before/benches/party.rs crates/before/benches/version.rs ; sed -n 13p crates/before/results/benchmarks/speedup.md ; awk 'NR>=117 && NR<=120' crates/before/scripts/plot_benchmarks.py ; awk 'NR>=72 && NR<=73' crates/before/results/benchmarks/README.md
```

Output:

```text
786fd8e4 2026-06-02 before: add benchmark comparison plots (impl vs oracle)
7139904b 2026-06-02 More docs; remove PartialOrd for Party
(grep partial_cmp: hits only in crates/before/benches/version.rs:153,158,159,185,192,277; none in benches/party.rs)
| Version | merge  ( | , least-upper-bound) | 32768 | 2.12 ms | 2.10 ms | 1.0× |
   117	            cmp_panel("party/partial_cmp", "partial_cmp: ancestor",
   118	                      "before/ancestor", "oracle/ancestor"),
   119	            cmp_panel("party/partial_cmp", "partial_cmp: equal",
   120	                      "before/equal", "oracle/equal"),
All curves in the committed figures were collected in a single run on
2026-06-02.
```

Explanation:

Assessed by reading and read-only git, not by running benches (forbidden). Verified: the figures date to 2026-06-02 (README:72-73); 786fd8e4 and 7139904b are both 2026-06-02 commits and the latter removed Party's PartialOrd; benches/party.rs has no partial_cmp group today while plot_benchmarks.py:117-120 and speedup.md:10-11 still name party/partial_cmp; speedup.md:13 carries the unescaped `|` in "merge  ( | , least-upper-bound)"; and nothing outside the directory links to results/benchmarks except plot_benchmarks.py's own default OUT_DIR. The ~31x attribution and the version-join speedup inversion I did not measure.

## fuzz-guests-pins-29

Outcome: demonstrated

Witness agent: 2

Test code:

```rust
/// fuzz-guests-pins-29: the rank pins observe only `0 < r < 1` and
/// `r == r.clone()`. Decode the correct synthesized rank (bits 65 and exp)
/// and the seam-dropped value (bit 65 only); the pin's observations should
/// separate them if they are worth anything.
#[test]
fn fuzz_guests_pins_29_pin_observations_separate_seam_drop() {
    fn synth(exp: u64, seam: bool) -> Vec<u8> {
        // Mirror of wasm32-pins/guest/src/lib.rs `synth_rank`, with the
        // group count set by the last set bit so the stream stays
        // canonical for both variants.
        let last = if seam { exp } else { 65 };
        let groups = last.div_ceil(8);
        let total_bits = 9 * groups + 2;
        let total_bytes = usize::try_from(total_bits.div_ceil(8)).unwrap();
        const TILE: [u8; 9] = [0x80, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01, 0x00];
        let mut bytes = vec![0u8; total_bytes];
        for (b, byte) in bytes.iter_mut().enumerate() {
            let hi = if b == 0 { 0 } else { TILE[(b - 1) % 9] };
            *byte = (hi << 7) | (TILE[b % 9] >> 1);
        }
        let close = 1 + 9 * groups;
        let close_byte = usize::try_from(close / 8).unwrap();
        let offset = (close % 8) as u32;
        bytes[close_byte] = if offset == 0 {
            0
        } else {
            bytes[close_byte] & (0xFFu8 << (8 - offset))
        };
        for byte in bytes.iter_mut().skip(close_byte + 1) {
            *byte = 0;
        }
        let set: &[u64] = if seam { &[65, exp] } else { &[65] };
        for &e in set {
            let p = 1 + 9 * ((e - 1) / 8) + 1 + ((e - 1) % 8);
            bytes[usize::try_from(p / 8).unwrap()] |= 0x80 >> (p % 8);
        }
        bytes
    }
    let pin = |r: &Rank| -> i64 {
        if *r <= Rank::ZERO {
            return -2;
        }
        let one = Version::try_from(1u64).unwrap().rank();
        if *r >= one {
            return -4;
        }
        if *r != r.clone() {
            return -5;
        }
        0
    };
    for exp in [128u64, 1024, 4096] {
        let full = Rank::decode(&synth(exp, true)[..]).expect("full stream decodes");
        let dropped = Rank::decode(&synth(exp, false)[..]).expect("seam-dropped stream decodes");
        eprintln!(
            "MEASURED fuzz_guests_pins_29: exp={exp} full!=dropped={} pin(full)={} pin(dropped)={}",
            full != dropped,
            pin(&full),
            pin(&dropped)
        );
        assert_ne!(full, dropped, "the two values differ");
        assert_eq!(pin(&full), 0);
        assert_ne!(
            pin(&dropped),
            0,
            "the pin's observations accept the seam-dropped value at exp={exp}"
        );
    }
}
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --no-fail-fast -E 'binary(zz_witness_1) | test(/zz_witness/) | test(rank_pair_mismatch_envelope)'
```

Output:

```text
MEASURED fuzz_guests_pins_29: exp=128 full!=dropped=true pin(full)=0 pin(dropped)=0
assertion `left != right` failed: the pin's observations accept the seam-dropped value at exp=128
  left: 0
 right: 0
```

Explanation:

Ran (execution 1), on the host rather than wasm32 (`just wasm32-pins` is outside my permitted commands) using a faithful mirror of the guest's `synth_rank` layout (guest lib.rs:133-175) and the exact observation sequence of `pin_rank_decode` (lib.rs:196-208). The true value 2^-65 + 2^-128 and the seam-dropped value 2^-65 decode to different ranks, and the pin's three observations return 0 for both, so a decoder that drops the seam bit passes the pin exactly as the finding says. The loop stopped at exp = 128 because the first assertion fired; the arithmetic is identical at larger exp.

## fuzz-guests-pins-38

Outcome: demonstrated

Witness agent: 2

Test code:

```rust
(none: `just gate` is forbidden; verified by reading the recipes)
```

Command:

```text
grep -n "^gate:\|^gate-lints\|^gate-streams\|fuzz-build\|^ci:\|^all:" justfile ; awk 'NR>=366 && NR<=368' justfile ; sed -n 6p .github/workflows/ci.yml
```

Output:

```text
400:gate: gate-lints gate-streams
403:gate-lints: fmt-check doclint testdoc workflowlint manifestlint digestshare mutants-list readme-check
416:gate-streams:
468:    start_stream fuzz         10 fuzz-build
1000:ci: fmt-check doclint testdoc workflowlint digestshare readme-check fuelscape-claims mutants-list clippy clippy-default features wasm-check docs docs-internal test-all citecheck doctest bench-build fuzz-build fuelscape-verify viz
1003:all: ci (fuzz fuzz_smoke_secs) lean eventdag muxprobe bench-judge bench-judge-tripwire
   366	fuzz-build:
   367	    cargo fmt --check
   368	    {{ justfile_directory() }}/tools/memwatch cargo +{{ nightly_toolchain }} fuzz build --target {{ host_triple }}
# cheap-first. It builds the fuzz targets but does not run the smoke (`just all`
```

Explanation:

Verified by reading (mechanical grep), not by running the gate. `gate-streams` starts only `fuzz-build` (justfile:468), `ci` lists `fuzz-build` and not `fuzz` (justfile:1000), and only `all` runs `(fuzz fuzz_smoke_secs)` (justfile:1003); ci.yml:6 states the same. The fuzz targets' own agreement logic (e.g. fuzz_decode_differential.rs:168 `(_, c, f) if c == f => true`, read) therefore executes in no gate or CI run, as claimed. The predicate-flip construction itself was not run.

## fuzzfit-bands-10

Outcome: demonstrated

Witness agent: 2

Test code:

```rust
# fuzzfit_bands_10.py (scratchpad), arithmetic over the committed constants
import math

def check(name, slope, intercept, width_above, max_denom, exps):
    ceiling_at_max = intercept + slope * math.log10(max_denom) + width_above + 0.2
    print(f"{name}: pooled slope={slope} shape threshold (slope+0.3)={slope + 0.3:.6f} ceiling(log10 fuel) at d={max_denom}: {ceiling_at_max:.3f}")
    for e in exps:
        log_f128 = intercept + slope * math.log10(128)
        log_f_max = log_f128 + e * math.log10(max_denom / 128)
        excess = e - slope
        passes_ceiling = log_f_max <= ceiling_at_max
        passes_shape = excess < 0.3
        print(f"  exponent {e:.2f}: log10 fuel at d={max_denom} = {log_f_max:.3f} ({'under' if passes_ceiling else 'OVER'} ceiling), within-case excess {excess:.3f} ({'under' if passes_shape else 'OVER'} 0.3) -> {'PASSES BOTH LEGS' if passes_ceiling and passes_shape else 'caught'}")

check("ff_version_decode", 1.175460, 1.706616, 0.330785, 8768, [1.0, 1.2, 1.4, 1.45, 1.5, 1.55])
check("ff_party_join", 1.251540, 1.607059, 0.344635, 8622, [1.0, 1.3, 1.5, 1.55, 1.6])
```

Command:

```text
python3 fuzzfit_bands_10.py
```

Output:

```text
ff_version_decode: pooled slope=1.17546 shape threshold (slope+0.3)=1.475460 ceiling(log10 fuel) at d=8768: 6.872
  exponent 1.40: log10 fuel at d=8768 = 6.754 (under ceiling), within-case excess 0.225 (under 0.3) -> PASSES BOTH LEGS
  exponent 1.45: log10 fuel at d=8768 = 6.845 (under ceiling), within-case excess 0.275 (under 0.3) -> PASSES BOTH LEGS
  exponent 1.50: log10 fuel at d=8768 = 6.937 (OVER ceiling), within-case excess 0.325 (OVER 0.3) -> caught
ff_party_join: pooled slope=1.25154 shape threshold (slope+0.3)=1.551540 ceiling(log10 fuel) at d=8622: 7.077
  exponent 1.50: log10 fuel at d=8622 = 6.987 (under ceiling), within-case excess 0.248 (under 0.3) -> PASSES BOTH LEGS
  exponent 1.55: log10 fuel at d=8622 = 7.078 (OVER ceiling), within-case excess 0.298 (under 0.3) -> caught
```

Explanation:

Verified arithmetically (Python over constants I read from fuzzfit/harness/src/bands.rs:749-758 and 605-614, ENFORCE_MARGIN 0.2 at bands.rs:173, SLOPE_ALLOWANCE 0.3 and `local_slope_excess`'s `- band.slope` at curve.rs:71 and 83-100). A hypothetical d^1.45 mechanism on the ff_version_decode band line at d = 128 stays under both the ceiling leg (6.845 < 6.872 at max_denom) and the within-case shape leg (excess 0.275 < 0.3); ff_party_join's shape threshold is 1.5515 and a d^1.5 mechanism passes both legs. The finding's numbers reproduce exactly; the shape leg's baseline is the pooled slope, which bands.rs's own prose describes as mixture-tilted. Not run through the fuzzfit harness itself.

## fuelscape-pipeline-1

Outcome: demonstrated

Witness agent: 2

Test code:

```rust
(none executed: before-fuelscape is a detached workspace outside my -p before -p suanpan build scope; the match arm is read as a mechanical check)
```

Command:

```text
awk 'NR>=167 && NR<=180' crates/before-fuelscape/src/families.rs ; sed -n 414p crates/before/src/meter/board/worst.rs ; awk 'NR>=407 && NR<=409' crates/before-fuelscape/src/ops.rs
```

Output:

```text
   167	        [Operand::Version, Operand::Party] => {
   168	            out.extend(ramp("dense × scattered_id", max_bytes, |t| {
   169	                Some(vec![
   170	                    version_bytes(&Shape::Dense.packed1(t)),
   171	                    party_bytes(&Shape::ScatteredId.packed1(t)),
   172	                ])
   173	            }));
   174	            out.extend(ramp("hugeleaf × id_spine", max_bytes, |t| {
   175	                Some(vec![
   176	                    version_bytes(&Shape::Hugeleaf.packed1(8 * t)),
   177	                    party_bytes(&Shape::IdSpine.packed_flagged(t, false)),
   178	                ])
   179	            }));
   180	        }
   414	    ("default", "version_tick", ["ascend-cliff", "dominated-undercut", "hugeleaf", "mirror-narrow"]),
   407	    OpSpec {
   408	        name: "version_tick",
   409	        inputs: Inputs::Packed(&[Operand::Version, Operand::Party]),
```

Explanation:

Verified by reading the exhaustive `match operands` in `overlay_inputs` (families.rs:98-116, 167-180): for version_tick's signature `Inputs::Packed(&[Operand::Version, Operand::Party])` (ops.rs:409) the overlay is exactly the two ramps "dense × scattered_id" and "hugeleaf × id_spine", keyed by operand signature, while WORST_RANKINGS at worst.rs:414 names ascend-cliff, dominated-undercut, hugeleaf and mirror-narrow as that operation's worst families; the overlay is a signature-keyed hand list, not the committed frontier lib.rs:13-14 describes. I did not execute `overlay_inputs` (detached crate, separate cold build).

## fuelscape-render-27

Outcome: demonstrated

Witness agent: 2

Test code:

```rust
// fuelscape_stub.js (scratchpad): a stub DOM with <div class="docblock"><code>k</code></div>,
// <div class="docblock"><code>Vec<u8></code></div>, <div class="docblock"><code>W</code></div>, no .fuelscape
// element, document.readyState = "complete"; then `new Function(fs.readFileSync(fuelscape.js))()`.
// Key stub pieces:
globalThis.window = {};
globalThis.document = {
  readyState: "complete",
  querySelectorAll: sel => doc.querySelectorAll(sel),
  createElement: tag => new Node(tag),
  createElementNS: (_ns, tag) => new Node(tag),
  createTextNode: t => { const n = new Node("#text"); n._text = t; return n; },
  addEventListener() {},
};
console.log("before:", doc.outer());
const src = fs.readFileSync(path, "utf8");
new Function(src)();
console.log("after: ", doc.outer());
console.log("fuelscape elements present:", doc.querySelectorAll(".fuelscape").length);
console.log("lone `k` <code> replaced:", code.replacedWith !== null, "->", code.replacedWith && code.replacedWith.outer());
// (full stub in the scratchpad copy fuelscape_stub.js)
```

Command:

```text
node fuelscape_stub.js crates/before/docs/fuelscape.js
```

Output:

```text
before: <#document><div class="docblock"><code>k</code></div><div class="docblock"><code>Vec<u8></code></div><div class="docblock"><code>W</code></div></#document>
after:  <#document><div class="docblock"><span class="fs-math"><i>k</i></span></div><div class="docblock"><code>Vec<u8></code></div><div class="docblock"><code>W</code></div></#document>
fuelscape elements present: 0
lone `k` <code> replaced: true -> <span class="fs-math"><i>k</i></span>
`Vec<u8>` <code> replaced: false
`W` <code> replaced: false
```

Explanation:

Ran under node with a stub DOM. With zero `.fuelscape` elements on the page, loading docs/fuelscape.js calls `Fuelscape.hydrate()` at load (fuelscape.js:1575-1578), which calls `typesetDocMath(scope)` unconditionally (line 1545), and a lone lowercase `<code>k</code>` inside a `.docblock` is replaced by `<span class="fs-math"><i>k</i></span>` (MATH_SHAPED at line 1468 admits `[a-z]`); `Vec<u8>` and the uppercase `W` are left alone. So on every workspace crate's rustdoc page the script does act outside `.fuelscape` elements, contrary to justfile:253.

## gate-legs-6

Outcome: demonstrated

Witness agent: 2

Test code:

```rust
(none: `just gate`, `just ci`, `just all` are forbidden; verified by reading)
```

Command:

```text
grep -n "cargo mutants" justfile ; grep -n "^gate:\|^gate-lints\|^ci:\|^all:" justfile
```

Output:

```text
296:# over `cargo mutants --list` names. Nothing else holds those regexes to
324:    cargo mutants --list --colors=never --no-config --workspace > target/mutants-raw.txt
325:    cargo mutants --list --colors=never --workspace > target/mutants-filtered.txt
326:    ./tools/mutantcheck --raw target/mutants-raw.txt --filtered target/mutants-filtered.txt --config .cargo/mutants.toml --expected tools/mutantcheck-expected.json --tool-version "$(cargo mutants --version)"
400:gate: gate-lints gate-streams
403:gate-lints: fmt-check doclint testdoc workflowlint manifestlint digestshare mutants-list readme-check
1000:ci: fmt-check doclint testdoc workflowlint digestshare readme-check fuelscape-claims mutants-list clippy clippy-default features wasm-check docs docs-internal test-all citecheck doctest bench-build fuzz-build fuelscape-verify viz
1003:all: ci (fuzz fuzz_smoke_secs) lean eventdag muxprobe bench-judge bench-judge-tripwire
```

Explanation:

Verified by reading (mechanical grep). The only `cargo mutants` invocations in the justfile are the two `--list` runs inside `mutants-list` (justfile:324-325), and neither `gate`, `ci` nor `all` names a campaign recipe, so the roster header's kill claims (.cargo/mutants.toml:48-51, read) are re-attested by nothing that runs. The deleted-assertion construction was not executed since it requires the forbidden recipes; my own mutant runs above are a small ad hoc campaign that found two survivors (read_bits at build.rs:297 and watermark drop_below's latent ordering).

## module-graph-2

Outcome: inconclusive

Witness agent: 2

Test code:

```rust
(none: clippy is not among the commands I was permitted to run)
```

Command:

```text
awk 'NR>=146 && NR<=150' justfile ; sed -n 49p crates/before/Cargo.toml
```

Output:

```text
   146	clippy-default:
   147	    cargo clippy -p suanpan --lib --tests -- -D warnings
   148	    cargo clippy -p before --lib --tests -- -D warnings
   149	    cargo clippy -p rumors --lib --tests -- -D warnings
   150	    cargo clippy -p rumors --lib -- -D warnings
before = { workspace = true, features = ["oracle", "meter"] }
```

Explanation:

Not constructed: the finding's own construction is a clippy invocation, which my brief does not permit, and the resolver-2 feature-unification claim needs a compiler run to settle. By reading I confirmed the two facts the argument rests on: the self-dev-dependency at crates/before/Cargo.toml:49 lights oracle+meter on the lib whenever test targets are built, and justfile:148 is the only default-feature before line, with `--lib --tests` and no bare `--lib` line for before (rumors gets one at 150). Whether the bare default-feature before lib is clean under -D warnings today is unverified.

## party-11

Outcome: demonstrated

Witness agent: 3

Test code:

```rust
macro_rules! chain {
    ($e:expr ;) => { $e };
    ($e:expr ; x $($rest:tt)*) => { chain!(($e, 0u8) ; $($rest)*) };
}

fn literal_scan_bits(build: impl FnOnce() -> Party) -> (u64, u64) {
    meter::reset_scan_bits();
    let p = build();
    let bits = meter::scan_bits();
    (bits, p.encoded_bits())
}

/// party-11: the tuple-literal door documents `O(n)`; a linear door reads
/// about x2 scanned bits per depth doubling, a per-level re-copy and
/// re-parse reads about x4.
#[test]
fn witness_party_11_tuple_literal_scan_bits_are_linear_in_depth() {
    let (b4, s4) = literal_scan_bits(|| Party::try_from(chain!(1u8 ; x x x x)).unwrap());
    let (b8, s8) = literal_scan_bits(|| Party::try_from(chain!(1u8 ; x x x x x x x x)).unwrap());
    let (b16, s16) = literal_scan_bits(|| {
        Party::try_from(chain!(1u8 ; x x x x x x x x x x x x x x x x)).unwrap()
    });
    let (b32, s32) = literal_scan_bits(|| {
        Party::try_from(chain!(1u8 ; x x x x x x x x x x x x x x x x x x x x x x x x x x x x x x x x))
            .unwrap()
    });
    eprintln!(
        "MEASURED party_11: depth4 scan={b4} size={s4}b | depth8 scan={b8} size={s8}b | \
         depth16 scan={b16} size={s16}b | depth32 scan={b32} size={s32}b | \
         growth 8->16 x{:.2}, 16->32 x{:.2} (size growth x{:.2}, x{:.2})",
        b16 as f64 / b8 as f64,
        b32 as f64 / b16 as f64,
        s16 as f64 / s8 as f64,
        s32 as f64 / s16 as f64,
    );
    assert!(
        (b32 as f64) <= 2.5 * (b16 as f64),
        "tuple-literal door: scanned bits grew x{:.2} across a depth doubling (16 -> 32); \
         an O(n) door reads about x2",
        b32 as f64 / b16 as f64
    );
}
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --success-output immediate -E 'test(/witness_/)' -E 'test(party_join_all_log_factor_is_alive)'
```

Output:

```text
MEASURED party_11: depth4 scan=28 size=10b | depth8 scan=88 size=18b | depth16 scan=304 size=34b | depth32 scan=1120 size=66b | growth 8->16 x3.45, 16->32 x3.68 (size growth x1.89, x1.94)

thread 'witness_party_11_tuple_literal_scan_bits_are_linear_in_depth' (241406473) panicked at crates/before/tests/zz_witness_3.rs:53:5:
tuple-literal door: scanned bits grew x3.68 across a depth doubling (16 -> 32); an O(n) door reads about x2
```

Explanation:

Ran (scan-meter). A left-nested tuple literal of depth d builds a party of ~2d bits (size grows x1.9 per doubling) while the door's scanned bits grow x3.45 and x3.68 per doubling: quadratic in depth, against the documented `O(n)` at party.rs:899. Mechanism verified by reading: `(T, S)::into_id_bits` (party.rs:838-843) calls `codec::id_node` at every level, which copies both children (`extend_from_buf`, literal.rs:62-63) and runs `validate_id` = a full `parse_id` over the assembled subtree (literal.rs:64, tree.rs:114-124).

## party-23

Outcome: inconclusive

Witness agent: 3

Test code:

```rust
(not constructed: an accumulator over 2^32 packed bits, >= 512 MiB of id, is not materializable within the test cap)
```

Command:

```text
(none)
```

Output:

```text
(none)
```

Explanation:

Not constructed within the cap. Verified by reading: `IdIndex::build` returns `IdIndex { bits, rights: None }` when `bits.len() > u64::from(u32::MAX)` (index.rs:73-75), and `is_disjoint` on a `None` table returns `IdReader::root(self.bits).is_disjoint(other)` (index.rs:174-178), a full cursor walk of the accumulator per input, which is the Θ(k·|self|) fold cost the finding names. The field doc at index.rs:53-56 states the fallback; whether the public island contract carries a size clause is a documentation reading I did not re-check beyond the finding's quote.

## rank-20

Outcome: demonstrated

Witness agent: 3

Test code:

```rust
fn spine(depth: usize) -> Version {
    let mut text = String::from("1");
    for _ in 0..depth {
        text = format!("(0, {text}, 0)");
    }
    text.parse().unwrap()
}

fn rank_sum_touches(ranks: Vec<Rank>) -> (u64, Rank) {
    meter::reset_touch_ops();
    let total: Rank = ranks.into_iter().sum();
    (meter::touch_ops(), total)
}

/// rank-20: the doc says a summand raising the maximum exponent rescales the
/// accumulator once, "paid by the exponent the summand itself carries". In
/// ascending exponent order every summand raises the maximum by one, so if
/// each rescale is O(held digits) the ascending sum costs about n * W/32
/// touches against the descending order's about n + W/32.
#[test]
fn witness_rank_20_ascending_exponent_sum_is_amortized() {
    let wide_ticks: Ticks = "9".repeat(80_000).parse().unwrap();
    let mut wv = Version::new();
    wv.ticks(&Party::seed(), wide_ticks);
    let wide = wv.rank();
    let held_digits = wide.encode().len() as u64 / 4; // ~ W/32 digits of 32 bits
    let steps = |n: usize| -> Vec<Rank> { (1..=n).map(|k| spine(k).rank()).collect() };

    let mut asc64 = vec![wide.clone()];
    asc64.extend(steps(64));
    let mut asc128 = vec![wide.clone()];
    asc128.extend(steps(128));
    let mut desc128: Vec<Rank> = steps(128).into_iter().rev().collect();
    desc128.push(wide.clone());

    let (t_asc64, s1) = rank_sum_touches(asc64);
    let (t_asc128, s2) = rank_sum_touches(asc128);
    let (t_desc128, s3) = rank_sum_touches(desc128);
    assert_eq!(s2, s3, "the sum is order-independent by value");
    assert!(s1 < s2);
    eprintln!(
        "MEASURED rank_20: held_digits~{held_digits} asc(n=64) touches={t_asc64} \
         asc(n=128) touches={t_asc128} desc(n=128) touches={t_desc128} \
         asc/desc x{:.1}",
        t_asc128 as f64 / t_desc128.max(1) as f64
    );
    assert!(
        t_asc128 <= 4 * t_desc128,
        "ascending-exponent sum costs {t_asc128} touches against {t_desc128} descending: \
         the accumulator is rescaled per summand, not amortized"
    );
}
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --success-output immediate -E 'test(/witness_/)' -E 'test(party_join_all_log_factor_is_alive)'
```

Output:

```text
MEASURED rank_20: held_digits~8306 asc(n=64) touches=1023760 asc(n=128) touches=2019782 desc(n=128) touches=20897 asc/desc x96.7
thread 'witness_rank_20_ascending_exponent_sum_is_amortized' (241406464) panicked at crates/before/tests/zz_witness_3.rs:109:5:
ascending-exponent sum costs 2019782 touches against 20897 descending: the accumulator is rescaled per summand, not amortized
```

Explanation:

Ran (limb-meter/touch counter). A ~265k-bit integer rank followed by 1/2, 1/4, ..., 1/2^n in ascending order costs 1,023,760 touches at n=64 and 2,019,782 at n=128 (about n times the ~8,306 held digits), while the same summands in descending order cost 20,897: the ascending sum is x96.7 the descending one and doubles with n while the content grows by 64 unit bits. Mechanism verified by reading: `sum_ranks` calls `acc.shl(rank.exp - exp)` on every exponent raise (rank.rs:1036-1039) and `Accumulator::shl` on a digit-engine value is `O(|self|)` (`mem::take` then `add_accum_shl`, accumulator.rs:594, 626-627), so the doc's 'paid by the exponent the summand itself carries' (rank.rs:1013-1015) does not hold for this order.

## span-causally-36

Outcome: demonstrated

Witness agent: 3

Test code:

```rust
struct HoleWorld {
    holes: Vec<Version>,
    hi: Version,
}

fn hole_world(k: usize, extra: usize) -> HoleWorld {
    let mut root = Clock::seed();
    let mut kids: Vec<Clock> = root.forks(k as u64).collect();
    let holes: Vec<Version> = kids.iter_mut().map(|c| c.tick().clone()).collect();
    let mut extras: Vec<Clock> = root.forks(extra as u64).collect();
    // Distinct tick counts per extra party keep the join from collapsing to a
    // plateau, so `hi` stays wide.
    let more: Vec<Version> = extras
        .iter_mut()
        .enumerate()
        .map(|(i, c)| c.ticks(i as u64 + 1).clone())
        .collect();
    let hi = holes[0].join_all(holes[1..].to_vec()).join_all(more);
    HoleWorld { holes, hi }
}

fn conjoin_since(holes: &[Version]) -> Query<'_, before::causally::Down> {
    let mut q = since(&holes[0]);
    for h in &holes[1..] {
        q = q & since(h);
    }
    q
}

fn scanned<R>(f: impl FnOnce() -> R) -> (u64, R) {
    meter::reset_scan_bits();
    let r = f();
    (meter::scan_bits(), r)
}

/// span-causally-36: `Query::coverage` promises at most two traversals of
/// the endpoints and bounds, O(|self| + |span|). With k pairwise-concurrent
/// holes and a span whose top strictly dominates every hole, the Partial
/// refinement should add only the new holes' own bits when k grows; a
/// per-hole re-sweep of the clamped top adds about (k2 - k1) * |hi| bits.
#[test]
fn witness_span_causally_36_coverage_cost_is_linear_in_holes() {
    let world = hole_world(64, 2048);
    let lo = Version::new();
    let span = Span::new(&lo, &world.hi).unwrap();
    let hi_bits = world.hi.encoded_bits();
    let hole_bits: Vec<u64> = world.holes.iter().map(|h| h.encoded_bits()).collect();

    let q8 = conjoin_since(&world.holes[..8]);
    let q64 = conjoin_since(&world.holes[..64]);
    let (cov8, v8) = scanned(|| q8.coverage(span.reborrow()));
    let (cov64, v64) = scanned(|| q64.coverage(span.reborrow()));
    let (con8, _) = scanned(|| q8.contains(&world.hi));
    let (con64, _) = scanned(|| q64.contains(&world.hi));
    assert_eq!(v8, Coverage::Partial);
    assert_eq!(v64, Coverage::Partial);
    let added_hole_bits: u64 = hole_bits[8..64].iter().sum();
    eprintln!(
        "MEASURED span_causally_36: |hi|={hi_bits}b holes 8->64 add {added_hole_bits}b; \
         coverage scan k=8: {cov8} k=64: {cov64} (delta {}) ; contains scan k=8: {con8} \
         k=64: {con64} (delta {}); delta/|hi| = {:.1}",
        cov64 - cov8,
        con64 as i64 - con8 as i64,
        (cov64 - cov8) as f64 / hi_bits as f64
    );
    assert!(
        cov64 - cov8 <= 4 * added_hole_bits + hi_bits,
        "coverage: adding 56 holes added {} scanned bits, {:.1}x |hi|; the contract \
         O(|self| + |span|) bounds the addition by the holes' own bits",
        cov64 - cov8,
        (cov64 - cov8) as f64 / hi_bits as f64
    );
}
```

Command:

```text
cargo nextest run -p before --all-features --success-output immediate -E 'test(witness_envelopes_b_18_sequential_join_fold_reads_superlinear_on_stagger)' -E 'test(witness_span_causally_36_coverage_cost_is_linear_in_holes)'
```

Output:

```text
MEASURED span_causally_36: |hi|=10285b holes 8->64 add 1342b; coverage scan k=8: 92984 k=64: 671628 (delta 578644) ; contains scan k=8: 80 k=64: 280 (delta 200); delta/|hi| = 56.3
thread 'witness_span_causally_36_coverage_cost_is_linear_in_holes' (241421860) panicked at crates/before/tests/zz_witness_3.rs:183:5:
coverage: adding 56 holes added 578644 scanned bits, 56.3x |hi|; the contract O(|self| + |span|) bounds the addition by the holes' own bits
```

Explanation:

Ran (scan-meter), second iteration. 64 pairwise-concurrent once-ticked forks are the holes; `hi` is their join plus 2048 further forks ticked 1..2048 times (distinct heights keep the join from normalizing to a plateau; the first iteration's `hi` collapsed to 61 bits and still failed the bound, 6100 vs 5429). Going from 8 to 64 holes adds 1,342 hole bits but 578,644 scanned bits on `coverage`, i.e. 56.3 x |hi| for 56 added holes, exactly one full sweep of the clamped top per hole; the linear control `contains(&hi)` adds 200 bits. Mechanism verified by reading: `refine_partial` evaluates `P::hole_covers` per hole (query.rs:161-165), which is `le(clamped_hi, hole.at)` via `partial_cmp` (polarity.rs:69-79, causally.rs:162-164), and `order_exit` breaks only when both directions are refuted (sweep.rs:236-242); with `hi` strictly above every hole, `ge` is never refuted and each sweep runs to exhaustion.

## skyline-fill-grow-2

Outcome: demonstrated

Witness agent: 3

Test code:

```rust
#[global_allocator]
static HEAP: PeakAlloc = PeakAlloc;

fn tick_peak_heap(ev: meter::Packed, id: meter::Packed) -> (usize, usize) {
    let mut v = ev.version();
    let p = Party::decode(&id.bytes[..]).expect("registry id is canonical");
    let input = v.encode().len() + id.bytes.len();
    HEAP.reset_peak_usage();
    let baseline = HEAP.current_usage();
    v.tick(&p);
    let peak = HEAP.peak_usage().saturating_sub(baseline);
    drop(v);
    (peak, input)
}

/// skyline-fill-grow-2: the crate promises transient space at most a small
/// constant multiple of the input and the board enforces 16 B/B over an
/// 8192 B flat allowance; the memo comb and distinct-minima memo chain are
/// envelope-only families no heap instrument reads.
#[test]
fn witness_skyline_fill_grow_2_memo_families_heap_within_board_ceiling() {
    let mut worst = 0.0f64;
    for (name, ev, id) in [
        (
            "memo_comb d=1000",
            Shape::MemoComb.packed1(1_000),
            Shape::MemoCombId.packed1(1_000),
        ),
        (
            "memo_comb d=2000",
            Shape::MemoComb.packed1(2_000),
            Shape::MemoCombId.packed1(2_000),
        ),
        (
            "memo_chain(distinct) k=1000",
            Shape::MemoChain.packed_flagged(1_000, true),
            Shape::MemoChainId.packed1(1_000),
        ),
        (
            "memo_chain(distinct) k=2000",
            Shape::MemoChain.packed_flagged(2_000, true),
            Shape::MemoChainId.packed1(2_000),
        ),
    ] {
        let (peak, input) = tick_peak_heap(ev, id);
        let per_byte = peak.saturating_sub(8_192) as f64 / input as f64;
        eprintln!(
            "MEASURED skyline_fill_grow_2 {name}: input={input}B peak_heap={peak}B \
             (peak-8192)/input={per_byte:.1} B/B"
        );
        worst = worst.max(per_byte);
    }
    assert!(
        worst <= 16.0,
        "memo family tick peaked {worst:.1} transient heap bytes per input byte over the \
         8192 B allowance; the board ceiling is 16 B/B"
    );
}
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --success-output immediate -E 'test(/witness_/)' -E 'test(party_join_all_log_factor_is_alive)'
```

Output:

```text
MEASURED skyline_fill_grow_2 memo_comb d=1000: input=7374B peak_heap=779904B (peak-8192)/input=104.7 B/B
MEASURED skyline_fill_grow_2 memo_comb d=2000: input=15738B peak_heap=1559424B (peak-8192)/input=98.6 B/B
MEASURED skyline_fill_grow_2 memo_chain(distinct) k=1000: input=6497B peak_heap=356605B (peak-8192)/input=53.6 B/B
MEASURED skyline_fill_grow_2 memo_chain(distinct) k=2000: input=13985B peak_heap=714203B (peak-8192)/input=50.5 B/B
thread 'witness_skyline_fill_grow_2_memo_families_heap_within_board_ceiling' (241406465) panicked at crates/before/tests/zz_witness_3.rs:237:5:
memo family tick peaked 104.7 transient heap bytes per input byte over the 8192 B allowance; the board ceiling is 16 B/B
```

Explanation:

Ran under a PeakAlloc global allocator (process-per-test). Ticking the memo comb peaks at 104.7 and 98.6 transient heap bytes per input byte (d = 1000, 2000) and the distinct-minima memo chain at 53.6 and 50.5 B/B (k = 1000, 2000), flat across the doubling and 3x to 6.5x over the board's 16 B/B ceiling (ceilings.rs:73, 78). This is the layout-predicted order (the finding estimated 50-100 B/B). The prose at fill.rs:121-134 and the `SuspendedLevel` with two `Accumulator`s per level (prescan.rs:107-112) were read; whether the excess is exactly the suspend stack versus `Memo::links` was not separated by measurement.

## skyline-sweep-place-masked-21

Outcome: demonstrated

Witness agent: 3

Test code:

```rust
fn dyadic_holes(k: usize, leaves: usize) -> Vec<Version> {
    let mut root = Clock::seed();
    let subs: Vec<Clock> = root.forks(k as u64).collect();
    subs.into_iter()
        .map(|mut sub| {
            let vs: Vec<Version> = sub
                .forks(leaves as u64)
                .map(|mut c| c.tick().clone())
                .collect();
            vs[0].join_all(vs[1..].to_vec())
        })
        .collect()
}

fn touched<R>(f: impl FnOnce() -> R) -> (u64, u64, R) {
    meter::reset_touch_ops();
    meter::reset_scan_bits();
    let r = f();
    (meter::touch_ops(), meter::scan_bits(), r)
}

/// skyline-sweep-place-masked-21: the fused membership walk is stated
/// `O(|v| + Σ|bound|)`. With k concurrent holes each a B-leaf subtree under
/// its own dyadic prefix and an empty probe (no hole ever refuted), a walk
/// that reads every live pair per elementary interval costs about k * Σ|h_i|
/// touches: doubling k at fixed B quadruples it, where the composed
/// per-hole sweeps double.
#[test]
fn witness_filter_21_fused_admits_touches_are_linear_in_bounds() {
    let probe = Version::new();
    let run = |k: usize| {
        let holes = dyadic_holes(k, 64);
        let total_bits: u64 = holes.iter().map(|h| h.encoded_bits()).sum();
        let q = conjoin_since(&holes);
        let (fused_t, fused_s, admitted) = touched(|| q.contains(&probe));
        assert!(!admitted, "the empty probe is at or below every hole");
        let (comp_t, comp_s, _) = touched(|| {
            holes
                .iter()
                .map(|h| since(h).contains(&probe))
                .fold(0usize, |n, b| n + usize::from(b))
        });
        eprintln!(
            "MEASURED filter_21 k={k}: Σ|h|={total_bits}b fused touches={fused_t} scan={fused_s} \
             | composed touches={comp_t} scan={comp_s}"
        );
        (fused_t, comp_t, total_bits)
    };
    let (f16, c16, b16) = run(16);
    let (f32, c32, b32) = run(32);
    eprintln!(
        "MEASURED filter_21 growth 16->32: fused x{:.2} composed x{:.2} bounds bits x{:.2}",
        f32 as f64 / f16 as f64,
        c32 as f64 / c16 as f64,
        b32 as f64 / b16 as f64
    );
    assert!(
        (f32 as f64) <= 3.0 * (f16 as f64),
        "fused filter walk: touches grew x{:.2} across a hole-count doubling at fixed hole \
         size (bounds bits x{:.2}); O(|v| + Σ|bound|) reads about x2",
        f32 as f64 / f16 as f64,
        b32 as f64 / b16 as f64
    );
}
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --success-output immediate -E 'test(/witness_/)' -E 'test(party_join_all_log_factor_is_alive)'
```

Output:

```text
MEASURED filter_21 k=16: Σ|h|=625b fused touches=2095 scan=627 | composed touches=224 scan=657
MEASURED filter_21 k=32: Σ|h|=1345b fused touches=8287 scan=1347 | composed touches=480 scan=1409
MEASURED filter_21 growth 16->32: fused x3.96 composed x2.14 bounds bits x2.15
thread 'witness_filter_21_fused_admits_touches_are_linear_in_bounds' (241406463) panicked at crates/before/tests/zz_witness_3.rs:302:5:
fused filter walk: touches grew x3.96 across a hole-count doubling at fixed hole size (bounds bits x2.15); O(|v| + Σ|bound|) reads about x2
```

Explanation:

Ran (touch and scan meters). With k concurrent holes (each 64 once-ticked leaves under its own dyadic prefix) and the empty probe, `Query::contains` through the fused walk grows x3.96 in accumulator touches when k doubles at fixed hole size, while the composed per-hole `since(h).contains(&probe)` sweeps grow x2.14 alongside the bounds' bits (x2.15). Scan bits stay linear (each stream decoded once), so the superlinear term is the per-interval pair work, as the finding says. Mechanism verified by reading: `side.pair.read()` per live side per elementary interval (filter.rs:184-187) and the probe's step folding into every surviving side's pair (filter.rs:279-287), against the module's stated `O(|v| + Σ|bound|)` (filter.rs:46-47).

## clock-3

Outcome: demonstrated

Witness agent: 3

Test code:

```rust
/// clock-3: `Forks` documents "Yields exactly `k` disjoint clocks" and
/// `Clock::forks` "yields `n` children"; at `k == u64::MAX` the count is
/// asserted here as documented.
#[test]
fn witness_clock_3_forks_max_yields_exactly_k() {
    let mut c = Clock::seed();
    let yielded = c.forks(u64::MAX).len() as u64;
    assert_eq!(
        yielded,
        u64::MAX,
        "forks(u64::MAX) yields {yielded} children, not the documented `exactly k`"
    );
}
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --success-output immediate -E 'test(/witness_/)' -E 'test(party_join_all_log_factor_is_alive)'
```

Output:

```text
thread 'witness_clock_3_forks_max_yields_exactly_k' (241406471) panicked at crates/before/tests/zz_witness_3.rs:320:5:
assertion `left == right` failed: forks(u64::MAX) yields 18446744073709551614 children, not the documented `exactly k`
  left: 18446744073709551614
```

Explanation:

Ran. `Clock::seed().forks(u64::MAX).len()` is 18446744073709551614 = u64::MAX - 1, against clock/forks.rs:9 'Yields exactly `k` disjoint clocks' and clock.rs:166 'yields `n` children'. The saturation is stated only in a private comment (party/forks.rs:111-114, `Split::new(whole, k.saturating_add(1))`), and tests/forks_max.rs:49-50 pins exactly u64::MAX - 1. The 'keeps the last share' wording versus the residual being the first preorder leaf (party/forks.rs:107-110) was verified by reading only.

## codec-bits-23

Outcome: inconclusive

Witness agent: 3

Test code:

```rust
(not constructed: requires a wasm32 or other 32-bit target; the permitted nextest runs execute on the 64-bit host, where the arm is dead)
```

Command:

```text
(none)
```

Output:

```text
(none)
```

Explanation:

Not runnable within the cap or on this host. Verified by reading the dependency: `gamma.rs:249-253` admits any `k` that fits `usize` and calls `UBig::ZERO.set_bit(k)`; dashu-int 0.5.0 `Repr::set_bit` on a `Small` value with `n >= DWORD_BITS` calls `with_bit_dword_spilled` (bits.rs:428-439), which does `Buffer::allocate(idx + 1)` with `idx = n / WORD_BITS`, `push(lo)`, `push(hi)`, `push_zeros(idx - 2)`, then `push(1 << (n % WORD_BITS))` (bits.rs:551-561); `allocate` clamps via `default_capacity(...).min(MAX_CAPACITY)` with `MAX_CAPACITY = usize::MAX / WORD_BITS_USIZE` (buffer.rs:48, 57-60, 121-123), and `push` asserts `self.len < self.capacity` (buffer.rs:205-206). With `Word = u32` on the 32-bit arch (arch/generic_32_bit/word.rs:2), `MAX_CAPACITY = 2^27 - 1`; for `k` in [2^32 - 32, 2^32 - 1], `idx = 2^27 - 1`, the buffer is clamped to 2^27 - 1 words, and the final push is the 2^27-th, so the release `assert!` fires. The same guard sits at dsi.rs:255-267. Whether the ~1 GiB working set is reachable before the trap on wasm32 was not verified; the arch cfg selecting `generic_32_bit` for wasm32 was not read.

## crate-root-25

Outcome: demonstrated

Witness agent: 3

Test code:

```rust
(textual construction; no test)
```

Command:

```text
(none)
```

Output:

```text
lib.rs:5-6: "//! maintaining asymptotically linear and practically quick performance even\n//! over the most adversarially pessimal inputs."
lib.rs:351-353: "//! pathological input shapes. Any asymptotic claim is a hard guarantee that the\n//! operation will perform in time proportionate to that bound, for all input\n//! sizes, no matter how unlikely and contorted the shape of the input."
crates/before/fuelscape/version_rank.json: "contract":"`O(M(|self|) · log |self|)` time, `O(|self|)` space"
version.rs:293: "/// Typical inputs run far below the worst case; `M` is the complexity of unbounded-integer multiplication (about `O(n log n)` in this implementation)."
```

Explanation:

Verified by reading, not by run. The headline at lib.rs:5-6 says 'asymptotically linear'; lib.rs:351-353 makes every asymptotic claim a hard guarantee; the rendered `Version::rank` contract (included from OUT_DIR via the version_rank fuelscape, whose committed JSON carries `O(M(|self|) · log |self|)` with `M` about `O(n log n)`) is superlinear on the same doc set. The asymptotics pins `version_join_all_log_factor_is_alive` (asymptotics.rs:257-260) and `party_join_all_log_factor_is_alive` (339-349) exist and hold a log factor above linear; the latter ran green in run 1 (see testing-diff-gen-28).

## suanpan-2

Outcome: demonstrated

Witness agent: 3

Test code:

```rust
#![cfg(feature = "touch-meter")]

use std::sync::atomic::{AtomicU64, Ordering};

use proptest::prelude::*;
use suanpan::{touch_meter, Accumulator};

#[derive(Debug, Clone)]
enum Op {
    AddSmall(i64),
    AddLimbsShl(Vec<u64>, u16),
    SubLimbsShl(Vec<u64>, u16),
    Shl(u8),
    Negate,
    Sign,
}

impl Op {
    /// The operand limbs a wide operation reads, one for everything else.
    fn work(&self) -> u64 {
        match self {
            Op::AddLimbsShl(l, _) | Op::SubLimbsShl(l, _) => l.len().max(1) as u64,
            _ => 1,
        }
    }
}

fn arb_op() -> impl Strategy<Value = Op> {
    prop_oneof![
        any::<i64>().prop_map(Op::AddSmall),
        (prop::collection::vec(any::<u64>(), 1..6), 0..4096u16)
            .prop_map(|(l, s)| Op::AddLimbsShl(l, s)),
        (prop::collection::vec(any::<u64>(), 1..6), 0..4096u16)
            .prop_map(|(l, s)| Op::SubLimbsShl(l, s)),
        (0..64u8).prop_map(Op::Shl),
        Just(Op::Negate),
        Just(Op::Sign),
    ]
}

fn apply(acc: &mut Accumulator, op: &Op) {
    match op {
        Op::AddSmall(d) => acc.add_small(*d),
        Op::AddLimbsShl(l, s) => acc.add_limbs_shl(l.iter().copied(), u64::from(*s)),
        Op::SubLimbsShl(l, s) => acc.sub_limbs_shl(l.iter().copied(), u64::from(*s)),
        Op::Shl(s) => acc.shl(u64::from(*s)),
        Op::Negate => acc.negate(),
        Op::Sign => {
            acc.sign();
        }
    }
}

static MAX_MILLI_RATIO: AtomicU64 = AtomicU64::new(0);

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    /// Touches stay linear in the work the sequence itself carries (operand
    /// limbs plus one per operation and per sign read), on an arbitrary
    /// interleaving of small deltas, wide shifted deltas, shifts, negations,
    /// and sign reads. The constants are generous placeholders: the point of
    /// the witness is the instrument's shape, and the printed maximum ratio
    /// is the measured basis a real pin would tighten from.
    #[test]
    fn witness_suanpan_2_touches_are_linear_in_work(
        ops in prop::collection::vec(arb_op(), 1..300)
    ) {
        let mut acc = Accumulator::new();
        let mut work = 0u64;
        touch_meter::reset();
        for op in &ops {
            apply(&mut acc, op);
            work += op.work();
            acc.sign();
            work += 1;
        }
        let touches = touch_meter::touches();
        let milli = touches * 1000 / work.max(1);
        let prev = MAX_MILLI_RATIO.fetch_max(milli, Ordering::Relaxed);
        if milli > prev {
            eprintln!(
                "MEASURED suanpan_2: new max touches/work = {}.{:03} (touches {touches}, \
                 work {work}, ops {})",
                milli / 1000,
                milli % 1000,
                ops.len()
            );
        }
        prop_assert!(
            touches <= 64 * work + 4096,
            "touches {touches} exceed 64 * work {work} + 4096 on a {}-op sequence",
            ops.len()
        );
    }
}
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --success-output immediate -E 'test(/witness_/)' -E 'test(party_join_all_log_factor_is_alive)'
```

Output:

```text
PASS [   1.549s] (13/13) suanpan::zz_witness_3 witness_suanpan_2_touches_are_linear_in_work
MEASURED suanpan_2: new max touches/work = 31.276 (touches 24020, work 768, ops 296)
...
MEASURED suanpan_2: new max touches/work = 44.011 (touches 30412, work 691, ops 271)
```

Explanation:

The gap itself is verified by reading: among suanpan's test modules only `accumulator/tests/metered.rs` references `touch_meter`/`touches()` (grep -ln over crates/suanpan/src/accumulator/tests/*.rs), so the proptests in `differential.rs:340` and `ledger.rs:241` never read the meter, and `tests/amortized_sequences.rs` drives one fixed schedule. The instrument the finding asks for is constructible: the proptest above ran 512 arbitrary mixed sequences serially, reading a maximum of 44.0 touches per unit of work under a placeholder bound of 64·work + 4096. My work model counts operand limbs and one per op and sign read; it does not charge `shl`/`negate` their documented O(held digits), which is why the observed ratio is well above the crate's per-limb constants, and a real pin would need a work denominator derived from the crate's stated costs. No known-bad mutation was run against this instrument (cap).

## suanpan-tests-25

Outcome: demonstrated

Witness agent: 3

Test code:

```rust
// Mutation applied to crates/suanpan/src/accumulator.rs (lines 21-26), then restored:
-fn touch(count: u64) {
-    #[cfg(feature = "touch-meter")]
-    crate::touch_meter::record(count);
-    #[cfg(not(feature = "touch-meter"))]
-    let _ = count;
-}
+fn touch(count: u64) {
+    // WITNESS MUTATION (suanpan-tests-25): the counter is dark.
+    let _ = count;
+}
```

Command:

```text
cargo nextest run -p suanpan --all-features --success-output immediate -E 'test(sign_flip_oscillation_has_no_width_product)' -E 'test(/accumulator::tests::metered::/)'
```

Output:

```text
PASS [   0.010s] (17/17) suanpan::amortized_sequences sign_flip_oscillation_has_no_width_product
MEASURED s1_sign_flip: grid [0, 0, 0, 0] mixed 0 bound 0
...
thread 'accumulator::tests::metered::sign_fold_skips_certified_runs' (241435468) panicked at crates/suanpan/src/accumulator/tests/metered.rs:232:5:
assertion `left == right` failed: 3 fold reads + 3 collapse zeroes: a zero partial crosses the certified run in one skip, touching none of its digits
  left: 0
 right: 6
...
Summary [   0.018s] 17 tests run: 3 passed, 14 failed, 44 skipped
```

Explanation:

Ran with the counter made dark. `sign_flip_oscillation_has_no_width_product` passes with grid [0, 0, 0, 0] (mixed 0 <= bound 0): `assert_no_product` (amortized_sequences.rs:52-61) has no liveness floor. In the same run 14 of the 16 `metered.rs` pins fail (the two that pass are `magnitude_dispatch_costs_its_width_path` and `limb_stream_matches_the_wide_entry`, so the finding's 'every pin in metered.rs fails' is slightly overstated). The uniformly-undercounting variant (deleting only the `touch(1)` at accumulator.rs:850) was not run (cap); the before-side floor `run.touches >= run.ops` at tests/meter.rs:6493-6494 was read as quoted.

## meter-registry-tier2-10

Outcome: demonstrated

Witness agent: 3

Test code:

```rust
(textual construction; greps and line reads, no test)
```

Command:

```text
(none)
```

Output:

```text
registry.rs:1109: const TICK_CROSS_UNBANDED: &str = "tick cross: the tick gate pins in tests/meter.rs price its walk";
registry.rs:1379/1390/1401/1412/1423 shapes: [Dense, NestedFullId] / [Bigroot, NestedFullId] / [WideTail, NestedLeftFullId] x2 / [Staircase, IdSpine]; all five reason: TICK_CROSS_UNBANDED
tests/meter.rs tick pins: 471-473 `let seed = Party::seed(); ... v.tick(&seed)`; 484-485 Bigroot.packed2 x NestedFullId; 504-505 WideTail.packed2(TICK_CROSS_SCALE, TICK_CROSS_SCALE) x NestedLeftFullId; 530-531 Staircase x IdSpine.packed_flagged(HOLE_ID_DEPTH, true)
grep 'Shape::Staircase|Shape::IdSpine|NestedFullId|NestedLeftFullId' tests/meter.rs: 485, 505, 530, 531, 683, 702, 738, 743, 897, 902, 2077, 2108, 6114-6173 (none a Dense x NestedFullId, WideTail(1, d) x NestedLeftFullId, or Staircase x IdSpine(d, false) tick)
registry.rs:1657-1664 (CliffFan): "priced by the in-crate skyline and tier2 suites' pinned envelopes" / "its pins are absolute envelopes in the in-crate skyline and tier2 suites, not two-point bands in tests/meter.rs"
skyline/tests.rs:555-566: Shape::CliffFan.packed2(64, 64), .packed2(512, 128) inside `for p in &shapes { assert_agreement(&version_of(p)); }`
tests/meter.rs:6637 fn accum_fan_touches_flat(); 6670 fn accum_cancelling_touches_flat()
tests/meter.rs:2704: let r = meter::skyline::query::rank(meter::skyline::view(&enc));
```

Explanation:

Verified by reading. (1) The shared reason names tick pins in tests/meter.rs, but the tick pins there cover Bigroot x NestedFullId and WideTail(s, s) x NestedLeftFullId only; `tick_dense_envelope` ticks with `Party::seed()` and the staircase tick pin uses the diverted `IdSpine(HOLE_ID_DEPTH, true)`, so NestedFull, MirrorNarrow, and Staircase have no matching tick pin at the named site. (2) In `src/version/skyline` CliffFan appears only as a corpus member of `assert_agreement` loops (skyline/tests.rs:555-566 and `version_of(&Shape::CliffFan.packed2(16, 8))` in fill/grow/sweep/query/emit tests), and the two-point bands `accum_fan_touches_flat`/`accum_cancelling_touches_flat` live in tests/meter.rs; caveat: `meter/tier2/tests.rs:531` does use `cliff_fan(48, 32).version()` and I did not read its context, so the 'tier2 suites' half of the reason may hold there. (3) `rank_wide_tooth_run` calls the internal `meter::skyline::query::rank` (2704) and its doc (2688-2697) states only the liveness floor and the answer pin, no internal-entry rationale.

## board-families-floors-judge-21

Outcome: demonstrated

Witness agent: 3

Test code:

```rust
/// Review witness (witness:3, finding board-families-floors-judge-21): a peak
/// heap of `8192 + n^2 / 20480` bytes over the ladder 4096..32768 is a
/// quadratic amplifier whose whole excess sits at allowance scale. The
/// acceptance judgment fits the heap exponent over allowance-inclusive
/// readings; the finding predicts both windows read green on "heap exponent"
/// while the top window's own two-point fit reads red. Written to fail when
/// the finding is right.
#[test]
fn witness_board_judge_21_quadratic_heap_under_allowance_reads_red() {
    use super::floors::na;
    use super::judge::{evaluate, evaluate_acceptance, trend};
    use super::measure::Sample;
    use super::{ByCurrency, Floors, HEAP_FLAT_ALLOWANCE_BYTES};
    const PROBE_NA: &str = "probe: the heap exponent leg alone is under test";
    let heap_reading = |n: usize| (HEAP_FLAT_ALLOWANCE_BYTES + n * n / 20480) as u64;
    let sample = |denom: usize| -> Sample {
        Sample {
            denom_bytes: denom,
            exp_denom_bytes: denom,
            limb_denom: denom as u64,
            text_row: false,
            floors: Floors {
                heap: na(PROBE_NA),
                segments: na(PROBE_NA),
                limb: na(PROBE_NA),
                scan: na(PROBE_NA),
                touch: na(PROBE_NA),
            },
            fold_arity: None,
            fold_search_bits: 0,
            heap_model: None,
            declared_heap: None,
            declared_limb: None,
            readings: ByCurrency {
                heap: Some(heap_reading(denom)),
                segments: Some(0),
                limb: None,
                scan: None,
                touch: None,
            },
        }
    };
    let ladder = [4096usize, 8192, 16384, 32768];
    let raw: Vec<(usize, u64)> = ladder.iter().map(|&n| (n, heap_reading(n))).collect();
    let residual: Vec<(usize, u64)> = ladder
        .iter()
        .map(|&n| (n, heap_reading(n) - HEAP_FLAT_ALLOWANCE_BYTES as u64))
        .collect();
    let (lo, hi) = evaluate_acceptance(
        "heap_probe",
        "quadratic-under-allowance",
        (sample(ladder[0]), sample(ladder[1])),
        (sample(ladder[2]), sample(ladder[3])),
    );
    let top = evaluate("heap_probe", "top-window", sample(ladder[2]), sample(ladder[3]));
    eprintln!(
        "MEASURED board_judge_21: readings {raw:?}; four-point trend on raw readings {:.3}, \
         on allowance-subtracted readings {:.3}; top-window two-point trend {:.3}; \
         acceptance red lo={:?} hi={:?}; single top window red={:?}; heap constants \
         lo={:?} hi={:?} B/B",
        trend(&raw),
        trend(&residual),
        trend(&raw[2..]),
        lo.red,
        hi.red,
        top.red,
        lo.scores.heap.per_unit,
        hi.scores.heap.per_unit,
    );
    assert!(
        top.red.iter().any(|r| r.contains("heap exponent")),
        "premise: the top window's own two-point fit reads the amplifier red"
    );
    assert!(
        lo.red.iter().any(|r| r.contains("heap exponent"))
            || hi.red.iter().any(|r| r.contains("heap exponent")),
        "a quadratic heap term of allowance-scale magnitude passes acceptance in both \
         windows (lo red {:?}, hi red {:?})",
        lo.red,
        hi.red
    );
}
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --success-output immediate -E 'test(/witness_/)' -E 'test(party_join_all_log_factor_is_alive)'
```

Output:

```text
MEASURED board_judge_21: readings [(4096, 9011), (8192, 11468), (16384, 21299), (32768, 60620)]; four-point trend on raw readings 0.914, on allowance-subtracted readings 2.000; top-window two-point trend 1.509; acceptance red lo=[] hi=[]; single top window red=["heap exponent"]; heap constants lo=Some(0.39990234375) hi=Some(1.5999755859375) B/B

thread 'meter::board::tests::witness_board_judge_21_quadratic_heap_under_allowance_reads_red' (241406470) panicked at crates/before/src/meter/board/tests.rs:1359:5:
a quadratic heap term of allowance-scale magnitude passes acceptance in both windows (lo red [], hi red [])
```

Explanation:

Ran as a unit test appended to `board/tests.rs` (the judge functions are `pub(super)`), then removed. Exactly the finding's numbers: readings 8192 + n^2/20480 over 4096..32768 give a four-point raw trend of 0.914 (green under MAX_SCALING_EXPONENT 1.15), 2.000 on the allowance-subtracted residuals, 1.509 on the top window's own two points; `evaluate_acceptance` reports no red in either window (heap constants 0.40 and 1.60 B/B), while `evaluate` on the top window alone reports "heap exponent". Mechanism verified by reading: `fit_currency` filters points above the allowance but fits `trend` over the raw readings (judge.rs:132-141) while the constant leg subtracts the allowance (judge.rs:257-259).

## board-ops-render-15

Outcome: demonstrated

Witness agent: 3

Test code:

```rust
(textual construction; grep and line reads, no test; the amp_board example was not built or run)
```

Command:

```text
grep -rn 'SEGMENTS_GROWN' crates/before/src
```

Output:

```text
crates/before/src/recurse.rs:69:static SEGMENTS_GROWN: AtomicU64 = AtomicU64::new(0);
crates/before/src/recurse.rs:79:    SEGMENTS_GROWN.load(Ordering::Relaxed)
crates/before/src/recurse.rs:85:    SEGMENTS_GROWN.store(0, Ordering::Relaxed);
crates/before/src/recurse.rs:99:/// that arm — can count the segment in [`SEGMENTS_GROWN`].
crates/before/src/recurse.rs:106:        SEGMENTS_GROWN.fetch_add(1, Ordering::Relaxed);
recurse.rs:68: #[cfg(any(test, feature = "meter"))]  (the static)
recurse.rs:100-102: #[cfg(test)] #[inline] pub(crate) fn grow<R>(f: impl FnOnce() -> R) -> R {
recurse.rs:74-75: "/// Compiled only for the meter surface: the counter is always written (the bump\n/// is inseparable from the growth arm), but nothing outside the meters ever"
render.rs:212-214: "green iff every meter's exponent <= {MAX_SCALING_EXPONENT}, constants within: \ heap <= {MAX_HEAP_BYTES_PER_INPUT_BYTE} B/B over {HEAP_FLAT_ALLOWANCE_BYTES} B flat, \ segments <= {MAX_GROWN_STACK_SEGMENTS}, \"
```

Explanation:

Verified by reading. The only writer of `SEGMENTS_GROWN` is the `fetch_add` at recurse.rs:106 inside `grow`, which is `#[cfg(test)]` (recurse.rs:100-102), while the static and its readers are `#[cfg(any(test, feature = "meter"))]` (68, 77, 83). Any binary compiled without `cfg(test)`, including the `amp_board` example (release, `required-features = ["limb-meter", "scan-meter"]`, Cargo.toml:115-117) and integration-test binaries, therefore reads zero by construction, while render.rs:212-214 judges the segments column and recurse.rs:17-20 and 74-75 describe the zero as measured and the counter as 'always written'. The unit test `stack_segment_meter_counts_deterministically_and_resets` (meter/tests.rs:408) lives in the lib's own test build. I did not attempt the temporary `descend!` board row or run the example (not permitted).

## surface-roster-7

Outcome: demonstrated

Witness agent: 3

Test code:

```rust
(textual construction; `just surface-totality` was not run; the Neg impl was not added)
```

Command:

```text
grep -n 'FAMILY_SURFACE\|METHOD_SURFACE\|TRAIT_IMPLS' crates/before/surfacecheck/src/main.rs
```

Output:

```text
5://! The surface roster (`before::surface::METHOD_SURFACE`) is enforced
15://!   public-trait-declared methods): a roster row in `METHOD_SURFACE`, or
18://!   a pinned row in [`census::TRAIT_IMPLS`], reconciled both ways, so a
20://!   behind `FAMILY_SURFACE`'s per-family dispositions;
100:    let rostered: BTreeSet<&str> = before::surface::METHOD_SURFACE
171:        println!("{row:100} {}", census_disposition(row, census::TRAIT_IMPLS));
census.rs:251 "Version: impl core::convert::From<&Version> for Cow<Version>", 256 "Version: impl core::convert::From<Version> for Cow<Version>", 262 "Version: impl core::default::Default for Version", 401-405 "error::Overlap: impl ..."
FAMILY_SURFACE (surface.rs:1016..EOF) `op:` rows matching default|cow|neg|overlap|toowide: only "causally ! complement (atom negation into the polar hole)"
surface_coverage/tests.rs:334-336: "/// The family roster's rows are unique by op description (totality over\n/// the operator/trait surface is by review of the file; this pins the\n/// table's internal hygiene)."
```

Explanation:

Verified by reading. surfacecheck reconciles function-like items against `METHOD_SURFACE` (main.rs:100-107) and trait impls against `census::TRAIT_IMPLS`; `FAMILY_SURFACE` is named only in its module doc (line 20) and never read. The census already pins `Default for Version`, both `Cow<Version>` conversions, and the `error::Overlap` impls with no corresponding `FAMILY_SURFACE` row, and the coverage suite's own comment says totality over that roster is 'by review of the file'. So the three prose sites' claim that a new operator impl 'fails that gate until ... a family row here ... is added' is not what the gate checks. Not exercised by adding an impl and running the gate (not permitted).

## testing-oracles-28

Outcome: demonstrated

Witness agent: 3

Test code:

```rust
(textual construction; grep and line reads, no test)
```

Command:

```text
grep -n -i 'fuzz\|wasm32\|surfacecheck\|covcheck\|mutant\|verdict_matrix\|tripwire\|benchjudge\|bench judge\|tamper' crates/before/src/testing/validation_index.rs
```

Output:

```text
55://! [`super::algebraic_laws`] and shared with the fuzz targets). Law
106://! diff. Its adequacy kernels are also the tripwires proving the
109://! **The bench judge** (`tools/benchjudge` over `benches/board.rs`;
112://! expected-verdict roster (`tools/benchjudge-expected.json`, membership
121://! **The fuzz-fit bands** (the `fuzzfit` workspace under this crate).
169://! strict-decode rejection, fuzz seeds) ride the codec test suites and
validation_index.rs:1-3: "//! The validation index: every instrument that guards this crate, what\n//! failure class each one catches that the others cannot, and where it\n//! lives."
validation_index.rs:64-65: "//! **Exhaustive small-scope enumeration** ([`super::exhaustive`]). Total\n//! enumeration of every reachable state and operation pairing inside"
ls: .cargo/mutants.toml crates/before/fuzz crates/before/surfacecheck crates/before/tests/superlinear_tripwires.rs crates/before/tests/verdict_matrix.rs crates/before/wasm32-pins tools/benchjudge tools/covcheck tools/mutantcheck (all exist)
```

Explanation:

Verified by reading, with two corrections to the finding. The page's totality claim (lines 1-3) and the 'every reachable state' wording (64-65) are as quoted, and no row names the wasm32 pins, surfacecheck, covcheck, mutantcheck/.cargo/mutants.toml, the verdict matrix, the tests/ tamper pins, or `tests/superlinear_tripwires.rs` by name. However the finding overstates: the page does carry a row for the bench judge (lines 109-119, naming tools/benchjudge, the expected roster, and tests/bench_judge_roster.rs) and a row for the fuzz-fit bands (121-125), and it mentions fuzz seeds (169) and 'the tripwires' generically (106-107). The fs replay having no row, and whether the enumerator's `all_normal_events(2)` includes shapes no tick sequence produces, were not verified.

## testing-diff-gen-28

Outcome: demonstrated

Witness agent: 3

Test code:

```rust
// The committed pin itself was run (crates/before/src/testing/asymptotics.rs:339-349):
#[cfg(feature = "scan-meter")]
#[test]
fn party_join_all_log_factor_is_alive() {
    const MIN_GROWTH: f64 = 4.89;
    assert_log_factor_alive(
        "Party::join_all",
        party_join_all_scan_bits(256),
        party_join_all_scan_bits(1024),
        MIN_GROWTH,
    );
}
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --success-output immediate -E 'test(/witness_/)' -E 'test(party_join_all_log_factor_is_alive)'
```

Output:

```text
PASS [   0.078s] (11/13) before testing::asymptotics::party_join_all_log_factor_is_alive
MEASURED party_join_all_scan: n=256 input_bytes=40960 scan_bits=7260488
MEASURED party_join_all_scan: n=1024 input_bytes=196608 scan_bits=36191048
```

Explanation:

Ran the committed pin to re-measure its readings: door growth 36191048 / 7260488 = x4.985, byte growth 196608 / 40960 = x4.80, floor 4.89; the floor sits 1.9% below the door reading and 1.9% above the linear reference, matching the finding's numbers. The pin's prose ('a crossing is a class change, never noise', asymptotics.rs module comment as quoted; 'both endpoints are exact counters, so the gap is stable, not noisy' at 336-337) was read. The two-term decomposition (a·k·log k + b·k with b/a about 43) and the claim that a ~5% change in the linear index term crosses the floor are the finder's arithmetic model and were not verified by mutation.

## envelopes-b-18

Outcome: demonstrated

Witness agent: 3

Test code:

```rust
struct FoldRun {
    bytes: u64,
    touches: u64,
    limb_ops: u64,
}

fn stagger_versions(n: usize, m: usize) -> Vec<Version> {
    let (versions, _) = Shape::StaggerPopulation.population(n, m);
    versions.iter().map(meter::Packed::version).collect()
}

fn sequential_join_run(n: usize, m: usize) -> FoldRun {
    let versions = stagger_versions(n, m);
    let bytes: u64 = versions.iter().map(|v| v.encode().len() as u64).sum();
    meter::reset_touch_ops();
    meter::reset_limb_ops();
    let out = versions.iter().fold(Version::new(), |acc, v| acc | v);
    let run = FoldRun {
        bytes,
        touches: meter::touch_ops(),
        limb_ops: meter::limb_ops(),
    };
    assert_eq!(out, "1".parse::<Version>().unwrap());
    run
}

fn balanced_join_run(n: usize, m: usize) -> FoldRun {
    let mut versions = stagger_versions(n, m);
    let bytes: u64 = versions.iter().map(|v| v.encode().len() as u64).sum();
    let rest = versions.split_off(1);
    let receiver = versions.pop().unwrap();
    meter::reset_touch_ops();
    meter::reset_limb_ops();
    let out = receiver.join_all(rest);
    let run = FoldRun {
        bytes,
        touches: meter::touch_ops(),
        limb_ops: meter::limb_ops(),
    };
    assert_eq!(out, "1".parse::<Version>().unwrap());
    run
}

/// envelopes-b-18: tests/meter.rs names the sequential left version fold as
/// the known-bad mechanism the stagger bands separate from ("quadratic in
/// arity ... several times over these bands") but never meters it. The
/// mechanism should read superlinear per byte across an arity doubling
/// (>= x1.49, the meet tripwire's floor); if it does not, the module
/// comment's adequacy claim is itself the finding.
#[test]
fn witness_envelopes_b_18_sequential_join_fold_reads_superlinear_on_stagger() {
    let (n, m) = (64, 64);
    let seq_small = sequential_join_run(n, m);
    let seq_large = sequential_join_run(2 * n, m);
    let bal_small = balanced_join_run(n, m);
    let bal_large = balanced_join_run(2 * n, m);
    let per_byte_growth = |s: &FoldRun, l: &FoldRun, f: fn(&FoldRun) -> u64| {
        (f(l) as f64 / l.bytes as f64) / (f(s) as f64 / s.bytes as f64)
    };
    eprintln!(
        "MEASURED envelopes_b_18 sequential: ({n},{m}) bytes={} touches={} limb={} | \
         ({},{m}) bytes={} touches={} limb={} | per-byte growth touches x{:.2} limb x{:.2}",
        seq_small.bytes,
        seq_small.touches,
        seq_small.limb_ops,
        2 * n,
        seq_large.bytes,
        seq_large.touches,
        seq_large.limb_ops,
        per_byte_growth(&seq_small, &seq_large, |r| r.touches),
        per_byte_growth(&seq_small, &seq_large, |r| r.limb_ops),
    );
    eprintln!(
        "MEASURED envelopes_b_18 balanced: ({n},{m}) touches={} limb={} | ({},{m}) touches={} \
         limb={} | per-byte growth touches x{:.2} limb x{:.2}",
        bal_small.touches,
        bal_small.limb_ops,
        2 * n,
        bal_large.touches,
        bal_large.limb_ops,
        per_byte_growth(&bal_small, &bal_large, |r| r.touches),
        per_byte_growth(&bal_small, &bal_large, |r| r.limb_ops),
    );
    for (name, f) in [
        ("touches", (|r: &FoldRun| r.touches) as fn(&FoldRun) -> u64),
        ("limb ops", |r: &FoldRun| r.limb_ops),
    ] {
        if f(&seq_small) == 0 {
            eprintln!("MEASURED envelopes_b_18: {name} column reads zero at this scale; skipped");
            continue;
        }
        let g = per_byte_growth(&seq_small, &seq_large, f);
        assert!(
            g >= 1.49,
            "the sequential join fold reads only x{g:.2} per-byte growth ({name}) across an \
             arity doubling on the stagger population: the module comment's known-bad \
             mechanism does not fail the band's class test here"
        );
    }
}
```

Command:

```text
cargo nextest run -p before --all-features --success-output immediate -E 'test(witness_envelopes_b_18_sequential_join_fold_reads_superlinear_on_stagger)' -E 'test(witness_span_causally_36_coverage_cost_is_linear_in_holes)'
```

Output:

```text
PASS [   0.081s] (2/2) before::zz_witness_3 witness_envelopes_b_18_sequential_join_fold_reads_superlinear_on_stagger
MEASURED envelopes_b_18 sequential: (64,64) bytes=12800 touches=325561 limb=0 | (128,64) bytes=28672 touches=1263480 limb=0 | per-byte growth touches x1.73 limb xNaN
MEASURED envelopes_b_18 balanced: (64,64) touches=159675 limb=0 | (128,64) touches=401274 limb=0 | per-byte growth touches x1.12 limb xNaN
MEASURED envelopes_b_18: limb ops column reads zero at this scale; skipped
```

Explanation:

The gap is verified by reading: tests/meter.rs:7720-7729 names the sequential left fold as the refuted mechanism and defers its readings to the pin commit, and `version_fold_run` builds `sequential` at 7750 before the meters reset at 7753-7755, so it is never metered. The constructed tripwire (this test, second iteration after a divide-by-zero on the limb column in run 1) shows the mechanism is separable today: the sequential `|` fold reads x1.73 per-byte touch growth across the (64,64) -> (128,64) arity doubling against the balanced fold's x1.12, clearing the x1.49 class floor the meet tripwire uses. The limb column reads 0 for both folds at this scale (unit teeth ride the quick register), so a limb-column tripwire here would be vacuous; a committed demonstration should pin touches.

## tests-other-10

Outcome: demonstrated

Witness agent: 3

Test code:

```rust
// Mutation applied to crates/before/src/span.rs:380, then restored:
-        if self.lo.view().ptr_eq(self.hi.view()) {
+        if false { // WITNESS MUTATION (tests-other-10): precedence rung deleted
```

Command:

```text
cargo nextest run -p before --all-features -E 'binary(coincident_span)' -E 'binary(verdict_matrix)' -E 'test(/^span::tests::/)' -E 'test(/identity_fast_paths/)' -E 'test(precedence_bails_at_the_refuted_end)'
```

Output:

```text
PASS [   0.007s] (20/40) before::coincident_span coincident_argument_collapses_to_the_membership_walk
PASS [   0.008s] (22/40) before::coincident_span coincident_dominance_collapses_to_one_containment
PASS [   0.011s] (23/40) before::coincident_span coincident_place_collapses_to_the_pair_sweep
PASS [   0.009s] (27/40) before::meter placement::precedence_bails_at_the_refuted_end
PASS [   0.125s] (36/40) before span::tests::coincident_span_rungs_agree_across_buffer_identity
PASS [   3.238s] (39/40) before::verdict_matrix production_verdicts_agree_across_the_surfaces
Summary [   3.260s] 40 tests run: 40 passed, 856 skipped
```

Explanation:

Ran with the `Span::precedence` clone-identity rung at span.rs:380 replaced by `if false`. All 40 tests in the suites the finding names (tests/coincident_span.rs, tests/verdict_matrix.rs, `span::tests`, the meter suite's `identity_fast_paths` module and `placement::precedence_bails_at_the_refuted_end`) pass: coincident_span pins place, dominance, and the argument door only (its three tests above), and the verdict suites compare verdicts, which the general arm answers identically. This is not the full gate (not run); clippy/`unreachable_code` warnings under the mutation were not checked. The receiver rung at span.rs:459 was not mutated (cap).

## tests-other-18

Outcome: demonstrated

Witness agent: 3

Test code:

```rust
(textual construction; the `% 7` edit was not applied because before's test tree never compiles the target, so a run is green by construction)
```

Command:

```text
grep -n 'fuzz_decode_ops\|fuzz_differential\|fuzz_parse\|fn ' crates/before/tests/fuzz_seeds.rs
```

Output:

```text
31:fn committed_seeds_match_the_live_derivation() {
51:fn seed_directories_hold_exactly_the_set_of_record() {
101:fn decode_seeds_decode_as_named_and_round_trip() {
173:fn differential_seeds_exercise_their_genre_seams() {
254:fn parse_seeds_parse_as_named_and_round_trip() {
342:fn laws_seeds_decode_per_framing_and_stay_wide() {
fuzz_seed_set.rs:266-267: "// `fuzz/fuzz_targets/fuzz_decode_ops.rs` (its `run` carves the value,\n// its `drive_clock` op table is the `% 8` dispatch the script bytes"
fuzz_decode_ops.rs:72:        match op % 8 {
```

Explanation:

Verified by reading. tests/fuzz_seeds.rs holds per-seed contract tests for fuzz_decode (101), fuzz_differential (173), fuzz_parse (254), and the laws seeds (342), plus byte identity (31) and the directory census (51); nothing asserts anything about the two fuzz_decode_ops seeds beyond byte identity, and the `% 8` table is described only in a comment of the derivation (fuzz_seed_set.rs:266-267). The target source lives in the detached `crates/before/fuzz` workspace and is not compiled into any before test binary, so editing its dispatch to `% 7` cannot redden these tests; I did not perform the edit-and-run since the outcome follows from the dependency structure.

## tests-other-27

Outcome: demonstrated

Witness agent: 3

Test code:

```rust
fn scatter_pair() -> (Version, Version) {
    let mut a = Clock::seed();
    let mut b = a.fork();
    let va = a.tick().clone();
    let vb = b.tick().clone();
    (va, vb)
}

fn weave_pair() -> (Version, Version) {
    let mut a = Clock::seed();
    let mut b = a.fork();
    let mut c = a.fork();
    let mut d = b.fork();
    a.tick();
    b.tick();
    c.tick();
    d.tick();
    (a.version().join(c.version()), b.version().join(d.version()))
}

/// tests-other-27: the verdict matrix's weave pair (verbatim copy of
/// `weave_pair` in tests/verdict_matrix.rs) must differ by value from the
/// scatter pair, or the Weave family contributes nothing to the interned pool.
#[test]
fn witness_tests_other_27_weave_pair_differs_from_scatter_pair() {
    let (wa, wb) = weave_pair();
    let (sa, sb) = scatter_pair();
    eprintln!("MEASURED tests_other_27: weave=({wa}, {wb}) scatter=({sa}, {sb})");
    assert_ne!(
        (wa, wb),
        (sa, sb),
        "the weave pair collapses to the scatter pair under ITC normal form"
    );
}
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --success-output immediate -E 'test(/witness_/)' -E 'test(party_join_all_log_factor_is_alive)'
```

Output:

```text
MEASURED tests_other_27: weave=((0, 1, 0), (0, 0, 1)) scatter=((0, 1, 0), (0, 0, 1))

thread 'witness_tests_other_27_weave_pair_differs_from_scatter_pair' (241406466) panicked at crates/before/tests/zz_witness_3.rs:357:5:
assertion `left != right` failed: the weave pair collapses to the scatter pair under ITC normal form
  left: ((0, 1, 0), (0, 0, 1))
```

Explanation:

Ran with verbatim copies of `weave_pair` (verdict_matrix.rs:127-137) and `scatter_pair` (116-122). Both render as ((0, 1, 0), (0, 0, 1)): the join of a.tick and c.tick over sibling quarters of one half normalizes to the half's height-1 plateau, which is exactly the scatter pair's value, so an `Eq`/`Hash` intern dedups the Weave family's smallest organic instance away and the coverage floor at verdict_matrix.rs:1298-1302 (nonemptiness only) cannot see it.

## fuzz-guests-pins-1

Outcome: demonstrated

Witness agent: 3

Test code:

```rust
(textual construction; no test; no push was made)
```

Command:

```text
grep -n 'run: just\|just ' .github/workflows/ci.yml ; grep -n 'wasm32-pins' .github/workflows/ci.yml justfile
```

Output:

```text
ci.yml:108: run: just ci | 172: run: just supply-chain | 178: run: just amp-board-acceptance | 181: run: just worst-cases-pin | 184: run: just surface-totality | 227: run: just coverage-kernel | 230: run: just coverage-kernel-branch
ci.yml grep 'wasm32-pins': (no matches)
justfile:467:    start_stream wasm         10 fuzzfit fuelscape-test wasm32-pins
justfile:1000: ci: fmt-check doclint testdoc workflowlint digestshare readme-check fuelscape-claims mutants-list clippy clippy-default features wasm-check docs docs-internal test-all citecheck doctest bench-build fuzz-build fuelscape-verify viz
ci.yml:117-120: "#   - the wasmtime fuel tier (the fuzzfit bands and the fuelscape pins):\n#     deterministic, but it brings a wasm32 guest build plus wasmtime to\n#     police asymptotics the board legs below already judge at the scales\n#     of record — the gate keeps the second jaw."
justfile:613-614: "# `wasm-check` proves before *compiles* for a 32-bit target; this leg is\n# the tree's one place 32-bit code *executes*: the guest drives the"
```

Explanation:

Verified by reading. CI invokes `just ci`, `supply-chain`, `amp-board-acceptance`, `worst-cases-pin`, `surface-totality`, and the two coverage legs; neither the workflow nor the `ci` recipe (justfile:1000) names `wasm32-pins` (or `fuzzfit`/`fuelscape-test`), while the gate's wasm stream does (justfile:467). The two comments that enumerate what stays local (ci.yml:110-120, justfile:985-986) name only the wall-time judge and the wasmtime fuel tier; wasm32-pins is neither, and the justfile calls it the tree's one place 32-bit code executes (613-614). The push-a-wrong-pin construction was not performed (no pushes).

## fuzz-guests-pins-33

Outcome: demonstrated

Witness agent: 3

Test code:

```rust
(textual construction; no test)
```

Command:

```text
(none)
```

Output:

```text
pins.rs:4-10: "//! Red-first discipline: a boundary found misbehaving is pinned AS FOUND —\n//! the assertion names the trap or wrong value, and the doc comment names\n//! the wrong behavior it stands for — and the commit that engineers the\n//! seam around flips the same test to the correct-value assertion. A pinned\n//! trap is therefore never an accepted behavior: it is a committed bad\n//! baseline its cure must move."
pins.rs:130-131: "/// PINNED AS FOUND: a valid ~1 GiB (1073741817-byte) version encoding\n/// aborts on allocation failure — the doors' one terminal here."
pins.rs:139-140: "/// big-integer backend's 2^32 - 32-bit capacity, so the capacity itself\n/// is unreachable through the doors on this target"
pins.rs:145-146: "/// `rank_decode_past_backend_bit_capacity`.) A leaner working set — not\n/// a wider denomination — is what would move this terminal outward."
pins.rs:371-372: "/// growth inside the re-derivation. A leaner working set — not a wider\n/// denomination — is what would flip this pin to `Value(0)`."
pins.rs:733-735: "/// numerator's limb arm (`rank_decode_past_backend_bit_capacity`). A\n/// leaner working set — not a wider denomination — is what would move\n/// this terminal outward."
harness/src/lib.rs:23-26: "/// A trap is a first-class outcome, not a driver failure: a guest panic\n/// surfaces as the `unreachable` trap under `panic = abort`, and a boundary\n/// found panicking is pinned as exactly that trap until its cure lands, so\n/// the pins assert on this axis directly."
```

Explanation:

Verified by reading the file at the cited ranges (1-12, 128-148, 359-372, 715-735) and harness/src/lib.rs:20-30. The header contracts a pinned trap as a bad baseline awaiting a cure, while the three memory-terminal pins' docs describe the trap as 'the doors' one terminal', argue the backend capacity is unreachable through the doors on this target, and name only a hypothetical 'leaner working set'. Whether any agent note tracks a cure was not searched.

## fuzzfit-strategies-11

Outcome: inconclusive

Witness agent: 3

Test code:

```rust
(not constructed: the mirror's step log and `judge_against` over pinned bands live in the detached fuzzfit workspace, which builds a wasm32 guest and wasmtime; not within the permitted runs)
```

Command:

```text
(none)
```

Output:

```text
ops.rs:610-614: "// The lattice op's rung is canonical equality\n// (a ∨ a = a hands the operand back).\nlet identity = va == *vb;\nself.put(dst, NVal::V(va | vb));\ndone_pair(denom, OK, identity)"
ops.rs:623-626: "// The lattice op's rung is canonical equality, as join.\nlet identity = va == *vb;\nself.put(dst, NVal::V(va & vb));\ndone_pair(denom, OK, identity)"
version.rs:893-898: "if skyline::is_empty_stream(b.0.live()) {\n    return a.clone(); // v ∨ 0 = v\n}\nif skyline::is_empty_stream(a.0.live()) {\n    return b.clone(); // 0 ∨ v = v\n}"
version.rs:916-923: meet_view's `0 ∧ v = 0` and `v ∧ 0 = 0` arms
bands.rs:814: width_below: 2.789563 (ff_version_join); bands.rs:850: width_below: 2.839589
```

Explanation:

Not constructed within the cap. Verified by reading: the mirror's identity predicate for VersionJoin/VersionMeet is `va == *vb` alone (ops.rs:612, 624), while `join_refs`/`join_view` and `meet_view` also settle in O(1) on an empty operand (version.rs:864-878, 889-900, 912-925); the pinned `width_below` values 2.789563 and 2.839589 were confirmed at bands.rs:814 and 850. The claim that the generators routinely produce empty versions and that the empty-operand steps account for the wide band was not measured.

## fuzzfit-bands-17

Outcome: inconclusive

Witness agent: 3

Test code:

```rust
(not constructed: `judge_against` and the pinned `Band`s live in the detached fuzzfit harness crate; building it is outside the permitted runs)
```

Command:

```text
(none)
```

Output:

```text
calibrate.rs:273-290: the floor-evidence loop: "for (&key, f) in &fits {\n    let floor = f.intercept + f.slope * (f.min_denom as f64).log10()\n        - f.width_below\n        - ENFORCE_MARGIN_BELOW;\n    let gap = floor - nop_log;" (an eprintln! follows at 291-297)
bands.rs:194: pub const ENFORCE_MARGIN_BELOW: f64 = 0.8;
bands.rs:690-691 (ff_rank_checked_sub [err]): rejected: true, slope: -0.018256
calibrate.rs:176, 215, 316: small_fits is a separate map; the floor loop at 282 iterates `fits` only
sanity.rs:213-214: "// The dead meter: nop-level fuel on a large case reads Below.\nassert_eq!(judge_against(&band, 1_000_000, 2), Verdict::Below);" (a synthetic `band`)
grep 'nop|floor' tests/enforce.rs: 24 (module doc), 69-80, 169-211 (small-band sub-floor checks); ff_nop called at 251
```

Explanation:

Not constructed within the cap. Verified by reading: the only floor-versus-nop evaluation is the calibrate binary's stderr loop over `fits` at `min_denom` (calibrate.rs:273-297), which walks neither `small_fits` nor the committed `SMALL_BANDS`; the band `ff_rank_checked_sub` [err] has a negative slope (bands.rs:691), so its lowest floor is at max_denom; sanity.rs's dead-meter check uses a synthetic band. I did not read the context of the `ff_nop` call at enforce.rs:251, so whether enforce.rs asserts anything about pinned floors beyond meter liveness is unverified; the finding's `width_below + 0.12` construction was not run.

## fuelscape-pipeline-23

Outcome: inconclusive

Witness agent: 3

Test code:

```rust
(not constructed: measuring an overlay point requires the fuelscape wasm guest in the detached before-fuelscape pipeline)
```

Command:

```text
(none)
```

Output:

```text
families.rs:158-165: "out.extend(ramp(\"dense × self\", max_bytes, |t| {\n    let v = version_bytes(&Shape::Dense.packed1(t));\n    Some(vec![v.clone(), v])\n}));\nout.extend(ramp(\"hugeleaf × self\", max_bytes, |t| {\n    let v = version_bytes(&Shape::Hugeleaf.packed1(8 * t));\n    Some(vec![v.clone(), v])\n}));"
families.rs:91-93: "/// Binary rows pair a family with itself (declared by the label) unless a\n/// committed pair generator exists — `jump_pair`, `tooth_tail`, and\n/// `concurrent_pair` are the pair-shaped families"
version.rs:890-891: "if codec::canonical_eq(&a.0, &b.0) {\n    return a.clone(); // a ∨ a = a"
```

Explanation:

Not constructed within the cap. Verified by reading: the 'dense × self' and 'hugeleaf × self' overlay points feed byte-identical operand pairs (families.rs:158-165), and `join_refs` returns at its first rung `codec::canonical_eq` on equal bytes (version.rs:890-891; `meet_view` likewise at 913-915), so on those rows the marked point measures the equality rung. The [Version, Version] arm carrying both pair generators and self-pairs (155-165) was seen; the fuel comparison against the version_eq row was not measured.

## inventory-1

Outcome: inconclusive

Witness agent: 3

Test code:

```rust
(not constructed: the trigger needs a ~2 GiB right-nested id and 2^32 distinct-minima sites)
```

Command:

```text
(none)
```

Output:

```text
memo.rs:133-139: "pub(super) fn set_link(&mut self, slot: usize, link: Accumulator) {\n    // Push first: the store's length is then provably a valid, nonzero\n    // 1-based index (the `expect` is the u32 capacity contract alone).\n    self.links.push(link);\n    let index = u32::try_from(self.links.len()).expect(\"site count fits u32\");\n    self.queue[slot] = NonZeroU32::new(index);\n}"
prescan.rs:344: "                self.memo.set_link(slot, head);"
prescan.rs:43-45: "//! containers. The ledger's capacity contract is different in kind:\n//! stored-link indices fail loudly at their `u32` cap\n//! ([`Memo::set_link`])."
```

Explanation:

Not constructed within the cap. Verified by reading: `Memo::set_link` converts the running link count with `expect("site count fits u32")` (memo.rs:137), is called once per recorded site with a nonzero link (prescan.rs:344), and the module doc acknowledges the cap as a loud failure rather than a bound derived from input limits (prescan.rs:43-45). Whether the decode doors admit a 2 GiB id and whether `tick`/`ticks` carry a `# Panics` section were not re-read.

## gate-legs-8

Outcome: demonstrated

Witness agent: 3

Test code:

```rust
(textual construction; the doctest deletion and `just gate` were not run)
```

Command:

```text
grep -n -A4 'op: "Rank::encode_to"\|op: "Ranked::encode_to"\|op: "Ranked::encode_rank_to"\|op: "Span::encode_to"' crates/before/src/surface.rs
```

Output:

```text
surface.rs:507-510, 746-749, 786-789, 798-801, 988-991: each of Version::encode_rank_to, Rank::encode_to, Ranked::encode_to, Ranked::encode_rank_to, Span::encode_to carries `Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] })` on prod_tree, prod_fs, and tree_fs
surface.rs:244-247: prod_tree pins: &["encode_to_matches_encode"], prod_fs pins: &[]
codec/tests.rs:812-832: `fn encode_to_matches_encode(...)` exercises party.encode_to, version.encode_to, clock.encode_to only
rank.rs:414-420: the doctest "/// let rank = Version::try_from(5).unwrap().rank();\n/// let mut buf = Vec::new();\n/// rank.encode_to(&mut buf).unwrap();\n/// assert_eq!(buf, rank.encode());"
testing/surface_coverage/tests.rs:214-217: names.extend(*pins) for NoWireFormatInReferences and three sibling exclusions
```

Explanation:

Verified by reading. The five writer-sink rows cite empty pin lists on all three legs; the family doc (surface.rs:78-81) says each writer-sink door is pinned by its doctest, but the coverage suite extends its name set only from `pins` (214-217) and citecheck resolves against the nextest inventory (justfile:289-292), which doctests are outside of; the one named writer test covers Party, Version, and Clock (codec/tests.rs:812-832). The construction (delete rank.rs's example and run `just gate`) was not executed (not permitted).

## deps-2

Outcome: inconclusive

Witness agent: 3

Test code:

```rust
(not constructed: `cargo deny check bans` with and without `multiple-versions-include-dev = true` is not a test run and was not executed)
```

Command:

```text
(none)
```

Output:

```text
deny.toml:18-24: "[bans]\nmultiple-versions = \"deny\"\n...\nskip = []" (no multiple-versions-include-dev key)
Cargo.lock: package `proptest` (1802, version 1.11.0) depends on "rand 0.9.4" and "rand_chacha 0.9.0" (1811-1812); the wezterm-* packages (2941-2976) depend on "thiserror 1.0.69"
Cargo.toml:144-168 ([dev-dependencies] of the root): 150 proptest = { workspace = true }; 168 ratatui = { workspace = true }
crates/before/Cargo.toml:37 and crates/suanpan/Cargo.toml:17: proptest under [dev-dependencies]
justfile:334-336: "# ... cargo-deny holds the root workspace's\n# resolved graph (all members, all features, all targets) to one version\n# per crate; deny.toml is the roster of record, every tolerated duplicate"
```

Explanation:

The lock structure the finding rests on is verified by reading: rand 0.9/rand_chacha 0.9 enter via proptest and thiserror 1 via the wezterm-* crates under ratatui, and proptest and ratatui are dev-dependencies everywhere they appear, while deny.toml sets no `multiple-versions-include-dev`. Whether cargo-deny's default excludes dev-only duplicates was not verified locally (the finding quotes cargo-deny's documentation), and the decisive `cargo deny` runs were not executed, so the enforcement gap is plausible but not demonstrated here.

## party-12

Outcome: demonstrated

Witness agent: 4

Test code:

```rust
// crates/before/tests/zz_witness_4.rs (public API)
fn full_leaf_depth(share: &Party) -> usize {
    let text = share.to_string();
    let mut depth = 0usize;
    let mut found = None;
    for c in text.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            '1' => {
                assert!(found.is_none(), "a share of the seed holds exactly one full leaf: {text}");
                found = Some(depth);
            }
            _ => {}
        }
    }
    found.expect("a share owns a nonempty region")
}

fn ceil_log2(n: u64) -> usize {
    n.next_power_of_two().trailing_zeros() as usize
}

/// party-12: every share `Party::forks(k)` yields (and the residual) is a leaf
/// of a minimal-depth id tree, `⌈log₂(k + 1)⌉` counting the residual; the
/// static `[Party; N]` split's shares sit at depth at most `⌈log₂ N⌉`.
#[test]
fn w4_party12_forks_shares_sit_at_minimal_depth() {
    let mut worst = Vec::new();
    for k in (1u64..=40).chain([63, 64, 65, 100, 127, 128, 129, 1000, 4097]) {
        let mut p = Party::seed();
        let shares: Vec<Party> = p.forks(k).collect();
        assert_eq!(shares.len() as u64, k);
        let bound = ceil_log2(k + 1);
        let deepest = shares
            .iter()
            .chain(core::iter::once(&p))
            .map(full_leaf_depth)
            .max()
            .unwrap();
        worst.push((k, deepest, bound));
        assert!(
            deepest <= bound,
            "forks({k}): a share sits at depth {deepest}, past the minimal depth {bound}"
        );
        p.join_all(shares).expect("shares rejoin");
        assert!(p.is_seed());
    }
    eprintln!("MEASURED forks (k, deepest, bound): {worst:?}");
    fn split<const N: usize>() {
        let shares: [Party; N] = Party::seed().into();
        let bound = ceil_log2(N as u64);
        let deepest = shares.iter().map(full_leaf_depth).max().unwrap();
        eprintln!("MEASURED [Party; {N}]: deepest={deepest} bound={bound}");
        assert!(
            deepest <= bound,
            "[Party; {N}]: a share sits at depth {deepest}, past the minimal depth {bound}"
        );
    }
    split::<2>(); split::<3>(); split::<5>(); split::<7>(); split::<8>(); split::<9>();
    split::<16>(); split::<17>(); split::<31>(); split::<33>(); split::<64>(); split::<100>();
}

// Mutation applied for the second run (crates/before/src/party/forks.rs:57):
// -            let left_count = count.div_ceil(2);
// +            let left_count = count - (count / 4).max(1); // witness4 mutation: 3:1 split
```

Command:

```text
HEAD: cargo nextest run -p before -p suanpan --all-features -E 'test(/w4_/) | test(/witness4_index/) | test(/witness4_uncited_generators_round/)' --success-output immediate
Mutated forks.rs:57: cargo nextest run -p before -p suanpan --all-features --no-fail-fast -E 'test(/fork/) | test(/laws_hold/) | test(/generator_classes/) | test(/doc_hidden/) | test(/roster_order|family_names|every_shape|band_citations|board_roster|envelope_only/) | test(/memo_chain/) | test(/shape/) | test(/laws_seeds/) | test(/w4_party12|witness4_uncited/) | test(/every_family_answers/)'
```

Output:

```text
HEAD:
        PASS [   0.011s] (7/8) before::zz_witness_4 w4_party12_forks_shares_sit_at_minimal_depth
    MEASURED forks (k, deepest, bound): [(1, 1, 1), (2, 2, 2), (3, 2, 2), (4, 3, 3), ... (63, 6, 6), (64, 7, 7), (65, 7, 7), (100, 7, 7), (127, 7, 7), (128, 8, 8), (129, 8, 8), (1000, 10, 10), (4097, 13, 13)]
    MEASURED [Party; 100]: deepest=7 bound=7

Under the 3:1 mutation:
        PASS [   0.009s] (28/49) before::forks_max clock_forks_max_saturates_without_panic
        PASS [   0.006s] (29/49) before::forks_max party_forks_max_saturates_without_panic
        PASS [   0.102s] (45/49) before party::tests::d_fork_join_roundtrip
        PASS [   0.425s] (48/49) before testing::algebraic_laws::tests::laws_hold_on_organic_populations
        FAIL [   0.005s] (37/49) before::zz_witness_4 w4_party12_forks_shares_sit_at_minimal_depth
    thread 'w4_party12_forks_shares_sit_at_minimal_depth' (241347371) panicked at crates/before/tests/zz_witness_4.rs:51:9:
    forks(3): a share sits at depth 3, past the minimal depth 2
     Summary [   0.579s] 49 tests run: 46 passed, 3 failed, 909 skipped
```

Explanation:

Verified by running. At HEAD every share of forks(k) and of [Party; N] sits at exactly the minimal depth for every k tried up to 4097. With forks.rs:57 changed to a 3:1 split, every committed test whose name matches fork/forks, the organic law population driver (laws_hold_on_organic_populations, which runs forks_matches_from_array and forks_partial_drop_folds_back), and both forks_max pins stay green; the only failures in the 49-test run are the three scratch witnesses (this one, plus the two meter-core-2 witnesses that fail for their own reasons). The balance property has no committed instrument; the scratch test above is one. The fuelscape island and the ff_party_forks band were not run (detached workspace, out of my permitted commands).

## party-25

Outcome: demonstrated

Witness agent: 4

Test code:

```rust
// appended to crates/before/src/party/tests.rs
#[cfg(feature = "scan-meter")]
#[test]
fn witness4_index_searches_fire_on_left_only_input_nodes() {
    for d in [8usize, 64, 512] {
        // a = L_d: L_0 = (1, 0), L_{k+1} = (L_k, 1).
        let mut a = String::from("(1, 0)");
        for _ in 0..d {
            a = format!("({a}, 1)");
        }
        // b = d left-only nodes over one right-only node over a terminal.
        let mut b = String::from("(0, 1)");
        for _ in 0..d {
            b = format!("({b}, 0)");
        }
        let a: Party = a.parse().unwrap();
        let b: Party = b.parse().unwrap();
        assert!(a.is_disjoint(&b), "the pair is disjoint by construction");
        let index = IdIndex::build(a.as_bits());
        crate::codec::scan::reset();
        assert!(index.is_disjoint(b.view()));
        let indexed = crate::codec::scan::scan_bits();
        crate::codec::scan::reset();
        assert!(a.view().is_disjoint(b.view()));
        let walk = crate::codec::scan::scan_bits();
        // The indexed walk's tag reads: one 2-bit input read per visited pair
        // (d + 2 pairs) and one 2-bit indexed-side read per present pair (d + 1).
        let tag_bits = (2 * (d + 2) + 2 * (d + 1)) as u64;
        let excess = indexed.saturating_sub(tag_bits);
        eprintln!(
            "MEASURED d={d}: indexed scan bits={indexed} (tag reads ~{tag_bits}, excess {excess} = {} 32-bit probes); cursor walk scan bits={walk}",
            excess / 32
        );
        assert!(
            indexed >= tag_bits + 32 * (d as u64 - 1),
            "d={d}: the indexed test read {indexed} bits, no room for the d-1 searches the code runs on left-only input nodes"
        );
    }
}
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features -E 'test(/w4_/) | test(/witness4_index/) | test(/witness4_uncited_generators_round/)' --success-output immediate
```

Output:

```text
        PASS [   0.007s] (5/8) before party::tests::witness4_index_searches_fire_on_left_only_input_nodes
    MEASURED d=8: indexed scan bits=454 (tag reads ~38, excess 416 = 13 32-bit probes); cursor walk scan bits=56
    MEASURED d=64: indexed scan bits=8710 (tag reads ~262, excess 8448 = 264 32-bit probes); cursor walk scan bits=392
    MEASURED d=512: indexed scan bits=117094 (tag reads ~2054, excess 115040 = 3595 32-bit probes); cursor walk scan bits=3080
```

Explanation:

Verified by running under scan-meter. The input b has zero both-present nodes (every node is left-only, then one right-only, then a terminal), so the module doc's bound O(Σ inputs + B log n) with B = 0 predicts no table searches; the indexed walk nevertheless records 3595 32-bit table probes at d = 512 (115040 bits beyond the tag reads), 38x the cursor walk's 3080 bits on the same pair. This matches the code at index.rs:220-230, which runs metered_partition_point whenever the indexed node is both-present and the input node is Internal, and the (true, false) arm at 244-246 discards a_right/right_entry.

## rank-32

Outcome: demonstrated

Witness agent: 4

Test code:

```rust
// crates/before/tests/zz_witness_4.rs
#[cfg(feature = "limb-meter")]
#[test]
fn w4_rank32_encode_rank_costs_exactly_rank_encode() {
    use before::meter::registry::Shape;
    use before::{meter, Ranked};
    let versions: Vec<(&str, Version)> = vec![
        ("text", "(1, 2, (0, (1, 0, 2), 0))".parse().unwrap()),
        ("leaf 5", Version::try_from(5).unwrap()),
        ("empty", Version::new()),
        ("bigroot(100, 50)", Shape::Bigroot.packed2(100, 50).version()),
        ("cliff_comb(64, 32)", Shape::CliffComb.packed2(64, 32).version()),
        ("dense(200)", Shape::Dense.packed1(200).version()),
        ("hugeleaf(300)", Shape::Hugeleaf.packed1(300).version()),
    ];
    for (name, v) in &versions {
        meter::reset_limb_ops();
        let via_rank = v.rank().encode();
        let rank_ops = meter::limb_ops();
        meter::reset_limb_ops();
        let via_ranked = Ranked::from(v).encode_rank();
        let ranked_ops = meter::limb_ops();
        meter::reset_limb_ops();
        let via_version = v.encode_rank();
        let version_ops = meter::limb_ops();
        eprintln!(
            "MEASURED {name}: rank().encode() limb_ops={rank_ops}; Ranked::encode_rank limb_ops={ranked_ops}; Version::encode_rank limb_ops={version_ops}"
        );
        assert_eq!(via_rank, via_ranked);
        assert_eq!(via_rank, via_version);
        assert_eq!(
            rank_ops, ranked_ops,
            "{name}: the 'fused' emission records a different limb count than rank().encode()"
        );
    }
}
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features -E 'test(/w4_/) | test(/witness4_index/) | test(/witness4_uncited_generators_round/)' --success-output immediate
```

Output:

```text
        PASS [   0.006s] (3/8) before::zz_witness_4 w4_rank32_encode_rank_costs_exactly_rank_encode
    MEASURED text: rank().encode() limb_ops=7; Ranked::encode_rank limb_ops=7; Version::encode_rank limb_ops=7
    MEASURED leaf 5: rank().encode() limb_ops=7; Ranked::encode_rank limb_ops=7; Version::encode_rank limb_ops=7
    MEASURED empty: rank().encode() limb_ops=5; Ranked::encode_rank limb_ops=5; Version::encode_rank limb_ops=5
    MEASURED bigroot(100, 50): rank().encode() limb_ops=16; Ranked::encode_rank limb_ops=16; Version::encode_rank limb_ops=16
    MEASURED cliff_comb(64, 32): rank().encode() limb_ops=20; Ranked::encode_rank limb_ops=20; Version::encode_rank limb_ops=20
    MEASURED dense(200): rank().encode() limb_ops=7; Ranked::encode_rank limb_ops=7; Version::encode_rank limb_ops=7
    MEASURED hugeleaf(300): rank().encode() limb_ops=28; Ranked::encode_rank limb_ops=28; Version::encode_rank limb_ops=28
```

Explanation:

Verified by running under limb-meter: on seven versions spanning the empty version, a wide root, a cliff comb, a deep spine, and a 300-bit leaf, `v.rank().encode()`, `Ranked::from(&v).encode_rank()`, and `v.encode_rank()` record identical limb counts and emit identical bytes. Read alongside ranked.rs:211-215 (`let rank = self.version.rank(); ... encode_parts(num, exp)`) and rank.rs:395-397 (`encode_parts(&self.num, self.exp)`), the three spellings are the same computation; the 'more efficient' / 'without materializing the intermediate Rank' prose describes work the code does not save.

## skyline-coding-6

Outcome: demonstrated

Witness agent: 4

Test code:

```rust
// crates/before/tests/zz_witness_4.rs
#[test]
fn w4_skyline6_span_decode_rejects_mid_stream_collapsible_pair() {
    let lo = Version::new().encode();
    assert_eq!(lo, vec![0xE0], "the empty version's wire form");
    // Root internal, left internal; leaves: gamma(0), zigzag(0) (the
    // collapsible pair under the left child), then zigzag(+1) as the root's
    // right leaf, marker-padded.
    let hi: Vec<u8> = vec![0x3E, 0xE0];
    let component = Version::decode(&hi[..]);
    assert!(
        matches!(component, Err(Decode::NotCanonical)),
        "the hi component alone is structurally whole and non-canonical: {component:?}"
    );
    let composite = [lo, hi].concat();
    let span = Span::decode(&composite[..]);
    assert!(
        matches!(span, Err(Decode::NotCanonical)),
        "the admission walk must reject the mid-stream collapsible pair: {span:?}"
    );
    // The root-level twin the committed suites use closes in `finish()`:
    let root_level: Vec<u8> = [Version::new().encode(), vec![0b0111_1000]].concat();
    assert!(matches!(Span::decode(&root_level[..]), Err(Decode::NotCanonical)));
}

// Mutation applied for the second run (crates/before/src/version/skyline/admit.rs:173, inside step() only; finish() at 213 untouched):
// -        let mut zero_delta = self.last_delta_zero;
// +        let mut zero_delta = false; // witness4 mutation: the step arm never sees a zero delta
```

Command:

```text
HEAD: cargo nextest run -p before -p suanpan --all-features -E 'test(/w4_|witness4_/)' --no-fail-fast
Mutated admit.rs:173: cargo nextest run -p before -p suanpan --all-features --no-fail-fast -E 'test(/span|borsh|admit|collapsible|canonical|seeds/) | test(/sweep|causal|cmp|order|exit/) | test(/verdict|matrix|polarity|equality_inverting/) | test(/w4_skyline6|w4_masked32/)'
```

Output:

```text
HEAD:
        PASS [   0.006s] ( 2/11) before::zz_witness_4 w4_skyline6_span_decode_rejects_mid_stream_collapsible_pair

Under the step-arm mutation (the same run also carried the order_exit and verdict_matrix mutations of masked-32 / tests-other-30):
        PASS [   0.006s] ( 13/227) before borsh_impls::tests::span_borsh_rejects_collapsible_join
        PASS [   0.063s] ( 32/227) before borsh_impls::tests::span_wire_decode_matches_bitwise_reference
        PASS [   0.066s] (146/227) before version::skyline::tests::planted_collapsible_pairs_are_rejected_at_every_leaf
        PASS [   0.012s] (147/227) before version::skyline::tests::rejects_non_root_collapsible_pair
        PASS [   0.012s] (172/227) before::fuzz_seeds laws_seeds_decode_per_framing_and_stay_wide
        FAIL [   0.012s] (208/227) before::zz_witness_4 w4_skyline6_span_decode_rejects_mid_stream_collapsible_pair
    thread 'w4_skyline6_span_decode_rejects_mid_stream_collapsible_pair' (241370272) panicked at crates/before/tests/zz_witness_4.rs:140:5:
    the admission walk must reject the mid-stream collapsible pair: Ok(Span { lo: 0, hi: (0, (0, 0, 0), 1) })
     Summary [   5.176s] 227 tests run: 224 passed (1 leaky), 3 failed, 731 skipped
```

Explanation:

Verified by running. The composite [0xE0, 0x3E, 0xE0] decodes at HEAD to Err(NotCanonical) through Span::decode, and its hi component alone is rejected by Version::decode as NotCanonical (structurally whole). With the step arm's zero_delta forced false, Span::decode accepts the composite as `Span { lo: 0, hi: (0, (0, 0, 0), 1) }` — a span whose hi carries the collapsible pair (0, 0, 0) — while every committed span, borsh, validator, and fuzz-seed test in the 227-test run stays green (the other two failures in that run belong to the masked-32 pin and the coincident_span test, both caused by the order_exit mutation, not this one). The mid-stream close-out arm at admit.rs:174-180 has no committed witness; the scratch test is one.

## skyline-sweep-place-masked-5

Outcome: demonstrated

Witness agent: 4

Test code:

```rust
// Instrumentation (scratch) in crates/before/src/codec/stack.rs and codec.rs:
// +/// witness4 scratch: spilled words read by `trailing_ones` (never committed).
// +#[cfg(test)]
// +pub(crate) static TRAILING_WORDS_READ: core::sync::atomic::AtomicU64 =
// +    core::sync::atomic::AtomicU64::new(0);
// ... inside BitStack::trailing_ones:
//          for &word in self.words.iter().rev() {
// +            #[cfg(test)]
// +            TRAILING_WORDS_READ.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
//              let w = word.trailing_ones();
// codec.rs: +#[cfg(test)] pub(crate) use stack::TRAILING_WORDS_READ;

// appended to crates/before/src/version/skyline/masked/tests.rs
#[test]
fn witness4_parked_cursor_peek_flip_rescans_trailing_run() {
    use core::sync::atomic::Ordering;

    use crate::codec::TRAILING_WORDS_READ;
    use crate::{Party, Version};

    // root = (0, L_1, 0); L_k = (0, 1, L_{k+1}); L_{r+1} = `bottom`.
    fn spine(r: usize, bottom: &str) -> String {
        let mut t = bottom.to_string();
        for _ in 0..r {
            t = format!("(0, 1, {t})");
        }
        format!("(0, {t}, 0)")
    }
    // n leaves of alternating heights 1, 0, 1, ... under a right comb, terminal 0.
    fn comb(n: usize) -> String {
        let mut t = String::from("0");
        for i in (1..n).rev() {
            let h = if i % 2 == 1 { 1 } else { 0 };
            t = format!("(0, {h}, {t})");
        }
        t
    }
    let p: Party = "(0, 1)".parse().unwrap(); // unowned on [0, 1/2)
    let mut rows = Vec::new();
    for s in [4096usize, 8192] {
        let (r, n) = (s, s);
        let v: Version = spine(r, "0").parse().unwrap();
        let w: Version = spine(r, &comb(n)).parse().unwrap();
        let input_bits = v.encoded_bits() + w.encoded_bits() + p.encoded_bits();
        TRAILING_WORDS_READ.store(0, Ordering::Relaxed);
        let verdict = (&v / &p).partial_cmp(&w);
        let words = TRAILING_WORDS_READ.load(Ordering::Relaxed);
        eprintln!(
            "MEASURED r=n={s}: input_bits={input_bits} spilled words read by trailing_ones={words} verdict={verdict:?}"
        );
        rows.push((input_bits, words));
    }
    let ((b1, w1), (b2, w2)) = (rows[0], rows[1]);
    let input_ratio = b2 as f64 / b1 as f64;
    let words_ratio = w2 as f64 / w1 as f64;
    eprintln!("MEASURED input x{input_ratio:.3}, trailing-run words x{words_ratio:.3}");
    assert!(
        words_ratio < 2.5,
        "the parked cursor's trailing-run reads grow x{words_ratio:.2} across an input doubling of x{input_ratio:.2}: quadratic"
    );
}
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features -E 'test(/w4_|witness4_/)' --no-fail-fast
```

Output:

```text
        FAIL [   0.067s] (11/11) before version::skyline::masked::tests::witness4_parked_cursor_peek_flip_rescans_trailing_run
    MEASURED r=n=4096: input_bits=45071 spilled words read by trailing_ones=262144 verdict=Some(Less)
    MEASURED r=n=8192: input_bits=90127 spilled words read by trailing_ones=1048576 verdict=Some(Less)
    MEASURED input x2.000, trailing-run words x4.000
    thread 'version::skyline::masked::tests::witness4_parked_cursor_peek_flip_rescans_trailing_run' (241317182) panicked at crates/before/src/version/skyline/masked/tests.rs:123:5:
    the parked cursor's trailing-run reads grow x4.00 across an input doubling of x2.00: quadratic
```

Explanation:

Verified by running with a scratch counter on the spilled-word loop of BitStack::trailing_ones. On the finding's family (masked operand parked at path [0, 1^r] under an unowned mask while the unmasked operand advances through n leaves inside that interval), the spilled words read are exactly n*r/64 (4096*4096/64 = 262144; 8192*8192/64 = 1048576): x4.000 on an input that grows x2.000 (45071 -> 90127 bits). The lockstep spine phase contributes nothing because those leaves' paths end in a left branch (trailing run zero), so the whole reading is the parked-cursor re-scan the finding describes. This is quadratic work invisible to the scan, limb, heap, and segment meters, on the production OwnVersion comparison path.

## skyline-sweep-place-masked-32

Outcome: demonstrated

Witness agent: 4

Test code:

```rust
// crates/before/tests/zz_witness_4.rs
fn order_exit_pair(t: usize) -> (Version, Version) {
    let mut tail = String::from("0");
    for _ in 0..t {
        tail = format!("(0, 0, (0, 5, {tail}))");
    }
    let a: Version = format!("(0, 1, {tail})").parse().unwrap();
    let b: Version = "(0, 0, 7)".parse().unwrap();
    (a, b)
}

#[cfg(all(feature = "limb-meter", feature = "scan-meter"))]
#[test]
fn w4_masked32_order_exit_readings_are_tail_independent() {
    use before::meter;
    use suanpan::touch_meter;
    let p: Party = "1".parse().unwrap();
    let mut readings: Vec<[(u64, u64); 3]> = Vec::new();
    for t in [2_000usize, 4_000] {
        let (a, b) = order_exit_pair(t);
        let bytes = a.encode().len() + b.encode().len();
        let mut row = [(0u64, 0u64); 3];
        touch_meter::reset();
        meter::reset_scan_bits();
        let v = a.partial_cmp(&b);
        row[0] = (touch_meter::touches(), meter::scan_bits());
        assert_eq!(v, None, "the pair is concurrent");
        touch_meter::reset();
        meter::reset_scan_bits();
        let v = (&a / &p).partial_cmp(&b);
        row[1] = (touch_meter::touches(), meter::scan_bits());
        assert_eq!(v, None);
        touch_meter::reset();
        meter::reset_scan_bits();
        let v = (&a / &p).partial_cmp(&(&b / &p));
        row[2] = (touch_meter::touches(), meter::scan_bits());
        assert_eq!(v, None);
        eprintln!(
            "MEASURED t={t} ({bytes} operand bytes): plain (touches, scan)={:?} one-mask={:?} two-mask={:?}",
            row[0], row[1], row[2]
        );
        readings.push(row);
    }
    for (i, name) in ["causal_cmp", "masked one-sided", "masked two-sided"].iter().enumerate() {
        assert_eq!(
            readings[0][i], readings[1][i],
            "{name}: readings moved with the refuted tail (t -> 2t): the exit is not tail-independent"
        );
    }
}

// Mutation applied for the second run (crates/before/src/version/skyline/sweep.rs, order_exit body):
// -    if !directions.le && !directions.ge {
// -        ControlFlow::Break(None)
// -    } else {
// -        ControlFlow::Continue(())
// -    }
// +    let _ = directions; // witness4 mutation: never exit early
// +    ControlFlow::Continue(())
```

Command:

```text
HEAD: cargo nextest run -p before -p suanpan --all-features -E 'test(/w4_/) | test(/witness4_index/) | test(/witness4_uncited_generators_round/)' --success-output immediate
Mutated order_exit: cargo nextest run -p before -p suanpan --all-features --no-fail-fast -E 'test(/span|borsh|admit|collapsible|canonical|seeds/) | test(/sweep|causal|cmp|order|exit/) | test(/verdict|matrix|polarity|equality_inverting/) | test(/w4_skyline6|w4_masked32/)'
```

Output:

```text
HEAD:
        PASS [   0.012s] (8/8) before::zz_witness_4 w4_masked32_order_exit_readings_are_tail_independent
    MEASURED t=2000 (4504 operand bytes): plain (touches, scan)=(5, 21) one-mask=(6, 23) two-mask=(8, 25)
    MEASURED t=4000 (9004 operand bytes): plain (touches, scan)=(5, 21) one-mask=(6, 23) two-mask=(8, 25)

Under order_exit -> always Continue:
    MEASURED t=2000 (4504 operand bytes): plain (touches, scan)=(8005, 36020) one-mask=(8006, 36022) two-mask=(12008, 36024)
    MEASURED t=4000 (9004 operand bytes): plain (touches, scan)=(16005, 72020) one-mask=(16006, 72022) two-mask=(24008, 72024)
    thread 'w4_masked32_order_exit_readings_are_tail_independent' (241370271) panicked at crates/before/tests/zz_witness_4.rs:223:9:
    assertion `left == right` failed: causal_cmp: readings moved with the refuted tail (t -> 2t): the exit is not tail-independent
      left: (8005, 36020)
     right: (16005, 72020)
        PASS [   3.984s] (225/227) before::verdict_matrix production_verdicts_agree_across_the_surfaces
        PASS [   4.005s] (226/227) before::verdict_matrix equality_inverting_sweep_reads_inverted_through_the_matrix
        PASS [   4.003s] (227/227) before::verdict_matrix polarity_flipped_sweep_reads_inverted_through_the_matrix
        FAIL [   0.013s] (165/227) before::coincident_span coincident_place_collapses_to_the_pair_sweep
    thread 'coincident_place_collapses_to_the_pair_sweep' (241370119) panicked at crates/before/tests/coincident_span.rs:83:5:
    coincident endpoints in distinct buffers must take the fused three-stream walk (101 vs 116 scanned bits): the clone rung must not fire across buffers
```

Explanation:

Verified by running. At HEAD the production causal_cmp and both masked exits read tail-independent absolutes (touches/scan identical at t and 2t), so the pin the finding asks for is constructible and passes today. With order_exit always continuing, the readings become tail-linear for all three entry points (8005 -> 16005 touches, 36020 -> 72020 scan bits) while every verdict suite in the run — verdict_matrix production and both twins, the sweep/cmp tests, the span suites — stays green. One caveat against the finding's literal 'every verdict suite stays green': tests/coincident_span.rs::coincident_place_collapses_to_the_pair_sweep does go red under this mutation, but only through its relative inequality (the pair sweep's 116 scanned bits now exceed the fused walk's 101), an incidental catch rather than a pin of the exit; no committed test pins the masked exits or the production order_exit directly.

## clock-17

Outcome: inconclusive

Witness agent: 4

Test code:

```rust
// Not constructed: no 32-bit target can run here. The environment has the
// wasm32-unknown-unknown target installed but no wasmtime/wasmer binary, and
// building the wasm32-pins harness is outside the permitted commands.
// Mechanical check performed instead (toolchain 1.97.1 core source):
//   awk 'NR>=112 && NR<=124' $(rustc --print sysroot)/lib/rustlib/src/rust/library/core/src/iter/traits/exact_size.rs
```

Command:

```text
awk 'NR>=112 && NR<=124 {printf "%6d\t%s\n", NR, $0}' "$(rustc --print sysroot)/lib/rustlib/src/rust/library/core/src/iter/traits/exact_size.rs"; rustc --version
```

Output:

```text
   116	    fn len(&self) -> usize {
   117	        let (lower, upper) = self.size_hint();
   118	        // Note: This assertion is overly defensive, but it checks the invariant
   119	        // guaranteed by the trait. If this trait were rust-internal,
   120	        // we could use debug_assert!; assert_eq! will check all Rust user
   121	        // implementations too.
   122	        assert_eq!(upper, Some(lower));
   123	        lower
   124	    }
rustc 1.97.1 (8bab26f4f 2026-07-14)
```

Explanation:

Not constructed within the cap: the panic is 32-bit-only and no 32-bit runtime is available to this worktree (wasmtime/wasmer absent; the wasm32-pins harness build is not among my permitted commands). Assessed by reading: Split::size_hint (party/forks.rs:65-73) returns `(usize::try_from(remaining).unwrap_or(usize::MAX), usize::try_from(remaining).ok())`, i.e. (usize::MAX, None) once remaining exceeds usize; `impl ExactSizeIterator for Split {}` (forks.rs:76), `for Forks<'_> {}` (forks.rs:132) and clock/forks.rs:57 all take the default len, which the pinned toolchain's core source asserts `upper == Some(lower)` on. The chain to a panic from `Clock::seed().forks(1u64 << 32).len()` on wasm32 is consistent with the code, but I did not execute it; tests/forks_max.rs:26 and :49 are explicitly 64-bit-only ('64-bit test host').

## codec-bits-29

Outcome: demonstrated

Witness agent: 4

Test code:

```rust
// Same scratch instrumentation and test as skyline-sweep-place-masked-5:
// TRAILING_WORDS_READ counter in BitStack::trailing_ones' spilled-word loop and
// witness4_parked_cursor_peek_flip_rescans_trailing_run in
// crates/before/src/version/skyline/masked/tests.rs (code quoted verbatim under
// skyline-sweep-place-masked-5). The `to_version()` projection variant was not
// constructed (one family suffices for the shared claim).
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features -E 'test(/w4_|witness4_/)' --no-fail-fast
```

Output:

```text
    MEASURED r=n=4096: input_bits=45071 spilled words read by trailing_ones=262144 verdict=Some(Less)
    MEASURED r=n=8192: input_bits=90127 spilled words read by trailing_ones=1048576 verdict=Some(Less)
    MEASURED input x2.000, trailing-run words x4.000
    thread 'version::skyline::masked::tests::witness4_parked_cursor_peek_flip_rescans_trailing_run' (241317182) panicked at crates/before/src/version/skyline/masked/tests.rs:123:5:
    the parked cursor's trailing-run reads grow x4.00 across an input doubling of x2.00: quadratic
```

Explanation:

Verified by running (shared with masked-5): the finding's Version/Party/Version construction at r = n = 4096 and 8192 reads 262144 and 1048576 spilled words in trailing_ones — exactly n*r/64 — against input bits of 45071 and 90127, a x4 growth on a x2 input. stack.rs:106-107's pricing ('the run the caller is about to pop (or has decided not to)') is correct per call; the omission is that block_skip evaluates `self.a.peek_flip()` on every advance of the other operand while a is parked (masked.rs:320-321), so the per-call price is paid Theta(n) times without a pop. The projection-loop variant (`(&v / &p2).to_version()`) was not constructed; the masked comparison alone establishes the non-popping reads the linear derivation at masked.rs:52-55 omits.

## crate-root-32

Outcome: demonstrated

Witness agent: 4

Test code:

```rust
// crates/before/tests/zz_witness_4.rs (an integration binary: cfg(test) is off for the library)
#[test]
fn w4_segments_counter_has_no_writer_in_this_binary() {
    fn deep(n: usize) -> usize {
        if n == 0 {
            0
        } else {
            1 + std::hint::black_box(deep(n - 1))
        }
    }
    before::meter::reset_stack_segments();
    let frames = stacker::grow(256 << 20, || deep(200_000));
    assert_eq!(frames, 200_000);
    let segments = before::meter::stack_segments();
    eprintln!("MEASURED segments after stacker::grow over a 200_000-frame recursion: {segments}");
    assert_eq!(
        segments, 0,
        "the counter moved: some writer exists in this binary after all"
    );
}

// Mechanical check: grep -rn "descend!" crates/before/src | grep -v recurse.rs
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features -E 'test(/w4_/) | test(/witness4_index/) | test(/witness4_uncited_generators_round/)' --success-output immediate
grep -rn "descend!" crates/before/src | grep -v "^crates/before/src/recurse.rs"
```

Output:

```text
        PASS [   0.007s] (6/8) before::zz_witness_4 w4_segments_counter_has_no_writer_in_this_binary
    MEASURED segments after stacker::grow over a 200_000-frame recursion: 0

crates/before/src/testing/bridge.rs:56:            descend!(0, emit_ev(out, l));
crates/before/src/testing/bridge.rs:57:            descend!(0, emit_ev(out, r));
crates/before/src/testing/bridge.rs:150:        let (l, after_l) = descend!(0, read_ev(bits, pos + 1, prev));
crates/before/src/testing/bridge.rs:151:        let (r, after_r) = descend!(0, read_ev(bits, after_l, prev));
crates/before/src/version/skyline/grow/tests.rs:125:    let cost = descend!(
crates/before/src/version/skyline/grow/tests.rs:172:            let lc = descend!(
crates/before/src/version/skyline/grow/tests.rs:177:            let rc = descend!(
crates/before/src/meter/tests.rs:417:        let below = descend!(depth + 1, dive(depth + 1, floor));
```

Explanation:

Verified by running in an integration-test binary built with --all-features: a genuine heap stack segment grown through `stacker::grow` under a 200 000-frame recursion leaves `meter::stack_segments()` at 0, because the only `fetch_add` on SEGMENTS_GROWN lives in `recurse::grow`, which is `#[cfg(test)]` (recurse.rs:100-109) and unreachable from a binary that links the library without cfg(test). The grep confirms every `descend!` call site is a test surface (testing/bridge.rs, grow/tests.rs, meter/tests.rs), so no library kernel can bump the counter in any build. The finding's second construction (deleting the liveness test body and running the whole meter suite plus amp-board acceptance) was not run: the board and the whole suite are outside my cap.

## suanpan-24

Outcome: inconclusive

Witness agent: 4

Test code:

```rust
// Not constructed: 32-bit release semantics cannot be exercised on this
// 64-bit host (no 32-bit target/runtime available within the permitted
// commands). Read-only verification of the cited lines instead:
//   awk 'NR>=1467 && NR<=1489' crates/suanpan/src/accumulator.rs
//   grep -n profile Cargo.toml
```

Command:

```text
awk 'NR>=315 && NR<=325 || NR>=560 && NR<=570 || NR>=1325 && NR<=1332 || NR>=1365 && NR<=1378 || NR>=1460 && NR<=1490 {printf "%6d\t%s\n", NR, $0}' crates/suanpan/src/accumulator.rs; grep -n profile Cargo.toml
```

Output:

```text
  1470	        let digit_shift = usize::try_from(digit_shift).expect("digit positions fit a usize");
  1471	        for (limb_index, limb) in limbs.enumerate() {
  1477	            if low != 0 {
  1478	                self.add_at(
  1479	                    2 * limb_index + digit_shift,
  1483	            if high != 0 {
  1484	                self.add_at(
  1485	                    2 * limb_index + 1 + digit_shift,
   565	                self.add_at(
   566	                    offset + digit_shift,
  1329	            position += 1;
  1370	            if pos >= self.digits.len() {
  1371	                self.digits.resize(pos + 1, 0);
174:[profile.dev]
183:[profile.dev.package.before]
186:[profile.dev.package.suanpan]
191:[profile.bench]
```

Explanation:

Not constructed within the cap: the wrap is a 32-bit release-profile behavior and this worktree has only the 64-bit host toolchain plus a wasm32 target with no runtime. Assessed by reading: the only checked conversion is `shift / 32` at accumulator.rs:1470 (and 560); the landing positions at 1479, 1485, 566, and the `position += 1` at 1329 are plain usize additions; zero contributions are skipped at 1477/1483 before any `add_at`/`resize`, so a low neighbourhood of zero limbs reaches the wrapped add with no allocation in between; the workspace Cargo.toml declares [profile.dev] and [profile.bench] but no [profile.release], so release overflow checks are off by default. The arithmetic in the finding's three cases is consistent with those lines; the runtime effect was not observed.

## meter-core-2

Outcome: demonstrated

Witness agent: 4

Test code:

```rust
// appended to crates/before/src/meter/tests.rs
#[test]
fn witness4_uncited_generator_closed_forms_match_their_docs() {
    let mut mismatches = Vec::new();
    let mut check = |name: &str, params: String, got: usize, doc: usize| {
        eprintln!("MEASURED {name}({params}): bits={got} doc={doc}");
        if got != doc {
            mismatches.push(format!("{name}({params}): emitted {got}, doc says {doc}"));
        }
    };
    for (b, d) in [(1usize, 1usize), (3, 2), (10, 7)] {
        check("wide_tail", format!("{b}, {d}"), super::wide_tail(b, d).bits, 4 * d + 2 * b + 3);
    }
    for k in [1usize, 2, 5] {
        check("memo_chain", format!("{k}, false"), super::memo_chain(k, false).bits, 13 * k + 9);
    }
    for d in [1usize, 2, 5] {
        check("memo_comb_id", format!("{d}"), super::memo_comb_id(d).bits, 14 * d + 12);
        check("memo_churn_id", format!("{d}"), super::memo_churn_id(d).bits, 14 * d + 6);
    }
    // The approximate docs, reported against the finding's derivations.
    for d in [1usize, 6, 7, 20] {
        let p = super::staircase(d);
        eprintln!(
            "MEASURED staircase({d}): bits={} finding 6d+2={} capacity hint 5d+8={}",
            p.bits,
            6 * d + 2,
            5 * d + 8
        );
        assert_eq!(p.bits, 6 * d + 2, "staircase({d}) is not 6d + 2");
    }
    for (k, b) in [(1usize, 1usize), (2, 3), (4, 5)] {
        let p = super::memo_fanout(k, b);
        eprintln!(
            "MEASURED memo_fanout({k}, {b}): bits={} finding k(4b+6)+6={} doc ~13k+2kb+9={}",
            p.bits,
            k * (4 * b + 6) + 6,
            13 * k + 2 * k * b + 9
        );
        assert_eq!(p.bits, k * (4 * b + 6) + 6, "memo_fanout({k}, {b}) is not k(4b+6)+6");
    }
    assert!(
        mismatches.is_empty(),
        "exact closed forms disagree with the generators:\n{}",
        mismatches.join("\n")
    );
}

#[test]
fn witness4_uncited_generators_round_trip_through_strict_decode() {
    for (name, p) in [
        ("nested_full_id(3)", super::nested_full_id(3)),
        ("nested_left_full_id(3)", super::nested_left_full_id(3)),
        ("memo_chain_id(3)", super::memo_chain_id(3)),
        ("memo_comb_id(3)", super::memo_comb_id(3)),
        ("memo_churn_id(3)", super::memo_churn_id(3)),
        ("descending_raises_id(3)", super::descending_raises_id(3)),
        ("reveal_comb_id(3)", super::reveal_comb_id(3)),
        ("pure_comb_id(3)", super::pure_comb_id(3)),
        ("ascend_cliff_id(3)", super::ascend_cliff_id(3)),
    ] {
        eprintln!("checking id {name}: {} bits", p.bits);
        check_party(&p, p.bits);
    }
    for (name, p) in [
        ("wide_tail(3, 2)", super::wide_tail(3, 2)),
        ("staircase(5)", super::staircase(5)),
        ("memo_chain(3, true)", super::memo_chain(3, true)),
        ("memo_chain(3, false)", super::memo_chain(3, false)),
        ("memo_comb(3)", super::memo_comb(3)),
        ("memo_fanout(3, 3)", super::memo_fanout(3, 3)),
        ("memo_oscillating(3, 3)", super::memo_oscillating(3, 3)),
        ("memo_churn(3)", super::memo_churn(3)),
        ("descending_raises(3)", super::descending_raises(3)),
        ("reveal_comb(3, 3)", super::reveal_comb(3, 3)),
        ("reveal_comb_hifloor(3, 3)", super::reveal_comb_hifloor(3, 3)),
        ("pure_comb(3, 3)", super::pure_comb(3, 3)),
    ] {
        eprintln!("checking event {name}: {} bits", p.bits);
        check_version(&p, p.bits);
    }
}

// Mutation applied for the second run (crates/before/src/meter.rs:1005):
// -        ev_leaf(&mut bits, if distinct { j as u64 } else { 1 }); // its range: one leaf, the site minimum
// +        ev_leaf(&mut bits, if distinct { j as u64 } else { 0 }); // its range: one leaf, the site minimum
```

Command:

```text
HEAD: cargo nextest run -p before -p suanpan --all-features -E 'test(/w4_|witness4_/)' --no-fail-fast
Mutated meter.rs:1005: cargo nextest run -p before -p suanpan --all-features --no-fail-fast -E 'test(/fork/) | test(/laws_hold/) | test(/generator_classes/) | test(/doc_hidden/) | test(/roster_order|family_names|every_shape|band_citations|board_roster|envelope_only/) | test(/memo_chain/) | test(/shape/) | test(/laws_seeds/) | test(/w4_party12|witness4_uncited/) | test(/every_family_answers/)'
```

Output:

```text
HEAD, closed forms:
        FAIL [   0.007s] ( 8/11) before meter::tests::witness4_uncited_generator_closed_forms_match_their_docs
    MEASURED wide_tail(1, 1): bits=8 doc=9
    MEASURED wide_tail(3, 2): bits=16 doc=17
    MEASURED wide_tail(10, 7): bits=50 doc=51
    MEASURED memo_chain(1, false): bits=16 doc=22
    MEASURED memo_chain(2, false): bits=26 doc=35
    MEASURED memo_chain(5, false): bits=56 doc=74
    MEASURED memo_comb_id(1): bits=22 doc=26
    MEASURED memo_churn_id(1): bits=14 doc=20
    MEASURED memo_comb_id(2): bits=36 doc=40
    MEASURED memo_churn_id(2): bits=24 doc=34
    MEASURED memo_comb_id(5): bits=78 doc=82
    MEASURED memo_churn_id(5): bits=54 doc=76
    MEASURED staircase(7): bits=44 finding 6d+2=44 capacity hint 5d+8=43
    MEASURED staircase(20): bits=122 finding 6d+2=122 capacity hint 5d+8=108
    MEASURED memo_fanout(4, 5): bits=110 finding k(4b+6)+6=110 doc ~13k+2kb+9=101
HEAD, canonicality:
        PASS [   0.007s] ( 6/11) before meter::tests::witness4_uncited_generators_round_trip_through_strict_decode

Under the memo_chain shared -> 0 mutation:
        PASS [   0.008s] (32/49) before::meter memo_resolution_cost::memo_chain_distinct_resolution_reads_linear
        PASS [   0.008s] (33/49) before::meter memo_resolution_cost::memo_chain_shared_control_is_flat_per_unit
        FAIL [   0.009s] ( 5/49) before meter::tests::witness4_uncited_generators_round_trip_through_strict_decode
    checking event memo_chain(3, false): 30 bits
    thread 'meter::tests::witness4_uncited_generators_round_trip_through_strict_decode' (241347257) panicked at crates/before/src/meter/tests.rs:34:43:
    a stored version's wire bytes decode: NotCanonical
```

Explanation:

Verified by running. Closed forms: the exact formulas the docs state are wrong for wide_tail (4d+2b+2 emitted vs 4d+2b+3 at meter.rs:641-642), memo_chain shared (10k+6 vs 13k+9 at 973-975), memo_comb_id (14d+8 vs 14d+12 at 1091-1092) and memo_churn_id (10d+4 vs 14d+6 at 1258-1260); staircase is exactly 6d+2 and exceeds its 5d+8 capacity hint from d = 7 (44 > 43); memo_fanout is k(4b+6)+6, not ~13k+2kb+9. Canonicality: at HEAD all twenty uncited generators do round-trip strictly (no present defect), but the module's own tests never check them — with memo_chain's shared range leaf set to 0 the emitted stream is rejected by Version::decode as NotCanonical while the two committed memo_chain rows in tests/meter.rs still build and measure it green; only the scratch witness fails.

## meter-registry-tier2-11

Outcome: demonstrated

Witness agent: 4

Test code:

```rust
// Mutation (no test added; the committed registry tests are the subject):
// crates/before/src/meter/registry.rs
//      LatentLadder,
// +    /// witness4 probe variant (mutation).
// +    Probe,
//  }
// ...
//              FamilyId::LatentLadder => 51,
// +            FamilyId::Probe => 52,
// ...
// +            FamilyId::Probe => FamilySpec {
// +                name: "probe",
// +                shapes: &[Shape::LatentLadder],
// +                coverage: Coverage::EnvelopeOnly { reason: "witness4 probe", decided: "2026-09-02" },
// +                bands: Bands::Unbanded { reason: "witness4 probe", decided: "2026-09-02" },
// +                denominator: "packed input bytes",
// +                closed_form: None,
// +            },
//              FamilyId::LatentLadder => FamilySpec {
// FamilyId::ALL left at 52 entries, untouched.
// crates/before/src/meter/board/family.rs:783 and board/ops.rs:182:
// -            | FamilyId::LatentLadder => unreachable!(
// +            | FamilyId::LatentLadder
// +            | FamilyId::Probe => unreachable!(
// crates/before/tests/verdict_matrix.rs:411 (also an exhaustive match, compiler-forced):
// +        FamilyId::Probe => (vec![Shape::LatentLadder.packed2(4, 4).version()], vec![]),
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --no-run   # compiled clean, no warnings
cargo nextest run -p before -p suanpan --all-features --no-fail-fast -E 'test(/fork/) | test(/laws_hold/) | test(/generator_classes/) | test(/doc_hidden/) | test(/roster_order|family_names|every_shape|band_citations|board_roster|envelope_only/) | test(/memo_chain/) | test(/shape/) | test(/laws_seeds/) | test(/w4_party12|witness4_uncited/) | test(/every_family_answers/)'
```

Output:

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 41.39s
        PASS [   0.008s] ( 1/49) before meter::registry::tests::roster_order_is_committed
        PASS [   0.008s] ( 2/49) before meter::registry::tests::family_names_are_unique
        PASS [   0.009s] ( 6/49) before meter::registry::tests::band_citations_are_unique_and_nonempty
        PASS [   0.009s] ( 8/49) before meter::registry::tests::envelope_only_rulings_are_dated
        PASS [   0.010s] (10/49) before meter::registry::tests::board_roster_derives_from_coverage_answers
        PASS [   0.009s] (38/49) before::verdict_matrix every_family_answers_the_matrix_coverage_question
        PASS [   0.090s] (41/49) before meter::registry::tests::every_shape_is_cited_by_a_family
```

Explanation:

Verified by running: a FamilyId::Probe variant with an index() arm of 52, a spec() row, and the compiler-forced or-pattern arms — but no entry in FamilyId::ALL — compiles without a warning and passes every registry test plus the verdict matrix's coverage census, all of which iterate ALL and therefore never see it. One correction to the finding's inventory: tests/verdict_matrix.rs:162-412 also matches FamilyId exhaustively (matrix_operands), so a new variant needs an arm there too; that arm is compiler-forced like the other two and does not make any roster test fire. index()'s doc claim at registry.rs:1170-1172 (a variant 'cannot be declared without joining the roster') is not what the tests enforce.

## board-ops-render-6

Outcome: demonstrated

Witness agent: 4

Test code:

```rust
// appended to crates/before/src/meter/board/tests.rs
#[test]
fn witness4_declared_heap_ceiling_has_no_under_side() {
    use super::ceilings::ASCEND_CLIFF_TICK_HEAP_BYTES_PER_INPUT_BYTE;
    use super::floors::na;
    use super::judge::evaluate;
    use super::measure::Sample;
    use super::{ByCurrency, Floors};
    const PROBE_NA: &str = "probe: the declared heap ceiling alone is under test";
    let sample = |denom: usize, heap: u64| -> Sample {
        Sample {
            denom_bytes: denom,
            exp_denom_bytes: denom,
            limb_denom: denom as u64,
            text_row: false,
            floors: Floors {
                heap: na(PROBE_NA),
                segments: na(PROBE_NA),
                limb: na(PROBE_NA),
                scan: na(PROBE_NA),
                touch: na(PROBE_NA),
            },
            fold_arity: None,
            fold_search_bits: 0,
            heap_model: None,
            declared_heap: Some(ASCEND_CLIFF_TICK_HEAP_BYTES_PER_INPUT_BYTE),
            declared_limb: None,
            readings: ByCurrency {
                heap: Some(heap),
                segments: Some(0),
                limb: None,
                scan: None,
                touch: None,
            },
        }
    };
    let declared = evaluate(
        "version_tick",
        "ascend-cliff",
        sample(10_000, 2_270_000), // exactly the declared 227 B/B
        sample(20_000, 4_540_000),
    );
    eprintln!("MEASURED at-declaration red={:?}", declared.red);
    let cured = evaluate(
        "version_tick",
        "ascend-cliff",
        sample(10_000, 100_000), // 10 B/B: a certificate-memory cure
        sample(20_000, 200_000),
    );
    eprintln!(
        "MEASURED cured (10 B/B under a 227 B/B declaration): red={:?} heap_exp_judged={}",
        cured.red, cured.scores.heap.exp_judged
    );
    assert!(
        !cured.red.is_empty(),
        "a reading at 10 B/B under a 227 B/B declaration reads green: no under-side check exists"
    );
}
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features -E 'test(/w4_|witness4_/)' --no-fail-fast
```

Output:

```text
        FAIL [   0.007s] ( 7/11) before meter::board::tests::witness4_declared_heap_ceiling_has_no_under_side
    MEASURED at-declaration red=[]
    MEASURED cured (10 B/B under a 227 B/B declaration): red=[] heap_exp_judged=true
    thread 'meter::board::tests::witness4_declared_heap_ceiling_has_no_under_side' (241317180) panicked at crates/before/src/meter/board/tests.rs:1343:5:
    a reading at 10 B/B under a 227 B/B declaration reads green: no under-side check exists
```

Explanation:

Verified by running through judge::evaluate with the same Sample shape the capacity-model probe uses (board/tests.rs:1018-1044): two samples carrying `declared_heap: Some(227.0)` and no model, reading 10 B/B at denominators 10_000 and 20_000 (above the flat allowance, exponent leg judged and reading 1.00), produce an empty `red`. Contrast the capacity model's 'improved' probe at tests.rs:1074-1086, which reads `heap capacity-model floor (stale model)` for the same shape of event. judge.rs:341-345 substitutes only the ceiling, so a certificate-memory cure leaves the 227 B/B declaration stale and green.

## board-ops-render-26

Outcome: refuted

Witness agent: 4

Test code:

```rust
// appended to crates/before/src/meter/board/tests.rs
#[cfg(feature = "limb-meter")]
#[test]
fn witness4_delegating_parse_floor_separates_the_bypass_reading() {
    let mut rows = Vec::new();
    for ((packed, name), (floor_name, pinned_floor)) in [
        (hugeleaf(16_000), "hugeleaf"),
        (bigroot(8_000, 2_000), "bigroot"),
    ]
    .into_iter()
    .zip(DELEGATING_PARSE_LIMB_FLOORS)
    {
        assert_eq!(name, floor_name);
        let v = version_of(&packed);
        let s = v.to_string();
        crate::meter::reset_limb_ops();
        let parsed: Version = s.parse().expect("a displayed version parses back");
        let ops = crate::meter::limb_ops();
        assert_eq!(parsed, v);
        let radix: u64 = stored_bases(&v)
            .iter()
            .map(|b| b.bits().div_ceil(64).max(1))
            .sum();
        let bypass = ops - radix;
        eprintln!(
            "MEASURED {name}: pipeline ops={ops} radix site={radix} ({:.0}% of total) bypass=ops-radix={bypass} pinned floor={pinned_floor} floor/total={:.3}",
            100.0 * radix as f64 / ops as f64,
            pinned_floor as f64 / ops as f64
        );
        rows.push((name, ops, radix, bypass, pinned_floor));
    }
    for (name, _ops, _radix, bypass, pinned_floor) in rows {
        assert!(
            bypass < pinned_floor,
            "{name}: the pinned floor {pinned_floor} does not sit above the bypass reading {bypass}: a parser that stopped recording at the radix site would still pass"
        );
    }
}
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --no-fail-fast --success-output immediate -E 'test(/witness4_delegating|delegating_parser|shape|plateau|combine|region|cells/)'
```

Output:

```text
        PASS [   0.036s] (20/33) before meter::board::tests::witness4_delegating_parse_floor_separates_the_bypass_reading
    MEASURED hugeleaf: pipeline ops=501 radix site=250 (50% of total) bypass=ops-radix=251 pinned floor=425 floor/total=0.848
    MEASURED bigroot: pipeline ops=8259 radix site=4127 (50% of total) bypass=ops-radix=4132 pinned floor=7020 floor/total=0.850
        PASS [   0.062s] (22/33) before meter::board::tests::delegating_parser_stays_under_the_text_limb_ceiling
```

Explanation:

Verified by running under limb-meter, computing the radix site's recording exactly as base.rs:92-93 / limb_meter::record_wide does (one `bits.div_ceil(64).max(1)` per parsed value). The finding's numerical evidence does not hold at HEAD: the radix site contributes 50% of the pipeline total on both families (250 of 501; 4127 of 8259), so the doc's 'roughly half' is accurate, and the pinned floors sit above the bypass reading with margin (251 < 425; 4132 < 7020) — a parser that deleted the radix recording would trip both floors today. The 752 / 24_399 totals the finding quotes from bc222bd2 are not today's readings. What survives by reading is the verification-gap half: tests.rs:180-189 asserts only `ops >= floor` and `ops >= pinned_floor`, never the bypass separation; the scratch test above is that missing per-run check, and it passes.

## surface-roster-23

Outcome: demonstrated

Witness agent: 4

Test code:

```rust
// Mutation (crates/before/src/shape.rs), no test added:
//  pub struct Plateaus<'a> {
//      walk: VersionWalk<'a>,
//      finished: bool,
// +    _marker: core::marker::PhantomData<*const ()>, // witness4 mutation
//  }
// ... in of_version:
//              finished: false,
// +            _marker: core::marker::PhantomData,
//
// Mechanical check of auto_traits.rs coverage:
// for t in Limbs TooWide shape::Cell shape::Cells shape::Overlay shape::Plateau shape::Plateaus shape::Region shape::Regions shape::Rise causally::Down causally::Up causally::Neutral; do printf '%s: ' "$t"; grep -c "$t" crates/before/src/auto_traits.rs; done
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --no-run   # with the PhantomData<*const ()> field in place
cargo nextest run -p before -p suanpan --all-features --no-fail-fast -E 'test(/fork/) | ... | test(/shape/) | ...'   # the M1 batch
```

Output:

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 41.39s   (no warnings, no errors)
        PASS [   0.006s] (12/49) before shape::tests::shape_walks_fuse
        PASS [   0.006s] (13/49) before shape::tests::empty_combine_is_the_trivial_cell
        PASS [   0.007s] (14/49) before shape::tests::single_input_combine_is_the_shape
        PASS [   0.007s] (16/49) before shape::tests::trivial_values_walk_whole
        PASS [   0.101s] (44/49) before shape::tests::comparison_agrees_with_pointwise_order
        PASS [   0.110s] (46/49) before shape::tests::join_and_meet_are_pointwise_extrema

Limbs: 0
TooWide: 0
shape::Cell: 0
shape::Cells: 0
shape::Overlay: 0
shape::Plateau: 0
shape::Plateaus: 0
shape::Region: 0
shape::Regions: 0
shape::Rise: 0
causally::Down: 1
causally::Up: 1
causally::Neutral: 0
```

Explanation:

Verified by building: with a `PhantomData<*const ()>` field, `shape::Plateaus` is neither Send nor Sync, and the crate plus every test target compiles under --all-features with no diagnostic, because auto_traits.rs (the compile-time pin extract.rs:267-270 defers to) never names Plateaus; the shape tests then pass unchanged. The grep confirms none of Limbs, error::TooWide, the eight shape:: types, or causally::Neutral appear in auto_traits.rs (causally::Down/Up appear only as Query type parameters at lines 23-24, not as asserted types). `just gate` itself was not run (not permitted), but the pin in question is a static_assertions compile-time check, so a clean compile is the relevant observation.

## testing-diff-gen-14

Outcome: demonstrated

Witness agent: 4

Test code:

```rust
// Mutation (crates/before/src/testing/generators.rs:328), no test added:
// -        1 => (0u32..96).prop_map(|k| (codec::Base::from(1u8) << k) + codec::Base::from(1u8)),
// +        1 => (0u32..64).prop_map(|k| (codec::Base::from(1u8) << k) + codec::Base::from(1u8)),
// The committed census test testing::generators::tests::generator_classes_stay_under_mass is the subject.
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --no-fail-fast -E 'test(/fork/) | test(/laws_hold/) | test(/generator_classes/) | test(/doc_hidden/) | test(/roster_order|family_names|every_shape|band_citations|board_roster|envelope_only/) | test(/memo_chain/) | test(/shape/) | test(/laws_seeds/) | test(/w4_party12|witness4_uncited/) | test(/every_family_answers/)'
```

Output:

```text
        PASS [   0.563s] (49/49) before testing::generators::tests::generator_classes_stay_under_mass
```

Explanation:

Verified by running: with arm 5's shift range cut from 0..96 to 0..64 (so the arm can no longer produce a value beyond 2^64 on its own), the census still passes — the `wide` class is fed by arms 4, 6, and 7 as well, and the census floors are per class (tests.rs:125-137), not per arm. This is the finding's exact construction; the census doc's promise that 'a dead arm ... reads red' (tests.rs:78-80) is not what the test measures. The census counts under the mutation were not captured (nextest suppresses passing tests' stderr in that run).

## envelopes-a-2

Outcome: demonstrated

Witness agent: 4

Test code:

```rust
// Same test as crate-root-32: crates/before/tests/zz_witness_4.rs
#[test]
fn w4_segments_counter_has_no_writer_in_this_binary() {
    fn deep(n: usize) -> usize {
        if n == 0 {
            0
        } else {
            1 + std::hint::black_box(deep(n - 1))
        }
    }
    before::meter::reset_stack_segments();
    let frames = stacker::grow(256 << 20, || deep(200_000));
    assert_eq!(frames, 200_000);
    let segments = before::meter::stack_segments();
    eprintln!("MEASURED segments after stacker::grow over a 200_000-frame recursion: {segments}");
    assert_eq!(
        segments, 0,
        "the counter moved: some writer exists in this binary after all"
    );
}
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features -E 'test(/w4_/) | test(/witness4_index/) | test(/witness4_uncited_generators_round/)' --success-output immediate
```

Output:

```text
        PASS [   0.007s] (6/8) before::zz_witness_4 w4_segments_counter_has_no_writer_in_this_binary
    MEASURED segments after stacker::grow over a 200_000-frame recursion: 0
```

Explanation:

Verified by running in an integration-test binary of the same kind as tests/meter.rs (library linked without cfg(test), built with --all-features): even a real heap-grown stack segment under a 200 000-frame recursion leaves meter::stack_segments() at 0, so every `segments <= env.segments` ceiling in tests/meter.rs (metered(), lines 387-391) judges a signal that cannot move in that binary. The one increment of SEGMENTS_GROWN is inside `#[cfg(test)] fn grow` (recurse.rs:100-109); `descend!` is `#[cfg(test)]` too (recurse.rs:118-119) and its only call sites are test surfaces (see crate-root-32's grep).

## envelopes-b-21

Outcome: inconclusive

Witness agent: 4

Test code:

```rust
// Not constructed: the construction requires mutating the fill kernel's
// ledger-link folds (or the Shape::MemoChain scaling) and running the six
// memo_resolution_cost rows; not done within the cap. Read-only anchor:
//   awk 'NR>=8413 && NR<=8432' crates/before/tests/meter.rs
```

Command:

```text
awk 'NR>=8400 && NR<=8440 {printf "%6d\t%s\n", NR, $0}' crates/before/tests/meter.rs
```

Output:

```text
  8404	        assert!(
  8405	            run.touches >= run.input / 8,
  8420	    fn assert_flat(name: &str, small: &Run, large: &Run) {
  8425	        assert!(
  8426	            u128::from(large.touches) * 2 <= u128::from(small.touches) * 5,
  8427	            "{name}: touch growth across the doubling exceeds x2.5 \
  8428	             ({} -> {}): a ledger link is being read more than once",
```

Explanation:

Not constructed within the cap. By reading, tests/meter.rs:8425-8426 asserts the raw cross-scale ratio `large.touches * 2 <= small.touches * 5` with no absolute ceiling and no per-byte normalization, and the only other assertion in the helper is the `touches >= input / 8` liveness floor at 8405; the input doubling itself is asserted nowhere in the helper. The claim that a uniform constant inflation of the folds stays green everywhere except memo_fanout's absolute ceiling is consistent with that code, but I did not run a kernel mutation to observe it. In the M1 run, memo_chain_distinct_resolution_reads_linear and memo_chain_shared_control_is_flat_per_unit both passed even with memo_chain's shared generator emitting a non-canonical stream (meter-core-2), which illustrates how loosely these rows bind, but that is a different mutation from the one this finding names.

## tests-other-13

Outcome: demonstrated

Witness agent: 4

Test code:

```rust
// Mutation (crates/before/src/party.rs), no test added; the committed
// tests/doc_hidden.rs::doc_hidden_occurrences_match_the_committed_roster is the subject:
// @@ -154,6 +154,7 @@ impl Party {
// +    #[doc(hidden)]
//      pub fn is_seed(&self) -> bool {
// @@ -815,7 +816,6 @@ mod sealed {
//  #[doc(hidden)]
//  pub trait PartyLiteral: sealed::Sealed {
// -    #[doc(hidden)]
//      fn into_id_bits(self) -> Result<codec::BitsBuf, Parse>;
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --no-fail-fast -E 'test(/fork/) | test(/laws_hold/) | test(/generator_classes/) | test(/doc_hidden/) | test(/roster_order|family_names|every_shape|band_citations|board_roster|envelope_only/) | test(/memo_chain/) | test(/shape/) | test(/laws_seeds/) | test(/w4_party12|witness4_uncited/) | test(/every_family_answers/)'
```

Output:

```text
        PASS [   0.012s] (30/49) before::doc_hidden doc_hidden_occurrences_match_the_committed_roster
```

Explanation:

Verified by running: moving `#[doc(hidden)]` off the sealed trait method `into_id_bits` (party.rs:818) and onto the public `Party::is_seed` (party.rs:157) keeps the count in party.rs at 2, and the roster test passes — a public inherent method is now hidden from rustdoc and from the surfacecheck leg with no committed check noticing, although doc_hidden.rs:1-2 and :16 describe the pin as 'by name'. The `#[cfg_attr(..., doc(hidden))]` spelling variant was not separately exercised; the count at doc_hidden.rs:32 matches the literal `#[doc(hidden)]` only, so by reading it would be invisible to the scan.

## tests-other-22

Outcome: demonstrated

Witness agent: 4

Test code:

```rust
// Mutation (crates/before/fuzz/fuzz_targets/fuzz_laws.rs:65), no test added:
// -const ARITY_SPAN: usize = 18;
// +const ARITY_SPAN: usize = 16;
// The committed tests/fuzz_seeds.rs::laws_seeds_decode_per_framing_and_stay_wide is the subject.
// Companion arithmetic (crates/before/tests/zz_witness_4.rs):
#[test]
fn w4_other22_arity_band_arithmetic() {
    assert_eq!(17 % 16, 1);
    assert_eq!(16 % 16, 0);
    assert_eq!(15 % 16, 15);
    assert!(17 < 18 && 16 < 18);
    let _ = Ordering::Equal;
}
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --no-fail-fast -E 'test(/fork/) | test(/laws_hold/) | test(/generator_classes/) | test(/doc_hidden/) | test(/roster_order|family_names|every_shape|band_citations|board_roster|envelope_only/) | test(/memo_chain/) | test(/shape/) | test(/laws_seeds/) | test(/w4_party12|witness4_uncited/) | test(/every_family_answers/)'
grep -n "fuzz_laws\|include_str\|read_to_string" crates/before/tests/fuzz_seeds.rs
```

Output:

```text
        PASS [   0.008s] (31/49) before::fuzz_seeds laws_seeds_decode_per_framing_and_stay_wide

287:/// Carve the next length-prefixed chunk off a `fuzz_laws` seed, exactly as
301:/// Carve one list script — `[arity: u8][pool indices]` — off a `fuzz_laws`
330:/// Every `fuzz_laws` seed decodes positionally per the target's framing —
347:        if seed.target != "fuzz_laws" {
(no include_str! or read of the fuzz target source)
```

Explanation:

Verified by running: with the target's ARITY_SPAN changed to 16, the seed checker stays green because it checks against its own copy `const ARITY_SPAN: usize = 18` (fuzz_seeds.rs:308) and never reads the fuzz target (the grep shows the binary references fuzz_laws only by seed directory name). Under the mutated target the committed arity bytes 17/16/15 fold to 1/0/15 (`% ARITY_SPAN` at fuzz_laws.rs:81), so the laws_wide_gamma seed would no longer drive the second-octave crossings; that half is arithmetic (verified above), not an executed fuzz run — `just fuzz` is outside my permitted commands.

## tests-other-30

Outcome: demonstrated

Witness agent: 4

Test code:

```rust
// Mutation (crates/before/tests/verdict_matrix.rs:1139), no test added:
// -            if dom != expected_dominance(lo_rel, hi_rel) {
// +            if false && dom != expected_dominance(lo_rel, hi_rel) { // witness4 mutation
// The committed production run and both twins are the subject.
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --no-fail-fast -E 'test(/span|borsh|admit|collapsible|canonical|seeds/) | test(/sweep|causal|cmp|order|exit/) | test(/verdict|matrix|polarity|equality_inverting/) | test(/w4_skyline6|w4_masked32/)'
```

Output:

```text
        PASS [   0.010s] (203/227) before::verdict_matrix every_family_answers_the_matrix_coverage_question
        PASS [   0.018s] (205/227) before::verdict_matrix inverted_verdict_tripwires_match_the_committed_roster
        PASS [   0.012s] (206/227) before::verdict_matrix verdict_class_floors_read_red_on_degenerate_pools
        PASS [   0.112s] (213/227) before::verdict_matrix pool_witnesses_every_verdict_class
        PASS [   3.984s] (225/227) before::verdict_matrix production_verdicts_agree_across_the_surfaces
        PASS [   4.005s] (226/227) before::verdict_matrix equality_inverting_sweep_reads_inverted_through_the_matrix
        PASS [   4.003s] (227/227) before::verdict_matrix polarity_flipped_sweep_reads_inverted_through_the_matrix
```

Explanation:

Verified by running: with the dominance leg neutralized (`if false && ...`), the production run passes (no violation and the census insert at 1134 is unconditional), the equality twin passes (dominance is not among its pinned legs), and the polarity twin passes because Axis::Placement still fires through the placement/precedence/coincident legs, satisfying `axis_count(axis) > 0` at 1373-1379. The twin therefore pins axes, not legs. This run also carried the admit.rs and order_exit mutations of skyline-coding-6 / masked-32; neither touches verdict_matrix.rs's own checker, so the passes attribute to this mutation.

## fuzz-guests-pins-14

Outcome: inconclusive

Witness agent: 4

Test code:

```rust
// Not constructed: `just fuzz` / cargo-fuzz runs are outside the permitted commands.
// Read-only anchors checked:
//   awk 'NR>=8 && NR<=32' crates/before/fuzz/src/lib.rs
//   grep -rn "under_heap_cap\|PEAK_HEAP_CAP_BYTES" crates/before/fuzz crates/before/tests
//   grep -n '^\[\[bin\]\]' -A 4 crates/before/fuzz/Cargo.toml | grep -c 'test = false'
```

Command:

```text
awk 'NR>=8 && NR<=32 {printf "%6d\t%s\n", NR, $0}' crates/before/fuzz/src/lib.rs; grep -rn "under_heap_cap\|PEAK_HEAP_CAP_BYTES" crates/before/fuzz/src crates/before/fuzz/fuzz_targets crates/before/tests; grep -n "^\[\[bin\]\]" -A 4 crates/before/fuzz/Cargo.toml | grep -c "test = false"
```

Output:

```text
    10	//! The ceiling is generous and absolute: a flat [`PEAK_HEAP_CAP_BYTES`],
    11	//! not yet proportional to input size. The known amplifiers are linear in
    29	pub const PEAK_HEAP_CAP_BYTES: usize = 1 << 30;
crates/before/fuzz/src/lib.rs:37:pub fn under_heap_cap<R>(body: impl FnOnce() -> R) -> R {
crates/before/fuzz/src/lib.rs:42:        peak <= PEAK_HEAP_CAP_BYTES,
crates/before/fuzz/fuzz_targets/fuzz_laws.rs:38:    before_fuzz::under_heap_cap(|| run(data));
5
```

Explanation:

Not constructed: the construction is a libFuzzer run (`just fuzz`), which I may not launch. Assessed by reading: the cap is a flat 1 GiB (lib.rs:29), its only callers are the fuzz targets (no test under crates/before/tests references it), and all five [[bin]] targets in fuzz/Cargo.toml are `test = false`, so nothing committed demonstrates the cap firing. The 2^18x arithmetic at -max_len 4096 (4096^2 = 16 MiB, 64x under the cap) is straightforward but unexercised here.

## fuzz-guests-pins-35

Outcome: inconclusive

Witness agent: 4

Test code:

```rust
// Not constructed: `just wasm32-pins` builds the wasm32 guest and runs the
// wasmtime harness in a detached workspace, outside the permitted commands.
// Read-only anchors checked:
//   awk 'NR>=140 && NR<=175' crates/before/wasm32-pins/harness/tests/pins.rs
//   grep -n UnreachableCodeReached crates/before/wasm32-pins/harness/src/lib.rs
```

Command:

```text
awk 'NR>=147 && NR<=153 {printf "%6d\t%s\n", NR, $0}' crates/before/wasm32-pins/harness/tests/pins.rs; grep -n "UnreachableCodeReached" crates/before/wasm32-pins/harness/src/lib.rs | head -3
```

Output:

```text
   147	#[test]
   148	fn version_decode_memory_terminal_traps() {
   149	    assert_eq!(
   150	        call1("pin_version_decode", 1_073_741_817),
   151	        Outcome::Trapped(Trap::UnreachableCodeReached),
   152	    );
   153	}
32:    /// The export trapped; `Trap::UnreachableCodeReached` is a guest panic.
```

Explanation:

Not constructed within the cap (the wasm32-pins workspace cannot be built or run under my permissions). By reading, the terminal pin asserts only `Outcome::Trapped(Trap::UnreachableCodeReached)` and the harness itself documents that trap as 'a guest panic' — the genre every wasm32-unknown-unknown abort (allocation failure, explicit panic, overflow trap, backend capacity panic) produces — so the pin cannot distinguish an allocation-failure terminal from any other panic at that coordinate. Whether the finding's specific `assert!(n < 1_000_000_000)` mutation stays green was not observed.

## fuzzfit-strategies-16

Outcome: inconclusive

Witness agent: 4

Test code:

```rust
// Not constructed: the fuzzfit suite (`just fuzzfit`) builds a wasm32 guest
// and runs a release-profile harness in a detached workspace, outside the
// permitted commands. No independent reading of strategies.rs / enforce.rs
// was performed beyond the lines quoted in the finding.
```

Command:

```text
(not run)
```

Output:

```text
(none)
```

Explanation:

Not constructed within the cap, and I did not independently verify the finding's line citations in fuzzfit/harness/src/strategies.rs or enforce.rs; this entry records only that the construction (lowering ESCALATION_BUDGET.max_ops and running the fuzzfit suite) was not attempted because the fuzzfit workspace is outside what I may build or run here.

## fuzzfit-bands-19

Outcome: inconclusive

Witness agent: 4

Test code:

```rust
// Not constructed (fuzzfit workspace not runnable here). Mechanical grep performed:
//   grep -rn "run_program" crates/before/fuzzfit/harness/src
//   grep -rn "assert_eq!(.*fuel\|fuel.*==" crates/before/fuzzfit/harness/src
//   grep -n "fuel" crates/before/fuzzfit/harness/tests/enforce.rs crates/before/fuzzfit/harness/tests/sanity.rs | grep -n "assert_eq\|==\|determin"
//   grep -n "fuzzfit" justfile
```

Command:

```text
grep -rn "run_program" crates/before/fuzzfit/harness/src | grep -v "^\s*//"; grep -rn "assert_eq!(.*fuel\|fuel.*==" crates/before/fuzzfit/harness/src; grep -n "fuel" crates/before/fuzzfit/harness/tests/enforce.rs crates/before/fuzzfit/harness/tests/sanity.rs | grep -n "assert_eq\|==\|determin"; grep -n "fuzzfit" justfile | sed -n '/^5[7-9][0-9]:/p'
```

Output:

```text
crates/before/fuzzfit/harness/src/drive.rs:40:pub fn run_program(program: &[Op]) -> Result<Vec<Sample>, Malformed> {
crates/before/fuzzfit/harness/src/drive.rs:127:        let samples = run_program(&program)
crates/before/fuzzfit/harness/src/drive.rs:154:        let samples = run_program(&program)
crates/before/fuzzfit/harness/src/bin/diag.rs:45:        let samples = run_program(&program)
crates/before/fuzzfit/harness/src/bin/calibrate.rs:308:        let samples = run_program(&program)
crates/before/fuzzfit/harness/src/bin/probe.rs:64:    assert_eq!(a, b, "fuel is not deterministic across fresh instances");
(no fuel equality or determinism assertion in tests/enforce.rs or tests/sanity.rs)
575:fuzzfit-build:
576:    cargo build -p fuzzfit-guest --release --target wasm32-unknown-unknown --target-dir {{ fuzzfit_target }}
577:    {{ justfile_directory() }}/tools/memwatch cargo build -p fuzzfit-harness --tests --release
594:fuzzfit: fuzzfit-build
597:    FUZZFIT_GUEST_WASM={{ fuzzfit_guest_wasm }} {{ justfile_directory() }}/tools/memwatch cargo nextest run --cargo-profile release
```

Explanation:

Not constructed as a runtime demonstration (the fuzzfit workspace is outside my permitted commands), but the finding's grep construction was performed mechanically and agrees with it: the only comparison of two independent fuel readings in the harness is bin/probe.rs:64, the two committed test files contain no fuel-equality or determinism assertion, and the fuzzfit recipes build the guest and run `cargo nextest run` — which executes tests, never the probe binary — so no recipe exercises the determinism check.

## fuelscape-pipeline-28

Outcome: inconclusive

Witness agent: 4

Test code:

```rust
// Not constructed (the guest rebuild and `just fuelscape` are outside the
// permitted commands). Mechanical check performed:
//   grep -n COMBINE_ARITY_CAP crates/before-fuelscape/src/*.rs crates/before/fuzzfit/guest/src/lib.rs
//   awk 'NR>=15 && NR<=30' crates/before-fuelscape/src/render/tests.rs
//   grep -rn "ARITY_CAP\|arity_cap" crates/before-fuelscape/src crates/before/fuzzfit/harness/src | grep -i "test\|assert"
```

Command:

```text
grep -n "COMBINE_ARITY_CAP" crates/before-fuelscape/src/*.rs crates/before/fuzzfit/guest/src/lib.rs; awk 'NR>=18 && NR<=22 {printf "%6d\t%s\n", NR, $0}' crates/before-fuelscape/src/render/tests.rs; grep -rn "ARITY_CAP\|arity_cap" crates/before-fuelscape/src crates/before/fuzzfit/harness/src | grep -i "test\|assert"
```

Output:

```text
crates/before-fuelscape/src/ops.rs:203:const COMBINE_ARITY_CAP: u32 = 16;
crates/before-fuelscape/src/ops.rs:395:        inputs: Inputs::VersionSliceCapped(COMBINE_ARITY_CAP),
crates/before/fuzzfit/guest/src/lib.rs:791:const COMBINE_ARITY_CAP: u32 = 16;
crates/before/fuzzfit/guest/src/lib.rs:808:    if n > COMBINE_ARITY_CAP {
    18	    let plan = Plan {
    19	        base_seed: 0x5eed,
    20	        samples_per_column: 3,
    21	        max_bytes: 8,
    22	    };
(no test or assertion references either constant)
```

Explanation:

Not constructed as a runtime demonstration (rebuilding the guest at a lower cap and running `just fuelscape` are outside my permitted commands). The mechanical checks agree with the finding's premises: the host and guest constants are separate declarations both equal to 16, the smoke plan's `max_bytes: 8` caps every smoke arity at 8, and no test or assertion in either crate references the constants, so the doc's 'kept equal by the pipeline smoke test's combine case' (ops.rs:200-202) names a check that does not exist. Whether a mismatch aborts a survey at plan.rs:395 was not observed.

## gate-legs-2

Outcome: inconclusive

Witness agent: 4

Test code:

```rust
// Not constructed: installing a different cargo-mutants release is outside
// the permitted actions. Mechanical check of the pin and the CI install:
//   cargo mutants --version
//   grep -n '"tool"' tools/mutantcheck-expected.json
//   awk 'NR>=76 && NR<=88' .github/workflows/ci.yml
//   awk 'NR>=140 && NR<=152' tools/mutantcheck
```

Command:

```text
cargo mutants --version; grep -n '"tool"' tools/mutantcheck-expected.json; awk 'NR>=83 && NR<=86 {printf "%6d\t%s\n", NR, $0}' .github/workflows/ci.yml; awk 'NR>=145 && NR<=151 {printf "%6d\t%s\n", NR, $0}' tools/mutantcheck
```

Output:

```text
cargo-mutants 27.1.0
2:  "tool": "cargo-mutants 27.1.0",
    83	      - name: Install just, cargo-nextest, cargo-rdme, cargo-fuzz, cargo-mutants, and wasm-pack
    84	        uses: taiki-e/install-action@37f7c5781271959fb65b6b35224e28652ff2b63d # v2.87.0
    85	        with:
    86	          tool: just,cargo-nextest,cargo-rdme@2.1.0,cargo-fuzz,cargo-mutants,wasm-pack
   145	    pinned_tool = expected.get("tool")
   146	    if tool_version != pinned_tool:
   147	        problems.append(
   148	            f"tool version {tool_version!r} does not match the pinned "
   149	            f"{pinned_tool!r}: operator sets move between releases, so "
   150	            "re-pin the counts in the same diff as the tool bump"
```

Explanation:

Not constructed (no alternate cargo-mutants was installed). The mechanical checks confirm the finding's premises: the locally installed cargo-mutants (27.1.0) equals the committed pin, CI installs `cargo-mutants` with no version while `cargo-rdme@2.1.0` is pinned on the same line, and mutantcheck rejects any version string other than the pinned one before comparing counts. The consequence — a red mutants-list leg on an untouched tree when the action's manifest advances — follows from those lines but was not observed.

## meter-adequacy-1

Outcome: demonstrated

Witness agent: 4

Test code:

```rust
// Mutation (crates/before/src/shape.rs), no test added; output-preserving, cost quadratic:
//  pub struct Plateaus<'a> {
//      walk: VersionWalk<'a>,
//      finished: bool,
// +    bits: crate::codec::BitsView<'a>, // witness4 mutation: re-scan from 0 per item
// +    taken: usize,
//  }
// ... in of_version:
//              finished: false,
// +            bits: version.view().live(),
// +            taken: 0,
// ... in Iterator::next, after the `finished` check:
// +        // witness4 mutation: rebuild the walk from position 0 and replay every
// +        // advance so far (output identical, cost quadratic in the drain).
// +        let mut fresh = VersionWalk::open(self.bits);
// +        for _ in 0..self.taken {
// +            fresh.take_rise();
// +            fresh.advance();
// +        }
// +        self.walk = fresh;
// +        self.taken += 1;
//          let plateau = Plateau {
//
// Mechanical check: grep -rn "\.shape()" crates/before/tests/*.rs crates/before/src/meter/board/*.rs crates/before/src/meter/*.rs crates/before/benches/*.rs
```

Command:

```text
cargo nextest run -p before -p suanpan --all-features --no-fail-fast --success-output immediate -E 'test(/witness4_delegating|delegating_parser|shape|plateau|combine|region|cells/)'
grep -rn "\.shape()" crates/before/tests/*.rs crates/before/src/meter/board/*.rs crates/before/src/meter/*.rs crates/before/benches/*.rs
```

Output:

```text
        PASS [   0.008s] ( 1/33) before shape::tests::trivial_values_walk_whole
        PASS [   0.008s] ( 3/33) before shape::tests::single_input_combine_is_the_shape
        PASS [   0.009s] ( 4/33) before shape::tests::shape_walks_fuse
        PASS [   0.010s] ( 7/33) before shape::tests::empty_combine_is_the_trivial_cell
        PASS [   0.107s] (25/33) before shape::tests::comparison_agrees_with_pointwise_order
        PASS [   0.120s] (26/33) before shape::tests::join_and_meet_are_pointwise_extrema
     Summary [   0.817s] 33 tests run: 33 passed, 926 skipped

(grep: no `.shape()` call in tests/*.rs, meter/board/*.rs, meter/*.rs, or benches/*.rs)
```

Explanation:

Verified by running: with `Plateaus::next` rebuilt to re-open the stored stream from position 0 and replay every prior advance on each item (identical output, quadratic drain), all 33 tests matching shape/plateau/combine/region/cells pass, and the grep finds no `.shape()` call in the meter suites, the board's operation table, or the benches — so the linear drain claims at version.rs:738 and the board's excusal text (coverage.rs:611-615) have no enforcing instrument. `just gate` was not run (not permitted); the fuelscape leg is the only other measurement of these walks and is documented as audit-only (fuelscape/src/lib.rs:16).

## deps-3

Outcome: demonstrated

Witness agent: 4

Test code:

```rust
// No test: a three-build experiment in this worktree.
// (a) warm: cargo build -p before
// (b) edit one color literal in the figure, then build verbosely at HEAD's build.rs:
//     perl -0pi -e 's/#00d7d7/#00d7d8/' crates/before/results/space_consumption/itc_space_consumption.svg
//     cargo build -p before -v
// (c) with the figure still edited, add to build.rs after line 30:
// +    println!("cargo:rerun-if-changed=results/space_consumption/itc_space_consumption.svg"); // witness4
// +    println!("cargo:rerun-if-changed=docs/itc_space_consumption_readme.svg"); // witness4
//     cargo build -p before -v
// Both files restored with git restore --source=HEAD --staged --worktree afterwards.
```

Command:

```text
cargo build -p before; perl -0pi -e 's/#00d7d7/#00d7d8/' crates/before/results/space_consumption/itc_space_consumption.svg; cargo build -p before -v; (extend build.rs rerun-if-changed); cargo build -p before -v
```

Output:

```text
(b) HEAD build.rs, figure edited (git diff --stat: 1 file changed, 1 insertion(+), 1 deletion(-)):
       Fresh before v0.1.0 (/Users/oxide/src/rumors/.claude/worktrees/wf_85d96464-ea4-261/crates/before)
    EXIT=0   (no build-script run, no error)

(c) build.rs with the two rerun-if-changed lines, figure still edited:
     Running `/Volumes/forge/build/0b/72ad0d880ac58c/debug/build/before-991ccbe9849abf36/build-script-build`
  thread 'main' (241427173) panicked at crates/before/build.rs:194:5:
  assertion `left == right` failed: docs/itc_space_consumption_readme.svg is stale relative to results/space_consumption: run `just doc-figure`
    EXIT=101
```

Explanation:

Verified by building: after a warm build, editing a color literal in results/space_consumption/itc_space_consumption.svg and rebuilding at HEAD's build.rs yields `Fresh before` — the script does not rerun and the stale figure goes unnoticed, exactly because build.rs:27-30 lists fuelscape and three docs/ files but neither figure path. Adding `cargo:rerun-if-changed` for the two figure files makes the very next build rerun the script and fail at build.rs:194 with the documented stale message. Both scratch edits were restored afterwards.

## Cleanup status

### Witness agent 1

Complete. (1) Build dir recorded from inside the worktree: /Volumes/forge/build/38/59b4c9f7ee46fe (697M). (2) Test sources archived to <session scratchpad>/witness/1/ (zz_witness_1.rs verbatim; witness_edits.patch = git diff of every edit to existing files: the integral.rs tap, query/tests.rs and board/tests.rs lib tests, suanpan accumulator/tests.rs pins, the verdict_matrix.rs #[ignore], the tests/meter.rs deletion; run logs build.log, runA.log, runA2.log, runB.log, runC.log, runD.log, runE.log; the surface28 fixtures; guest_exports.txt and band_kernels.txt). Every created file deleted (crates/before/tests/zz_witness_1.rs removed) and every edited file restored with `git restore --source=HEAD --staged --worktree` (the lib mutations party.rs, lib.rs, place.rs, watermark.rs, dsi.rs, suanpan accumulator.rs were each restored right after their run). Final `git status --porcelain` output: empty (printed as STATUS:[]). `git rev-parse HEAD` = 9e5784fb4dce977cfbdfd1619886d1482b5ce764. (3) Deleted /Volumes/forge/build/38/59b4c9f7ee46fe and the worktree's local target/; both confirmed absent afterwards.

### Witness agent 2

Complete. (1) Build directory resolved from inside the worktree before any removal: `cargo metadata --no-deps --format-version 1 | jq -r .build_directory` -> /Volumes/forge/build/5a/0c7358e1cdadfa (recorded in the scratchpad's build_directory.txt). (2) Test sources copied to <session scratchpad>/witness/2/ (before_tests_zz_witness_1.rs, suanpan_tests_zz_witness_1.rs, in_tree_additive_tests.diff for the four in-tree additions, mutants_m1.diff / mutants_m2.diff / mutants_codec.diff / mutants_codec_278.diff for the mutations, mutant_run_seed_changes.diff plus stray_seed_place/ and stray_seed_text/ for the proptest seeds the mutant runs wrote, fuelscape_stub.js/.out, fuzzfit_bands_10.py/.out, build*.log and run1..run9.log). Then deleted crates/before/tests/zz_witness_1.rs and crates/suanpan/tests/zz_witness_1.rs, deleted the untracked seed directories crates/before/proptest-regressions/version/skyline/{place,text}/ that failing proptests wrote under the mutants, and restored every tracked file I edited or the mutant runs touched with `git restore --source=HEAD --staged --worktree` (src/meter/board/tests.rs, src/party/ops/index.rs, src/party/tests.rs, src/testing/asymptotics.rs, the mutated codec/build.rs, place/filter.rs, watermark.rs, span/wire.rs, oracle/version.rs, suanpan accumulator.rs, and proptest-regressions/{borsh_impls,party,version/skyline/fill,version/skyline,version}/tests.txt). (3) Deleted /Volumes/forge/build/5a/0c7358e1cdadfa; the worktree had no local target/. Final `git -C <worktree> status --porcelain` output is empty (zero lines); `git rev-parse HEAD` is 9e5784fb4dce977cfbdfd1619886d1482b5ce764.

### Witness agent 3

Cleanup performed in the required order. (1) Build directory resolved from inside the worktree with `cargo metadata --no-deps --format-version 1 | jq -r .build_directory`: /Volumes/forge/build/e4/89a506a00b8a1d (recorded in the scratchpad as build_directory.txt). (2) Test sources archived to <session scratchpad>/witness/3/ as before_zz_witness_3.rs, suanpan_zz_witness_3.rs, and board_tests_witness.diff (alongside build.log, run1.log, run2.log, run3_dark_touch.log, run4_span_rung.log); then crates/before/tests/zz_witness_3.rs and crates/suanpan/tests/zz_witness_3.rs were deleted, and the three edited files were restored with `git restore --source=HEAD --staged --worktree` (crates/suanpan/src/accumulator.rs after run 3, crates/before/src/span.rs after run 4, crates/before/src/meter/board/tests.rs at the end). (3) `rm -rf /Volumes/forge/build/e4/89a506a00b8a1d` (verified absent afterward) and the worktree's local target/ (created 04:09 by my own build, 11 MB) deleted and verified absent. Final `git status --porcelain` output: (empty). HEAD remains 9e5784fb4dce977cfbdfd1619886d1482b5ce764.

### Witness agent 4

Complete. (1) Build directory resolved from inside the worktree via `cargo metadata --no-deps --format-version 1 | jq -r .build_directory` = /Volumes/forge/build/0b/72ad0d880ac58c (recorded in the scratchpad's build_dir.txt). (2) Test sources copied to <session scratchpad>/witness/4/ (zz_witness_4.rs; witness4_in_crate_scratch.diff holding the stack.rs/codec.rs counter instrumentation and the four appended sibling-tests.rs snippets; the append_*.rs sources; build*.log, run1.log, run2.log, run3_m1.log, run4_m2.log, run5_m3.log, deps3_build_{a,b,c}.log). The created file crates/before/tests/zz_witness_4.rs was deleted and every edited file (codec.rs, codec/stack.rs, meter/board/tests.rs, meter/tests.rs, party/tests.rs, version/skyline/masked/tests.rs, plus every mutation target: party/forks.rs, testing/generators.rs, party.rs, meter/registry.rs, meter/board/family.rs, meter/board/ops.rs, tests/verdict_matrix.rs, shape.rs, fuzz/fuzz_targets/fuzz_laws.rs, meter.rs, version/skyline/admit.rs, version/skyline/sweep.rs, build.rs, results/space_consumption/itc_space_consumption.svg) was restored with `git restore --source=HEAD --staged --worktree`. (3) Deleted /Volumes/forge/build/0b/72ad0d880ac58c and the worktree's local target/; both `ls -d` now report No such file or directory. Final `git status --porcelain` output: (empty). `git rev-parse HEAD` = 9e5784fb4dce977cfbdfd1619886d1482b5ce764.

## 32-bit pass

These four constructions (each recorded as inconclusive by the witness agents above) were run in the tree's wasm32-pins executor, a genuinely 32-bit `usize` (wasm32-unknown-unknown under wasmtime), after the completeness critic found that the earlier 32-bit runs had been blocked only by a cap on building detached workspaces.

## suanpan-24

Outcome: demonstrated

Witness agent: 32-bit pass

Test code:

```rust
// Guest exports (crates/before/wasm32-pins/guest/src/witness.rs, scratch; full file preserved at
// <session scratchpad>/witness/wasm32/witness.rs)

/// suanpan-24 (1): `add_limbs_shl([0, 0, 5], 32 * (2^32 - 4))`. The two zero
/// limbs are skipped before any allocation; limb 2's low half lands at digit
/// `2 * 2 + (2^32 - 4) = 2^32`, one past a 32-bit `usize`. Predicted: with
/// overflow checks off the position wraps to 0 and the value reads back as
/// 5 in a single digit; with overflow checks on, a trap.
#[no_mangle]
pub extern "C" fn pin_suanpan_add_limbs_shl_wrap() -> i64 {
    let mut a = Accumulator::new();
    a.add_limbs_shl([0u64, 0, 5], 32 * (u64::from(u32::MAX) - 3));
    let digits = a.digit_count();
    let (sign, limbs) = a.sign_limbs();
    if sign != Ordering::Greater {
        return -1;
    }
    if limbs.len() != 1 {
        return -2;
    }
    if digits != 1 {
        return -3;
    }
    i64::try_from(limbs[0]).unwrap_or(-4)
}

/// suanpan-24 (2): a held `[0, 0, 1]` digit-engine value (`1 << 64`) shifted
/// by `32 * (2^32 - 2)`: `fold_accum` skips offsets 0 and 1, and offset 2
/// lands at `2 + (2^32 - 2) = 2^32`. Predicted: the value reads back as 1
/// with overflow checks off; a trap with them on.
#[no_mangle]
pub extern "C" fn pin_suanpan_shl_wrap() -> i64 {
    let mut a = Accumulator::new();
    a.add_u64(1);
    a.shl(64);
    a.shl(32 * (u64::from(u32::MAX) - 1));
    let (sign, limbs) = a.sign_limbs();
    if sign != Ordering::Greater {
        return -1;
    }
    if limbs.len() != 1 {
        return -2;
    }
    i64::try_from(limbs[0]).unwrap_or(-4)
}

/// suanpan-24 (3), the boundary sibling: one limb at `digit_shift = 2^32 - 1
/// = usize::MAX`. The landing position itself fits; `resize(pos + 1)` is the
/// arithmetic that leaves `usize`. Predicted: a trap in both builds (the
/// overflow check, or `resize(0)` followed by an out-of-bounds index).
#[no_mangle]
pub extern "C" fn pin_suanpan_add_limbs_shl_boundary() -> i64 {
    let mut a = Accumulator::new();
    a.add_limbs_shl([5u64], 32 * u64::from(u32::MAX));
    let (_, limbs) = a.sign_limbs();
    i64::try_from(limbs.len()).unwrap_or(-1)
}

// Harness tests (crates/before/wasm32-pins/harness/tests/zz_witness.rs, scratch; full file preserved at
// .../scratchpad/before/witness/wasm32/zz_witness.rs)

use wasm32_pins_harness::{call0, call1, Outcome, Trap};

const TRAP: Outcome = Outcome::Trapped(Trap::UnreachableCodeReached);

/// suanpan-24 (1), checked guest: the landing position `2^32` is caught by
/// the overflow check as a trap, before any allocation.
#[test]
fn suanpan_add_limbs_shl_wrap_checked_guest() {
    assert_eq!(call0("pin_suanpan_add_limbs_shl_wrap"), TRAP);
}

/// suanpan-24 (1), unchecked guest: the position wraps to digit 0 and the
/// value silently reads back as 5 in one digit.
#[test]
fn suanpan_add_limbs_shl_wrap_unchecked_guest() {
    assert_eq!(call0("pin_suanpan_add_limbs_shl_wrap"), Outcome::Value(5));
}

/// suanpan-24 (2), checked guest: `shl` past `usize` digits traps.
#[test]
fn suanpan_shl_wrap_checked_guest() {
    assert_eq!(call0("pin_suanpan_shl_wrap"), TRAP);
}

/// suanpan-24 (2), unchecked guest: `1 << 64` shifted by `32 * (2^32 - 2)`
/// silently reads back as 1.
#[test]
fn suanpan_shl_wrap_unchecked_guest() {
    assert_eq!(call0("pin_suanpan_shl_wrap"), Outcome::Value(1));
}

/// suanpan-24 (3), checked guest: the `usize::MAX` landing site traps at
/// `resize(pos + 1)`.
#[test]
fn suanpan_add_limbs_shl_boundary_checked_guest() {
    assert_eq!(call0("pin_suanpan_add_limbs_shl_boundary"), TRAP);
}

/// suanpan-24 (3), unchecked guest: `resize(0)` then an out-of-bounds index,
/// still a trap (the harness cannot see the panic message).
#[test]
fn suanpan_add_limbs_shl_boundary_unchecked_guest() {
    assert_eq!(call0("pin_suanpan_add_limbs_shl_boundary"), TRAP);
}
```

Command:

```text
WT=/Users/oxide/src/rumors/.claude/worktrees/wf_2bc55159-381-9; S=<session scratchpad>/witness/wasm32
# build 1 (checked guest, the workspace profile: overflow-checks = true):
cd $WT/crates/before/wasm32-pins && CARGO_BUILD_BUILD_DIR=$S/build-dir-guest cargo build -p wasm32-pins-guest --release --target wasm32-unknown-unknown --target-dir $S/target/wasm32-pins -v
# build 3 (unchecked guest, the profile a consumer's release build has):
cd $WT/crates/before/wasm32-pins && CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS=false CARGO_BUILD_BUILD_DIR=$S/build-dir-guest cargo build -p wasm32-pins-guest --release --target wasm32-unknown-unknown --target-dir $S/target/wasm32-pins-unchecked -v
# run 1 (checked guest):
cd $WT/crates/before/wasm32-pins && WASM32_PINS_GUEST_WASM=$S/target/wasm32-pins/wasm32-unknown-unknown/release/wasm32_pins_guest.wasm CARGO_BUILD_BUILD_DIR=$S/build-dir-harness cargo nextest run --cargo-profile release --target-dir $S/target/wasm32-pins-harness --no-fail-fast -E 'test(/_checked_guest$/)'
# run 2 (unchecked guest):
cd $WT/crates/before/wasm32-pins && WASM32_PINS_GUEST_WASM=$S/target/wasm32-pins-unchecked/wasm32-unknown-unknown/release/wasm32_pins_guest.wasm CARGO_BUILD_BUILD_DIR=$S/build-dir-harness cargo nextest run --cargo-profile release --target-dir $S/target/wasm32-pins-harness --no-fail-fast -E 'test(/_unchecked_guest$/)'
```

Output:

```text
# run 2 (unchecked guest; run2-unchecked.log):
    Starting 3 tests across 4 binaries (59 tests skipped)
        PASS [   0.131s] (1/3) wasm32-pins-harness::zz_witness suanpan_add_limbs_shl_wrap_unchecked_guest
        PASS [   0.133s] (2/3) wasm32-pins-harness::zz_witness suanpan_shl_wrap_unchecked_guest
        PASS [   0.135s] (3/3) wasm32-pins-harness::zz_witness suanpan_add_limbs_shl_boundary_unchecked_guest
     Summary [   0.135s] 3 tests run: 3 passed, 59 skipped
# run 1 (checked guest; run1-checked.log), the suanpan lines:
        PASS [   0.139s] ( 1/11) wasm32-pins-harness::zz_witness suanpan_add_limbs_shl_boundary_checked_guest
        PASS [   0.140s] ( 3/11) wasm32-pins-harness::zz_witness suanpan_add_limbs_shl_wrap_checked_guest
        PASS [   0.144s] ( 8/11) wasm32-pins-harness::zz_witness suanpan_shl_wrap_checked_guest
# profile verification from the -v build logs (grep of the suanpan / wasm32_pins_guest rustc lines):
# checked:   crate-name suanpan	-C overflow-checks=on	--target wasm32-unknown-unknown
# unchecked: crate-name suanpan	-C opt-level=3	--target wasm32-unknown-unknown   (no overflow-checks flag anywhere in the log: `grep -c overflow-checks` = 0)
```

Explanation:

Verified by execution on a genuinely 32-bit usize (wasm32-unknown-unknown under wasmtime 47.0.3). In the unchecked guest (release, rustc's default overflow-checks off, the profile a production consumer ships) construction (1) returned Value(5): the export only returns the limb when sign == Greater, limbs.len() == 1, AND digit_count() == 1, so the finding's predicted `(Greater, vec![5])` with `digit_count() == 1` is observed exactly; construction (2) returned Value(1), the predicted wrong value. Both are silent: no trap, a wrong value read back. In the checked guest (the pins workspace's deliberate `[profile.release] overflow-checks = true`) the same two constructions trap, i.e. the position arithmetic at accumulator.rs:1479 and :566 is exactly what wraps; nothing allocates first, consistent with the finding's `resize`-comes-later reading (zero contributions skipped at 1477/1483 and 563). Construction (3) trapped in both builds, as predicted (still the panic class). What I assessed by reading only: the specific message/site of the (3) panic (overflow at `resize(pos + 1)` line 1371 checked; `resize(0)` then index-out-of-bounds at 1374 unchecked) -- wasm32-unknown-unknown has no stderr and the harness sees only the trap kind, so the finding's 'not the documented message' clause is not observable here. Reproduction of the `# Panics` contract breach (lines 319-323 promise a panic or allocation failure; the unchecked build delivers a wrong value) is direct. Note for the coordinator: the tree's own 32-bit executor could never have observed this as a wrong value because its guest profile turns wraps into traps by design; and no existing pin reaches suanpan's shift entry points at all.

## codec-bits-23

Outcome: demonstrated

Witness agent: 32-bit pass

Test code:

```rust
// Guest (witness.rs)

/// A lazily synthesized skyline stream for the borsh door: the leaf flag
/// `1`, the gamma prefix of `k` zero bits, then the mantissa's leading `1`
/// at bit position `k + 1`. The stream ends (`UnexpectedEof`) after the byte
/// holding that `1`, so a decode that survives `set_bit(k)` fails as a typed
/// error on its next read instead of streaming `k` mantissa bits. Nothing is
/// buffered on this side: one byte per `read` call.
struct WideGammaStream {
    k: u64,
    next_byte: u64,
}

impl Read for WideGammaStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let lead = self.k + 1;
        let lead_byte = lead / 8;
        if self.next_byte > lead_byte {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "the witness stream ends after the mantissa lead",
            ));
        }
        let mut byte = 0u8;
        if self.next_byte == 0 {
            byte |= 0x80; // the leaf flag ending an empty topology run
        }
        if self.next_byte == lead_byte {
            byte |= 0x80 >> (lead % 8); // the mantissa's leading 1
        }
        buf[0] = byte;
        self.next_byte += 1;
        Ok(1)
    }
}

/// codec-bits-23: borsh-deserialize a `Version` whose first leaf height has
/// a `k`-zero gamma prefix. The per-bit reader calls `UBig::set_bit(k)` after
/// the prefix alone. Predicted: `k = 2^32 - 32` traps inside the backend
/// (`Buffer::push`'s capacity assert after `allocate` clamped the capacity
/// to `2^27 - 1` words); `k = 2^32 - 33` allocates, sets the bit, then meets
/// the stream's end: a typed error, returned as -1.
#[no_mangle]
pub extern "C" fn pin_version_borsh_wide_gamma(k: u64) -> i64 {
    let mut reader = WideGammaStream { k, next_byte: 0 };
    match <Version as BorshDeserialize>::deserialize_reader(&mut reader) {
        Ok(_) => 1,
        Err(_) => -1,
    }
}

// Harness (zz_witness.rs)

/// codec-bits-23: a `2^32 - 32`-zero gamma prefix passes the `usize` guard
/// and `set_bit(k)` traps inside the big-integer backend.
#[test]
fn version_borsh_wide_gamma_at_loose_guard_checked_guest() {
    assert_eq!(
        call1("pin_version_borsh_wide_gamma", (1u64 << 32) - 32),
        TRAP,
    );
}

/// codec-bits-23, lower adjacency: at `2^32 - 33` zeros the same-size buffer
/// allocates and the bit is set; the decode proceeds to the stream's end and
/// returns a typed error (-1), so the trap above is the guard's, not memory's.
#[test]
fn version_borsh_wide_gamma_below_loose_guard_checked_guest() {
    assert_eq!(
        call1("pin_version_borsh_wide_gamma", (1u64 << 32) - 33),
        Outcome::Value(-1),
    );
}
```

Command:

```text
WT=/Users/oxide/src/rumors/.claude/worktrees/wf_2bc55159-381-9; S=<session scratchpad>/witness/wasm32
cd $WT/crates/before/wasm32-pins && CARGO_BUILD_BUILD_DIR=$S/build-dir-guest cargo build -p wasm32-pins-guest --release --target wasm32-unknown-unknown --target-dir $S/target/wasm32-pins -v
cd $WT/crates/before/wasm32-pins && CARGO_BUILD_BUILD_DIR=$S/build-dir-harness cargo nextest run --cargo-profile release --target-dir $S/target/wasm32-pins-harness --no-run
cd $WT/crates/before/wasm32-pins && WASM32_PINS_GUEST_WASM=$S/target/wasm32-pins/wasm32-unknown-unknown/release/wasm32_pins_guest.wasm CARGO_BUILD_BUILD_DIR=$S/build-dir-harness cargo nextest run --cargo-profile release --target-dir $S/target/wasm32-pins-harness --no-fail-fast -E 'test(/_checked_guest$/)'
```

Output:

```text
        PASS [  29.695s] (10/11) wasm32-pins-harness::zz_witness version_borsh_wide_gamma_below_loose_guard_checked_guest
        PASS [  29.744s] (11/11) wasm32-pins-harness::zz_witness version_borsh_wide_gamma_at_loose_guard_checked_guest
     Summary [  29.745s] 11 tests run: 11 passed, 51 skipped
```

Explanation:

Verified by execution on wasm32: borsh-deserializing a `Version` from a stream of leaf flag + 2^32 - 32 zero bits + the mantissa's leading 1 traps (`Trap::UnreachableCodeReached`, the guest-panic trap) with no mantissa byte ever read -- my reader ends the stream right after the lead byte, and the trap fires before that end is reached. The lower adjacency at 2^32 - 33 zeros returned Value(-1): `set_bit(k)` succeeded (it allocates the identical 2^27 - 1-word buffer, since `default_capacity` clamps both to `MAX_CAPACITY`), the decoder went on to read mantissa bits, hit the stream's end, and surfaced a typed error. The same working set (the ReaderCursor's ~512 MiB byte buffer plus the 512 MiB backend buffer) therefore fits the 4 GiB guest, so the trap at 2^32 - 32 is not memory exhaustion; this resolves the earlier witness's open caveat about the ~1 GiB working set. Each run took ~29.7 s (the 2^32 per-bit reads at borsh_impls.rs:95-103). Assessed by reading, not observed: the trap's exact site, `Buffer::push`'s `assert!(self.len < self.capacity)` after `push_zeros(idx - 2)` fills the clamped buffer (dashu-int 0.5.1 bits.rs:551-561, buffer.rs:57-59, :121-123, :205-206, :230-231; the 32-bit arch branch is selected for wasm32 at arch/mod.rs:43-50) -- the wasm32 guest has no stderr, so the panic message is invisible to the harness. Note: the pins workspace's Cargo.lock pins dashu-int 0.5.1 while the root Cargo.lock pins 0.5.0 (the finding quotes 0.5.0); the cited lines are the same in both versions, and the run above exercised 0.5.1. Not exercised: the word-parallel twin at dsi.rs:255-267, which the finding says needs a `2k + 1`-live-bit slice; the existing pin `version_decode_memory_terminal_traps` records that the byte door's working set exhausts memory first at that size.

## clock-17

Outcome: demonstrated

Witness agent: 32-bit pass

Test code:

```rust
// Guest (witness.rs)

/// clock-17: `Clock::seed().forks(k).len()`, the same predictions as the
/// party twin (the clock iterator delegates its hint to the party split).
#[no_mangle]
pub extern "C" fn pin_clock_forks_len(k: u64) -> i64 {
    let mut c = Clock::seed();
    let it = c.forks(k);
    i64::try_from(it.len()).unwrap_or(-1)
}

/// clock-17: the `size_hint` behind `len`, coded as the party twin.
#[no_mangle]
pub extern "C" fn pin_clock_forks_size_hint(k: u64) -> i64 {
    let mut c = Clock::seed();
    let it = c.forks(k);
    match it.size_hint() {
        (lower, Some(upper)) if upper == lower => i64::try_from(lower).unwrap_or(-1),
        (_, None) => -2,
        _ => -3,
    }
}

// Harness (zz_witness.rs)

/// clock-17, adjacency: `Clock::forks(2^32 - 1).len()` returns `usize::MAX`.
#[test]
fn clock_forks_len_at_usize_checked_guest() {
    assert_eq!(
        call1("pin_clock_forks_len", (1u64 << 32) - 1),
        Outcome::Value(4_294_967_295),
    );
}

/// clock-17: `Clock::forks(2^32).len()` traps in the default `len`.
#[test]
fn clock_forks_len_past_usize_checked_guest() {
    assert_eq!(call1("pin_clock_forks_len", 1u64 << 32), TRAP);
}

/// clock-17: the hint behind that trap is `(usize::MAX, None)`.
#[test]
fn clock_forks_size_hint_past_usize_checked_guest() {
    assert_eq!(
        call1("pin_clock_forks_size_hint", 1u64 << 32),
        Outcome::Value(-2),
    );
}
```

Command:

```text
WT=/Users/oxide/src/rumors/.claude/worktrees/wf_2bc55159-381-9; S=<session scratchpad>/witness/wasm32
cd $WT/crates/before/wasm32-pins && WASM32_PINS_GUEST_WASM=$S/target/wasm32-pins/wasm32-unknown-unknown/release/wasm32_pins_guest.wasm CARGO_BUILD_BUILD_DIR=$S/build-dir-harness cargo nextest run --cargo-profile release --target-dir $S/target/wasm32-pins-harness --no-fail-fast -E 'test(/_checked_guest$/)'   # (same guest and harness builds as codec-bits-23)
```

Output:

```text
        PASS [   0.140s] ( 2/11) wasm32-pins-harness::zz_witness clock_forks_len_past_usize_checked_guest
        PASS [   0.141s] ( 4/11) wasm32-pins-harness::zz_witness clock_forks_size_hint_past_usize_checked_guest
        PASS [   0.141s] ( 5/11) wasm32-pins-harness::zz_witness clock_forks_len_at_usize_checked_guest
```

Explanation:

Verified by execution on wasm32: `Clock::seed().forks(1u64 << 32).len()` traps (guest panic), `size_hint()` at the same k reports an upper bound of `None` (the export's -2 code), and the adjacency `forks((1u64 << 32) - 1).len()` returns 4294967295 = usize::MAX without panicking. So the panic threshold is exactly k = 2^32, reached from caller input in O(1) (the run took 0.14 s), as the finding claims; the trap is the default `ExactSizeIterator::len`'s `assert_eq!(upper, Some(lower))` (toolchain 1.97.1 exact_size.rs:116-123, read earlier in this review), which is the only assertion on that path. One narrative correction: the finding says `usize::try_from(4294967297).ok()` is `None`; the value actually hinted is 4294967296 -- `Forks::new` (party/forks.rs:113-118) builds the split with `k + 1` shares and immediately draws the residual, so `remaining` is `k` = 2^32 when `len()` runs. The conclusion is unchanged (2^32 also exceeds a 32-bit usize), and the adjacency result confirms 2^32 is the minimal panicking k. The checked/unchecked profile is irrelevant here (the panic is an assert, not an overflow check); the test ran under the checked guest only.

## party-13

Outcome: demonstrated

Witness agent: 32-bit pass

Test code:

```rust
// Guest (witness.rs)

/// party-13: `Party::seed().forks(k).len()`. Predicted: `k = 2^32 - 1`
/// returns 4294967295 (the count remaining after the residual is drawn fits
/// `usize`); `k = 2^32` traps in the default `ExactSizeIterator::len`, whose
/// `assert_eq!(upper, Some(lower))` fails on a `(usize::MAX, None)` hint.
#[no_mangle]
pub extern "C" fn pin_party_forks_len(k: u64) -> i64 {
    let mut p = Party::seed();
    let it = p.forks(k);
    i64::try_from(it.len()).unwrap_or(-1)
}

/// party-13: the `size_hint` behind `len`: the shared bound when both bounds
/// agree, -2 when the upper bound is `None`, -3 when the bounds differ.
#[no_mangle]
pub extern "C" fn pin_party_forks_size_hint(k: u64) -> i64 {
    let mut p = Party::seed();
    let it = p.forks(k);
    match it.size_hint() {
        (lower, Some(upper)) if upper == lower => i64::try_from(lower).unwrap_or(-1),
        (_, None) => -2,
        _ => -3,
    }
}

// Harness (zz_witness.rs)

/// party-13, adjacency: `forks(2^32 - 1)` leaves `2^32 - 1 = usize::MAX`
/// shares after the residual is drawn, so `len()` returns it.
#[test]
fn party_forks_len_at_usize_checked_guest() {
    assert_eq!(
        call1("pin_party_forks_len", (1u64 << 32) - 1),
        Outcome::Value(4_294_967_295),
    );
}

/// party-13: `forks(2^32).len()` traps in the default `len`.
#[test]
fn party_forks_len_past_usize_checked_guest() {
    assert_eq!(call1("pin_party_forks_len", 1u64 << 32), TRAP);
}

/// party-13: the hint behind that trap is `(usize::MAX, None)`.
#[test]
fn party_forks_size_hint_past_usize_checked_guest() {
    assert_eq!(
        call1("pin_party_forks_size_hint", 1u64 << 32),
        Outcome::Value(-2),
    );
}
```

Command:

```text
WT=/Users/oxide/src/rumors/.claude/worktrees/wf_2bc55159-381-9; S=<session scratchpad>/witness/wasm32
cd $WT/crates/before/wasm32-pins && WASM32_PINS_GUEST_WASM=$S/target/wasm32-pins/wasm32-unknown-unknown/release/wasm32_pins_guest.wasm CARGO_BUILD_BUILD_DIR=$S/build-dir-harness cargo nextest run --cargo-profile release --target-dir $S/target/wasm32-pins-harness --no-fail-fast -E 'test(/_checked_guest$/)'   # (same guest and harness builds as codec-bits-23)
```

Output:

```text
        PASS [   0.142s] ( 6/11) wasm32-pins-harness::zz_witness party_forks_len_past_usize_checked_guest
        PASS [   0.143s] ( 7/11) wasm32-pins-harness::zz_witness party_forks_size_hint_past_usize_checked_guest
        PASS [   0.144s] ( 9/11) wasm32-pins-harness::zz_witness party_forks_len_at_usize_checked_guest
```

Explanation:

Verified by execution on wasm32: `Party::seed().forks(u64::from(u32::MAX) + 1).len()` (k = 2^32) traps; `size_hint()` at that k has upper bound `None` (the (usize::MAX, None) spelling at party/forks.rs:69-72, which `impl ExactSizeIterator for Split` at :76 and `for Forks` at :132 inherit the default `len()` over); and the adjacency k = 2^32 - 1 returns len() = 4294967295 = usize::MAX with no panic, so k = 2^32 -- exactly the value the finding names -- is the minimal panicking input, and the panic is O(1) from caller input with no `# Panics` section on `Party::forks` (party.rs:274). One narrative correction, same as clock-17: after the residual is drawn `remaining` is 2^32, not the finding's '2^32 + 1' (`Split::new(whole, k.saturating_add(1))` then one `next()` at party/forks.rs:114-117); the mechanism and conclusion stand. The trap kind is `UnreachableCodeReached` (a guest panic); attribution to the std default `len`'s `assert_eq!(upper, Some(lower))` is by reading (the only assertion on the path), since the guest has no stderr.

## Cleanup status: 32-bit pass

Complete. (1) Build directories recorded from inside the worktree: root workspace `/Volumes/forge/build/b0/3e4af26f5e7ed8`, wasm32-pins workspace `/Volumes/forge/build/94/d509fc1cd21207` (via `cargo metadata --no-deps --format-version 1 | jq -r .build_directory`, plain and with `--manifest-path crates/before/wasm32-pins/Cargo.toml`). Neither was ever created: every build ran with `CARGO_BUILD_BUILD_DIR` pointed into the scratchpad, and `ls -d` on both paths reports 'No such file or directory' before and after. (2) Test sources copied to <session scratchpad>/witness/wasm32/ (witness.rs, zz_witness.rs, plus hook-edits.diff, git-status-before-cleanup.txt, build-guest-checked.log, build-guest-unchecked.log, build-harness.log, run1-checked.log, run2-unchecked.log). Deleted the two created files (guest/src/witness.rs, harness/tests/zz_witness.rs); restored the three edited files with `git restore --source=HEAD --staged --worktree crates/before/wasm32-pins/Cargo.lock crates/before/wasm32-pins/guest/Cargo.toml crates/before/wasm32-pins/guest/src/lib.rs`. Final `git status --porcelain` output: (empty); `git diff --stat HEAD`: (empty); HEAD unchanged at 7440d1a3af5446eefdd79b031b5a18adc9fe4320. (3) Deleted the redirected directories under the scratchpad: target/ (748K), build-dir-guest (154M), build-dir-harness (617M). No worktree-local `target/` exists at the root or under crates/before/wasm32-pins (both `ls -d` report 'No such file or directory'). Nothing under /Users/oxide/src/rumors or any /Users/oxide/src/rumors-* was touched; no git state-changing command was run other than the sanctioned `git restore` on the three files I had edited in this worktree.
