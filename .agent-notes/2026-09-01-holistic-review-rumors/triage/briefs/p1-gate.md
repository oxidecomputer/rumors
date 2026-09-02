<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P1 lane: the gate, CI, tooling, and docs.rs holes

## Goal

The gate is what every later lane is judged by, and the review found holes
in its own instruments: a test binary compiled out of every committed run,
coverage legs in no composite recipe, a `testdoc` that cannot see two test
forms and walks other agents' worktrees, a CI workflow installing unpinned
tools and floating toolchains, a docs.rs configuration that fails docs.rs's
own build, and a public rustdoc quoting a measurement nothing runs. The
invariant restored: every committed check runs under a committed recipe,
every tool the gate shells out to is pinned where its output is pinned,
and the justfile's opening claim that every artifact has a recipe is true.

## Ground rules

These apply to every P1 lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal `0926fe32` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `0926fe32`, fast-forward; if it has diverged, stop and report. Never call
  EnterWorktree; operate on the worktree through `git -C <path>` and
  absolute paths, one shell invocation at a time.
- **The review documents are the specification.** Each member entry below
  quotes its Resolution and Acceptance verbatim from the topic document in
  `.agent-notes/2026-09-01-holistic-review-rumors/`; the entry's full record
  (evidence, construction, demonstration) is under `### <id>:` there and in
  `evidence/`. Line anchors are at the reviewed commit `9e5784fb`; re-anchor
  from the quoted evidence, never from the line numbers.
- **Goal beside mechanism.** Where a quoted resolution and the goal above
  come apart, the goal wins, and the discrepancy is reported.
- **Stops.** Report and leave the entry open; do not work around: anything
  that moves an `insta` snapshot or a committed pin; any change to a public
  signature or public rustdoc contract the resolution does not name;
  anything that contradicts a ruling in `triage/rulings.md`; any deviation
  from the stated resolution; anything this brief marks as a stop. A stop on
  one entry does not block the others.
- **Negative controls.** Every repaired instrument lands with a committed
  demonstration that a known-bad artifact fails it. The constructions in
  `evidence/witness.md` and each entry's Construction line are those
  artifacts; convert each into a committed test (`should_panic`, an asserted
  `Err`, a `--self-test` case, or a reversible mutation whose observed
  failure the commit message records verbatim).
- **Resource discipline.** Build with `cargo nextest run --no-run` before
  running; during iteration run only the binaries the entry names; never
  iterate on a timing measurement. One full `just gate` before each commit,
  run in the background redirected to a log under `<scratchpad>/p1-gate/`,
  polled with short foreground checks (the foreground command cap is ten
  minutes). Keep every working file under that directory. `just all` and
  `just ci` are run once each at the end, not per commit.
- **Commits.** One commit per logical unit, its message describing the
  change and naming the entry ids and ruling. Commit every proptest seed
  file that appears. Prose speaks in the present tense: no reference to code
  that no longer exists, no dated rationale at a declaration site. Comments
  use spaced double-hyphens, never em-dashes; every test has a doc comment
  stating its invariant. Never delete anything outside your worktree; if the
  disk fills, stop and report.
- **Self-retirement.** After the final commit, from inside the worktree:
  `cargo metadata --no-deps --format-version 1 | jq -r .build_directory`,
  then delete that directory and the worktree's `target/`. Leave the
  worktree in place.
- **Report.** For each entry: landed, stopped, or open; the commit sha(s);
  the acceptance evidence (the command and its decisive output, verbatim).
  Then anything left open and why. Your report is data: the coordinator
  verifies each entry's Acceptance against the tree at the reported sha
  before the ledger records it. Report what you could not do rather than
  working around it.

## Members

### tests-wire-format-26 (high): ruling T7, amended

