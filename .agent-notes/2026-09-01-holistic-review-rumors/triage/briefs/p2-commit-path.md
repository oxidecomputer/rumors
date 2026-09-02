<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P2 lane: the commit path

## Goal

Every `send`, `send_all`, `redact`, `Batch`, and gossip commit runs
`Tree::act` or `Tree::join` inside `watch::Sender::send_if_modified`, which
holds the channel's write lock for the whole closure. At the reviewed
commit, user payload destructors run inside that critical section at three
places (the commit-point pre-image drop and two mid-walk last-handle drops
of incoming values), so a `Drop` that touches the same replica hangs
(demonstrated); the act walk sorts and re-materializes its action list at
every height; the leaf stores a running join whose equality with the
applied action's version holds only by a chain property stated nowhere at
the storage site; `ErasedPrefix::assume`'s length check is debug-only; and
`warm_caches` forces three of four memos while claiming every one. The
invariants this lane restores: no user code runs under the replica's lock;
the commit path does its sorting once; a leaf's stored version is the one
its path derives from, by construction; programmer error panics in every
profile; and a calibration hook's contract is its body.

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
  run in the background redirected to a log under `<scratchpad>/p2-commit-path/`,
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

## Ordering inside the lane

1. `async-hazards-3` first (T34): the evacuation changes both commit
   sections, and everything after it is measured or pinned against the
   evacuated shape.
2. `tree-core-27` (T39) with `tree-typed-30` riding along, measured at the
   parent commit before the change and after it.
3. `tree-core-30` (T38), which simplifies the leaf level the single sort
   just touched.
4. `ErasedPrefix::assume` (T36) and `tree-core-8` (T42) at any point.

## Members

### async-hazards-3 (high): ruling T34, resolution amended to full evacuation

