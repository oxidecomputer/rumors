<!-- CAVEAT LECTOR: review packet for lane p1-gate, written by Claude (Fable 5.1) for Finch under triage/WORKFLOW.md. The goal, rulings, stack position, acceptance rows, fresh-eyes rounds, and stops are the coordinator's record; nothing in this packet is authored or endorsed by Finch. Read with the ground rules in .agent-notes/README.md. -->

# Review packet: p1-gate

## Goal

The gate is the instrument every later lane is judged by, and the review
found holes in it: a test binary no committed run compiles, coverage legs
outside every composite recipe, a `testdoc` blind to two test forms and
reading other agents' worktrees, a CI workflow installing unpinned tools
and floating toolchains, a docs.rs configuration that docs.rs's own build
rejects, and public rustdoc quoting a measurement nothing runs. This lane
closes them so that every committed check runs under a committed recipe,
every tool whose output is pinned is itself pinned, and the justfile's
opening claim that every artifact has a recipe is true, with the hand-run
probes named as the one exception.

## Rulings landed

- T7: the `cfg(not(debug_assertions))` gate on `tests/future_size.rs` is
  lifted; the budgets are re-measured under the dev profile and pinned
  there; a first-run failure is a finding to triage, never a reason to
  gate the binary back out; the module doc is restated against
  `Reconciliation::reconcile`.
- T15: `just all` runs the coverage legs, so the local ladder cannot pass
  while CI's coverage job fails; the justfile header describes `ci` as the
  recipe the CI job runs, not as a mirror of every check; the gate is
  unchanged.
- T16: no recipe is added for `tests/tradeoff_probe.rs`; the probe stays a
  hand-run instrument and its module doc says so; `Peer::sync_memory_budget`'s
  rustdoc stops quoting a measurement nothing enforces; the justfile's
  totality claim is restated to admit hand-run probes.
- T20: the `doc_cfg` configuration is corrected, and a nightly
  `--cfg docsrs` rustdoc leg is added to both `gate` and `ci`.
- T26: the entries it names (here `deps-1`, `tests-wire-format-25`,
  `verification-infra-7`, `-8`, `-9`) land exactly as their Resolution
  states and are judged by their Acceptance; a lane agent that must
  deviate stops.
- T28: the P1 roster (here `deps-7`, `deps-8`, `deps-9`,
  `verification-infra-10`, `-12`, `-13`) lands per each entry's stated
  Resolution and is reviewed as the lane's diff; `streaming-tests-28` is
  excluded from this ruling and awaits its own.
- T29: `lean_wedge_literal()` stays a hand transcription of `Mux.wedge`
  with no gate leg comparing them; its rustdoc states the human-check
  discipline and why it suffices.
- T30: `tools/testdoc` recognizes `#[pollster::test]`, and a committed
  probe demonstrates that an undocumented pollster test fails the gate.

## Stack position

- Base: `d631cda6` (main after the memwatch merge; the lane rebased there with no conflicts)
- Parent: `main`
- Children: `p1-envelope` (shares `src/lib.rs` and the justfile with this
  lane; it branches from this lane's branch and is rebased onto `main`
  the day this lane merges)

## Acceptance table

Every row is one the coordinator's verification runner ran against the
lane at `5378ede9` (first pass) or a detached scratch worktree at
`b06000df` (final pass, cargo on the illumos box); an entry whose
acceptance does not hold is not in this table.

| entry | commit | acceptance command | decisive output |
|---|---|---|---|
| `tests-wire-format-26` | `dec6bd83` | `just future-size` (box) | `Starting 4 tests across 1 binary`; `4 tests run: 4 passed` |
| (negative control, adequacy) | | `Reconciliation::reconcile` unboxed, `just future-size` | `gossip future is 3936 bytes, exceeds budget 3072`; `retire future is 4976 bytes, exceeds budget 3072`; `2 failed` |
| (negative control, liveness) | | `#![cfg(any())]` atop `tests/future_size.rs`, `just future-size` | `Starting 0 tests`; `error: no tests to run`; recipe exit 4 |
| `tests-wire-format-25` | `dec6bd83` | `grep -n -E 'Levels\|Below<\|mirror\(\)' tests/future_size.rs src/tree/traverse.rs` | empty; `Reconciliation::reconcile` named 6 times in the test file |
| `verification-infra-2` | `31882a53` | `grep -E '^all:\|^ci:' justfile` | `all: ci coverage-kernel coverage-kernel-branch (fuzz ...) ...`; header lines 10-18 restated |
| `verification-infra-14`, `tests-resource-link-window-18` | `c727dac2` | `grep -n tradeoff_probe src/peer.rs` | empty |
| `deps-3` | `05c365a3` | `just docs-docsrs` (box) | `RUSTDOCFLAGS="--cfg docsrs -D warnings" cargo +nightly-2026-06-30 doc -p rumors ...`; `Documenting rumors`; `Finished`, no warnings |
| `deps-7`, `verification-infra-12`, `verification-infra-8` | `c35943ca`, `5378ede9`, repair pending | `grep -E 'cargo \+nightly\|toolchain: (nightly\|stable)' .github/workflows/ci.yml`; `just --evaluate nightly_toolchain` | empty; `nightly-2026-06-30` (three jobs derive it); cargo-mutants install restored by the repair round |
| `verification-infra-7` | `31882a53` | `tracing = "0.1"` planted in `crates/rumors-tracing/Cargo.toml`, `./tools/manifestlint` | `[dependencies] tracing declares '0.1' instead of inheriting the workspace entry`; exit 1 |
| `verification-infra-9` | `ef349d3d` | undocumented test planted at `.claude/worktrees/agent-probe/src/probe.rs`, `just testdoc` | exit 0 (the tool pointed at the file directly reports it: exit 1); `./tools/testdoc --self-test` exit 0 |
| `verification-infra-10`, `tests-observation-37` | `ef349d3d` | doc stripped above `tests/changes.rs:18`'s `#[pollster::test]`, `just testdoc` | `tests/changes.rs:16: test \`first_poll_yields_immediately\` is missing a \`///\` doc comment`; exit 1 |
| `deps-1` | `a329a0e3` | `pub struct Z;` given a `u8` field, `cargo check -p rumors --lib --locked` (box) | `error[E0080]: evaluation panicked: assertion failed: size_of::<Z>() == 0 && align_of::<Z>() == 1` |
| `deps-9` | `95143e76` | read in the diff | tokio's default features not re-enabled |
| `verification-infra-13` | `c985bc48` | `just --list --unsorted \| sed -n '2,$p' \| grep -v '\.$'` | empty |
| `streaming-tests-28` | `02083a72` | read in the diff (doc only) | the wedge literal's doc states the manual discipline |
| `deps-8` | stop | `grep -n '^bytes' Cargo.toml` | line unchanged from base |
| all | `5378ede9` | `just gate` per commit (lane logs `gate-N.log`, Mac, before the box rule); `just ci` on the final tree | clean; `CI_EXIT=0` |
| post-rebase tip | `b06000df` | `git grep -i -E 'memwatch\|SWAP_LIMIT_GB\|PROC_LIMIT_GB' -- . ':!.agent-notes'`; the `future-size` recipe | empty; runs bare with `binary_id(rumors::future_size)` |
| `deps-8` (T147) | `b06000df` | `grep -B1 '^bytes' Cargo.toml`; `cargo check -p rumors --lib --locked` and `--all-targets --locked` (box) | plain dependency at line 123, dev-dependency with `serde` at 150 under its comment; both checks `Finished` |
| `tests-wire-format-26` | `b06000df` | `pset-run -n 40 -- just future-size` (box) | `Starting 4 tests across 1 binary`; `4 passed` |
| `verification-infra-13`, `-2` | `b06000df` | `just --list --unsorted` sentence check; `grep -E '^all:\|^ci:' justfile` | empty; `ci` carries `manifestlint`, `docs-docsrs`, `future-size`; `all` carries both coverage legs |
| `deps-7`, `verification-infra-12`, `-8` | `b06000df` | `grep -E 'cargo-mutants@\|just --evaluate nightly_toolchain\|sed .*rust-toolchain' ci.yml` | `cargo-mutants@27.1.0` installed; in each of the three jobs the stable pin is read by `sed`, then the nightly by `just --evaluate` |
| all | `b06000df` | `just doclint`; `just testdoc` (Mac) | both exit 0 |
| all | `3627da08` (`b06000df` moves one manifest feature only, checked above) | `just gate` on the box after the rebase (lane log `box-gate.log`) | eight streams: `audit`, `docsrs`, `internal-docs`, `doctest`, `surface`, `board`, `workspace`, `wasm` ok; `fuzz` the accepted illumos leg |

## Fresh-eyes rounds

**Round 1** (surface correctness with operational validity), against
`5378ede9`, by sha, with the testdoc grammar exercised in python over
an archived copy of the tree. Three medium findings, all repaired in
`b1a12f97`:

