<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from rulings T89, T90, T91, and T132 (T84's and T115's rows named, not landed) in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P4 lane: residue of three removals and one rewrite

## Goal

Nothing remains from a removal or a rewrite. The V1 retirement and the
two erasures left manual impls a derive now equals, bounds a supertrait
implies, `repr`s and `cfg` gates that guard nothing, a V1 skip in a
tool, a `V2` series label that distinguishes one dialect (T89), an
assertion implied by the equality before it and the feature it needed
(T90), a hand-rolled temp dir (T91), a fully shadowed dependency; the
`send_all` rewrite left braces scoping nothing. Each goes, and the
common-trait derives the crate's public types are owed land (widening
only, on T84's precedent). Effort: high (production derives and
manifest edits, each mechanical and judged by the suites).

## Ground rules

These apply to every P4 lane; the lane sections below add to them and
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
  run in the background redirected to a log under `<scratchpad>/p4-leftovers/`,
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

1. **Braces, one commit.** Oracle, empty:
   `rg -U -n '\{\s*\n\s*[^\n{}]*send_all\([^\n]*;\s*\n\s*\}' src tests`;
   `just fmt-check`. Known-bad: the tree at base (eleven files).
2. **Manual impls to derives, one commit per module family.** Oracle:
   `git grep -n -E '^impl(<[^>]*>)? (Clone|Default|PartialEq|Eq) for' -- src tests`
   lists only impls whose comment states the reason (the crate's
   convention at `bootstrap.rs:81-83`, `path.rs:61`, `prefix.rs:202`);
   the report lists every survivor. Known-bad: a derive that differs
   from the impl it replaces changes behavior the suites observe; name
   the suite per type in the commit.
3. **Erasure residue**: materialized-12 (`+ Sync`), tree-typed-10,
   remote-adapter-tests-19, tests-disruption-handshake-27,
   tests-observation-13, verification-infra-11, tests-wire-format-28 and
   tests-common-10 (the allow narrows to `dead_code`).
4. **Lint attributes**: inventory-12, module-graph-11.
5. **Manifest commits**: deps-4, T89, T90, T91; `Cargo.lock` regenerated
   in each, `--locked` thereafter (T155).
6. **Common-trait derives**: api-core-19, session-bookmark-3,
   session-bookmark-39.
7. **Closing commit**: every oracle above, plus
   `git grep -n -E '\+ Sync|B: Sync' -- 'src/tree/mirror/streaming/materialized*'`,
   `git grep -n 'repr(C)' -- src/tree/typed/height.rs`,
   `git grep -n encoded_bits -- src tests benches Cargo.toml`,
   `git grep -n futures_util -- src`, `grep -n 'V1\|│' tools/digestshare`,
   all empty, and `just clippy` clean with `#![allow(dead_code)]` alone
   on `tests/common/mod.rs`.

## Members

Heading entries quote Resolution and Acceptance verbatim from the topic document; nit rows quote their table row (site, issue, resolution). Each line opens with the entry's primary site at `9e5784fb`; the full record, with its related sites and evidence, is under `### <id>:` in the document the ledger's `doc` column names. Rulings are cited by number; T132 is the roster approval for every low and nit.