Resolution: Two steps, the first required regardless of the second. (1) State under "Choosing a payload type" that `T`'s destructor may run inside the replica's commit critical section on any `send`, `redact`, or gossip commit, must not block, and must not touch a `Rumors`, `Snapshot`, or observer of the same replica (re-entering the replica's lock deadlocks). Add a matching maintainer comment at batch.rs:132 and gossip.rs:816 naming that user code runs under the write guard. (2) Have `Tree::act` and `Tree::join` hand the pre-image root back to the caller (return it alongside the changed flag) so `Batch::commit` and `gossip_inner` drop it after `send_if_modified` returns; `Tree` is private to the crate, so this is an internal signature change, and the replace-assign-then-drop unwind argument is unchanged because the tree is consistent before the drop either way. This removes the bulk deallocation from the lock hold; the mid-walk drops remain inside it, which is why step (1) stays. Evacuating those too (collect skipped and duplicate messages into a sink the walk hands back) is a design proposal for the owner. Acceptance: the payload-type section names the destructor constraint; `drop(pre_image)` (or its equivalent) executes outside both `send_if_modified` closures, checked by a test whose `T: Drop` records whether `inner.borrow()` succeeds during the drop (it must, once the drop is outside the lock).

Amendment (T34): the "design proposal" is adopted, so step (1)'s
constraint is inverted. At both commit sites the pre-image root leaves the
`send_if_modified` closure through a `mut Option` outside it and drops
after the lock is released; the act and join walks push every discarded
incoming value (a causally-skipped action's message in `traverse::act`, a
duplicate or deletion-filtered incoming subtree in `traverse::join`; our
own side's subtrees are safe mid-walk because the pre-image still holds
them) into a sink returned from the closure and dropped after the lock as
well. "Choosing a payload type" states that destructors never run under
the replica's lock; the maintainer comments at the two commit sites say
the same. Amended acceptance: the witness pass's construction
(`evidence/witness.md`, section `async-hazards-3`: a `redact` whose
payload destructor calls `snapshot()` on the same replica) lands as a
committed test that must complete, under a bounded wait; a second test
pins the mid-walk case (a causally-prior action whose message's destructor
touches the replica, applied through a batch, and a join whose incoming
duplicate subtree's destructor does the same); the panic-atomicity pins
(`act_unwind_leaves_tree_byte_identical`,
`act_destructor_unwind_leaves_tree_byte_identical`,
`act_mid_walk_unwind_leaves_tree_byte_identical`, and their join siblings)
still pass. The measurement of a large `send_all` under the lock is P7
(owner decision 64); do not take it here.

### tree-core-27 (medium): ruling T39, single-sort tier only

Resolution: Adoptable now: in `Tree::react` (tree.rs:491-497) sort the collected `Vec` once, stably, by path (`actions.sort_by_key(|(path, ..)| *path)`), state "sorted by `Path<Self>`" as `Act::act`'s precondition, and replace `.sorted_by_key(..).chunk_by(..)` with `.chunk_by(..)` alone; fix the `react` comment to describe the one sort that remains. For the reassembly, merge the two ascending runs (`updated.into_iter().merge_by(existing_children, |a, b| a.0 < b.0)`; radixes are disjoint because each updated child was `remove`d) or insert recursed children back into `existing_children` and drop `updated`, so `Fan::from_iter` takes its one-pass branch and fan.rs:181-183 becomes true. Acceptance: `benches/in_memory.rs` `batch_insert` and `redact` measured at the parent commit and after, identical-or-improved at every N, with fewer allocations (an allocation count per commit can be pinned the way `tests/encode_alloc.rs` pins encode); `react_batch_partitioning_preserves_hash`, `tree_shape_is_canonical_in_the_leaf_set`, and `insert_and_delete_same_batch_is_empty` still pass, proving order semantics survived; a debug assertion or test that the reassembled iterator is ascending.

Amendment (T39): the "adoptable now" tier is the whole of this entry; the
slice-recursion redesign stays with owner decision 64 in P7. The bench
runs are the one timing measurement this lane makes: one run of
`benches/in_memory.rs` at the parent commit and one after, on a machine
you have checked for load (report the load average with the figures);
never iterate on them. Prefer the allocation-count pin as the committed
number, since it is load-independent.

### tree-typed-30 (low): rides with tree-core-27 under T39

Resolution: in act.rs:129 merge the two ascending sequences by radix (`itertools::merge_by`, itertools already being used in act.rs; or `Children::insert` the updated children into `existing_children`, a binary-search insert), which makes the fan doc true; if act.rs stays as is, reword the doc to name it as the site that takes the sort path. Optionally skip the `deduped` rebuild when a post-sort adjacent scan finds no equal radixes. Acceptance: a fan/tests.rs test constructing the `updated ++ existing` interleaving that shows the fast path is taken from act.rs, or the doc reworded to match the code.

The merge branch is the one T39 takes, so the fan doc becomes true; land
the fan/tests.rs test.

### tree-core-30 (low): ruling T38, resolution amended

Resolution: Store the applied action's version: `Some(Node::leaf(version.clone(), value))` (move `version` into the arm; it is not needed after the comparison), keeping `greatest_version` for the observer only; this is equivalent under `act` and makes the path/version coupling hold at the storage site. Then either qualify `react`'s doc ("for a causally ascending sequence at each key, which `act` guarantees") or track the per-key ceiling across a Forget so the stated rule holds for arbitrary sequences. Acceptance: the leaf's stored version is by construction the one its path was derived from; existing `act`/`react` suites unchanged; a committed test pins the two constructions below. Construction: (1) tree holds a leaf at synthetic path `p` with version A@1; `tree.react([(p, B@1, None::<Message>), (p, C@1, Some(msg))])` with A, B, C disjoint parties: the Forget at B@1 is concurrent with A@1, not `<`, so the leaf is removed; the Insert at C@1 lands on `None`; the stored version is `B@1 | C@1`. Assert `tree.iter().next().unwrap().0 == &C@1`, which fails at this commit. (2) fresh tree, `p = Path::for_leaf(&v1)`, `v1 < v2` on one party: `tree.react([(p, v2, None::<Message>), (p, v1, Some(msg))])`. The doc predicts an empty tree; `tree.len()` is 1 and the stored version is `v2`, so `Path::for_leaf(stored) != p`.

Amendment (T38): `greatest_version`, the per-key running join, is removed
outright, not kept for the observer: the ceiling observer already receives
each effectual action's version, and under `Tree::act` the versions at one
key form a causally ascending chain (each action ticks the party in
specification order; an insert's path is fresh from its post-tick version,
so only forgets of an existing path share a key), so the join at a key
equals the last version and a surviving leaf has exactly one. Take the
first branch for `react`'s doc: state that it is `act`'s commit section
over versioned actions and that its contract assumes the ascending per-key
order `act` guarantees. Construction (1) lands and now yields `C@1`.
Construction (2) is outside the stated contract: land it as a test of
what the contract says (documented as such) or make it hold, and report
which. Any further simplification the removal exposes (in `Z::act`, the
observer, or `react`'s signature) is reported, not landed.

### Correctness open question 16: ruling T36

No finding id. The O(1) length-versus-height check in
`ErasedPrefix::assume` (`src/tree/typed/prefix.rs`, the `debug_assert!`
comparing the erased prefix's length against the target height) becomes a
release-profile `assert!` whose message carries the proof that only a
cross-height re-tag can reach it. Acceptance: no `debug_assert!` remains
at that site; a `#[should_panic]` test re-tags across heights and observes
the message; the typed suites pass unchanged.

### tree-core-8 (low): ruling T42

Resolution: Add `let _ = root.version_bytes();` (which forces the bounds span, so the `ceiling`/`floor` calls become redundant and can go), and word the doc as the list of memos it forces so a future memo cannot fall outside "every" unnoticed; mirror the wording at `snapshot.rs:163-165`. Acceptance: a test calls `warm_caches` then observes the memo set (below). Construction: add a `#[cfg(test)]` probe on `untyped::Node` returning whether the branch's `version_bytes` `OnceLock` is populated (`get().is_some()`); build a tree of at least two leaves, call `warm_caches()`, assert the probe is true. The assertion fails at this commit and passes after the fix.

`warm_caches` is `#[doc(hidden)] pub`; its gating is owner decision 19
(P6). Change its body and doc only; its visibility is a stop.

## Hazards and stops

- `Tree::act`, `Tree::join`, and the traversals are crate-private; their
  signatures may change as T34 and T39 need. `Batch::commit`, `Rumors::send`,
  `send_all`, `redact`, and every other `pub` signature are unchanged; a
  change to any is a stop.
- The graveyard sink must not change the panic-atomicity argument: a
  destructor that panics now runs after the commit, with the tree already
  consistent; state that at both sites and keep the three unwind pins
  passing.
- The bench comparison is two runs, parent and child, under a checked
  load; if the machine is loaded, say so with the figures rather than
  re-running.
- A committed `insta` snapshot moving is a stop (none should: the wire is
  untouched).
