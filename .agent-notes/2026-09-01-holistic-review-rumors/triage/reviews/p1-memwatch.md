<!-- CAVEAT LECTOR: review packet for lane p1-memwatch, written by Claude (Fable 5.1) for Finch under triage/WORKFLOW.md. The goal, rulings, stack position, acceptance rows, fresh-eyes rounds, and stops are the coordinator's record; nothing in this packet is authored or endorsed by Finch. Read with the ground rules in .agent-notes/README.md. -->

# Review packet: p1-memwatch

## Goal

`tools/memwatch` wrapped every building recipe to abort builds on a
per-process ceiling and a machine-wide swap figure. The swap figure is
one macOS drains lazily, so the wrapper killed healthy gates on a
128 GiB machine with tens of gigabytes free, and on illumos its `ps`
flags are invalid. Finch ruled it retired with nothing in its place:
the concurrent-builder cap is the resource control of record. This
lane deletes the script, unwraps every recipe so each runs the same
command as before, and re-states every mention of the wrapper in the
present tense. It runs first and unstacked; the gate lane rebases onto
it.

## Rulings landed

- T136: memwatch is retired; every wrapped recipe unwrapped; every
  mention re-stated; replacement nothing, by owner decision.
- T141: the prose standard, applied to every touched paragraph.

## Stack position

- Base: `51ccf03c` (main)
- Parent: `main`
- Children: `p1-gate` (rebases onto this lane's merge; its own
  `future-size` recipe carries the wrapper prefix this lane could not
  see and unwraps it at the rebase).

## Acceptance table

Every row is one the coordinator's verification runner ran against the
worktree at `6c54b1a6`, with whole logs kept.

| entry | commit | acceptance command | decisive output |
|---|---|---|---|
| `memwatch-retirement` | `6c54b1a6` | `git grep -n -i -E 'memwatch\|SWAP_LIMIT_GB\|PROC_LIMIT_GB' -- . ':!.agent-notes'`; `test ! -e tools/memwatch` | empty (exit 1); `absent` |
| (same) | `6c54b1a6` | `just --list --unsorted` at the parent justfile and at head, diffed | identical, 62 lines each |
| (same) | `6c54b1a6` | every changed justfile line filtered for anything but a `memwatch` prefix removal or a comment | twelve `+` lines, each the same command as its removed line minus the prefix; `citecheck` loses its `bash -c` carrier and keeps its redirect under the same `mkdir -p target &&` guard |
| (same) | `6c54b1a6` | `just citecheck` (box) | `citecheck: all 131 cited names resolve against the runner-collected inventory`; wrapper exit 0 |
| (same) | `6c54b1a6` | the prose diff of `ci.yml`, `Cargo.toml`, `design/rumors-frame-fuzz.md` | three wrapper mentions re-stated, nothing else; em-dashes in the justfile 57 to 56, `ci.yml` unchanged at 9 |
| (same) | `6c54b1a6` | `just gate` on the box (lane log `gate-box-2.log`) | `audit`, `internal-docs`, `doctest`, `surface`, `board`, `workspace`, `wasm` ok; `fuzz` failed on `FuzzerPlatform.h:72:2: error: #error "Support for your platform has not been implemented"` (the accepted illumos leg); zero `ps` usage errors in the log |
| (same) | `6c54b1a6` | `just mutants-list` and `just bench-build` on the box, the unwrapped recipes `ci` alone reaches | `mutantcheck: all 14 exclusion patterns match their pinned listed/suppressed counts`; `Finished bench profile`; exit 0 |

## Fresh-eyes rounds

