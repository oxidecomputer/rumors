<!-- AMENDED under ruling T155 (2026-09-02): this lane gains one member beyond its roster rows: every `cargo` invocation in the justfile's gate and `ci` recipes takes `--locked`, and a committed check in the lint tier fails on any that lacks it (allow list: recipes that legitimately update the lock, named). Acceptance: `git grep -n 'cargo ' justfile` shows `--locked` on every invocation outside the allow list; the check's self-test; a negative control removing one flag fails the check by naming the recipe; and a demonstration that a lagging lock fails the gate by name (add a dev-dependency without updating the lock in a scratch tree; `just check` refuses). -->

<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from rulings T47, T54, and T55 in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P3 lane: the `[lints]` table, the allow layers, and the open-enum rule

## Goal

One place says which lints the crate holds itself to, and the gate holds
them: a `[lints]` table in `Cargo.toml` carries the levels two crate
attributes carry today (`#![forbid(unsafe_code)]`, `#![deny(clippy::large_futures)]`)
plus T47's seven, riding `just clippy`'s `-D warnings`, so a regression is
a build failure. The lane also leaves one layer of `type_complexity` allow
under `streaming/` (T54) and states once, in `src/error.rs`, which public
enums are open and which closed, each closed enum saying so at its definition (T55). Effort: high.

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
  run in the background redirected to a log under `<scratchpad>/p3-lints/`,
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

1. **Enumerate before the table**, one run captured whole to
   `<scratchpad>/p3-lints/enumeration.log`:
   `cargo clippy -p rumors --all-targets --all-features -- -W clippy::elidable_lifetime_names -W clippy::redundant_closure_for_method_calls -W clippy::manual_let_else -W clippy::match_wildcard_for_single_variants -W missing_debug_implementations -W unnameable_types -W missing_docs`.
   The report states the count per lint per target (the table applies to
   lib, tests, benches, and examples alike).
2. **Sweep the four clippy lints to zero** per the rows below, one commit
   per lint naming its rows.
3. **Land the table**: `[lints.rust]` and `[lints.clippy]` in the rumors
   `[package]` manifest (never `[workspace.lints]`; `crates/before` has its
   own triage), every entry at `warn`, plus `unsafe_code = "forbid"` and
   `large_futures = "deny"` moved out of `src/lib.rs` with their comments.
   `just clippy` and `just clippy-default` clean. A `warn` in the table is
   an error under the recipes' `-D warnings`: that is T47's promotion and
   the check that fires on a regression. A lint enters the table at zero
   or not at all; an `allow` entry is never a way past a remainder.
4. **Negative control per lint**, in the table commit's message: a
   reversible mutation (the wildcard arm restored at `untyped.rs`, a
   closure respelled, a `match` put back, a lifetime named once, one Debug
   impl dropped) and the `just clippy` failure line it produced, verbatim.
5. **The three rustc lints stop after step 1.** Part of each remainder is
   P6's: `unnameable_types` names `typed::Iter` in `Snapshot::iter` (T60);
   `missing_docs` the codec constructors T63 narrows;
   `missing_debug_implementations` the twelve types T84 gives impls.
   Report the remainder per lint with the ruling owning each item.
   Recommendation: land T84's impls here (their shape is ruled), write docs
   for public items no P6 ruling reshapes, and add each rustc lint only at
   zero; T47 places API-decision sweeps in P6 and the rest here.
6. **T54**: delete every item-level `type_complexity` allow under
   `src/tree/mirror/streaming/` and `protocol.rs`'s inner allow; the
   module-root allow at `streaming.rs` stays, its comment naming the reason
   (the phase-schedule types). Oracle: `git grep -n type_complexity -- src/tree/mirror/streaming`
   prints the root line only. Allows outside `streaming/` are outside T54.
7. **T55**: the rule in `src/error.rs`'s module doc; `#[non_exhaustive]` on
   `EncodeError` (`message.rs`), `SendError` (`remote/streams.rs`), the
   adapter's `EncodeError<E>`, `EncodeErrorKind`, `DecodeLeafError`,
   `LeafRunError`, `DecodeSignalError`; `EndpointError` and `LinkError`
   classified in the same commit; one comment at every deliberately closed
   public enum naming the rule. Oracle: the commit message lists every
   `pub enum` reachable from the public surface with its class; every
   doctest and `matches!` test compiles; no snapshot moves. The ruling
   names no standing check for the rule, and none lands here.

## Members

Nit rows are quoted from their table row (`| <id> |`); heading entries carry Acceptance.

### T47 (owner decision 2)