- `just --evaluate` evaluates the justfile's backtick assignments, so
  the CI pins step ran `rustc` before any toolchain step and passed only
  by rustup's auto-install; the stable toolchain now installs first
  (its pin read from `rust-toolchain.toml`), then `just` derives the
  nightly. The round's proposed control (`just --evaluate
  nightly_toolchain` with no rustc on the path) does not fire under
  `just` 1.58's lazy evaluation; a bare `just --evaluate` does, and the
  commit records that reading.
- the `ci` roster still runs `mutants-list` while the cargo-mutants
  install had been dropped, so the workflow could not pass at HEAD; the
  install is restored, pinned at the version the expectation file names,
  for the `before` mutants lane to delete together with the leg.
- `gossip_when_stream_fits_budget` had no negative control: measured
  with the unfold's `Box::pin` removed, the stream is 1080 bytes, under
  the budget, so that box is not a budget-protected boundary; the doc
  and assert message now say the boundary the budget protects is
  `Reconciliation::reconcile` and the unfold's box exists for `Unpin`.

Five low findings repaired in the same commit: ragged comment lines and
em-dashes the prose pass left, testdoc's recipe comment restated as
what the mechanism holds (the five roots doclint walks, minus the
ignored names), unmeasured cost readings in the tradeoff probe's doc
replaced by the class, one verbless first sentence, and one wrong
"inert on stable builds".

**Round 2** (operational validity and tree interaction), against
`b1a12f97`: the round traced every CI job's
step order against dtolnay's action at its pinned sha (explicit
toolchain input, no toolchain-file read; nightly ends as rustup's
default, harmless under the checkout's toolchain file), walked the `ci`
roster recipe by recipe against the install step (every tool present),
confirmed the future-size liveness leg sits in both the gate stream and
`ci`, and confirmed testdoc's five roots cover every tracked Rust file
with no ignored name shadowing a tracked path. Three small repairs
followed as the lane's last commit: the justfile header's claim that
the tradeoff probe is the sole hand-run ignored test (two `before`
tests are the same class); the repaired `gossip_when` doc crediting the
reconcile boundary for a number only the unfold's own box produces; and
the future-size leg's name-scoped nextest filter made package-scoped
(`binary_id(rumors::future_size)`). One finding outside the roster,
recorded in `triage/new-findings.md`: doclint's walk has no ignore set
and can read untracked build trees under `crates/`. The rounds stopped
here.

Reviewer notes not acted on: the "twelve scopes" count in the wedge
literal's doc stays as T29 ruled it; recipe names cited from source
prose (`docs-docsrs`, `future-size`) are taste; `Cargo.lock`'s one-line
drop is the consequence of removing `static_assertions`; the docs.rs
leg sets `--cfg docsrs` for rustdoc only, harmless until a dependency
gains a `docsrs` attribute.

## Stops

1. **Ruled: T147.** The `bytes` serde feature moves to a
   dev-dependency (the suites use `Bytes` as a payload type; the library
   never serializes one). Landed in `b06000df`.

Interaction item, not a stop: the workflow's `mutants-list` leg and its
install are deleted together by the `before` mutants lane, which stacks
on this branch. `future-size`'s memwatch prefix went in `3627da08` at
the rebase onto the memwatch merge.

## Reading order

### deviations from a stated resolution

- verification-infra-8 (T26) at `.github/workflows/ci.yml:54` ([hunk](#hunk-3))

### new tests and negative controls

- fresh-eyes repair (T28) at `.github/workflows/ci.yml:76` ([hunk](#hunk-3))
- tests-wire-format-26 (T7) at `justfile:309` ([hunk](#hunk-15))
- verification-infra-7 (T26) at `justfile:1030` ([hunk](#hunk-24))
- deps-1 (T26) at `src/tree/typed/height.rs:168` ([hunk](#hunk-30))
- tests-wire-format-26 (T7) at `tests/future_size.rs:15` ([hunk](#hunk-32))
- fresh-eyes repair (T7) at `tests/future_size.rs:97` ([hunk](#hunk-35))
- verification-infra-9 (T26) at `tools/testdoc:30` ([hunk](#hunk-38))
- tests-observation-37 (T30) at `tools/testdoc:32` ([hunk](#hunk-38))
- verification-infra-10 (T28) at `tools/testdoc:150` ([hunk](#hunk-41))

### production edits

- deps-1 (T26) at `src/lib.rs:295` ([hunk](#hunk-26))
- deps-3 (T20) at `src/lib.rs:300` ([hunk](#hunk-26))
- T141 prose pass (T141) at `src/lib.rs:296` ([hunk](#hunk-26))
- fresh-eyes repair (T20) at `src/lib.rs:298` ([hunk](#hunk-26))
- tests-resource-link-window-18 (T16) at `src/peer.rs:414` ([hunk](#hunk-27))
- tests-wire-format-25 (T7) at `src/tree/traverse.rs:9` ([hunk](#hunk-29))

### tests and prose

- deps-7 (T28) at `.github/workflows/ci.yml:18` ([hunk](#hunk-2))
- deps-7 (T28) at `.github/workflows/ci.yml:76` ([hunk](#hunk-3))
- verification-infra-12 (T28) at `.github/workflows/ci.yml:70` ([hunk](#hunk-3))
- verification-infra-8 (T26) at `.github/workflows/ci.yml:54` ([hunk](#hunk-3))
- T141 prose pass (T141) at `.github/workflows/ci.yml:54` ([hunk](#hunk-3))
- deps-7 (T28) at `.github/workflows/ci.yml:95` ([hunk](#hunk-3))
- fresh-eyes repair (T26) at `.github/workflows/ci.yml:68` ([hunk](#hunk-3))
- verification-infra-12 (T28) at `.github/workflows/ci.yml:140` ([hunk](#hunk-4))
- verification-infra-12 (T28) at `.github/workflows/ci.yml:145` ([hunk](#hunk-4))
- verification-infra-12 (T28) at `.github/workflows/ci.yml:235` ([hunk](#hunk-5))
- deps-1 (T26) at `Cargo.lock:0` ([hunk](#hunk-6))
- deps-3 (T20) at `Cargo.toml:88` ([hunk](#hunk-7))
- deps-1 (T26) at `Cargo.toml:125` ([hunk](#hunk-8))
- deps-8 (T147) at `Cargo.toml:123` ([hunk](#hunk-8))
- deps-9 (T28) at `Cargo.toml:151` ([hunk](#hunk-9))
- verification-infra-14 (T16) at `justfile:3` ([hunk](#hunk-10))
- fresh-eyes repair (T16) at `justfile:3` ([hunk](#hunk-10))
- verification-infra-2 (T15) at `justfile:14` ([hunk](#hunk-11))
- fresh-eyes repair (T141) at `justfile:18` ([hunk](#hunk-11))
- verification-infra-13 (T28) at `justfile:111` ([hunk](#hunk-12))
- verification-infra-9 (T26) at `justfile:190` ([hunk](#hunk-13))
- fresh-eyes repair (T26) at `justfile:182` ([hunk](#hunk-13))
- fresh-eyes repair (T26) at `justfile:183` ([hunk](#hunk-13))
- deps-3 (T20) at `justfile:283` ([hunk](#hunk-14))
- T141 prose pass (T141) at `justfile:276` ([hunk](#hunk-14))
- verification-infra-13 (T28) at `justfile:312` ([hunk](#hunk-15))
- fresh-eyes repair (T7) at `justfile:310` ([hunk](#hunk-15))
- verification-infra-13 (T28) at `justfile:344` ([hunk](#hunk-16))
- verification-infra-13 (T28) at `justfile:388` ([hunk](#hunk-17))
- deps-3 (T20) at `justfile:413` ([hunk](#hunk-18))
- deps-3 (T20) at `justfile:493` ([hunk](#hunk-19))
- verification-infra-13 (T28) at `justfile:615` ([hunk](#hunk-20))
- verification-infra-13 (T28) at `justfile:663` ([hunk](#hunk-21))
- verification-infra-13 (T28) at `justfile:735` ([hunk](#hunk-22))
- verification-infra-13 (T28) at `justfile:834` ([hunk](#hunk-23))
- deps-3 (T20) at `justfile:1030` ([hunk](#hunk-24))
- verification-infra-2 (T15) at `justfile:1033` ([hunk](#hunk-24))
- verification-infra-2 (T15) at `justfile:1035` ([hunk](#hunk-24))
- verification-infra-2 (T15) at `justfile:1047` ([hunk](#hunk-25))
- streaming-tests-28 (T29) at `src/tree/mirror/streaming/tests/wedge.rs:41` ([hunk](#hunk-28))
- T141 prose pass (T141) at `src/tree/mirror/streaming/tests/wedge.rs:41` ([hunk](#hunk-28))
- fresh-eyes repair (T29) at `src/tree/mirror/streaming/tests/wedge.rs:38` ([hunk](#hunk-28))
- deps-1 (T26) at `src/tree/typed/height/tests.rs:0` ([hunk](#hunk-31))
- tests-wire-format-26 (T7) at `tests/future_size.rs:3` ([hunk](#hunk-32))
- tests-wire-format-25 (T7) at `tests/future_size.rs:26` ([hunk](#hunk-32))
- T141 prose pass (T141) at `tests/future_size.rs:15` ([hunk](#hunk-32))
- tests-wire-format-25 (T7) at `tests/future_size.rs:52` ([hunk](#hunk-33))
- tests-wire-format-25 (T7) at `tests/future_size.rs:72` ([hunk](#hunk-34))
- fresh-eyes repair (T7) at `tests/future_size.rs:95` ([hunk](#hunk-35))
- tests-resource-link-window-18 (T16) at `tests/tradeoff_probe.rs:8` ([hunk](#hunk-36))
- T141 prose pass (T141) at `tests/tradeoff_probe.rs:8` ([hunk](#hunk-36))
- fresh-eyes repair (T16) at `tests/tradeoff_probe.rs:9` ([hunk](#hunk-36))
- verification-infra-14 (T16) at `tests/tradeoff_probe.rs:191` ([hunk](#hunk-37))
- verification-infra-10 (T28) at `tools/testdoc:37` ([hunk](#hunk-38))
- T141 prose pass (T141) at `tools/testdoc:9` ([hunk](#hunk-38))
- verification-infra-10 (T28) at `tools/testdoc:53` ([hunk](#hunk-39))
- verification-infra-10 (T28) at `tools/testdoc:87` ([hunk](#hunk-40))
- verification-infra-10 (T28) at `tools/testdoc:100` ([hunk](#hunk-40))
- verification-infra-9 (T26) at `tools/testdoc:227` ([hunk](#hunk-42))

## The change

<a id="hunk-1"></a>
### .agent-notes/2026-09-01-holistic-review-rumors/triage/annotations/p1-gate.tsv `@@ -0,0 +1,76 @@`

```diff
@@ -0,0 +1,76 @@
+# Lane p1-gate annotations: path <TAB> line <TAB> entry id <TAB> ruling <TAB> note. Lines are in the lane's final tree.
+tools/testdoc	30	verification-infra-9	T26	The entry claimed the gate's testdoc verdict depended on other agents' worktrees under `.claude/`, because the recipe walked `.` and the tool skipped only `.git`, `node_modules`, and `target`. I added `.claude` to the ignore set as the second guard the resolution asks for; the first guard is the recipe naming explicit roots (see the justfile row). Negative control: an undocumented test placed under `.claude/worktrees/agent-probe/src/probe.rs` is reported by the HEAD tool walking `.` (exit 1) and by neither the new recipe nor the new tool walking `.` (exit 0); the transcript is in the commit message.
+tools/testdoc	32	tests-observation-37	T30	The entry (with its duplicates remote-proxy-tests-27 and tests-disruption-handshake-33) claimed `#[pollster::test]` never matched the attribute regex, so the 29 pollster tests were outside doc enforcement. I took the general form the duplicates propose: any attribute path ending in `::test`, beside `rstest` and `test_case` exactly as before. I considered also allowing a path prefix on `rstest`/`test_case` (`#[rstest::rstest]`) and rejected it as wider than the brief's stated form; nothing in the tree uses it. After widening, `./tools/testdoc benches crates examples src tests` on the tree reports nothing: every pollster test is documented today, so no documentation change rides with this commit. Negative control: stripping the doc above `tests/changes.rs:18` and running `just testdoc` fails naming `first_poll_yields_immediately` (restored; diff empty).
+tools/testdoc	37	verification-infra-10	T28	The entry claimed a `proptest! { fn .. }` block function without an explicit `#[test]` was invisible to the tool. I track the block form lexically from its `proptest! {` opener by brace depth and treat every `fn` declared at depth 1 as a test entry point. The closure form `proptest!(|..| { .. })` is not an opener: it sits inside a function body and declares no test.
+tools/testdoc	87	verification-infra-10	T28	Brace counting strips char literals, string literals, and `//` comments first, in that order, so a `"{"` in a test body cannot leave the tracker inside the block forever. This is still lexical (a raw string or a block comment holding a brace would count); the self-test pins the forms the tree uses. I judged that the right depth for a tool whose header promises no parser.
+tools/testdoc	100	verification-infra-10	T28	A block function carrying an explicit `#[test]` is found by both the attribute scan and the block scan; `reported` (keyed by the function's line) makes it one finding, not two. The attribute scan keys by the function line it names so the two scans agree on the identity of a test.
+tools/testdoc	150	verification-infra-10	T28	Self-test cases added for `#[pollster::test]` (documented and not), `#[async_std::test]`, `#[rstest]`, the `proptest!` block form with and without `#[test]`, the single-finding dedupe, a multi-line `#![proptest_config(..)]` inner attribute, braces inside literals and comments, a helper `fn` nested in a block function's body (not a test), and the closure form (no test). Negative control for the block form: the entry's scratch file (an undocumented `proptest!` function plus an undocumented `#[test]`) yields two findings; transcript in the commit message.
+tools/testdoc	227	verification-infra-9	T26	The self-test case for the ignore list the resolution asks for: a temporary tree where `.claude/worktrees/..`, `target/`, and nested `.git`/`node_modules`/`target` directories under a scanned root are pruned, files under `src/` are kept, and a file named explicitly is scanned regardless of its directory (the recipe passes directories, but the behavior is pinned so a future caller naming a file inside an ignored tree is not silently skipped).
+justfile	190	verification-infra-9	T26	The recipe names the same explicit roots doclint does instead of `.`; the comment above states why (untracked trees at the repository root). I verified with `git ls-files '*.rs'` that the five roots cover every tracked Rust file.
+src/tree/typed/height.rs	168	deps-1	T26	The entry claimed `static_assertions` was a normal dependency serving one test module, and that one redundant test there (`zero_size_val`, whose value/type distinction does not exist for `Sized` types) was the only reason `forbid(unsafe_code)` was conditional. Of the two homes the resolution offers I chose `height.rs` beside the existing `const _` assert: these are compile-time facts of the lib, they fire on every build without a `#[test]` wrapper, and the alternative would have kept a test file holding nothing but them. `tests.rs` and its `mod tests;` line are gone with it. `size_of`/`align_of` come from the prelude. Negative control (reversible mutation, restored): `pub struct Z;` made `pub struct Z(u8);` fails `cargo check -p rumors --lib` with `error[E0080]: evaluation panicked: assertion failed: size_of::<Z>() == 0 && align_of::<Z>() == 1` at this site; the transcript is in the commit message.
+src/lib.rs	295	deps-1	T26	`#![forbid(unsafe_code)]` is unconditional and the comment explaining the old `cfg_attr(not(test), ..)` is deleted, as the resolution states: with the `assert_eq_size_val!` call gone, nothing in the crate, tests included, expands to `unsafe`.
+Cargo.toml	125	deps-1	T26	The `static_assertions` entry is deleted from rumors' `[dependencies]` (annotated at the line that follows it); the workspace table's entry stays because `crates/before` inherits it. `Cargo.lock` loses the one line naming it under the rumors package.
+Cargo.toml	151	deps-9	T28	The nit claimed `default-features = true` on the tokio dev-dependency is vacuous; it is: tokio 1.x declares no default features, so widening the workspace table's `default-features = false` enables nothing. Deleted; `cargo check -p rumors --all-targets` is unchanged and `Cargo.lock` loses nothing for it.
+Cargo.toml	123	deps-8	T147	Landed on ruling T147, which closes the stop this row recorded: the library never serializes a `Bytes`, so the root dependency loses its `serde` feature, and a `[dev-dependencies]` entry carries the feature for the suites, which use `Bytes` as a payload type (the one-line comment at the entry says so). Acceptance on the box: `cargo check -p rumors --lib --locked` and `cargo check -p rumors --all-targets --locked` both pass; `grep -n '^bytes' Cargo.toml` shows the plain dependency and the featured dev entry. `Cargo.lock` is unchanged (features are not recorded in it).
+src/lib.rs	300	deps-3	T20	The entry claimed `#![cfg_attr(docsrs, feature(doc_auto_cfg))]` fails the docs.rs build because the nightly removed `doc_auto_cfg` (merged into `doc_cfg` at 1.92). I reproduced the failure on the unfixed tree myself before changing anything: `RUSTDOCFLAGS="--cfg docsrs" cargo +nightly-2026-06-30 doc -p rumors --all-features --no-deps` gives `error[E0557]: feature has been removed ... merged into doc_cfg` (verbatim in the commit message). The attribute now names `doc_cfg`; the comment states what the cfg does and points at the leg that builds it. Verified after the fix: the rendered `conformance` page under `--cfg docsrs` carries the portability label "Available on crate features conformance only.", and the stable `docs` build carries none, so the labeling is automatic under `doc_cfg` as RFC 3631 states.
+Cargo.toml	88	deps-3	T20	The docs.rs metadata comment named `doc_auto_cfg`; it now names `doc_cfg` and the `docs-docsrs` leg that builds this configuration.
+justfile	283	deps-3	T20	The new leg: rumors' rustdoc under the pinned nightly with `--cfg docsrs -D warnings`, `-p rumors --no-deps`, in `target/doc-docsrs` so nightly artifacts never invalidate the stable doc passes. I omitted the `--html-in-header` fuelscape flag the stable `docs` recipes carry: it injects `before`'s widget assets, and with `--no-deps` no `before` page is rendered here. The leg passes on this tree in about two seconds after the deps are cached.
+justfile	493	deps-3	T20	Ruling T20 puts the leg in `gate` as well as `ci`. It is its own gate stream: it writes a target dir nothing else touches, which is the property the stream grouping comment names as what lets streams overlap; the census of such directories in that comment gains the docs.rs target.
+justfile	1030	deps-3	T20	The leg joins the `ci` roster beside the other doc passes, cheap-first order kept.
+.github/workflows/ci.yml	18	deps-7	T28	The entry claimed the workflow installed a floating `nightly` (unused: every nightly recipe names the dated `nightly_toolchain`) and a floating `stable` (overridden by `rust-toolchain.toml`), so the toolchain provenance CI's comments described was not what ran. The header's inventory now describes the two pinned toolchains and where each pin lives.
+.github/workflows/ci.yml	76	deps-7	T28	Each pin has exactly one site in the tree (stable's `channel` in rust-toolchain.toml, nightly in the justfile) and this step reads both with `sed`, failing loudly if either read is empty, so the workflow cannot diverge from a pin. I chose reading over naming the pins a second time because the entry asks for exactly that ("ideally read from the justfile"), and I verified the two `sed` expressions print `1.97.1` and `nightly-2026-06-30` on this tree. The dtolnay action at the pinned SHA requires an explicit `toolchain:` input and reads no toolchain file (I fetched its action.yml to check), which is why the stable install keeps an input rather than being dropped.
+.github/workflows/ci.yml	70	verification-infra-12	T28	The install comments now state today's mechanism: the action does run `rustup default` (confirmed in its action.yml), so the ordering note stays, restated to say what the default governs; the claim that recipes invoke `cargo +nightly` is gone. The same step pair, with the same pins step, appears in all three jobs; I kept it duplicated rather than hoisting it into a composite action, which would be a larger change than the entry names.
+.github/workflows/ci.yml	54	verification-infra-8	T26	The entry claimed cargo-mutants rode the install action's floating manifest while `tools/mutantcheck-expected.json` pins `cargo-mutants 27.1.0` and refuses any other version. The install line pins `cargo-mutants@27.1.0` beside `cargo-rdme@2.1.0`, and the comment is rewritten as the resolution states: two tools carry versions because two committed artifacts depend on them, and a bump re-pins the artifact in the same reviewed diff. The local tool is 27.1.0 too (`cargo mutants --version`), which the gate's `mutants-list` leg already holds.
+.github/workflows/ci.yml	140	verification-infra-12	T28	The "lib-as-bench compile known to exceed 16 GiB" claim named a compile `[lib] bench = false` removed; the runner-fit note now says only where rumors' release-profile bench builds live. The coverage job's "known-heavy bench/release tier" (line 227) carried the same expired figure and is swept to the same statement.
+.github/workflows/ci.yml	145	verification-infra-12	T28	"This job tracks nightly, so a format bump upstream can turn the leg red on an untouched tree" cannot happen under a dated pin; the coupling note now says the format moves only with a deliberate pin bump, and keeps the checker's loud refusal and the re-pin pointer.
+.github/workflows/ci.yml	235	verification-infra-12	T28	The coverage job installed `llvm-tools` on a floating nightly the branch leg never used; both pinned toolchains now carry the component, per the resolution's first option, so cargo-llvm-cov finds it on whichever toolchain it drives.
+justfile	1030	verification-infra-7	T26	The entry claimed `ci` omitted `manifestlint`, which `gate-lints` runs, so a member-local version restatement passed CI. Added to the `ci` roster right after `workflowlint`, where the gate runs it. Negative control (reversible, restored): `tracing = { workspace = true }` in crates/rumors-tracing/Cargo.toml made `tracing = "0.1"` fails `./tools/manifestlint` ("[dependencies] tracing declares '0.1' instead of inheriting the workspace entry"), and `just --dry-run ci` lists `./tools/manifestlint` in the roster; the full `just ci` runs once at the end of the lane, not per entry, per the brief's resource discipline.
+justfile	14	verification-infra-2	T15	Ruling T15: the header describes `ci` as the recipe CI's `ci` job runs, not as a mirror of every check, and says what `all` actually covers. These are the named lines 12-14 restated; the rest of the header is untouched (the brief says not to reflow it).
+justfile	1033	verification-infra-2	T15	The coverage legs join `all` (ruling T15's first option; no `ci-full`), placed right after `ci` so the deterministic sweep legs run before the fuzz smoke and the wall-time bench judge. Acceptance is the end-of-lane `just all`: if it fails on the `before` meter under instrumentation at this base, that is the expected outcome and is reported, not touched.
+justfile	1035	verification-infra-2	T15	The section heading and the "CI legs, never gate legs" sentence now name `all` as the second cadence; the design paragraph is otherwise unchanged.
+tests/tradeoff_probe.rs	8	tests-resource-link-window-18	T16	The entry claimed the `#[ignore]` gate's cost rationale lived only in history and that public rustdoc quoted a measurement nothing schedules. Ruling T16: no recipe; the file stays; its module doc states that it is a hand-run instrument and why. I state the cost by mechanism (design-corpus sessions per cell; seconds in release, several times that under the gate's dev profile) rather than the entry's eleven-second figure, which I did not measure and which would rot as a hand-maintained number.
+tests/tradeoff_probe.rs	191	verification-infra-14	T16	The ignore message names the instrument's class and cost class beside the invocation hint, so a reader at the attribute sees the rationale without opening the module doc.
+src/peer.rs	414	tests-resource-link-window-18	T16	The `Measured: .. ran 1.35-1.96x the form's figure (tests/tradeoff_probe.rs)` sentence is excised (annotated at the blank doc line that now follows the accuracy-band paragraph): it quoted a run no recipe performs. Excised rather than restated because with the figure and the citation removed nothing of the sentence remained that the accuracy-band paragraph above it does not already say. Nothing else in this public rustdoc moved; the sizing guide's relocation is owner decision 80 and out of scope.
+justfile	3	verification-infra-14	T16	The opening totality claim now admits its one exception class, the hand-run instrument, defined by what makes it one (an `#[ignore]`-gated test whose module doc states the cost and the command) and naming the sole member. A sentence appended; the header is not reflowed.
+justfile	111	verification-infra-13	T28	The two-line explanatory comment abutted the recipe, so `just --list` showed its second line ("module build only here)."). The explanation stays as a block ending in a blank line and a one-line summary sits directly above the recipe, the pattern the other recipes follow.
+justfile	312	verification-infra-13	T28	The summary was one sentence wrapped over two comment lines, so the listing showed its tail. Rewritten as one line (shorter, same content).
+justfile	344	verification-infra-13	T28	The recipe had a summary line, then a `#` blank and the `--colors=never` paragraph directly above the recipe, so the listing showed that paragraph's last line. The paragraph moved into the explanatory block above; the summary now abuts the recipe.
+justfile	388	verification-infra-13	T28	The fmt-line explanation moved into the block above; the summary line above the attribute mentions the fmt check in a clause so the listing still says what the recipe does beyond building.
+justfile	615	verification-infra-13	T28	The lint-leg explanation moved into the block above (it already described the suites); the summary is the line that was already its first sentence.
+justfile	663	verification-infra-13	T28	The cost and lint-leg explanation became a block above the recipe; the summary keeps the cost class in a parenthetical because that is what a reader choosing a recipe from the tour needs to know.
+justfile	735	verification-infra-13	T28	A blank line and a summary line separate the explanation from the recipe.
+justfile	834	verification-infra-13	T28	A blank line and a summary line separate the long protocol comment from the recipe; the listing showed "(never quoted)." before.
+tests/future_size.rs	3	tests-wire-format-26	T7	The entry claimed the binary was `#![cfg(not(debug_assertions))]` and so had never run under any committed check. The cfg is lifted (T7, amended: no release leg). Measured once in each profile with the budget set to zero so every test reported its size: dev and release agree exactly (stream 8, bootstrap 336, gossip 960, retire 2000 bytes), which is the profile-independence the resolution's premise states. The first dev run did not fail the existing 2048 budget, but retire sat 48 bytes under it, so the budget is re-pinned with headroom as the resolution asks, and this note says so: 3072, half again above the largest, and below the roughly 3.9 KiB that unboxing `Reconciliation::reconcile` alone produces, so the stated demonstration trips on every session future. `gossip_when`'s stream joins the measured surface (8 bytes: its unfold box).
+tests/future_size.rs	15	tests-wire-format-26	T7	Negative controls (reversible mutations in src/peer/gossip.rs and src/tree/mirror/streaming.rs, restored, diff empty; transcripts in the commit message): (1) `Reconciliation::reconcile` returning `impl Future` instead of `Box::pin`: gossip 3936 and retire 4976 bytes, both over 3072, bootstrap and the stream unmoved; (2) every box down to the descent removed (`Handshaken::reconcile` and both `Box::pin(handshaken.reconcile())` sites too): gossip 95,256 and retire 96,296 bytes. So the old doc's "jumps to tens of KiB" was true of the inner boundary, not of the outer one the entry names; the doc now states both measured outcomes instead. Finding for the owner: the outer box alone hides the handshake state (about 3 KiB), and the descent's erasure is `Handshaken::reconcile`'s box; `bootstrap_reconcile` is bootstrap's own boundary (its size never moved under either mutation).
+tests/future_size.rs	26	tests-wire-format-25	T7	The mechanism is restated against today's code: the typed phase schedule under `streaming::protocol` is the deep type; `Reconciliation::reconcile`'s boxed `inline(never)` future (with `Handshaken::reconcile` below it), `bootstrap_reconcile`, and `gossip_when`'s boxed unfold are the boundaries; the assert messages name `Reconciliation::reconcile` (and `bootstrap_reconcile` for the bootstrap test, since that is the box a maintainer would have removed there). The `mirror()` and `Levels` references are gone.
+src/tree/traverse.rs	9	tests-wire-format-25	T7	The comment cited the `Levels` docs, a type deleted with `typed::levels`; it now gives the reason without naming a deleted item (rustdoc elsewhere in the crate links to the traversal traits).
+justfile	309	tests-wire-format-26	T7	The liveness guard the amendment lands: this recipe reruns the future_size binary from the same build set as test-all (workspace, all features, so no rebuild) through nextest's `-E 'binary(future_size)'` filter with `--no-tests=fail`. It sits after test-all in the workspace stream and in the `ci` roster. I chose the filter over `-p rumors --test future_size` because the latter is a different feature set and would recompile rumors for one four-test binary; and over adding `--no-tests=fail` to test-all because nextest already fails an entirely empty run, so that flag on the workspace run guards nothing per-binary. Negative control (reversible, restored): `#![cfg(any())]` at the top of the test file makes `just future-size` report `Starting 0 tests across 1 binary`, `error: no tests to run`, and fail the recipe with exit 4.
+src/tree/mirror/streaming/tests/wedge.rs	41	streaming-tests-28	T29	Ruling T29 (model): the literal stays a hand transcription and no gate leg compares it to `Instances.lean`; the doc states the manual discipline (human-checked whenever either side changes) and the three reasons the ruling gives for why that suffices, naming the two pins by their test names. No code change. The "twelve scopes" count is the ruling's own argument for the discipline, so it stays as the ruling states it.
+.github/workflows/ci.yml	54	T141 prose pass	T141	Prose pass over the lane's diff. The pins comment loses its restatement of what the install steps do and the motivating sentence about floating channels (the reader holds the step; what it needs is where each pin lives and why the stable step still takes an input). The tools comment says the pin rule once. The instruments and coverage pin comments drop the spelled-out `cargo +<toolchain>` forms, which the recipes already carry. Net shorter.
+justfile	276	T141 prose pass	T141	The docs-docsrs and future-size recipe comments are cut to the mechanism a recipe reader needs (what the leg builds, why nothing else covers it); the sentences about publication-time surfacing and about the failure mode being invisible were motivation that changes nothing the reader does. The header's exception sentence and the testdoc recipe comment are shortened the same way. Lines this lane left ragged in the header, the stream census, and the coverage section are reflowed; nothing else in those paragraphs moves. The recipe comments this lane added are the only prose added net of deletions; each buys the reader the leg's reason for existing, which no other line states.
+tests/future_size.rs	15	T141 prose pass	T141	The module doc keeps the mechanism and the boundaries and drops the measured readings (the quadrupling, the 95 KiB) and the path citation: readings a code change can falsify with the prose untouched belong in the commit message, where they now live alone. It also corrects a claim my earlier wording made: removing an inner box alone is hidden by the outer one, so the sentence names the outer boxes. The budget constant's doc keeps one reading, the largest measured size, because ruling T7's resolution asks for the measured band beside the budget; that is the one added sentence this pass leaves that a reviewer must weigh.
+src/tree/mirror/streaming/tests/wedge.rs	41	T141 prose pass	T141	The two test names leave the doc (a rename would falsify them with the prose untouched); the pins are named by role and are in this file. Shorter by three lines with the ruling's three reasons intact.
+tests/tradeoff_probe.rs	8	T141 prose pass	T141	The corpus size the sentence repeated is already in the Method list below it; the sentence now states class and cost only.
+tools/testdoc	9	T141 prose pass	T141	The docstring's two paragraphs say each fact once; `#[pollster::test]` as a second example and the clause about roots growing ignored subtrees were restatement.
+src/lib.rs	296	T141 prose pass	T141	Four lines to three: the same three facts (the cfg, the feature it enables, the leg that builds it) without the second clause restating what auto-labeling is.
+.github/workflows/ci.yml	95	deps-7	T28	Cross-triage change adopted here so ci.yml has one owner: the nightly pin is read with `just --evaluate nightly_toolchain` (the justfile stays the one pin of record) instead of a `sed` over the justfile, so each job installs its tools first; taiki-e/install-action ships prebuilt binaries and needs no toolchain. The stable pin is still read from rust-toolchain.toml's `channel` with `sed`, since the justfile carries no stable variable. `just --evaluate nightly_toolchain` prints `nightly-2026-06-30` on this tree.
+.github/workflows/ci.yml	54	verification-infra-8	T26	Superseded before merge: the mutation campaign is being retired workspace-wide by a `before` lane, so the cargo-mutants install (and the 27.1.0 pin this lane had added) leaves the workflow rather than pinning a tool about to be deleted; `.cargo/mutants.toml`, `tools/mutantcheck`, and the `mutants-list` recipe are untouched here, their deletion being that lane's. The comment returns to the one-tool rule for cargo-rdme. The entry's acceptance clause "tools/workflowlint still passes" is moot, that tool being deleted by the same triage; it passes on this tree regardless. Deviation from the entry's stated resolution, on the coordinator's relayed owner instruction. Note for the merge order: until the mutants lane removes `mutants-list` from the `ci` roster, a CI run of this workflow would fail there for lack of the tool.
+.github/workflows/ci.yml	76	fresh-eyes repair	T28	Item 1: each job now installs the tools, reads the stable pin from rust-toolchain.toml with `sed` and installs stable, then reads the nightly with `just --evaluate nightly_toolchain` and installs it, so `just`'s evaluation of the justfile's `rustc` backtick (`host_triple`) can only run after a toolchain is installed; the comments at the steps and the header's tool bullet (cargo-fuzz and cargo-mutants added) say what the workflow does. The round's negative control does not fire as stated on this machine: `env PATH=/usr/bin:/bin just --evaluate nightly_toolchain` prints `nightly-2026-06-30` (exit 0) on just 1.58.0 because it evaluates the one named variable lazily; whole-scope `just --evaluate` under the same PATH fails with `backtick failed with exit code 127` at `host_triple`, so the reorder removes a dependence on `just`'s evaluation order rather than a failure observed today. Stable now precedes nightly, so `rustup default` ends on nightly; harmless, since rust-toolchain.toml governs every bare `cargo` under the checkout and every nightly recipe names its toolchain.
+.github/workflows/ci.yml	68	fresh-eyes repair	T26	Item 2: the cargo-mutants install is restored, pinned at 27.1.0 as tools/mutantcheck-expected.json names, so the `ci` roster's `mutants-list` leg has its tool at every commit on main; the `before` triage's mutants lane deletes the install together with the leg. The install comment returns to the two-tool rule.
+tests/future_size.rs	97	fresh-eyes repair	T7	Item 3 (negative control): with the `Box::pin` around `gossip_when`'s unfold removed (and the `Unpin` bound dropped from both signatures so it compiles), measured on `-p rumors --test future_size`: `gossip_when stream is 1080 bytes`, under the 3072 budget (the other three read 960, 2000, 336 as before). So that box is not a boundary the budget protects; the doc and the assert message now say the stream's session awaits through `Reconciliation::reconcile` and the unfold's box exists for `Unpin`. `src` restored; identical to base.
+justfile	18	fresh-eyes repair	T141	Items 4 and 5: the ragged lines this lane left (header, stream census, sweep header, the `all` paragraph, the coverage section) are reflowed into their paragraphs, and every em-dash on a line this lane added or reflowed is a spaced double hyphen. Untouched lines keep their em-dashes for T48's sweep. This makes the earlier prose-pass row true.
+justfile	182	fresh-eyes repair	T26	Item 6: the testdoc recipe comment states what the mechanism holds (files under the five roots doclint walks, minus the ignored names), not a stronger invariant.
+tests/tradeoff_probe.rs	9	fresh-eyes repair	T16	Item 7: the cost is stated as its class, too slow for the gate and hand-run, with no reading (nobody measured one); the ignore message says the same.
+src/tree/mirror/streaming/tests/wedge.rs	38	fresh-eyes repair	T29	Item 8: the doc's first sentence has a main verb ("Transcribes ...").
+src/lib.rs	298	fresh-eyes repair	T20	Item 8: "inert on stable builds" replaced by what is true: no build but docs.rs sets the cfg, so the attribute is absent everywhere else (and would error on stable if the cfg were set).
+justfile	3	fresh-eyes repair	T16	Round 2, item 1: the header's exception class is stated by what makes a test one (`#[ignore]`-gated, no recipe runs it, its doc carries cost and command) without naming a sole member, since `before` holds two more of the class (`exhaustive_deep`, `clock_text_split_survives_two_gib_of_parens`); no count is claimed.
+justfile	183	fresh-eyes repair	T26	Round 2, item 3: the testdoc recipe comment names testdoc's ignore set, not doclint's (which has none).
+justfile	310	fresh-eyes repair	T7	Round 2, item 3: the liveness filter is `binary_id(rumors::future_size)`, the package-scoped id nextest prints, so a same-named binary in another member cannot keep the leg green while rumors' collects nothing. Verified: `cargo nextest list --workspace --all-features -E 'binary_id(rumors::future_size)'` lists the four tests.
+tests/future_size.rs	95	fresh-eyes repair	T7	Round 2, item 2: the gossip_when doc and message say what the test holds (the public stream is one boxed pointer; it would first observe a stream handed out unboxed with deep state) and that the reconcile boundary is held by the gossip and retire tests, since the stream cannot observe it (my own control: 8 bytes with that boundary in place, 1080 with only the unfold's box removed). The module doc's clause about the stream says only that it is handed out boxed.
+Cargo.lock	0	deps-1	T26	Consequence of removing `static_assertions` from rumors' `[dependencies]`: the lock's entry for the rumors package loses the one line naming it. Nothing else in the lock moves; the crate itself stays resolved for `crates/before`.
+justfile	413	deps-3	T20	The stream-grouping comment's census of directories nothing else touches gains the docs.rs doc target (`target/doc-docsrs`), the property that lets `docs-docsrs` be its own stream; the reflow of the sentence and the em-dash to a spaced double hyphen are the prose and fresh-eyes passes on the same lines.
+justfile	1047	verification-infra-2	T15	The coverage design paragraph's "CI legs, never gate legs" sentence now names both sweep cadences (`all` and CI's `coverage` job), since T15 puts the legs in `all`; the paragraph is reflowed and its em-dash on a line this lane touched is a spaced double hyphen.
+src/tree/typed/height/tests.rs	0	deps-1	T26	Deleted: the module held only the three `static_assertions` layout tests (`zero_size`, the redundant `zero_size_val`, `one_align`), which are now `const _: () = assert!(..)` items beside the endpoint assert in `height.rs`, firing on every build; the `mod tests;` line in `height.rs` goes with it.
+tests/future_size.rs	52	tests-wire-format-25	T7	The gossip test's assert message names the boundary a maintainer would have removed, `Reconciliation::reconcile`'s returned `Pin<Box<dyn Future>>`, instead of an unnamed "internal indirection"; the em-dash is a semicolon.
+tests/future_size.rs	72	tests-wire-format-25	T7	The retire message points at the same boundary by name, and the bootstrap test's doc names its own: `bootstrap_reconcile`'s boxed future, the same discipline as `Reconciliation::reconcile` (under both adequacy mutations bootstrap's size never moved, which is what identified that boundary).
+tools/testdoc	53	verification-infra-10	T28	`has_attached_doc` now takes either an attribute line or the function line itself, because the `proptest!` block scan starts its upward walk at a `fn` that may carry no attribute at all; the walk's rules (skip attributes and ordinary comments, stop at anything else) are unchanged, so the attribute path behaves as before.
```

*(review record; no annotation expected)*

<a id="hunk-2"></a>
### .github/workflows/ci.yml `@@ -15,10 +15,13 @@`

```diff
@@ -15,10 +15,13 @@
 # exactly one definition (the justfile) and the two can't drift.
 #
 # What the runner has to supply that a dev machine already has:
-#   - a current stable toolchain (edition 2024 needs 1.85+), with clippy + rustfmt
-#     and the wasm32-unknown-unknown target (wasm-check, viz),
-#   - a nightly toolchain for merged doctests and cargo-fuzz's target build,
-#   - `just`, `cargo-nextest` (just test), `cargo-rdme` (readme-check), and
+#   - the two pinned toolchains, installed by name from their in-tree sites:
+#     stable per rust-toolchain.toml (with clippy, rustfmt, and the
+#     wasm32-unknown-unknown target for wasm-check and viz) and the dated
+#     nightly per the justfile's `nightly_toolchain` (merged doctests, the
+#     docs.rs rustdoc leg, cargo-fuzz's target build),
+#   - `just`, `cargo-nextest` (just test), `cargo-rdme` (readme-check),
+#     `cargo-fuzz` (fuzz-build), `cargo-mutants` (mutants-list), and
 #     `wasm-pack` (viz),
 #   - node + npm for the viz TypeScript typecheck and bundle,
 #   - python3 for the tools/ linters and bash, the justfile's recipe shell
```

<!-- annotation -->
> **deps-7** (T28), line 18:
>
> The entry claimed the workflow installed a floating `nightly` (unused: every nightly recipe names the dated `nightly_toolchain`) and a floating `stable` (overridden by `rust-toolchain.toml`), so the toolchain provenance CI's comments described was not what ran. The header's inventory now describes the two pinned toolchains and where each pin lives.

<a id="hunk-3"></a>
### .github/workflows/ci.yml `@@ -48,39 +51,55 @@ jobs:`

```diff
@@ -48,39 +51,55 @@ jobs:
     steps:
       - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
 
-      # Install nightly *first* so the stable install below wins the default:
-      # each dtolnay/rust-toolchain step runs `rustup default`, last one sticks.
-      # The doctest and fuzz recipes invoke `cargo +nightly`, so nightly only
-      # needs to exist, not be the default — whereas the gate's bare
-      # `cargo fmt`/`clippy`/`check` must hit stable.
-      #
-      # A SHA ref names no channel: every dtolnay step carries an explicit
-      # `toolchain:` input, and the pin rides the action's v1 tag, which
-      # Dependabot resolves like any other (the channel branches publish
-      # no tags).
-      - name: Install nightly toolchain (merged doctests and fuzz build)
-        uses: dtolnay/rust-toolchain@6c977a6ca4077a0ceb28ffbe03f59d46e9ac8772 # v1
+      # Tools first: they are prebuilt binaries needing no toolchain, and
+      # `just` reads the nightly pin below. Two tools carry versions because
+      # two committed artifacts depend on them: readme-check holds the
+      # READMEs byte for byte against what cargo-rdme emits, and
+      # mutants-list holds tools/mutantcheck-expected.json to the
+      # cargo-mutants release that listed its counts. A floating release
+      # could redefine either expectation with no commit in the repo;
+      # bumping a pin is a reviewed diff that re-pins its artifact in the
+      # same commit. The other tools report findings rather than generate
+      # bytes, so they ride the pinned action's own tool manifest, moving
+      # when Dependabot bumps the pin.
+      - name: Install just, cargo-nextest, cargo-rdme, cargo-fuzz, cargo-mutants, and wasm-pack
+        uses: taiki-e/install-action@37f7c5781271959fb65b6b35224e28652ff2b63d # v2.87.0
         with:
-          toolchain: nightly
+          tool: just,cargo-nextest,cargo-rdme@2.1.0,cargo-fuzz,cargo-mutants@27.1.0,wasm-pack
+
+      # Stable first, read from rust-toolchain.toml's `channel` (the one site
+      # of that pin) because `just` may evaluate the justfile's `rustc`
+      # backtick before it prints the nightly pin; rustup applies this
+      # toolchain to every bare `cargo` under the checkout whatever the
+      # default is, and the dtolnay action takes an explicit `toolchain:`
+      # input and reads no toolchain file.
+      - name: Read the stable pin
+        id: stable
+        run: |
+          channel=$(sed -n 's/^channel = "\(.*\)"$/\1/p' rust-toolchain.toml)
+          if [ -z "$channel" ]; then echo "stable pin not found in rust-toolchain.toml" >&2; exit 1; fi
+          echo "channel=$channel" >> "$GITHUB_OUTPUT"
 
-      - name: Install stable toolchain (default)
+      - name: Install the pinned stable toolchain
         uses: dtolnay/rust-toolchain@6c977a6ca4077a0ceb28ffbe03f59d46e9ac8772 # v1
         with:
-          toolchain: stable
+          toolchain: ${{ steps.stable.outputs.channel }}
           components: clippy, rustfmt
           targets: wasm32-unknown-unknown
 
-      # cargo-rdme carries a version because it is the only tool here whose
-      # output is a committed artifact compared byte for byte: readme-check
-      # holds the READMEs against what it emits. Tracking the newest release
-      # would let an upstream change redefine that expectation with no commit
-      # in the repo, turning the sweep red on a tree nobody touched. The others
-      # report findings rather than generate bytes, so they ride the pinned
-      # action's own tool manifest, moving when Dependabot bumps the pin.
-      - name: Install just, cargo-nextest, cargo-rdme, cargo-fuzz, cargo-mutants, and wasm-pack
-        uses: taiki-e/install-action@37f7c5781271959fb65b6b35224e28652ff2b63d # v2.87.0
+      # The nightly's one site is the justfile's `nightly_toolchain`, which
+      # every nightly recipe names; `just` prints it.
+      - name: Read the nightly pin
+        id: nightly
+        run: |
+          channel=$(just --evaluate nightly_toolchain)
+          if [ -z "$channel" ]; then echo "nightly pin not found in the justfile" >&2; exit 1; fi
+          echo "channel=$channel" >> "$GITHUB_OUTPUT"
+
+      - name: Install the pinned nightly toolchain (merged doctests, docs.rs rustdoc, fuzz build)
+        uses: dtolnay/rust-toolchain@6c977a6ca4077a0ceb28ffbe03f59d46e9ac8772 # v1
         with:
-          tool: just,cargo-nextest,cargo-rdme@2.1.0,cargo-fuzz,cargo-mutants,wasm-pack
+          toolchain: ${{ steps.nightly.outputs.channel }}
 
       # The viz bundle (`just viz`) runs the dependency install (`npm ci`
       # here, where CI=true), a strict TypeScript typecheck, and an esbuild
```

<!-- annotation -->
> **deps-7** (T28), line 76:
>
> Each pin has exactly one site in the tree (stable's `channel` in rust-toolchain.toml, nightly in the justfile) and this step reads both with `sed`, failing loudly if either read is empty, so the workflow cannot diverge from a pin. I chose reading over naming the pins a second time because the entry asks for exactly that ("ideally read from the justfile"), and I verified the two `sed` expressions print `1.97.1` and `nightly-2026-06-30` on this tree. The dtolnay action at the pinned SHA requires an explicit `toolchain:` input and reads no toolchain file (I fetched its action.yml to check), which is why the stable install keeps an input rather than being dropped.

<!-- annotation -->
> **verification-infra-12** (T28), line 70:
>
> The install comments now state today's mechanism: the action does run `rustup default` (confirmed in its action.yml), so the ordering note stays, restated to say what the default governs; the claim that recipes invoke `cargo +nightly` is gone. The same step pair, with the same pins step, appears in all three jobs; I kept it duplicated rather than hoisting it into a composite action, which would be a larger change than the entry names.

<!-- annotation -->
> **verification-infra-8** (T26), line 54:
>
> The entry claimed cargo-mutants rode the install action's floating manifest while `tools/mutantcheck-expected.json` pins `cargo-mutants 27.1.0` and refuses any other version. The install line pins `cargo-mutants@27.1.0` beside `cargo-rdme@2.1.0`, and the comment is rewritten as the resolution states: two tools carry versions because two committed artifacts depend on them, and a bump re-pins the artifact in the same reviewed diff. The local tool is 27.1.0 too (`cargo mutants --version`), which the gate's `mutants-list` leg already holds.

<!-- annotation -->
> **T141 prose pass** (T141), line 54:
>
> Prose pass over the lane's diff. The pins comment loses its restatement of what the install steps do and the motivating sentence about floating channels (the reader holds the step; what it needs is where each pin lives and why the stable step still takes an input). The tools comment says the pin rule once. The instruments and coverage pin comments drop the spelled-out `cargo +<toolchain>` forms, which the recipes already carry. Net shorter.

<!-- annotation -->
> **deps-7** (T28), line 95:
>
> Cross-triage change adopted here so ci.yml has one owner: the nightly pin is read with `just --evaluate nightly_toolchain` (the justfile stays the one pin of record) instead of a `sed` over the justfile, so each job installs its tools first; taiki-e/install-action ships prebuilt binaries and needs no toolchain. The stable pin is still read from rust-toolchain.toml's `channel` with `sed`, since the justfile carries no stable variable. `just --evaluate nightly_toolchain` prints `nightly-2026-06-30` on this tree.

<!-- annotation -->
> **verification-infra-8** (T26), line 54:
>
> Superseded before merge: the mutation campaign is being retired workspace-wide by a `before` lane, so the cargo-mutants install (and the 27.1.0 pin this lane had added) leaves the workflow rather than pinning a tool about to be deleted; `.cargo/mutants.toml`, `tools/mutantcheck`, and the `mutants-list` recipe are untouched here, their deletion being that lane's. The comment returns to the one-tool rule for cargo-rdme. The entry's acceptance clause "tools/workflowlint still passes" is moot, that tool being deleted by the same triage; it passes on this tree regardless. Deviation from the entry's stated resolution, on the coordinator's relayed owner instruction. Note for the merge order: until the mutants lane removes `mutants-list` from the `ci` roster, a CI run of this workflow would fail there for lack of the tool.

<!-- annotation -->
> **fresh-eyes repair** (T28), line 76:
>
> Item 1: each job now installs the tools, reads the stable pin from rust-toolchain.toml with `sed` and installs stable, then reads the nightly with `just --evaluate nightly_toolchain` and installs it, so `just`'s evaluation of the justfile's `rustc` backtick (`host_triple`) can only run after a toolchain is installed; the comments at the steps and the header's tool bullet (cargo-fuzz and cargo-mutants added) say what the workflow does. The round's negative control does not fire as stated on this machine: `env PATH=/usr/bin:/bin just --evaluate nightly_toolchain` prints `nightly-2026-06-30` (exit 0) on just 1.58.0 because it evaluates the one named variable lazily; whole-scope `just --evaluate` under the same PATH fails with `backtick failed with exit code 127` at `host_triple`, so the reorder removes a dependence on `just`'s evaluation order rather than a failure observed today. Stable now precedes nightly, so `rustup default` ends on nightly; harmless, since rust-toolchain.toml governs every bare `cargo` under the checkout and every nightly recipe names its toolchain.

<!-- annotation -->
> **fresh-eyes repair** (T26), line 68:
>
> Item 2: the cargo-mutants install is restored, pinned at 27.1.0 as tools/mutantcheck-expected.json names, so the `ci` roster's `mutants-list` leg has its tool at every commit on main; the `before` triage's mutants lane deletes the install together with the leg. The install comment returns to the two-tool rule.

<a id="hunk-4"></a>
### .github/workflows/ci.yml `@@ -117,42 +136,54 @@ jobs:`

```diff
@@ -117,42 +136,54 @@ jobs:
   #     of record — the gate keeps the second jaw.
   #
   # Runner fit: every step here builds `before` alone (release profile,
-  # counter features) plus the small detached surfacecheck workspace —
-  # never the rumors bench/release tier, whose lib-as-bench compile is
-  # known to exceed 16 GiB; that build lives in `ci`'s bench-build leg,
+  # counter features) plus the small detached surfacecheck workspace;
+  # rumors' release-profile bench builds live in `ci`'s bench-build leg,
   # not in this job.
   #
   # Toolchain coupling: surface-totality parses nightly rustdoc JSON
-  # through a `rustdoc-types` pin matched to the installed nightly's
-  # format_version. This job tracks nightly, so a format bump upstream can
-  # turn the leg red on an untouched tree — the checker refuses loudly,
+  # through a `rustdoc-types` pin matched to the pinned nightly's
+  # format_version. The nightly is dated, so the format moves only when
+  # the pin is bumped deliberately; the checker refuses a mismatch loudly,
   # naming both version numbers, and the justfile's surface-totality
-  # comment documents the re-pin procedure.
+  # comment documents the paired re-pin procedure.
   instruments:
     runs-on: ubuntu-latest
     steps:
       - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
 
-      # Nightly first, stable last (each dtolnay/rust-toolchain step runs
-      # `rustup default`, last one sticks): only the surface-json recipe
-      # invokes `cargo +nightly`; every other step must hit stable.
-      - name: Install nightly toolchain (surface-totality rustdoc JSON)
-        uses: dtolnay/rust-toolchain@6c977a6ca4077a0ceb28ffbe03f59d46e9ac8772 # v1
+      - name: Install just, cargo-nextest, cargo-audit, and cargo-deny
+        uses: taiki-e/install-action@37f7c5781271959fb65b6b35224e28652ff2b63d # v2.87.0
         with:
-          toolchain: nightly
+          tool: just,cargo-nextest,cargo-audit,cargo-deny
 
+      # The pins, read as in `ci`, stable first: only surface-json names the
+      # nightly; every other step runs bare `cargo` under the pinned stable.
       # clippy and rustfmt are for the surfacecheck workspace's own lint
       # legs, which the surface-totality recipe carries.
-      - name: Install stable toolchain (default)
+      - name: Read the stable pin
+        id: stable
+        run: |
+          channel=$(sed -n 's/^channel = "\(.*\)"$/\1/p' rust-toolchain.toml)
+          if [ -z "$channel" ]; then echo "stable pin not found in rust-toolchain.toml" >&2; exit 1; fi
+          echo "channel=$channel" >> "$GITHUB_OUTPUT"
+
+      - name: Install the pinned stable toolchain
         uses: dtolnay/rust-toolchain@6c977a6ca4077a0ceb28ffbe03f59d46e9ac8772 # v1
         with:
-          toolchain: stable
+          toolchain: ${{ steps.stable.outputs.channel }}
           components: clippy, rustfmt
 
-      - name: Install just, cargo-nextest, cargo-audit, and cargo-deny
-        uses: taiki-e/install-action@37f7c5781271959fb65b6b35224e28652ff2b63d # v2.87.0
+      - name: Read the nightly pin
+        id: nightly
+        run: |
+          channel=$(just --evaluate nightly_toolchain)
+          if [ -z "$channel" ]; then echo "nightly pin not found in the justfile" >&2; exit 1; fi
+          echo "channel=$channel" >> "$GITHUB_OUTPUT"
+
+      - name: Install the pinned nightly toolchain (surface-totality rustdoc JSON)
+        uses: dtolnay/rust-toolchain@6c977a6ca4077a0ceb28ffbe03f59d46e9ac8772 # v1
         with:
-          tool: just,cargo-nextest,cargo-audit,cargo-deny
+          toolchain: ${{ steps.nightly.outputs.channel }}
 
       - name: Cache cargo build artifacts
         uses: Swatinem/rust-cache@6323deb102c322ba6fcbdcafc7e3dddab59af2b6 # v2.9.2
```

<!-- annotation -->
> **verification-infra-12** (T28), line 140:
>
> The "lib-as-bench compile known to exceed 16 GiB" claim named a compile `[lib] bench = false` removed; the runner-fit note now says only where rumors' release-profile bench builds live. The coverage job's "known-heavy bench/release tier" (line 227) carried the same expired figure and is swept to the same statement.

<!-- annotation -->
> **verification-infra-12** (T28), line 145:
>
> "This job tracks nightly, so a format bump upstream can turn the leg red on an untouched tree" cannot happen under a dated pin; the coupling note now says the format moves only with a deliberate pin bump, and keeps the checker's loud refusal and the re-pin pointer.

<a id="hunk-5"></a>
### .github/workflows/ci.yml `@@ -185,37 +216,51 @@ jobs:`

```diff
@@ -185,37 +216,51 @@ jobs:
   # pinned expectation (tools/covcheck-expected.json) by tools/covcheck —
   # tamper-evident in both directions, so a new uncovered kernel line or a
   # stale pin entry fails by name. The justfile's coverage section carries
-  # the design and why these are CI legs, never gate legs: each run is a
+  # the design and why these are sweep legs, never gate legs: each run is a
   # full instrumented rebuild plus the whole suite under instrumentation.
   #
   # Runner fit: both legs build the workspace test tier (debug profile,
   # instrumented) — the same shape as `ci`'s test-all leg, well inside the
-  # runner's memory; the known-heavy bench/release tier is never built here.
+  # runner's memory; the bench/release tier is never built here.
   coverage:
     runs-on: ubuntu-latest
     steps:
       - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
 
-      # Nightly first, stable last (each dtolnay/rust-toolchain step runs
-      # `rustup default`, last one sticks): only the branch leg invokes
-      # `cargo +nightly`; the line leg must hit stable. llvm-tools is the
-      # coverage instrumentation's profdata component, needed on both.
-      - name: Install nightly toolchain (branch coverage)
-        uses: dtolnay/rust-toolchain@6c977a6ca4077a0ceb28ffbe03f59d46e9ac8772 # v1
+      - name: Install just, cargo-nextest, and cargo-llvm-cov
+        uses: taiki-e/install-action@37f7c5781271959fb65b6b35224e28652ff2b63d # v2.87.0
         with:
-          toolchain: nightly
-          components: llvm-tools
+          tool: just,cargo-nextest,cargo-llvm-cov
 
-      - name: Install stable toolchain (default)
+      # The pins, read as in `ci`, stable first: the branch leg names the
+      # nightly, the line leg runs bare `cargo` under the pinned stable.
+      # llvm-tools, the coverage instrumentation's profdata component, is
+      # installed on both.
+      - name: Read the stable pin
+        id: stable
+        run: |
+          channel=$(sed -n 's/^channel = "\(.*\)"$/\1/p' rust-toolchain.toml)
+          if [ -z "$channel" ]; then echo "stable pin not found in rust-toolchain.toml" >&2; exit 1; fi
+          echo "channel=$channel" >> "$GITHUB_OUTPUT"
+
+      - name: Install the pinned stable toolchain
         uses: dtolnay/rust-toolchain@6c977a6ca4077a0ceb28ffbe03f59d46e9ac8772 # v1
         with:
-          toolchain: stable
+          toolchain: ${{ steps.stable.outputs.channel }}
           components: llvm-tools
 
-      - name: Install just, cargo-nextest, and cargo-llvm-cov
-        uses: taiki-e/install-action@37f7c5781271959fb65b6b35224e28652ff2b63d # v2.87.0
+      - name: Read the nightly pin
+        id: nightly
+        run: |
+          channel=$(just --evaluate nightly_toolchain)
+          if [ -z "$channel" ]; then echo "nightly pin not found in the justfile" >&2; exit 1; fi
+          echo "channel=$channel" >> "$GITHUB_OUTPUT"
+
+      - name: Install the pinned nightly toolchain (branch coverage)
+        uses: dtolnay/rust-toolchain@6c977a6ca4077a0ceb28ffbe03f59d46e9ac8772 # v1
         with:
-          tool: just,cargo-nextest,cargo-llvm-cov
+          toolchain: ${{ steps.nightly.outputs.channel }}
+          components: llvm-tools
 
       - name: Cache cargo build artifacts
         uses: Swatinem/rust-cache@6323deb102c322ba6fcbdcafc7e3dddab59af2b6 # v2.9.2
```

<!-- annotation -->
> **verification-infra-12** (T28), line 235:
>
> The coverage job installed `llvm-tools` on a floating nightly the branch leg never used; both pinned toolchains now carry the component, per the resolution's first option, so cargo-llvm-cov finds it on whichever toolchain it drives.

<a id="hunk-6"></a>
### Cargo.lock `@@ -1271,7 +1271,6 @@ dependencies = [`

```diff
@@ -1271,7 +1271,6 @@ dependencies = [
  "serde",
  "sha3",
  "smallvec",
- "static_assertions",
  "stats_alloc",
  "thiserror",
  "tinyvec",
```

<!-- annotation -->
> **deps-1** (T26), line 0:
>
> Consequence of removing `static_assertions` from rumors' `[dependencies]`: the lock's entry for the rumors package loses the one line naming it. Nothing else in the lock moves; the crate itself stays resolved for `crates/before`.

<a id="hunk-7"></a>
### Cargo.toml `@@ -85,7 +85,8 @@ readme = "README.md"`

```diff
@@ -85,7 +85,8 @@ readme = "README.md"
 
 # docs.rs renders every feature-gated module the crate docs advertise
 # (`conformance`), matching the gate's all-features rustdoc passes. The
-# `docsrs` cfg turns on lib.rs's `doc_auto_cfg`, so gated items self-label.
+# `docsrs` cfg turns on lib.rs's `doc_cfg` feature, so gated items label
+# their gate; the justfile's `docs-docsrs` leg builds this configuration.
 [package.metadata.docs.rs]
 all-features = true
 rustdoc-args = ["--cfg", "docsrs"]
```

<!-- annotation -->
> **deps-3** (T20), line 88:
>
> The docs.rs metadata comment named `doc_auto_cfg`; it now names `doc_cfg` and the `docs-docsrs` leg that builds this configuration.

<a id="hunk-8"></a>
### Cargo.toml `@@ -119,9 +120,8 @@ meter = ["before/limb-meter", "before/scan-meter"]`

```diff
@@ -119,9 +120,8 @@ meter = ["before/limb-meter", "before/scan-meter"]
 
 [dependencies]
 before = { workspace = true, features = ["serde"] }
-bytes = { workspace = true, features = ["serde"] }
+bytes = { workspace = true }
 sha3 = { workspace = true }
-static_assertions = { workspace = true }
 itertools = { workspace = true }
 serde = { workspace = true, default-features = true, features = ["derive"] }
 ciborium = { workspace = true }
```

<!-- annotation -->
> **deps-1** (T26), line 125:
>
> The `static_assertions` entry is deleted from rumors' `[dependencies]` (annotated at the line that follows it); the workspace table's entry stays because `crates/before` inherits it. `Cargo.lock` loses the one line naming it under the rumors package.

<!-- annotation -->
> **deps-8** (T147), line 123:
>
> Landed on ruling T147, which closes the stop this row recorded: the library never serializes a `Bytes`, so the root dependency loses its `serde` feature, and a `[dev-dependencies]` entry carries the feature for the suites, which use `Bytes` as a payload type (the one-line comment at the entry says so). Acceptance on the box: `cargo check -p rumors --lib --locked` and `cargo check -p rumors --all-targets --locked` both pass; `grep -n '^bytes' Cargo.toml` shows the plain dependency and the featured dev entry. `Cargo.lock` is unchanged (features are not recorded in it).

<a id="hunk-9"></a>
### Cargo.toml `@@ -146,7 +146,9 @@ before = { workspace = true, features = ["serde", "meter"] }`

```diff
@@ -146,7 +146,9 @@ before = { workspace = true, features = ["serde", "meter"] }
 proptest = { workspace = true }
 criterion = { workspace = true, features = ["html_reports"] }
 insta = { workspace = true }
-tokio = { workspace = true, default-features = true, features = [
+# The suites use `Bytes` as a payload type, which must serialize.
+bytes = { workspace = true, features = ["serde"] }
+tokio = { workspace = true, features = [
     "rt",
     "rt-multi-thread",
     "macros",
```

<!-- annotation -->
> **deps-9** (T28), line 151:
>
> The nit claimed `default-features = true` on the tokio dev-dependency is vacuous; it is: tokio 1.x declares no default features, so widening the workspace table's `default-features = false` enables nothing. Deleted; `cargo check -p rumors --all-targets` is unchanged and `Cargo.lock` loses nothing for it.

<a id="hunk-10"></a>
### justfile `@@ -1,6 +1,8 @@`

```diff
@@ -1,6 +1,8 @@
 # rumors workspace: the source of truth for verification. Every artifact in
 # the workspace has a recipe here, tiered by feedback speed, and `just --list`
-# is the tour.
+# is the tour. The one exception class is the hand-run instrument: an
+# `#[ignore]`-gated test no recipe runs, whose own doc states the cost that
+# keeps it out and the command that runs it.
 #
 #   inner loop   just check / just test <filter>     seconds to a minute
 #   commit gate  just gate                           fully clean before every commit
```

<!-- annotation -->
> **verification-infra-14** (T16), line 3:
>
> The opening totality claim now admits its one exception class, the hand-run instrument, defined by what makes it one (an `#[ignore]`-gated test whose module doc states the cost and the command) and naming the sole member. A sentence appended; the header is not reflowed.

<!-- annotation -->
> **fresh-eyes repair** (T16), line 3:
>
> Round 2, item 1: the header's exception class is stated by what makes a test one (`#[ignore]`-gated, no recipe runs it, its doc carries cost and command) without naming a sole member, since `before` holds two more of the class (`exhaustive_deep`, `clock_text_split_survives_two_gib_of_parens`); no count is claimed.

<a id="hunk-11"></a>
### justfile `@@ -9,14 +11,15 @@`

```diff
@@ -9,14 +11,15 @@
 # The gate runs every check a commit must pass: build-free lints first,
 # then every building leg concurrently (see the comment above `gate` for
 # the stream grouping and why parallelism cannot move a verdict).
-# `ci` builds the artifacts the gate doesn't reach (the feature matrix, wasm,
-# bench builds, the viz bundle), exactly as GitHub CI builds them; `all` adds
-# what CI cannot run (the fuzz smoke and the formal tier). Neither sweep
-# repeats the gate's instrument legs — the fuel bands, the board verdicts
-# and pins, and surface totality run in `just gate`, and GitHub CI's
-# `instruments` job re-runs the counter-based subset (the workflow file
-# says which legs stay local and why). The comment above each recipe states
-# what it verifies and why.
+# `ci` is the recipe GitHub CI's `ci` job runs: the gate's lints and tests
+# plus the artifacts the gate doesn't reach (the feature matrix, wasm, bench
+# builds, the viz bundle). `all` adds the coverage legs (CI's `coverage`
+# job) and what CI cannot run (the fuzz smoke, the formal tier, the bench
+# judge). Neither sweep repeats the gate's instrument legs -- the fuel
+# bands, the board verdicts and pins, and surface totality run in
+# `just gate`, and GitHub CI's `instruments` job re-runs the counter-based
+# subset (the workflow file says which legs stay local and why). The
+# comment above each recipe states what it verifies and why.
 
 set shell := ["bash", "-euo", "pipefail", "-c"]
 
```

<!-- annotation -->
> **verification-infra-2** (T15), line 14:
>
> Ruling T15: the header describes `ci` as the recipe CI's `ci` job runs, not as a mirror of every check, and says what `all` actually covers. These are the named lines 12-14 restated; the rest of the header is untouched (the brief says not to reflow it).

<!-- annotation -->
> **fresh-eyes repair** (T141), line 18:
>
> Items 4 and 5: the ragged lines this lane left (header, stream census, sweep header, the `all` paragraph, the coverage section) are reflowed into their paragraphs, and every em-dash on a line this lane added or reflowed is a spaced double hyphen. Untouched lines keep their em-dashes for T48's sweep. This makes the earlier prose-pass row true.

<a id="hunk-12"></a>
### justfile `@@ -102,8 +105,10 @@ check:`

```diff
@@ -102,8 +105,10 @@ check:
 test *args:
     cargo nextest run --workspace {{ args }}
 
-# The gate's test run: every feature (the meter suites and the conformance
-# module build only here).
+# Every feature is lit here and nowhere else in the gate: the meter suites
+# and the conformance module build only under `--all-features`.
+
+# Run the test suites under every feature (the gate's test run).
 test-all *args:
     cargo nextest run --workspace --all-features {{ args }}
 
```

<!-- annotation -->
> **verification-infra-13** (T28), line 111:
>
> The two-line explanatory comment abutted the recipe, so `just --list` showed its second line ("module build only here)."). The explanation stays as a block ending in a blank line and a one-line summary sits directly above the recipe, the pattern the other recipes follow.

<a id="hunk-13"></a>
### justfile `@@ -174,10 +179,15 @@ doclint:`

```diff
@@ -174,10 +179,15 @@ doclint:
     ./tools/doclint --self-test
     ./tools/doclint benches crates examples src tests
 
+# tools/testdoc checks the Rust files under the five roots doclint walks,
+# minus the directory names testdoc's ignore set prunes; it never walks `.`,
+# whose untracked trees (other agents' worktrees under `.claude/`) would
+# otherwise enter the verdict.
+
 # Require every Rust test to document the behavior and invariant it protects.
 testdoc:
     ./tools/testdoc --self-test
-    ./tools/testdoc .
+    ./tools/testdoc benches crates examples src tests
 
 # No other gate leg polices what the CI workflows themselves execute:
 # tools/workflowlint holds every workflow step to committed or
```

<!-- annotation -->
> **verification-infra-9** (T26), line 190:
>
> The recipe names the same explicit roots doclint does instead of `.`; the comment above states why (untracked trees at the repository root). I verified with `git ls-files '*.rs'` that the five roots cover every tracked Rust file.

<!-- annotation -->
> **fresh-eyes repair** (T26), line 182:
>
> Item 6: the testdoc recipe comment states what the mechanism holds (files under the five roots doclint walks, minus the ignored names), not a stronger invariant.

<!-- annotation -->
> **fresh-eyes repair** (T26), line 183:
>
> Round 2, item 3: the testdoc recipe comment names testdoc's ignore set, not doclint's (which has none).

<a id="hunk-14"></a>
### justfile `@@ -263,6 +273,16 @@ docs:`

```diff
@@ -263,6 +273,16 @@ docs:
 docs-internal:
     RUSTDOCFLAGS="-D warnings --html-in-header {{ justfile_directory() }}/crates/before/docs/fuelscape-header.html" cargo doc --workspace --all-features --no-deps --document-private-items --target-dir target/doc-internal
 
+# docs.rs builds rumors under nightly with `--cfg docsrs` (Cargo.toml's
+# docs.rs metadata), the one configuration where lib.rs's `doc_cfg` gate is
+# live; `docs` and `docs-internal` run stable rustdoc without it, so only
+# this leg compiles the path docs.rs takes. One crate, no deps, warnings
+# denied, under the pinned nightly, in its own target dir.
+
+# Build rumors' rustdoc as docs.rs does (pinned nightly, `--cfg docsrs`), warnings denied.
+docs-docsrs:
+    RUSTDOCFLAGS="--cfg docsrs -D warnings" cargo +{{ nightly_toolchain }} doc -p rumors --all-features --no-deps --target-dir target/doc-docsrs
+
 # The before coverage roster (crates/before/src/surface.rs) and the bespoke
 # half of the pointwise-differential tiling (src/testing/diff_ops.rs) cite
 # their binding checks as bare strings. The in-crate suite holds those names
```

<!-- annotation -->
> **deps-3** (T20), line 283:
>
> The new leg: rumors' rustdoc under the pinned nightly with `--cfg docsrs -D warnings`, `-p rumors --no-deps`, in `target/doc-docsrs` so nightly artifacts never invalidate the stable doc passes. I omitted the `--html-in-header` fuelscape flag the stable `docs` recipes carry: it injects `before`'s widget assets, and with `--no-deps` no `before` page is rendered here. The leg passes on this tree in about two seconds after the deps are cached.

<!-- annotation -->
> **T141 prose pass** (T141), line 276:
>
> The docs-docsrs and future-size recipe comments are cut to the mechanism a recipe reader needs (what the leg builds, why nothing else covers it); the sentences about publication-time surfacing and about the failure mode being invisible were motivation that changes nothing the reader does. The header's exception sentence and the testdoc recipe comment are shortened the same way. Lines this lane left ragged in the header, the stream census, and the coverage section are reflowed; nothing else in those paragraphs moves. The recipe comments this lane added are the only prose added net of deletions; each buys the reader the leg's reason for existing, which no other line states.

<a id="hunk-15"></a>
### justfile `@@ -278,8 +298,18 @@ docs-internal:`

```diff
@@ -278,8 +298,18 @@ docs-internal:
 # invocation builds test binaries in the root target/, which is why this leg
 # rides the workspace stream, after test-all has warmed those artifacts.
 
-# Resolve every roster, bespoke, and adequacy-tripwire citation against the
-# collected test inventory.
+# tests/future_size.rs pins the public futures' sizes, and a `cfg` that
+# compiles it empty would read as a pass. This leg reruns that binary from
+# test-all's build set (same features, so no rebuild) with
+# `--no-tests=fail`, so an empty binary fails the gate. The filter names
+# the binary by its package-scoped id, so a same-named binary in another
+# member cannot keep the leg green while rumors' collects nothing.
+
+# Fail unless the future-size pins were collected and pass (liveness for tests/future_size.rs).
+future-size:
+    cargo nextest run --workspace --all-features -E 'binary_id(rumors::future_size)' --no-tests=fail
+
+# Resolve every roster, bespoke, and tripwire citation against the collected test inventory.
 citecheck:
     ./tools/citecheck --self-test
     mkdir -p target && cargo nextest list -p before --all-features --message-format json > target/citecheck-tests.json
```

<!-- annotation -->
> **verification-infra-13** (T28), line 312:
>
> The summary was one sentence wrapped over two comment lines, so the listing showed its tail. Rewritten as one line (shorter, same content).

<!-- annotation -->
> **tests-wire-format-26** (T7), line 309:
>
> The liveness guard the amendment lands: this recipe reruns the future_size binary from the same build set as test-all (workspace, all features, so no rebuild) through nextest's `-E 'binary(future_size)'` filter with `--no-tests=fail`. It sits after test-all in the workspace stream and in the `ci` roster. I chose the filter over `-p rumors --test future_size` because the latter is a different feature set and would recompile rumors for one four-test binary; and over adding `--no-tests=fail` to test-all because nextest already fails an entirely empty run, so that flag on the workspace run guards nothing per-binary. Negative control (reversible, restored): `#![cfg(any())]` at the top of the test file makes `just future-size` report `Starting 0 tests across 1 binary`, `error: no tests to run`, and fail the recipe with exit 4.

<!-- annotation -->
> **fresh-eyes repair** (T7), line 310:
>
> Round 2, item 3: the liveness filter is `binary_id(rumors::future_size)`, the package-scoped id nextest prints, so a same-named binary in another member cannot keep the leg green while rumors' collects nothing. Verified: `cargo nextest list --workspace --all-features -E 'binary_id(rumors::future_size)'` lists the four tests.

<a id="hunk-16"></a>
### justfile `@@ -302,16 +332,15 @@ citecheck:`

```diff
@@ -302,16 +332,15 @@ citecheck:
 # recipe holds only the count seam. The counts derive
 # from the installed cargo-mutants release, so the pin of record
 # (tools/mutantcheck-expected.json) carries the tool version and a bump
-# re-pins in the same reviewed diff.
+# re-pins in the same reviewed diff. `--colors=never` pins the captures
+# byte-deterministic: cargo-mutants honors CARGO_TERM_COLOR=always even
+# when piped (CI toolchain actions export it job-wide), and escapes land
+# inside the operator and function fields the roster patterns match -- the
+# checker refuses a colored capture, and this flag is what keeps that
+# refusal from ever firing.
 # Needs cargo-mutants: `cargo install cargo-mutants`.
 
 # Hold the mutants exclusion roster to its pinned counts (list-only, never a campaign).
-#
-# `--colors=never` pins the captures byte-deterministic: cargo-mutants
-# honors CARGO_TERM_COLOR=always even when piped (CI toolchain actions
-# export it job-wide), and escapes land inside the operator and function
-# fields the roster patterns match — the checker refuses a colored
-# capture, and this flag is what keeps that refusal from ever firing.
 mutants-list:
     ./tools/mutantcheck --self-test
     @mkdir -p target
```

<!-- annotation -->
> **verification-infra-13** (T28), line 344:
>
> The recipe had a summary line, then a `#` blank and the `--colors=never` paragraph directly above the recipe, so the listing showed that paragraph's last line. The paragraph moved into the explanatory block above; the summary now abuts the recipe.

<a id="hunk-17"></a>
### justfile `@@ -351,11 +380,12 @@ supply-chain:`

```diff
@@ -351,11 +380,12 @@ supply-chain:
 # because the drift it catches is caused by ordinary refactors — a rename in
 # `before` breaks a fuzz target in the same commit that lands it, and a
 # compile is seconds of gate time. Only the build: the libFuzzer smoke is
-# poor per-commit spend and runs at `just all` cadence.
+# poor per-commit spend and runs at `just all` cadence. The fmt line is
+# the detached workspace's formatting leg: the root `cargo fmt --all`
+# cannot reach it.
 # Needs cargo-fuzz: `cargo install cargo-fuzz`.
 
-# Build the libFuzzer targets (nightly). The fmt line is the detached
-# workspace's formatting leg: the root `cargo fmt --all` cannot reach it.
+# Build the libFuzzer targets (nightly), with the detached workspace's fmt check.
 [working-directory("crates/before/fuzz")]
 fuzz-build:
     cargo fmt --check
```

<!-- annotation -->
> **verification-infra-13** (T28), line 388:
>
> The fmt-line explanation moved into the block above; the summary line above the attribute mentions the fmt check in a clause so the listing still says what the recipe does beyond building.

<a id="hunk-18"></a>
### justfile `@@ -380,10 +410,10 @@ fuzz-build:`

```diff
@@ -380,10 +410,10 @@ fuzz-build:
 # root `target/` sits in one stream and runs in order there, cheap-first,
 # preserving the fail-fast ordering within it. Every other leg already
 # writes a directory nothing else touches — the nightly doctest target,
-# the private-items doc target, the rustdoc-JSON target, and the detached
-# fuzz/fuzzfit/fuelscape/surfacecheck workspaces — and that is exactly what
-# lets them overlap. `fuzzfit` and `fuelscape-test` share one stream
-# because both build the same wasm guest.
+# the private-items doc target, the docs.rs doc target, the rustdoc-JSON
+# target, and the detached fuzz/fuzzfit/fuelscape/surfacecheck workspaces
+# -- and that is exactly what lets them overlap. `fuzzfit` and
+# `fuelscape-test` share one stream because both build the same wasm guest.
 #
 # Concurrency multiplies peak memory, not just cores. Nothing here caps
 # memory; how many gates share a machine is the operator's call.
```

<!-- annotation -->
> **deps-3** (T20), line 413:
>
> The stream-grouping comment's census of directories nothing else touches gains the docs.rs doc target (`target/doc-docsrs`), the property that lets `docs-docsrs` be its own stream; the reflow of the sentence and the em-dash to a spaced double hyphen are the prose and fresh-eyes passes on the same lines.

<a id="hunk-19"></a>
### justfile `@@ -453,13 +483,14 @@ gate-streams:`

```diff
@@ -453,13 +483,14 @@ gate-streams:
     # tests keep first call on the cores, and the shorter streams fill
     # what the tests leave idle instead of competing for it.
     began=$SECONDS
-    start_stream workspace     0 clippy clippy-default docs test-all citecheck
+    start_stream workspace     0 clippy clippy-default docs test-all future-size citecheck
     start_stream doctest      10 doctest
     start_stream board        10 amp-board-acceptance worst-cases-pin
     start_stream wasm         10 fuzzfit fuelscape-test wasm32-pins
     start_stream fuzz         10 fuzz-build
     start_stream surface      10 surface-totality
     start_stream internal-docs 10 docs-internal
+    start_stream docsrs       10 docs-docsrs
     start_stream audit        10 supply-chain
     wait
 
```

<!-- annotation -->
> **deps-3** (T20), line 493:
>
> Ruling T20 puts the leg in `gate` as well as `ci`. It is its own gate stream: it writes a target dir nothing else touches, which is the property the stream grouping comment names as what lets streams overlap; the census of such directories in that comment gains the docs.rs target.

<a id="hunk-20"></a>
### justfile `@@ -575,13 +606,13 @@ fuzzfit-build:`

```diff
@@ -575,13 +606,13 @@ fuzzfit-build:
 # plus the whole 256-program deterministic prefix judged step by step:
 # the random draws probe novelty, the prefix leg is total). A failure
 # shrinks to a minimal out-of-band shape and writes a proptest seed
-# file — commit any seed that appears.
+# file -- commit any seed that appears. The fmt/clippy lines are the
+# detached workspace's own lint leg (the root `cargo fmt --all`/clippy
+# cannot reach a detached workspace, so without them its source rots
+# invisibly through green gates -- the fuelscape and surfacecheck recipes
+# carry the same discipline).
 
-# Run the fuzz-fit asymptotics suites against the pinned fuel bands. The
-# fmt/clippy lines are the detached workspace's own lint leg (the root
-# `cargo fmt --all`/clippy cannot reach a detached workspace, so without
-# them its source rots invisibly through green gates — the fuelscape and
-# surfacecheck recipes carry the same discipline).
+# Run the fuzz-fit asymptotics suites against the pinned fuel bands.
 [working-directory("crates/before/fuzzfit")]
 fuzzfit: fuzzfit-build
     cargo fmt --check
```

<!-- annotation -->
> **verification-infra-13** (T28), line 615:
>
> The lint-leg explanation moved into the block above (it already described the suites); the summary is the line that was already its first sentence.

<a id="hunk-21"></a>
### justfile `@@ -622,13 +653,14 @@ wasm32-pins-build:`

```diff
@@ -622,13 +653,14 @@ wasm32-pins-build:
     cargo build -p wasm32-pins-guest --release --target wasm32-unknown-unknown --target-dir {{ wasm32pins_target }}
     cargo build -p wasm32-pins-harness --tests --release
 
-# Run the 32-bit boundary pins under wasmtime. The deep pins walk
-# hundreds of megabytes inside a 32-bit guest, so this leg costs minutes
-# of wall time and peaks at a few GiB of host memory across nextest's
-# parallel workers. The fmt/clippy lines are the detached workspace's own
-# lint leg (the root `cargo fmt --all`/clippy cannot reach a detached
-# workspace, so without them its source rots invisibly through green
-# gates — the fuzzfit recipes carry the same discipline).
+# The deep pins walk hundreds of megabytes inside a 32-bit guest, so the
+# run costs minutes of wall time and peaks at a few GiB of host memory
+# across nextest's parallel workers. The fmt/clippy lines are the detached
+# workspace's own lint leg (the root `cargo fmt --all`/clippy cannot reach
+# a detached workspace, so without them its source rots invisibly through
+# green gates -- the fuzzfit recipes carry the same discipline).
+
+# Run the 32-bit boundary pins under wasmtime (minutes; a few GiB of host memory).
 [working-directory("crates/before/wasm32-pins")]
 wasm32-pins: wasm32-pins-build
     cargo fmt --check
```

<!-- annotation -->
> **verification-infra-13** (T28), line 663:
>
> The cost and lint-leg explanation became a block above the recipe; the summary keeps the cost class in a parenthetical because that is what a reader choosing a recipe from the tour needs to know.

<a id="hunk-22"></a>
### justfile `@@ -699,6 +731,8 @@ fuelscape-verify:`

```diff
@@ -699,6 +731,8 @@ fuelscape-verify:
 # README references it by URL, so it must exist in the tree — and
 # before's build.rs holds it fresh; the env var is the explicit opt-in
 # that lets the build write into the source tree.
+
+# Regenerate the README's space-consumption figure from the measurement artifact.
 doc-figure:
     BEFORE_REGEN_DOC_FIGURE=1 cargo build -p before
 
```

<!-- annotation -->
> **verification-infra-13** (T28), line 735:
>
> A blank line and a summary line separate the explanation from the recipe.

<a id="hunk-23"></a>
### justfile `@@ -796,6 +830,8 @@ bench-quick target *filter:`

```diff
@@ -796,6 +830,8 @@ bench-quick target *filter:
 # crates/before/Cargo.toml, whose `deny` keeps roster and seams in sync.
 # Reduced-sampling smoke: append `--sample-size 10 --measurement-time 1`
 # (never quoted).
+
+# Run one allocation-strategy A/B arm of a bench target, saving its criterion baseline.
 bench-alloc-ab target arm="shipped" *filter:
     @case "{{ arm }}" in (shipped|projection_growth|projection_shrink|display_growth) ;; (*) echo 'bench-alloc-ab: unknown arm "{{ arm }}"' >&2; exit 2;; esac
     RUSTFLAGS='{{ if arm == "shipped" { "" } else { '--cfg before_alloc_ab="' + arm + '"' } }}' cargo bench -p before --bench {{ target }} -- --save-baseline {{ target }}-{{ arm }} {{ filter }}
```

<!-- annotation -->
> **verification-infra-13** (T28), line 834:
>
> A blank line and a summary line separate the long protocol comment from the recipe; the listing showed "(never quoted)." before.

<a id="hunk-24"></a>
### justfile `@@ -967,34 +1003,36 @@ worst-cases-pin:`

```diff
@@ -967,34 +1003,36 @@ worst-cases-pin:
 # ── the no-rot sweep ─────────────────────────────────────────────────────────
 # `ci` is the build-everything tier: formatting and lints, the feature matrix,
 # wasm, docs, the full test+doctest run, bench builds, the fuzz-target *build*,
-# and the viz bundle, ordered cheap-first so failures surface early. GitHub CI
-# runs exactly this. Neither `ci` nor `all` runs the gate's instrument legs —
-# the fuel bands, the fuelscape pins, the board's acceptance verdicts and
-# ranking pin, and surface
-# totality run in `just gate` (its recipe line is the
-# roster of record), pre-commit on a developer machine; GitHub CI's
-# `instruments` job re-runs the counter-based subset (board verdicts, the
-# ranking pin, surface totality, and the supply-chain leg) beside
-# the `ci` sweep, leaving the wall-time judge and the wasm fuel tier local.
+# and the viz bundle, ordered cheap-first so failures surface early. GitHub
+# CI's `ci` job runs exactly this. Neither `ci` nor `all` runs the gate's
+# instrument legs -- the fuel bands, the fuelscape pins, the board's
+# acceptance verdicts and ranking pin, and surface totality run in
+# `just gate` (its recipe line is the roster of record), pre-commit on a
+# developer machine; GitHub CI's `instruments` job re-runs the counter-based
+# subset (board verdicts, the ranking pin, surface totality, and the
+# supply-chain leg) beside the `ci` sweep, leaving the wall-time judge and
+# the wasm fuel tier local.
 # CI's `coverage` job carries the two instrumented-coverage legs (the
 # coverage section below): too slow for the gate, judged against the
 # curated kernel pin.
 #
-# `all` is `ci` plus what CI cannot run: a short libFuzzer smoke (poor
-# per-commit spend), the formal tier (the runner has no Lean toolchain) —
-# the kernel-checked proofs, the eventdag oracle/schedule gate, and the
-# muxprobe matrix gate — and the bench judge's two legs: the roster-mode
-# judgment (minutes of criterion runs at two scales; quick mode, so its
-# exponents are judged but never quoted) and the seconds-scale live
-# tripwire, so the judge's red path rides every sweep.
+# `all` is `ci` plus the coverage legs (so the local ladder cannot pass
+# while CI's `coverage` job fails) and what CI cannot run: a short
+# libFuzzer smoke (poor per-commit spend), the formal tier (the runner has
+# no Lean toolchain) -- the kernel-checked proofs, the eventdag
+# oracle/schedule gate, and the muxprobe matrix gate -- and the bench
+# judge's two legs: the roster-mode judgment (minutes of criterion runs at
+# two scales; quick mode, so its exponents are judged but never quoted)
+# and the seconds-scale live tripwire, so the judge's red path rides every
+# sweep.
 
 # Build everything (no fuzz run): the no-rot sweep as CI runs it.
-ci: fmt-check doclint testdoc workflowlint digestshare readme-check fuelscape-claims mutants-list clippy clippy-default features wasm-check docs docs-internal test-all citecheck doctest bench-build fuzz-build fuelscape-verify viz
+ci: fmt-check doclint testdoc workflowlint manifestlint digestshare readme-check fuelscape-claims mutants-list clippy clippy-default features wasm-check docs docs-internal docs-docsrs test-all future-size citecheck doctest bench-build fuzz-build fuelscape-verify viz
 
-# Everything: the no-rot sweep, plus the fuzz smoke, the formal tier, and the bench judge.
-all: ci (fuzz fuzz_smoke_secs) lean eventdag muxprobe bench-judge bench-judge-tripwire
+# Everything: the no-rot sweep, the coverage legs, the fuzz smoke, the formal tier, and the bench judge.
+all: ci coverage-kernel coverage-kernel-branch (fuzz fuzz_smoke_secs) lean eventdag muxprobe bench-judge bench-judge-tripwire
 
-# ── the coverage legs (CI cadence; the gate never runs them) ─────────────────
+# ── the coverage legs (`all` and CI cadence; the gate never runs them) ───────
 # GOAL: no skyline-kernel arm goes silently unexercised — every uncovered
 # kernel line and every untaken branch direction is either curated (a
 # panic-arm or an unreachable arm, its argument stated at the entry) or a
```

<!-- annotation -->
> **deps-3** (T20), line 1030:
>
> The leg joins the `ci` roster beside the other doc passes, cheap-first order kept.

<!-- annotation -->
> **verification-infra-7** (T26), line 1030:
>
> The entry claimed `ci` omitted `manifestlint`, which `gate-lints` runs, so a member-local version restatement passed CI. Added to the `ci` roster right after `workflowlint`, where the gate runs it. Negative control (reversible, restored): `tracing = { workspace = true }` in crates/rumors-tracing/Cargo.toml made `tracing = "0.1"` fails `./tools/manifestlint` ("[dependencies] tracing declares '0.1' instead of inheriting the workspace entry"), and `just --dry-run ci` lists `./tools/manifestlint` in the roster; the full `just ci` runs once at the end of the lane, not per entry, per the brief's resource discipline.

<!-- annotation -->
> **verification-infra-2** (T15), line 1033:
>
> The coverage legs join `all` (ruling T15's first option; no `ci-full`), placed right after `ci` so the deterministic sweep legs run before the fuzz smoke and the wall-time bench judge. Acceptance is the end-of-lane `just all`: if it fails on the `before` meter under instrumentation at this base, that is the expected outcome and is reported, not touched.

<!-- annotation -->
> **verification-infra-2** (T15), line 1035:
>
> The section heading and the "CI legs, never gate legs" sentence now name `all` as the second cadence; the design paragraph is otherwise unchanged.

<a id="hunk-25"></a>
### justfile `@@ -1006,9 +1044,10 @@ all: ci (fuzz fuzz_smoke_secs) lean eventdag muxprobe bench-judge bench-judge-tr`

```diff
@@ -1006,9 +1044,10 @@ all: ci (fuzz fuzz_smoke_secs) lean eventdag muxprobe bench-judge bench-judge-tr
 # Deliberately NOT a global coverage threshold: the worst artifact passing a
 # threshold is a suite that pads covered lines elsewhere; the pin names lines.
 #
-# CI legs, never gate legs: each run is a full instrumented rebuild plus the
-# whole suite under instrumentation — minutes, not gate seconds. The line leg
-# runs on stable; branch instrumentation needs the pinned nightly (the same
+# Sweep legs (`all` and CI's `coverage` job), never gate legs: each run is a
+# full instrumented rebuild plus the whole suite under instrumentation --
+# minutes, not gate seconds. The line leg runs on stable; branch
+# instrumentation needs the pinned nightly (the same
 # toolchain-pin argument as the other nightly legs, and each leg judges only
 # its own toolchain's records — the two map a few regions to different
 # lines). One residual to know when a red arrives: proptest populations draw
```

<!-- annotation -->
> **verification-infra-2** (T15), line 1047:
>
> The coverage design paragraph's "CI legs, never gate legs" sentence now names both sweep cadences (`all` and CI's `coverage` job), since T15 puts the legs in `all`; the paragraph is reflowed and its em-dash on a line this lane touched is a spaced double hyphen.

<a id="hunk-26"></a>
### src/lib.rs `@@ -292,11 +292,12 @@`

```diff
@@ -292,11 +292,12 @@
 //! soundness) and by the wire-format snapshots. Found a gap? An issue or a
 //! test is very welcome.
 
-// Static assertions uses #[allow(unsafe_code)], so we allow it only in tests
-#![cfg_attr(not(test), forbid(unsafe_code))]
-// docs.rs builds pass `--cfg docsrs` (see Cargo.toml's docs.rs metadata), so
-// every feature-gated item self-labels its gate there; inert on stable builds.
-#![cfg_attr(docsrs, feature(doc_auto_cfg))]
+#![forbid(unsafe_code)]
+// docs.rs passes `--cfg docsrs` (Cargo.toml's docs.rs metadata), enabling
+// rustdoc's `doc_cfg` feature so feature-gated items label their gate; no
+// other build sets the cfg, so the attribute is absent everywhere else. The
+// justfile's `docs-docsrs` leg is the same build.
+#![cfg_attr(docsrs, feature(doc_cfg))]
 // Programmer error in recursive async traits can create large futures, so we
 // check to make sure it's not an issue
 #![deny(clippy::large_futures)]
```

<!-- annotation -->
> **deps-1** (T26), line 295:
>
> `#![forbid(unsafe_code)]` is unconditional and the comment explaining the old `cfg_attr(not(test), ..)` is deleted, as the resolution states: with the `assert_eq_size_val!` call gone, nothing in the crate, tests included, expands to `unsafe`.

<!-- annotation -->
> **deps-3** (T20), line 300:
>
> The entry claimed `#![cfg_attr(docsrs, feature(doc_auto_cfg))]` fails the docs.rs build because the nightly removed `doc_auto_cfg` (merged into `doc_cfg` at 1.92). I reproduced the failure on the unfixed tree myself before changing anything: `RUSTDOCFLAGS="--cfg docsrs" cargo +nightly-2026-06-30 doc -p rumors --all-features --no-deps` gives `error[E0557]: feature has been removed ... merged into doc_cfg` (verbatim in the commit message). The attribute now names `doc_cfg`; the comment states what the cfg does and points at the leg that builds it. Verified after the fix: the rendered `conformance` page under `--cfg docsrs` carries the portability label "Available on crate features conformance only.", and the stable `docs` build carries none, so the labeling is automatic under `doc_cfg` as RFC 3631 states.

<!-- annotation -->
> **T141 prose pass** (T141), line 296:
>
> Four lines to three: the same three facts (the cfg, the feature it enables, the leg that builds it) without the second clause restating what auto-labeling is.

<!-- annotation -->
> **fresh-eyes repair** (T20), line 298:
>
> Item 8: "inert on stable builds" replaced by what is true: no build but docs.rs sets the cfg, so the attribute is absent everywhere else (and would error on stable if the cfg were set).

<a id="hunk-27"></a>
### src/peer.rs `@@ -411,10 +411,6 @@ impl<T, B: BookmarkError> Peer<T, B> {`

```diff
@@ -411,10 +411,6 @@ impl<T, B: BookmarkError> Peer<T, B> {
     /// few percent past ~300 MB. It also prices no population ceiling,
     /// so where windows reach corpus scale, the exact solve's numbers
     /// (the table below, and the pinned crossover) replace it.
-    /// Measured: sessions whose serialized one-way trips are counted
-    /// exactly on a virtual clock, at 8–26 MB budgets on the minimal
-    /// and design corpora, ran 1.35–1.96× the form's figure
-    /// (`tests/tradeoff_probe.rs`).
     ///
     /// The ballpark answers, at the specification BDP:
     ///
```

<!-- annotation -->
> **tests-resource-link-window-18** (T16), line 414:
>
> The `Measured: .. ran 1.35-1.96x the form's figure (tests/tradeoff_probe.rs)` sentence is excised (annotated at the blank doc line that now follows the accuracy-band paragraph): it quoted a run no recipe performs. Excised rather than restated because with the figure and the citation removed nothing of the sentence remained that the accuracy-band paragraph above it does not already say. Nothing else in this public rustdoc moved; the sizing guide's relocation is owner decision 80 and out of scope.

<a id="hunk-28"></a>
### src/tree/mirror/streaming/tests/wedge.rs `@@ -35,9 +35,15 @@ const LEAN_WEDGE_FAN: usize = 7;`

```diff
@@ -35,9 +35,15 @@ const LEAN_WEDGE_FAN: usize = 7;
 /// satisfies (`Mux.wedge`: `capLevel := 1`; the theorem `Mux.wedge_margin0`).
 const LEAN_WEDGE_CAP_LEVEL: usize = 1;
 
-/// The Lean wedge literal, transcribed scope-for-scope from the Lean
-/// definition `Mux.wedge` — the Lean definition is the source of truth;
-/// if the literal changes there, change this.
+/// Transcribes the Lean definition `Mux.wedge`, the source of truth, scope
+/// for scope.
+///
+/// The transcription is human-checked: whenever either side changes, the
+/// editor reads the twelve scopes here against `Mux.wedge` in
+/// `Instances.lean`. No gate leg compares them, and that suffices: the
+/// literal is twelve scopes; the generator pin and the session pin below
+/// hold the Rust side rigid; and a Lean edit to the witness is an
+/// owner-level change to the theorems' subject, never an incidental one.
 fn lean_wedge_literal() -> Skel {
     let sc = |kind, height, kids: &[usize], leaf_reqs| Scope {
         kind,
```

<!-- annotation -->
> **streaming-tests-28** (T29), line 41:
>
> Ruling T29 (model): the literal stays a hand transcription and no gate leg compares it to `Instances.lean`; the doc states the manual discipline (human-checked whenever either side changes) and the three reasons the ruling gives for why that suffices, naming the two pins by their test names. No code change. The "twelve scopes" count is the ruling's own argument for the discipline, so it stays as the ruling states it.

<!-- annotation -->
> **T141 prose pass** (T141), line 41:
>
> The two test names leave the doc (a rename would falsify them with the prose untouched); the pins are named by role and are in this file. Shorter by three lines with the ruling's three reasons intact.

<!-- annotation -->
> **fresh-eyes repair** (T29), line 38:
>
> Item 8: the doc's first sentence has a main verb ("Transcribes ...").

<a id="hunk-29"></a>
### src/tree/traverse.rs `@@ -6,10 +6,10 @@`

```diff
@@ -6,10 +6,10 @@
 
 use super::*;
 
-// `act` and `unknown` are `pub(crate)` so rustdoc elsewhere (e.g. the
-// `Levels` docs) can link to the traversal traits inside them: a private
-// `mod` is unnameable from outside `traverse`, so the links would not
-// resolve. The free-function facade below remains the API.
+// `act` and `unknown` are `pub(crate)` so rustdoc elsewhere in the crate
+// can link to the traversal traits inside them: a private `mod` is
+// unnameable from outside `traverse`, so the links would not resolve. The
+// free-function facade below remains the API.
 pub(crate) mod act;
 pub use act::{Action, act};
 
```

<!-- annotation -->
> **tests-wire-format-25** (T7), line 9:
>
> The comment cited the `Levels` docs, a type deleted with `typed::levels`; it now gives the reason without naming a deleted item (rustdoc elsewhere in the crate links to the traversal traits).

<a id="hunk-30"></a>
### src/tree/typed/height.rs `@@ -165,12 +165,16 @@ alias_heights!(`

```diff
@@ -165,12 +165,16 @@ alias_heights!(
 /// denotes its own number's height.
 const _: () = assert!(H0::HEIGHT == 0 && H32::HEIGHT == 32);
 
+/// Heights exist only at the type level: every height type, the full
+/// `Root` chain included, is zero-sized with alignment 1, so a phantom
+/// height neither costs nor pads the struct that carries it.
+const _: () = assert!(size_of::<Z>() == 0 && align_of::<Z>() == 1);
+const _: () = assert!(size_of::<S<Z>>() == 0 && align_of::<S<Z>>() == 1);
+const _: () = assert!(size_of::<Root>() == 0 && align_of::<Root>() == 1);
+
 mod sealed {
     use super::*;
     pub trait Sealed {}
     impl Sealed for Z {}
     impl<H: Sealed> Sealed for S<H> {}
 }
-
-#[cfg(test)]
-mod tests;
```

<!-- annotation -->
> **deps-1** (T26), line 168:
>
> The entry claimed `static_assertions` was a normal dependency serving one test module, and that one redundant test there (`zero_size_val`, whose value/type distinction does not exist for `Sized` types) was the only reason `forbid(unsafe_code)` was conditional. Of the two homes the resolution offers I chose `height.rs` beside the existing `const _` assert: these are compile-time facts of the lib, they fire on every build without a `#[test]` wrapper, and the alternative would have kept a test file holding nothing but them. `tests.rs` and its `mod tests;` line are gone with it. `size_of`/`align_of` come from the prelude. Negative control (reversible mutation, restored): `pub struct Z;` made `pub struct Z(u8);` fails `cargo check -p rumors --lib` with `error[E0080]: evaluation panicked: assertion failed: size_of::<Z>() == 0 && align_of::<Z>() == 1` at this site; the transcript is in the commit message.

<a id="hunk-31"></a>
### src/tree/typed/height/tests.rs `@@ -1,28 +0,0 @@`

```diff
@@ -1,28 +0,0 @@
-use super::*;
-
-/// Every height type, including the full `Root` chain, is zero-sized:
-/// heights exist only at the type level and cost nothing in any struct
-/// that carries one.
-#[test]
-fn zero_size() {
-    static_assertions::assert_eq_size!(Z, ());
-    static_assertions::assert_eq_size!(S<Z>, ());
-    static_assertions::assert_eq_size!(Root, ());
-}
-
-/// Height *values* (not just the types) are zero-sized, so constructing
-/// one is free.
-#[test]
-fn zero_size_val() {
-    static_assertions::assert_eq_size_val!(Z, ());
-    static_assertions::assert_eq_size_val!(S::<Z>::default(), ());
-}
-
-/// Heights have alignment 1, so a phantom height never pads the struct
-/// it tags.
-#[test]
-fn one_align() {
-    static_assertions::assert_eq_align!(Z, ());
-    static_assertions::assert_eq_align!(S<Z>, ());
-    static_assertions::assert_eq_align!(Root, ());
-}
```

<!-- annotation -->
> **deps-1** (T26), line 0:
>
> Deleted: the module held only the three `static_assertions` layout tests (`zero_size`, the redundant `zero_size_val`, `one_align`), which are now `const _: () = assert!(..)` items beside the endpoint assert in `height.rs`, firing on every build; the `mod tests;` line in `height.rs` goes with it.

<a id="hunk-32"></a>
### tests/future_size.rs `@@ -1,42 +1,42 @@`

```diff
@@ -1,42 +1,42 @@
 //! Guardrail that the public futures stay type-erased.
 //!
-//! The mirror protocol's `Levels<Below<…, Below<…, …>>>` chain is ~30 deep,
-//! enough that any layout query that traverses it inline blows past the
-//! default `recursion_limit = 128` and forces downstream crates to bump
-//! their own limit. We defuse that by type-erasing inside the protocol and
-//! `tree::traverse::act`, which leaves the public futures (`Rumors::gossip`,
-//! `Peer::retire`, `Bootstrap::join`) holding nothing more than a
-//! `Pin<Box<dyn Future>>` plus a few locals.
+//! The streaming mirror's typed phase schedule (`streaming::protocol`) is a
+//! deep generic type: a layout query that traverses it inline blows past
+//! the default `recursion_limit = 128` and forces downstream crates to
+//! bump their own limit. Boxed boundaries keep it out of the public
+//! futures: `Reconciliation::reconcile` returns its `#[inline(never)]`
+//! body as a `Pin<Box<dyn Future>>` (with `Handshaken::reconcile`'s boxed
+//! descent below it) and `Rumors::gossip` and `Peer::retire` await through
+//! it; `bootstrap_reconcile` does the same for `Bootstrap::join`; and
+//! `gossip_when` hands out its stream boxed. Each public future therefore
+//! holds one pointer plus its own locals, in either profile, so the budget
+//! is pinned under the dev profile the gate runs.
 //!
-//! If the deep chain is reintroduced inline (say, the `Box::pin` indirection
-//! removed, or a new public future driving the protocol directly), the
-//! future size jumps from a couple hundred bytes to tens of KiB and trips
-//! the budget — before downstream crates discover the `recursion_limit`
-//! regression.
-//!
-//! The budget is enforced only in release builds: debug layouts carry
-//! additional state, and they are not what users ship.
-
-#![cfg(not(debug_assertions))]
+//! Removing one of those outer boxes, or adding a public future that drives
+//! the protocol without one, trips the budget here before downstream crates
+//! discover the `recursion_limit` regression. The justfile's `future-size`
+//! recipe reruns this binary with `--no-tests=fail`, so a `cfg` that
+//! compiles it empty fails the gate instead of reading as a pass.
 
 use std::mem::size_of_val;
 
+use futures::stream;
 use rumors::{Peer, Rumors};
 
-/// Upper bound for the unawaited public futures.
+/// Upper bound for the unawaited public futures and the `gossip_when` stream.
 ///
-/// The budget is set generously above the measured sizes (a few hundred bytes)
-/// so legitimate growth — an extra captured local, a slightly fatter error type
-/// — doesn't fail the test, but any *order-of-magnitude* growth (i.e. the inner
-/// protocol state machine leaking out inline) will.
-const PUBLIC_FUTURE_BUDGET: usize = 2048;
+/// Measured identically under both profiles, the largest is `Peer::retire`
+/// at 2000 bytes. The budget sits half again above it, so an extra captured
+/// local or a fatter error type passes, and below what removing
+/// `Reconciliation::reconcile`'s box alone produces, so the cheapest
+/// boundary regression fails.
+const PUBLIC_FUTURE_BUDGET: usize = 3072;
 
 /// `Rumors::gossip` drives the full mirror protocol against a peer; the
 /// public future is type-erased.
 ///
-/// The erasure is `mirror()`'s internal `Pin<Box<dyn
-/// Future>>`, so the protocol's `Levels` chain doesn't appear in the
-/// caller's layout query.
+/// The erasure is `Reconciliation::reconcile`'s boxed future, so the typed
+/// phase schedule never appears in the caller's layout query.
 #[test]
 fn gossip_future_fits_budget() {
     let (mut link, peer) = rumors::link::memory();
```

<!-- annotation -->
> **tests-wire-format-26** (T7), line 3:
>
> The entry claimed the binary was `#![cfg(not(debug_assertions))]` and so had never run under any committed check. The cfg is lifted (T7, amended: no release leg). Measured once in each profile with the budget set to zero so every test reported its size: dev and release agree exactly (stream 8, bootstrap 336, gossip 960, retire 2000 bytes), which is the profile-independence the resolution's premise states. The first dev run did not fail the existing 2048 budget, but retire sat 48 bytes under it, so the budget is re-pinned with headroom as the resolution asks, and this note says so: 3072, half again above the largest, and below the roughly 3.9 KiB that unboxing `Reconciliation::reconcile` alone produces, so the stated demonstration trips on every session future. `gossip_when`'s stream joins the measured surface (8 bytes: its unfold box).

<!-- annotation -->
> **tests-wire-format-26** (T7), line 15:
>
> Negative controls (reversible mutations in src/peer/gossip.rs and src/tree/mirror/streaming.rs, restored, diff empty; transcripts in the commit message): (1) `Reconciliation::reconcile` returning `impl Future` instead of `Box::pin`: gossip 3936 and retire 4976 bytes, both over 3072, bootstrap and the stream unmoved; (2) every box down to the descent removed (`Handshaken::reconcile` and both `Box::pin(handshaken.reconcile())` sites too): gossip 95,256 and retire 96,296 bytes. So the old doc's "jumps to tens of KiB" was true of the inner boundary, not of the outer one the entry names; the doc now states both measured outcomes instead. Finding for the owner: the outer box alone hides the handshake state (about 3 KiB), and the descent's erasure is `Handshaken::reconcile`'s box; `bootstrap_reconcile` is bootstrap's own boundary (its size never moved under either mutation).

<!-- annotation -->
> **tests-wire-format-25** (T7), line 26:
>
> The mechanism is restated against today's code: the typed phase schedule under `streaming::protocol` is the deep type; `Reconciliation::reconcile`'s boxed `inline(never)` future (with `Handshaken::reconcile` below it), `bootstrap_reconcile`, and `gossip_when`'s boxed unfold are the boundaries; the assert messages name `Reconciliation::reconcile` (and `bootstrap_reconcile` for the bootstrap test, since that is the box a maintainer would have removed there). The `mirror()` and `Levels` references are gone.

<!-- annotation -->
> **T141 prose pass** (T141), line 15:
>
> The module doc keeps the mechanism and the boundaries and drops the measured readings (the quadrupling, the 95 KiB) and the path citation: readings a code change can falsify with the prose untouched belong in the commit message, where they now live alone. It also corrects a claim my earlier wording made: removing an inner box alone is hidden by the outer one, so the sentence names the outer boxes. The budget constant's doc keeps one reading, the largest measured size, because ruling T7's resolution asks for the measured band beside the budget; that is the one added sentence this pass leaves that a reviewer must weigh.

<a id="hunk-33"></a>
### tests/future_size.rs `@@ -49,9 +49,9 @@ fn gossip_future_fits_budget() {`

```diff
@@ -49,9 +49,9 @@ fn gossip_future_fits_budget() {
     assert!(
         size <= PUBLIC_FUTURE_BUDGET,
         "gossip future is {size} bytes, exceeds budget {PUBLIC_FUTURE_BUDGET}; \
-         if a recent change removed the internal `Pin<Box<dyn Future>>` \
-         indirection, restore it — otherwise downstream crates will hit \
-         `recursion_limit` overflow",
+         if a recent change removed the `Pin<Box<dyn Future>>` returned by \
+         `Reconciliation::reconcile`, restore it; otherwise downstream crates \
+         will hit `recursion_limit` overflow",
     );
 }
 
```

<!-- annotation -->
> **tests-wire-format-25** (T7), line 52:
>
> The gossip test's assert message names the boundary a maintainer would have removed, `Reconciliation::reconcile`'s returned `Pin<Box<dyn Future>>`, instead of an unnamed "internal indirection"; the em-dash is a semicolon.

<a id="hunk-34"></a>
### tests/future_size.rs `@@ -69,12 +69,13 @@ fn retire_future_fits_budget() {`

```diff
@@ -69,12 +69,13 @@ fn retire_future_fits_budget() {
     assert!(
         size <= PUBLIC_FUTURE_BUDGET,
         "retire future is {size} bytes, exceeds budget {PUBLIC_FUTURE_BUDGET}; \
-         see gossip_future_fits_budget for rationale",
+         see gossip_future_fits_budget: the erasure is `Reconciliation::reconcile`",
     );
 }
 
-/// `Bootstrap::join` runs the same mirror descent from an empty tree.
-/// Same erasure boundary as `gossip`.
+/// `Bootstrap::join` runs the same mirror descent from an empty tree; its
+/// erasure is `bootstrap_reconcile`'s boxed future, the same discipline
+/// as `Reconciliation::reconcile`.
 #[test]
 fn bootstrap_future_fits_budget() {
     let (mut link, peer) = rumors::link::memory();
```

<!-- annotation -->
> **tests-wire-format-25** (T7), line 72:
>
> The retire message points at the same boundary by name, and the bootstrap test's doc names its own: `bootstrap_reconcile`'s boxed future, the same discipline as `Reconciliation::reconcile` (under both adequacy mutations bootstrap's size never moved, which is what identified that boundary).

<a id="hunk-35"></a>
### tests/future_size.rs `@@ -86,6 +87,29 @@ fn bootstrap_future_fits_budget() {`

```diff
@@ -86,6 +87,29 @@ fn bootstrap_future_fits_budget() {
     assert!(
         size <= PUBLIC_FUTURE_BUDGET,
         "bootstrap future is {size} bytes, exceeds budget {PUBLIC_FUTURE_BUDGET}; \
-         see gossip_future_fits_budget for rationale",
+         if a recent change removed the `Pin<Box<dyn Future>>` returned by \
+         `bootstrap_reconcile`, restore it; see gossip_future_fits_budget",
+    );
+}
+
+/// `Rumors::gossip_when`'s public stream is one boxed pointer.
+///
+/// What this test would first observe is a stream handed out unboxed with
+/// deep state behind it; the stream cannot see the `Reconciliation::reconcile`
+/// boundary, which the gossip and retire tests beside it hold.
+#[test]
+fn gossip_when_stream_fits_budget() {
+    let (mut link, peer) = rumors::link::memory();
+    drop(peer);
+
+    let alice: Rumors<()> = Peer::seed().sync_window_floor().into_rumors();
+    let sessions = alice.gossip_when(stream::empty::<()>(), &mut link);
+    let size = size_of_val(&sessions);
+
+    assert!(
+        size <= PUBLIC_FUTURE_BUDGET,
+        "gossip_when stream is {size} bytes, exceeds budget {PUBLIC_FUTURE_BUDGET}; \
+         the stream is handed out unboxed with deep state behind it: restore \
+         the `Box::pin` around `gossip_when`'s unfold",
     );
 }
```

<!-- annotation -->
> **fresh-eyes repair** (T7), line 97:
>
> Item 3 (negative control): with the `Box::pin` around `gossip_when`'s unfold removed (and the `Unpin` bound dropped from both signatures so it compiles), measured on `-p rumors --test future_size`: `gossip_when stream is 1080 bytes`, under the 3072 budget (the other three read 960, 2000, 336 as before). So that box is not a boundary the budget protects; the doc and the assert message now say the stream's session awaits through `Reconciliation::reconcile` and the unfold's box exists for `Unpin`. `src` restored; identical to base.

<!-- annotation -->
> **fresh-eyes repair** (T7), line 95:
>
> Round 2, item 2: the gossip_when doc and message say what the test holds (the public stream is one boxed pointer; it would first observe a stream handed out unboxed with deep state) and that the reconcile boundary is held by the gossip and retire tests, since the stream cannot observe it (my own control: 8 bytes with that boundary in place, 1080 with only the unfold's box removed). The module doc's clause about the stream says only that it is handed out boxed.

<a id="hunk-36"></a>
### tests/tradeoff_probe.rs `@@ -3,8 +3,12 @@`

```diff
@@ -3,8 +3,12 @@
 //!
 //! It validates, deterministically, that measured slowdowns stay at or
 //! inside the wave form evaluated at the window the real derivation
-//! grants — the quantity the committed trade-off table tabulates — and
-//! runs only by explicit request:
+//! grants -- the quantity the committed trade-off table tabulates.
+//!
+//! It is a hand-run instrument, in no recipe: every cell drives whole
+//! sessions over the design corpus, too slow for the gate, for a claim
+//! that moves only with the derivation or the wire law. Run it after
+//! either moves:
 //!
 //!     cargo nextest run --release --test tradeoff_probe \
 //!         --run-ignored all --no-capture
```

<!-- annotation -->
> **tests-resource-link-window-18** (T16), line 8:
>
> The entry claimed the `#[ignore]` gate's cost rationale lived only in history and that public rustdoc quoted a measurement nothing schedules. Ruling T16: no recipe; the file stays; its module doc states that it is a hand-run instrument and why. I state the cost by mechanism (design-corpus sessions per cell; seconds in release, several times that under the gate's dev profile) rather than the entry's eleven-second figure, which I did not measure and which would rot as a hand-maintained number.

<!-- annotation -->
> **T141 prose pass** (T141), line 8:
>
> The corpus size the sentence repeated is already in the Method list below it; the sentence now states class and cost only.

<!-- annotation -->
> **fresh-eyes repair** (T16), line 9:
>
> Item 7: the cost is stated as its class, too slow for the gate and hand-run, with no reading (nobody measured one); the ignore message says the same.

<a id="hunk-37"></a>
### tests/tradeoff_probe.rs `@@ -184,7 +188,7 @@ fn run_cells<T>(`

```diff
@@ -184,7 +188,7 @@ fn run_cells<T>(
 /// assertion of record holds the observation at or inside the
 /// solve-derived wave form, in hops.
 #[test]
-#[ignore = "one-shot validation instrument: run explicitly with --run-ignored"]
+#[ignore = "hand-run validation instrument, too slow for the gate: run explicitly with --run-ignored"]
 fn tradeoff_closed_form_validation_run() {
     // m = 9: minimal u64 records (the table's first column; a random
     // u64's CBOR encoding is nine bytes). Targets (denominated in the
```

<!-- annotation -->
> **verification-infra-14** (T16), line 191:
>
> The ignore message names the instrument's class and cost class beside the invocation hint, so a reader at the attribute sees the rationale without opening the module doc.

<a id="hunk-38"></a>
### tools/testdoc `@@ -6,22 +6,42 @@ Usage:`

```diff
@@ -6,22 +6,42 @@ Usage:
     testdoc --self-test
 
 The checker is deliberately lexical. Rust test entry points advertise
-themselves with an attribute, so recognizing those attributes and the item
-preamble is both faster and less brittle than depending on a Rust parser.
+themselves in two forms: a test attribute (`#[test]`, any attribute path
+ending in `::test` such as `#[tokio::test]`, `#[rstest]`, `#[test_case]`),
+and a function declared directly inside a `proptest! { .. }` block, which
+the macro makes a test whether or not `#[test]` is written above it.
+Recognizing those forms and the item preamble is faster and less brittle
+than depending on a Rust parser.
+
+Directories named in `IGNORED_DIRECTORIES` are never entered under any
+root: build output and vendored trees hold none of this repository's
+tests, and `.claude` may hold other agents' worktrees, whose state must
+not move this tree's verdict. The gate's explicit roots are the first
+guard; this set is the second.
 """
 
 import os
 import re
 import sys
+import tempfile
 from pathlib import Path
 
 
-IGNORED_DIRECTORIES = {".git", "node_modules", "target"}
+IGNORED_DIRECTORIES = {".git", ".claude", "node_modules", "target"}
 TEST_ATTRIBUTE = re.compile(
-    r"^\s*#\[\s*(?:test|tokio::test|async_std::test|rstest|test_case)"
+    r"^\s*#\[\s*(?:(?:[A-Za-z_][A-Za-z0-9_]*::)*test|rstest|test_case)"
     r"\s*(?:\([^]]*\))?\s*\]\s*$"
 )
+# The block form's opener; the closure form `proptest!(|..| { .. })` sits
+# inside a function body and declares no test of its own.
+PROPTEST_BLOCK = re.compile(r"^\s*(?:[A-Za-z_][A-Za-z0-9_]*::)*proptest!\s*\{")
 FUNCTION = re.compile(r"\b(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)")
+FUNCTION_DECL = re.compile(
+    r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)"
+)
+CHAR_LITERAL = re.compile(r"'(?:[^'\\]|\\.)'")
+STRING_LITERAL = re.compile(r'"(?:[^"\\]|\\.)*"')
+LINE_COMMENT = re.compile(r"//.*$")
 
 
 def is_outer_doc(line):
```

<!-- annotation -->
> **verification-infra-9** (T26), line 30:
>
> The entry claimed the gate's testdoc verdict depended on other agents' worktrees under `.claude/`, because the recipe walked `.` and the tool skipped only `.git`, `node_modules`, and `target`. I added `.claude` to the ignore set as the second guard the resolution asks for; the first guard is the recipe naming explicit roots (see the justfile row). Negative control: an undocumented test placed under `.claude/worktrees/agent-probe/src/probe.rs` is reported by the HEAD tool walking `.` (exit 1) and by neither the new recipe nor the new tool walking `.` (exit 0); the transcript is in the commit message.

<!-- annotation -->
> **tests-observation-37** (T30), line 32:
>
> The entry (with its duplicates remote-proxy-tests-27 and tests-disruption-handshake-33) claimed `#[pollster::test]` never matched the attribute regex, so the 29 pollster tests were outside doc enforcement. I took the general form the duplicates propose: any attribute path ending in `::test`, beside `rstest` and `test_case` exactly as before. I considered also allowing a path prefix on `rstest`/`test_case` (`#[rstest::rstest]`) and rejected it as wider than the brief's stated form; nothing in the tree uses it. After widening, `./tools/testdoc benches crates examples src tests` on the tree reports nothing: every pollster test is documented today, so no documentation change rides with this commit. Negative control: stripping the doc above `tests/changes.rs:18` and running `just testdoc` fails naming `first_poll_yields_immediately` (restored; diff empty).

<!-- annotation -->
> **verification-infra-10** (T28), line 37:
>
> The entry claimed a `proptest! { fn .. }` block function without an explicit `#[test]` was invisible to the tool. I track the block form lexically from its `proptest! {` opener by brace depth and treat every `fn` declared at depth 1 as a test entry point. The closure form `proptest!(|..| { .. })` is not an opener: it sits inside a function body and declares no test.

<!-- annotation -->
> **T141 prose pass** (T141), line 9:
>
> The docstring's two paragraphs say each fact once; `#[pollster::test]` as a second example and the clause about roots growing ignored subtrees were restatement.

<a id="hunk-39"></a>
### tools/testdoc `@@ -30,9 +50,13 @@ def is_outer_doc(line):`

```diff
@@ -30,9 +50,13 @@ def is_outer_doc(line):
     return stripped.startswith("///") and not stripped.startswith("////")
 
 
-def has_attached_doc(lines, attribute):
-    """Whether the attribute block containing `attribute` is led by `///`."""
-    line = attribute - 1
+def has_attached_doc(lines, item):
+    """Whether the attribute block ending at line `item` is led by `///`.
+
+    `item` may be an attribute line or the function line itself: the walk
+    upward skips attributes and ordinary comments either way.
+    """
+    line = item - 1
     while line >= 0:
         stripped = lines[line].strip()
         if is_outer_doc(lines[line]):
```

<!-- annotation -->
> **verification-infra-10** (T28), line 53:
>
> `has_attached_doc` now takes either an attribute line or the function line itself, because the `proptest!` block scan starts its upward walk at a `fn` that may carry no attribute at all; the walk's rules (skip attributes and ordinary comments, stop at anything else) are unchanged, so the attribute path behaves as before.

<a id="hunk-40"></a>
### tools/testdoc `@@ -47,24 +71,68 @@ def has_attached_doc(lines, attribute):`

```diff
@@ -47,24 +71,68 @@ def has_attached_doc(lines, attribute):
     return False
 
 
-def test_name(lines, attribute):
-    """Find the function named by a test attribute for a useful diagnostic."""
-    for line in lines[attribute : attribute + 16]:
+def function_after(lines, attribute):
+    """The `(line_index, name)` of the function a test attribute names.
+
+    Returns `(None, "<unknown>")` when no function follows within the item
+    preamble, so the diagnostic still names the attribute's line.
+    """
+    for offset, line in enumerate(lines[attribute : attribute + 16]):
         match = FUNCTION.search(line)
         if match:
-            return match.group(1)
-    return "<unknown>"
+            return attribute + offset, match.group(1)
+    return None, "<unknown>"
+
+
+def brace_delta(line):
+    """Net `{` minus `}` on a line, with literals and line comments removed.
+
+    Lexical, like the rest of the tool: a brace inside a block comment or a
+    raw string still counts. The self-test pins the forms the tree uses.
+    """
+    code = CHAR_LITERAL.sub("", line)
+    code = STRING_LITERAL.sub("", code)
+    code = LINE_COMMENT.sub("", code)
+    return code.count("{") - code.count("}")
 
 
 def missing_test_docs(lines):
-    """Yield `(line_number, test_name)` for undocumented test attributes."""
+    """Yield `(line_number, test_name)` for undocumented test entry points.
+
+    A function is reported at most once, whichever form announced it: a
+    `proptest!` block function carrying an explicit `#[test]` is found by
+    both scans and reported by the first.
+    """
+    reported = set()
+    # Brace depth inside the enclosing `proptest!` block; 0 outside one. A
+    # function declared at depth 1 is the block's direct child, and so a test.
+    depth = 0
     for index, line in enumerate(lines):
-        if TEST_ATTRIBUTE.match(line) and not has_attached_doc(lines, index):
-            yield index + 1, test_name(lines, index)
+        if TEST_ATTRIBUTE.match(line):
+            function, name = function_after(lines, index)
+            key = index if function is None else function
+            if key not in reported and not has_attached_doc(lines, index):
+                reported.add(key)
+                yield index + 1, name
+            continue
+        if depth == 0:
+            if PROPTEST_BLOCK.match(line):
+                depth = brace_delta(line)
+            continue
+        if depth == 1:
+            declaration = FUNCTION_DECL.match(line)
+            if (
+                declaration
+                and index not in reported
+                and not has_attached_doc(lines, index)
+            ):
+                reported.add(index)
+                yield index + 1, declaration.group(1)
+        depth += brace_delta(line)
 
 
 def rust_files(root):
-    """Yield repository source files without descending into generated trees."""
+    """Yield repository source files without descending into ignored trees."""
     if root.is_file():
         if root.suffix == ".rs":
             yield root
```

<!-- annotation -->
> **verification-infra-10** (T28), line 87:
>
> Brace counting strips char literals, string literals, and `//` comments first, in that order, so a `"{"` in a test body cannot leave the tracker inside the block forever. This is still lexical (a raw string or a block comment holding a brace would count); the self-test pins the forms the tree uses. I judged that the right depth for a tool whose header promises no parser.

<!-- annotation -->
> **verification-infra-10** (T28), line 100:
>
> A block function carrying an explicit `#[test]` is found by both the attribute scan and the block scan; `reported` (keyed by the function's line) makes it one finding, not two. The attribute scan keys by the function line it names so the two scans agree on the identity of a test.

<a id="hunk-41"></a>
### tools/testdoc `@@ -79,7 +147,7 @@ def rust_files(root):`

```diff
@@ -79,7 +147,7 @@ def rust_files(root):
 
 
 def self_test():
-    """Pin the lexical forms the gate relies on."""
+    """Pin the lexical forms the gate relies on, and the directory walk."""
     cases = [
         ("/// significance\n#[test]\nfn documented() {}", []),
         ("#[test]\nfn missing() {}", [(1, "missing")]),
```

<!-- annotation -->
> **verification-infra-10** (T28), line 150:
>
> Self-test cases added for `#[pollster::test]` (documented and not), `#[async_std::test]`, `#[rstest]`, the `proptest!` block form with and without `#[test]`, the single-finding dedupe, a multi-line `#![proptest_config(..)]` inner attribute, braces inside literals and comments, a helper `fn` nested in a block function's body (not a test), and the closure form (no test). Negative control for the block form: the entry's scratch file (an undocumented `proptest!` function plus an undocumented `#[test]`) yields two findings; transcript in the commit message.

<a id="hunk-42"></a>
### tools/testdoc `@@ -97,15 +165,88 @@ def self_test():`

```diff
@@ -97,15 +165,88 @@ def self_test():
             "async fn asynchronous() {}",
             [],
         ),
+        ("/// significance\n#[pollster::test]\nasync fn blocking() {}", []),
+        ("#[pollster::test]\nasync fn blocking() {}", [(1, "blocking")]),
+        ("#[async_std::test]\nasync fn other_runtime() {}", [(1, "other_runtime")]),
+        ("#[rstest]\nfn fixtures() {}", [(1, "fixtures")]),
         ("// ordinary comment\n#[test]\nfn ordinary() {}", [(2, "ordinary")]),
         ("//! module docs\n#[test]\nfn module_only() {}", [(2, "module_only")]),
         ("//// separator\n#[test]\nfn separator() {}", [(2, "separator")]),
+        # The `proptest!` block form: each direct-child `fn` is a test, with
+        # or without an explicit `#[test]`.
+        (
+            "proptest! {\n    fn property(x in 0..1u8) {\n        assert!(x < 2);\n"
+            "    }\n}",
+            [(2, "property")],
+        ),
+        (
+            "proptest! {\n    /// significance\n    fn property(x in 0..1u8) {}\n}",
+            [],
+        ),
+        (
+            "proptest! {\n    /// significance\n    #[test]\n"
+            "    fn property(x in 0..1u8) {}\n}",
+            [],
+        ),
+        # An explicit `#[test]` on an undocumented block function is one
+        # finding, not two.
+        (
+            "proptest! {\n    #[test]\n    fn property(x in 0..1u8) {}\n}",
+            [(2, "property")],
+        ),
+        # A block-level `#![proptest_config(..)]`, its braces split across
+        # lines, does not hide the functions after it.
+        (
+            "proptest! {\n    #![proptest_config(ProptestConfig {\n"
+            "        cases: 8,\n        ..ProptestConfig::default()\n    })]\n"
+            "    fn configured(x in 0..1u8) {}\n}",
+            [(6, "configured")],
+        ),
+        # Braces inside string and char literals and line comments do not
+        # move the depth: the second function is still the block's child.
+        (
+            "proptest! {\n    /// significance\n    fn first(x in 0..1u8) {\n"
+            "        let _ = \"{\"; let _ = '}'; // {\n    }\n"
+            "    fn second(y in 0..1u8) {}\n}",
+            [(6, "second")],
+        ),
+        # A function nested inside a block function's body is not a test.
+        (
+            "proptest! {\n    /// significance\n    fn outer(x in 0..1u8) {\n"
+            "        fn helper() {}\n    }\n}",
+            [],
+        ),
+        # The closure form declares no test.
+        ("fn body() {\n    proptest!(|(x in 0..1u8)| {\n        assert!(x < 2);\n    });\n}", []),
     ]
     for source, expected in cases:
         actual = list(missing_test_docs(source.splitlines()))
         if actual != expected:
             raise AssertionError(f"expected {expected}, got {actual} for:\n{source}")
 
+    # The walk: an explicitly named root is scanned; the ignored directory
+    # names are pruned wherever they appear beneath it.
+    with tempfile.TemporaryDirectory() as scratch:
+        root = Path(scratch)
+        expected = [root / "src" / "kept.rs", root / "src" / "nested" / "kept.rs"]
+        pruned = [
+            root / ".claude" / "worktrees" / "agent" / "src" / "pruned.rs",
+            root / "target" / "debug" / "pruned.rs",
+            root / "src" / "target" / "pruned.rs",
+            root / "src" / ".git" / "pruned.rs",
+            root / "src" / "node_modules" / "pruned.rs",
+            root / "src" / "not_rust.txt",
+        ]
+        for path in expected + pruned:
+            path.parent.mkdir(parents=True, exist_ok=True)
+            path.write_text("#[test]\nfn undocumented() {}\n", encoding="utf-8")
+        walked = sorted(rust_files(root))
+        if walked != sorted(expected):
+            raise AssertionError(f"expected {sorted(expected)}, walked {walked}")
+        # A file named directly is scanned regardless of its directory.
+        if list(rust_files(pruned[0])) != [pruned[0]]:
+            raise AssertionError(f"explicit file {pruned[0]} was not yielded")
+
 
 def main(argv):
     if argv[1:] == ["--self-test"]:
```

<!-- annotation -->
> **verification-infra-9** (T26), line 227:
>
> The self-test case for the ignore list the resolution asks for: a temporary tree where `.claude/worktrees/..`, `target/`, and nested `.git`/`node_modules`/`target` directories under a scanned root are pruned, files under `src/` are kept, and a file named explicitly is scanned regardless of its directory (the recipe passes directories, but the behavior is pinned so a future caller naming a file inside an ignored tree is not silently skipped).