One round (surface correctness with operational validity), by sha: no
defect in the change. The reviewer compared all twelve unwrapped recipe
lines token by token against their base forms minus the wrapper,
confirmed `citecheck`'s redirect lands in the same file under the same
guard with the command's own exit status (the recipe shell is `bash
-euo pipefail` at both base and head), read the deleted script to
confirm it passed its child's status through, found no indirect use of
the wrapper anywhere (no variable, helper recipe, workflow step, or
tool), confirmed `just --list` byte-identical, and confirmed the
monomorphization hazard the deleted comment pointed at is stated where
it lives (`src/tree/traverse/act.rs`) with nothing dangling. Three
prose items landed as the lane's last commit: the CI bullet now gives
the real reason bash is required (it is the justfile's recipe shell);
the `Cargo.toml` paragraph loses an unbacked "heaviest" and two
em-dashes; one reflow finished. The fourth item was a wrong
parenthetical in T136 itself, corrected in the record. Reviewer notes
not acted on: wrapped commands used to receive `/dev/null` on stdin and
now inherit `just`'s (no wrapped command reads stdin); the wrapper's
heartbeat lines vanish from the gate stream logs, which nothing
parsed.

## Stops

None reported by the lane. <!-- STOPS -->

## Reading order

### tests and prose

- memwatch-retirement (T136) at `.github/workflows/ci.yml:24` ([hunk](#hunk-2))
- memwatch-retirement (T136) at `.github/workflows/ci.yml:26` ([hunk](#hunk-2))
- fresh-eyes repair (T136) at `.github/workflows/ci.yml:24` ([hunk](#hunk-2))
- memwatch-retirement (T136) at `Cargo.toml:97` ([hunk](#hunk-3))
- fresh-eyes repair (T136) at `Cargo.toml:97` ([hunk](#hunk-3))
- memwatch-retirement (T136) at `design/rumors-frame-fuzz.md:283` ([hunk](#hunk-4))
- fresh-eyes repair (T136) at `design/rumors-frame-fuzz.md:283` ([hunk](#hunk-4))
- memwatch-retirement (T136) at `justfile:101` ([hunk](#hunk-5))
- memwatch-retirement (T136) at `justfile:103` ([hunk](#hunk-5))
- memwatch-retirement (T136) at `justfile:108` ([hunk](#hunk-5))
- memwatch-retirement (T136) at `justfile:117` ([hunk](#hunk-6))
- memwatch-retirement (T136) at `justfile:285` ([hunk](#hunk-7))
- memwatch-retirement (T136) at `justfile:362` ([hunk](#hunk-8))
- memwatch-retirement (T136) at `justfile:388` ([hunk](#hunk-9))
- memwatch-retirement (T136) at `justfile:530` ([hunk](#hunk-10))
- memwatch-retirement (T136) at `justfile:569` ([hunk](#hunk-11))
- memwatch-retirement (T136) at `justfile:589` ([hunk](#hunk-12))
- memwatch-retirement (T136) at `justfile:655` ([hunk](#hunk-13))
- memwatch-retirement (T136) at `justfile:936` ([hunk](#hunk-14))
- memwatch-retirement (T136) at `justfile:1026` ([hunk](#hunk-15))
- memwatch-retirement (T136) at `justfile:1033` ([hunk](#hunk-15))
- memwatch-retirement (T136) at `tools/memwatch:0` ([hunk](#hunk-16))

## The change

<a id="hunk-1"></a>
### .agent-notes/2026-09-01-holistic-review-rumors/triage/annotations/p1-memwatch.tsv `@@ -0,0 +1,24 @@`

```diff
@@ -0,0 +1,24 @@
+# p1-memwatch lane annotations: path, line (new side of `git diff 51ccf03c...HEAD`), entry id, ruling, note.
+# Written by Claude (Fable 5.1) as the lane implementer for Finch's review; not authored or endorsed by Finch.
+tools/memwatch	0	memwatch-retirement	T136	The script itself, deleted whole. Its swap check reads a figure the kernel drains lazily and kills healthy legs; its per-process ceiling never caught a demonstrated runaway. Replacement: nothing, by owner decision; the builder cap per machine is the resource control of record.
+justfile	101	memwatch-retirement	T136	The comment block above `test` went whole: every sentence explained the wrapper, its trigger, or its env override. The monomorphization-bomb pointer to `src/tree/traverse/act.rs` went with it because it existed to justify the watchdog, not to document `test`.
+justfile	103	memwatch-retirement	T136	`test`: wrapper prefix removed; command, arguments, and exit status unchanged.
+justfile	108	memwatch-retirement	T136	`test-all`: wrapper prefix removed; nothing else on the line moved.
+justfile	117	memwatch-retirement	T136	`doctest`: wrapper removed between the RUSTDOCFLAGS assignment and the cargo invocation; the assignment still scopes the same command.
+justfile	285	memwatch-retirement	T136	`citecheck`: the `bash -c` existed only to carry the redirect through the wrapper, so it went with the wrapper; the redirect now sits directly on the command under the same `mkdir -p target &&` guard, same exit-status semantics.
+justfile	362	memwatch-retirement	T136	`fuzz-build`: wrapper removed; the `[working-directory]` attribute and the nightly `fuzz build` line are untouched.
+justfile	388	memwatch-retirement	T136	The `gate-streams` paragraph keeps its one surviving fact (concurrency multiplies peak memory) and drops the three watchdog sentences. The replacement sentence states what is true now: the justfile caps nothing, and the operator chooses how many gates share a machine. Four lines became two.
+justfile	530	memwatch-retirement	T136	`bench-build`: wrapper prefix removed.
+justfile	569	memwatch-retirement	T136	`fuzzfit-build`: wrapper removed from the harness build line only; the guest build line was never wrapped.
+justfile	589	memwatch-retirement	T136	`fuzzfit`: wrapper removed between the FUZZFIT_GUEST_WASM assignment and the nextest run; the assignment still scopes the same command.
+justfile	655	memwatch-retirement	T136	`fuelscape-test`: same shape as `fuzzfit`; wrapper removed, assignment kept.
+justfile	936	memwatch-retirement	T136	`surface-totality`: wrapper removed from the nextest line; the surrounding fmt, clippy, and run lines are untouched.
+justfile	1026	memwatch-retirement	T136	`coverage-kernel`: wrapper removed from the llvm-cov line; covcheck lines untouched.
+justfile	1033	memwatch-retirement	T136	`coverage-kernel-branch`: wrapper removed from the nightly llvm-cov line; covcheck lines untouched.
+.github/workflows/ci.yml	24	memwatch-retirement	T136	The prerequisites bullet now names python3 for the linters and bash for the shell recipes: bash stays a real prerequisite because `gate-streams` is a `#!/usr/bin/env bash` recipe, so dropping it would have made the list false. Named by kind, not by recipe, so the sentence cannot rot when a recipe is renamed.
+.github/workflows/ci.yml	26	memwatch-retirement	T136	The paragraph arguing the wrapper was harmless off macOS is deleted: it argued for something that no longer runs, and its 8 GiB figure was a hypothesis the script itself contradicted (the script defaulted to 32).
+Cargo.toml	97	memwatch-retirement	T136	The `bench = false` rationale keeps its claim (the auto-target is the heaviest compile in the workspace) and loses the parenthetical that measured it against the wrapper. No figure replaces it: an unmeasured number is a hypothesis. The rest of the paragraph is re-wrapped, words unchanged.
+design/rumors-frame-fuzz.md	283	memwatch-retirement	T136	The justfile-wiring bullet loses only its wrapper clause; the toolchain clause it was attached to stays. Lines re-wrapped, words otherwise unchanged.
+.github/workflows/ci.yml	24	fresh-eyes repair	T136	Fresh-eyes round 1: bash is the configured shell of every recipe line (`set shell := ["bash", ...]`), not only the one shebang recipe, so the bullet names it as the justfile's recipe shell.
+Cargo.toml	97	fresh-eyes repair	T136	Fresh-eyes round 1: "heaviest compile in the workspace" rested only on the deleted watchdog observation; the argument beside it (whole crate, every test module, opt-level 3, one rustc) supports "heavy", so the clause says that. The two em-dashes in the re-wrapped paragraph became spaced double hyphens.
+design/rumors-frame-fuzz.md	283	fresh-eyes repair	T136	Fresh-eyes round 1: the reflow is finished through the end of the sentence so no short line remains; words unchanged.
```

*(review record; no annotation expected)*

<a id="hunk-2"></a>
### .github/workflows/ci.yml `@@ -21,11 +21,8 @@`

```diff
@@ -21,11 +21,8 @@
 #   - `just`, `cargo-nextest` (just test), `cargo-rdme` (readme-check), and
 #     `wasm-pack` (viz),
 #   - node + npm for the viz TypeScript typecheck and bundle,
