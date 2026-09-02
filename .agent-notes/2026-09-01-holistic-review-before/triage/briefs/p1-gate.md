<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P1 lane: the gate's own provenance

## Goal

The gate is what every later lane is judged by, and the review found that
its own instruments are not bound to what they judge: a CI coverage job
red on a byte-identical tree with no local leg that would have shown it;
a lockfile the supply-chain audit never reads, carrying an advisory-listed
wasmtime; a 32-bit leg that runs only in the local gate; nightly and tool
installs in CI that float free of the justfile's pins; a mutation-campaign
roster and a count-checking leg whose authority (a recurring campaign) is
ruled never to exist; and a fuzzfit lint leg whose liveness is in doubt.
The invariant restored: every committed check runs under a committed
recipe or is declared local with its reason, every list the gate iterates
is derived rather than hand-kept, every tool and toolchain the gate shells
out to comes from one pinned source, and nothing in the tree names an
authority that does not run.

## Ground rules

These apply to every P1 lane; the lane sections below add to them and
never relax them.

- **Base.** This lane is stacked on the rumors triage's gate lane
  (ruling 107): your worktree's HEAD must equal the SHA the coordinator
  names in your launch message, the tip of the branch `triage/p1-gate`.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of that
  SHA, fast-forward; if it has diverged, stop and report. That branch
  already edits `ci.yml`, the justfile header and `ci` recipe, the root
  lockfile, and `tools/testdoc`; read its diff against `main` before you
  edit any of those files, and never rewrite its commits. Never call
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
- **Prose (ruling 105).** Every paragraph you touch passes the three tests in
  `PROSE.md` (altitude, concision, legibility); the reviewer applies its
  checks; the diff is net shorter in prose unless your report says what the
  additions buy.
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

## Ordering inside the lane

1. gate-legs-1 first, before anything else in this lane or any other
   lane touches a heap pin under coverage: run `just coverage-kernel`
   twice at your base and record both `MEASURED masked_cmp_hole` lines
   under `<scratchpad>/p1-gate/coverage-{1,2}.log` before choosing the
   remedy.
2. fuzzfit-strategies-13's `just fuzzfit` run next, since its outcome is
   a process finding the coordinator wants early.
3. The roster retirement (ruling 18) before the CI tool-pin edits
   (ruling 25), since the retirement decides what the install step
   contains.
4. gate-legs-4 (ruling 50) with the roster retirement, since both edit
   the `ci` recipe line: `ci` is defined from `gate-lints` in the same
   commit that removes the mutants leg from both.
5. gate-legs-8 (ruling 50): the writer-door law and the coverage-suite
   assertion; library-side files no other P1 lane touches.
6. The lockfile audit and wasm32 leg (ruling 16) last: the wasmtime bump
   inside `wasm32-pins` wants `just wasm32-pins` run once after it, and
   that is minutes and a few GiB.

Files this lane shares with others: none of the lane files above are
touched by another P1 lane, except `fuzzfit/harness/src/ops.rs`, which
`p1-fuzz` also edits (its bands and `Op` vocabulary). Land the two-cast
removal as its own small commit so the fuzz lane rebases over it cleanly.

## Members

### gate-legs-1 (high, correctness): ruling 24

Resolution: Settle the source before touching the pin. Run `just coverage-kernel` twice at 9e5784fb and compare the `MEASURED masked_cmp_hole` lines with the uninstrumented 384. If the instrumented reading moves between identical runs, find what allocates inside the scenario body only under `-C instrument-coverage` (or only sometimes) and either isolate the meter from it or exclude the heap column under coverage builds with the reason stated at the exclusion. If the instrumented reading is stable but differs from 384, the envelope suite and the coverage leg judge different profiles, and the coverage recipes should filter the meter suite out (they exist to measure kernel coverage, not envelopes). Never widen 480 to accommodate. Acceptance: two consecutive green `coverage` jobs on main with no change to any envelope constant, and a comment at the chosen site naming the mechanism.

Ruled (24): exactly this, at your base (which is byte-identical to
`9e5784fb` outside `.agent-notes/`). The 480 B pin at
`tests/meter.rs:6860` is never widened. Both remedies stay open until the
two readings are in hand; choose by the readings, say which branch the
readings selected, and quote them. The acceptance's "two consecutive green
coverage jobs on main" is the coordinator's to observe after merge; your
acceptance evidence is the two local `MEASURED` lines, the chosen
remedy's commit, and a third `just coverage-kernel` run green after it.
If the readings are stable and differ from 384 and you filter the meter
suite out of the coverage recipes, the recipe comment states that the
recipes measure kernel coverage and that the envelope suite is judged
under the uninstrumented profile by `test-all`.

