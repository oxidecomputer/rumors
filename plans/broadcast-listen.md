# Plan: `Broadcast::listen` / `listen_from`, version-cursor subscriptions

Status: implemented on `shared-state-wip` (`ec2915f` the `before::causally`
module, `92a16c9` the listener and range iteration), except for the rumors
test suite (§6), which is gated on the stale `src/sync.rs` / `src/tests.rs`
still blocking every test target. Builds on the in-progress
internally-synchronized `Known`/`Broadcast` rework (`watch::Sender<Inner>`).

An earlier revision of this plan built the pass on the `Unknown` traversal
with a `rebuild: bool` flag; that was superseded before merge by the
version-filtered borrowing iterator (§2), which subsumes it — no consuming
walk, no flag through the wire path, and a public dividend
(`Snapshot::range`). A later revision then retired the callback-driven
`listen`/`listen_from` (and the separate `stream`/`stream_from`) in favor
of `Messages`, the pull-based observer over the frozen walk: pull restores
control flow to the caller, so early exit is the caller's own `break`,
pausing is simply not asking, and the `ControlFlow` apparatus dissolves.

## 1. Shape and intent

```rust
impl<T> Known<T> {  // and identically on Broadcast<T>
    pub fn messages(&self) -> Messages<T>;                      // from genesis
    pub fn messages_from(&self, since: Version) -> Messages<T>; // from a cursor
}

impl<T> Messages<T> {
    /// The lending face: borrows live until the next call.
    pub async fn borrow_next(&mut self) -> Option<(Key, &Version, &Arc<T>)>;
    /// The sound resume point: the last completed pass's frontier.
    pub fn cursor(&self) -> &Version;
}

/// The owned-item face, for select!-and-combinate consumers; the sync
/// mirror exposes the same face as Iterator.
impl<T: Send + Sync + 'static> Stream for Messages<T> {
    type Item = (Key, Version, Arc<T>);
}
```

The cursor is a causal `Version`, not a tree. Each time the shared tree
changes, a version-filtered leaf walk yields precisely the leaves the
cursor does not dominate, then the cursor absorbs the snapshot's ceiling.
The observer is pull-based: early exit is the caller's own `break`,
pausing is holding the observer (its idle state is a constant-size descent
spine), and `borrow_next` resolves `None` once no further change is
possible, having yielded the complete final state. `cursor()` is the
persistable resume point, valid as a later `since` against any replica of
the same network.

Advantages over the earlier tree-cursor design:

1. **Arbitrary starting version.** `Version::new()` is genesis; the
   subscription-time ceiling is from-now; anything else is a resume point.
2. **No pinned structure between passes.** An undriven listener holds a
   `watch::Receiver` and a `Version` — it does not keep old `Arc` clones of
   the tree alive. (A per-pass snapshot clone lives only for the pass.)
3. **Resumability and portability** (§4): cursors compose across listen
   calls, across `ControlFlow::Break` exits (at pass granularity), and
   across replicas.

## 2. The mechanism, as built

- **A `Walk` engine generic over `RangeBounds<Version>`**
  (`src/tree/typed/untyped/node/iter.rs`): the leaf iterator's frontier
  deque, with two shells sharing it — `Iter` (the old unfiltered walk,
  instantiated at `RangeFull`, still an `ExactSizeIterator`, never touching
  a version memo) and `Range` (filtered, upper-bound `size_hint` only).
  Both are borrowing, lazy, and double-ended.
- **Prune/promote by memoized version bounds.** A popped subtree is
  resolved against the range before it is entered: pruned whole when its
  memoized `ceiling`/`floor` prove no leaf can pass, promoted when they
  prove every leaf must (descendants then skip all version comparisons),
  descended undecided only when genuinely straddling a bound. For a leaf —
  whose floor and ceiling are both its version — prune-or-promote is
  exhaustive, so the walk never compares versions leaf-by-leaf. A pass over
  a small delta against a large tree costs the delta plus the pruning
  frontier, not the tree.
- **Range semantics: a difference of causal down-sets.** Keep what the end
  bound contains, subtract what the start bound contains; `Included` vs
  `Excluded` adjusts each at the bound itself. A start bound of either kind
  keeps versions *concurrent* to it; an end bound of either kind drops
  them. The listener's filter is `(Excluded(cursor), Unbounded)`.
