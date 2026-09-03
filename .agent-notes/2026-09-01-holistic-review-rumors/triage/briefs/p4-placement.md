<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from rulings T85, T92, T93, T95, T97, T99, T100, T102, T103, T128, and T132 (T96's row named, not landed) in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P4 lane: public rustdoc at its reader's altitude, and the owner's placement rulings

> **Ruled after drafting.** T159 item 4: T97 and T99 ride whichever of this lane and the P6 API lane launches first.

## Goal

Every public page names only what its reader can reach and sits at
that reader's altitude: the operator sizing guide moves to an
explanation module beside `reconciliation` (T92); the CBOR evolution
tests stop attributing rules to docs that do not state them (T93); the
trust-model text names the one non-member adversary, in Finch's words
(T95); `Snapshot::hash` and `MERKLE_HASH_LEN` leave the public surface
(T97); one enum names the election (T99); `link.rs`'s two owner
passages are restated (T100); the `Backend` family addresses the
maintainer with no mention of the future (T102); the window module's
three placements land (T103); every public error variant has one doc
line (T85, T128); and every file path, test name, and private item
leaves public rustdoc (T132). Effort: high (public rustdoc; two ruled
public-surface edits).

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
  run in the background redirected to a log under `<scratchpad>/p4-placement/`,
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

1. **The sizing page** (api-audit-9, api-core-15, fresh-eyes-3,
   api-audit-8), one commit: a docs-only public module beside
   `reconciliation`; the setter keeps its contract and one link.
2. **The window module** (T103: streaming-backend-window-24, -27, -10)
   and the `Backend` family (T102), one commit.
3. **Error docs** (remote-codec-19's prose half, materialized-18),
   then the altitude rows (conformance-3, mirror-common-1,
   prose-hygiene-5, session-bookmark-36, testing-infra-1,
   tests-disruption-handshake-20), then T100's two passages.
4. **T93**: the two `cbor_evolution` rows.
5. **T97**, one commit: gate `Snapshot::hash` and `Tree::hash`'s public
   face on `any(test, feature = "test-internals")`, make
   `MERKLE_HASH_LEN` crate-private, then tree-typed-2's sentence.
6. **T99**, one commit: one public election enum; every site uses it.
7. **T95 last**: draft the reconciliation-page and crate-doc wording and
   the `hash.rs` trim, then stop; the wording is Finch's.

Oracle: `git grep -n -E '^\s*(///|//!).*\btests/' -- src` hits only
private docs; `grep -n "tests/\|_matches_the_solve\|SCOPE_ENVELOPE_BYTES\|Tree::join" src/peer.rs src/tree/mirror/streaming/window.rs src/reconciliation.rs`
hits no public doc line; `grep -c 'PROGRESS.md' justfile` is 0; the
standing check for links is `just docs` (`-D warnings`, which already
denies `private_intra_doc_links`). A backticked private name has no
mechanical check: that half is a one-time pass with its site list in
the report.

## Members

Heading entries quote Resolution and Acceptance verbatim from the topic document; nit rows quote their table row (site, issue, resolution). Each line opens with the entry's primary site at `9e5784fb`; the full record, with its related sites and evidence, is under `### <id>:` in the document the ledger's `doc` column names. Rulings are cited by number; T132 is the roster approval for every low and nit.

- **remote-codec-19** (low; T85; the prose half (T85); the `Origin` constructors' narrowing is T63's). `src/tree/mirror/streaming/remote/codec/error.rs:97-102`. Resolution: One line per variant stating what separates it from its neighbors (`Read`: "The transport failed while the frame's `part` was being read."; `Truncated`: "The transport closed cleanly before the frame's `missing` component arrived."; `TrailingBytes`: "Bytes followed the frame where the input was required to end."; likewise for `EncodeErrorKind`, `FramePart`, `Speaker`, `StreamClass`, `DecodeSignalError::Placement`). Rewrite error.rs:99-100 as "Produced when a supplied record is decoded, after the run's structure has already been validated at the wire ([`LeafRunError`])." and error.rs:150-153 as "The frame's charged wire size: its run body plus a fixed frame-head envelope, which may exceed the frame's actual size by a few bytes of head slack." Document or narrow to `pub(crate)` the `Origin` constructors. Acceptance: every public variant and public method in error.rs and signal.rs has a doc comment; no public doc in error.rs names a type, function, or constant that `rumors::error` does not export.
- **api-audit-8** (low; T92). `src/peer.rs:391-394`. Resolution: in the public docs, state the provenance without the path ("measured; pinned by test", as link.rs already does) and keep the numbers with their validity bands; move the measurement narrative to maintainer docs (a private module doc or the note under `.agent-notes/2026-07-22-sync-budget/`). Replace the private-constant names with the quantities they denote. Acceptance: `grep -rn 'tests/' src --include='*.rs' | grep -E '^[^:]+:[0-9]+:\s*(///|//!)'` returns hits only in private docs (`src/testing.rs`, test-module docs), and no public doc names an item `cargo doc` does not render.
- **api-audit-9** (low; T92). `src/peer.rs:308-470`. Resolution: move "# Choosing a budget" and the table into a public explanation module beside `reconciliation` (or a section of it), leave the method doc with the contract plus one pointer, and have `DEFAULT_SYNC_MEMORY_BUDGET` and reconciliation.rs:233-235 point at the new page. Acceptance: the method doc states the contract and links to the sizing page; the sizing page renders the table.
- **api-core-15** (low; T92). `src/peer.rs:308-465`. Resolution: Replace "storage backend" and "in-memory backend" with "per disputed subtree in flight" (here and at 527); replace each test-file and test-fn citation with the stable claim ("the crate's tests pin the envelope and the crossover") or drop it; state the wire-buffer bound once, in the "# What this does not bound" list, and have the opening paragraph point there. Acceptance: no `tests/*.rs` path or test fn name appears in public rustdoc in the partition; "backend" appears in no public doc; the bound is stated once.
- **fresh-eyes-3** (medium; T92). `src/peer.rs:393-457`. Resolution: Keep the contract, the closed form with its accuracy band, the worked answers, and the table. Replace each "(`tests/x.rs`)" with the claim's status alone ("measured", "pinned by test") and move the file names to the tests' own doc comments or to a maintainer comment on the constant each pins. Replace "the storage backend's own cost function" and "under the in-memory backend" with what the user can see (per disputed subtree in flight, at the default) or cut. window.rs:273-274: drop the parenthetical naming `SCOPE_ENVELOPE_BYTES` from the public constant's doc (a `//` maintainer comment beside the constant may keep it). reconciliation.rs:248- 249: cut the oracle sentence from the public page (it is a test-suite fact). Acceptance: `grep -n "tests/\|_matches_the_solve\|SCOPE_ENVELOPE_BYTES\|Tree::join" src/peer.rs src/tree/mirror/streaming/window.rs src/reconciliation.rs` matches only lines that are not `///` or `//!` docs of public items.
- **tests-wire-format-14** (medium; T93; T93: option (b); the test routes through `Peer`, or the exception is stated at the site). `tests/cbor_evolution.rs:216-251`. Resolution: Route the tolerated case through `exchanged::<Narrow, WideDefaulted>` (the filled default must arrive over a session) and the rejected case through the failing-bootstrap shape `undecodable_payload_fails_bootstrap_cleanly` already uses (a `Peer<Wide>` bootstrapping from a `Narrow` donor returns an error, never a panic, and moves nothing). If the owner chooses option (b) of tests-wire-format-9 and wants to keep a serializer-level pin, say so at the test with a comment naming it as the one deliberate exception. Acceptance: the test contains no direct `ciborium::` call and both branches run through `Peer`/`Rumors`, or the exception is stated at the site.
- **tests-wire-format-9** (medium; T93; T93: option (b): the crate doc does not promise the rules; the test docs state what each test pins). `tests/cbor_evolution.rs:12-14`. Resolution: Owner's call. (a) Add the two rules back to lib.rs's compatibility paragraph (they answer the first evolution question a user asks, "may I add a field?", and the tests already pin them), keeping the test docs. (b) If the crate does not want to promise serde's field semantics, reword the module doc to "the rules serde's name-keyed decoding gives" and drop "documented" from line 217. Whichever way, see tests-wire-format-14 for the test body. Acceptance: either lib.rs names both rules, or no doc in the file says the crate documents them.
- **session-bookmark-34** (low; T95; STOP after drafting: T95's wording is Finch's trust model and goes to him for read before it lands). `src/reconciliation.rs:164-177`. Resolution: Restore the ruling's framing: keep the structural claim (message bytes contribute zero bits to any compared digest, a property of the construction) and mark the birthday-floor sentence explicitly as an off-model aside that is not part of the acceptance ("Off-model note: ..."), or move it after the hostile-peers sentence so the paragraph's argument visibly ends at the accident bound. Acceptance: the section's acceptance rests on the per-comparison accident bound alone, and any adversary sentence is labeled as outside the model.
- **tree-typed-3** (medium; T95; STOP after drafting, with session-bookmark-34: one derivation, stated once, in Finch's words). `src/tree/typed/hash.rs:39-43`. Resolution: preferred: cut lines 26-47 down to what is local to the type (a Merkle hash is an equality probe between subtrees at one prefix; a false-equal's cost and the width that prices it live in `crate::reconciliation#twenty-four-byte-digests`), keeping the SP 800-107 sentence at 19-24 and the link. Minimum: rewrite the Grinding bullet to match reconciliation.rs: content contributes zero bits by construction, and the 2⁹⁶ floor is the unconditional bound against an actor steering which versions are created. Acceptance: the crate states the 24-byte pricing derivation once, and `Hash`'s doc asserts nothing about content authors that lines 100-102 contradict.
- **tree-typed-2** (nit; T97; T97 first gates `Snapshot::hash` and un-exports `MERKLE_HASH_LEN` (a ruled public-surface edit); then the sentence). `src/tree/typed/hash.rs:6-12`: The partition's only public rustdoc opens its second paragraph with a verbless fragment. Resolution: Proposed sentence in the entry; owner-gated as user-facing prose.
- **remote-codec-32** (nit; T99; T99: `Speaker` and `observe::Role` become one public enum (ruled); `p4-remote-simplify` reads the surviving name). `src/tree/mirror/streaming/remote/codec/signal.rs:112-135`: `Speaker` and `observe::Role` name one election; the reason (the hook is rumors-blind) lives only in a commit message. Resolution: One sentence at `Speaker`; unification is owner-gated.
- **link-15** (nit; T100). `src/link/routed.rs:252-259`: `Dial::recycle` obligates pooling dials around a router-written byte whose value the public contract never states. Resolution: State that the value is unspecified and must be consumed; list the router-to-dialer bytes in the layout block.
- **link-2** (nit; T100). `src/link.rs:107-113`: The pooled-flow-control paragraph narrates a past observation that a committed test now pins. Resolution: Present-tense restatement naming the pin (owner-gated: owner phrasing).
- **streaming-backend-window-1** (low; T102). `src/tree/mirror/streaming/backend.rs:1-20`. Resolution: Add one sentence to the module doc stating the boundary is crate-internal and `Local` is the sole production implementation (the wording at conformance/backend.rs:43-47 already exists). Keep the implementer guidance that serves the maintainer adding a backend. When the boundary is published, do the trait-shape pass then: `parent` takes an owned `Vec`, `assemble`'s signature names the `pub(crate)` alias `BoxNodeStream`, and `Error: Send + 'static` carries no `Debug` or `std::error::Error` bound. Acceptance: backend.rs states the boundary's status in its module doc; no sentence in the family addresses an external implementer without that framing.
- **streaming-backend-window-10** (low; T103). `src/tree/mirror/streaming/backend/local.rs:195-200`. Resolution: Either (a) restate the comment with the quantity: the override trades the default chain's fan-per-level bookkeeping for one proportional to the run (state the per-leaf figure as a `size_of` expression, and that it is reclaimed at run end and falls under the budget's "replica itself" exclusion), and add one clause to decode.rs:356-358 saying the backend's own run buffer is the backend's custody; or (b) make the bulk build incremental with a radix-stack builder that emits each compressed subtree when its prefix closes. Acceptance: for (a), both comments describe the run buffer truthfully; for (b), a census-ledger measurement of one large supply run shows peak transient bounded by fan times depth rather than growing with run length.
- **streaming-backend-window-24** (low; T103). `src/tree/mirror/streaming/window.rs:110-121`. Resolution: Move the "Sizing the flushed-question edge" section onto `queues::local_questions`, where the premises become resolving links and `Scope` is in scope, and reduce window.rs to one sentence pointing there; invert the pointer at queues.rs:34-36. Acceptance: no file path in window.rs prose; the derivation lives beside `local_questions`.
- **streaming-backend-window-27** (low; T103). `src/tree/mirror/streaming/window.rs:267-275`. Resolution: First sentence: "The default budget for the memory a synchronization may spend on pipelining: 512 MiB." Keep the link to `Peer::sync_memory_budget`. Move the `SCOPE_ENVELOPE_BYTES` pointer into a `//` maintainer comment or drop it (its own doc carries the decomposition). Acceptance: the public first sentence is consistent with the method's exclusions and no cfg-gated or crate-private item is named.
- **materialized-18** (medium; T128). `src/tree/mirror/streaming/materialized/error.rs:10-39`. Resolution: Give `Error::Backend` and `Error::Violation` one-line docs (noting that in `MirrorError` the backend error is `Infallible`). In `Violation`'s enum doc, define the three answer kinds once in plain language (a reply answers each child the question listed, in order, with an agreement, a sub-question, or, for a child the asker lacks, a supplied subtree) and rewrite the variants against those words; replace "we"/"our" with "the receiving replica". Acceptance: `cargo doc` for `rumors::error::MaterializedViolation` reads without a backtick identifier that does not resolve to a public item; every variant of both enums has a doc comment.
- **conformance-3** (nit; T132). `src/conformance.rs:10-19`: The backend suite's visibility rationale is stated three times, once in public rustdoc, and omits why the items are `pub(crate)`. Resolution: One statement of record in backend.rs; drop the public paragraph.
- **mirror-common-1** (low; T132). `src/tree/mirror.rs:44-49`. Resolution: Variant docs: "The client-position participant failed; in a gossip session ([`MirrorError`](crate::MirrorError)) this is the local replica's walk." and "... the wire proxy for the peer." Move the coherence paragraph to a `//` comment above the impl and leave its rustdoc at "Lift a client-position error into the sum." Acceptance: the public docs on `Error<C, S>` and its `From` impl name only what a `MirrorError` holder can observe; the coherence argument survives as a maintainer comment.
- **prose-hygiene-5** (low; T132). `justfile:731`. Resolution: Delete the parenthetical; the comment already lists what `eventdag` checks. Acceptance: `grep -c 'PROGRESS.md' justfile` is 0.
- **session-bookmark-36** (low; T132). `src/reconciliation.rs:246-249`. Resolution: End the paragraph at "described above"; leave the oracle statement to the streaming module doc and AGENTS.md. Acceptance: `reconciliation.rs` names no item absent from the public surface.
- **testing-infra-1** (nit; T132). `src/testing.rs:69 (ten sites); src/testing/transport.rs:667-669`: Rustdoc cites consumers by file path; two divergent acceptors are called "duplicated". Resolution: Name consumers by role; state the divergence.
- **tests-disruption-handshake-20** (low; T132). `tests/handshake.rs:1-1`. Resolution: Cite `tree::mirror::handshake::preamble`, or drop the parenthetical (the next sentence already says what is driven). Acceptance: the cited path resolves under src/.

## Named here, landed elsewhere

These rows belong to this lane's pattern and are counted in its roster, but another ruling's lane lands them; this lane verifies the state at base, quotes it in the report, and edits nothing at their sites.

- **async-hazards-4** (low, T96): T96 rides `p2-commit-path` (after T34): memos warm eagerly and `warm_caches` is deleted; verify at base that the public first-greeting sentence exists and report if the merged lane omitted it.

## Hazards and stops

- **Stops**: T95's wording (draft, then hand to Finch); T97 and T99 are
  ruled public-surface edits, but if the P6 lane has already moved
  `Snapshot` (T60, T61) or `observe.rs` (T66, T74) at base, report
  before editing; T93 if the tolerated case cannot be expressed through
  `Peer`.
- fresh-eyes-8 (T94) is its own lane, `p4-bookmark-example`, after P6.
- `src/peer.rs`, `src/lib.rs`, `src/snapshot.rs`, `src/rumors.rs` are
  edited by `p2-commit-path` (unmerged): after its merge.
- `README.md` is derived: `just readme`.
