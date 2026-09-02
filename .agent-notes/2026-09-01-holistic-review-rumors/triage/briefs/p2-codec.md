<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P2 lane: the codec's exactness and error order

## Goal

`framing::resume_payload` promises `read_payload`'s exactness but hands
`read_buf` a caller buffer's whole spare capacity, so the codec's
over-budget lone-record path can consume the next frame's bytes
(demonstrated at both altitudes; latent under conforming encoders by one
byte of `Vec` minimum capacity). Separately, the async opener read
discards a transport error that arrives after a partial fill and re-reads,
so the classification depends on what the transport does after failing,
against a doc that promises one-head-at-a-time order. The invariant
restored: every read is bounded by the bytes still owed, and a transport
failure is reported at the item it interrupted, identically in both
decoders.

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
  run in the background redirected to a log under `<scratchpad>/p2-codec/`,
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

## Ordering against P1

Both entries are judged by the `decode_both` differential that
`p1-harness-crate` touches (T24 dissolves the duplicated fragments and
adds the listing-head fixture). Land this lane before `p1-harness-crate`
if it has not started, so that lane's refactor rebases over two small
fixture additions; otherwise rebase onto its landed sha before your gate
run. Say in the report which happened. Do not dissolve any shared fragment
here.

## Members

### mirror-common-8 (medium): ruling T33

Resolution: Make the claim true by construction: bound each read by the bytes still owed, `use bytes::BufMut;` (already a dependency, Cargo.toml:126) and `read.read_buf(&mut (&mut payload).limit(len - payload.len())).await?` (`Limit<&mut Vec<u8>>` is `BufMut`). Keep the growth policy. State the remaining precondition (`payload.len() <= len`) in the doc and check it with an O(1) `debug_assert!` or return `InvalidData` (the sanctioned place for a runtime assert: a cheap spot check at a contract boundary). In framing/tests.rs, add a differential proptest for `resume_payload`: for `prefix_len <= len` and arbitrary prefix capacity, `resume_payload(rest, prefix, len)` equals `read_payload(prefix ++ rest, len)` byte for byte, truncation classification included, with trailing bytes left unread; the suite currently exercises `read_payload` only (:87, :115, :135). Acceptance: a committed test passes `resume_payload` a 3-byte prefix in a `Vec::with_capacity(64)`, `len = 9`, and 16 trailing transport bytes, and asserts the returned payload is exactly 9 bytes and the cursor stops at the trailing bytes; it fails at this commit and passes after the change; `chunked_read_matches_whole_read_reference` still passes.

Ruled (T33): the clamp lives here, in the callee. For the precondition,
return `InvalidData` rather than a `debug_assert!`: a prefix longer than
`len` is a caller bug, but the doctrine puts environmental and boundary
checks in every profile. The witness pass's fixture (`evidence/witness.md`,
section `mirror-common-8`) is the committed negative control.

### remote-codec-14 (medium): ruling T33

Resolution: Clamp the read in `resume_payload` so no iteration can fill past `len`: read through `(&mut payload).limit(len - payload.len())` (`BufMut::limit` caps `chunk_mut`), or shrink so `capacity() <= len` before the loop; the callee is the right home because its documentation already promises exactness without a capacity precondition, and `record_prefix` should not have to know std's minimum capacity. Then add to `overbatched_corners_classify_exactly` (or a sibling) a case decoding, through `decode_both`, a zero-budget frame carrying `raw_record(&[0x00])` followed by a second frame's bytes, asserting the first decode is `Ok` and, through `FrameRead`, that the second frame then decodes cleanly; also feed the same bytes through a chunked reader so the resume path runs under partial delivery. Acceptance: the new test fails at HEAD (the async reader returns `InvalidRun(NotARecord { remaining: 3, .. })` while the sync oracle returns `Ok`, so `decode_both` panics on disagreement) and passes after the clamp; `supply_full_delivery_costs_at_most_payload_plus_chunk` and `overbatched_supply_rejects_without_buffering_its_body` in `tests/decode_alloc.rs` still pass.

One clamp closes both entries; the two fixtures are two tests. The witness
pass's bytes (`evidence/witness.md`, section `remote-codec-14`) are the
fixture, verbatim.

### remote-codec-11 (low): ruling T41

Resolution: Carry `arrived.failure` into the wire-order judgment: parse the bytes in hand and, at the first head that needs more bytes, return `Read { part, source: failure }` instead of re-reading (for example, give `Exact` a pending-failure slot that `fill_exact` surfaces before touching the transport). Add the async failing-reader fixture of remote-codec-15 with a non-sticky shape (`[0x82]`, then `Err(Other)`, then EOF). Acceptance: the non-sticky fixture yields `Read { part: Signal, source: Other }` in both decoders (today the async reader yields `Truncated { missing: Signal }`), and the sticky-error and full-delivery cases are unchanged.

`remote-codec-15` (a P4 verification entry) asks for a general
`FailAfterAsyncReader` and a reader-factory `decode_both`; build only the
fixture this acceptance needs, in the shape that entry describes, so the
P4 lane can generalize it rather than replace it.

## Hazards and stops

- The `decode_alloc.rs` meters have liveness floors; if the clamp changes
  an allocation count, the meter's committed number moves with it in the
  same commit, with the measured value in the message.
- The error atlas snapshots must not move (no classification here
  changes for a delivered, well-formed stream); movement is a stop.
- `resume_payload`'s doc is crate-private prose; the public
  `DecodeErrorKind` docs are unchanged.
