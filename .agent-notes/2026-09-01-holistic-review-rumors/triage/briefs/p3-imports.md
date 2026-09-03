<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from ruling T52 in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P3 lane: import grouping on the pinned nightly

> **Ruled after drafting.** T158 item 7: land the option and this crate's reflow; the `crates/` reflow is a separate final commit after the `before` triage's open lanes merge, announced in the merge queue first.

## Goal

Every file's imports sit in one layout, std, then external crates, then
this crate, produced by the formatter and held by `fmt-check` so no hand
sweep is ever needed again: `rustfmt.toml` gains
`group_imports = "StdExternalCrate"`, `fmt` and `fmt-check` run on the
pinned nightly the gate already names (`nightly_toolchain` in the
justfile; stable rustfmt ignores the option with a warning, so the stable
recipe would hold nothing), and the first run is the sweep (T52). The
thirty rows are the hand half: qualified paths merged into imports, stray
`serde` lines folded into their groups, glob and function-local imports
made explicit, one `Inner` alias dissolved. Effort: medium (a formatter
run plus mechanical merges; nothing behavioral).

## Ground rules

These apply to every P3 lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal `<base sha>` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `<base sha>`, fast-forward; if it has diverged, stop and report. Never call
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
  run in the background redirected to a log under `<scratchpad>/p3-imports/`,
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
- **Prose.** `PROSE.md` in this directory binds this lane (ruling T141): every
  paragraph you touch passes its three tests (altitude, concision,
  legibility), and the diff is net shorter in prose unless your report says
  what the added sentences buy.
- **Machine.** The illumos box (`ox-east-1`, per the `building-on-illumos` skill) is
  where a lane builds, tests, and gates. The wrapper syncs the Mac
  worktree to `~/src/<worktree basename>` on the box and runs one command
  there with its own target directory, so lanes do not collide; cargo
  runs `--locked` there; nothing is edited or committed on the box. A
  clean gate on the box is the gate of record for a commit; the Mac runs
  no gate (Finch's ruling). The box gate is `on-illumos.sh <worktree> 'just gate'`. One leg is
  expected red there and counts as clean when it is the only failure:
  `fuzz`, because libFuzzer has no illumos port (`FuzzerPlatform.h`
  refuses the target); a lane quotes that line and runs no fuzz build
  elsewhere (Finch's ruling: fuzzing is CI's). clippy's
  `missing_const_for_thread_local` misfires on illumos, where
  `thread_local!` expands through the OS-keyed path; `before` carries an
  illumos-scoped crate-level allow, so a lane based before that landed
  either rebases or passes `RUSTFLAGS="-A clippy::missing_const_for_thread_local"`
  in the remote command for that one run. Two legs that
  pin toolchain-derived numbers may fire on the box if its toolchains
  differ from the pinned ones; a lane reports such a leg with both numbers
  rather than re-pinning anything. `tools/memwatch` is deleted by the
  `p1-memwatch` lane, which runs first and unstacked; `p1-gate` rebases
  onto it. Benchmarks whose committed baselines are the Mac's run on the
  Mac, once, on a quiet machine. Clock guard, checked before every box
  run: rsync preserves mtimes and cargo's rebuild detection is
  mtime-based, so a box clock ahead of the Mac by more than a couple of
  seconds means a green build of stale code; on skew, a lane either runs
  with a fresh target directory on the box (a cold build, no stale
  artifact to trust) or waits, and says which. Stepping the box's clock is
  admin work on a shared machine and is Finch's, never a lane's. The
  builder cap counts Mac builders only; on the box (96 cores, 1 TiB) there
  is no lane cap, only the load: hold a launch while the one-minute load
  average sits above about 150 on 192 threads, and keep wall-time
  measurements under `pset-run` to one at a time, announced in the merge
  queue first.
- **Out of scope.** The formal tier (`lean`, `eventdag`, `muxprobe`, everything under
  `formal/`) and `before`'s bench judge (`bench-judge`,
  `bench-judge-tripwire`) are never run or edited by a rumors lane; a
  recipe that composes them (`all`) is exercised by its other legs
  individually. A rustdoc on a Rust-side literal derived from the Lean
  artifact is Rust prose and may be edited where a ruling names it.

