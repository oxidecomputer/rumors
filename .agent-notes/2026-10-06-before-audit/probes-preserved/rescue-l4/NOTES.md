# rescue-l4 NOTES (resumption record)

Task: catalogue every instrument on explore/l4-measures (tip 2c82c7bac, base 58285ca51)
per coordinator-briefs/instrument-rescue.md. Output:
.agent-notes/2026-10-06-before-audit/instrument-rescue/04-measures.md. Do not commit.
Box runs <= 3 (census/runtime only), scratch worktree /Users/oxide/src/rumors-rescue-l4.
Mark every claim verified / reported / inferred. Part IV style.

## Read (done)
- common.md, auditor.md, rescue brief, README, baseline, lane brief L4, all round-1 records
- instruments.md, survey 2.1/2.3/4/5 + rulings, ranker handoff-COMMON.md
- QUESTIONS #28,#29,#31,#53,#63,#72,#85, notices 96/98/88
- whole diff (8 commits, +1369), auditor-l4 scratch (mutants-integral.patch, logs), box l4logs tails
  (copy: box-l4logs-tails.txt)

## Key facts (verified unless marked)
- lane prod files identical base->main except suanpan add.rs (limb fix) + wasm pins
- committed: RANK_TRIPLE laws (stream_rank: odd numerators, exp 0..1e5 -> integral ranks ~never),
  rank_cmp 25k alignment oracle, rank_sum_equals_pairwise_fold, rank_formatting_behaves_as_text (ONE point "101.01"),
  canonical_rank_text (<=129 bits each side), rank_is_a_valuation law (version_pair), measure/tests.rs
  arbitrary_trees_agree (arb_oracle_version: depth<=4, nodes<=21, heights p50 128 max 514 bits per L8 census)
  + named freeze families with FREEZE_HITS floors.
- L4 census at final settings (box sweep3-none.log): 2000 cases froze 1730, deferred 1015, >=3 defer 522,
  distinct ties 581, frozen ties 511, max_bits 2209. Deep regime (deep-1.log): 1729/1061/619/545/476, max 6000.
- L8 survivor rank-42 (rank.rs:989 AddAssign<&Rank> -> ()) missed by committed suite; L4 check_values does
  `acc += &b` vs oracle -> would kill (INFERRED, not run). No other lane-file survivor L4 catches (C/E/U/decode-reader G).
- Rank mutants R1-R7,E8 and schedule probes E13-15: source NOT saved (only logs on box). Integral mutants M*: patch saved.
- wasm: #29 carries Sum cases (5,6,7); #31 carries panic message but NOT memory pages. Cases 9-12 (O6) not carried.
  #28 renames reserve_digits->reserve_bits (replay case 12 spelling).
- runtimes known: wasm cases 7/8 ~26 s/15 s alone; 5-10 together 120 s; 11/12 ~15 s. Default-config native
  runtimes of L4 tests UNKNOWN (L4_CASES unrecorded) -> measure.

## Instruments (entries planned)
A gen arb_leaves+arb_related; B flat oracle; C check_pair property; D rational oracle+gens; E rank values prop;
F Sum any order; G seven routes; H Count sweep; I DEFER_HITS+coverage census; J cost_search; K tie cost;
L sum order cost; M integral mutant patch; N E13-15 schedule probes (lost); O R mutants (lost);
P wasm cases 9-12; Q wasm pages readout. One-liners: #29, #31.

## Box plan
1 build --no-run at explore tip + census file rescue_l4_census.rs; 2 run census + explore_l4* timings.

## Next
- set up worktree, write census, run, then write 04-measures.md

## Milestone: box runs done (2 of 3 used), worktree removed
- box1-build.log: --no-run build at explore tip + census patch, exit 0 (3m36s)
- box2-run.log: 15 tests pass, load 28-37. Census (2000 pairs): L4 froze 86.5 defer 50.8 >=3 26.1 ties 29.1
  equal 9.7, depth p50 12 p90 86 max 130, leaves p50 43 max 175, bits p50 2019 max 2209, bytes p50 3887 max 58888.
  committed arb pairs: froze 24.0 defer 0 ties 0 equal 0 depth max 4 bits p50 128 max 513.
  organic: froze 0 defer 0 ties 1.6 equal 34.8 (6342 pairs).
  rank: L4 integral 33.3% even-int 24.5% zero 6.8% equal-value pairs 30.0 same-exp 43.0, exp p50 9 max 5172, text p50 217
  committed stream_rank: integral 0 even-int 0 equal 0 same-exp 0.01, exp p50 50050, text p50 50052.
  runtimes (s): cost_search 5.7, tree xcheck 0.56, gen coverage 8.3, measures prop 3.2, tie cost 5.9,
  count 0.05, sum any order 0.23, sum order cost 0.32, rank values 0.17, seven routes 0.96.
- census code saved: rescue-census.patch (applies at explore tip). Worktree removed (plain). Box mirror
  ~/src/rumors-rescue-l4 (765M) left for coordinator.
- mutant patch: applies at explore tip, NOT at main (DEFER_HITS context).
- Findings beyond catalogue: (1) rank_sum_mixed_envelope deleted in 193a14744 consolidation; board rank_sum adds
  integers only -> no committed meter of ascending-fraction Sum (closed fix rank-20). (2) measure/tests.rs docs claim
  arb_magnitude "tops out near 2^128" / "128-bit ceiling": false (513 bits; 24% of arb pairs freeze; 0 defer).
  (3) Count TryFrom committed test oracle = same num-bigint call (common-mode).
- 00-baseline.md now exists (main ce67ab083; nextest limit 300 s). main now dbc169291, no lane code change.

## Next: write instrument-rescue/04-measures.md, then hand back.

## Done
- Wrote instrument-rescue/04-measures.md (17 full entries + 2 carried one-liners), not committed.
- Cross-references and arithmetic re-checked (E8 projection k/12 per byte: ~11, 21, 43, 85).
- Remaining: hand back report.