-#   - python3 for the tools/ linters, and bash for tools/memwatch (all
-#     preinstalled).
-# memwatch's swap backstop is sysctl-based and degrades to a no-op off macOS, so
-# wrapping the test/doctest/bench recipes is harmless here; its 8 GiB per-process
-# cap sits well above anything a normal Linux build reaches.
+#   - python3 for the tools/ linters and bash, the justfile's recipe shell
+#     (both preinstalled).
 #
 # Actions are pinned to full commit SHAs with the source ref in a trailing
 # comment (tools/workflowlint enforces the shape; .github/dependabot.yml
```

<!-- annotation -->
> **memwatch-retirement** (T136), line 24:
>
> The prerequisites bullet now names python3 for the linters and bash for the shell recipes: bash stays a real prerequisite because `gate-streams` is a `#!/usr/bin/env bash` recipe, so dropping it would have made the list false. Named by kind, not by recipe, so the sentence cannot rot when a recipe is renamed.

<!-- annotation -->
> **memwatch-retirement** (T136), line 26:
>
> The paragraph arguing the wrapper was harmless off macOS is deleted: it argued for something that no longer runs, and its 8 GiB figure was a hypothesis the script itself contradicted (the script defaulted to 32).

<!-- annotation -->
> **fresh-eyes repair** (T136), line 24:
>
> Fresh-eyes round 1: bash is the configured shell of every recipe line (`set shell := ["bash", ...]`), not only the one shebang recipe, so the bullet names it as the justfile's recipe shell.

<a id="hunk-3"></a>
### Cargo.toml `@@ -92,14 +92,13 @@ rustdoc-args = ["--cfg", "docsrs"]`

```diff
@@ -92,14 +92,13 @@ rustdoc-args = ["--cfg", "docsrs"]
 
 [lib]
 # Keep `cargo bench` from compiling the library itself as a libtest bench
-# harness. That auto-target rebuilds the whole crate — every `#[cfg(test)]`
-# module included — at opt-level 3 in a single rustc invocation, and its
-# codegen peak is the heaviest compile in the workspace (it trips the
-# memwatch per-process ceiling). The harness would run nothing: stable Rust
-# has no bench items, and every real benchmark is a `harness = false`
-# criterion target below. The test modules keep their coverage through the
-# dev-profile test runs; `cargo bench` still builds this library once, as
-# the ordinary rlib the criterion targets link against.
+# harness. That auto-target rebuilds the whole crate -- every `#[cfg(test)]`
+# module included -- at opt-level 3 in a single rustc invocation, a heavy
+# compile. The harness would run nothing: stable Rust has no bench items,
+# and every real benchmark is a `harness = false` criterion target below.
+# The test modules keep their coverage through the dev-profile test runs;
+# `cargo bench` still builds this library once, as the ordinary rlib the
+# criterion targets link against.
 bench = false
 
 [features]
