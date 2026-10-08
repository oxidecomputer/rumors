# rescue-l6 NOTES (resumption record)

Task: catalogue every instrument on `explore/l6-codecs` (tip 431011b3c, base 58285ca51)
per `coordinator-briefs/instrument-rescue.md`. Output:
`/Users/oxide/src/rumors/.agent-notes/2026-10-06-before-audit/instrument-rescue/06-codecs.md`
(do not commit). Template per instrument: what it is / reaches / coverage beyond committed /
evidence / fold-in cost / overlaps / dependencies / value. Mark verified/reported/inferred.
Box runs: <= 3, only for census/runtime the records lack; scratch worktree
`/Users/oxide/src/rumors-rescue-l6` (detached at branch; remove with plain `git worktree remove`).
Read branches with `git show`; never check out in main worktree.

## Read so far
- common.md, auditor.md, instrument-rescue brief, README, baseline, lane-l6 brief
- lanes/l6-codecs/round-1/* (all 12 files), instruments.md, instrument-survey.md (all)
- QUESTIONS.md header, notices 96/98/88, entries 51, 53, 67, 78, 85
- ranker handoff-COMMON.md (no handoff-l6 at first look)
- 00-baseline.md does NOT exist yet (instrument-rescue/ empty at start)

## Established
- 9 commits, 11 files, +3020 lines (git diff --stat).

## Next
- Read every added file on the branch via git show.
- Check survey's claim that spec harness misses the 2 padding-precedence survivors.
- Check L8 census numbers for committed generators (baseline proxy).

## Established (2026-10-08, by reading; all at main 434cbfc82 unless noted)
- Read every added file: l6_probes.rs, l6_spec.rs (saved here), l6_writer.rs, tests/l6_resource.rs,
  wasm32 protocol/guest checks+synthesis/harness pins, guest Cargo.toml (+borsh feature), lock.
- Codec src identical base 58285ca51 -> main (git diff --stat empty); only wasm32-pins drift (SuanpanLanding case 4).
- APIs used by explore files exist at main (bridge, VersionWriter, VersionSubtree, Rank::from_raw/raw_parts pub(crate)).
- main reader tests: only Rank::decode via OneByteReader + FailedTail chain; no Interrupted anywhere; no failing writer
  (OneByteWriter partial writes only, for rank/composites). Serde tests: no accept-set/class comparison with slice decode.
- Board: rejection rows for version/span/party/clock only; none for rank/ranked rejection (ops.rs names).
- L8 census (lanes/l8-adequacy/round-1/census-baseline.txt): arb_oracle_version depth max 4, nodes max 21, heights p50 128 bits max 514;
  arb_oracle_party_nonempty depth max 4, bits max 26. Spec generator heights are NOT wider (reach-hist.log).
- Logs verified (auditor-l6 scratch): spec2.log 11 tests at default counts 10.31 s total (libtest, debug); spec-long2 1940 s 300k;
  adequacy.log counts M1..M13 = 9,93,55,83,17,1,3,4,13,5,24,58,2; adequacy2 M14/M15 caught by 2 committed tests each;
  writer W1 by 60; wasm1/wasm2 probe times 124-148 s release.
- Ready carriers: #51 (JSON-array leniency property), #53 (rank restructure), #67 (3 simplifications), #78 (L8 Rank reader property),
  #85 (docs incl. CBOR bridging sentence). Polished wasm32 probe versions on NON-ready branch
  fix/before-wasm32-buffer-growth commit 8208efaeb (to be deleted at retirement unless owner keeps it).
- #92: guest builds without borsh; borsh check in guest needed for position pins. L6 borsh at-limit probe reaches
  StreamBitsReader::read_bit (borsh_impls.rs:104-106) at positions up to ~2^33 (inferred narrowing would misread).
- Survey claim "spec harness misses G survivors span/wire.rs:136, party/io.rs:29 (> to >=)": by reading prefix_stage, the
  spec's applicable set for a lone first field filling the buffer with dirty padding is {TrailingBits} only, so a Truncated
  report fails judge -> catches IF generated. Census planned. Caveat: this enforces a stage-order precedence the owner left open (#67).
- Mutation scripts mutate*.py, reach5.py exist only in /private/tmp scratch (not copied to lanes/).
- L2 explore model.rs has its own encode_version/encode_party (overlap with spec encoder leg).

## Box run plan (<=3)
1. build --no-run in /Users/oxide/src/rumors-rescue-l6 (detached at explore/l6-codecs) with appended census test
2. nextest -E 'test(/l6_/)' : census + per-test runtimes

## Box runs
- Worktree /Users/oxide/src/rumors-rescue-l6 (detached 431011b3c) + UNCOMMITTED census test appended to
  crates/before/src/testing/l6_spec.rs (rescue_l6_census). Box dir ~/src/rumors-rescue-l6.
- Run 1 (build --no-run, lib tests): exit 0, 3m51s, run1-build.log.
- Run 2 (census RESCUE_N=40000 x 12 threads, libtest): background task budd6xeve -> run2-census.log.
- At the end: git -C worktree diff shows only the census; `git worktree remove` needs a clean tree ->
  restore l6_spec.rs by deleting the appended block (string swap), verify `git diff` empty, then plain remove.

## Entry plan (16 full + 1 one-liner): E1 spec decode differential; E2 near-valid generators; E3 encoder agreement;
E4 first-event model; E5 reach histograms; E6 Chaos reader probe; E7 Sink writer probe; E8 CBOR probes; (JSON->#51);
E9 writer copy harness; E10 resource families; E11 wasm32 borsh wide-leaf (+guest borsh); E12 rank group stream;
E13 reader wide-leaf OOM; E14 Vec growth diag; E15 decoder mutation schemas M1-M17; E16 writer mutants W1-3 + reach R31-33.
- calib-spec2.log: M12 caught only by encoder leg among harness legs; M6 by rank+ranked legs.
- committed single-bit flip tests: soundness re-encode check, depth<=2 exhaustive, <=4 arbitrary.

## Census result (run2-census.log, exit 0, 270 s, 480k span + 480k clock, 40k rank/encoder)
- clock lone party dirty padding: 683/480000 (1 in 703); production TrailingBits all; judge rejects Truncated all.
  default 2000 cases: expected 2.85 -> P(>=1) ~94%.
- span lone lo dirty padding: 48/480000 (1 in 10000); same verdicts. default: expected 0.2 -> ~18%; 300k: ~30.
- span documented precedence narrows report: 29113/480000 (6.1%).
- span first field depth>32: 88338/480000 (18.4%).
- encoder operand depth after normalize: <=4 75.8%, 5..32 4.7%, 33..128 5.0%, >=129 14.5% (19.5% >32).
- rank >64 bytes: 7549/40000 (18.9%), of which accepted 4077.
- Draft written to instrument-rescue/06-codecs.md; fill CENSUS-PENDING, fix entry refs (mutation scripts = entries 10, 14),
  borsh line 107 not 106. Then revert census in worktree, verify diff empty, git worktree remove.

## DONE (2026-10-08)
- Output written: instrument-rescue/06-codecs.md (uncommitted). 16 full entries + 1 one-liner.
- Census code saved: rescue-l6/census-test.rs. Worktree reverted (diff empty), removed with plain `git worktree remove`.
- Box dir ~/src/rumors-rescue-l6 (619M) left for coordinator retirement.
- Next: SubagentHandback report.
