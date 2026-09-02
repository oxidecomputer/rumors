<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P6 lane: benches and examples

## Goal

The criterion benches' and examples' remaining entries, landed per their Resolutions inside an approved roster, after ruling 78 retired the judge and ruling 79 retired the expired instruments.

## Roster summary

1 ruled (1 medium); 0 medium ruled (93 to 103); 15 roster members approved (ruling 104) (8 low, 7 nit).

## Ground rules

These apply to every lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal `5328537c` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `5328537c`, fast-forward; if it has diverged, stop and report. Never call
  EnterWorktree; operate on the worktree through `git -C <path>` and
  absolute paths, one shell invocation at a time.
- **The review documents are the specification.** Each member entry below
  quotes its Resolution and Acceptance verbatim from the class document in
  `.agent-notes/2026-09-01-holistic-review-before/`; the entry's full
  record (evidence, construction, witness outcome) is under `### <id>:`
  there (`#### <id>:` in `simplification.md`; nits in `documentation.md`,
  `simplification.md`, and `verification.md` are one-line table rows) and
  in `evidence/`. Line anchors are at the reviewed commit `9e5784fb`, and
  the tree outside `.agent-notes/` is byte-identical at your base, so they
  hold; re-anchor from the quoted evidence, never from a line number
  alone, once your own commits move the file.
- **The rulings govern.** `triage/rulings.md` records Finch's decisions.
  Where a quoted Resolution offers alternatives, the ruling named beside
  it picks one; where the ruling amends the Resolution, the amendment is
  stated under the quote and wins. Where a quoted Resolution and the lane
  goal come apart, the goal wins, and the discrepancy is reported.
- **Typed references, never strings (ruling 43).** Wherever this lane
  touches `meter/registry.rs`, a family roster, `TRIPWIRE_ROSTER`, the
  surface rosters, or any test that names another test, file, or line:
  reasons, pins, and enforcement homes are expressed as references the
  compiler resolves (function items, registered law names, `Shape` and
  `Op` values), never as strings naming a test function, a file, or a
  line number. A lane that sees a cleaner idiomatic shape for a roster is
  authorized to adopt it and reports the reshaping in its diff. Finch's
  words: "please make these instruments impossible to drift in the
  future. I *really don't like* the pattern of hard-coded strings and
  Rust source locations embedded in tests; the way these family rosters
  ended up is not really to my taste, but I haven't had time to make it
  more idiomatic and obviously correct. If you see a good way to clean it
  up, please do."
- **Stops.** Report and leave the entry open; do not work around: anything
  that moves an `insta` snapshot or a committed pin the brief does not
  name as moving; any change to a public signature or public rustdoc
  contract the resolution does not name (the meter surface under
  `any(test, feature = "meter")` is instrument surface by ruling 8 and
  not public API, but a change there lands with the `rumors` test update
  in the same commit); anything that contradicts a ruling; any deviation
  from the stated resolution; anything this brief marks as a stop. A stop
  on one entry does not block the others.
- **Negative controls.** Every repaired or added instrument lands with a
  committed demonstration that a known-bad artifact fails it. The
  constructions in `evidence/witness.md` and each entry's Construction
  line are those artifacts; convert each into a committed test
  (`should_panic`, an asserted `Err`, a judge test over synthetic samples,
  or a reversible mutation whose observed failure the commit message
  records verbatim). Ruling 20 is the one place this brief set says
  otherwise, and it says so at the entry.
- **Resource discipline.** Build with `cargo nextest run --no-run` before
  running; during iteration run only the binaries the entry names; never
  iterate on a timing measurement; launch no daemons or services. One full
  `just gate` per agent, before the final commit, run in the background
  redirected to a log under `<scratchpad>/<lane>/` and polled with short
  foreground checks (the foreground command cap is ten minutes; a
  foreground gate run cannot finish). Keep every working file and evidence
  log under that directory, never loose at the scratchpad top level.
- **Commits.** One commit per logical unit, its message describing the
  change and naming the entry ids and the ruling. Every re-pin is measured
  at the parent and its movement named in the commit. Commit every
  proptest seed file that appears. Prose speaks in the present tense: no
  reference to code that no longer exists, no dated rationale at a
  declaration site. Comments use spaced double-hyphens, never em-dashes;
  every test has a doc comment stating its invariant. Commit with a
  descriptive message before finishing.
