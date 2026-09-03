<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from rulings T128, T130, and T132 in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P4 lane: incident narrative, dated rationale, and roster tags out of code

## Goal

Prose speaks in the present tense. No comment or rustdoc narrates the
incident that motivated it, dates its rationale, compares today's code
to a prior state, or cites a roster tag with no home in the tree (a
finding number, a plan's section number, a bridge ordinal); each such
sentence becomes the invariant, the mechanism, or nothing (T128, T130,
T132). Lean names with a kernel-checked home stay (`B5`,
`wc_impossibility`, `d5`), restated inline. Effort: medium (prose; one
test rename).

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
  run in the background redirected to a log under `<scratchpad>/p4-narrative/`,
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

1. **Roster tags, one commit**: materialized-20, prose-hygiene-4,
   materialized-21, materialized-19's tag half rides `p4-tallies`,
   prose-hygiene-3 and tests-observation-15 (the same fourteen
   `listen.rs` docs; one edit). Oracle:
   `git grep -n -E 'finding #[0-9]|adjudicat|§|\bF4\b|\bT3\b|\bD5\b|Bridge [0-9]' -- src tests`
   empty; `B5` survives (T128).
2. **Incident narrative, one commit per file family**: tree (tree-core-21,
   tree-core-35, tree-typed-32), mirror and streaming (mirror-common-23,
   materialized-25, streaming-tests-9, streaming-tests-27), remote
   (remote-proxy-16, remote-adapter-streams-1), session
   (session-bookmark-45), the suites (prose-hygiene-7, tests-bookmark-16,
   tests-common-11, tests-disruption-handshake-5), benches
   (benches-envelope-30).
3. **Closing commit**: the union grep below, with every survivor named
   and its reason stated ("no longer" and "today" have legitimate uses;
   each one that stays is listed).

Oracle (a one-time pass: no standing check can tell narrative from a
present-tense sentence; the report carries the site list):

    git grep -n -i -E 'Historical|Reconstructed|the fix\b|motivated|observed while|exactly as before|as the typed tower|this replaces|"frozen"|awaits owner|minted|was found|proptest-regressions|\bexisting\b|no longer|today' -- src tests benches

## Members

Heading entries quote Resolution and Acceptance verbatim from the topic document; nit rows quote their table row (site, issue, resolution). Each line opens with the entry's primary site at `9e5784fb`; the full record, with its related sites and evidence, is under `### <id>:` in the document the ledger's `doc` column names. Rulings are cited by number; T132 is the roster approval for every low and nit.

- **materialized-20** (medium; T128). `src/tree/mirror/streaming/materialized/progress.rs:82-98`. Resolution: Delete every "(finding #N)" parenthetical. Rewrite progress.rs:206-212 to state the decision positively ("the walk publishes a scope's parent resolution last, trading any-capacity deadlock freedom for pipelining under the assembler's fan floor; `Sched.deadlock_free_d5` is the theorem for the other placement") without "adjudicated" or "Retained as the design-space record". In transcript.rs:7-13 keep the premise and the axiom name `B5`, drop "promoted to a proptest bridge by the mux adjudication". Sweep the sites outside the partition in the same pass. Acceptance: `grep -rn 'finding #\|adjudicat' src tests` returns nothing; the rewritten paragraphs read as present-tense statements of the discipline and the theorem that backs it.
- **streaming-tests-27** (medium; T128). `src/tree/mirror/streaming/tests/wedge.rs:12-18`. Resolution: Replace 12-18 with one present-tense sentence without paths: "The pair is hand-placed: a structural-equality pin needs exact paths, which content-addressed generators cannot supply." If the seed observation must survive, it belongs in `.agent-notes/`. Instances.lean:49-55 carries the same narrative and should be trimmed in the same pass (outside this partition). Acceptance: `grep -rn 'proptest-regressions' src/tree/mirror/streaming/tests/wedge.rs` is empty.
- **prose-hygiene-3** (medium; T130). `tests/listen.rs:73`. Resolution: Strip the "§6.N " prefix from each of the fourteen testdocs, keeping the title phrase as the first sentence. If the coverage map matters, list the observer-contract clauses in the module doc in English, without numbers. Acceptance: `grep -c '§' tests/listen.rs` is 0.
- **tests-common-11** (medium; T130). `tests/common/overlap.rs:9-11`. Resolution: restate the motivation forward: the failure class is an install that re-joins a session's fork-time state and must not drop a leaf a concurrent install added, and the pincer is the minimal schedule that puts a mutation between one session's fork and its install. Re-derive the preamble range from today's tree (for example: enough leaves that the root fan has several children, so an install touches a branch node) or drop the parenthetical. Apply the same treatment at 425-427 and at tests/session_overlap.rs:13, 73, 148. Acceptance: no sentence in overlap.rs refers to a past defect, incident, or the number 16; the preamble comment names a property of today's tree or names none.
- **tests-observation-15** (medium; T130). `tests/listen.rs:73-75`. Resolution: Delete the `§6.n ` prefix at each of the fourteen doc comments, keeping the English title; the parentheticals "(retire variant)" and "(negative control)" may stay as plain words ("Retire variant of termination: ...", "Negative control: ..."). Acceptance: `grep -c '§' tests/listen.rs` prints 0 and every affected doc still opens with its English title.
- **benches-envelope-30** (nit; T132; `examples/envelope_sim.rs` is deleted by T43 (`p1-envelope`): land only if present at base, else quote its absence). `examples/envelope_sim.rs:304-308`: A verification is claimed done whose artifact is not in the tree. Resolution: Drop the parenthetical, or name the `--manifest` affordance.
- **materialized-21** (low; T132). `src/tree/mirror/streaming/materialized/progress.rs:97`. Resolution: Replace "the encoder" with "the walk" throughout progress.rs and progress/tests.rs (rename the test to `walk_order_violates_parent_early_discipline` and update the citation at progress.rs:212). Replace "the weave's parent-early discipline" with "the parent-early discipline, `d5` in the formal model" (or cite the Lean definition `weaveScope` by name if the owner wants the model term). Acceptance: `grep -n encoder` in the two progress files returns nothing; `weave` is defined at first use or absent.
- **materialized-25** (low; T132). `src/tree/mirror/streaming/materialized/unknown.rs:19-23`. Resolution: "Each recursive call boxes its future ([`BoxFuture`]) so the future type stays finite, and the depth is bounded by the prefix's remaining height (the key depth), so the recursion is stack-safe by construction." Fix erased.rs:30 in the same pass. Acceptance: `git grep 'typed tower\|as before' src/tree/mirror/streaming/` returns nothing; the paragraph describes what is.
- **mirror-common-23** (low; T132). `src/tree/mirror/streaming/erased.rs:29-30`. Resolution: Delete ", exactly as before". Acceptance: `grep -n 'as before' src/tree/mirror/streaming/erased.rs` is empty.
- **prose-hygiene-4** (low; T132). `src/tree/mirror/streaming/tests/wedge.rs:1-10`. Resolution: Drop `T3` and keep `wc_impossibility`; drop "adjudication repair F4" and keep the clause it introduces ("for impossibilities, realizability flows from Rust to the model"); "finding #6" and "finding #7" become "sibling contiguity" and "parent placement", which the surrounding text already calls them; "Bridge 1/2/3" module docs become titles ("Wedge realizability", "`LocalEq` soundness", "Announced-skeleton reconstruction"); "D5 as stated" becomes "the `d5` placement as the model states it". Acceptance: `grep -rnE '\bF4\b|\bT3\b|\bD5\b|finding #[0-9]|Bridge [0-9]' src` returns nothing.
- **prose-hygiene-7** (low; T132; the `tests/disruption.rs:589` site may be gone under T143; the other four stand). `tests/payload_depth.rs:325-327`. Resolution: payload_depth.rs:325-327: "the one shape that fails at ingress between equal limits, because admission runs the receiving decode." session_overlap.rs:73-76: "An innocent leaf deleted under this overlap at any sweep position fails here; the sweep is total, so any regression with that symptom fails whichever layer produces it." common/overlap.rs:9-11: "That gap is where an overlap defect hides: an innocent leaf lost under exactly such an overlap." Optionally "Reconstructed" for "Historical" at disruption.rs:589 and bookmark_causality.rs:1263. Acceptance: the five sites read as present-tense statements of mechanism with no tally or incident.
- **remote-adapter-streams-1** (low; T132). `src/tree/mirror/streaming/remote/adapter.rs:4-14`. Resolution: Link `Reply` to `erased::Reply`, draw the diagram as `Reply<E>` (or `erased::Reply<B::Erased>`) with one clause saying the height rides as the retained scope's prefix length; reword line 52 to "the prefix at the scope's children height"; rewrite 56-58 as "through a fan-bounded channel into [`Backend::assemble`] (dispatched at the scope's children height by `erased::ops::assemble`)", dropping "existing", and apply the same fix to remote.rs:59 ("the backend's existing conversion fold"). Acceptance: no `Reply<B, H>` or bare `H` parameter remains in adapter.rs prose; the `Reply` intra-doc link resolves to the erased type the signatures use; adapter.rs names `Backend::assemble` for the decode side as it names `Backend::leaves` (line 44) for the encode side; `grep -n existing` over adapter.rs and remote.rs is empty.
- **remote-proxy-16** (low; T132). `src/tree/mirror/streaming/remote/proxy/work/encode.rs:123-124`. Resolution: "never opens, and a session without initiator exclusives opens no initiator-direction stream at all." Acceptance: the grep is empty and the sentence states the behavior without temporal contrast.
- **session-bookmark-45** (nit; T132). `src/message.rs:53-59`: The default depth limit is justified by a fleet upgrade with no prior release to upgrade from. Resolution: State the fact positively: at the default, admission enforces nothing the decoder does not.
- **streaming-tests-9** (low; T132). `src/tree/mirror/streaming/tests/announced.rs:26-32`. Resolution: Cut from "(observed" through "unaffected)". Keep the mechanism once, at `trace_channels` (skeleton.rs:641-648), and have announced.rs say "the claim is per channel, not the global interleaving ([`trace_channels`] explains why)". Acceptance: no "observed while" phrasing; one explanation of the unbiased `select!` in the partition.
- **tests-bookmark-16** (low; T132; `p1-causality` (merged) rewrote these files: re-anchor from the quoted evidence). `tests/bookmark_causality.rs:1081-1087`. Resolution: rewrite L1075-1092 as the invariant (retiring into an absorber that has itself reclaimed from a bookmark absorbs cleanly and leaves the absorber holding `Party::seed()`), the trigger (both peers have reclaimed; one side alone does not exercise it), and why it lives at the library boundary (no harness). Reword L696-705 to state the classification rule without the incident. Replace L1263-1267 with the live mechanism ("these plans pin the shapes below explicitly; the committed seeds regenerate through the strategy's cut range, so a range change re-maps them") and open L1269 and L1290 with the invariant. Drop "Diagnostic helper" and "the in-flight-window fix" (name the schedule the test does not cover instead). Acceptance: `grep -n 'fixed\|motivated\|Historical\|Reconstructed\|the fix\|Diagnostic helper' tests/bookmark_*.rs` returns nothing.
- **tests-disruption-handshake-5** (low; T132; the inter-process section was deleted by T143 (`p1-harness-tests`, merged): quote the absence at base if the sites are gone). `tests/disruption.rs:589-594`. Resolution: Rewrite the header in the present tense ("Shrunk counterexamples as explicit constructions. A committed seed regenerates through the fault strategy's cut range, so a range change re-maps its offsets and the seed replays a different plan; these constructions carry the plans themselves.") and re-denominate the four docs to what is stable: fixed fault plans from shrunk seeds, run under the full invariant battery, with no claim about where the cuts fall. If the original geometry matters, derive the cut from a metered clean run of the same plan (a fraction of the measured extent) so it survives wire changes. Acceptance: no "Historical", "no longer", or "was found" in the section; no doc states a session position a wire-format change could falsify.
- **tree-core-21** (low; T132). `src/tree/arb.rs:174-175`. Resolution: At 174-175 and 185-192, describe the budget and what the wide pairs reach (multi-level disputes mixed with provisions in the opening reply) without "deadlock" or "gap". At 300-302: "The shape stresses whole-subtree provisions queued behind a dispute on one reply stream. Version addressing means it cannot be dictated, so it is searched: ...". Acceptance: `arb.rs` contains no incident nouns ("deadlock", "gap", "should have"); each generator doc states shape and stressed property in the present tense.
- **tree-core-35** (nit; T132). `src/tree/traverse/unknown/tests.rs:1-3, 24-25, 127-129; src/tree/tests.rs:1342`: The live cost oracle is framed as "replaced"; the meter doc states a ratio the assert does not pin; one dated "now". Resolution: Present-tense restatements; make the stated pin match the `assert!`.
- **tree-typed-32** (low; T132). `src/tree/typed/untyped/iter.rs:1-6`. Resolution: iter.rs:1-6: "Leaf walks over the untyped tree: a shared borrowing frontier engine beneath [`Iter`] and [`Range`], and the owned spine walk [`RangeOwned`]. A child module of [`untyped`](super) so the walks can match the parent's private `Children` variants directly." Delete one "32-byte" at 376-377; insert the blank `///` at 287-288; hash.rs:218: "A fixed value: compute it once, on first read, rather than re-hashing the four bytes on every empty-root read"; prefix.rs:24: "whose length determines the height (`32 - height` bytes at `height`)"; untyped.rs:568-569: a present-tense statement of the fused hull's cost (one decode per operand serving both lattice directions); drop the quoted "frozen" and "Freeze" for "owned". Acceptance: each cited line reads correctly; `grep -rn 'this replaces\|"frozen"' src/tree/typed` is empty.

## Findings routed here

- From `../new-findings.md` (the `p2-walk` lane): `src/tree/mirror/streaming/materialized/work/levels.rs`, `internal_level`'s doc claims "a mid-loop failure here would strand the counterparty's reply pump on a full slot, ahead of the error's own publication"; the wire twin exercises mid-loop backend failures across stages and every one returns, so the premise is undemonstrated. Demonstrate it (a committed test that reaches the stranding) or delete the sentence; the report says which and why. No ledger row: record the outcome in `new-findings.md`'s disposition column.

## Hazards and stops

- `p3-vocabulary` swept "knob" and "genuine" through the same streaming
  test files (streaming-tests-8): re-anchor every site from the quoted
  evidence, never the line numbers.
- T128 keeps `B5` where materialized-20 and prose-hygiene-4 agree;
  report any site where they do not.
- `tree-core-21` (`arb.rs`) and `tree-core-35` (`traverse/unknown/tests.rs`,
  `tree/tests.rs`) sit in files `p2-commit-path` (unmerged) rewrites:
  after its merge.
- `Instances.lean:49-55` (streaming-tests-27) is the formal tier: out of
  scope; report it for the `before`/formal owner instead of editing.
