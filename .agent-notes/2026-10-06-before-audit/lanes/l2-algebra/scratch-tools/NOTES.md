# auditor-l2 resumption record (lane L2, version algebra)

Worktree `/Users/oxide/src/rumors-audit-l2-algebra`, branch `explore/l2-algebra`,
base `58285ca5`. Scratch: this directory (`S` below). Box copy:
`~/src/rumors-audit-l2-algebra` (builds in its own `target/`, ~1 GB).

## Instruments I built (explore branch only)

- `crates/before/tests/l2_probe/` — public-API differential probe.
  - `model.rs`: independent leaf-list model (versions as `(depth, height)`
    leaves, parties as `(depth, owned)` regions), exact dyadic refinement over
    BigUint positions, canonicalization by stack merge, independent byte
    encoders for versions and parties, `canon_stats` counting the writer's
    collapse paths, `overlay`, `canon_party`.
  - `gen.rs`: byte-tape generators (shrink toward shallow/zero). Partition
    strategies: bushy, left spine, right spine, random-walk deepening, combs.
    Heights = base + offset straddling the 63-bit code boundary (deltas at
    2^31, absolutes at 2^32 − 2) and far past the word. Pair families:
    independent, perturbed (± sparse), join/meet decompositions (force collapse
    cascades), equal. Parties with at least one owned region.
  - `main.rs`: `algebra_matches_model` (default 256 cases; ~48 s) checks every
    join/meet operator cell and assign form, span and `^`, bit and byte size
    bounds, all comparison spellings (owned/borrowed, clone and re-decoded
    copies), hash on equality, projection materialization and every
    view/version/view comparison cell, `shape`, `combine` (N=0,2,3),
    `Clock::shape`, `Party::shape`, `is_empty`/`Default`, chained production
    results, span-decoded endpoints (shared allocation), folds (`join_all`,
    `meet_all`, `span_all` owned/borrowed, `Sum`, `collect`; up to 40 items
    with clones, re-decodes, adjacent repeats), span `place`/`dominance`/
    `precedence`/`contains` (cross-lane, L5's theme). `par00..par15`: ignored
    parallel copies for long runs. `census` (ignored): generator histograms.
    `deep_surfaces_are_stack_safe`, `deep_fork_halves_are_stack_safe`:
    depth-100k surfaces; `deep_calibration_recursion_overflows` (ignored):
    aborts with stack overflow by design.
  - Env: `PROPTEST_CASES`, `PROPTEST_MAX_SHRINK_ITERS`.
- `crates/before/tests/l2_cost/main.rs` — meter probes (needs
  `--features touch-meter,scan-meter`): ripple, ripple-close,
  fragmented-mask, fragmented-mask-stepping families.
- `S/mutate.py` + `S/mutants1.json`, `S/mutants2.json`, `S/mutants-all.json`:
  string-swap mutation driver (restores and checks `git diff --quiet`).

## Established (with evidence)

- Probe passes: 256-case runs at each revision (latest `S/run9.log`, exit 0)
  and a long run of 16 × 2000 = 32,000 cases (`S/long1.log`, all PASS) at
  `2e557419` (before the overlay/emptiness/chained/span-decoded additions).
- Census (`S/census1.log`, 2000 tapes): relations Less 399 / Equal 522 /
  Greater 312 / None 767; depth up to 65+ in 359; joins with direct wide
  collapse 375, cascade wide 329, narrow cascade after a split 167.
- Mutation calibration: 20 mutants (`S/mut/round1`, `S/mut/round2-probe`)
  all killed by the probe; all 20 also killed by the committed `before`
  suite (`S/mut/round2-suite`, first failing test per mutant in each log).
  So: the probe catches no first-order mutant the committed suite misses.
- Cost (`S/cost1.log`, `S/cost2.log`): every lattice, order, fold, and
  masked comparison op is flat per input bit across 8× growth on carry-ripple
  and fragmented-mask families; `project` grows with its output (~2n² bits),
  as documented.
- Committed deep-input proof (`src/clock/tests.rs`
  `deep_tree_stack_safety`, `deep_tree_query_and_causal_stack_safety`) does
  not drive shape walks, the concurrent-pair hull, or `Sum`/`collect`; my
  depth-100k probe shows they are safe today, and the calibration shows the
  depth catches a one-frame-per-level recursion on a 2 MiB stack.
- Writer, BitStack, PackedU64Stack, and BitsWriter indexing reviewed by hand;
  no 32-bit narrowing hazard (byte index = u64 position / 8).
- Model bug fixed along the way: `Clock::shape` refines by the canonical
  party; my first overlay model used the raw region list (production correct).

## Defects

None confirmed.

## Background jobs

None running. Final long run done: `S/long2.log`, 16 × 1400 = 22,400 cases at
`d78c6129`, all PASS.

## Deliverables (written)

`coverage.md`, `machinery-deep-surfaces.md`,
`simplification-lattice-entry-delegation.md`,
`simplification-lattice-one-sweep.md`, `observations.md`.

## Next

Report round 1 (no defects; diminishing returns). If resumed: the leads in
the report's "what I would examine next" list.

# Round 2 (bounded task from the coordinator, 2026-10-07)

Task: run the algebra-relevant L8 survivors (version, span, rest groups;
diffs under `.agent-notes/2026-10-06-before-audit/lanes/l8-adequacy/round-2/survivors/diffs/`)
against the probe; settle the `projection.rs:420` tie order; base is `main`
`1745d377`; at most two box runs; records committed on `explore/l2-algebra`
only; report by hand-back. Also say whether results change under
`simplify/lattice-one-sweep` (cdba4b5f6), `simplify/lattice-entry-delegation`
(6efb45624), `simplify/clone-free-dedup-and-moves` (ab548dafd).

- Merged `1745d377` into the explore branch (merge commit `5c53b5d26`);
  `main` changed no lane source file since `58285ca5`.
- Patches: `crates/before/tests/l2_probe/mutants/00..17` (L8 survivors,
  extracted by `S/extract.py` from `S/selected.txt`; 08 regenerated because
  the markdown lost a blank context line), `18`, `19` (tie order reversed in
  `projection.rs` and `place.rs` priorities). Loop: `mutants/run.sh` (commit
  `b2dff4dc8`).
- Box run 1: `on-illumos.sh <wt> 'unset CARGO_TARGET_DIR; export NEXTEST_TEST_THREADS=24; bash crates/before/tests/l2_probe/mutants/run.sh'`,
  local log `S/round3-mutants.log`, per-mutant logs on the box in
  `~/src/rumors-audit-l2-algebra/target/l2-mutants/` (copy with scp).
- Tie-order argument (by reading): the four CursorSets (projection
  Comparison, place Cursors, filter MemberCursors and SpanCursors) step only
  by folding additive deltas into accumulators; filter's `Comparison::fold`
  reads widths to drop a private difference, which `read` rebuilds exactly
  from the always-updated absolute heights. Reads happen only between
  `advance_set` calls; a slot's depth changes only when it steps, so the set
  of stepped slots is order-independent. Verdicts are order-insensitive;
  cost can differ in the filter.
- Box run 1 done (`S/round3-mutants.log`, per-mutant logs copied to
  `S/round3/`): baseline and all 20 mutants pass the probe (18 tests each:
  16 × 200 cases + 2 deep), each rebuilt, each restored.
- Box run 2 (DONE; trial merge aborted, tree clean at `1023d3ea8`): it ran on an uncommitted trial merge of
  `simplify/clone-free-dedup-and-moves` + `simplify/lattice-one-sweep` on top
  of `1023d3ea8`. After the run, `git -C <wt> merge --abort` and confirm
  `git status --short` is empty and HEAD is `1023d3ea8`. Command:
  `bash crates/before/tests/l2_probe/mutants/run_on_tree.sh` with patches 01,
  02, 14, 15; local log `S/round3-branches.log`; box logs in
  `~/src/rumors-audit-l2-algebra/target/l2-mutants-tree/`.
- Run 2 result: baseline and mutants 01, 02, 14, 15 pass (19 tests each).
- Record committed: `.agent-notes/2026-10-06-before-audit/lanes/l2-algebra/round-2/report.md`
  at `6748bcd41`. Nothing running. Next: hand back.