- **Never delete anything outside your worktree.** If the disk fills
  (ENOSPC), stop and report; it is the coordinator's problem, not a
  reason to trim caches you do not own.
- **Self-retirement.** After the final commit, from inside the worktree:
  `cargo metadata --no-deps --format-version 1 | jq -r .build_directory`,
  then delete that directory and the worktree's `target/`. Leave the
  worktree in place.
- **Report.** For each entry: landed, stopped, or open; the commit sha(s);
  the acceptance evidence (the command and its decisive output, verbatim).
  Then anything left open and why, and every deviation from a stated
  resolution, explicitly. Your report is data: the coordinator verifies
  each entry's Acceptance against the tree at the reported sha before the
  ledger records it. Report what you could not do rather than working
  around it.
- **No lower bound on performance is pinned anywhere (ruling 88).** Every
  touch, scan, limb, or heap pin is a ceiling: a reading over it fails; a
  reading under it is an improvement that lands by tightening the ceiling
  with its attribution in the commit. The only floors are liveness floors
  derived from a mechanism's irreducible work, never from a measured
  reading. An entry whose Resolution asks for an exact pin or a measured
  floor is read as a ceiling plus any mechanism-derived floor it names.

## Ordering

Every P6 lane runs after the P1 to P5 and P7 lanes that touch its files have landed on main, or rebases onto them before its final gate run; the coordinator names the base SHA at launch. Lows and nits inside this lane's approved roster are swept without a question to Finch; every high and medium has an individual ruling. A change that would alter a rendered `before` doc panel is a stop (ruling 89). Follows `p5-judge`. Owns `crates/before/benches/**`, `crates/before/examples/**`.

## Members

### benches-examples-19 (medium, simplification): ruling 93

Hand-synchronized copies with parity asserted in prose: code_study.rs duplicates space_consumption.rs's simulation; perf_probe.rs duplicates benches/common

- Owner-gated: no

Resolution: move `Scenario`, `seed_for`, `checkpoints`, `build_population`, `step_data`, `step_process` into `examples/support/simulation.rs` and include it from both examples with `#[path]`, so code_study's `tag: u64` becomes `Scenario`; replace perf_probe.rs:18-141 with `#[path = "../benches/common/mod.rs"] mod common;` and `use common::{SEED, plan, impl_clocks, hole_pair, oracle_clocks}` (name the salts as constants in common if the correspondence with benches/clock.rs matters, and use them on both sides). The code_study half is moot if benches-examples-18 dissolves the example. Acceptance: one definition of each simulation and corpus function in the tree; `just check` green; the two parity comments disappear or read as mechanical ("shares `common::plan`").

Dup (ruling 93): both halves are moot; `code_study.rs` is deleted under ruling 71 (`p4-ghosts`) and `perf_probe.rs` retires under ruling 79 (`p5-judge`). Nothing to land in this lane. See ../rulings.md.

Ledger note: both examples retired (rulings 71, 79)

## Roster members approved (ruling 104)

Lows and nits approved as this lane's roster by ruling 104. Land each per its quoted Resolution and Acceptance, swept with the ruled members; report rather than choose if a Resolution conflicts with a ruling or offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89).

### benches-examples-10 (low, verification): roster: approved (ruling 104)

The ceiling-class assert compares the pinned set with itself for every board cell, and the sidecar prose describes a cell-site declaration board cells do not have

- Owner-gated: no

Resolution: make membership the single declaration: `write_denoms` takes `(id, denominator_bytes)` pairs and derives the class from `TEXT_CEILING_CELLS` internally (this reproduces today's sidecar byte for byte, since both wide-pair IDs are in the set and the tripwire's is not); drop the assert and the three literal `Ceiling` arguments; reword sidecar.rs:27-30, 82-85, and 143-147 to "the class is set membership, pinned by tests/bench_judge_roster.rs". Add to tests/bench_judge_roster.rs (which already includes the sidecar module) a test that every entry other than `version_display_wide/hugeleaf` and `display_schoolbook/hugeleaf` is an `op/family` of `board::bench_cells(0.02, BenchMode::Full)`, mirroring `bench_riders_name_declared_model_cells`. Acceptance: `Ceiling` no longer appears in board.rs or tripwire.rs; the sidecar written by `just bench-judge` is byte-identical to before; renaming a text-class op in ops.rs without editing the set fails a gate test naming the stale entry. Construction: rename `clock_parse_trailing` to `clock_parse_trail` in src/meter/board/ops.rs and its callers, leave `TEXT_CEILING_CELLS` and the roster test untouched; `just test-all` is green; `just bench-judge` judges `clock_parse_trail/hugeleaf` at 1.3, and the stale entry never fires.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### benches-examples-14 (low, verification): roster: approved (ruling 104)

