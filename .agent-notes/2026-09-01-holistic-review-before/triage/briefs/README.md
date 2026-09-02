<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as lane briefs derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. Read with the ground rules in ../../README.md. -->

# Lane briefs for P1 through P8

One brief per lane for the instruments phase (P1), the correctness and
cost-cure phase (P2), the vocabulary phase (P3), and the pattern-sweep
phase (P4), the scaffolding-dissolution phase (P5), the module lanes
(P6), the API phase (P7), and the performance phase (P8) of the before
and suanpan triage. Every P1 lane is
based on `bba0e31a` (main, the commit recording the S2 rulings; the tree
outside `.agent-notes/` is byte-identical to the reviewed commit
`9e5784fb`, so every line number in the class documents holds). The P2
lanes name their base by dependency (a main commit carrying the P1 lanes
they build on; the coordinator names the SHA at launch). Every lane is
governed by `../rulings.md`. Each brief quotes its members' Resolution and
Acceptance verbatim from the class documents and names the ruling that
governs each, with any amendment stated beside the quote. A lane agent
reads only its brief; the brief carries the ground rules in full.

Rulings 1 to 103 have individually ruled every high and medium in every lane, P6 included; each P6 brief's former "awaiting individual ruling" entries now carry their ruling (93 to 103) and any amendment beside the quoted Resolution. Ruling 104 approved every lane roster of lows and nits: members marked "roster: approved (ruling 104)" land per their entries, swept with the ruled members, and a lane agent reports rather than chooses when a Resolution conflicts with a ruling, offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89). Ruling 98 dissolves the surface roster's citations in favor of coverage totality; `p6-surface.md` carries that regime and re-reads its rows under it. Ruling 94 (suanpan drops dashu) is in `p7-api.md`; ruling 93's headline tightening is `p2-cures.md`'s final commit.

Two ground rules are new since the first draft and stand in every P6 and
P8 brief (the first in every brief):
ruling 43's direction that reasons, pins, and enforcement homes are typed
references the compiler resolves, never strings naming a test, a file, or
a line, and that a lane which sees a cleaner idiomatic shape for a roster
is authorized to adopt it; and ruling 88's rule that no pin anywhere
fixes a lower bound on performance: every pin is a ceiling, an
improvement lands by tightening it with attribution, and the only floors
are liveness floors derived from a mechanism's irreducible work.

## P1 lanes

| Brief | Rulings | Touches | Size |
|---|---|---|---|
| `p1-gate.md` | 24, 28, 16, 25, 18 (retirement half), 50 (gate-legs-4, gate-legs-8) | `justfile`, `.github/workflows/ci.yml`, `deny.toml`, `rust-toolchain.toml`, `.cargo/mutants.toml`, `tools/mutantcheck*`, `AGENTS.md`, the six lockfiles, `fuzzfit/harness/src/ops.rs`, `src/surface.rs` and the codec law | medium |
| `p1-survivors.md` | 18 (code half), 50 (suanpan-tests-25) | `src/codec/build.rs` tests, `src/codec/bits.rs` tests, `src/version/skyline/watermark/tests.rs`, `crates/suanpan/src/accumulator.rs` and its metered tests | small |
| `p1-harness.md` | 4, 38 (envelope half), 50 (envelopes-a-6), 51 | `crates/before/tests/meter.rs`, one `just test` recipe line | large |
| `p1-board.md` | 11, 12, 13, 9, 38 (board half), 50 (tests-other-27, benches-examples-17) | `src/meter/board/**`, `src/recurse.rs`, `src/meter.rs`'s segments readers, `worst-cases` pins, `benches/version.rs`, two band docs in `tests/meter.rs` | large |
| `p1-fuzz.md` | 14, 15, 17, 10, 50 (fuzz-guests-pins-26, fuzzfit-bands-17, tests-other-26, fuelscape-pipeline-23) | `crates/before/fuzzfit/**`, `crates/before/fuzz/**`, `crates/before/wasm32-pins/**`, `tests/fuzz_seed_set.rs` and the seeds example, `crates/before-fuelscape/src/families.rs`, the `fuzz-build` and `fuzzfit` recipes | large |
| `p1-suites.md` | 19, 20, 21, 22, 27, 50 (envelopes-b-18, skyline-sweep-place-masked-20 and -32, meter-core-8, testing-oracles-22) | `src/meter.rs`, `src/testing/**`, `src/laws.rs`, `tests/{answer_embedded,fold_skeleton,coincident_span}.rs`, bands, rows, and reset sites in `tests/meter.rs`, `src/meter/registry.rs`, `src/version/skyline/query/integral.rs`, suanpan's metered tests | large |

### Placement decisions