```

<!-- annotation -->
> **memwatch-retirement** (T136), line 97:
>
> The `bench = false` rationale keeps its claim (the auto-target is the heaviest compile in the workspace) and loses the parenthetical that measured it against the wrapper. No figure replaces it: an unmeasured number is a hypothesis. The rest of the paragraph is re-wrapped, words unchanged.

<!-- annotation -->
> **fresh-eyes repair** (T136), line 97:
>
> Fresh-eyes round 1: "heaviest compile in the workspace" rested only on the deleted watchdog observation; the argument beside it (whole crate, every test module, opt-level 3, one rustc) supports "heavy", so the clause says that. The two em-dashes in the re-wrapped paragraph became spaced double hyphens.

<a id="hunk-4"></a>
### design/rumors-frame-fuzz.md `@@ -280,10 +280,10 @@ grows organically thereafter.`

```diff
@@ -280,10 +280,10 @@ grows organically thereafter.
   does.
 - **Justfile wiring, mirroring the before recipes:** `fuzz-build`
   gains (or is joined by a sibling for) the rumors workspace built
-  under `tools/memwatch` with the nightly toolchain; the `fuzz` smoke
-  recipe gains one line running `fuzz_session` for `fuzz_smoke_secs`
-  (20s default) with `corpus/fuzz_session seeds/fuzz_session` named in
-  that order. `ci` keeps building fuzz targets without running them;
+  with the nightly toolchain; the `fuzz` smoke recipe gains one line
+  running `fuzz_session` for `fuzz_smoke_secs` (20s default) with
+  `corpus/fuzz_session seeds/fuzz_session` named in that order. `ci`
+  keeps building fuzz targets without running them;
   `all` inherits the smoke through the existing `(fuzz
   fuzz_smoke_secs)` dependency. The gate is untouched.
 - **Landing obligations:** committed seed corpus plus its derivation
```

<!-- annotation -->
> **memwatch-retirement** (T136), line 283:
>
> The justfile-wiring bullet loses only its wrapper clause; the toolchain clause it was attached to stays. Lines re-wrapped, words otherwise unchanged.

<!-- annotation -->
> **fresh-eyes repair** (T136), line 283:
>
> Fresh-eyes round 1: the reflow is finished through the end of the sentence so no short line remains; words unchanged.

<a id="hunk-5"></a>
### justfile `@@ -98,20 +98,14 @@ default:`

```diff
@@ -98,20 +98,14 @@ default:
 check:
     cargo check --workspace --all-targets
 
-# Codegen-running recipes go through tools/memwatch: a runaway rustc (e.g. a
-# monomorphization bomb — see src/tree/traverse/act.rs) or a runaway test
-# fails the build with the offender named instead of wedging the machine.
-# `check`/`clippy` skip codegen, so they can't detonate one and run bare.
-# Override the limits per-invocation: `PROC_LIMIT_GB=64 just test`.
-
 # Run the test suites; pass a filter to narrow (`just test mirror`).
 test *args:
-    {{ justfile_directory() }}/tools/memwatch cargo nextest run --workspace {{ args }}
+    cargo nextest run --workspace {{ args }}
 
 # The gate's test run: every feature (the meter suites and the conformance
 # module build only here).
 test-all *args:
-    {{ justfile_directory() }}/tools/memwatch cargo nextest run --workspace --all-features {{ args }}
+    cargo nextest run --workspace --all-features {{ args }}
 
 # Stable rustdoc compiles one executable per example; `before` has nearly 100,
 # and their macOS link work dominates the gate. Nightly's merged mode compiles
```

<!-- annotation -->
> **memwatch-retirement** (T136), line 101:
>
> The comment block above `test` went whole: every sentence explained the wrapper, its trigger, or its env override. The monomorphization-bomb pointer to `src/tree/traverse/act.rs` went with it because it existed to justify the watchdog, not to document `test`.

<!-- annotation -->
> **memwatch-retirement** (T136), line 103:
>
> `test`: wrapper prefix removed; command, arguments, and exit status unchanged.

<!-- annotation -->
> **memwatch-retirement** (T136), line 108:
>
> `test-all`: wrapper prefix removed; nothing else on the line moved.

<a id="hunk-6"></a>
### justfile `@@ -120,7 +114,7 @@ test-all *args:`

```diff
@@ -120,7 +114,7 @@ test-all *args:
 
 # Run the doctests (nightly), which nextest does not run.
 doctest:
-    RUSTDOCFLAGS="-Z unstable-options --merge-doctests yes" {{ justfile_directory() }}/tools/memwatch cargo +{{ nightly_toolchain }} test --workspace --doc --all-features --target-dir target/doctest-nightly
+    RUSTDOCFLAGS="-Z unstable-options --merge-doctests yes" cargo +{{ nightly_toolchain }} test --workspace --doc --all-features --target-dir target/doctest-nightly
 
 # Lint every target, warnings denied (the commit-gate setting).
 clippy:
```

<!-- annotation -->
> **memwatch-retirement** (T136), line 117:
>
> `doctest`: wrapper removed between the RUSTDOCFLAGS assignment and the cargo invocation; the assignment still scopes the same command.

<a id="hunk-7"></a>
### justfile `@@ -288,7 +282,7 @@ docs-internal:`

```diff
@@ -288,7 +282,7 @@ docs-internal:
 # collected test inventory.
 citecheck:
     ./tools/citecheck --self-test
