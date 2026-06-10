# Plan: `Broadcast::listen`, an observing subscription

Status: draft for review. Not yet implemented. Builds on the
`shared-state-wip` rework (internally-synchronized `Known`/`Broadcast` over a
`watch::Sender<Inner>`).

## 1. Shape and intent

```rust
impl<T> Broadcast<T> {
    pub fn listen<F, Fut>(self, on_message: F) -> impl Future<Output = ()> + Send
    where
        T: Send + Sync,
        F: FnMut(Key, &Version, &Arc<T>) -> Fut + Send,
        Fut: Future<Output = ()> + Send,
}
```

The returned future owns a private `Tree<T>` (the *observed* tree) and loops:
snapshot the shared tree out of the watch channel, observing-`join` it into
the observed tree (firing `on_message` once per leaf gained), then await the
channel's change notification. It completes when no further change is
possible: when every sender on the channel (the `Known` and all `Broadcast`s)
has dropped.

This is the subscription analogue of `plans/shared-state.md` §3's drain: the
observed tree is the same exclusively-owned dedup cursor, and exactly-once
observation holds for the same reason — the cursor lives behind the future's
unique ownership, so no race can double-observe and no buffer is needed.

The `F`/`Fut` bounds are exactly `Tree::join`'s `R`/`RFut` bounds
(`src/tree.rs:351`), so the callback threads straight through with no adapter;
`&mut F: FnMut` lets each loop iteration pass `Some(&mut on_message)` without
consuming it.

## 2. Verified code facts the design rests on

- **`Tree::join` is an observing CRDT merge.** `on_recv` fires once per leaf
  `self` gains; deletion-honoring drops leaves the other side has redacted
  (version-dominated absences), with no callback for removals
  (`src/tree.rs:340-385`). So the observed tree tracks redactions silently
  and never re-observes.
- **Repeated joins cost O(delta), not O(n).** The join recursion
  short-circuits subtree equality by `ptr_eq`-or-hash, and `OrdMap::diff`
  prunes pointer-equal spans (`src/tree/traverse/join.rs:175-205`). The
  observed tree is built from clones of the shared tree's own nodes, so
  successive wakeups share almost all structure and pay only for what
  changed since the last wakeup.
- **`watch` cannot miss updates.** `Receiver::borrow_and_update` marks the
  current version seen; a write landing after it makes the next `changed()`
  resolve immediately. Borrow-then-wait is the lossless idiom. `changed()`
  returns `Err` exactly when every sender has dropped, after which no write
  can ever occur, so "drain, then `changed() == Err`" observes the final
  state completely.
- **Joins suspend only at user callbacks** (`plans/shared-state.md` §2). The
  listener's join runs against owned/cloned data with no lock held, so the
  user's `on_message` future may suspend arbitrarily long without blocking
  any writer.

## 3. The loop

```rust
pub fn listen<F, Fut>(self, mut on_message: F) -> impl Future<Output = ()> + Send {
    // Subscribe while our own sender still holds the channel open.
    let mut rx = self.known.inner.subscribe();
    // Dissolve the Broadcast eagerly (when `listen` returns, not when the
    // future first polls): the data sender goes, so the channel closes when
    // the last *actor* does; the liveness receiver goes, so a listener does
    // not pin `until_no_broadcasts` (a listener is a pure observer).
    drop(self);
    async move {
        let mut observed = Tree::new();
        loop {
            // Snapshot and release: the watch read guard blocks writers, so
            // it must never be held across an await. Tree clone is O(1) COW.
            let current = rx.borrow_and_update().tree.clone();
            observed
                .join(current, Some(&mut on_message), None::<Silent<T>>)
                .await;
            // Err: every sender is gone, no further change is possible, and
            // the borrow above already drained the final state.
            if rx.changed().await.is_err() {
                break;
            }
        }
    }
}
```

Placement: `src/broadcast.rs`. Everything needed is already in reach
(`known.inner` is `pub(crate)`; `Tree`, `Silent` are crate-internal imports).

## 4. Decisions and rationale

- **Replay from genesis (observed tree starts empty).** The first join fires
  `on_message` for every message live at subscription time, then deltas
  follow. This gives the clean invariant "every message live at some wakeup
  is observed exactly once" and matches §3 of the shared-state plan, where
  bootstrap replays history through the same reducer as live traffic. The
  from-now alternative is the one-line change `observed = current.clone()`
  before the loop; defer until a use case asks for it.