- **Ruling 18 is split across two lanes.** The two surviving-mutant tests
  and the suanpan exclusion dissolution are `p1-survivors.md`, not the
  gate lane: no file in common with the CI and recipe edits, and an
  ordering hazard (suanpan-40's `read_digits` restructure changes the
  listed mutant count the `mutantcheck` leg compares; while that leg
  exists the survivors lane's gate would fail on a count that is
  retiring). The survivors lane runs after the gate lane's retirement
  commit lands.
- **Ruling 38 (the segments currency) is split by file.** The harness lane
  deletes the envelope field, every `segments:` pin, and the MEASURED
  fragment as step 2 of the unification (envelopes-a-2, and the
  `tests/meter.rs` half of crate-root-32); the board lane deletes the
  board currency, the readers in `before::meter`, confines
  `SEGMENTS_GROWN` to `cfg(test)`, and restates `recurse.rs`
  (board-ops-render-15, inventory-2, module-graph-1, recursion-1, and
  crate-root-32's board half), after the harness lane has landed so the
  readers it removes have no caller left. The earlier hazard that the
  harness lane must carry the column through unchanged is withdrawn.
- **Ruling 50's fifteen, by the files they touch.** envelopes-a-6 (scan
  floors) inside the harness unification; envelopes-b-18,
  skyline-sweep-place-masked-20 and -32, and meter-core-8 in the suites
  lane, since they add bands and rows to `tests/meter.rs` after the
  harness lands and the suites lane already owns the band work there
  (meter-core-8 also touches `integral.rs` and the `hoisted_window`
  band's re-pin); testing-oracles-22 in the suites lane
  (`src/testing/exhaustive`); fuzz-guests-pins-26 and fuzzfit-bands-17
  in the fuzz lane (the wasm32 guest and the fuzzfit bands);
  tests-other-26 in the fuzz lane (the fuzz seeds, regenerated before
  the seed-replay leg lands); fuelscape-pipeline-23 in the fuzz lane as
  the detached-workspace lane (the fuelscape survey that regenerates the
  committed datasets is not run there); gate-legs-4 and gate-legs-8 in
  the gate lane; tests-other-27 and benches-examples-17 in the board
  lane (the matrix pool is the board's; the bench row is a source edit
  with no bench run); suanpan-tests-25 in the survivors lane beside
  suanpan-40.
- **skyline-query-9 (ruling 10) moved from the suites lane to the fuzz
  lane**, so `fuzzfit/harness/src/bands.rs` has one owner and the rank
  band's pin comes from the lane's single calibration run.
- **The six pattern-placed P1 lows and nits with no ruling** are placed
  as roster members (approved, ruling 104): board-families-floors-judge-19 and
  board-frame-23 in the board lane; meter-adequacy-9 in the gate lane
  (its wasm-guest clause coordinated with the fuzz lane);
  meter-adequacy-10 and skyline-sweep-place-masked-4 in the suites lane;
  suanpan-39 in the survivors lane, where it is the same defect as
  suanpan-tests-25 and is proposed as a `dup` of it.
- **Six P1 rows are deliberately in no brief**: board-families-floors-
  judge-16, codec-base-text-tree-8, clippy-pedantic-1, module-graph-3,
  skyline-sweep-place-masked-35 (their owner-gating settled by ruling 8,
  each waiting on a second owner decision, 51, 52, 71, or 72, ruled in a
  later session) and meter-adequacy-4 (decision 72). They join the lane
  their decision names once it is ruled.

### Independence and launch order

`tests/meter.rs` is the shared file. The harness lane rewrites its four
harnesses into one and deletes the segments column; the board lane edits
two band docs in it (envelopes-a-16, envelopes-b-4); the suites lane
touches every reset site in it (ruling 19), three band docs (ruling 20),
folds three satellite binaries into it (ruling 22), and adds the ruling 50
rows. Run the harness lane first and alone against that file; the board
and suites lanes rebase onto it before their gate runs. If the harness
lane is still running when the others are ready, the others may land
everything outside `tests/meter.rs` and hold their `meter.rs` commits for
the rebase.

Two further orderings from the rulings and TRIAGE.md:

- Ruling 24 (the coverage leg's reproduction) runs before any change to a
  heap pin under coverage. The harness lane's one-time re-measure adds
  limb, scan, and touch pins to rows that lack them; it does not move any
  heap pin, so it may run concurrently with the gate lane. Any heap pin
  the harness lane finds itself moving is a stop.
- The board lane's segments dissolution (step 4 of its ordering) waits for
  the harness lane's landed deletion of the envelope field.

Recommended order, with waves of at most four builders and the disk
checked before each wave:

1. `p1-gate`, `p1-harness`, `p1-fuzz`
2. `p1-survivors` (after the gate lane's roster retirement lands),
   `p1-board` (rebased onto harness), `p1-suites` (rebased onto harness)

The rumors triage runs lanes in the same workspace. Lanes from the two
plans do not run concurrently against `just gate`; the coordinator
sequences them (ruling 5).

## P2 lanes

| Brief | Rulings | Base | Touches | Size |
|---|---|---|---|---|
| `p2-rows.md` | 1, 2, 29, 30, 31, 32, 36, 37 (instrument halves), 45, plus crate-root-40 | after `p1-harness` (and `p1-board` if landed) | `tests/meter.rs` rows and header, the board families, `src/codec/stack.rs`'s scan charge, `src/party/tests.rs`, `src/shape/tests.rs` | large |
| `p2-cures.md` | 1, 2, 29, 30, 31, 32, 36 (cure halves) | after `p2-rows` and `p2-widths`' `memo.rs` widening | `src/version/skyline/{build,masked,overlay,fill,text}.rs`, `src/version/{ranked,rank}.rs`, `src/causally/**`, the fuelscape islands and `fuelscape/*.json` | large |
| `p2-widths.md` | 33, 34, 35, 39, 41 | `bba0e31a` or above; wasm32 pins in CI once `p1-gate` lands | `src/version/skyline/fill/memo.rs`, `query/web.rs`, `src/party/ops/index.rs`, the literal composers, `src/{clock,party}/forks.rs`, `tests/forks_max.rs`, `crates/suanpan/src/accumulator.rs`, `src/codec/{gamma,dsi}.rs`, the `wasm32-pins` guest and harness | large |
| `p2-surface.md` | 40, 42, 43, 44, 46, 47, 48, 49 | `bba0e31a` or above; after `p1-suites` for the registry | `crates/surface-scan`, `surfacecheck`, `src/surface.rs`, `src/meter/registry.rs`, `src/meter/board/render.rs`, `crates/before-fuelscape/src/ops.rs` and the fuzzfit guest's cap, the prose sites | large |

### Placement decisions

- **Each cost row and its cure are one entry in two lanes.** The entry's
  Resolution names both the instrument and the cure; `p2-rows` lands the
  instrument (red, with the breach and readings in the commit) and
  `p2-cures` the cure, each brief stating which half is its own. The
  ledger's `sha` for such an entry is the cure's commit; the coordinator
  verifies the entry's whole Acceptance there.
- **crate-root-40 and prose-hygiene-1 are in `p2-rows`**, since both
  rewrite the meter header and that lane is the first to touch the file
  after the harness lands; `p2-cures` shrinks the under-repair list the
  header carries.
- **`memo.rs` is edited by both `p2-widths` (ruling 33) and `p2-cures`
  (ruling 2).** The widening lands first; the memo-heap representation
  rebases onto it.
- **The P2 lows and nits** (roster approved, ruling 104) are placed by the files they touch:
  rows and judge entries in `p2-rows` (envelopes-b-20, envelopes-b-28,
  version-core-1, board-families-floors-judge-14, board-ops-render-31);
  width and production-correctness entries in `p2-widths` (clock-17 and
  party-13 under decision 4, clippy-pedantic-2, inventory-7,
  codec-bits-8, fuzz-guests-pins-27, fuzz-guests-pins-15,
  testing-oracles-4, skyline-sweep-place-masked-3 under decision 70,
  skyline-query-13); extractor and cost-prose entries in `p2-surface`
  (surface-roster-21, surface-roster-29, crate-root-17, paper-fidelity-5,
  skyline-coding-2, skyline-coding-16, skyline-sweep-place-masked-14,
  version-core-23, codec-bits-27, codec-bits-10, suanpan-4, rank-23,
  crate-root-6); the headline question (crate-root-25, paper-fidelity-3,
  decision 21's open remainder) in `p2-cures` as a report to Finch, not
  an edit.
- **Not placed.** Eleven P2 lows and nits touch tools, benches, or the
  detached fuelscape and fuzz workspaces that no P2 brief owns:
  benches-examples-13, benches-examples-24, fuelscape-render-3,
  fuelscape-render-8, fuelscape-render-22, fuelscape-pipeline-19,
  fuzz-guests-pins-16, fuzz-guests-pins-32, fuzzfit-bands-16, tools-17,
  tools-25. They are correctness lows in instrument code, not in the
  library; the coordinator's options are a small `p2-tools` lane, or
  folding them into the P6 `tools`, `fuelscape`, `benches`, and `fuzz`
  lanes whose files they share. Recommendation: the P6 lanes, since none
  gates a P2 cure.

### Launch order

1. `p2-widths` and `p2-surface` may start on `bba0e31a` or any later
   main; `p2-surface`'s registry commits wait for `p1-suites`.
2. `p2-rows` after `p1-harness` lands (rebasing onto `p1-board` for the
   board families if it has landed).
3. `p2-cures` after `p2-rows` and `p2-widths` land.

Every P2 cure is measured at the parent with its re-pin named in the
commit; no cure lands before its row is red in the tree (rulings 1 and
2). The P2 lanes precede the rumors plan's performance phase (ruling 5).

## P3 and P4 lanes

Based on `10cdd255` (the commit recording ruling 78; the tree outside
`.agent-notes/` is still byte-identical to `9e5784fb`). Rulings 52 to 77
govern them. Each carries its ruled members with the ruling's choice
stated beside the quoted Resolution, then the P3 or P4 lows and nits no
ruling had reached as a roster, approved by ruling 104, placed by the
files they touch.

| Brief | Rulings | Members | Touches | Size |
|---|---|---|---|---|
| `p3-vocabulary.md` | 52, 53, 54, 55, 56, 57 | 27 ruled, 62 roster approved by ruling 104 (3 medium, 37 low, 49 nit) | prose in nearly every file of `crates/before/src`, `tools/`, the detached workspaces; `Cargo.toml` editions; `results/benchmarks` | large, mechanical |
| `p4-ghosts.md` | 58, 59, 60, 66, 68, 69, 70, 71, 72 (prose), 73, 74 (prose) | 47 ruled, 105 roster approved by ruling 104 (5 high, 21 medium, 84 low, 42 nit) | rustdoc and comments across the crate, `tests/meter.rs` header and row docs, `crates/before/AGENTS.md`, `examples/`, `tools/citecheck`, `fuzzfit/harness/src/bands.rs` docs, the fuelscape render docs | large |
| `p4-structure.md` | 72 (operand walk), 74 (idbits helpers), 75, 76, 77 | 13 ruled, 22 roster approved by ruling 104 (13 medium, 16 low, 6 nit) | `src/meter/board/{operand,defect}.rs`, the fill walks, `place.rs`, `filter.rs`, `overlay.rs`, `admit.rs`, `shape.rs`, `validate.rs`, `watermark.rs`, `span/algebra.rs`, `idbits.rs`, the party ops, `fuzz/framing.rs`, `tests/fuzz_seeds.rs` | large |
| `p4-rosters.md` | 61, 62, 63, 64, 65, 67 | 29 ruled, 25 roster approved by ruling 104 (5 medium, 43 low, 6 nit) | `src/meter/board/**`, `registry.rs`, the validation index, the guard sites, `Cargo.toml` features, the relocated test files, `tools/doclint` | medium |

### Placement decisions

- **The four briefs split by kind of edit, not by module.** Vocabulary
  (P3) is one mechanical sweep with six rulings; the P4 sweeps split
  into prose (`p4-ghosts`), structural extractions (`p4-structure`), and
  roster, guard, scope, and relocation work (`p4-rosters`), so that no
  two lanes edit the same file for the same reason. Ruling 72's two
  structural entries (board-families-floors-judge-24, board-frame-22)
  and ruling 74's party-4 are in `p4-structure`; the rest of 72 and 74
  are prose and sit in `p4-ghosts`.
- **Pending lows and nits by class document.** P3's unruled rows all go
  to `p3-vocabulary` (they are the vocabulary pattern's instances).
  P4's unruled rows go by document: documentation and claims to
  `p4-ghosts`; simplification to `p4-structure`; verification and
  correctness to `p4-rosters`.
- **Twenty P3 and P4 rows are deliberately in no brief.** Twelve wait on
  an owner decision ruled in a later session: rank-10 (decision 10),
  api-audit-9 and fresh-eyes-8 (15), version-core-5 (18), crate-root-2
  (20), envelopes-b-19 (5), codec-bits-12 (41 and 45),
  meter-registry-tier2-14 and testing-diff-gen-17 (50),
  fuelscape-pipeline-1 (66), inventory-5 (72), oracle-laws-4 (91's
  carried item). Eight are disposed by ruling 78 and belong to the P5
  briefs: envelopes-b-22, envelopes-b-25, meter-registry-tier2-3,
  prose-hygiene-7, suite-economics-3, surface-roster-10,
  surface-roster-11, tests-other-24.
- **meter-adequacy-3's judge half** (the affine-residual exclusion of a
  log factor on the board, ruling 73) edits `judge.rs`, which
  `p1-board` owns; the brief lands the doc halves and reports the judge
  half for the board lane if that lane has not landed.
- **prose-hygiene-3** is recorded as `dup` of board-ops-render-29 and
  **surface-roster-6** as `dup` of gate-legs-8; both are listed in their
  brief only so the lane knows the edit is owned elsewhere.

### Launch order

These lanes follow the P1 and P2 lanes that rewrite the same files:

1. `p4-rosters`' ruling 64 deletions and ruling 67's doclint rule may
   start on `10cdd255`; its ruling 61 roster work waits for `p1-board`
   and `p2-surface`; its ruling 65 relocation runs alone, with no other
   lane building against the moved files.
2. `p4-structure` after `p2-cures` for the kernel files it shares, and
   after `p1-fuzz` for the framing file.
3. `p4-ghosts` after `p1-harness` and `p2-rows` (the meter header) and
   after `p1-fuzz` (the bands file); ruling 60's one measured run on a
   quiet machine.
4. `p3-vocabulary` last, since it touches every file; its mechanical
   commits may land earlier and be rebased.

## P5 lanes

Based on `33779b10` (the commit recording ruling 80; the tree outside
`.agent-notes/` is still byte-identical to `9e5784fb`). Rulings 78, 79,
and 80 govern them, under the retirement discipline every brief states:
the replacement lands and is shown firing on what the old instrument
caught, then the instrument is deleted, one commit series per
retirement, no trace left in code.

| Brief | Rulings | Members | Touches | Size |
|---|---|---|---|---|
| `p5-scanners.md` | 78 (decision 44), 80 (decision 79) | 13 ruled (5 medium, 8 low), 1 roster approved by ruling 104 | `tools/citecheck*` (rewritten in Rust), `tools/doclint*`, the superlinear, twin, and band roster tests, `src/surface.rs`'s scan half, the justfile legs | large |
| `p5-judge.md` | 78 (decision 77), 79 (decisions 49 and 78) | 19 ruled (8 medium, 10 low, 1 nit), 4 roster approved by ruling 104 | `tools/benchjudge*`, `tools/digestshare*`, `benches/{amplify,emit_probe,perf_probe,tripwire}.rs` and sidecars, `tests/bench_judge_roster.rs`, the A/B arms, `fuelscape/spanbands`, the cliff-fan family, the render `overlay` field, `just all` and `ci` | large |
| `p5-buffers.md` | 78 (decisions 46, 47), 79 (decisions 48, 50), 80 (fuzzfit-strategies-19, version-core-11) | 17 ruled (1 high, 6 medium, 9 low, 1 nit), 7 roster approved by ruling 104 | `tools/covcheck*`, the `EXEMPTIONS` and `ITEM_EXCEPTIONS` sites, `registry.rs` and `surface.rs` date fields, `src/oracle/**` and `fold.rs`, `tier2.rs` and `testing/compactness.rs`, `fuzzfit/harness/src/strategies.rs`, `version.rs`'s `_view` doors | large |

### Placement decisions

- **Three lanes by kind of dissolution**: scanners (one typed
  collection authority replaces the hand scanners and an in-house lint
  rule), judge and expired instruments (retirements with a fuzz-fit
  demonstration first), buffers and dead machinery (acceptance lists,
  dates, the oracle copy of the fold, unread models). No two lanes edit
  a file for the same reason; `registry.rs` and the justfile are shared
  with P1 and P2 lanes and every brief says to rebase.
- **The eight ruling-78 rows the P3/P4 README left for P5**
  (envelopes-b-22, envelopes-b-25, meter-registry-tier2-3,
  prose-hygiene-7, suite-economics-3, surface-roster-10,
  surface-roster-11, tests-other-24) are in `p5-scanners` (the roster
  rebinding and old-scanner deletions) and `p5-buffers`
  (prose-hygiene-7, a date site).
- **meter-adequacy-4** (P1, decision 72, ruling 78) is in `p5-judge`:
  moot with the judge deleted.
- **Three P5-phase rows owned by P4 rulings** (board-frame-25 under
  ruling 61; inventory-4 and suanpan-21 under ruling 64) are appended
  to `p4-rosters.md` under their own heading, since the P3/P4 generator
  filtered by phase and missed them.
- **The P5 rows under decision 42 (ruling 38)** (board-frame-1,
  meter-core-11, board-ops-render-15, inventory-2, module-graph-1) and
  **decision 43 (ruling 4)** (envelopes-a-8, envelopes-a-11) are owned
  by `p1-harness` and `p1-board`; board-frame-1 and meter-core-11 are
  the board-side records of the segments dissolution and land with
  `p1-board`'s step 4 (that brief's segments section covers them by
  mechanism; the coordinator writes their sha from that lane).
- **Pending P5 lows and nits by kind**: gate-legs-10 (doclint's walk)
  in `p5-scanners`; fuelscape-pipeline-13, fuelscape-render-26,
  fuzz-guests-pins-19, fuzzfit-bands-18 (dead instrument code) in
  `p5-judge`; crate-root-22, envelopes-b-17, oracle-laws-24, rank-26,
  skyline-fill-grow-32, skyline-watermark-28, testing-oracles-9 (dead
  code and unread counters) in `p5-buffers`.
- **Three P5 rows are deliberately in no brief**: deps-16 (decision 20,
  packaging, ruled with P7), testing-oracles-17 and testing-oracles-25
  (decision 72, the small instrument rulings that ride with the P6
  board lane).
- **Ruling 26 is superseded** by ruling 78 (the judge retires); no
  brief carries a load-average stamp item. `p1-gate` never carried one;
  nothing is struck there.

### Launch order

1. `p5-buffers`' ruling 64-style deletions, the covcheck category
   removal, and version-core-11 may start on `33779b10`; version-core-11
   lands before `p4-structure`'s span-causally-9. Its oracle rewrite and
   date dissolution rebase onto `p1-suites`, `p2-surface`, and
   `p4-rosters` where landed.
2. `p5-judge` after `p1-fuzz` has landed `bands.rs` (the tripwire's
   fuzz-fit case is this lane's one calibration run) and after
   `p1-gate` for the justfile and lockfile edits.
3. `p5-scanners` after `p2-surface` (surface-roster-28's `syn`
   extractor must exist before before's line scan retires) and after
   `p1-gate` for the justfile; its citecheck rewrite may start earlier
   on a branch and rebase.

## P7 lane

Based on `59998ad0` (the commit recording ruling 87; the tree outside
`.agent-notes/` is still byte-identical to `9e5784fb`). Rulings 81 to
87 govern it: every change to the stable surface is one Finch named,
recorded in those rulings; ruling 82 supersedes ruling 81's item (4) and
restates ruling 35 at `usize::MAX`; ruling 85 completes ruling 84's item
(12) with the text forms.

| Brief | Rulings | Members | Touches | Size |
|---|---|---|---|---|
| `p7-api.md` | 81, 82, 83, 84, 85, 86, 87 | 40 ruled (7 medium, 24 low, 9 nit), 4 roster approved by ruling 104, 3 held | `src/{clock,party}.rs` and their `forks.rs`, `src/iter.rs`, `src/version/{ticks,rank,ranked}.rs`, `src/span.rs` and `span/wire.rs`, `src/shape.rs`, `src/error.rs`, the serde impls, `src/codec/text.rs`'s three entries, the `Floor`/`Ceiling`/`Query` docs, `src/laws.rs`, `tests/forks_max.rs`, `crates/suanpan/src/{limbs,accumulator}.rs`, `crates/before/Cargo.toml` | large |

### Placement decisions

- **One lane for the whole API block**, so the stable surface is edited
  in one reviewable diff and every signature change is named as
  owner-directed in its commit. Order inside the lane: additive
  attributes and impls, then the renames and the `usize` signature, then
  the `Parse` rule, then `serde_bytes` and the byte pins, then the text
  forms and the human-readable branch, then the rumors-relied contracts,
  then the `suanpan` items, then packaging.
- **The `forks(k: usize)` change moves out of `p2-widths`.** Ruling 82
  supersedes the ruling 35 entries there (clock-3, tests-other-17,
  party-14, api-audit-10) and the two decision-4 entries
  (clock-17, party-13); each of those six entries in `p2-widths.md` now
  carries a "Superseded by ruling 82" paragraph pointing here. If
  `p2-widths` lands first with a `u64` count, `p7-api` rebases and
  changes the type; `tests/forks_max.rs` is re-pinned at `usize::MAX`.
- **Rows from other phases that rulings 81 to 87 dispose** are members
  here regardless of phase: envelopes-b-19 (P4), api-audit-9,
  fresh-eyes-8, span-causally-38 (P4; the `Query` sentence, whose
  dangling-pointer half is ruling 66 in `p4-ghosts`, coordinated so the
  site is edited once), crate-root-2 and version-core-5 (P4), deps-16
  (P5). The P3/P4 and P5 READMEs listed several of these as waiting on
  later-session decisions; they are now placed.
- **Held**: suanpan-10, suanpan-tests-4, suanpan-tests-8 (decision 17)
  are listed but not landed until the `merge_into_wider` swap pin and
  decision 25's zero-operand row have been read; the lane reports the
  readings so Finch can rule.
- **Pending nits**: api-audit-5, codec-bits-11, fresh-eyes-11,
  fuelscape-pipeline-14 (the P7 rows with no ruling), roster approved by ruling 104
  approval.
- **citecheck's rumors root** (ruling 87, item 19) belongs to
  `p5-scanners` once the typed checker exists; `p7-api` only reports any
  `rumors` public rustdoc citing a `before` law name as a rumors-ledger
  finding.
- **Not wire.** The text forms and the human-readable serde branch are
  `Display`/`FromStr` and the `is_human_readable()` branch only; a
  moving wire snapshot or encoding pin is a stop.

### Launch order

`p7-api` runs after `p3-vocabulary`, `p4-ghosts`, and `p4-rosters` have
landed (they touch the same rustdoc), or rebases onto them before its
final gate run; after `p1-harness` for the test-local `Ticks` helper's
home in `tests/meter.rs`; and, for the rumors-relied contracts, after
`p2-cures` has landed the cost rows that the `# Complexity` sentences
sit beside. Its `suanpan` items are independent of the `before` items
and may be a separate commit series on the same branch.

## P8 lane

Based on `5328537c` (the commit recording ruling 92). Rulings 88 (no
lower bound pinned; the fixed-sign batch; the parity floor derived first;
the measure-first trades), 89 (decisions 45 and 71; party-25), 90
(rank-22), and 92 (the P8 roster) govern it.

| Brief | Rulings | Members | Touches | Size |
|---|---|---|---|---|
| `p8-performance.md` | 88, 89, 90, 92 | 28 ruled (4 medium, 17 low, 7 nit); no roster rows | `crates/suanpan/src/accumulator.rs` and its metered tests, `src/party/ops/index.rs`, `src/codec/{build,buf,stack}.rs`, `src/version/rank/num.rs`, `src/version/skyline/sweep.rs`, `src/causally/**`'s `le`/`lt` routing, the batch's kernels, the re-pinned rows in `tests/meter.rs` | large; strict resource discipline |

### Placement decisions

- **Three rows move into P8 from other phases**: rank-22 (P6 core;
  ruling 90's limbs-only `Num`, which is a measured change with two
  re-pins and a regression stop), codec-bits-12 (P4; ruling 88's A/B of
  the staging register), and skyline-sweep-place-masked-35 (P1; ruling
  89 lifts `le` rather than narrowing it, with span-causally-26). Their
  P6, P4, and P1 briefs no longer carry them.
- **The batch (decision 39)**: party-25 and the thirteen ruling 92
  roster rows land as one commit series, measured at the parent, every
  moved reading tightened with its attribution in the batch; no reading
  may rise.
- **Resource discipline** is stated in the brief's Ordering: load
  checked and disclosed, one run per A/B side, no iterating on timings,
  ox-east-1 under `pset-run` as the approved runner when the local
  machine is contended. Deterministic counters are the readings of
  record.
- The bench judge is retired (ruling 78), so every Acceptance that named
  a `bench-judge` run reads as the fuel bands plus the deterministic
  meters; each such entry says so.

### Launch order

After `p1-harness` (rows), `p1-survivors` and `p7-api` (suanpan and
`Rank`'s `Display`), `p2-cures` (ruling 2's representation), `p2-surface`
(rank.rs), and `p4-structure` (the pair and fold consolidations). Inside
the lane: the suanpan contract restatement, then suanpan-17 and -14;
the parity floor, then party-26; the batch; the `PackedBuilder` A/B and
the consolidation; the remaining trades; `sweep::le`; rank-22 last.

## P6 lanes

Based on `5328537c` for placement; rulings 93 to 104 (through `3ca52603`) are folded in. The 544 module-lane rows, placed by the files they
touch. The board lane is split six ways per ruling 6. Every P6 row is in
exactly one brief; 571 rows are placed across P6 and P8 in total (the
544 P6 rows plus the P8 phase and the three rows moved into P8).

| Brief | Ruled | Mediums ruled 93 to 103 | Roster approved (ruling 104) | Follows |
|---|---|---|---|---|
| `p6-core.md` | 5 | 4 (clock-22, clock-28, deps-3, module-graph-2) | 99 (49 low, 50 nit) | p2-widths, p2-cures, p4-ghosts, p4-structure, p4-rosters, p7-api |
| `p6-skyline.md` | 5 | 6 (skyline-coding-6, -23, skyline-fill-grow-12, -24, skyline-sweep-place-masked-19, skyline-watermark-21) | 76 (26 low, 50 nit) | p2-rows, p2-cures, p2-widths, p4-structure, p4-ghosts, p8 |
| `p6-codec.md` | 0 | 1 (codec-bits-30) | 22 (6 low, 16 nit) | p2-widths, p7-api, p8, p4-rosters |
| `p6-suanpan.md` | 1 | 1 (suanpan-tests-7) | 35 (17 low, 18 nit) | p1-survivors, p7-api, p8, p4-rosters |
| `p6-tools.md` | 3 | 4 (gate-legs-5, tools-4, -28, -33) | 27 (13 low, 14 nit) | p1-gate, p5-scanners, p5-judge, p5-buffers, p4-rosters |
| `p6-harness.md` | 4 | 12 (envelopes-a-9, -14, -15, -17, envelopes-b-7, -21, testing-diff-gen-14, testing-oracles-3, tests-other-10, -11, -16, -30) | 57 (28 low, 29 nit) | p1-harness (alone against meter.rs first), p1-suites, p2-rows, p4-rosters, p4-ghosts, p5-scanners |
| `p6-fuzz.md` | 1 | 4 (fuzz-guests-pins-29, fuzzfit-bands-19, fuzzfit-strategies-11, -16) | 37 (23 low, 14 nit) | p1-fuzz, p2-widths |
| `p6-fuelscape.md` | 6 | 2 (fuelscape-render-18, -19) | 34 (16 low, 18 nit) | p1-fuzz, p2-surface, p7-api; the dataset survey is the coordinator's run |
| `p6-benches.md` | 1 | 0 | 15 (8 low, 7 nit) | p5-judge |
| `p6-surface.md` | 0 | 1 (surface-roster-4, ruling 98's regime) | 11 (3 low, 8 nit), re-read under ruling 98 | p2-surface, p5-scanners, p6-tools |
| `p6-board-frame.md` | 0 | 0 | 9 (4 low, 5 nit) | p1-board, p1-harness, p4-rosters |
| `p6-board-families.md` | 2 | 0 | 9 (6 low, 3 nit) | p1-board, p4-rosters, p4-structure |
| `p6-board-ops.md` | 1 | 0 | 12 (8 low, 4 nit) | p1-board, p2-rows, p4-ghosts |
| `p6-board-registry.md` | 0 | 1 (meter-registry-tier2-16) | 7 (2 low, 5 nit) | p2-surface, p4-rosters, p5-buffers |
| `p6-board-oracle.md` | 2 | 1 (oracle-laws-13) | 18 (8 low, 10 nit) | p5-buffers, p7-api |
| `p6-board-meter-core.md` | 0 | 1 (meter-core-2) | 6 (5 low, 1 nit) | p1-suites, p2-surface, p4-ghosts, p4-rosters |

The "ruled" and "approved" columns count rows; the mediums are listed by id, each now carrying its ruling in its brief.

### Placement decisions

- **Lane by id prefix, not by the ledger's `lane` column alone.** The
  ledger derived `board` from the module heading, and the three
  documents that use the generic heading "The instruments" put 201 rows
  there that belong to the harness, fuzz, fuelscape, benches, surface,
  and tools lanes by the files they touch. Each row's lane is its ledger
  lane unless that lane is `board`, in which case the id prefix decides:
  `board-frame`, `board-families-floors-judge`, `board-ops-render`,
  `meter-registry-tier2`, `oracle-laws` (with `paper-fidelity`), and
  `meter-core` are the six board lanes; `envelopes-*`, `testing-*`,
  `tests-other`, `suite-economics`, `meter-adequacy`, and `recursion` go
  to harness; `fuzz-*` and `fuzzfit-*` to fuzz; `fuelscape-*` to
  fuelscape; `benches-examples` to benches; `surface-roster` to surface;
  `tools` to tools. The ledger's `lane` column is unchanged (this fork
  changes no ledger row); the coordinator may reseed it from this map
  with a `phase!` note if the split is approved.
- **The 42 P6 mediums** are ruled (93 to 103); each brief states the decision and any amendment beside the entry (board-ops-render-26's own counter site; envelopes-b-21 ceilings only; envelopes-b-7 deleted; fuzzfit-bands-19's probe deleted with no test; tools-33's interpreter recognizer deleted, uses-pinning kept; testing-diff-gen-14's floors derived from arm weights; benches-examples-19, tools-4, tools-28, envelopes-a-15 as dups; surface-roster-4 under ruling 98's regime).
- **The 474 P6 lows and nits** are approved rosters (ruling 104), each with its Resolution quoted (nit rows quote the table row and point at `evidence/`); the surface lane's are re-read under ruling 98.
- **Rendering stop.** Ruling 89's rule stands in every P6 brief: a change
  that would alter a rendered `before` doc panel is a stop for a
  deliberate ruling; the two ruled rendering changes (fuelscape-render-23
  and -27's other-crate gating) are in `p6-fuelscape`.
- **Held rows** (suanpan-10, suanpan-tests-4, suanpan-tests-8; ruling
  86) are `p7-api`'s and appear in no P6 brief.

### Launch order

P6 lanes run last, each after the earlier lanes named in its Follows
column have landed on main (or it rebases before its final gate run).
Among themselves: `p6-harness` first and alone against `tests/meter.rs`;
the six board lanes may run concurrently with each other (disjoint
files) but after `p6-harness`; `p6-skyline` after `p8-performance`;
`p6-core`, `p6-codec`, `p6-suanpan` after `p7-api` and `p8-performance`;
`p6-tools` after the P5 lanes; `p6-fuzz`, `p6-fuelscape`, `p6-benches`,
`p6-surface` whenever their Follows lanes are in. No two lanes touching
one file run concurrently; the coordinator sequences them on the machine.

## What the coordinator does with a report

A lane's report is data. For each entry the coordinator runs the
Acceptance against the tree at the reported sha, then writes the sha into
`../ledger.tsv` (the disposition is already `fix` or `fix-amended` from
the ruling; the sha is what makes it terminal). Entries reported as
stopped stay pending and go to Finch as a numbered block. Merge is by
reported sha, never by branch name. Every re-pin in a landed lane is
checked for the attribution the rulings demand: the movement named in the
commit, measured at the parent. Roster members (ruling 104) are ruled `fix` as their lanes land them,
with the sha written the same way; a surface-lane row reported moot
under ruling 98 is ruled `dup` against it.
