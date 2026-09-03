<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from rulings T132 in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P4 lane: hazard headings, module docs, rewraps, and one home per argument

> **Ruled after drafting.** T159 item 9: no doclint heading-vocabulary rule; rename the one `# Cancellation` and align the remaining headings by hand, listing them in the report.

## Goal

Rustdoc has one shape: hazards sit under the crate's named sections
(`# Cancel safety`, `# Errors`, `# Panics`), every multi-item file opens
with a `//!`, every doc paragraph is wrapped to its measure, and each
argument has one home that the other sites link to. The one mechanical
piece is a `tools/doclint` rule holding the section vocabulary. Effort:
medium.

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
  run in the background redirected to a log under `<scratchpad>/p4-docshape/`,
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

1. **Hazard headings, one commit.** Extend `tools/doclint` with a
   heading-vocabulary rule: every `# ` heading inside `///` or `//!`
   must be one of the crate's section names (derive the list at base
   with `git grep -h -E '^\s*(///|//!)\s*# ' -- src | sort | uniq -c`,
   state the rule beside it). Self-test cases: `# Cancellation` fails,
   `# Cancel safety` passes. Then api-core-28, api-audit-16 (the
   `# Errors` sections; do not add `clippy::missing_errors_doc`: T47
   fixed the lint table, and a new lint is a ruling), link-4,
   mirror-common-4, tree-typed-27. Known-bad: the tree at base fails the
   new rule at `src/rumors.rs:563`.
