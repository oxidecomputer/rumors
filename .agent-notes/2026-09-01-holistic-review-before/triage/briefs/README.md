<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as lane briefs derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. Read with the ground rules in ../../README.md. -->

# Lane briefs for P1 through P4

One brief per lane for the instruments phase (P1), the correctness and
cost-cure phase (P2), the vocabulary phase (P3), and the pattern-sweep
phase (P4) of the before and suanpan triage. Every P1 lane is
based on `bba0e31a` (main, the commit recording the S2 rulings; the tree
outside `.agent-notes/` is byte-identical to the reviewed commit
`9e5784fb`, so every line number in the class documents holds). The P2
lanes name their base by dependency (a main commit carrying the P1 lanes
they build on; the coordinator names the SHA at launch). Every lane is
governed by `../rulings.md`. Each brief quotes its members' Resolution and
Acceptance verbatim from the class documents and names the ruling that
governs each, with any amendment stated beside the quote. A lane agent
reads only its brief; the brief carries the ground rules in full.

Rulings 1 to 77 have individually ruled every high and medium in these
lanes. Lows and nits inside a roster are swept per their entries under the
same rulings; members marked "roster: pending Finch's approval" are lows
and nits no ruling has reached yet, placed here so the roster can be
approved as a block (rumors precedent T28) and struck by exception. A lane
agent lands a pending member only once the coordinator confirms the
roster is approved, and reports a low or nit whose stated resolution
conflicts with a ruling or a sibling entry rather than choosing.

One ground rule is new since the first draft and stands in every brief:
ruling 43's direction that reasons, pins, and enforcement homes are typed
references the compiler resolves, never strings naming a test, a file, or
a line, and that a lane which sees a cleaner idiomatic shape for a roster
is authorized to adopt it.

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
  as pending roster members: board-families-floors-judge-19 and
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
- **The pending P2 lows and nits** are placed by the files they touch:
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
ruling has reached as a roster pending Finch's approval, placed by the
files they touch.

| Brief | Rulings | Members | Touches | Size |
|---|---|---|---|---|
| `p3-vocabulary.md` | 52, 53, 54, 55, 56, 57 | 27 ruled, 62 pending (3 medium, 37 low, 49 nit) | prose in nearly every file of `crates/before/src`, `tools/`, the detached workspaces; `Cargo.toml` editions; `results/benchmarks` | large, mechanical |
| `p4-ghosts.md` | 58, 59, 60, 66, 68, 69, 70, 71, 72 (prose), 73, 74 (prose) | 47 ruled, 105 pending (5 high, 21 medium, 84 low, 42 nit) | rustdoc and comments across the crate, `tests/meter.rs` header and row docs, `crates/before/AGENTS.md`, `examples/`, `tools/citecheck`, `fuzzfit/harness/src/bands.rs` docs, the fuelscape render docs | large |
| `p4-structure.md` | 72 (operand walk), 74 (idbits helpers), 75, 76, 77 | 13 ruled, 22 pending (13 medium, 16 low, 6 nit) | `src/meter/board/{operand,defect}.rs`, the fill walks, `place.rs`, `filter.rs`, `overlay.rs`, `admit.rs`, `shape.rs`, `validate.rs`, `watermark.rs`, `span/algebra.rs`, `idbits.rs`, the party ops, `fuzz/framing.rs`, `tests/fuzz_seeds.rs` | large |
| `p4-rosters.md` | 61, 62, 63, 64, 65, 67 | 29 ruled, 25 pending (5 medium, 43 low, 6 nit) | `src/meter/board/**`, `registry.rs`, the validation index, the guard sites, `Cargo.toml` features, the relocated test files, `tools/doclint` | medium |

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

## What the coordinator does with a report

A lane's report is data. For each entry the coordinator runs the
Acceptance against the tree at the reported sha, then writes the sha into
`../ledger.tsv` (the disposition is already `fix` or `fix-amended` from
the ruling; the sha is what makes it terminal). Entries reported as
stopped stay pending and go to Finch as a numbered block. Merge is by
reported sha, never by branch name. Every re-pin in a landed lane is
checked for the attribution the rulings demand: the movement named in the
commit, measured at the parent. Pending roster members are landed only
after Finch approves the roster and are then ruled `fix` by that approval
ruling, with the sha written the same way.