-    mkdir -p target && {{ justfile_directory() }}/tools/memwatch bash -c 'cargo nextest list -p before --all-features --message-format json > target/citecheck-tests.json'
+    mkdir -p target && cargo nextest list -p before --all-features --message-format json > target/citecheck-tests.json
     ./tools/citecheck --tests target/citecheck-tests.json --root crates/before
 
 # The mutants exclusion roster (.cargo/mutants.toml) names every mutant
```

<!-- annotation -->
> **memwatch-retirement** (T136), line 285:
>
> `citecheck`: the `bash -c` existed only to carry the redirect through the wrapper, so it went with the wrapper; the redirect now sits directly on the command under the same `mkdir -p target &&` guard, same exit-status semantics.

<a id="hunk-8"></a>
### justfile `@@ -365,7 +359,7 @@ supply-chain:`

```diff
@@ -365,7 +359,7 @@ supply-chain:
 [working-directory("crates/before/fuzz")]
 fuzz-build:
     cargo fmt --check
-    {{ justfile_directory() }}/tools/memwatch cargo +{{ nightly_toolchain }} fuzz build --target {{ host_triple }}
+    cargo +{{ nightly_toolchain }} fuzz build --target {{ host_triple }}
 
 # The gate runs in two tiers. First `gate-lints`, sequential and
 # build-free, because a formatting slip must not cost four minutes to
```

<!-- annotation -->
> **memwatch-retirement** (T136), line 362:
>
> `fuzz-build`: wrapper removed; the `[working-directory]` attribute and the nightly `fuzz build` line are untouched.

<a id="hunk-9"></a>
### justfile `@@ -391,10 +385,8 @@ fuzz-build:`

```diff
@@ -391,10 +385,8 @@ fuzz-build:
 # lets them overlap. `fuzzfit` and `fuelscape-test` share one stream
 # because both build the same wasm guest.
 #
-# Concurrency multiplies peak memory, not just cores: every build-heavy
-# leg runs under tools/memwatch, whose per-process ceiling and global swap
-# backstop are scoped so concurrent instances never kill each other's
-# compiles. A stream that trips it fails loudly and names the crate.
+# Concurrency multiplies peak memory, not just cores. Nothing here caps
+# memory; how many gates share a machine is the operator's call.
 
 # Run the pre-commit gate; it must come up fully clean before every commit.
 gate: gate-lints gate-streams
```

<!-- annotation -->
> **memwatch-retirement** (T136), line 388:
>
> The `gate-streams` paragraph keeps its one surviving fact (concurrency multiplies peak memory) and drops the three watchdog sentences. The replacement sentence states what is true now: the justfile caps nothing, and the operator chooses how many gates share a machine. Four lines became two.

<a id="hunk-10"></a>
### justfile `@@ -535,7 +527,7 @@ viz:`

```diff
@@ -535,7 +527,7 @@ viz:
 
 # Compile (don't run) the criterion benches.
 bench-build:
-    {{ justfile_directory() }}/tools/memwatch cargo bench --workspace --no-run
+    cargo bench --workspace --no-run
 
 # The decode invariant (accepted input re-encodes stably and decodes back to
 # itself) and the `before::laws` law collection are asserted inline in the
```

<!-- annotation -->
> **memwatch-retirement** (T136), line 530:
>
> `bench-build`: wrapper prefix removed.

<a id="hunk-11"></a>
### justfile `@@ -574,7 +566,7 @@ fuzz secs=fuzz_smoke_secs:`

```diff
@@ -574,7 +566,7 @@ fuzz secs=fuzz_smoke_secs:
 [working-directory("crates/before/fuzzfit")]
 fuzzfit-build:
     cargo build -p fuzzfit-guest --release --target wasm32-unknown-unknown --target-dir {{ fuzzfit_target }}
-    {{ justfile_directory() }}/tools/memwatch cargo build -p fuzzfit-harness --tests --release
+    cargo build -p fuzzfit-harness --tests --release
 
 # Run the fuzz-fit suites: generator sanity, meter liveness, the judgment
 # and shape-leg tripwires, the quadratic-burner adequacy check, the
```

<!-- annotation -->
> **memwatch-retirement** (T136), line 569:
>
> `fuzzfit-build`: wrapper removed from the harness build line only; the guest build line was never wrapped.

<a id="hunk-12"></a>
### justfile `@@ -594,7 +586,7 @@ fuzzfit-build:`

```diff
@@ -594,7 +586,7 @@ fuzzfit-build:
 fuzzfit: fuzzfit-build
     cargo fmt --check
     cargo clippy --all-targets -- -D warnings
-    FUZZFIT_GUEST_WASM={{ fuzzfit_guest_wasm }} {{ justfile_directory() }}/tools/memwatch cargo nextest run --cargo-profile release
+    FUZZFIT_GUEST_WASM={{ fuzzfit_guest_wasm }} cargo nextest run --cargo-profile release
 
 # Re-fit the pinned bands from the committed deterministic corpus (4096
 # programs; byte-reproducible, so any diff is a real change). Rewrites
