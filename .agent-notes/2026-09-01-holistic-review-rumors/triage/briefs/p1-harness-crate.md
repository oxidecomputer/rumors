<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P1 lane: the in-crate streaming harnesses

## Goal

The streaming mirror's own suites under `src/tree/mirror/streaming/` are
the instruments every later change to the protocol is judged by. The
review found: stall probes that reduce completion, protocol violation, and
poll-budget exhaustion to one boolean, so a session that dies with a
violation counts as "did not stall" (demonstrated); a leaf-height deletion
verdict no committed test discriminates (demonstrated); a proxy ordering
trace with no liveness floor, satisfied by an empty trace; a proxy harness
whose default arrangement production never uses, hand-rolled seven times;
codec decode fragments copied between the async reader and its sync
oracle, with a listing-head spelling that differs between them and no
differential driving it; no test that cancels the mirror mid-session; and
every join differential running at the floor window. The invariant
restored: each suite fails on the failure it exists to catch, and each
claim its docs make is one its body checks.

## Ground rules

These apply to every P1 lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal `0926fe32` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `0926fe32`, fast-forward; if it has diverged, stop and report. Never call
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
  `Err`, or a reversible mutation whose observed failure the commit message
  records verbatim).
- **Resource discipline.** Build with `cargo nextest run --no-run` before
  running; during iteration run only the binaries the entry names; never
  iterate on a timing measurement. One full `just gate` before each commit,
  run in the background redirected to a log under
  `<scratchpad>/p1-harness-crate/`, polled with short foreground checks
  (the foreground command cap is ten minutes). Keep every working file
  under that directory. The capacity suite is the slowest in the crate;
  run it only through the gate, never in iteration.
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

### streaming-tests-11 (high): ruling T26

Resolution: Replace the boolean probes with a three-way outcome: `fn outcome(pair, capacity, schedules) -> Result<Result<(Root, Root), MirrorError<..>>, Quiescence>` (or the `Outcome` of the `LocalSession` builder in streaming-tests-3), with `stalls(o) = matches!(o, Err(Quiescence::Stalled))` and `completes(o)` returning the roots so the caller can compare them to `join_oracle`. Rewrite `stalls_under_any_schedule` as `any(stalls)` and add `completes_under_every_schedule` as `all(completes)` for the negative sites; treat `Ok(Err(_))` and `Err(Quiescence::PollBudget)` as failures with their own messages. Acceptance: inject `Fault::Reply(Violation::UnexpectedQuery)` into the `internal_fan(3)` session (or temporarily make the completion path return `Ok(Err(..))`) and confirm `parent_delay_single_parent_boundary` fails; restore and confirm it passes; every `!stalls` site is a `completes` site whose roots are compared to `join_oracle`.

`streaming-tests-3` (the `LocalSession` builder) is a P5 entry; do not
build it here. Ordering hazard from TRIAGE.md: this lands before any P7
measurement of the capacity suite's width; do not change the suite's
parent count.

### materialized-27 (high): ruling T26

Resolution: Two committed tests. (1) In `unknown/tests.rs`, a `leaf_sibling_path` variant of `tree_and_known` (k leaves under one shared 31-byte prefix with per-leaf known flags) driven through the existing differential oracle, the cheapest kill. (2) A cross-peer fixture beside `leaf_parent_redaction_pair` in `src/tree/arb.rs`: side `a` holds leaves at `leaf_sibling_path(0x00)` (party 0) and `leaf_sibling_path(0x01)` (party 1); side `b` has forgotten 0x00 and never held 0x01, so it holds nothing under that parent, with ceiling `v00 | tick(party 2)`; drive it through `streaming_mirror_sides` in both orientations, assert both endpoints equal `join_oracle`, and assert the holder sheds exactly one and the other gains exactly one. Then generalize (2) into a proptest with `Tree::join` as oracle. Verify the kill by applying the inversion as a reversible string swap, confirming the new tests fail, restoring, and checking `git diff` is empty. Record the disposition in the handoff note (which may then be retired). Acceptance: a committed test fails with `!self::known` replaced by `self::known` and passes on HEAD.