### fuzzfit-strategies-13 (low, vestigial): ruling 28

Resolution: drop both casts and run the fuzzfit recipe's clippy line. Acceptance: `grep -n 'encoded_bits() as u64' ops.rs` is empty and `cd crates/before/fuzzfit && cargo clippy --all-targets -- -D warnings` is clean.

Ruled (28): before removing the casts, run `just fuzzfit` once on the
untouched base and capture its full output to
`<scratchpad>/p1-gate/fuzzfit-base.log`. If the clippy line fails on the
two casts, the fuzzfit gate stream had not been running when 4e64a4fb
reported a clean gate: report that as a process finding with the log,
then remove the casts. If it passes with the casts present, report that
too (it means `unnecessary_cast` does not fire here, and the entry's
premise about the lint is wrong; the casts still go as vestigial). Either
way the acceptance grep and clippy run are the evidence.

### deps-1 (high, verification-gap): ruling 16

Resolution: enumerate lockfiles mechanically in the recipe body (a loop over `git ls-files -z -- '*Cargo.lock'` feeding `cargo audit --file`), so a new detached workspace is audited the commit its lock lands; re-denominate the four prose enumerations to "every committed Cargo.lock"; bump wasm32-pins to wasmtime 47.0.4 (`cargo update -p wasmtime` inside the workspace) and run `just wasm32-pins` to confirm every pin's outcome is unchanged. Acceptance: `just supply-chain` at HEAD fails naming RUSTSEC-2026-0269 in crates/before/wasm32-pins/Cargo.lock; after the bump it passes; a seventh lockfile added anywhere in the tree is audited with no recipe edit.

Ruled (16): as stated. Land the mechanical enumeration first as its own
commit and record the red run's output (the advisory named against the
wasm32-pins lock) in that commit message as the negative control; the
wasmtime bump is the next commit. The "seventh lockfile" acceptance
clause is demonstrated by a scratch lockfile under the worktree that is
removed before commit, with the run recorded in the commit message. If
`just wasm32-pins` changes any pin's outcome after the bump, that is a
stop: report the changed pin, do not re-pin.

### gate-legs-3 (medium, verification-gap): ruling 16

Resolution: Add `cargo audit --file crates/before/wasm32-pins/Cargo.lock` to `supply-chain`, or drive the list from `find . -name Cargo.lock -not -path '*/target/*'` inside the recipe so the enumeration cannot rot again; fix the four prose rosters; then either add `wasm32-pins` to the `instruments` job or add it to that job's "what stays local, and why" comment with the reason (justfile:634-636 says it peaks at a few GiB across nextest workers). Acceptance: `just supply-chain` names every lockfile `find` reports, and every roster that enumerates detached workspaces names wasm32-pins or is replaced by a mechanical enumeration.

Ruled (16): the `git ls-files` enumeration (one mechanism with deps-1,
not `find`), and the wasm32-pins leg joins the `instruments` CI job
rather than the local-only comment. The four prose rosters (justfile:330,
justfile:390, deny.toml:4, rust-toolchain.toml:3) are re-denominated to
"every committed Cargo.lock" or made to derive; none keeps a hand list.

### fuzz-guests-pins-37 (medium, verification-gap): ruling 16

Resolution: Add the wasm32-pins lockfile, and derive the list mechanically so the next detached workspace cannot be missed: `git ls-files '*Cargo.lock' | xargs -n1 cargo audit --file` (or a `find` excluding `target/`), and drop the hand enumeration at 329-330. Consider aligning the three wasmtime pins while touching the locks. Acceptance: the recipe audits every committed lockfile without naming them; adding a lockfile to the tree changes nothing in the justfile.

Ruled (16): one change with deps-1 and gate-legs-3. The wasmtime pins
align at 47.0.4 through deps-1's bump.

### fuzz-guests-pins-1 (medium, verification-gap): ruling 16

Resolution: Owner call. Either add `wasm32-pins` to the `instruments` job (the recipe prices it at minutes and a few GiB across nextest workers; it builds `before` for wasm32 plus wasmtime), or add a bullet to both comments declaring it local and why. Acceptance: every leg in the gate's stream roster (justfile:464-471) either appears in `ci` or the `instruments` job, or is named in the local-only list with its reason.