- **`before::causally` names every shape** (`crates/before/src/causally.rs`):
  `all()`, `since(&s)` / `not_before(&s)`, `known_at(&e)` / `before(&e)`,
  and the `delta(&s, &e)` / `delta_before(&s, &e)` shorthands, composing in
  any order into a `causally::Range` that implements
  `RangeBounds<Version>`. `Range::contains` is the causal membership
  predicate (shadowing the `RangeBounds::contains` default, whose start
  check drops concurrent versions); `Range::placement_of` totally orders a
  `Version` against a range — `Less` / `Equal` (contained) / `Greater` —
  with the totality carried by the bare `Ordering` return type. No operator
  overloads: cross-type `PartialEq` meaning membership would violate the
  trait's transitivity contract. Unit-tested and doctested in `before`,
  which compiles independently of the rumors blockers.
- **Surfaced as `Tree::range` and the public `Snapshot::range`**, named for
  the `BTreeMap::iter`/`range` precedent, accepting range syntax
  (`&v1..=&v2`), `causally` constructors, and `Bound` tuples alike.
- **Frames are allocation-free**: each carries its path bytes in an inline
  `ArrayVec<[u8; 32]>` (the `Prefix` shape; depth is fixed at 32), so the
  walk allocates nothing but its frontier deque.
- **`watch` cannot miss updates.** `borrow_and_update` marks seen; a write
  after it makes the next `changed()` resolve immediately; `changed()`
  errors exactly when every sender is gone, after which no write can occur.
  Borrow-then-drain-then-wait therefore observes the complete final state.

## 3. The loop (as implemented in `src/broadcast.rs`)

```rust
pub fn listen_from<B, F, Fut>(self, since: Version, mut on_message: F)
    -> impl Future<Output = (Version, Option<B>)> + Send
{
    // Subscribe while our own sender still holds the channel open, then
    // dissolve the Broadcast eagerly: the data sender goes (the channel
    // closes when the last *actor* does), and the liveness receiver goes
    // (a listener must not pin `until_no_broadcasts`).
    let mut rx = self.known.inner.subscribe();
    drop(self);
    async move {
        let mut cursor = since;
        loop {
            // Snapshot under the guard and release immediately: the guard
            // blocks writers and must never be held across an await. The
            // tree clone is a cheap copy-on-write handle.
            let snapshot = rx.borrow_and_update().tree.clone();
            let ceiling = snapshot.latest().clone();

            // Fire for precisely the leaves `cursor` does not dominate.
            // (`since`, not `not_before`: a leaf at exactly the cursor was
            // observed by the pass that absorbed it, and must not re-fire.)
            // A break resolves with the pass-start cursor — the last
            // causally closed frontier (§4) — so the filter borrows a
            // clone, letting the break move the cursor out.
            let pass = cursor.clone();
            for (key, version, message) in snapshot.range(causally::since(&pass)) {
                if let ControlFlow::Break(value) = on_message(key, version, message).await {
                    return (cursor, Some(value));
                }
            }

            // The pass observed every survivor at or under the snapshot's
            // ceiling; absorbing the ceiling (not just observed leaf
            // versions) also covers redaction ticks, which have no leaves.
            cursor |= &ceiling;

            // Err: every sender is gone, the drain above was final.
            if rx.changed().await.is_err() {
                break (cursor, None);
            }
        }
    }
}
```

Because the walk is a synchronous iterator, the listener needs no async
recursion and no `Box::pin`; the crate's `deny(clippy::large_futures)`
stayed quiet.

## 4. Decisions and rationale

- **The cursor is a `Version`, committed per pass.** During a pass the
  filter is the pass-start cursor, so intra-pass ordering doesn't matter;
  at pass end `cursor |= ceiling` marks everything in the snapshot
  observed. Exactly-once within one listen call: every fired leaf is `<=`
  the absorbed ceiling, and the next pass filters against it.