## Mechanism, in order

1. **The configuration and recipes, one commit.** `rustfmt.toml` at the
   workspace root with the one option and a comment saying why it needs
   the nightly; `fmt` becomes `cargo +{{ nightly_toolchain }} fmt --all`
   and `fmt-check` its `--check` twin. CI: the nightly install step gains
   the `rustfmt` component (at base, `p1-gate`'s workflow pins the nightly
   from the justfile; confirm the pin reads the same name). On the box:
   `rustup run <nightly_toolchain> rustfmt --version` before the gate run;
   a missing component is a stop, never a fallback to stable.
2. **The formatter sweep, one commit, nothing else in it.** `just fmt`
   once. It reformats every crate in the workspace: a read-only dry run at
   `6c90bd7d` (`cargo +nightly-2026-06-30 fmt --all --check -- --config group_imports=StdExternalCrate`)
   moves 171 files, 80 under `src/`, 15 under `tests/`, 76 under `crates/`.
   The `crates/` files belong to the `before` triage, whose lanes are in
   flight; this commit is kept separate so the coordinator can re-run it
   after any rebase (a formatter commit is cheap to redo) and announce it
   in `.agent-notes/merge-queue.md` before it merges.
3. **The hand rows, one commit per file family** (crate root and observers;
   tree; mirror common and streaming tests; remote adapter, codec, proxy;
   testing scaffold; integration suites), each followed by `just fmt-check`
   clean. A merged import must not change what a name resolves to: build
   after each commit (`cargo check --workspace --all-targets --all-features`).
4. **Negative control, recorded in step 1's commit message:** a reversible
   mis-grouping (a `std` import moved below a `crate` import in one file)
   and the `just fmt-check` diff line it produced, verbatim.

Oracle for the lane: `just fmt-check` clean on the box; for the qualified
paths, each row's site read against its quoted Resolution; the diff of the
sweep commit contains only moved `use` lines.

## Members

Nit rows are one-line table rows in the topic document (`| <id> |` under
its module heading), quoted as `Resolution:` with no Acceptance of their
own; heading entries quote both. All rows are T52 (owner decision 7)
except the last.

