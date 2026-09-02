<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P6 lane: tools (tools/, gate recipes, workspace sweeps)

## Goal

The workspace tools' and gate recipes' per-module entries, landed per their Resolutions inside an approved roster, under rulings 18 (no mutants roster), 78 (the typed checker, the retired judge), 79 (digestshare deleted, the buffers gone), 91 and 92 (covcheck's file floor, the shared walker), and 43.

## Awaiting individual ruling

The coordinator is walking these mediums with Finch; nothing below lands for them until the ruling is appended here: gate-legs-5, tools-28, tools-33, tools-4.

## Roster summary

3 ruled (2 medium, 1 low); 4 medium awaiting ruling; 27 pending roster approval (13 low, 14 nit).

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

Every P6 lane runs after the P1 to P5 and P7 lanes that touch its files have landed on main, or rebases onto them before its final gate run; the coordinator names the base SHA at launch. Lows and nits inside this lane's approved roster are swept without a question to Finch; every high and medium has, or awaits, an individual ruling. A change that would alter a rendered `before` doc panel is a stop (ruling 89). Follows `p1-gate`, `p5-scanners`, `p5-judge`, `p5-buffers`, and `p4-rosters`. Owns `tools/**`, the `justfile`, `.github/workflows/**`, `.cargo/**`, and the workspace manifests.

## Members

### deps-4 (low, simplification): ruling 89

build.rs is a docs-only formatter every consumer compiles; a design question against a recorded decision

- Owner-gated: yes (the direction of a documentation pipeline the owner designed)

Resolution: a design proposal for the owner, costs named. Have the compactor (crates/before-fuelscape, which owns the format) emit the islands, the `.open` variant, and the two themed SVGs as committed derived files under crates/before/docs/, included by path from the doc sites; hold them fresh by `fuelscape-verify`'s existing diff plus an in-crate test that re-derives the header and README figure from CARGO_MANIFEST_DIR; point the totality test (fuelscape_islands.rs:29, which reads `$OUT_DIR/fuelscapes/index`) at the committed directory listing. Then build.rs and the serde_json build-dependency dissolve, `just doc-figure` becomes a compactor mode, and a dataset defect fails a test instead of every consumer's compile. Cost: roughly 1.5 to 3 MB more committed derived HTML (the islands embed the JSON payload), or the standalone JSON goes if tools/fuelscape-claims learns to read the islands. Acceptance: if taken, `crates/before` has no build.rs and no `[build-dependencies]`, `just docs` renders every island, and `fuelscape-verify` fails on a hand-edited island. If declined, the deps-3 and deps-5 fixes stand on their own and this entry closes as a recorded decision.

Ruled (89, decision 66): model. `build.rs` stays a docs-only formatter every consumer compiles; the home is its header comment, which states that this is intended. No code change.

### tools-16 (medium, verification): ruling 91

covcheck has no per-file presence floor: a scope file absent from the lcov is invisible unless it already carries an entry

- Owner-gated: no

Resolution: enumerate `Path(root, scope).rglob("*.rs")` at check time and fail by name for every file with no SF record, in both modes; state the presence rule in the docstring; decide deliberately at the scope declaration whether tests.rs siblings are in or out; add a self-test case with a second scope file the lcov omits; then delete the `place/filter.rs: []` entry (tools-18), whose only role this replaces. Acceptance: a coverage run with `--ignore-filename-regex 'skyline/walk\.rs'` makes `just coverage-kernel` fail naming walk.rs; the committed roster passes on a fresh lcov; the self-test case is committed. Construction: gate `mod walk;` in crates/before/src/version/skyline.rs behind an unlit cfg, or pass cargo-llvm-cov `--ignore-filename-regex 'skyline/walk\.rs'`: the lcov carries no SF record for walk.rs, `files` and `resolved` have no key for it, the floor sees covered lines elsewhere, and covcheck exits 0.

Ruled (91, decision 80): a per-file presence floor over `git ls-files` under covcheck's scope, each absent file red unless excused with its reason; the twelve `tests.rs` siblings under `skyline/` stay in scope, the decision stated at the scope declaration.

### tools-22 (medium, correctness): ruling 92

Two hand-rolled Rust-file walkers read build output and a foreign worktree into the gate

- Owner-gated: no

Resolution: one discovery routine shared by doclint and testdoc (a small helper module in tools/, offered to seed_liveness.rs as well): `git ls-files -z --cached --others --exclude-standard -- '*.rs'` restricted to the given roots (tracked plus untracked-not-ignored, so pre-add work is still linted while target/, .claude/worktrees, and other excluded trees are invisible), or, if git is not wanted in the lint tier, prune directories carrying CACHEDIR.TAG and hidden directories. Keep each tool's missing-root guard; pin the routine in both self-tests with a fixture tree containing `target/x.rs` beside a CACHEDIR.TAG and `.hidden/y.rs`. Acceptance: doclint and testdoc report the same file set on the same tree; with the two generated files and the `.claude/worktrees` files present, neither tool visits them. Construction: write a `///` paragraph of 300 characters into a scratch `.rs` under crates/before/fuzz/target/ and run `./tools/doclint crates`: it fails; on a fresh checkout the same commit passes. Write an undocumented `#[test]` into `.claude/worktrees/w/src/t.rs` and run `./tools/testdoc .`: it reports it.

Ruled (92, decision 80): one shared file-enumeration helper in `tools/` over `git ls-files`, used by doclint, testdoc, and the test that copies their walk; no shared self-test scaffold (each tool stays self-contained).

## Mediums awaiting individual ruling

Listed with their Resolution so the lane knows the files they touch; not landed until ruled.

### gate-legs-5 (medium, documentation): awaiting individual ruling

ci.yml installs and describes floating toolchains the recipes never invoke; the dated toolchains arrive by rustup auto-install

- Owner-gated: no

Resolution: Install what the recipes name: `toolchain: nightly-2026-06-30` in each nightly step (with `components: llvm-tools` in the coverage job), sourced from one place so the pin cannot fork (a workflow `env` the justfile variable is checked against, or a step that reads `nightly_toolchain` from the justfile); drop the floating `stable` steps and let rust-toolchain.toml provision stable (add `llvm-tools` to its `components` if the coverage job should not rely on cargo-llvm-cov's self-install); rewrite lines 17-20, 54-58, and 128-133 for the pinned regime and its paired bump procedure. Acceptance: CI logs show no `syncing channel updates` for a toolchain the workflow did not name, and the workflow prose names the same nightly date as justfile:40.

Awaiting individual ruling: the coordinator is walking the P6 mediums with Finch now. Do not land this entry and do not choose among its alternatives; when the ruling arrives it is appended to this brief by the coordinator.

### tools-28 (medium, verification): awaiting individual ruling

mutantcheck never accounts for mutants missing from the filtered listing, and reads only `exclude_re`

- Owner-gated: no

Resolution: after parsing both captures, compute the unclaimed set (mutants in raw minus filtered matched by no pinned pattern) and fail by name for each; refuse any filtering key in the config beyond `exclude_re` (`exclude_globs`, `examine_globs`, `examine_re`) as an unpinned campaign restriction; add a self-test case: raw {a, b, c, d}, filtered {c}, d matched by no pattern, must red with "suppressed by no pinned pattern". Acceptance: with `exclude_globs = ["**/watermark.rs"]` appended to .cargo/mutants.toml and the captures regenerated, `just mutants-list` fails naming the watermark mutants or the key; the current tree stays green; the new case is committed.

Awaiting individual ruling: the coordinator is walking the P6 mediums with Finch now. Do not land this entry and do not choose among its alternatives; when the ruling arrives it is appended to this brief by the coordinator.

### tools-33 (medium, verification): awaiting individual ruling

workflowlint's interpreter recognizer stops at the first non-prefix token, so `| sudo -E bash -` and `| env -i sh` pass as fetch-without-execute

- Owner-gated: no

Resolution: adopt the tool's own over-matching policy: after a prefix word, skip flag tokens (those starting with `-`) and their arguments, or treat a post-fetch segment as fetch-execute when an interpreter token appears anywhere in it after stripping prefix words, flags, and assignments. Add `curl ... | sudo -E bash -`, `curl ... | sudo -u runner sh`, and `curl ... | env -i sh` to the self-test's red fixtures. Acceptance: `./tools/workflowlint --self-test` fails on the current recognizer with the three new fixtures and passes after the fix; the fetch-to-a-file green fixture stays green.

Awaiting individual ruling: the coordinator is walking the P6 mediums with Finch now. Do not land this entry and do not choose among its alternatives; when the ruling arrives it is appended to this brief by the coordinator.

### tools-4 (medium, verification): awaiting individual ruling

`ci` omits manifestlint, so the lint tier has two hand-maintained rosters and GitHub CI never runs it

- Owner-gated: no

Resolution: make `ci` depend on `gate-lints` instead of re-listing its legs (`ci: gate-lints fuelscape-claims clippy clippy-default ...`), so the lint tier has one definition; ci.yml:83-86 already installs cargo-mutants and cargo-rdme, so nothing new is required of the runner. Acceptance: `just --show ci` names `gate-lints` as a dependency; a member manifest carrying `path = "../suanpan"` beside `workspace = true` fails `just ci`. Construction: add `path = "../suanpan"` beside `workspace = true` on any member dependency; `just gate-lints` fails at manifestlint and `just ci` passes.

Awaiting individual ruling: the coordinator is walking the P6 mediums with Finch now. Do not land this entry and do not choose among its alternatives; when the ruling arrives it is appended to this brief by the coordinator.

## Roster members pending Finch's approval

Lows and nits no ruling has reached, placed here by the files they touch. Land only after the coordinator confirms the roster is approved.

### gate-legs-7 (low, documentation): roster: pending Finch's approval

`all` is documented as "Everything" while omitting the gate's instrument legs and the coverage legs, and its exclusive legs have no recorded cadence

- Owner-gated: no

Resolution: Rename the doc line and header entry to what `all` is (the no-rot sweep plus the manual tier), or make `all` include `gate` so the word is true. For cadence, see the open question below: either a scheduled workflow for the shared-runner-safe legs (the fuzz smoke) or a committed attestation of the last `all` run. Acceptance: `just --list` describes `all` accurately, and the tree or CI states when the `all`-only legs last ran.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### module-graph-4 (low, simplification): roster: pending Finch's approval

Limb-meter taps are spelled three ways and their module path five ways

- Owner-gated: no

Resolution: Fold `limb_metered.rs` into `limb_meter.rs` in the `codec::scan` shape: an inner `#[cfg(feature = "limb-meter")] mod counter` holding the statics, readers, and resets; ungated shims `record(u64)`, `record_wide(&UBig)`, `record_densified(u64)`, and the existing `meter_limbs*`; make `pub(crate) mod limb_meter` unconditional and the codec.rs:39-40 re-export ungated; replace the five inline `#[cfg]` taps with plain calls; reduce the four site-local shims to direct calls; spell the path one way (`crate::codec::limb_meter`); re-word base.rs:10 to name the feature. Acceptance: no `#[cfg(feature = "limb-meter")]` attribute appears outside `limb_meter.rs` and `meter.rs`; behaviour and every envelope reading unchanged.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### module-graph-9 (low, simplification): roster: pending Finch's approval

meter.rs holds a private generator table and the public counter read surface in one file

- Owner-gated: no

Resolution: Move the constructors, `Packed::from_bits`, and the `ev_*`/`pow2*` helpers into `meter/shapes.rs` as `pub(super)` items; leave meter.rs with its module doc, submodule declarations, `Packed`, the counter readers, and the re-exports; rewrite registry.rs's `super::x` to `super::shapes::x` (mechanical). Nit in the same spirit: `ops()` spans lines 193-2275 under a `too_many_lines` allow; concatenated per-`OpGroup` functions would keep the table declarative while making a row findable. Acceptance: `before::meter`'s public items and the `compile_fail,E0603` doctest at registry.rs:23-27 are unchanged; file motion only.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### tools-1 (low, documentation): roster: pending Finch's approval

ci.yml restates memwatch's per-process cap as a number that has rotted

- Owner-gated: no

Resolution: drop the figure and name the variable ("its per-process cap, `PROC_LIMIT_GB` in tools/memwatch, sits well above ..."). Acceptance: `grep -n GiB .github/workflows/ci.yml` is empty.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### tools-11 (low, verification): roster: pending Finch's approval

benchjudge's self-test is fifty bare asserts that `python3 -O` strips

- Owner-gated: no

Resolution: open `self_test` with `if sys.flags.optimize: raise SystemExit("benchjudge --self-test needs asserts; run without -O")`, or convert the asserts to explicit `if not ...: raise AssertionError(...)` as the sibling tools do; same one-liner for citecheck:805. Acceptance: `PYTHONOPTIMIZE=1 python3 tools/benchjudge --self-test` exits nonzero.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### tools-15 (low, verification): roster: pending Finch's approval

covcheck validates only the disposition: a `why`-less or extra-keyed entry passes and a missing `anchor` tracebacks

- Owner-gated: no

Resolution: validate each entry's key set as exactly {anchor, disposition, why} plus optional {offset}, with anchor and why nonempty strings and offset an int, reporting violations as problems; add self-test cases for a missing why, an extra key, and a missing anchor. Acceptance: `{"anchor": "...", "disposition": "unreachable"}` fails by name; the committed file still passes.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### tools-18 (low, simplification): roster: pending Finch's approval

covcheck-expected.json carries an empty `place/filter.rs` branch key, the residue of a cured remediation entry

- Owner-gated: no

Resolution: delete the key; if file presence is the intended pin, it belongs in covcheck as the scope-wide rule of tools-16, not in one empty row. Acceptance: the branch map has no empty lists and `just coverage-kernel-branch` is unaffected.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### tools-2 (low, documentation): roster: pending Finch's approval

The validation index omits citecheck, covcheck, and mutantcheck

- Owner-gated: no

Resolution: add one row each under the semantic instruments (citecheck, covcheck, mutantcheck), naming the recipe and the roster file; consider rows for surfacecheck and the wasm32 pins in the same pass. Acceptance: `grep -n 'citecheck\|covcheck\|mutantcheck' crates/before/src/testing/validation_index.rs` returns one row each.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### tools-23 (low, verification): roster: pending Finch's approval

fuelscape-claims has no liveness floor and no self-test

- Owner-gated: no

Resolution: fail when `index.ops.length === 0` or when any `doc.op.claim`/`sizes` is missing, naming the file; add `--self-test` asserting `Fuelscape.accepts` refuses a syntax error and the `log n` at n=1 case and accepts a linear claim; run the self-test first in the recipe like the other tools. Acceptance: `{"ops": []}` exits 1 naming the floor; `tools/fuelscape-claims --self-test` fails when `accepts` is stubbed to `() => ({})`; the committed index still passes with its count printed.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### tools-26 (low, verification): roster: pending Finch's approval

memwatch's kill path has no committed demonstration

- Owner-gated: no

Resolution: add a `--self-test` that runs memwatch with a tiny `PROC_LIMIT_GB` around a child whose command matches the filter (a copied `python3` named `rustc-selftest`, or a script) that allocates past the limit, asserting the `KILL pid` note and a nonzero exit; and a second case where the child is named outside the filter and survives; run it at the head of one recipe; degrade explicitly (skip with a message) where `ps -o rss` differs. Acceptance: both cases pass on macOS; the construction below fails the self-test. Construction: swap the fields at line 109 to `print rss[p], p`: every recipe still passes, and pid numbers are compared against `lim_kib`, so the next runaway is never killed.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### tools-30 (low, verification): roster: pending Finch's approval

readme's crate roster is hand-maintained with no totality check, and `image_refs` is never exercised by the self-test

- Owner-gated: no

Resolution: derive the crate set from `cargo metadata --no-deps` packages whose README contains the markers, keeping only `image_refs` as per-crate data; or, minimally, assert in `check()` that the marker-bearing README set equals CRATES and fail by name. Add a self-test case for the comment-wrapped image reference. Acceptance: adding the markers to crates/before-viz/README.md with no roster change makes `just readme-check` derive it or fail naming it; `tools/readme self-test` has an `image_refs` case.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### tools-32 (low, simplification): roster: pending Finch's approval

workflowlint spells the interpreter roster twice

- Owner-gated: no

Resolution: build the alternation from the set (`"|".join(map(re.escape, sorted(INTERPRETERS)))` joined with the python form) and compile `SUBST_FETCH` from it. Acceptance: each interpreter name appears once in the file; a self-test case adds a name to the set and shows both the pipeline and substitution forms red.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### tools-9 (low, documentation): roster: pending Finch's approval

benchjudge cites code that no longer exists and narrates its own migration

- Owner-gated: no

Resolution: benchjudge:128-130: "This ceiling separates the divide-and-conquer class from the schoolbook (quadratic) class." Delete the sibling clause at 65-67 and in the JSON notes; at 29-30 write "at the general ceiling — the same convention"; at 857 and 864 name the attack ("a roster class cannot select a ceiling"; "a rostered red is still judged at its own ceiling"); reflow the orphaned short lines at 23 and 128. memwatch:4-8: state the invariant ("a codegen runaway fails the build with the crate named instead of wedging the machine") and leave the incident to git. Acceptance: `grep -rn 'wall-ratio\|expected-failure\|moved here\|review.s .*attack\|once made' tools/` is empty.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### clippy-pedantic-7 (nit, simplification): roster: pending Finch's approval

A bundle of pedantic hits that are pure spelling improvements with no behavior change

Where: `crates/before/src/codec/base.rs:46-50`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): One mechanical commit of the listed spellings (`is_ok_and`, `ilog2`, `&self`, elided lifetimes, digit separators)

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### deps-13 (nit, simplification): roster: pending Finch's approval

before names one type two ways: `dashu_int::UBig` in codec, `suanpan::UBig` in meter and the query tests

Where: `crates/before/src/meter.rs:1660-1663`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `use dashu_int::UBig;` in meter.rs and query/tests.rs

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### deps-14 (nit, simplification): roster: pending Finch's approval

before-fuelscape carries a second bignum (num-bigint + num-traits) beside the dashu-int already in its graph

Where: `crates/before-fuelscape/Cargo.toml:42-45`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Port count.rs and sample.rs to dashu-int's `rand` feature; hold the atlas byte-identical

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### gate-legs-12 (nit, documentation): roster: pending Finch's approval

`just --list` renders eight recipe descriptions as sentence fragments

Where: `justfile:2-3`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Insert a blank line and a one-sentence doc comment above each of the eight recipes

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### inventory-11 (nit, simplification): roster: pending Finch's approval

Capacity hint spelled as an `expect` where the crate elsewhere degrades to zero

Where: `crates/before/src/version/rank.rs:838-843`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `unwrap_or(0)` with the hint sentence

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### inventory-8 (nit, simplification): roster: pending Finch's approval

`#[allow(clippy::result_large_err)]` on `Clock::join_all` carries no local justification

Where: `crates/before/src/clock.rs:257`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Put the rationale at line 257

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### module-graph-10 (nit, simplification): roster: pending Finch's approval

The production module cycles are facade/engine pairs; one (codec::display -> idbits) is the substrate reaching up

Where: `crates/before/src/codec/display.rs:1-3`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Consider moving `write_id` beside `idbits`; leave the five facade pairs

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### module-graph-12 (nit, simplification): roster: pending Finch's approval

Redundant cfg on the exported law-roster macro inside an already-gated module

Where: `crates/before/src/laws.rs:107-109`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Drop the redundant cfg attribute

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### suite-economics-7 (nit, simplification): roster: pending Finch's approval

surface-scan test fixtures are written under the shared temp dir and never removed

Where: `crates/surface-scan/src/tests.rs:14-25`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `tempfile::tempdir()` from `fixture`

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### tools-10 (nit, simplification): roster: pending Finch's approval

`checked_denominators` runs twice per judged cell (the call inside `fit_exponent` can never raise), and mutantcheck computes each pattern's listed and suppressed counts twice

Where: `tools/benchjudge:244-247`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Drop the call at benchjudge:246 and say in `fit_exponent`'s docstring that the caller validated the pair; a `counts(rx, raw, filtered)` helper in mutantcheck

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### tools-21 (nit, simplification): roster: pending Finch's approval

Two overlapping fixture suites (`summary_cases` 3-tuples and the named `cases` 4-tuples) drive `long_summaries`, pinning the crate-root exemption twice

Where: `tools/doclint:279-282`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Fold `summary_cases` into `cases` with names, one loop

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### tools-24 (nit, simplification): roster: pending Finch's approval

One self-test expectation is a bare tuple, and the `isinstance` branch at 149-150 exists only to normalize it

Where: `tools/manifestlint:140-143`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Write the expectation as a one-element list; delete the branch

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### tools-27 (nit, documentation): roster: pending Finch's approval

Two prose sites restate the pinned cargo-mutants release that `tool` in tools/mutantcheck-expected.json holds, and edits left docstring lines orphaned mid-clause (mutantcheck:19-20, benchjudge:23 and :128, justfile:308-309)

Where: `tools/mutantcheck:33-36`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Name the pin file instead of the version at both sites; reflow the orphaned lines

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### tools-31 (nit, simplification): roster: pending Finch's approval

House style varies per tool: self-test success announced by five and silent in three; `read_text`/`write_text` without an encoding at four sites; three defs without docstrings; argparse in four tools and hand-parsed argv in six

Where: `tools/testdoc:111-113`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): One convention per axis: `<tool>: self-test ok`, `encoding="utf-8"` on every call, a docstring per def, one argv style

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