- **Resumption is sound only at pass granularity.** A `Version` can encode
  only a *causally closed* frontier, and delivery is in key order, not
  causal order — so the prefix delivered before a mid-pass stop need not be
  causally closed, and no `Version` can cover exactly it. In particular
  folding the delivered versions (`resume |= version`) is **unsound**: a
  fold can causally contain a message that was never delivered (deliver
  `m2@(A:2)` before `m1@(A:1)` in key order, stop between them — the fold
  covers `A:1`), which a resume would then skip *forever*. Loss, not
  re-delivery. `Messages::cursor()` therefore exposes the last completed
  pass's frontier: resuming from it is at-least-once for the in-progress
  pass (dedup by `Key` if re-delivery matters), exactly-once everywhere
  else. In-process pause needs no cursor at all — hold the observer, whose
  idle state is constant-size. Mid-pass exactness would require delivering
  each pass in a linear extension of the causal order (any prefix of a
  topological order is causally closed), at the cost of materializing and
  sorting every delta by partial-order comparison — a possible future
  opt-in, not the default.
- **Cursors are portable across replicas.** A `Version` is causal and
  network-global, not replica-local: a cursor earned against replica A is a
  valid `since` against replica B's `Broadcast` in the same universe, and
  the terminal `Output` of one listener can seed a listener elsewhere.
  Messages observed via A are dominated and skipped; messages B holds that
  A never saw are concurrent to the cursor and fire. The sharp edge: a
  `Version` from a *different universe* is meaningless and undetectable
  (`Version` carries no `Network`); same-network `since` is the caller's
  obligation, an instance of the crate's existing Law-of-Disjointness
  rules. A *future* version (ahead of this replica) is legal: nothing
  fires until causality catches up.
- **A listener is an observer, not an actor.** `listen_from` consumes its
  `Broadcast` and drops both handles up front: the data sender (listeners
  never hold the channel open; termination tracks the actors) and the
  liveness receiver (listeners do not block `until_no_broadcasts`). The
  XOR exists to make `retire` safe against concurrent sessions and mints;
  a listener can do neither. The `Known` can be reclaimed, even retired,
  while listeners run.
- **At-most-once is inherent for redaction-raced messages.** The channel
  holds only the latest tree; a message inserted and redacted between two
  passes is never observed — content already redacted is never delivered.
  The delivery contract: every message is observed at most once per call;
  every message live at any pass whose version exceeds `since` is observed
  exactly once; redactions are honored silently. This rides the same
  version-bounds discipline as the crate's tombstone-free deletion.
- **Iteration and delivery order are key order, not causal order** —
  documented at every layer where the assumption could form
  (`Snapshot::iter`, `Snapshot::range`, `Broadcast::listen_from`, and the
  internal iterators): keys are content-derived hashes, so a message may be
  yielded or delivered before one that causally precedes it, and filtering
  by versions does not mean yielding in version order. Callers order by the
  yielded `Version`s if causality matters.
- **Coalescing is the backpressure.** A slow `on_message` never blocks
  writers; the next pass runs against a fresher snapshot and pays one
  delta-walk regardless of how many writes coalesced.
- **Termination is real and meaningful: `Output = Version`, not a never
  type.** The future completes exactly when every sender is gone: the
  `Known`'s (in hand or parked in a pending `until_no_broadcasts`), every
  `Broadcast` clone's, and the transient `PartyGuard` clones (inside gossip
  futures that borrow an actor). Dropping our own sender up front is the
  linchpin: holding it would self-pin the listener and deadlock listeners
  against each other. Completion guarantees the complete final state was
  observed, and the returned cursor (the final absorbed ceiling) is the
  proof — feed it to a later `listen_from` anywhere in the universe. A
  never type (`Output = Infallible`) would promise the universe outlives
  the listener, which retiring or dropping the last `Known` falsifies.
- **Naming: `range`, not `iter_between`.** `BTreeMap` has exactly this
  method pair — `iter()` for everything, `range(R: RangeBounds<K>)`
  filtered, with the same `ExactSizeIterator` asymmetry — and the filtered
  iterator struct is likewise `Range`, mirroring `btree_map::Range`. The
  "range over which dimension?" question has only one answer here: keys
  are uniformly random hashes, so the causal order is the only meaningful
  order in the domain.

## 5. Risks and verification items

- ~~`large_futures`~~: moot — the pass is a synchronous iterator; no async
  recursion in the listener.