Resolution: Lift the cfg and pin a budget that holds in both profiles: the claim is that the public futures hold only a `Pin<Box<dyn Future>>` plus locals, which is a layout fact in either profile; measure the three sizes once in dev and release (a handed number is a hypothesis), set `PUBLIC_FUTURE_BUDGET` with headroom, and state the measured band in its doc, replacing "a few hundred bytes". This keeps the gate's build set unchanged and keeps the guard inside the mutants observer, which a release-only leg would not. Add `gossip_when`'s stream (boxed at gossip.rs:946, unmeasured) to the measured surface. Add an adequacy demonstration: the commit landing the fix records that removing the `Box::pin` in `Reconciliation::reconcile` trips the budget. If the owner prefers a release leg instead, add `cargo nextest run -p rumors --test future_size --cargo-profile release` to `gate-streams` and `just ci` with a recipe comment stating why this one binary runs at release. Whichever option, add a liveness guard that the tests were collected (nextest's `--no-tests=fail`, or a count check in the recipe), since the failure mode is an empty binary reading as a pass (carried from verification-infra-1). Acceptance: `cargo nextest list -p rumors --test future_size` under the gate's invocation lists the three (or four) tests; `just gate` runs and passes them; a deliberate removal of the `Box::pin` at gossip.rs:1128 fails them.

Amendment (T7): the release-leg alternative is withdrawn; the cfg is
lifted and the budget pinned under the dev profile (measure both profiles
once as the resolution says, so the doc can state the band). A first run
that fails the existing budget is a finding to report with the measured
sizes, not a reason to gate the binary back out. The `--no-tests=fail`
liveness guard lands.

### tests-wire-format-25 (medium): rulings T7, T26

Resolution: Restate the mechanism against today's code: the deep type is the typed phase schedule under `streaming::protocol`; the erasure boundaries are `Reconciliation::reconcile`'s boxed, `inline(never)` future (with `Handshaken::reconcile` below it) and `gossip_when`'s boxed unfold; point the assert messages at `Reconciliation::reconcile` by name; fix traverse.rs:9-12 in the same pass. Acceptance: `grep -n -E 'Levels|Below<|mirror\(\)' tests/future_size.rs src/tree/traverse.rs` returns nothing; the module doc names `Reconciliation::reconcile`.

### verification-infra-2 (medium): ruling T15

Resolution: add `coverage-kernel coverage-kernel-branch` to `all` (they are deterministic-verdict legs and `all` is already the slow sweep), or add a `ci-full` recipe that is `ci` plus the instruments and coverage job legs; then re-state justfile:12-14 so `ci` names only the ci job and `all` names what it actually covers. The `before` meter failure is a separate question for its owner (see open questions). Acceptance: at a commit where CI's coverage job fails, `just all` fails on the same leg.

Ruled (T15): the first option, into `all`; no `ci-full`. If `just all`
then fails on the `before` meter at your base, that is expected and is
the acceptance; report the failing leg's output and do not touch `before`.

### verification-infra-14 (low) and tests-resource-link-window-18 (medium): ruling T16, resolutions replaced

verification-infra-14, Resolution: add a `tradeoff-probe` recipe wrapping the documented command and include it in `all`, or amend the justfile's totality claim to name the hand-run exceptions. Acceptance: `grep tradeoff justfile` names the recipe and the module doc points at it instead of a bare command.

tests-resource-link-window-18, Resolution: In either outcome, state the cost rationale in the module doc at the `#[ignore]`. Then, owner's call: (a) add a `just tradeoff-probe` recipe (`cargo nextest run --release --test tradeoff_probe --run-ignored all`) to the CI `instruments` job beside `worst-cases-pin`, with the module doc pointing at the recipe instead of spelling the cargo command; or (b) retire the file, moving the design-corpus and design-record cell into an enforced suite if that claim is wanted, and re-denominating or excising the peer.rs:414-417 citation. Acceptance: the module doc names the cost; and either `grep -n tradeoff_probe justfile .github/workflows/*.yml` finds the recipe and step, or the file is gone and peer.rs cites no unrun instrument.

Ruled (T16): neither option as stated. No recipe; the file stays; its
module doc states the cost rationale at the `#[ignore]` and that it is a
hand-run instrument; `Peer::sync_memory_budget`'s rustdoc stops quoting
the probe's measurement (the sentence at peer.rs:414-417 is excised or
restated without the figure and the file citation); the justfile's opening
totality claim is restated to admit hand-run probes. Acceptance: `grep -n
tradeoff_probe src/peer.rs` is empty; the module doc names the cost; the
justfile header names the exception class.

### deps-3 (medium): ruling T20

Resolution: Change lib.rs:299 to `#![cfg_attr(docsrs, feature(doc_cfg))]` (under RFC 3631, `doc_cfg` labels gated items automatically; `#[doc(auto_cfg = false)]` opts out) and reword Cargo.toml:91 and lib.rs:297-298 to name `doc_cfg`. Add a leg to `ci` (and consider `gate`; it is one nightly rustdoc of one crate): `RUSTDOCFLAGS="--cfg docsrs -D warnings" cargo +{{ nightly_toolchain }} doc -p rumors --all-features --no-deps --target-dir target/doc-docsrs`. Acceptance: a committed justfile leg runs rustdoc for rumors under the pinned nightly with `--cfg docsrs` and `-D warnings`, `just ci` includes it, and lib.rs and Cargo.toml name the gate that leg accepts.

Ruled (T20): the leg is in both `gate` and `ci`. Negative control: the
witness pass's run (`evidence/witness.md`, section `deps-3`) shows the
E0557 failure at HEAD; record it in the commit message.

### deps-7 (low): ruling T28

