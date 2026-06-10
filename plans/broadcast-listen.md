# Plan: `Broadcast::listen` / `listen_from`, version-cursor subscriptions

Status: draft for review. Not yet implemented. Builds on the in-progress
internally-synchronized `Known`/`Broadcast` rework (`watch::Sender<Inner>`).

## 1. Shape and intent

```rust
impl<T> Broadcast<T> {
    /// Observe every message from genesis onward.
    pub fn listen<F, Fut>(self, on_message: F) -> impl Future<Output = Version> + Send
    where
        T: Send + Sync,
        F: FnMut(Key, &Version, &Arc<T>) -> Fut + Send,
        Fut: Future<Output = ()> + Send,
    {
        self.listen_from(Version::new(), on_message)
    }

    /// Observe every message not causally contained in `since`.
    pub fn listen_from<F, Fut>(self, since: Version, on_message: F)
        -> impl Future<Output = Version> + Send
    where /* same bounds */
}
```

The cursor is a causal `Version`, not a tree. Each time the shared tree
changes, the [`Unknown`] traversal fires `on_message` for precisely the
leaves the cursor does not dominate, then the cursor absorbs the snapshot's
ceiling. The future resolves — when no further change is possible — to the
version up to which it has processed, which is a valid `since` for a later
`listen_from`.

Advantages over the earlier tree-cursor design:

1. **Arbitrary starting version.** `Version::new()` is genesis; the
   subscription-time ceiling is from-now; anything else is a resume point.
2. **No pinned structure between passes.** An undriven listener holds a
   `watch::Receiver` and a `Version` — it does not keep old `Arc` clones of
   the tree alive. (A per-pass snapshot clone lives only for the pass.)
3. **Resumability and portability** (§4): cursors compose across listen
   calls, across cancellation (via a caller-side fold), and across replicas.

## 2. Verified code facts the design rests on

- **`Unknown` already takes async callbacks and prunes by version.**
  `Unknown::unknown(node, prefix, known, &mut Option<F>)` with
  `F: FnMut(Key, &Version, &Message<T>) -> Fut`, recursion Box::pin'd for
  `Send` (`src/tree/traverse/unknown.rs:64`). A subtree with
  `ceiling() <= known` returns `None` immediately — dominated subtrees are
  never entered, so a pass costs O(new leaves · depth), not O(n).
- **The keep-whole fast path is already non-rebuilding.** A subtree whose
  floor the cursor does not dominate is observed by a *read-only* leaf walk
  and returned verbatim as an `Arc` move (`unknown.rs:100-124`). Only the
  mixed known/unknown case destroys-and-rebuilds; running the walk over a
  disposable per-pass clone makes that wasted allocation, never
  incorrectness. A borrowing `&Node` variant is a later optimization, taken
  only if profiling asks (§7).
- **The callback adapter exists.** `unknown::from_arc` adapts the public
  `FnMut(Key, &Version, &Arc<T>) -> Fut` shape to the `&Message<T>` shape
  the walk fires (`unknown.rs:20`).
- **`Version::new()` is genesis** (`before::Version`, with `Default`), `|=`
  is the least-upper-bound absorb, and `<=` is causal containment.
- **`watch` cannot miss updates.** `borrow_and_update` marks seen; a write
  after it makes the next `changed()` resolve immediately; `changed()`
  errors exactly when every sender is gone, after which no write can occur.
  Borrow-then-drain-then-wait therefore observes the complete final state.

## 3. The loop

```rust
pub fn listen_from<F, Fut>(self, since: Version, mut on_message: F)
    -> impl Future<Output = Version> + Send
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
            // clone is disposable, so the consuming walk below is fine.
            let snapshot = rx.borrow_and_update().tree.clone();
            let ceiling = snapshot.latest().clone();

            // Fire for precisely the leaves `cursor` does not dominate.
            let mut observe = Some(unknown::from_arc(&mut on_message));
            Unknown::unknown(snapshot.root.into(), Prefix::new(), &cursor, &mut observe)
                .await;

            // The pass observed every survivor at or under the snapshot's
            // ceiling; absorbing the ceiling (not just observed leaf
            // versions) also covers redaction ticks, which have no leaves.
            cursor |= &ceiling;

            // Err: every sender is gone, the drain above was final.
            if rx.changed().await.is_err() {
                break cursor;
            }
        }
    }
}
```

