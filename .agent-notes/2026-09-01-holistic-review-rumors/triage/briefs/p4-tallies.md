<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from rulings T98, T104, T129, T131, and T132 (T112's two rows named, not landed) in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P4 lane: counts, tallies, and measurements out of prose

> **Ruled after drafting.** T159 item 1: the ledger's ruling column reads T82 for module-graph-4, prose-hygiene-2, and mirror-common-12.

## Goal

Prose states the structure, never the tally: no rustdoc, comment, or
config comment carries a count, ratio, byte figure, or measurement that
the code can change without touching the prose. Where a number matters
it lives in a mechanically enforced place (a constant, a pin test, a
`const _` assertion) that the prose cites by name: the stream bound
through `STREAM_COUNT` (T104), the wire law through
`dispute_overhead_bytes()` (T131), the nextest budget without figures
(T129), the hop bands without their recorded counts (T98), and every
other site per its own Resolution (T132). Where the number does not
matter, the sentence names the structure. Effort: medium (prose, one
grep-derived census, one `const` assertion).

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
  run in the background redirected to a log under `<scratchpad>/p4-tallies/`,
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

1. **Census first.** Run every member's own grep and the union below at
   base; keep the site list under `<scratchpad>/p4-tallies/census.txt`
   and quote its counts in the closing commit (the known-bad artifact
   for this sweep is the tree at base, where every member's grep is
   nonempty).
2. **One commit per file family**: crate root and session; link and
   conformance; tree; mirror and streaming; remote; test scaffolding;
   integration suites and benches; the config files. `README.md` is
   derived: edit `src/lib.rs`, then `just readme`.
3. **The one committed check** is conformance-5's
   `const _: () = assert!(CONTROL_DUPLEX_FILL > MEMORY_STREAM_CAPACITY, ..)`;
   its negative control is a scratch build with the inequality reversed,
   the compile error quoted in the commit message.
4. **Closing commit**: the union grep and every member's grep, empty,
   quoted.

Oracle (empty output is the acceptance; there is no standing check,
because a number in prose is not distinguishable by machine from one
that belongs there, so this sweep is a one-time pass whose report
carries the site list):

    git grep -n -i -E 'two deliberate|four invariants|two corners|three orders of magnitude|the four (`Retire`|ways)|seven checks|both tests|one of 17|one of ten|\b16[23]\b|17 of the fixed|33-arm|16 data streams|is 17|version 5 is|0x18 0x05|2-3x|28 \+ m|~36 B|0\.5 s|5 s budget|6 seconds|8 GiB|8 KiB|attempt 1581' -- src tests benches .config .github justfile

## Members

Heading entries quote Resolution and Acceptance verbatim from the topic document; nit rows quote their table row (site, issue, resolution). Each line opens with the entry's primary site at `9e5784fb`; the full record, with its related sites and evidence, is under `### <id>:` in the document the ledger's `doc` column names. Rulings are cited by number; T132 is the roster approval for every low and nit.

- **tests-resource-link-window-24** (nit; T98). `tests/window_corners.rs:93-97 and the sites listed`: Measured hop counts recorded in comments beside bands no assertion enforces. Resolution: Pin exactly, or drop the measurement (owner-gated).
- **fresh-eyes-1** (medium; T104). `src/reconciliation.rs:201-205`. Resolution: Rewrite reconciliation.rs:201-205 to state that a session opens at most [`STREAM_COUNT`] data streams per direction beside the persistent control stream, linking the constant for the derivation, and delete the "16 data streams plus a control stream" arithmetic; one derivation, at the constant. Acceptance: `grep -n "16 data\|is 17" src/reconciliation.rs` is empty and the page states the bound only through `STREAM_COUNT`.
- **session-bookmark-35** (medium; T104). `src/reconciliation.rs:201-205`. Resolution: "A 32-byte address gives the descent 32 heights; each reply phase descends two, and the opening question rides its own stream, so a side needs at most [`STREAM_COUNT`] data streams per direction, plus the persistent control stream; in practice far fewer are opened." Drop the literal 16 and 17. Acceptance: the page's two statements agree with each other and with link.rs:161-169; no literal stream count remains in the prose.
- **tests-resource-link-window-1** (medium; T129). `.config/nextest.toml:32-35`. Resolution: Rewrite lines 32-35 to state what is: the window suites assert on exact virtual time, and window_corners' real-clock leg asserts lower bounds only, which load can inflate but never falsify, so none needs isolation. Drop the 0.5 s and 5 s figures. Acceptance: the comment names no upper bound and no budget figure and agrees with window_corners.rs:206-212.
- **tests-resource-link-window-29** (medium; T131). `tests/window_operator.rs:3-10`. Resolution: At window_operator.rs:4 and :9, write the law in terms of the named constant (`DISPUTE_OVERHEAD_BYTES + m`, as window.rs:233-234 does) or as "the intercept `dispute_overhead_bytes()` exposes", and point at `Peer::sync_memory_budget` for the published form. At window_knee.rs:314-317, either excise the worked figures and keep the conclusion ("a BDP in messages below the binding capacity, per the intercept `tests/dispute_wire.rs` pins") or compute the BDP in messages from `dispute_overhead_bytes()` in code and assert it is below `capacity`. Acceptance: `grep -rn '28 + m\|~36 B' tests/` returns nothing; every wire-law figure in test prose is stated by constant name or derived from a `testing` accessor.
- **tests-wire-format-16** (medium; T131). `tests/dispute_wire.rs:82-100`. Resolution: Expose `testing::design_record_bytes()` beside `dispute_overhead_bytes()` and derive the design payload length from it (`design_record_bytes() - CBOR_BSTR_HEADER_BYTES`), or assert `DESIGN_ENCODED_PAYLOAD_BYTES == design_record_bytes()` up front with its own message naming the record-size coupling; reword the design cell's doc to "the anchor the budget docs quote"; split `envelope_and_wire_bytes` into `scope_envelope_bytes()` and `dispute_wire_bytes()`; delete `fixed_overhead_bytes` and call `dispute_overhead_bytes()` directly; add `MID_ENCODED_PAYLOAD_BYTES` and drop the literal totals from the docs; reword 29-31 and 348-352 to "the control bounds the greeting-and-epilogue share; exactness rests on the seeded corpus and in-memory link"; state the `eprintln!`s' purpose in one comment or delete them. Acceptance: changing `DESIGN_RECORD_BYTES` in window.rs fails dispute_wire with a message naming the record size; the two docs agree on what derives from `DISPUTE_WIRE_BYTES`; no literal 170, 172, or 64 appears in the file except as a constant's single definition; no `(_, x) = envelope_and_wire_bytes()` pattern remains in tests.
- **api-audit-17** (nit; T132). `src/tree/mirror/streaming/stats.rs:33-38`: "Two deliberate boundaries" lists one; the second was the V1 bullet. Resolution: Fix the count or restore a second bullet (same site as fresh-eyes-5 and mirror-common-35).
- **api-core-14** (nit; T132). `src/peer.rs:285; src/peer/bootstrap.rs:334-335`: "the four `Retire` outcomes" and "the four ways" count enums defined in other files. Resolution: "each `Retire` outcome"; "Each way that can end is a `Joined` variant".
- **benches-envelope-17** (nit; T132). `benches/support/grid.rs:37-40; window_wallclock.rs:34; gossip_fixed.rs:9, 67-69`: Hand-maintained counts beside the arrays they count, one wrong ("three orders of magnitude" for 10^2 to 10^6). Resolution: State the structure, not the tally.
- **conformance-5** (low; T132). `src/conformance/link.rs:87-88`. Resolution: (a) make `MEMORY_STREAM_CAPACITY` `pub(crate)` and pin the relation, `const _: () = assert!(CONTROL_DUPLEX_FILL > crate::link::MEMORY_STREAM_CAPACITY, "the duplex probe must overrun the reference link's control buffer");`, then let the prose state the inequality, not the ratio; (b) "the open past [`CAPPED_STREAMS`]"; (c) "a sparse `debug_assert` over a handful of fans"; (d) keep the mechanism ("a large compile-time cost per instantiation") and drop the figure; (e) "this crate's backend". Acceptance: no doc in the partition restates a count, ratio, or measurement a sibling constant or array literal owns, and a const assertion ties `CONTROL_DUPLEX_FILL` to `MEMORY_STREAM_CAPACITY`.
- **fresh-eyes-5** (low; T132). `src/tree/mirror/streaming/stats.rs:33-38`. Resolution: Drop the count ("One deliberate boundary:" or fold the bullet into a sentence), or restore a second item if one is intended (the "every count is local; the two ends report their own numbers" point at gossip.rs:174-175 is the natural candidate). Acceptance: the heading's count, if any, equals the bullets beneath it.
- **link-1** (nit; T132). `src/link.rs:19-22, 298-299, 547-549, 376-377`: An expired "cheap", a "stated where they arise" clause that precedes the full list, a hand-maintained "8 KiB", a missing blank line. Resolution: Four edits as the entry lists.
- **materialized-19** (nit; T132). `src/tree/mirror/streaming/materialized/progress.rs:58, 94, 117, 122-124; progress/tests.rs:23`: "Seven checks:", a one-off name for a named invariant, two glosses for `d6`, and a check cited by quoting another file's comment. Resolution: Four edits as the entry lists.
- **mirror-common-2** (nit; T132). `src/tree/mirror/cbor.rs:59-62 and the sites listed`: Derived arithmetic ("17 of the fixed item's 30 bytes", "a 33-arm match", MB figures) and caller rosters written as literals beside the constants that derive them. Resolution: Cite the constants and the pin by name; state the property instead of the roster.
- **mirror-common-35** (low; T132). `src/tree/mirror/streaming/stats.rs:33-38`. Resolution: Fold the bullet into a sentence ("There is deliberately no duration field: the caller owns the clock, ..."), dropping the enumerated form; or, if a second boundary is real today (`window_granted` at :143-145 already says it "is a summary, not the whole vector"), state it as the second bullet. Do not restore the V1 bullet. Acceptance: the heading's count matches the list beneath it, or there is no count.
- **prose-hygiene-12** (nit; T132; `tests/async_wire.rs` is deleted by T131 in P5: land only if the file exists at base, else quote its absence). `tests/async_wire.rs:13`: "Both tests" enumerates the module's contents by count. Resolution: "The tests share ...".
- **remote-adapter-streams-17** (low; T132). `src/tree/mirror/streaming/remote/streams.rs:3`. Resolution: "binds the protocol's [`Stream::COUNT`] logical streams per direction onto ..." (or link `crate::link::STREAM_COUNT`); likewise at the remote.rs and codec.rs sites. Acceptance: the literal 17 appears in prose only at its derivation site in link.rs; the intra-doc link resolves under `just docs`.
- **remote-adapter-tests-1** (low; T132). `src/tree/mirror/streaming/remote/adapter/tests.rs:1-11`. Resolution: add one sentence for `backend_errors` (its role is source-error propagation and atomicity across the backend operations the adapter reaches), or restate the map as structure rather than roster. Acceptance: every `mod` declared in tests.rs is named in its module doc.
- **remote-capture-atlas-1** (low; T132). `src/tree/mirror/streaming/remote.rs:12-41`. Resolution: Trim remote.rs to the role map (3-6), the transport sentence (8-10, citing `Stream::COUNT` by name rather than "17"), and the cross-cutting account only this level can tell (the opening question riding the greeting and the early-supply stream, 43-54), with intra-doc links to `codec`, `adapter`, and `streams` for their own contracts. Where a count survives anywhere in prose (codec.rs:25-26, streams.rs:3), name the enforced place (`VALID_PLACEMENTS`, `Stream::COUNT`) rather than the literal. Acceptance: `grep -nE '162|163|one of 17|one of ten' src/tree/mirror/streaming/remote.rs` is empty; no sentence in remote.rs's module doc duplicates one in codec.rs, adapter.rs, or streams.rs.
- **remote-codec-1** (low; T132). `src/tree/mirror/streaming/remote/codec.rs:18-27`. Resolution: In `codec.rs:18-27`, drop "(one of 17)" and "one of ten" (the structure is already stated) and replace "the initiator admits 162 placements and the responder 163" with "each speaker admits a phase-specific subset, pinned by `placements_match_the_phase_schedule_exhaustively` and `invalid_placement_snapshot`". In `signal/tests.rs:9-10` and `:146` cite `Signal::STATE_COUNT` and `Stream::COUNT`; in `budget/tests.rs:3` write "`FAN` full-fan query frames"; in `frame.rs:424-426` say "whichever reader drives it" without the roster; in `decode/tests.rs:798-806` say "every cut in `chunk_boundary_cuts`'s roster" and repeat none of the offsets (the first sentence, "cuts at every seeded offset all classify", also wants a rewrite). Apply the same edit at `remote.rs:13` and `:18` (outside this partition). Acceptance: `grep -n "162\|163" src/tree/mirror/streaming/remote` hits only `VALID_PLACEMENTS`; no literal 17, ten, or 256 remains in codec prose outside the enforcing constant or test; the truncation testdoc names the helper and none of its offsets.
- **remote-proxy-tests-13** (low; T132). `src/tree/mirror/streaming/remote/proxy/tests/declarations.rs:4-5`. Resolution: declarations.rs: name `target_message_size` (the run budget's input) beside the other two. harness.rs: "Two-proxy session harness: link decorators (I/O plans, frame scripts, greeting rewrites), the shared driver, and the role-election predicate." work/tests.rs: a `//!` line naming the `Work` executor's error-selection properties. Acceptance: each doc names what its file contains; `grep -L '^//!' -r --include='*.rs' src/tree/mirror/streaming/remote/proxy` lists no test file.
- **session-bookmark-28** (nit; T132). `src/bookmark/format.rs:61-69; format/tests.rs:171-173, 190-191`: "Version 5 is ..." and "0x18 0x05" hand-maintain the format version; "the earlier frame shapes" refers to formats no longer in the tree. Resolution: "The current version ..."; "0x18 <version>"; "no decoder exists for any lower version number".
- **streaming-tests-1** (low; T132). `src/tree/mirror/streaming/tests.rs:1-5`. Resolution: Either complete the map with one clause per module (`stats` pins the counters against a dispute oracle; `skeleton`, `wedge`, `local_eq`, and `announced` bridge sessions to the Lean model) or drop the enumeration and let each submodule's own first sentence serve, since the module listing already shows them. Acceptance: every `mod` at tests.rs:27-34 is named in the doc, or the doc enumerates none.
- **suite-economics-6** (low; T132). `.config/nextest.toml:28-35`. Resolution: delete the wall-clock-upper-bound sentence or re-anchor it to the floor-only claim (a floor cannot flake under load, which is the point worth stating). Restate the budget's premise without numbers: the deterministic pollers turn stalls into errors, every committed test finishes far inside one period, and the terminate budget is a last-resort bound for escaped hangs rather than a per-test bound. Acceptance: every sentence in the file describes a check or mechanism that exists in the tree, and no timing number remains that a reader would need to re-measure to trust.
- **suite-economics-9** (nit; T132; T136 retired memwatch: verify the `ci.yml` header at base and quote the absence if the figure is gone). `.github/workflows/ci.yml:26-28`: The header quotes an 8 GiB memwatch cap; the tool's default is 32. Resolution: Drop the figure, or name `PROC_LIMIT_GB` as the owner of the number.
- **testing-infra-15** (low; T132). `src/tests.rs:1-6`. Resolution: Restate: "Crate-level tests that need in-crate access: party linearity across bootstrap and retire, retire sessions severed at exact frame boundaries, link poisoning, the containment violation at the API tier, and the root-hash read meter." At 623, name the pins instead of counting them. Acceptance: the module doc names every family of test in the file; no "the N ... below" phrasing.
- **tests-bookmark-1** (low; T132). `tests/bookmark_attach.rs:4-8`. Resolution: drop the count and name all three claims (lazy persist for a pristine seed; a failed persist returns the peer intact; a failed attach reclaims nothing into the returned peer), or state that these are point tests on the attach contract's corners and let the testdocs enumerate. Acceptance: the module doc's list matches the `#[test]` functions in the file.
- **tests-common-9** (low; T132). `tests/common/mod.rs:6-29`. Resolution: add a bullet for each (`overlap`: a session parked at a chosen poll prefix while other events run, with its own valid-by-construction generator; `shape`: deterministic tree-shape staging for the pin fixtures). Alternatively restructure the map to state composition relations only and let each module's first doc sentence carry its role, so the list cannot rot. Acceptance: every `pub mod` in mod.rs appears in the map, or the map no longer enumerates modules.
- **tests-lifecycle-30** (low; T132). `tests/single_peer.rs:1-6`. Resolution: Extend the doc with one sentence per group: the commit lifecycle (commit on `Ok`, cancel on `Err` or panic), the bulk variants' all-or-nothing and skip semantics, and nesting. Acceptance: every test in the file is covered by a sentence in the module doc.
- **tests-observation-1** (low; T132). `tests/causal.rs:1-3`. Resolution: Keep the two-face tests together (they pin that both faces hold the same checkpoint boundary, which causal.rs:645-649 states) and restate the module doc: "...and the checkpoint-resume boundary both observer faces share, pinned here against both." `drain_unordered` dissolves with tests-observation-2. Acceptance: the module doc names the unordered-face tests present in the file.
- **tests-resource-link-window-2** (nit; T132). `benches/support/latency.rs:398-400 (three sibling comments)`: The dead_code comment names one caller where the partition has three. Resolution: Drop the caller clause; keep every `#[allow(dead_code)]`.
- **tree-core-10** (low; T132; T110 declined new meters: take the first arm (drop the figure, keep the qualitative claim) and say so). `src/tree.rs:387-390`. Resolution: Either drop the figure and keep the qualitative claim (one traversal instead of one per action, shared spine work amortized), or add a `single_insert` column beside `batch_insert` in `benches/in_memory.rs` and cite it by name. Acceptance: the doc either states no figure or names a committed bench that produces it.
- **verification-infra-18** (nit; T132). `justfile:272-273; .config/nextest.toml:7-10`: A comment path that resolves to nothing (`src/testing/diff_ops.rs`); a dated "today" and 6-second figure. Resolution: Fix the path; drop the dated figures.

## Named here, landed elsewhere

These rows belong to this lane's pattern and are counted in its roster, but another ruling's lane lands them; this lane verifies the state at base, quotes it in the report, and edits nothing at their sites.

- **suite-economics-3** (medium, T112): T112 (owner decision 66, P7): the `HINT_ATTEMPT` measurement lands there; this lane leaves `arb.rs:318-325` untouched and quotes the base.
- **tree-core-24** (medium, T112): T112 (owner decision 66, P7): same commit as suite-economics-3; not landed here.

## Hazards and stops

- T112 (P7) owns the `HINT_ATTEMPT` change; this lane touches nothing
  at `src/tree/arb.rs:318-325` (`p2-commit-path`, unmerged, edits
  `arb.rs` as well).
- remote-codec-1, remote-capture-atlas-1, and remote-adapter-streams-17
  cite the stream count by name; `p4-widths` (T87) moves its derivation
  into the codec. Cite the name that exists at base and say so.
- tests-wire-format-16 and tests-resource-link-window-29 add `testing`
  accessors (`design_record_bytes()`): additions to the `test-internals`
  facade, not the public surface.
- Any figure that turns out to be enforced somewhere (a pin the prose
  quotes correctly) stays, cited by the pin's name; report it.