`outgrow_family` asserts a relative sweep where its doc promises an absolute crossing of the pre-size

- Owner-gated: no

Resolution: assert both clauses, `first <= NEAR_PRESIZE_RATIO && last >= 4.0` with a named constant (1.5, say) and both ratios in the panic message; or reword the doc to the relative sweep if that is the intent. Acceptance: the assertion's inequality matches the doc sentence; a synthetic pair (0.3, 1.2) fails it. Construction: multiply `OUTGROW_FRAGMENTS` by 64 so the party's bits dominate every term: ratios fall well below 1 across the sweep while `last >= 4 * first` can still hold, and no growth doubling is crossed.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### benches-examples-15 (low, verification): roster: approved (ruling 104)

`tools/benchjudge --self-test` runs only at the head of the bench-judge recipes, not in `gate-lints`; tripwire.rs overstates its cadence

- Owner-gated: no

Resolution: add `./tools/benchjudge --self-test` to `gate-lints` in the position the other tool self-tests occupy; leave the two in-recipe invocations; reword tripwire.rs:12-14 to "pinned in the judge's `--self-test`, which the gate runs". Acceptance: `just gate-lints` invokes `tools/benchjudge --self-test`; with an uncommitted `MAX_WALL_SCALING_EXPONENT = 3.0`, `just gate` fails at the self-test before any build runs. Construction: set `MAX_WALL_SCALING_EXPONENT = 3.0` in tools/benchjudge (uncommitted): `just gate` passes today, since nothing in gate-lints or gate-streams invokes the judge.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### benches-examples-26 (low, documentation): roster: approved (ruling 104)

The space-consumption results README lists six CSV columns for an eight-column file; the figure of record predates the marker-padding change to `encode().len()`

- Owner-gated: no for the column list; the re-measure is the owner's call (long-running at paper parameters)