- **A listener is an observer, not an actor.** `listen` consumes its
  `Broadcast` and drops both handles: the data sender (so listeners never
  hold the channel open: termination tracks the *actors*) and the liveness
  receiver (so a listener does not block `until_no_broadcasts`). The XOR
  exists to make `retire` safe against concurrent *sessions and mints*;
  a listener can do neither. Consequences worth documenting on the method:
  the `Known` can be reclaimed and even retired while listeners run, and a
  listener outlives the `Broadcast` generation it was made from, completing
  only when the universe of senders is gone.
- **At-most-once is inherent for redaction-raced messages.** The channel
  holds only the latest tree; a message inserted and redacted between two
  wakeups is never observed. This is a feature: content already redacted is
  never delivered. The delivery contract to document: every message is
  observed at most once; every message live at any wakeup (including the
  final drain) is observed exactly once; redactions are honored silently.
- **Coalescing is the backpressure.** A slow `on_message` never blocks
  writers; the listener simply joins against a fresher snapshot next
  iteration, paying one delta-join regardless of how many writes coalesced.
- **Cancellation: drop means stop.** The observed tree dies with the future;
  no shared state is touched, so there is no partial-state hazard and no
  re-fire obligation (unlike `step`'s callback-then-install dance in the
  shared-state plan, which exists because `step`'s cursor survives the
  cancelled future).
- **Output stays `()`.** Returning the callback or a final `Snapshot` on
  completion is expressible later without breaking the signature's users.

## 5. Risks and verification items

- **`large_futures`.** The crate denies `clippy::large_futures`; the listen
  future embeds the join traversal. If the lint fires, `Box::pin` the async
  block (one allocation per listener, amortized over its life) — same
  treatment as the boxed `reconcile()` sites.
- **`subscribe`-before-drop ordering** is load-bearing: subscribing after
  dropping the last sender would panic/fail. The receiver must be taken from
  `self.known.inner` before `drop(self)`. (Compiler enforces the move order;
  a comment should enforce the why.)
- **Watch-guard discipline**: `borrow_and_update()` result must be bound only
  long enough to clone the tree, never across the join await. Worth a test
  that a listener blocked in `on_message` does not block a concurrent
  `send` (try_send-style timing assertion or a oneshot-gated callback).
- **Termination via retire**: a successful retire drops the `Known`
  (consumed) — confirm the listener then completes once any remaining
  `Broadcast`s drop. No special-casing should be needed; it falls out of
  sender-count-zero.

## 6. Tests (each with its invariant in the doc comment)

1. **Replay**: send N messages, `broadcast()`, `listen` — the listener
   observes exactly the live set, each message once.
2. **Live delivery**: messages sent through a sibling `Broadcast` clone after
   `listen` begins are observed.
3. **Wire delivery**: messages learned via `gossip` are observed (the
   listener doesn't care how the tree advanced).
4. **Redaction honored**: a message observed then redacted is not re-observed
   and produces no callback; a message redacted before `listen` starts is
   never observed.
5. **Exactly-once under interleaving** (prop test): arbitrary interleaving of
   sends/redactions from several `Broadcast` clones with a running listener;
   invariants: no key observed twice, and observed ⊇ the final live set.
6. **Termination**: drop the `Known` and all `Broadcast`s mid-listen — the
   future drains and completes. Variant: the `Known` leaves via `retire`.
7. **Pending while actors live**: with a `Known` or `Broadcast` alive, the
   listen future stays pending (poll via `now_or_never`).
8. **XOR non-interference**: `listen` consuming the last `Broadcast` lets
   `until_no_broadcasts` resolve while the listener is still pending.
9. **Non-blocking observer**: a listener parked inside `on_message` does not
   block `Broadcast::send` on another handle.

Test 5 wants `proptest` over an interleaving script; commit any
`proptest-regressions` seeds it mints.

## 7. Sequencing

1. Implement `listen` in `src/broadcast.rs` (~30 lines; no protocol surface,
   no wire change).
2. Clippy pass; `Box::pin` if `large_futures` objects.
3. Tests, gated on the crate compiling again (the stale `src/sync.rs` /
   `src/tests.rs` still block every test target) — a fresh `tests/listen.rs`
   needs only the lib to build.
4. Doc comments for the delivery contract (§4) when the public API gets its
   documentation pass.