Resolution: Install the pinned toolchains by name in the workflow (the stable step names `1.97.1`, or is dropped if the action honors `rust-toolchain.toml` without a `toolchain:` input; the nightly step names `nightly-2026-06-30`, ideally read from the justfile so the two cannot diverge, or the justfile's pin comment names ci.yml as the second site to move), and rewrite the comments at 17-20 and 54-58 to describe the pins. Acceptance: the workflow names no floating channel; `grep -n 'cargo +nightly' .github/workflows/ci.yml` is empty; the nightly pin appears in exactly one place or its two sites name each other.

### verification-infra-12 (low): ruling T28

Resolution: install the pinned nightly by name (read `nightly_toolchain` from the justfile in a step, or drop the nightly install and rely on rustup's provisioning as AGENTS.md describes), rewrite the four comments to today's mechanism, and either install `llvm-tools` on the pinned nightly or state that cargo-llvm-cov provisions it. Acceptance: no comment in ci.yml names a floating `nightly` as the toolchain a recipe uses, or a lib bench compile.

One change with `deps-7`; `tools/workflowlint` must still pass.

### verification-infra-13 (low): ruling T28

Resolution: for each of the eight recipes, end the explanatory block with a blank line and add a one-line summary comment directly above the recipe (or its attribute line), the pattern the other recipes already follow. Acceptance: every line of `just --list --unsorted` is a complete sentence.

### deps-8 (nit): ruling T28

Resolution: `bytes = { workspace = true }`

### deps-9 (nit): ruling T28

Resolution: Delete `default-features = true` from the tokio dev-dependency entry

### verification-infra-7 (medium): ruling T26

Resolution: add `manifestlint` to the `ci` recipe line; it is build-free and python3 is already a CI prerequisite. Acceptance: `just ci` fails on a scratch manifest edit that restates a workspace version.

### verification-infra-8 (medium): ruling T26

Resolution: pin `cargo-mutants@27.1.0` in the install step and rewrite the comment: two tools carry versions because two committed artifacts depend on them, and bumping either is a reviewed diff that re-pins `tools/mutantcheck-expected.json` or the READMEs in the same commit. Acceptance: `tools/workflowlint` still passes and the install line names both pins.

Ruling T11 disputes a rumors mutation campaign; this pin concerns the
`before` count pin that CI already runs and is unaffected.

### verification-infra-9 (medium): ruling T26

Resolution: give testdoc the same explicit roots doclint uses (`./tools/testdoc benches crates examples src tests`), or have it walk `git ls-files '*.rs'`; add `.claude` to the ignore set as a second guard and a self-test case for the ignore list. Acceptance: an undocumented test placed under `.claude/worktrees/` does not change `just testdoc`'s verdict.

### verification-infra-10 (low): ruling T28

Resolution: track `proptest! {` blocks lexically and treat each depth-1 `fn` inside as a test entry point; add both the block form and the omitted-attribute case to `--self-test`. Acceptance: the scratch file above yields two findings.

### tests-observation-37 (medium), with remote-proxy-tests-27 and tests-disruption-handshake-33 as duplicates: ruling T30

Resolution: Add `pollster::test` to the alternation (or match any `<path>::test` form) and add a `#[pollster::test]` case to `--self-test`. Independently, tests-observation-8 removes pollster from changes.rs. Acceptance: `./tools/testdoc --self-test` covers `#[pollster::test]`; stripping the doc above tests/changes.rs:18 fails `just testdoc`.

Take the general form the duplicates propose (any path ending in `test`,
beside `rstest|test_case`). `tests-observation-8` is a P5 entry; do not
touch `tests/changes.rs`. Run `just testdoc` on the tree after widening:
if any of the 29 pollster tests is undocumented today, that is a finding;
document it in the same commit and say so.

### deps-1 (medium): ruling T26

Resolution: Delete `zero_size_val` (its testdoc claims a value/type distinction that does not exist for `Sized` types, and `zero_size` covers both of its checks). Replace the remaining six assertions with `const _: () = assert!(size_of::<Z>() == 0 && align_of::<Z>() == 1);` and likewise for `S<Z>` and `Root`, either in `height.rs` beside line 166 (they are compile-time facts and need no `#[test]` wrapper to fire) or in `tests.rs` with the two testdocs kept. Remove `static_assertions` from rumors' `[dependencies]` (the workspace table entry stays; `crates/before` inherits it at its line 28). Make the crate attribute an unconditional `#![forbid(unsafe_code)]` and delete the comment at lib.rs:295. Acceptance: rumors' `[dependencies]` has no `static_assertions` entry; lib.rs carries `#![forbid(unsafe_code)]` with no `cfg_attr`; the three layout facts are `const _` assertions that fail compilation if a height type gains size or alignment; `just gate` is clean.

### streaming-tests-28 (low): ruling T29 (model)

Ruled: `lean_wedge_literal()` stays a hand transcription; no gate leg. The
one change is its rustdoc: state that the transcription is human-checked
against `Instances.lean`'s `Mux.wedge` whenever either side changes, and
why that suffices (twelve scopes; the generator pin and the session pin
hold the Rust side rigid; a Lean edit to the witness is an owner-level
change to the theorem's subject). Acceptance: the doc states the manual
discipline and its reason; no code change.

## Hazards and stops

- `src/lib.rs` is shared with `p1-envelope`; land this lane first.
- `Peer::sync_memory_budget`'s rustdoc is public prose. The T16 edit
  removes the probe citation and its figure only; the sizing guide's
  relocation is owner decision 80 (P4) and out of scope. Any wider edit
  there is a stop.
- The justfile header restatement (T15, T16) is small and named; do not
  reflow the header.
- If `future_size`'s first run under the dev profile fails its budget,
  report the measured sizes and stop on that entry; do not raise the
  budget to make it pass without saying so.
- Deleting the `Box::pin` for the adequacy demonstration is a reversible
  mutation in `src/peer/gossip.rs`, never committed.