- **benches-envelope-2** (low; T89; T89: discard the saved local criterion baselines and say so in the commit). `benches/gossip_fixed.rs:9-10`. Resolution: Use `BenchmarkId::from_parameter(param)` at 154 and `BenchmarkId::new(format!("divergence={param}"), latency_ms)` at 190, drop the parenthetical at 10, and note the baseline discontinuity in the commit; if the slot is deliberately reserved for a future dialect, say so in one sentence instead. Make the latency sweep table carry its group name directly (`(Scenario, &str, &[usize])`) and delete `latency_group_name`, or sweep the unilateral scenarios under latency and list them in the module doc. Acceptance: `grep -n '"V2' benches/` returns nothing (or the doc states the reservation); no function in the file returns a group name that no group is registered under.
- **tests-observation-35** (low; T90; T90: both assertions and the `meter` feature go; the commit names the superseded ruling in 05d87e1b). `tests/party_conservation.rs:326-335`. Resolution: If the owner agrees: delete the `encoded_bits` assertions and `baseline_bits` (314, 331-335, 377-381); reword module-doc item 4 (29-32) to state the size bound as a consequence of bit-for-bit return ("so the encoded size cannot grow with churn"); drop `"meter"` from the `before` dev-dependency and its three-line comment in Cargo.toml. If the owner keeps them: add one sentence at 331 stating that the size check is kept as an implementation-independent statement of the bound. Acceptance: either `grep -rn encoded_bits src tests benches examples Cargo.toml` is empty and `just gate` passes with `before = { workspace = true, features = ["serde"] }` under `[dev-dependencies]`, or the retained assertion carries its rationale at the site.
- **tests-wire-format-21** (low; T91; T91: `tempfile` joins `[dev-dependencies]`; regenerate `Cargo.lock` in the same commit). `tests/snapshot_liveness.rs:258-309`. Resolution: Add `tempfile` to dev-dependencies and hold a `tempfile::TempDir` in `FixtureTree`, dropping the constructor's cleanup and the `Drop` impl; share the remaining `file`/`sweep` shell between the two sweeps through a small path-included module (`#[path = "support/fixture_tree.rs"] mod fixture_tree;`) so neither binary compiles `tests/common`. Acceptance: one definition of `FixtureTree` holding a `TempDir`; no `remove_dir_all` in either sweep.
- **api-core-19** (nit; T132). ``src/peer/bootstrap.rs:96-104``: `Bootstrap`'s `Debug` omits the `observe` setting Resolution: Add `.field("observe", &self.observe)`
- **conformance-15** (nit; T132). ``src/conformance/link.rs:964-971``: Manual `Clone` impls identical to the derive, in a file whose other connectors derive it Resolution: Replace both with `#[derive(Clone)]`
- **conformance-21** (nit; T132). ``src/conformance/link/tests.rs:1049-1057``: Vestigial braces around `send_all`, and qualified paths where the production sibling imports Resolution: Remove the braces
- **deps-4** (low; T132). `Cargo.toml:139-140`. Resolution: Replace `futures_util::` with `futures::` at the six sites; remove `futures-util` from rumors' `[dependencies]` and from `[workspace.dependencies]`. Acceptance: `grep -rn futures_util src/` is empty; the workspace table has no `futures-util` row; `cargo tree -p rumors -e normal --depth 1` lists futures and not futures-util.
- **inventory-12** (nit; T132). ``src/tree/mirror/streaming/remote/streams.rs:423-432``: A `too_many_arguments` allow on a seven-parameter function Resolution: Delete the attribute
- **link-19** (nit; T132). ``src/link/routed/endpoint.rs:145-151``: Hand-written `Clone` impls where `#[derive(Clone)]` produces the same impl Resolution: replace both with `#[derive(Clone)]` on the struct definitions (endpoint.rs:141, stream.rs:24)
- **materialized-12** (low; T132). `src/tree/mirror/streaming/materialized.rs:610`. Resolution: Delete the twelve bounds (and the sibling sites elsewhere under `streaming/` in the same pass). Acceptance: `grep -rn '+ Sync\|B: Sync' src/tree/mirror/streaming/materialized*` returns nothing; `just check` clean.
- **module-graph-11** (nit; T132; if AGENTS.md at base already records that every `mod tests;` carries `#[cfg(test)]` (the P3 modules lane), keep the gates and quote the line; else delete the six). ``src/conformance.rs:18-19``: Redundant `#[cfg(test)]` gates under test-only parents, and a `pub(crate)` that widens nothing Resolution: Either delete the six inner gates or record once, in AGENTS.md's testing section, that every `mod tests;` carries `#[cfg(test)]` regardless of context
- **remote-codec-22** (low; T132). `src/tree/mirror/streaming/remote/codec/frame.rs:92-112`. Resolution: `#[derive(Default, Clone, PartialEq, Eq)]` on `LeafRun`, keeping the custom `Debug`. Acceptance: `frame.rs` has no `impl Default/Clone/PartialEq/Eq for LeafRun`; the codec tests pass.
- **session-bookmark-19** (nit; T132). ``src/peer/gossip/tests.rs:162-166``: Test helpers carry braces left over from removed `.batch(..)` closures, and one helper doc promises a value the function does not return Resolution: Remove the three brace pairs
- **session-bookmark-3** (nit; T132). ``src/peer/gossip.rs:165-167``: `Gossiped` lacks `PartialEq`/`Eq` although every field is `Eq`; `NoBookmark` derives only `Debug` Resolution: `#[derive(Debug, Clone, PartialEq, Eq)]` on `Gossiped`
- **session-bookmark-39** (nit; T132; land the five identity types now; `SessionInfo` only if `Protocol` already derives `Hash`/`Ord` at base (T84, P6)). ``src/observe.rs:129-131``: Observer identity types derive `PartialEq, Eq` but not `Hash`/`Ord`, so they cannot key a per-stream table Resolution: Derive `Hash, PartialOrd, Ord` on the five identity types now; on `SessionInfo` once `Protocol` derives them
- **streaming-backend-window-5** (low; T132). `src/tree/mirror/streaming/backend.rs:374-383`. Resolution: Replace the manual impl with `#[derive(Debug, Clone)]` on `Root<B>` and delete the comment; if the derive fails for a reason other than `T`, state that reason at the impl. At line 51, drop the clause or name what the wire produces ("decode as the peer's message type at the wire boundary"). Acceptance: no bare `T` in backend.rs prose; `Root<B>` is `Clone` by derive, or by a manual impl whose comment is true of today's code.
- **tests-common-10** (low; T132; narrow the allow here; T115 moves the module later). `tests/common/mod.rs:31-33`. Resolution: narrow the allow to `dead_code`; if rustc then warns on the `pub use` re-exports in schedule/mod.rs:19-26 or gossip_snapshot.rs:94, allow `unused_imports` on those items only. Delete `Protocol` and `readout_multiset` from the imports; drop `ByteMeter.read` (the field, and its clone into the meter at fault.rs:107) and make the doc at 78-80 singular; delete `let _ = self.label;`; delete `Peer::observations()` and read the field at tests/sanity.rs:84,92; unwrap the braces; write `Peer::<u64>::seed().into_rumors()` in `divergent_pair`, or make the seed's window a parameter. Acceptance: `#![allow(dead_code)]` alone stands at mod.rs:33 with `just clippy` clean; none of the seven items remains.
- **tests-common-5** (nit; T132). ``tests/common/fault.rs:189-196``: hand-written `Clone` where `derive` is identical Resolution: `#[derive(Clone)]` on `FaultConnector`
- **tests-disruption-handshake-27** (low; T132). `tests/handshake_liveness.rs:370-375`. Resolution: Inline each shape body into its `#[test] fn` inside `block_on(async { ... })`, keep one merged doc comment per test, replace "matrix" with the list it is, and drop "V2" from the cell docs (the dialect's name belongs where wire bytes are spelled). Acceptance: one function and one doc comment per session shape; `grep -c V2 tests/handshake_liveness.rs` is 0.
- **tests-disruption-handshake-6** (nit; T132). ``tests/disruption.rs:707-709``: Bare blocks around single send_all statements Resolution: Replace each block with the bare statement at the four sites
- **tests-lifecycle-13** (nit; T132). ``tests/pairwise.rs:30-37``: Vestigial leftovers: a one-line alias of `bootstrap_fork`, bare brace blocks that scope nothing, and window configuration in a suite that never gossips Resolution: Replace `dup(..)` with `bootstrap_fork(..)` and delete the fn
- **tests-observation-13** (nit; T132). ``tests/observe.rs:350-352``: The `Protocol::V2` assertion is vacuous with a one-variant enum Resolution: Drop line 351 and the words "and protocol" from the doc at 316
- **tests-observation-16** (low; T132). `tests/listen.rs:79-81`. Resolution: Unwrap the braces and re-flow to one line at listen.rs:79-81, 629-632, 653-656; session_stats.rs:262-268, 309-316; stale_floor.rs:35-37. Acceptance: none of the six sites has a bare `{` preceding a lone `send_all`; `just fmt-check` still passes.
- **tests-resource-link-window-13** (low; T132). `tests/routed_link.rs:76-90`. Resolution: Write one generic `endpoint<D: Dial>` and one `establish<D>(b, a_incoming, addr, dialer_first)`, and one `conformance_under(pair)` wrapper used by all eight conformance tests; drop the braces (and consider naming 48/96 so line 249's `96` is derived). hop_trace.rs:550-552 carries the same braces. Acceptance: one spelling of the establishment tail and one of the timeout wrapper; no single-statement bare blocks in the file.
- **tests-wire-format-28** (low; T132). `tests/common/wire.rs:14`. Resolution: Remove `Protocol` from the import; drop `unused_imports` from the allow at mod.rs:33 and fix whatever else `just clippy` then reports. Acceptance: `just clippy` is clean with `#![allow(dead_code)]` alone on tests/common/mod.rs.
- **tests-wire-format-8** (nit; T132). ``tests/gossip_snapshot.rs:505-511``: Braces left behind by the send_all conversion Resolution: Remove the braces
- **tree-core-4** (low; T132). `src/tree.rs:131-135`. Resolution: Change `Root`'s attribute to `#[derive(Clone, Debug, PartialEq, Eq)]` and delete lines 131-135; delete the three `where T: Send + Sync` clauses. Leave `Tree<T>`'s manual `Clone`/`PartialEq`/`Default` (137-156), which the phantom justifies. Acceptance: the crate compiles; the production callers `batch.rs:143` and `peer/gossip.rs:869` are untouched; no method in `tree.rs` names a bound its body does not use.
- **tree-typed-10** (nit; T132). ``src/tree/typed/height.rs:13-14``: `#[repr(C)]` on the zero-sized height markers Resolution: remove both attributes
- **tree-typed-11** (low; T132). `src/tree/typed/height.rs:79`. Resolution: reduce the trait to `pub trait Height: sealed::Sealed + 'static { const HEIGHT: usize; }` and run `just check`; the manual impls then do what their comments say. If the compile reveals a consumer, keep the bounds and rewrite the three comments to say the manual impls exist for `Copy` (and, for `S<T>`, the reason at height.rs:16-21). Acceptance: either `Height` names no `Debug + Clone + Default` and `just check` is clean, or the comments at path.rs:61, prefix.rs:202, and node.rs:23-39 state a true reason.
- **tree-typed-24** (low; T132). `src/tree/typed/untyped.rs:103-111`. Resolution: `#[derive(Clone)] struct NodeInner`; `#[derive(Debug, Clone)] enum Children`, moving the memo-carrying comment at untyped.rs:171-173 above the derive; `#[derive(Default, Clone)] pub struct Fan`. Acceptance: the three manual blocks are gone; `just check` and the untyped and fan proptests pass unchanged.
- **verification-infra-11** (low; T132). `tools/digestshare:20-23`. Resolution: delete the `│` skip, the `measure` docstring's "or None" clause, and the header paragraph; keep the liveness failure (a corpus with captures but zero parsed bytes) as the tool's only non-threshold exit. Acceptance: `grep -n 'V1\|│' tools/digestshare` is empty and `just digestshare` passes.

## Named here, landed elsewhere

These rows belong to this lane's pattern and are counted in its roster, but another ruling's lane lands them; this lane verifies the state at base, quotes it in the report, and edits nothing at their sites.

- **benches-envelope-22** (low, T115): T115 (owner decision 71, P7 with the P5 harness lane): the allows leave when `benches/support` becomes a crate; not landed here.
- **link-5** (low, T84): T84 (P6, three widening additions): the `Debug` impls land with `api-core-1` and `api-core-30`; not landed here.
- **tests-disruption-handshake-28** (low, T115): T115: the `#[path]` inclusions dissolve with the crate move; not landed here.

## Hazards and stops

- tests-disruption-handshake-6's sites (`tests/disruption.rs`) may be
  gone under T143: quote the absence.
- `tests/common/mod.rs` and `tests/routed_link.rs` are edited by the
  unmerged `p2-vanish-liveness` and `p2-link`: after their merges.
- session-bookmark-3 and -39 widen public types; any derive that would
  change a public type's existing behavior (a `PartialEq` where one
  exists) is a stop.
- `tests-lifecycle-13` also deletes the `dup` alias; the `pairwise.rs`
  window setting it names is a P5 question, untouched here.