Resolution: fix the column list now (or point at the example's `# Output` section as the column reference). Re-run `cargo run --release --example space_consumption` and the plot at the next convenient point, or state in the results README the commit the data was collected at. Acceptance: the README's column list equals `head -1 space.csv`; the README names the data's commit; after a re-run, the 4-replica final byte means in the README table match the new CSV.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### benches-examples-3 (low, documentation): roster: approved (ruling 104)

Bench doc comments misstate operand ownership: `recv` borrows, `Version` is `Clone`

- Owner-gated: no

Resolution: clock.rs: "The clock is mutated and rebuilt per iteration; the impl borrows the message (`recv(&Version)`), the oracle consumes a clone made in setup." common/mod.rs:22-24: "or `decode` (impl: `Party` and `Clock` are not `Clone`, and a `Version` clone shares its buffer, so decode gives each iteration a distinct one)". Acceptance: both comments match the signatures they describe; no bench doc claims Version is not Clone.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### benches-examples-4 (low, simplification): roster: approved (ruling 104)

benches/common spells the universe build and group fold five times, carries dead `map_err` adapters, and has `rng` copied into four targets

- Owner-gated: no

Resolution: two private generics, `universe<T>(seed: T, schedule: &[usize], fork: impl FnMut(&mut T) -> T) -> Vec<T>` and `fold_groups<T>(slots: Vec<T>, label: &[u8], groups: u8, join: impl FnMut(&mut T, T)) -> Vec<T>`; express the five builders through them (`hole_pair` reuses `universe` and a full fold); delete every `.map_err(|_| ())` (also in perf_probe.rs); move `rng` with party.rs's doc comment into common as `pub fn rng(salt: u64) -> StdRng`. Acceptance: one fork loop and one group fold in the module; `grep -rn 'map_err(|_| ())' crates/before/benches crates/before/examples` returns nothing; one `fn rng` under crates/before/benches; bench IDs, seeds, salts, and inputs unchanged.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### benches-examples-5 (low, verification): roster: approved (ruling 104)

`alloc_arms` is a third hand-spelled copy of the A/B arm roster, with no check against the manifest

- Owner-gated: no

Resolution: one `pub const ALLOC_ARMS: &[&str]` from which a small macro generates the `cfg!` checks, and a test that parses Cargo.toml's check-cfg line and asserts its value list equals the constant; or have `bench-alloc-ab` pass the arm through an environment variable the bench stamps, cross-checked against one `cfg!`. Acceptance: adding a value at Cargo.toml:107 without touching benches/common fails a committed test. Construction: add `"parse_growth"` to the `values(...)` list at Cargo.toml:107 and a `#[cfg(before_alloc_ab = "parse_growth")]` seam anywhere in the library; `RUSTFLAGS='--cfg before_alloc_ab="parse_growth"' cargo bench -p before --bench presize --no-run` builds clean, and running it prints `arm=shipped` on every `presize-resident` line.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### benches-examples-6 (low, verification): roster: approved (ruling 104)

The sidecar stamp binds sidecar to sidecar and to `--tip`, not to the criterion baseline

- Owner-gated: no

Resolution: State the binding precisely at sidecar.rs:16-25 (sidecar-to-sidecar and sidecar-to-invocation; the baseline is trusted to be the same run's). To close the gap instead: stamp `git describe --always --dirty` (or refuse when `git status --porcelain` is nonempty) in the recipes, and have the judge require each judged cell's estimates.json to be no older than its sidecar, which is written before any cell runs (board.rs:159). Acceptance: the doc names exactly the two cross-checks the judge performs, or the construction below exits 2 naming the stale cell. Construction: run `just bench-judge` clean; make an uncommitted edit that makes one pinned op quadratic; re-run only the lo pass with the recipe's environment (the justfile:843 line) and invoke the judge line (justfile:845) by hand: both sidecars stamp the same tip, profile, and sampling, the hi medians are the pre-edit tree's, and the judge scores the pair.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### benches-examples-11 (nit, verification): roster: approved (ruling 104)

fork/join/sync/send bench routines drop the consumed operand inside criterion's timed span while tick/receive return it; scopes differ across rows (symmetric across impl and oracle).

Where: `crates/before/benches/party.rs:34-75`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Return surviving operands from every `iter_batched` routine, or document the timed scope once in common/mod.rs.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### benches-examples-16 (nit, simplification): roster: approved (ruling 104)

Knuth's MMIX multiplier inlined as an unnamed literal in two instrument files

Where: `crates/before/benches/tripwire.rs:46-48`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Name `LCG_MULTIPLIER` once; use it at all sites

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### benches-examples-2 (nit, simplification): roster: approved (ruling 104)

Mechanical slips in board.rs and presize.rs: a 126-column doc line, a split import group, a forward-looking clause

Where: `crates/before/benches/board.rs:12-12`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Re-wrap the doc line; group the imports; reword the forward-looking clause

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### benches-examples-20 (nit, simplification): roster: approved (ruling 104)

code_study.rs dispatches on a string label and keeps parallel per-code arrays

Where: `crates/before/examples/code_study.rs:513-525`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Carry the denominator in the tuple; one `Vec<Option<u128>>`; moot if the example dissolves

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### benches-examples-23 (nit, documentation): roster: approved (ruling 104)

An unescaped `\

Where: `crates/before/examples/space_consumption.rs:31-31`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): ` inside a code span splits the API-mapping table row

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### benches-examples-9 (nit, simplification): roster: approved (ruling 104)

The denominator sidecar hand-rolls JSON with serde_json already a dev-dependency

Where: `crates/before/benches/common/sidecar.rs:159-187`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Derive `Serialize` with `preserve_order`, or leave the writer as is

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### paper-fidelity-14 (nit, documentation): roster: approved (ruling 104)

the results README's CSV column list omits the bit columns the file carries

Where: `crates/before/results/space_consumption/README.md:8-9`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): update the column list or point at the example's `# Output` section

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