2. **Module docs, one commit.** `git ls-files 'src/*.rs' 'src/**/*.rs' | xargs grep -L '^//!'`
   at base; document every multi-item file (module-graph-15,
   materialized-29, mirror-common-30, remote-proxy-18,
   streaming-backend-window-17, api-core-6's three field docs); the
   report lists each single-type file left bare with its reason (no
   standing check: module-graph-15 exempts them by judgment).
3. **Rewraps, one commit**: the listed sites. Oracle over the member
   files, empty: a `///` line of five words or fewer followed by a
   `///` line of sixty characters or more
   (`rg -U -n '^\s*///\s+(\S+\s+){0,4}\S+\n\s*///\s+.{60,}'`); no
   standing check, since rustfmt does not reflow comments.
4. **One home per argument**: api-core-27, materialized-32,
   remote-adapter-streams-8, remote-proxy-10, streaming-backend-window-4,
   tree-typed-13; oracle per entry (`grep -c` of the phrase is one).

## Members

Heading entries quote Resolution and Acceptance verbatim from the topic document; nit rows quote their table row (site, issue, resolution). Each line opens with the entry's primary site at `9e5784fb`; the full record, with its related sites and evidence, is under `### <id>:` in the document the ledger's `doc` column names. Rulings are cited by number; T132 is the roster approval for every low and nit.

- **api-audit-16** (nit; T132). `src/rumors.rs:563 (and the `# Errors`-less methods listed)`: Hazard headings are inconsistent: `# Cancellation` beside `# Cancel safety`, and `send`, `send_all`, `gossip`, `gossip_when`, and `Bootstrap::join` carry error contracts with no `# Errors` heading. Resolution: Rename the heading; add `# Errors` above the existing prose; consider `clippy::missing_errors_doc`.
- **api-core-27** (nit; T132). `src/rumors.rs:451-481`: The session promise with its three exceptions is stated in full on both `Rumors::gossip` and `Link`. Resolution: Cut `Rumors::gossip` to a summary linking `Link#what-a-session-promises`.
- **api-core-28** (nit; T132). `src/rumors.rs:563`: `# Cancellation` where the crate's nine other hazard sections say `# Cancel safety`. Resolution: Rename.
- **api-core-3** (nit; T132). `src/error.rs:6 and nine related lines`: Rewrap residue: one over-long or stub-short line mid-paragraph at ten sites. Resolution: Rewrap each paragraph to the surrounding width.
- **api-core-6** (nit; T132; the three field docs in `src/error.rs` only; every other error doc is `p4-placement`'s (remote-codec-19, materialized-18)). `src/error.rs:73-80, 233`: `MagicMismatch`, `VersionMismatch`, and `IntentInvalid` carry undocumented fields while every sibling documents each field. Resolution: One-line field docs.
- **benches-envelope-7** (nit; T132). `benches/gossip_grid.rs:14-16`: A stray one-word doc line. Resolution: Re-wrap.
- **link-4** (low; T132). `src/link.rs:220-227`. Resolution: add `# Cancel safety` to `Connector::connect` stating the retention discipline and the empty-stream residue, and mirror it in the contract's cancellation clause (link.rs:64-68), which today speaks only of `accept`. Add `# Cancel safety` to `Endpoint::link` stating the peer-side residue (a delivered link whose control stream is already closed; the peer's first session fails as transport failure and it re-links). The routed test's doc can then cite the clause it exercises. Acceptance: both sections present; `just doclint` clean.
- **materialized-29** (low; T132). `src/tree/mirror/streaming/materialized/work.rs:3-5`. Resolution: Add one-sentence `//!` docs to answer.rs ("The per-question answers: merge-joins of both listings at internal, leaf-parent, and leaf heights, where disputes and shed leaves are counted"), resolver.rs, and error.rs (common.rs dissolves under materialized-16; if it stays, document it and the cfg split's reason: the test receiver is itself a `Stream`, channel.rs:3-6). Extend work.rs:3-7 to name all five children. Doc `Resolver::new`, `ready`, `pending`, and `finish` (with the `UnfinishedReply` condition). Acceptance: every non-test `.rs` in the partition opens with a `//!` block; work.rs links all five submodules; every `pub fn` in resolver.rs has a doc comment.
- **materialized-32** (low; T132). `src/tree/mirror/streaming/materialized/work/assembly.rs:29-31`. Resolution: Keep the full arguments at `assembly_level_returns` and `outgoing_responses`; reduce assembly.rs:29-31 and work.rs:130-133 to one sentence each ending in a link to the constructor; end the module-doc paragraph at materialized.rs:79-87 with the same link. Acceptance: each mechanism ("full fan", "one buffered response") is spelled out at exactly one constructor; the other sites link there.
- **mirror-common-30** (low; T132). `src/tree/mirror/streaming/protocol.rs:24-31`. Resolution: One sentence apiece stating height, input, output, and successor, e.g. `Connect`: "The client's opening phase at the root: produce our greeting and the state that awaits the peer's."; `Reply`: "One descent step at `Self::Height`: consume the peer's replies at this height and yield ours one height down."; `Peer`/`Client`/`Server`: "The full chain from a connected state to a terminal, spelled out so the compiler holds `mirror_connected`'s schedule to it."; `Handshaken`: "Both participants past the greeting exchange, plus the election keys."; `Greeting::version`: "The sender's set version; equal versions end the session at the greeting." Acceptance: every `pub` or `pub(crate)` trait, struct, and field in protocol.rs, protocol/peer.rs, streaming.rs, and message.rs has a doc comment whose first sentence stands alone (a review check: `missing_docs` ignores private items).
- **mirror-common-4** (low; T132). `src/tree/mirror/cbor.rs:219-224`. Resolution: cbor.rs: move the sentence under `# Cancel safety`. handshake.rs:264: add `# Cancel safety` ("Cancel safe: bytes received before a drop stay in `self`, and the next call resumes from them.") and state the return arms. Acceptance: every async reader in the partition that is not cancel safe, and `Staged::fill`, carries a `# Cancel safety` section.
- **module-graph-15** (nit; T132). `src/tree/mirror/streaming/materialized/common.rs:1 (27 files)`: Twenty-seven non-test files open without a `//!` module doc. Resolution: One-sentence `//!` for the multi-item files; leave the single-type files.
- **remote-adapter-streams-8** (nit; T132). `adapter/decode.rs:492-496, 460-461; adapter/error.rs:84-98`: The version-bound rationale is stated three times. Resolution: Keep it on the variant; a one-line pointer at the check.
- **remote-proxy-10** (low; T132). `src/tree/mirror/streaming/remote/proxy/state.rs:264-270`. Resolution: keep the first sentence of each typestate impl doc and replace the body with what the impl alone decides (which streams are bound; that `early` is armed here) plus a link to the `Work` method that carries the mechanism. Collapse the codec doc to one sentence at `Work::codec` reading "this replica's `Peer` payload codec" (or code-font `Peer`), drop the explicit link line, and make the other three sites one-line pointers. Acceptance: each mechanism paragraph has one home in the partition; `grep -rn "peer's payload codec" src` is empty; `rg -n 'PayloadCodec\]: crate::message' src` is empty.
- **remote-proxy-18** (low; T132). `src/tree/mirror/streaming/remote/proxy/work/progress/trace.rs:8-15`. Resolution: add a one-line `//!` ("Test-only trace of the proxy's progress-critical publications."), document each `Kind` variant (e.g. `WireReply { questions }`: "one complete wire reply flushed; `questions` publications must follow before the next at this height"), document `new_work`/`record`, and change "height-1 scopes" to "height-1 questions" at trace.rs:68 and trace/tests.rs:99. Acceptance: trace.rs opens with a module doc; every `Kind` variant and pub fn has a doc comment; "scopes" in this file refers only to `NextScope` publications.
- **session-bookmark-14** (nit; T132). `src/peer/gossip.rs:901-904 and eight related sites`: Prose mechanics: a misplaced comment, a mid-doc link definition, ragged wraps, "self-inverse" for invertible, a stale cross-reference, two grammar slips. Resolution: Fix each as the entry lists.
- **session-bookmark-33** (low; T132). `src/reconciliation.rs:53-57`. Resolution: Add "its payload depth limit" with one clause ("the two limits must match, or the session fails before any descent") and re-wrap. Acceptance: the list names every field of `Greeting`; no line exceeds the surrounding measure.
- **streaming-backend-window-17** (nit; T132). `src/tree/mirror/streaming/channel/instrumented.rs:16-25`: No module doc; `RoleStats`'s fields, the vocabulary of the capacity assertions, are undocumented. Resolution: One line per field and a two-sentence `//!`.
- **streaming-backend-window-4** (nit; T132). `src/tree/mirror/streaming/backend.rs:115-117 and the sites listed`: One hazard stated three times, two phrases twice; a `///` doc on an anonymous `const`. Resolution: One home for each phrase; `//` on the const.
- **tests-bookmark-5** (nit; T132). `tests/bookmark_attach.rs:75-77 (twelve sites)`: Doc paragraphs left as a fragment line after the first-sentence split. Resolution: Re-wrap.
- **tests-disruption-handshake-24** (nit; T132). `tests/handshake_liveness.rs:56-59 (thirteen sites)`: Ragged doc paragraphs after the first-sentence split. Resolution: Reflow, keeping the blank `///`.
- **tests-observation-4** (nit; T132). `tests/causal.rs:184-189 (fourteen sites)`: Ragged wraps left by the first-sentence split. Resolution: Re-wrap.
- **tree-typed-13** (low; T132). `src/tree/typed/node.rs:116-123`. Resolution: keep the full argument once, on `S` in height.rs, and have `Node`, `Path`, and `Prefix` say "`PhantomData<fn() -> H>`; see [`S`]" (the owner last re-justified the node.rs copy at 8f87ddd01; if that wording is preferred, move it to `S`). For each typed wrapper, one sentence stating the typed contract plus a link to the untyped method of record; make `floor`'s doc "The dual of [`ceiling`]" and keep the memo paragraph once. Acceptance: "Function pointers are unconditionally `Send + Sync`" occurs once under src/tree/typed/; each typed wrapper doc is a few lines and links its untyped counterpart.
- **tree-typed-27** (low; T132). `src/tree/typed/untyped.rs:260-279`. Resolution: add `# Panics` to both docstrings: on an empty run, on a `None` slot, and (release) on duplicate paths, each with the one-line reason the wire path cannot produce it (the decoder rejects non-ascending and out-of-scope leaves; `assemble` builds non-empty runs); note that ordering and bareness are debug-asserted. Acceptance: both `from_sorted_leaves` docstrings carry a `# Panics` section matching the `expect` sites.

## Hazards and stops

- `src/error.rs`'s variant docs beyond api-core-6's three fields are
  `p4-placement`'s (remote-codec-19, materialized-18): launch after it
  or rebase; the same for `materialized.rs` and `state.rs` prose, which
  the code lanes refactor (small hunks; rebase).
- session-bookmark-33 adds a greeting field to a list: prose only; the
  greeting itself is unchanged.
- The doclint rule is shared with the `before` triage's docs only if
  its roots include `crates/`; keep the recipe's roots as they are and
  say so in the merge queue.