Placement: `src/broadcast.rs`. `Unknown`, `Prefix`, and `from_arc` are
crate-internal; expect a couple of small visibility/import adjustments.

## 4. Decisions and rationale

- **The cursor is a `Version`, committed per pass.** During a pass the
  filter is the pass-start cursor, so intra-pass ordering doesn't matter;
  at pass end `cursor |= ceiling` marks everything in the snapshot
  observed. Exactly-once within one listen call: every fired leaf is `<=`
  the absorbed ceiling, and the next pass filters against it.
- **Cancellation loses the closed-over cursor — by design, with a
  documented recipe.** A dropped future cannot return its `Version`. The
  resume recipe falls out of the signature: the callback receives
  `&Version`, so a caller wanting cancel-resume folds `resume |= version`
  as it processes; `listen_from(resume)` then re-fires nothing already
  folded (every fired leaf is `<=` the fold by construction). The single
  in-flight message is the caller's choice: fold-before-side-effect is
  at-most-once, fold-after is at-least-once. The fold sits under the true
  ceiling (redaction ticks aren't folded), which only weakens pruning for
  the first resumed pass; the pass-end absorb catches the cursor up. No
  `&mut self`-resumable variant: with per-pass commits it re-fires whole
  passes, and persisting a cursor inside the `Broadcast` buys nothing the
  fold doesn't.
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

## 5. Risks and verification items

- **`large_futures`**: `Unknown` Box::pins its own recursion, but the listen
  future still embeds a pass; if the crate's deny lint fires, `Box::pin`
  the async block (one allocation per listener).
- **Subscribe-before-drop ordering** is load-bearing: the receiver must be
  taken from `self.known.inner` before `drop(self)`.
- **Watch-guard discipline**: the `borrow_and_update` guard is bound only
  long enough to clone the tree, never across the walk.
- **Per-pass snapshot pinning**: during a pass the snapshot clone (and its
  leaf `Arc`s) stay alive across the user's callback awaits. If that ever
  matters, the collect-then-fire pattern from `Tree::act` (eagerly collect
  `(Key, Version, Arc<T>)`, drop the snapshot, then await) trades it for
  materializing the delta. v1 fires inline.
- **Termination via retire**: confirm the listener completes once the
  retired `Known` and remaining `Broadcast`s are gone; should fall out of
  sender-count-zero with no special-casing.

## 6. Tests (each with its invariant in the doc comment)

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
6. **Resume recipe** (prop test): cancel a listener at an arbitrary point
   (oneshot-gated callback), fold observed versions, `listen_from(fold)`;
   union of observations is exactly-once per message (fold-before-effect
   discipline).
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

Tests 5 and 6 want `proptest` over interleaving scripts; commit any
`proptest-regressions` seeds they mint.

## 7. Sequencing

1. Implement `listen_from` + `listen` in `src/broadcast.rs` over the
   existing consuming `Unknown` walk (~40 lines; no protocol surface, no
   wire change). Small visibility adjustments for `Unknown`/`Prefix`/
   `from_arc` as needed.
2. Clippy pass; `Box::pin` if `large_futures` objects.
3. Tests, gated on the crate compiling again (the stale `src/sync.rs` /
   `src/tests.rs` still block every test target); a fresh `tests/listen.rs`
   needs only the lib to build.
4. Profile-driven only: a borrowing, non-rebuilding `&Node` unknown-walk if
   per-pass rebuild allocation shows up; collect-then-fire if snapshot
   pinning across slow callbacks shows up.
5. Doc comments for the delivery contract, the resume recipe, and the
   same-universe obligation (§4) when the public API gets its
   documentation pass.