Also settle what the review left open: with the inversion applied, run
the whole committed suite once (the gate's test legs) and report whether
anything other than the new tests fails. The handoff note is
`.agent-notes/2026-08-21-unknown-pruning-survivor/README.md`; append its
disposition, do not delete it.

### remote-proxy-19 (medium): ruling T26

Resolution: give `Trace` a floor every divergent two-proxy session must meet and call it beside the channel floor in `instrumented_channels_cover_every_proxy_edge` and in the two proptests: for example `assert_covers_divergent_session()` requiring exactly one `DecodedReply` at `UnderRoot::HEIGHT` (the initiator-side proxy's greeting-seeded opening, `Work::initiator`, pump.rs:82-86) and exactly one `LocalQuestion` at `UnderRoot::HEIGHT` (the responder-side proxy's opening publication, encode.rs:142-143) across the trace, plus at least one `WireReply`. Commit the known-bad demonstration: `with_trace(|| ())` yields a trace the floor rejects. Acceptance: a `should_panic` test builds `Trace` from `with_trace(|| ())` and fails the floor with a named message; the three consuming tests call the floor and still pass; deleting the `progress.decoded_reply` call in `Work::initiator` fails at least one committed test.

### remote-proxy-tests-24 (medium): ruling T22, amended

Resolution: Ungated: give `drive` a topology axis (coupling with remote-proxy-tests-5) and make the production arrangement (`mirror(right, remote_left)`, mapping `right.map(|(root, _control)| root.into())`) the default for the transport, malformed, declaration, and greeting suites; keep the asymmetric arrangement for containment.rs, whose reason is documented at the check site; collapse the Server/Client projection accordingly (remote-proxy-tests-16). Gated: decide whether `impl Connect for Handshaking`, `impl CompleteConnect`, and `Connecting` in start.rs stay for containment's server-position wire check (an impl with no production caller, kept for one test) or go, with the in-process twin `uncontained_supply_is_rejected_by_streaming` carrying position-independence. Acceptance: no adversity, script, or rewrite test constructs `mirror(<RemoteHandshaking>, <materialized>)`; `grep -rn 'MirrorError::Client(' src/tree/mirror/streaming/remote/proxy` matches only materialized-side errors and containment.rs; the owner has recorded the ruling on the remote `Connect` impl.

Ruled (T22): the impls stay. Add a doc at the impls stating that they
exist for the harness's wire-path arrangement and have no production
caller. `remote-proxy-tests-16` (the Server/Client projection) is a P5
entry; collapse the projection only as far as the topology axis forces
it, and say how far.

### remote-proxy-tests-5 (medium): rulings T22, T26

Resolution: Make `harness::drive` the single constructor, parameterized by backend pair (`Local` or `Failing<Local>`, converting the `Root<Failing<Local>>` result as remote-proxy-tests-7 needs), a two-variant topology, payload type, and window, staying generic over the concrete link types; express the five hub helpers as wrappers that only wrap links and choose topology; delete `containment::reconcile_results` in favor of the harness with the asymmetric topology; drop `T` from `reconcile_symmetric_accepts`/`_reordered` (keep it on `reconcile_after_preamble`, whose `u64` caller exists). The codec incantation then has one site in the partition; a `pub(crate)` convenience constructor on `PayloadCodec` is a crate-wide question outside this partition. Acceptance: `RemoteHandshaking::start` has one call site in the partition; `grep -c 'PayloadCodec::new' src/tree/mirror/streaming/remote/proxy/tests.rs src/tree/mirror/streaming/remote/proxy/tests/*.rs` drops to one or two; the suite passes unchanged.

### remote-codec-8 (low): ruling T24

Resolution: In `budget.rs` add `pub(super) fn overbatched(self, body: usize) -> DecodeErrorKind` (or a free function in decode.rs), and in `frame.rs` a `pub(super) const MIN_RECORD_HEADS_LEN: usize = RECORD_TAG_LEN + 1` with a doc naming it as the tag head plus a one-byte byte-string head; have both decoders call them. Hoist `classify` to `decode.rs` as `pub(super)` and route `head_error`'s `Io` arm and the sync `read_exact` through it (`Arrived::short` can call it with a fresh `UnexpectedEof`). Acceptance: `OverbatchedRun { .. }` is constructed in exactly one place; `RECORD_TAG_LEN + 1` appears nowhere as a bare expression; `ErrorKind::UnexpectedEof => DecodeErrorKind::Truncated` appears once; the `decode_both` suites pass.

Ruled (T24): the sync decoder stays as the oracle; give its struct a doc
stating that role in the same change.

### remote-codec-10 (low): ruling T24

Resolution: Make `listing_issue(ListingIssue::Head(head))` use `head_detail(head)` so the two paths agree byte for byte (remote-codec-9 subsumes this by typing the source). Then route `unordered_query_is_rejected` and `empty_query_listing_is_rejected` through `decode_both`, and add a proptest over listing-head spellings (widened key `[0x18, r]` for r < 24, widened value head `[0x59, 0x00, 0x18]`, indefinite `0x5f`, reserved `0x1c`) through `decode_both`. Acceptance: before the one-line fix, the widened-key case panics inside `decode_both` ("the two decoders classify the failure differently"); after it, every listing-defect test passes through `decode_both`, and the atlas snapshot for `query/listing-key` is unchanged (a `Shape` defect, untouched).

`remote-codec-9` (typing the defect sources) is a P6 entry; land the
one-line fix, not the typed variants. The atlas snapshot must not move;
if it does, stop.

### Decision 56 (ruling T19): cancellation and wide-window coverage

No finding id; the ruling adopts two instruments from the review's open
questions (correctness open question 14, verification open question 20):

1. A walk-tier proptest that cancels the `mirror` future at a drawn poll
   count and asserts what must survive cancellation: the local root is
   unchanged (the session installs only at its end). The overlap harness in
   `tests/common/overlap.rs` says dropping an unfinished `Session` models a
   cancelled one; this pin lives in the streaming suites, driven under
   `run_to_quiescence` with a poll budget drawn per case.
2. One wide-window arm in `streaming_matches_join_oracle`
   (`src/tree/mirror/streaming/tests.rs`), so the differential runs the
   window's real behavior and not only `WindowConfig::FLOOR`.

The deterministic cancel-at-every-poll-prefix sweep is declined by the
ruling; do not build it. Negative control for (1): a reversible mutation
that installs before the session completes must fail the pin; report what
mutation you used and its output. Acceptance: both tests committed with
doc comments stating their invariants; (1) fails under the named mutation
and passes on HEAD; (2) runs the oracle at a window wider than the floor
and its width is asserted, not assumed.

## Hazards and stops

- `remote-codec-14` (P2, the exactness clamp) is judged by the
  `decode_both` differential this lane touches; do not change
  `decode_both`'s contract, and do not land the clamp here.
- `src/tree/arb.rs` fixtures are shared with `p1-collision-mode`; that
  lane rebases onto this one.
- Any `insta` snapshot movement is a stop.
- If the inverted `materialized-27` verdict survives the whole committed
  suite except the new tests, say so plainly; that is the expected result
  and the reason the tests exist.