Ruled (16): the first option. `wasm32-pins` joins the `instruments` job.
The two local-only comments (ci.yml:110-120, justfile:985-986) then
describe what still stays local (the wall-time judge, the wasmtime fuel
tier) and nothing else; `fuzzfit` is part of that fuel tier and stays
local, which the comments already say. Do not add `fuzzfit` to CI.

### deps-9 (low, verification-gap): ruling 16

Resolution: state the policy once, at the mechanical lockfile enumeration the deps-1 fix introduces: either a convention ("every detached lock is updated in the same commit as a root `cargo update`") or a cheap cross-lock diff in that leg (a small tool over `git ls-files -- '*Cargo.lock'` reporting any crate resolved at differing versions across locks; crates absent from the root, such as wasmtime, need no entry). Acceptance: the policy sentence exists in the recipe comment, and if the tool is taken, it fails at HEAD on dashu-int, borsh, and bytes and passes after one `cargo update` sweep.

Ruled (16): a convergence policy is stated. Take the cheap cross-lock
diff (the tool), since a stated convention with nothing checking it is
the convention-held-in-memory the doctrine forbids; land it red-first
(its failure on dashu-int, borsh, and bytes recorded in the commit
message), then the `cargo update` sweep across the detached locks in the
next commit with `just wasm32-pins`, `just fuzzfit`, `just fuzz-build`,
`just fuelscape-test`, and the surfacecheck recipe run once each
afterwards. If any of those legs changes a pinned outcome after the
sweep, stop on that leg and report; do not re-pin. If the sweep would
move a crate the fuzzfit bands' provenance note binds (wasmtime), state
the version movement in the commit and confirm the bands still pass.

### deps-2 (medium, verification-gap): ruling 16

Resolution: choose and make the text match. Either (a) set `multiple-versions-include-dev = true` under `[bans]` and roster the holdouts with their reasons (rand 0.8 held by the workspace pin against proptest's 0.9; thiserror 1 through ratatui's termwiz chain), which restores the header's promise and lets the roster shrink visibly as they converge; or (b) state the dev-edge exemption in both comments. The rand convergence path (workspace `rand`/`rand_chacha` to 0.9) touches rumors' normal dependency at Cargo.toml:142 and reseeds every corpus drawn through `gen_range`, so it is a separate, owner-ruled decision and is listed under open questions rather than proposed here. Acceptance: with option (a), `cargo deny --workspace check bans` at HEAD reports rand, rand_chacha, rand_core, and thiserror and passes once the skip entries name them; with option (b), the deny.toml and justfile comments both say dev-only duplicates are out of scope.

