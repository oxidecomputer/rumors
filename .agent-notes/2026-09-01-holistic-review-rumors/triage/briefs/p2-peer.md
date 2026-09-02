<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P2 lane: the peer's party, reclaim rule, and wake rule

## Goal

`Inner.party` is an `Option<Party>` only because `Peer::retire` leaves a
`None` behind while the party is in flight, which forces two unreachable
arms of different shapes and lets `Batch::commit`'s no-party arm degrade to
a silent drop in release builds. The public `Bookmark` doc promises reclaim
at the first dominating gossip while the code reclaims at the first
unsuppressed pre-session checkpoint after dominance, and no test pins the
timing. And two commit sites state different wake rules for a party-only
change. The invariants restored: the party is total in `Inner`; the trait
doc and the code state one reclaim rule, pinned; and a party-only change
wakes no watcher.

## Ground rules

These apply to every P2 lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal `62b30e1f` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `62b30e1f`, fast-forward; if it has diverged, stop and report. Never call
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
  run in the background redirected to a log under `<scratchpad>/p2-peer/`,
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

### api-core-2 (low): ruling T37, resolution amended to the redesign

Resolution: Replace the `let ... else` with `.expect("a Batch commits through a live Peer or Rumors handle; the Peer/Rumors XOR keeps one from coexisting with a retirement's in-flight party")`. The structural alternative, making `Inner.party` total by holding the in-flight retirement party elsewhere (it would also delete the `None => inner.party = Some(party)` arm at gossip.rs:825-828), is a larger design change; see the open questions. Acceptance: no `debug_assert!(false, ..)`-plus-fallback pattern remains in `Batch::commit`; the panic message states the XOR argument; `just test` still passes (the arm is unreachable, so no test changes).

Ruled (T37): the structural alternative. `Peer::retire` holds the party
it is moving inside the retire future; `Inner.party` becomes a plain
`Party`; the no-party arm in `Batch::commit` and the `None =>` arm in
`gossip.rs` disappear. The construction in the entry (forcing `party =
None` through `src/tests.rs`) becomes unwritable, which is the acceptance:
report that it no longer compiles. Every `Retire` outcome
(`Recovered`, `Uncertain`, and the success path) must still restore or
consume the party exactly as today; the identity-duplication argument
(`PartyGuard`, the slice-out-before-send order) is unchanged and its
tests pass. Public surface: no `pub` signature changes; `Peer::retire`'s
signature and the `Retire` enum are as they are. Any change there is a
stop. `bookmark_update`'s reading of the party moves with the field.

### session-bookmark-21 (low): ruling T32

Resolution: Owner decision between two consistent states. (a) Recommended, consistent with the recorded ruling: re-word bookmark.rs:61-66 to the actual gate ("reclaimed at the first session whose pre-session checkpoint is not suppressed after the frontier dominates it: the first session after attach, or after any own event or party change following the dominating gossip"); peer.rs:250-253's "behind that path's persist gate" is already accurate. (b) Make the pre-session gate also fire when the record holds a clock with `own_version() <= version` that the live party does not cover; this relaxes the never-write-on-hearsay rule `tests/bookmark_when.rs` pins, so its model needs a `pending` rule for reclaimable clocks. Under either choice, add a test asserting the persisted record's contents after a hearsay-only dominating gossip. Acceptance: a committed test constructs the scenario below and its expectation matches the documented rule; the trait doc and the code state the same rule.

Ruled (T32): option (a). The construction in the entry is the test; its
expectation is the checkpoint rule. `Bookmark`'s rustdoc is public prose:
the reworded sentence goes in the report for Finch's read; that is not a
stop.

### Correctness open questions 15 and 17: ruling T35

No finding ids. Two changes. (1) The `Changes` field doc (the `seen`
frontier in `src/rumors/changes.rs`) gains one sentence stating why
`latest()` suffices to detect every content change: every action in
`Tree::act` ticks the party before it is applied, forgets included; a
gained leaf carries a version outside the local ceiling (a leaf inside it
that we lack was redacted, and deletion honoring refuses it); a shed leaf
requires the peer's ceiling to contain the redaction tick, which we cannot
already hold because holding it means we already shed the leaf. (2) One
wake rule at both sites: a party-only change (take or fork) notifies no
watcher. `gossip.rs`'s take-or-fork closure (the `guarded.party.is_some()`
return) adopts `bookmark_update`'s rule, and one comment at the shared
rule says why: content processing is independent of the party. Acceptance
for (2): the retire, bootstrap, and bookmark suites pass unchanged; if any
test or observer relied on the party-only wake, that is a stop with the
failing test named, not a reason to keep the wake. Acceptance for (1): the
sentence is present and its three clauses are true of `Tree::act` and the
join's deletion honoring (cite by function name).

## Hazards and stops

- `src/peer/gossip.rs`'s identity-safety argument around `PartyGuard` is
  the one region T37 must not weaken; restate it in terms of the new
  holder and keep every exit path's test.
- Removing the party-only wake changes what `Extant`/`try_into_peer`
  observe only if they watch the channel; check before landing, and if
  they do, stop.
- Public rustdoc edits are confined to the `Bookmark` sentence (T32) and
  the `Changes` field doc (T35); anything wider is a stop.