```

<!-- annotation -->
> **memwatch-retirement** (T136), line 589:
>
> `fuzzfit`: wrapper removed between the FUZZFIT_GUEST_WASM assignment and the nextest run; the assignment still scopes the same command.

<a id="hunk-13"></a>
### justfile `@@ -660,7 +652,7 @@ wasm32-pins: wasm32-pins-build`

```diff
@@ -660,7 +652,7 @@ wasm32-pins: wasm32-pins-build
 fuelscape-test: fuzzfit-build
     cargo fmt --check
     cargo clippy --all-targets -- -D warnings
-    FUZZFIT_GUEST_WASM={{ fuzzfit_guest_wasm }} {{ justfile_directory() }}/tools/memwatch cargo nextest run
+    FUZZFIT_GUEST_WASM={{ fuzzfit_guest_wasm }} cargo nextest run
 
 # Renders one log-log heatmap per public operation into target/fuelscape
 # (SVG per op plus a gallery index.html): p(fuel | size) from uniform
```

<!-- annotation -->
> **memwatch-retirement** (T136), line 655:
>
> `fuelscape-test`: same shape as `fuzzfit`; wrapper removed, assignment kept.

<a id="hunk-14"></a>
### justfile `@@ -941,7 +933,7 @@ surface-json:`

```diff
@@ -941,7 +933,7 @@ surface-json:
 surface-totality: surface-json
     cargo fmt --check
     cargo clippy --all-targets -- -D warnings
-    {{ justfile_directory() }}/tools/memwatch cargo nextest run
+    cargo nextest run
     cargo run -q -- {{ justfile_directory() }}/target/surface-json/doc/before.json
 
 # The worst-case map answers "which committed shape is worst for operation
```

<!-- annotation -->
> **memwatch-retirement** (T136), line 936:
>
> `surface-totality`: wrapper removed from the nextest line; the surrounding fmt, clippy, and run lines are untouched.

<a id="hunk-15"></a>
### justfile `@@ -1031,12 +1023,12 @@ covcheck_expected := justfile_directory() + "/tools/covcheck-expected.json"`

```diff
@@ -1031,12 +1023,12 @@ covcheck_expected := justfile_directory() + "/tools/covcheck-expected.json"
 coverage-kernel:
     ./tools/covcheck --self-test
     @mkdir -p target/llvm-cov
-    {{ justfile_directory() }}/tools/memwatch cargo llvm-cov nextest --workspace --all-features --lcov --output-path target/llvm-cov/workspace.lcov
+    cargo llvm-cov nextest --workspace --all-features --lcov --output-path target/llvm-cov/workspace.lcov
     ./tools/covcheck --lcov target/llvm-cov/workspace.lcov --expected {{ covcheck_expected }} --root {{ justfile_directory() }}
 
 # Run the instrumented suite (pinned nightly, --branch) and hold kernel branch coverage to the pin.
 coverage-kernel-branch:
     ./tools/covcheck --self-test
     @mkdir -p target/llvm-cov
-    {{ justfile_directory() }}/tools/memwatch cargo +{{ nightly_toolchain }} llvm-cov nextest --branch --workspace --all-features --lcov --output-path target/llvm-cov/workspace-branch.lcov
+    cargo +{{ nightly_toolchain }} llvm-cov nextest --branch --workspace --all-features --lcov --output-path target/llvm-cov/workspace-branch.lcov
     ./tools/covcheck --branch --lcov target/llvm-cov/workspace-branch.lcov --expected {{ covcheck_expected }} --root {{ justfile_directory() }}