Ruled (16): option (a). The rand convergence itself is not this lane's
(it reseeds rumors' corpora and is owner-ruled elsewhere); the skip
entries name the four holdouts with their reasons. Record the red
`cargo deny` output before the skips land in the commit message.

### gate-legs-6 (medium, verification-gap): ruling 18, resolution replaced

Resolution: Add a campaign recipe of record (`cargo mutants --workspace` under the roster's keys) and a cheaper `--in-diff` variant for PRs; give the full campaign a cadence that fits its cost (a scheduled, sharded CI job, or a committed dated attestation of the last full run that a lint leg holds fresh), and record the expected outcome (zero MISSED under the roster) so a survivor is a red, not a report. Acceptance: a recipe exists whose exit status is the campaign verdict, and something in the tree or CI names when it last ran.

Ruled (18), Finch's words: "Don't do anything of the sort. This was a
one-time campaign." No recipe, no cadence, no attestation. Instead the
one-time campaign's residue retires: delete `.cargo/mutants.toml`,
`tools/mutantcheck`, `tools/mutantcheck-expected.json`, the
`mutants-list` recipe and its place in `ci` and `gate`, and the
cargo-mutants install in `.github/workflows/ci.yml`. Then restate every
sentence that names the roster or the campaign as an authority so no
ghost reference remains: `AGENTS.md`'s "Mutant exclusions" paragraph
under "Writing tests" (restate the standing policy it carries, which is
about refactoring unreachable branches into structural nonexistence and
making truly-unreachable branches assert, without the roster as its
home), `tools/workflowlint` (deleted whole under ruling 106),
`crates/before/tests/meter.rs:5073-5077` (the eq early-exit band cites
the roster's `sweep::eq_exit` entry; ruling 20's restatement in the
suites lane covers that doc, so coordinate: this lane deletes the roster,
the suites lane rewrites the citation; if you land first, leave the band
doc and say so in the report), and any `.agent-notes`-external prose
found by `grep -rn 'mutants\|mutantcheck' --exclude-dir=.agent-notes
--exclude-dir=target .`. Acceptance: that grep returns nothing outside
`.agent-notes/` and git history; `just gate` and `just ci` list no
mutants leg; `tools/workflowlint` is deleted by this lane (ruling 106).

The rumors gate lane, on whose branch you are stacked, has agreed to
drop the cargo-mutants install and pin from `ci.yml` itself (ruling
107's outcome), so expect to find none there; verify with a grep and
report if one remains. Do not amend or reorder the rumors lane's
commits. The root `AGENTS.md` paragraph on mutant exclusions is yours to
restate, as the entry says.

The mutants roster is also where suanpan-40's two exclusions live; their
code-side dissolution is `p1-survivors`' work and needs nothing from you
beyond the deletion.

### gate-legs-2 (medium, correctness): ruling 25, resolution replaced

Resolution: Pin `cargo-mutants@27.1.0` beside `cargo-rdme@2.1.0` on ci.yml:86 and reword lines 76-82: the pinned tools are those whose version is a committed expectation (cargo-rdme's emitted bytes; cargo-mutants' version string and operator inventory), and a bump touches the expectation file and the install line in one diff. Acceptance: the workflow names the same cargo-mutants version as tools/mutantcheck-expected.json, and tools/workflowlint (or a one-line grep in `mutants-list`) fails when the two disagree.

Ruled (25): cargo-mutants is not pinned; it leaves CI with the roster
(ruling 18). The install-step comment at ci.yml:76-82 is reworded to what
remains true: `cargo-rdme@2.1.0` is pinned because its emitted bytes are
a committed expectation (the READMEs), and every other installed tool
only reports findings. Acceptance: no `cargo-mutants` in ci.yml; the
comment names the one pinned tool and the reason. The comment as found
at your base is the rumors lane's wording (two pinned tools); restate it
to the one that remains. `tools/workflowlint` is deleted (ruling 106), so
its passing is not an acceptance.

### deps-6 (medium, documentation): ruling 25

Resolution: install the pinned toolchains from one source: read `just --evaluate nightly_toolchain` in a step and pass it as `toolchain:` (with `llvm-tools` in the coverage job), drop the floating nightly installs; drop the stable install steps or re-denominate their comments to "rust-toolchain.toml provisions 1.97.1 with clippy, rustfmt, and wasm32"; rewrite lines 17-25, 54-58, 128-133, 139-141, and 202-205 to the pinned regime. Optionally extend tools/workflowlint to require every dtolnay `toolchain:` input to equal the justfile pin or the rust-toolchain.toml channel. Acceptance: no `toolchain: nightly` or `toolchain: stable` remains in ci.yml; every comment naming a toolchain names the pinned one; the three jobs stay green.

Ruled (25, 106, 107): the mechanism is `just --evaluate
nightly_toolchain`, `just` installed before the toolchain steps (the
install action ships it as a prebuilt binary; no toolchain is needed to
run it). Your base carries, in each of the three jobs, a "Read the
toolchain pins" step that derives both pins with `sed` over
`rust-toolchain.toml` and the justfile; the coordinator's launch message
says whether the rumors lane has already replaced that step with the
`just --evaluate` derivation (then verify it and record the entry as
landed there) or whether this lane replaces it (then the nightly derives
from `just --evaluate` and the stable pin may keep its
`rust-toolchain.toml` read, since the dtolnay action reads no toolchain
file). `tools/workflowlint` is deleted entirely under ruling 106, so no
workflowlint extension is
taken: a `toolchain:` input in the workflow must be the derived
`nightly_toolchain` output or absent (rust-toolchain.toml provisions
stable). Land the lint extension red-first against the untouched
workflow and record its failure in the commit message. Do not write the
dated nightly string into ci.yml by hand anywhere; the single source is
`just --evaluate nightly_toolchain`. "The three jobs stay green" is
observed by the coordinator after merge; `tools/workflowlint` is deleted (ruling 106), so your evidence is the workflow diff itself
passing and `act`-free reasoning about the step (state which step reads
the value and how it is consumed).

### surface-roster-1 (low, correctness): ruling 25

Resolution: install the dated toolchain in CI from the justfile's single pin (a step that writes `just --evaluate nightly_toolchain` to `GITHUB_OUTPUT`, consumed by the `toolchain:` input), and rewrite ci.yml:128-133 to say the job runs the pinned nightly. The `ci` and `coverage` jobs' `toolchain: nightly` steps (64-67, 206-210) feed `doctest`, `fuzz-build`, and the branch-coverage leg, all spelled `+{{ nightly_toolchain }}` too; same fix, outside this partition. Acceptance: ci.yml derives `nightly-2026-06-30` wherever a `+{{ nightly_toolchain }}` recipe runs; the comment matches the mechanism; bumping one side alone fails the job with surfacecheck's format_version message, not a rustup error.

Ruled (25): one change with deps-6, covering all three jobs.

### gate-legs-4 (medium, verification-gap): ruling 50

Resolution: Add `manifestlint` to the `ci` line (build-free, seconds), and consider defining `ci` as `gate-lints` plus its build legs so the two rosters cannot diverge again. Acceptance: every recipe named in `gate-lints` appears in `ci`, ideally by construction.

Ruled (50): the second half is taken, not merely considered: `ci` is
defined as `gate-lints` plus its build legs, so every gate lint is in CI
by construction and `manifestlint` comes with it. The justfile's tier
comments describe the derived shape. Negative control: the commit message
records `just --evaluate` (or `just --show ci`) at the parent listing no
`manifestlint`, and after the change listing it.

### gate-legs-8 (medium, verification-gap): ruling 50

Resolution: Extend `encode_to_matches_encode` (or add one law over every `*_to` door) to cover the rank, ranked, and span writers, cite it in each of the five rows' `pins`, and add a coverage-suite assertion that a row excluded on all three legs cites at least one resolvable name, so the vacuous shape cannot recur. Acceptance: no roster row has three excluded legs with an empty union of pins, and the new assertion fails when one is introduced.
Construction: Delete the `# Example` block at rank.rs:412-420 and run `just gate`: green (no leg names `Rank::encode_to`).

Ruled (50): as stated. One law over every `*_to` door (extend
`encode_to_matches_encode` or add its sibling), cited from the five rows,
and the coverage-suite assertion that a row excluded on all three legs
cites at least one resolvable name. Under ruling 43 the citation is a
reference the compiler resolves, not a string, if the roster's row type
allows it at your base; if the row type only carries strings, cite by
name and report that the roster's typing is the p2-surface lane's
(meter-registry-tier2-10) to reshape. Negative control: the entry's
delete-the-doctest construction, run as a reversible mutation and
recorded in the commit message.

### meter-adequacy-9 (low, verification-gap): roster: approved (ruling 104)

Resolution: write the sidecar after `wide.bench(c)` returns (the directory
argument at sidecar.rs:149-152 holds either way, since `write_denoms`
creates the parent), or add a completion stamp the judge requires; have
the judge refuse an `estimates.json` older than the sidecar's write. For
the wasm guests, export a build identifier from the guest and assert it
from the harness. Acceptance: a judge run over a baseline pair in which one
cell's estimates predate the sidecar exits 2.

Roster note: a low with two halves. The sidecar-after-bench write and
the judge's refusal of estimates older than the sidecar are this lane's
(`benches/board.rs`, `tools/benchjudge`; the judge acceptance runs over a
synthetic baseline pair, which is not a bench run). The wasm-guest
build-identifier clause touches both guests, which `p1-fuzz` owns; land
the harness-side assert only after `p1-fuzz` has landed, or report it as
handed to that lane. Lands only if Finch approves the roster.

## Hazards and stops

- The 480 B pin at `tests/meter.rs:6860` never widens (ruling 24). Any
  remedy for gate-legs-1 that would change an envelope constant is a stop.
- The wasmtime bump and the cross-lock `cargo update` sweep may move
  detached-workspace behavior: any changed pin outcome in `wasm32-pins`,
  any fuzzfit band failure, any fuelscape pin movement is a stop on that
  entry, reported with the diff, never re-pinned here.
- The roster retirement removes a gate leg. `just gate`'s stream roster
  and `just ci`'s recipe line both change; `just --list` must still read
  as complete sentences and the justfile's tier comments must describe
  what remains.
- The band doc at `tests/meter.rs:5073-5077` is the suites lane's
  (ruling 20). Do not edit it; report if your grep finds it as the last
  roster reference.
- `fuzzfit/harness/src/ops.rs` is shared with `p1-fuzz`: the cast removal
  is one small commit touching two lines and nothing else in that file.