- **api-audit-12** (low, simplification). Resolution: add manual `Debug` impls printing a summary independent of the type parameters (`Link { session: .. }`, `UnorderedMessages { checkpoint: .. }`, `Changes { seen: .. }`, `Batch { queued: .. }`), and enable `#![warn(missing_debug_implementations)]` so the class stays closed. Acceptance: `cargo clippy -p rumors --lib -- -D warnings` passes with `#![warn(missing_debug_implementations)]` in lib.rs. (T47: the level lives in the table; the impls are step 5's stop.)
- **api-audit-7** (low, documentation). Resolution: add `#![warn(missing_docs)]` to lib.rs (the clippy leg's `-D warnings` then enforces it) and write the missing sentences, or make the item `pub(crate)` where the sentence would be "internal" (api-audit-14 covers the constructors). Replace both `[u8; 6]` with `[u8; MISMATCH_PREVIEW_LEN]` and either export the constant or document the field's width in words. Acceptance: `cargo clippy -p rumors --lib --all-features -- -D warnings` passes with `#![warn(missing_docs)]` in lib.rs. (The constructors are T63's; the `MISMATCH_PREVIEW_LEN` half lands here.)
- **api-core-34** (low, simplification): the `Iter` question is T60's (P6); only the `unnameable_types` enumeration is this lane's. Resolution: History already chose opacity, so the consistent completion is to delete the `pub use` with its doc and the `IntoIterator` impl (callers write `snapshot.iter()`). The alternative, exporting `Iter` at the crate root and returning it from `iter()` per std's naming convention, reverses 8dc0596ed and is the owner's to weigh. Acceptance: either `IntoIterator for &Snapshot` and snapshot.rs:4-7 are gone and the doctests using `snapshot.iter()` still compile, or `rumors::Iter` appears in the public index and `Snapshot::iter` names it; in both cases no public doc says "tree internals".
- **tree-core-5** (low, api): the same question, T60's. Resolution: Owner decision between (a) re-exporting `Iter` at the crate root, having `Snapshot::iter` return `Iter<'_, T>` concretely, and adding `Debug`/`Clone` where `typed::Iter` supports them and `FusedIterator` if `typed::Iter` keeps returning `None` after exhaustion; or (b) keeping opacity everywhere by boxing or wrapping `into_iter`'s type so no public signature names a hidden type. (a) matches std and 8dc0596ed's goal can be met by keeping `typed::Iter` private inside the wrapper. Acceptance: `Snapshot::iter` and `(&snapshot).into_iter()` have the same type; if that type is public, `cargo doc` lists it and a doctest stores it in a struct field by name.
- **clippy-pedantic-1** (low, simplification). Resolution: `frame.rs:455-458`: keep the `major` check, then `let radix = u8::try_from(head.value).map_err(|_| ListingIssue::Shape("listing key is not a radix"))?;`. `frame.rs:438-446`: `usize::try_from(count).ok().filter(|c| *c <= MAX_QUERY_CHILDREN).ok_or(ListingIssue::Shape(..))?`. `format.rs:360-363`: `usize::try_from(declared).ok().filter(|d| *d <= bytes.len() - reader.at).ok_or(FormatError::Truncated { len: bytes.len() })?`, then `reader.take(len)` with its `expect` gone. `iter.rs:411-413`: one arm, `Children::Branch { children, .. } => u8::try_from(level.next).ok().and_then(|next| children.successor(next)).map(..)`; line 429 `u16::from(radix) + 1`. `header.rs:248-254`: the minimal change is `bytes.push(u8::try_from(addr.len()).expect("advertised-name length is validated at endpoint construction"))` with the `debug_assert!` kept for its lower bound; the types-first change is a validated-length newtype for the encoded advertised name that `Endpoint` constructs at line 205 and `link_header` accepts, which deletes both the assert and the cast. All behavior-preserving on valid input. Acceptance: no `as` narrowing remains at the five sites; `just gate` clean; the wire and bookmark snapshot suites unchanged. (The `header.rs` site is link-25's, T44, landed by `p2-link`: verify it at base and land the other four. No lint in T47's seven holds them; report the count `clippy::cast_possible_truncation` would enumerate, as data.)
- **clippy-pedantic-2** (low, simplification). Resolution: `&self` at pump.rs:175, 277, 372; levels.rs:343, 541, 646; state.rs:75. At adversarial.rs:101 `cx: &Context<'_>` compiles (`Context::waker` takes `&self`), but `&mut Context<'_>` is the universal poll-adjacent convention; an owner taste call, see Open questions. This lint is nursery and has false positives around closure and async captures, so apply site by site and compile; a site that fails as `&self` is a lint false positive to leave in place. Acceptance: each site is `&self` or carries a one-line note naming the capture that needs `&mut`; `just gate` clean. (Leave `adversarial.rs:101` as `&mut Context`; the lint is not among T47's seven and is not enabled.)
- **clippy-pedantic-10** (nit). Resolution: `Children::Branch { .. } => None` in place of the wildcard arm; the three test sites over crate enums likewise
- **clippy-pedantic-3** (nit). Resolution: `impl<T: Send + Sync + 'static> DoubleEndedIterator for Iter<'_, T>` and likewise at 196
- **clippy-pedantic-4** (nit). Resolution: `.map(Root::hash)`
- **clippy-pedantic-6** (nit). Resolution: delete the second `Eq` at lines 115, 125, 142
- **clippy-pedantic-9** (nit). Resolution: Write both bindings as `let .. else`, the crate's idiom at 107 sites; the example and test sites likewise
- **inventory-18** (nit). Resolution: carried by link-25 (T44, `p2-link`; nothing to land beyond verifying the site at base).

### T54 (owner decision 9)

- **inventory-3** (low, simplification). Resolution: Keep one layer. Either delete the twelve production per-item allows (and the `clippy::type_complexity` half of levels.rs:341) plus the three test-file allows under `streaming/`, or drop the module-wide allow and keep per-item allows where each complex type lives. Acceptance: `just clippy` and `just clippy-default` stay clean with a single layer of `type_complexity` allow under `streaming/`. (T54 picks the first arm.)
- **materialized-8** (nit). Resolution: Delete the twelve per-item `type_complexity` allows (or the module-wide one), the two inner `#[cfg(test)]`, and reword progress.rs:106
- **mirror-common-29** (nit). Resolution: Delete protocol.rs:10

### T55 (owner decision 10)

- **api-audit-10** (nit). Resolution: State the rule in the `error` module doc; name `EncodeError`'s class at its definition.
- **link-17** (nit). Resolution: rule crate-wide which public enums are open taxonomies
- **materialized-17** (nit, api): the exhaustiveness half only. Resolution: Owner decision. Add `Clone, PartialEq, Eq` to both `MaterializedError` and `RemoteError` (or drop `Clone` from `mirror::Error`), and decide `#[non_exhaustive]` for both, recording the reason in each doc comment. Acceptance: `MirrorError: Clone + PartialEq` compiles, or `mirror::Error` no longer claims `Clone`; the exhaustiveness choice is stated at both enums.
- **remote-adapter-streams-22** (nit, api). Resolution: Either add `#[non_exhaustive]` to `SendError` (and `EncodeError<E>` if the same reasoning holds) or state at each declaration that its variant set is closed and why. Acceptance: every public error enum in `crate::error` either carries `#[non_exhaustive]` or a declaration-site sentence saying why it is closed.
- **remote-capture-atlas-35** (low, api). Resolution: State the rule once where the taxonomy is assembled (this module's doc or `crate::error`'s) and mark each deliberately closed enum with a one-line comment at its definition ("closed by design: a wire-grammar vocabulary; a new variant is a protocol change"). Separately decide `DecodeSignalError` and `LeafRunError` against the recorded criterion. Acceptance: every `pub enum` reachable from `crate::error` either carries `#[non_exhaustive]` or a closed-by-design comment; the rule is stated in one place the enums can cite.
- **remote-codec-18** (low, api). Resolution: State the rule once, in `src/error.rs`'s module doc or beside the first open enum ("taxonomies that grow as enforcement grows are `#[non_exhaustive]`; wire-grammar and contract-outcome enums are closed"), and decide `EncodeErrorKind`, `DecodeLeafError`, and `LeafRunError` under it. Acceptance: the rule is readable from the tree; each public error enum in the codec is consistent with it.
- **session-bookmark-46** (low, api). Resolution: Add `#[non_exhaustive]` to `EncodeError`. Acceptance: the attribute is present; the doctest at rumors.rs and the `matches!` tests still compile.

## Hazards and stops

Step 5 is a named stop; nothing else waits on it. `Cargo.toml` and
`src/lib.rs` are edited by `p1-gate`, `p1-memwatch`, `p1-swarm`,
`p1-renderer`, and `p1-envelope`; `header.rs` and `endpoint.rs` by
`p2-link`: launch only after every P1 and P2 lane has merged, and T63 (P6)
reads the tree this lane leaves. `#[non_exhaustive]` breaks an exhaustive
match in a doctest (an external crate): fix the doctest, never drop the attribute.