```

<!-- annotation -->
> **memwatch-retirement** (T136), line 1026:
>
> `coverage-kernel`: wrapper removed from the llvm-cov line; covcheck lines untouched.

<!-- annotation -->
> **memwatch-retirement** (T136), line 1033:
>
> `coverage-kernel-branch`: wrapper removed from the nightly llvm-cov line; covcheck lines untouched.

<a id="hunk-16"></a>
### tools/memwatch `@@ -1,219 +0,0 @@`

```diff
@@ -1,219 +0,0 @@
-#!/bin/bash
-# memwatch — run a build/test command under a memory watchdog.
-#
-# Why this exists: a monomorphization bomb (see the comment in
-# src/tree/traverse/act.rs) once made every leaf-crate rustc invocation
-# consume 25+ GiB at codegen, outrunning jetsam and wedging the machine into
-# repeated watchdog kernel panics. Under memwatch, a recurrence fails the
-# build with the offending crate named instead of taking the machine down.
-#
-# Two layers of protection:
-#   1. Per-process: any build-related process (rustc, clippy-driver, build
-#      scripts, test binaries under target/) whose resident size exceeds
-#      PROC_LIMIT_GB is killed, and its rustc --crate-name is logged.
-#   2. Global: if system swap usage exceeds SWAP_LIMIT_GB, this instance's
-#      build is aborted. This is the backstop against compressor
-#      exhaustion.
-#
-# Every kill is scoped to this instance's own process tree: a candidate
-# counts only if its parent chain reaches the command memwatch spawned.
-# Concurrent instances (another worktree's gate running beside a bench
-# build) therefore never kill each other's compiles, whatever their
-# limits. The swap *measurement* is machine-global — swap is — but the
-# response only fails this instance's build; a neighbor still over the
-# line trips its own watchdog.
-#
-# Usage:
-#   memwatch cargo nextest run --workspace --all-features
-#   PROC_LIMIT_GB=64 SWAP_LIMIT_GB=48 memwatch cargo bench ...
-#
-# Events are echoed and appended to $LOG (default memwatch.log under
-# $TMPDIR; the note at the default carries the ownership argument). The
-# swap check is macOS-specific and degrades to a no-op elsewhere.
-
-set -u
-
-# An uncalibrated backstop, generous by design: the per-process limit is
-# defense-in-depth against type-size exponentiality — a height-indexed
-# tower that fails to be boxed and erased in the right place sends codegen
-# memory exponential, and this tripwire exists to catch that runaway, not
-# to budget legitimate builds. A limit low enough to stop a legitimate
-# optimized build is miscalibrated in the direction that costs the most:
-# it reads as a verdict on the code rather than on the threshold. Sized
-# against the development machine's 128 GiB, this leaves a runaway a
-# quarter of memory before it trips. Override per invocation when one
-# needs more.
-PROC_LIMIT_GB="${PROC_LIMIT_GB:-32}"
-SWAP_LIMIT_GB="${SWAP_LIMIT_GB:-24}"
-# The log must never be opened through a path another user could have
-# pre-created or that routine cleanup destroys: a fixed name in a sticky
-# world-writable directory (/tmp) can be planted as a symlink by any
-# local user and macOS follows it on append, while a checkout-relative
-# default dies with a read-only cwd and loses the forensic record to
-# `cargo clean` — the usual recovery step after exactly the kill this
-# tool logs. macOS's $TMPDIR is per-user and mode 0700: only the
-# invoking user can create anything under it, which is the ownership
-# property wanted. Where TMPDIR is unset (non-macOS), the fallback is
-# the invoking directory's own target/; $LOG still overrides for
-# callers who want a shared or per-run log.
-LOG="${LOG:-${TMPDIR:-$PWD/target}/memwatch.log}"
-INTERVAL="${INTERVAL:-2}"
-HEARTBEAT_EVERY="${HEARTBEAT_EVERY:-60}"   # ticks between status lines
-
-lim_kib=$((PROC_LIMIT_GB * 1024 * 1024))   # ps reports rss in KiB
-swap_lim_mb=$((SWAP_LIMIT_GB * 1024))
-
-mkdir -p "$(dirname "$LOG")"
-
-# Every logged line is stripped of control bytes first: argv text from
-# watched processes flows into these lines, and a raw newline (or escape
-# sequence) inside one would mint synthetic records in the forensic log.
-sanitize() { printf '%s' "$1" | tr -d '[:cntrl:]'; }
-
-note() { echo "$(date +%H:%M:%S) memwatch: $(sanitize "$*")" | tee -a "$LOG"; }
-
-"$@" &
-ROOT=$!
-trap 'kill "$ROOT" 2>/dev/null' INT TERM
-
-echo "$(date +%H:%M:%S) memwatch: start pid $ROOT, per-proc ${PROC_LIMIT_GB} GiB, swap abort ${SWAP_LIMIT_GB} GiB: $(sanitize "$*")" >> "$LOG"
-
-# Only these are ever killed by the per-process check.
-is_build_proc() {
-    case "$1" in
-        *rustc*|*clippy-driver*|*build-script-build*|*/target/*/deps/*) return 0 ;;
-        *) return 1 ;;
-    esac
-}
-
-# Emit `pid rss` for every process in this instance's own tree: those
-# whose parent chain reaches $ROOT in one ps snapshot. The snapshot is
-# numeric-only: no byte of any process's command line appears on a
-# parsed line, so no process can forge a record that steers a later
-# signal (ps emits argv bytes raw, so a newline embedded in any
-# process's argv would otherwise mint a synthetic record). Commands are
-# fetched separately, per pid, to filter and describe a candidate these
-# numbers already chose; the candidate's own per-pid snapshot re-checks
-# the limit before any signal. The chain walk is bounded so a torn
-# snapshot (a recycled pid making the ppid graph cyclic) terminates
-# rather than spinning.
-own_procs() {
-    ps -axo pid=,ppid=,rss= | awk -v root="$ROOT" '
-        NF == 3 && $1 ~ /^[0-9]+$/ && $2 ~ /^[0-9]+$/ && $3 ~ /^[0-9]+$/ {
-            parent[$1] = $2; rss[$1] = $3
-        }
-        END {
-            for (p in parent) {
-                a = p
-                for (hops = 0; hops < 128; hops++) {
-                    if (a == root) { print p, rss[p]; break }
-                    if (!(a in parent) || a <= 1) break
-                    a = parent[a]
-                }
-            }
-        }'
-}
-
-tick=0
-while kill -0 "$ROOT" 2>/dev/null; do
-    biggest_kb=0 biggest_pid=
-
-    while read -r pid rss; do
-        # Comparisons and signals run only on pure-digit fields; a record
-        # failing this is dropped (defense in depth behind the numeric-only
-        # snapshot).
-        case "$pid" in '' | *[!0-9]*) continue ;; esac
-        case "$rss" in '' | *[!0-9]*) continue ;; esac
-        if [ "$rss" -gt "$biggest_kb" ]; then
-            biggest_kb=$rss
-            biggest_pid=$pid
-        fi
-        if [ "$rss" -gt "$lim_kib" ]; then
-            # Re-read rss and command in one per-pid snapshot: the digits
-            # decide and the text only filters and describes, with both
-            # read from the same occupant of the pid, so a pid recycled
-            # since the tree snapshot cannot borrow the old occupant's
-            # size (the new occupant must itself be over-limit and
-            # build-shaped to be signalled). The residual window is the
-            # irreducible kill(2)-by-pid race between this snapshot and
-            # the signal: the scoping promise is per-snapshot, not atomic.
-            snap=$(ps -o rss=,command= -p "$pid")
-            rss_now=$(printf '%s\n' "$snap" | awk 'NR == 1 { print $1 }')
-            case "$rss_now" in
-                '' | *[!0-9]*)
-                    # The over-limit sighting is still logged, so a
-                    # runaway that dies between snapshot and fetch stays
-                    # attributed instead of vanishing without trace.
-                    note "over-limit pid $pid at $((rss / 1048576)) GiB resident exited before identification"
-                    continue
-                    ;;
-            esac
-            [ "$rss_now" -gt "$lim_kib" ] || continue
-            cmd=$(printf '%s\n' "$snap" | sed '1s/^[[:space:]]*[0-9]*[[:space:]]*//')
-            is_build_proc "$cmd" || continue
-            crate=$(printf '%s' "$cmd" | sed -n 's/.*--crate-name \([A-Za-z0-9_]*\).*/\1/p')
-            note "KILL pid $pid at $((rss_now / 1048576)) GiB resident, crate=${crate:-unknown}"
-            echo "    full cmd: $(sanitize "$cmd")" >> "$LOG"
-            kill -9 "$pid"
-        fi
-    done < <(own_procs)
-
-    swap_used_mb=$(sysctl -n vm.swapusage 2>/dev/null | awk '{print $6}' | tr -d 'M' | cut -d. -f1)
-    if [ "${swap_used_mb:-0}" -gt "$swap_lim_mb" ]; then
-        note "ABORT: swap at ${swap_used_mb} MB exceeds ${SWAP_LIMIT_GB} GiB; killing this build"
-        # Scoped like the per-process check: this build's own tree, never
-        # a neighbor's. Freeze before killing: a stopped process cannot
-        # fork, so no compile spawned between enumeration and the kill
-        # survives orphaned. The root freezes first (no new children),
-        # then descendants until a pass finds nothing new — a child forked
-        # before its parent froze is caught by the next pass. The bounded
-        # passes cover any build-shaped tree (cargo's spawn depth is a few
-        # levels); only a fork-storm outrunning five enumerations escapes,
-        # and the bound keeps a torn ps snapshot from spinning. Should
-        # memwatch itself die mid-freeze, the traps thaw whatever is
-        # stopped so the build wedges nothing (a SIGKILL of memwatch
-        # itself is untrappable; that one path still strands the tree).
-        thaw() {
-            for pid in ${frozen:-}; do
-                kill -CONT "$pid" 2>/dev/null
-            done
-        }
-        trap 'thaw; kill "$ROOT" 2>/dev/null; exit 1' INT TERM
-        trap 'thaw' EXIT
-        kill -STOP "$ROOT" 2>/dev/null
-        frozen=" $ROOT "
-        for _ in 1 2 3 4 5; do
-            new=0
-            while read -r pid _rss; do
-                case "$pid" in '' | *[!0-9]*) continue ;; esac
-                case "$frozen" in *" $pid "*) continue ;; esac
-                kill -STOP "$pid" 2>/dev/null
-                frozen="$frozen$pid "
-                new=1
-            done < <(own_procs)
-            [ "$new" -eq 0 ] && break
-        done
-        for pid in $frozen; do
-            kill -9 "$pid" 2>/dev/null
-        done
-        trap - EXIT INT TERM
-        exit 1
-    fi
-
-    if [ "$tick" -gt 0 ] && [ $((tick % HEARTBEAT_EVERY)) -eq 0 ]; then
-        # The description is fetched at heartbeat time, by pid: display
-        # only, never a signal target. A proc that exited since the
-        # snapshot describes as its bare pid.
-        biggest_desc=none
-        if [ -n "$biggest_pid" ]; then
-            biggest_cmd=$(ps -o command= -p "$biggest_pid")
-            biggest_desc="pid $biggest_pid ${biggest_cmd:0:80}"
-        fi
-        note "status: largest tracked proc $((biggest_kb / 1048576)) GiB ($biggest_desc); swap ${swap_used_mb:-0} MB"
-    fi
-    tick=$((tick + 1))
-    sleep "$INTERVAL"
-done
-
-wait "$ROOT"
-exit $?
```

<!-- annotation -->
> **memwatch-retirement** (T136), line 0:
>
> The script itself, deleted whole. Its swap check reads a figure the kernel drains lazily and kills healthy legs; its per-process ceiling never caught a demonstrated runaway. Replacement: nothing, by owner decision; the builder cap per machine is the resource control of record.