- **api-core-31** (nit). Resolution: `use crate::Inner;` in the three observer files and `use crate::MERKLE_HASH_LEN;` in snapshot.rs (read with inventory-15 and module-graph-10: the root alias goes and the sites import `crate::peer::Inner`; `MERKLE_HASH_LEN` becomes crate-private under T97 later, so import it by its current path.)
- **clippy-pedantic-15** (nit). Resolution: `use std::collections::BTreeSet;` at the file head
- **clippy-pedantic-5** (nit). Resolution: explicit lists at traverse.rs:7 and materialized.rs:185 (`use common::children_of;`)
- **inventory-15** (nit). Resolution: Remove the root alias; import `crate::peer::Inner` at the nine use sites and the bookmark.rs doc link
- **materialized-9** (nit). Resolution: Import `PhantomData`, `Stream`, and `pin`
- **mirror-common-18** (nit). Resolution: `use std::pin::pin;` in driver.rs and tasks.rs (six more redundant `Future` imports sit outside the partition)
- **mirror-common-7** (low, simplification). Resolution: Add `?Sized` to both bounds; `use crate::tree::mirror::framing::read_payload;` in party.rs and call `read_payload(reader, len)`. Acceptance: party.rs:116 reads `read_payload(reader, len)`; the other callers (greeting.rs:273, decode/async_io.rs:430 and :460, testing.rs:111) are unchanged.
- **module-graph-10** (nit). Resolution: Add `use crate::peer::Inner;` to `causal.rs`, `changes.rs`, and `unordered.rs` and drop the inline qualifications
- **remote-adapter-streams-18** (nit). Resolution: Import `io`, `mem`, `Arc`, `Mutex`, `AsyncRead`, and `cbor` at file top; spell the label capacity with two `head_len` terms; fix the import groups in decode.rs, pump.rs, work.rs, and tests.rs
- **remote-adapter-tests-5** (nit). Resolution: One `use crate::{..}` tree per file in std / external / crate / super order; no item between imports; no `super::super::` or function-local `use`
- **remote-capture-atlas-26** (nit). Resolution: In codec/tests.rs:265-301 use `cbor::write_head`/`cbor::MAJOR_UINT`/`cbor::MAJOR_ARRAY`
- **remote-capture-atlas-30** (nit). Resolution: Regroup as std, external (`serde`, `tokio`), `crate`, `super`
- **remote-codec-13** (nit). Resolution: Extend the existing `use` blocks (`SUPPLY_FRAME_OVERHEAD`, `RECORD_TAG_LEN`, `lone_record_spans`, `VERSION_TAG`, `std::io`) and drop the qualified spellings
- **remote-proxy-11** (nit). Resolution: fold `PayloadCodec` into each file's `use crate::{...}` block
- **remote-proxy-tests-3** (nit). Resolution: Move the `crate::message` import into each file's crate group
- **session-bookmark-2** (nit). Resolution: Fold each into the main import block (`use serde::{Serialize, de::DeserializeOwned};`) and restore the blank line before the doc comment
- **session-bookmark-41** (nit). Resolution: Import `Pin`, `Context`, `Poll`, `AsyncRead`, `ReadBuf`, `fmt`, `io`, `Ordering`, `Infallible`; use the in-scope `Value` and `CLOCK_TAG` in the test
- **streaming-tests-21** (nit). Resolution: Add the imports at file top, merge the split groups, and put a blank line after fixtures.rs:17
- **testing-infra-3** (nit). Resolution: Hoist the std block to the top of testing.rs
- **tests-common-1** (nit). Resolution: Merge the serde pair into the external-crate import group and restore the blank line before the first item, at all seven sites
- **tests-common-26** (nit). Resolution: import at the top of each file and drop the two redundant annotations
- **tests-disruption-handshake-29** (nit). Resolution: Fold the serde imports into the sorted block and add the blank line (here and at the two tests/common sites)
- **tests-lifecycle-12** (nit). Resolution: Import `Peer` and `RangeInclusive` where qualified; fold the serde lines into the import group; use `Waker::noop()`; drop the scare quotes
- **tests-observation-21** (nit). Resolution: Switch the two files to `use crate::common::`
- **tests-resource-link-window-17** (nit). Resolution: Merge the imports into the main block and add the blank line
- **tests-wire-format-10** (nit, owner decision 7). Resolution: Fold each stray serde line into the existing serde import and restore the blank line, in one sweep over the 22 sites
- **tests-wire-format-24** (nit). Resolution: Import `block_on`, `Arc`, `AtomicBool`, `AtomicUsize`, `Error`, and the `observe` items at the top of payload_depth.rs
- **tree-core-19** (nit). Resolution: Give `insert_at` two parameters; inline or re-document `naive_max_version_bytes`; `use super::arb;` once; move the serde import into the block
- **tree-typed-15** (nit). Resolution: `use super::prefix::Prefix;` beside the other `super::` imports
- **tree-typed-4** (nit). Resolution: `use std::fmt;` (and `use std::cmp::Ordering;` where needed) per file, then `fmt::Formatter<'_>`, `fmt::Result`, `Ordering::Equal`
- **swarm-example-25** (nit, T27, moot). Resolution: Rename the counters struct (`Counters` or `Sample`), import `rumors::Snapshot`, and import the other qualified items. Disposed by the example's deletion (`p1-swarm`); the lane's only act is `test ! -e examples/swarm.rs` at base, quoted in the report.

## Hazards and stops

- The sweep touches every file the unmerged P1 and P2 lanes edit and
  every file the other P3 lanes edit; this lane launches last in P3, after
  `p3-dashes`, and its step 2 is redone on every rebase.
- `crates/before` reflows under step 2. The `before` session's in-flight
  lanes (`before/p1-harness`, `before/p2-surface`, `before/p2-widths` at
  the time of writing) will conflict in their import blocks; the merge
  queue decides the order, and this lane's report names the `crates/`
  file count so the other session can plan its rebases.
- `imports_granularity` and every other unstable rustfmt option stay out:
  T52 names one option. A merge that changes what a name resolves to is
  reported with the file and left as the formatter leaves it.