- **Subscribe-before-drop ordering** is load-bearing and implemented with
  the why in a comment: the receiver is taken from `self.known.inner`
  before `drop(self)`.
- **Watch-guard discipline** is implemented: the `borrow_and_update` guard
  lives only for the tree clone, never across an await. Test 12 (§6)
  pins the observable consequence.
- **Per-pass snapshot pinning**: during a pass the snapshot clone (and its
  leaf `Arc`s) stay alive across the user's callback awaits. If that ever
  matters, the collect-then-fire pattern from `Tree::act` (eagerly collect
  `(Key, Version, Arc<T>)`, drop the snapshot, then await) trades it for
  materializing the delta. v1 fires inline.
- **Termination via retire**: confirm the listener completes once the
  retired `Known` and remaining `Broadcast`s are gone; should fall out of
  sender-count-zero with no special-casing (test 9).

## 6. Tests (each with its invariant in the doc comment)

Done: `before::causally` is unit-tested and doctested in `before`
(constructor bound pairs, order-agnostic composition, latest-wins
rebinding, the concurrent-version asymmetry on both bound sides,
delta-as-composition, genesis edges, placement totality and `contains`
agreement). The rest below is gated on the rumors lib compiling again.

1. **Genesis replay**: send N, `broadcast()`, `listen` — observes exactly
   the live set, each once; returned cursor dominates every observed
   version.
2. **Arbitrary start**: `listen_from(v_mid)` observes exactly the leaves not
   dominated by `v_mid`.
3. **Live delivery**: messages sent through a sibling clone after listening
   begins are observed; gossip-learned messages likewise.
4. **Redaction honored**: observed-then-redacted fires nothing further;
   redacted-before-listen never fires; a from-now listener does not see
   pre-subscription content.
5. **Exactly-once under interleaving** (prop test): arbitrary send/redact
   interleavings from several clones; no key observed twice; observed ⊇
   final live set above `since`.
6. **Cursor-resume** (prop test): stop an observer at an arbitrary point,
   resume a fresh one from its `cursor()`; the union of observations covers
   every message (nothing lost), and re-deliveries occur only for messages
   delivered in the interrupted pass. Negative control: assert that folding
   delivered versions and resuming from the fold *can* lose a message (the
   causal-closure counterexample), pinning why the API exposes the pass
   cursor instead.
7. **Cursor round-trip**: terminal `Output` fed to a fresh `listen_from` on
   an unchanged set observes nothing and terminates with an equal cursor.
8. **Replica portability**: cursor earned on replica A, resumed against
   replica B post-gossip: A-observed messages are skipped, B-only messages
   fire.
9. **Termination**: drop the `Known` and all `Broadcast`s mid-listen — the
   future drains and completes. Variant: the `Known` leaves via `retire`.
10. **Pending while actors live**: with a `Known` or `Broadcast` alive, the
    future stays pending (`now_or_never`).
11. **XOR non-interference**: `listen` consuming the last `Broadcast` lets
    `until_no_broadcasts` resolve while the listener is still pending.
12. **Non-blocking observer**: a listener parked in `on_message` does not
    block `Broadcast::send` on another handle.
13. **`Snapshot::range` against the naive filter** (prop test): for
    arbitrary trees and bound pairs, `snapshot.range(r)` yields exactly
    `snapshot.iter().filter(|(_, v, _)| r.contains(v))` — the prune/promote
    shortcuts are pure optimization. This is the differential test for the
    `Walk` engine's partial-order reasoning.

Tests 5, 6, and 13 want `proptest` over interleaving scripts / arbitrary
trees; commit any `proptest-regressions` seeds they mint.

## 7. Remaining work

1. Tests (§6), gated on the crate compiling again (the stale `src/sync.rs`
   / `src/tests.rs` still block every rumors test target); a fresh
   `tests/listen.rs` needs only the lib to build.
2. Profile-driven only: collect-then-fire if snapshot pinning across slow
   callbacks shows up.
3. The public-API documentation pass: the delivery contract, the resume
   recipe, and the same-universe obligation (§4) are documented on the
   methods; `before`'s crate-level docs should mention `causally` when that
   crate gets its pass.
