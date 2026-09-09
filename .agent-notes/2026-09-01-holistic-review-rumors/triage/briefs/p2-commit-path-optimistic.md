<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from ruling T170 in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P2 lane `p2-commit-path`, re-scope: the optimistic gossip commit

## Goal

No user code runs while the replica's lock is held, and the library
upholds that without saying so publicly: a payload's `Drop` may read or
write the replica, may panic, may do anything, and cannot deadlock the
library. Before this re-scope the property is upheld by a per-node sink
(`Discarded`) that every walk fills and every commit site drops after
the guard. The re-scope upholds the same property by construction: the
gossip commit's join runs outside the lock, so everything a join sets
aside drops outside the lock because it never entered it; the critical
section swaps a root pointer when nothing moved and otherwise retries
outside; the one value any commit displaces under the lock, the
pre-image root, is returned out of the closure and dropped after it.
The sink, its `#[must_use]` plumbing, `Tree::discard`, and the
"Choosing a payload type" guarantee paragraph dissolve. Concurrency
rises as a side effect: joins no longer serialize behind each other or
behind sends for the length of a diff walk.

## Mechanism

- **The gossip commit.** The session already reconciles against an
  O(1) snapshot and produces `merged`. At commit, under
  `send_if_modified`: if the live root is the snapshot's root (pointer
  identity on the root node's `Arc`, and ceiling equality), swap
  `merged` in with `mem::replace`, compute the changed flag as today
  (content changed, ceiling advancing, peer retiring), and return the
  pre-image root out of the closure. Otherwise take a fresh snapshot,
  release, join `merged` into that snapshot outside the lock (a lattice
  join on clones, the same `traverse::join`), and try again with the new
  snapshot as the expected root. The party donation from a retiring
  peer stays inside the swap.
- **Fairness.** After a small fixed number of failed swaps (name the
  constant, state why that number), the committer takes an outer
  `RwLock<()>` exclusively for its next attempt; every fast-path swap
  attempt takes the same lock shared around its critical section, so
  an exclusive holder cannot be raced. No user code runs under the outer
  lock either (it wraps only the swap attempt). Say where the lock lives
  (beside the `watch::Sender`) and that it is never held across an
  await.
- **The local operations** (`send`, `redact`, `Batch`, `act`) keep their
  walk under the lock: they mint versions from the tree's ceiling inside
  the walk, a leaf's path is the hash of its version, and version order
  must equal commit order for deletion honoring, so the walk cannot be
  precomputed. What they displace is the pre-image root (every node the
  walk sets aside is reachable from it after copy-on-write), returned
  out of the closure like the join's. Anything created outside the lock
  that a commit may fail to consume (a batch's actions on an early
  return) is owned outside the closure and borrowed in, so its drop is
  outside by construction; no sink is needed for it.
- **What goes.** `Discarded`, `Tree::discard`, `Discarded::actions`, the
  `#[must_use]` on the sink, and every doc sentence about the sink; the
  public paragraph in "Choosing a payload type" that states the
  guarantee (the expectation is implicit and upheld without comment);
  the maintainer-facing statement of the invariant stays at
  `Inner::commit`, in terms of what IS. Ghost sweep: `git grep -n
  Discarded` and `discard` over `src tests` empty of the old mechanism.
- **Tests.** Keep every pin that holds the property today (the
  deadlocking `Drop`, the panicking destructor, the unwind atomicity
  pins) passing without modification of their assertions. Add: a
  commit whose root moves between snapshot and swap (force it with a
  hold or a concurrent send between the session's end and its commit),
  asserting the retry path is taken and the result equals the in-memory
  join oracle (`Tree::join` on clones); a starvation schedule forcing
  the fixed number of failures, asserting the exclusive path is taken
  and the commit lands with the oracle's result; negative controls in
  the report: the retry join skipped (a stale swap loses a concurrent
  send: the oracle test fails), the escalation removed (the starvation
  test never lands or spins to its bound). Deterministic schedules only
  (the memory network's single-threaded poller); no timing.
- **Meters.** The commit path's exact allocation pins (the three in
  `encode_alloc`) are re-measured at the parent and at the tip; the fast
  path must not allocate more than today; every moved number is
  attributed in the commit message (parent → tip, this change).

## Rulings and rules

T170 (this re-scope); T141 (prose); the documentation policy (public
prose changes toward the code; the removed guarantee paragraph is the
ruled exception, removed at Finch's word). Base: the lane's current tip
`5a4559e1` (the packet commit `14d0d5fb` sits above it; build on
`14d0d5fb`). Every commit `--no-gpg-sign` while the signer is locked,
with annotation rows; the box under the caps, the gate under the mutex
with the pid file once at the end; one full verification run. Report
the sha, each item's disposition, the negative controls quoted, the
meter numbers parent and tip, and the ghost sweep.
