# Sweep async-hazards: Async and concurrency hazards

## Method and coverage

This is the verification pass over the async-hazards sweep at commit
9e5784fb4dce977cfbdfd1619886d1482b5ce764 (working tree clean, checked with
`git rev-parse HEAD` and `git status --short`). For each of the sweep's six
findings I opened every cited site with line numbers (`awk`/`sed -n`) and
compared the sweep's quotations against the file; every quotation matched.
Beyond the cited lines I read:

- `src/peer/gossip.rs` 525-890 and 1367-1404 (`bookmark_update`,
  `bookmark_donate`, `gossip_inner` end to end, `PartyGuard`), `Retire` at
  gossip.rs:87-134, `Peer::retire` docs at peer.rs:281-290.
- `src/tree.rs` 490-620, `src/tree/traverse/act.rs` 140-170,
  `src/tree/traverse/join.rs` 60-72, `src/tree/tests.rs` 1685-1700 and the
  four unwind-atomicity test names at 1505, 1554, 1697, 1746.
- The pinned dependency source: Cargo.lock pins `tokio 1.52.3` (the sweep
  checked 1.53.1); I read `send_if_modified`, `send_modify`, and both
  `borrow` methods in `~/.cargo/registry/src/*/tokio-1.52.3/src/sync/watch.rs`
  (lines 1104-1112, 1171-1211, 629-638, 1253-1260).
- `src/tree/mirror/streaming/materialized.rs` 468-595,
  `materialized/common.rs` 1-60, `backend/local.rs` 125-150,
  `backend.rs` 390-414, `materialized/work.rs` 80-153, `tasks.rs` 1-59,
  `materialized/work/levels.rs` 105-125, 200-220, 380-460,
  `remote/proxy/work.rs` 120-140.
- `src/rumors/causal.rs` 15-110 and 165-200; `src/lib.rs` 225-270 and
  307-349 (module and re-export roster: `mod tree;` is private, only
  `MERKLE_HASH_LEN` and `SessionStats` are re-exported from it).

Mechanical checks: `grep -rn 'select!'` over `src/` (six `tokio::select!`
sites plus one doc mention, matching the sweep's roster);
`grep -rn 'poll_fn\|impl.*Stream for\|impl.*Drop for\|impl.*Future for'`;
`grep -rn 'send_if_modified\|send_modify'` (four production sites:
batch.rs:132, gossip.rs:538, 682, 816, plus `send_modify` at gossip.rs:1388);
`grep -rn 'spawn'`, `warm_caches`, `before any wire traffic`,
`recoverable only through` over `src/`, `tests/`, `.agent-notes/`;
`git log -S` for both disputed phrases; `git show 3f33afabf:src/peer/gossip.rs`
to check the await order at the commit that introduced the bookmark gate;
`git merge-base --is-ancestor f6cf25791 3f33afabf` (the preamble commit is an
ancestor of the bookmark-gate commit). I read the two review packets'
mentions of "wire traffic" and "carve-out"
(`.agent-notes/2026-07-23-review-link-transport/review-link-transport-branch.md`
lines 170-190, 600-630, 740-765): both concern other matters (a ghost
reference, and R54's epilogue-confirmation carve-out), so no prior ruling
covers these findings.

I ran none of the two permitted `cargo nextest` invocations: no existing test
settles any of the claims, and constructing the deadlock in finding 3 needs a
new test file, which I may not write. Every claim below is therefore
"verified" only in the sense of source reading and mechanical grep/history
checks; nothing was executed. After this report was finalized, the second
witness pass ran finding 3's construction as a new integration binary
(tests/zz_witness_drop_reentry.rs, deleted after the run): `redact` did not
return within 10 s, and the entry below carries the outcome and its raised
severity. I did not re-verify the sweep's positives that
lie outside the cited files (the observers' owned-wait design, the `Extant`
token protocol, the wire-fed buffer bounds); those are carried as
sweep-reported.

## Findings

### async-hazards-1: Bookmark gate is documented as preceding all wire traffic, but the preamble exchange precedes it
- Where: src/bookmark.rs:51-53 (related: src/rumors.rs:486-488, src/peer/gossip.rs:667-668, src/peer/gossip.rs:625-629, src/peer/gossip.rs:677, src/tree/mirror/handshake.rs:3-10)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (traced the await order in `gossip_inner`: `handshake::preamble(..).await` at 626 completes before `self.bookmark.lock().await` at 677 and `bookmark.write().await` at 713; checked `git show 3f33afabf:src/peer/gossip.rs`, the commit that introduced both the gate and the comment: the preamble was already at its line 582, the lock at 637)
- Verification: confirmed; history: no-rationale-found (the sentence was inaccurate when written, not drifted into)
- Owner-gated: no

Three sites say a bookmarked peer's sessions queue at the bookmark lock
"before any wire traffic". `gossip_inner` exchanges the fixed 30-byte
preamble with the peer first and takes the bookmark lock afterwards, so a slow
or failing store is observable on the wire: the peer's preamble has been
consumed and answered, and the peer waits for our greeting. The ordering is
deliberate (handshake.rs:9-10: the provider must learn whether the peer is
bootstrapping before it snapshots and forks), so the prose is what moves. The
other "before any wire traffic" phrases in the tree (error.rs:170,
gossip.rs:434, src/tests.rs:447 and 474) describe the `LinkPoisoned`
fail-fast, which does run before any byte, and are accurate.

Evidence:

    src/bookmark.rs
    51	/// A slow store delays session *starts* (sessions queue at the peer's
    52	/// bookmark lock before any wire traffic), never a `send` and never a
    53	/// session's in-flight wire progress.

    src/rumors.rs
    486	    /// which the `&mut` borrow enforces; a bookmarked peer's sessions also
    487	    /// queue at the bookmark lock before any wire traffic
    488	    /// ([`Bookmark`]).

    src/peer/gossip.rs
    625	        let remote =
    626	            match handshake::preamble(self.network, intent, staged, read, write, &observe).await {
    ...
    667	        // The lock order is bookmark-then-`watch`, as everywhere. A failed
    668	        // record write aborts the session before any wire traffic: dropping
    ...
    677	            let mut bookmark = self.bookmark.lock().await;

    src/tree/mirror/handshake.rs
    3	//! Every wire session first exchanges one fixed-size [`Preamble`] carrying
    4	//! the wire dialect's version, the network, and the session intent. Only
    5	//! after it succeeds does the mirror exchange its greeting, which
    ...
    9	//! Keeping these phases separate permits a provider to learn that its peer is
    10	//! bootstrapping before it atomically snapshots the tree and forks its party.

Resolution: At the two public sites (bookmark.rs:51-53, rumors.rs:486-488)
state the true position: sessions queue at the bookmark lock after the fixed
preamble exchange and before the greeting, so a slow store delays the peer's
greeting and a failed store fails a session the peer has already entered (the
peer abandons it by its own timeout, link poisoned). At gossip.rs:668 replace
"before any wire traffic" with "before the greeting". Acceptance: `grep -rn
'before any wire traffic' src/` returns only the `LinkPoisoned` fail-fast
sites (error.rs, gossip.rs:434, src/tests.rs), and the two public sentences
name the preamble.

### async-hazards-2: The retire cancellation carve-out promises bookmark recovery in the window where none exists
- Where: src/link.rs:313-316 (related: src/link.rs:318-321, src/peer/gossip.rs:773-782, src/bookmark.rs:363-364, src/peer/gossip.rs:108-110 and 121-124, src/peer/gossip.rs:1384-1404, tests/retire.rs, src/tests.rs:47-56)
- Class / severity / confidence: documentation / medium / high
- Provenance: assessed (read)
- Verification: reframed: the sentence is accurate over the window where `Err` would return `Retire::Recovered` and inaccurate over the window where `Err` would return `Retire::Uncertain`, and it does not scope itself; history: no-rationale-found (`git log -S'recoverable only through'` finds no commit touching the phrase under the paths searched, and the link-transport review packet's carve-out remarks concern the epilogue confirmation, R54)
- Owner-gated: no

The carve-out says a dropped `retire` future "loses the identity (recoverable
only through an attached bookmark)". `bookmark_donate` slices the whole party
out of the record and persists (gossip.rs:774, bookmark.rs:363-364) before
`party::send` runs (gossip.rs:782). A drop landing after that persist commits
(the tail of `bookmark_donate`, `party::send`, or the epilogue wait) destroys
the `Peer` with the party recorded in no bookmark on our side; it survives
only if the counterparty received and committed it. That is the same loss
`Retire::Uncertain` documents for `Err` in the same window, so the sentence's
contrast ("where `retire`'s `Err` would have handed the peer back") also holds
only for the `Recovered` window. The doc immediately below tells callers to
wrap sessions in a timeout and treat expiry as cancellation; a timeout fires
most plausibly while awaiting the stalled peer's epilogue, which is inside the
misdescribed window. No test cancels a `retire` mid-flight: `tests/retire.rs`
has no drop, cancel, or bounded-poll site (grep for
`drop|cancel|abort|poll|mid` is empty), and every `retire` in `src/tests.rs`
runs to completion under `tokio::join!`.

Evidence:

    src/link.rs
    313	///   carve-out: a `retire` future owns its consumed [`Peer`](crate::Peer),
    314	///   so dropping it destroys the peer and loses the identity (recoverable
    315	///   only through an attached bookmark), where `retire`'s `Err` would have
    316	///   handed the peer back through [`Retire`](crate::Retire)'s variants.
    ...
    318	/// No session imposes its own deadline: against a stalled peer a session
    319	/// waits forever, so the *caller* owns the timeout. Wrap sessions in your
    320	/// runtime's timeout and treat expiry as any other cancellation: replica
    321	/// intact or fully committed, link poisoned, reconnect.

    src/peer/gossip.rs
    773	            let donated = guarded.party.as_ref().expect("is_some");
    774	            if let Err(e) = self.bookmark_donate(donated).await {
    775	                return (Intent::Remain, Err(Error::Bookmark(e)));
    776	            }
    ...
    781	            let donated = guarded.party.take().expect("is_some");
    782	            match party::send(donated, write, &observe).await {

    src/bookmark.rs
    363	    /// Slice the donated `party` out of the record, since it has now left for
    364	    /// the network.

    src/peer/gossip.rs (Retire variants)
    108	    /// **Recovered, unchanged.** The session failed *before* our identity
    109	    /// ever crossed the wire; the replica is handed back intact, to try
    110	    /// retiring elsewhere.
    ...
    121	    /// **Uncertain.** The session failed while our identity itself was in
    122	    /// flight: the peer may or may not hold it, so our peer is consumed
    123	    /// rather than risk the same identity living twice. The link is
    124	    /// poisoned; discard it.

Resolution: Reword the carve-out to scope itself by the `Retire` variant the
same failure would have produced: where `Err` would return `Recovered`, a drop
instead destroys the peer and the identity survives only in an attached
bookmark; where `Err` would return `Uncertain` (the donation persisted, the
party frame or the epilogue in flight), a drop loses the identity exactly as
`Uncertain` does, and the local bookmark no longer records it. Add a test that
drives `retire` against a counterparty with the bounded-poll harness
`tests/lifecycle.rs` already uses for mid-descent cancellation, cuts at each
of the three await points (before `bookmark_donate`, after it, during
`party::send`), and asserts the record's contents and the counterparty's party
at each cut. Acceptance: the sentence names both windows, and a committed
test under `tests/` cancels `retire` after the donation persist and asserts
the bookmark record no longer contains the party.
Construction: attach an in-memory `Bookmark` to a peer, start `retire` against
a counterparty over `link::memory`, poll both manually until the retiree's
`bookmark_donate` write has completed but `party::send` has not (stall the
retiree's control write), drop the retire future, then load the bookmark
bytes: the retiree's party is absent (sliced) and no live handle holds it.

### async-hazards-3: User payload destructors run inside the watch write-lock critical section at both commit sites
- Where: src/tree.rs:531-539 (related: src/tree.rs:602-611, src/tree.rs:507-513, src/tree/traverse/act.rs:150-159, src/tree/traverse/join.rs:64-66, src/batch.rs:132-144, src/peer/gossip.rs:816-871, src/peer.rs:678-680, src/lib.rs:238-264)
- Class / severity / confidence: correctness / high / high
- Severity: raised from medium to high in reconciliation after the second witness pass demonstrated a caller-reachable hang in production code; the sweep and the refutation pass had rated it medium.
- Provenance: demonstrated (second witness pass: through the public API alone, `redact` of a payload whose `Drop` calls `snapshot()` on a clone of the same `Rumors` handle entered the destructor and had not returned after 10 s, `snapshot()` never returning; before the pass, verified by reading `send_if_modified` in the pinned tokio 1.52.3 source, where the closure runs under `self.shared.value.write()`, `Peer::snapshot`, which takes `self.inner.borrow()`, a `read()` on the same lock, and the mid-walk drop sites in `traverse::act` and `traverse::join`)
- Verification: reframed: the pre-image drop is one of three destructor sources inside the critical section, not the only one, so deferring it shrinks the lock hold but does not lift the re-entrancy constraint; history: no-rationale-found (`.agent-notes/` mentions of "critical section" and "destructor" are in the streaming-latency design note and concern other matters)
- Owner-gated: no

`Batch::commit` and the session commit call `Tree::act`/`Tree::join` inside
`watch::Sender::send_if_modified`, which holds the channel's `RwLock` write
guard for the whole closure. Inside that closure, `T` destructors run in three
places: the mid-walk drop of a causally-prior action's message
(act.rs:150-159, `continue`), the mid-walk drops deletion honoring and the
duplicate-subtree arm perform in `traverse::join` (join.rs:64-66), and the
commit point's `drop(pre_image)`, where everything the batch or merge
displaced becomes uniquely held (tree.rs:531-539, 602-611). A `T: Drop` that
touches the same replica (`snapshot()`, `send()`, any `inner.borrow()`)
re-enters the `RwLock` from the thread holding its write guard: std's `RwLock`
either deadlocks or panics there. Separately, the lock hold includes the
cascading deallocation of everything a `redact_all` or deletion-honoring join
removed, during which every `snapshot()` and observer poll on other threads
blocks. `Tree` is lock-agnostic and says only that the drop "runs user code";
the two callers that own the lock say nothing, and "Choosing a payload type"
(lib.rs:238-264) places no constraint on `T`'s destructor.

Evidence:

    src/tree.rs
    531	        // The commit point: the walk returned without unwinding. Both fields
    532	        // are assigned before the pre-image drops, because that drop runs
    533	        // user code — everything the batch displaced becomes uniquely held
    534	        // here, so its cascading `T` destructors run now, and a panicking
    535	        // destructor must find the tree already consistent. The defense is
    536	        // nothing subtler than statement order: replace, assign, then drop.
    537	        let pre_image = std::mem::replace(&mut self.root.root, new_root);
    538	        self.root.ceiling = new_ceiling;
    539	        drop(pre_image);

    src/tree.rs (the mid-walk sources, as the code itself states them)
    509	        // our own bug. Unwind sources survive inside this walk: the leaf
    510	        // level drops causally-skipped action messages and batch-internal
    511	        // displaced inserts mid-walk, and on the wire-apply path those
    512	        // messages are freshly deserialized, so the drop is the last handle
    513	        // and runs `T`'s destructor.

    src/tree/traverse/join.rs
    65	    // is the unwind-source region of `Tree::join`'s commit section (deletion
    66	    // honoring and the duplicate-subtree drops run `T` destructors), and its

    src/batch.rs
    132	        inner.send_if_modified(|inner| {
    ...
    143	            inner.tree.act(party, actions)
    144	        });

    src/peer/gossip.rs
    816	        self.inner.send_if_modified(|inner| {
    ...
    869	            let tree_changed = inner.tree.join(merged);

    ~/.cargo/registry/src/*/tokio-1.52.3/src/sync/watch.rs
    1177	            let mut lock = self.shared.value.write();
    1178	
    1179	            // Update the value and catch possible panic inside func.
    1180	            let result = panic::catch_unwind(panic::AssertUnwindSafe(|| modify(&mut lock)));

    src/peer.rs
    678	    pub(crate) fn snapshot(&self) -> Snapshot<T> {
    679	        Snapshot::new(self.network, self.inner.borrow().tree.clone())
    680	    }

Resolution: Two steps, the first required regardless of the second. (1) State
under "Choosing a payload type" that `T`'s destructor may run inside the
replica's commit critical section on any `send`, `redact`, or gossip commit,
must not block, and must not touch a `Rumors`, `Snapshot`, or observer of the
same replica (re-entering the replica's lock deadlocks). Add a matching
maintainer comment at batch.rs:132 and gossip.rs:816 naming that user code
runs under the write guard. (2) Have `Tree::act` and `Tree::join` hand the
pre-image root back to the caller (return it alongside the changed flag) so
`Batch::commit` and `gossip_inner` drop it after `send_if_modified` returns;
`Tree` is private to the crate, so this is an internal signature change, and
the replace-assign-then-drop unwind argument is unchanged because the tree is
consistent before the drop either way. This removes the bulk deallocation
from the lock hold; the mid-walk drops remain inside it, which is why step (1)
stays. Evacuating those too (collect skipped and duplicate messages into a
sink the walk hands back) is a design proposal for the owner. Acceptance: the
payload-type section names the destructor constraint; `drop(pre_image)` (or
its equivalent) executes outside both `send_if_modified` closures, checked by
a test whose `T: Drop` records whether `inner.borrow()` succeeds during the
drop (it must, once the drop is outside the lock).
Construction (for the hazard as it stands): a payload type whose `Drop` reads
a thread-local `Rumors<T>` handle and calls `snapshot()`. Seed, `into_rumors`,
store the handle in the thread-local, `send(value)`, drop every `Snapshot`,
then `redact(&version)`: `Batch::commit` -> `send_if_modified` (write guard
held) -> `Tree::act` -> `drop(pre_image)` -> `T::drop` -> `snapshot()` ->
`inner.borrow()` -> `RwLock::read` on the thread holding `write`. The test
hangs or panics with std's re-entrant-lock diagnostic.

Witness: the second witness pass ran this construction as a new integration binary, tests/zz_witness_drop_reentry.rs, deleted after the run (`witness/results.md`, `## async-hazards-3`). Through the public API alone (`Peer::seed`, `into_rumors`, `send`, `snapshot`, `iter`, `redact`), a payload type whose `Drop` reads a thread-local `Rumors<Payload>` handle and calls `snapshot()` was sent, the handle was installed only once the message was in the tree, and a worker thread called `redact(&version)` while the test waited on a channel with a 10 s timeout. Decisive output:

    WITNESS async-hazards-3: Payload::drop(7) calling snapshot()
    WITNESS async-hazards-3: redact() did not return within 10s: the destructor's snapshot() blocked on the lock the commit holds
    test zz_witness_payload_drop_reenters_replica_lock ... FAILED
    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.01s

The line `WITNESS async-hazards-3: snapshot() returned inside the destructor, ...` never appeared: the destructor was entered from inside `redact`, `snapshot()` never returned, and `redact` had not returned after 10 s, so the read acquisition in `Peer::snapshot` (src/peer.rs:678-680) blocked behind the write guard `Batch::commit`'s `send_if_modified` holds around `Tree::act` and its `drop(pre_image)` (src/batch.rs:132-144, src/tree.rs:537-539). The observed form is a hang, not a panic; that std's `RwLock` does not diagnose the re-entry (tokio built without `parking_lot`) is assessed, not verified. Without the test's own timeout the hang would have lasted until nextest's 180 s terminate budget.

### async-hazards-4: The first greeting on a cold tree hashes and bounds the whole tree inside one poll, stated only in a crate-private doc
- Where: src/tree/mirror/streaming/materialized.rs:513-527 (related: src/tree/mirror/streaming/materialized.rs:569-583, src/tree/mirror/streaming/materialized.rs:501-505, src/tree/mirror/streaming/backend.rs:400-407, src/tree/mirror/streaming/backend/local.rs:136-141, src/peer.rs:710-714, src/rumors.rs:410-416, src/lib.rs:231-236, src/link/routed.rs:37)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read `greeting_fan` -> `children_of` -> `Local::children`, which is `stream::iter` over the root's children with no yield point; `fan_listing` calls `node.hash()` per root child and `Root::max_version_bytes` forces the bounds memo; `grep -rn yield_now src/` finds nothing)
- Verification: reframed: the sweep's companion claim about `CausalMessages` is dropped (see Dropped); the greeting half stands; history: deliberate-and-holds for the hiding of `warm_caches` (all four sites say "For benchmark and test calibration only"), no-rationale-found for the absence of any public statement of the greeting cost
- Owner-gated: no for the doc sentence; yes for exposing `warm_caches` (API addition)

Both `connect` and `accept` build the greeting from `fan_listing(&fan)` and
`self.root.max_version_bytes()`. On a tree whose nodes carry empty memos (a
fresh process that rebuilt the set, or a replica that just bootstrapped) the
first of these hashes every subtree and the second folds every branch's
bounds, synchronously, inside the session future's poll: the local backend's
`children` is `stream::iter`, so `greeting_fan(..).await` never yields. The
only statement of this cost is the `pub(crate)` doc on `max_version_bytes`
(backend.rs:402-407). The public contract tells the caller they own the
executor (lib.rs:233-236) and, for routed links, to spawn or select over the
router future (routed.rs:37); on a current-thread executor that router, and
every other task, stalls for the duration of the first greeting's hash of a
large set. `warm_caches`, which pre-pays exactly this, is `#[doc(hidden)]` at
`Peer`, `Rumors`, `Snapshot`, and `Tree`, scoped to benchmarks by its docs.

Evidence:

    src/tree/mirror/streaming/materialized.rs
    513	        let fan = greeting_fan(&self.backend, self.root.root.clone())
    514	            .await
    515	            .map_err(Error::Backend)?;
    ...
    521	            max_version_bytes: self.root.max_version_bytes(),
    ...
    527	            listing: fan_listing(&fan),

    src/tree/mirror/streaming/backend.rs
    402	    /// when empty. The first read materializes it: every branch's
    403	    /// bounds memo is forced tree-wide, `O(#branches)` bound
    404	    /// folds, once per tree lineage — the memos are shared through the
    405	    /// node handles across snapshots, and a mutation invalidates only
    406	    /// its own spine. A fully converged pair pays this once, at
    407	    /// greeting time.

    src/tree/mirror/streaming/backend/local.rs
    136	        let children = stream::iter(
    137	            parent
    138	                .into_children()
    139	                .into_iter()
    140	                .map(move |(radix, child)| Ok((prefix.push(radix), child))),
    141	        );

    src/peer.rs
    710	    /// Force this set's tree to compute its lazy structural memos (observable
    711	    /// hash and ceiling/floor version bounds), so a subsequent operation is
    712	    /// timed against its own work. For benchmark and test calibration only.
    713	    #[doc(hidden)]
    714	    pub fn warm_caches(&self) {

    src/lib.rs
    233	//! Sessions and observers are plain futures and streams, driven entirely by
    234	//! the caller. The I/O traits are Tokio's runtime-independent
    235	//! [`AsyncRead`](tokio::io::AsyncRead) and [`AsyncWrite`](tokio::io::AsyncWrite);
    236	//! no Tokio runtime, spawning, sockets, or timers are required by this crate.

Resolution: Add one sentence to the runtime-independence section (or to
`Rumors::gossip`): the first session on a freshly built replica computes the
tree's hashes and version bounds synchronously inside the greeting, once per
tree lineage and proportional to the set's size, so a large set's first
session occupies its executor thread for that long. Owner call, separately:
whether to make `warm_caches` (or a named equivalent) public so an
application can pre-pay off the latency path; if it stays hidden, the doc
sentence should say the cost is paid at the first session. Acceptance: a
public doc sentence names the first-greeting materialization; the
`warm_caches` decision is recorded either as a public method with user-facing
docs or as an explicit "hidden stays" ruling.

### async-hazards-5: Dropped early hand-off senders are read as empty without the argument that makes that safe
- Where: src/tree/mirror/streaming/materialized/work/levels.rs:399-399 (related: src/tree/mirror/streaming/materialized/work/levels.rs:447-448, src/tree/mirror/streaming/materialized/work/levels.rs:118-120, src/tree/mirror/streaming/materialized/work/levels.rs:234, src/tree/mirror/streaming/materialized/work.rs:146-151, src/tree/mirror/streaming/tasks.rs:46-51)
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read `initiator_level`/`responder_level`: every return before `early_tx.send(..)` is a `?`/`violation` error path; read `pump`, which publishes the item and then parks forever on an error without polling the source again)
- Verification: confirmed, with the safety argument stated more carefully than the sweep did: the sender drops when the opening generator completes or is dropped, which after an error happens only at session teardown (the pump parks without polling it again) or if the pump's consumer went away; both are sessions already failing, and the published error wins the `biased` race in `driver::race_session`; history: no-rationale-found
- Owner-gated: no

Both hand-off receivers are consumed with `early.await.unwrap_or_default()`.
A reader sees a closed `oneshot` swallowed into "no early supplies", which
makes the request fall through to the pruned-away interpretation. It is safe
because the opening stage cannot end without sending except on its error
path, and that error reaches the error route and wins the session; the
default only keeps this stage total while the session unwinds. None of that is
at the sites or on the field docs (levels.rs:285-302).

Evidence:

    src/tree/mirror/streaming/materialized/work/levels.rs
    399	                        supplied = Some(early.await.unwrap_or_default().into_iter().collect());
    ...
    447	                                survivors =
    448	                                    Some(early.await.unwrap_or_default().into_iter().collect());
    ...
    118	            // Filled before the opening yields, so the level consuming it
    119	            // never waits: its first query cannot arrive earlier.
    120	            let _ = early_tx.send(early);

    src/tree/mirror/streaming/materialized/work.rs
    146	        let failed = item.is_err();
    147	        if send.send(item).await.is_err() {
    148	            return Ok(());
    149	        }
    150	        park_after_published_error(failed).await;

Resolution: One comment at the first site (or on the `early_survivors` /
`early_supplies` field docs at levels.rs:285-302) stating: the opening stage
sends before its first yield, so a closed hand-off means it failed first; its
error is on the route and wins the session, and the empty default only keeps
this stage total until cancellation. Acceptance: the comment exists at one
site and the other site points to it.

### async-hazards-6: `Work::spawn` names a task-bag push in a crate that promises no spawning
- Where: src/tree/mirror/streaming/remote/proxy/work.rs:132-135 (related: src/tree/mirror/streaming/materialized/work.rs:93 and 108, src/lib.rs:236, callers at proxy/work.rs:155 and proxy/work/pump.rs:103, 151, 259, 351)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read `Work::spawn` and the materialized twin; `grep -rn spawn src/` shows the production hits are this method, its five callers, one comment in proxy/work/encode.rs:8, and doc examples that spawn on tokio in user code)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The proxy's `spawn` pushes a boxed future into `self.tasks` for the terminal
`FuturesUnordered`; nothing reaches an executor. The materialized side pushes
the same way without the name. In a crate whose docs promise "no Tokio
runtime, spawning, sockets, or timers", the method is the one production hit
an auditor of that promise must read past.

Evidence:

    src/tree/mirror/streaming/remote/proxy/work.rs
    132	    /// Add one independently runnable protocol task.
    133	    fn spawn(&mut self, task: impl Future<Output = Result<(), Error<B::Error>>> + Send + 'static) {
    134	        self.tasks.push(Box::pin(task));
    135	    }

    src/lib.rs
    236	//! no Tokio runtime, spawning, sockets, or timers are required by this crate.

Resolution: Rename to `enqueue` or `add_task` (the doc comment already says
"Add"), and reword proxy/work/encode.rs:8 ("the proxy state that spawns it")
to match. Acceptance: `grep -rn 'spawn' src/` returns only user-code doc
examples and the crate-docs promise.

## Positives

- Every `tokio::select!` in production source is cancellation-clean by
  construction (verified at tasks.rs:24-36, driver.rs:73-74 with `biased;`
  toward the error route, and the roster of six sites matches the sweep's).
  `tasks::complete` races pinned `&mut` futures and awaits the loser
  afterwards, so no task's progress is lost.
- The commit critical sections are panic-atomic by statement order alone
  (tree.rs:531-539, 602-611) and that property is pinned by four committed
  tests, two of which exercise the real mid-walk destructor source
  (`act_destructor_unwind_leaves_tree_byte_identical`,
  `join_destructor_unwind_leaves_tree_byte_identical`).
- `PartyGuard` (gossip.rs:1379-1404) makes the speculative fork's recovery a
  drop guard, so every error and unwind path re-joins it without a per-path
  case, and the `bookmark_donate`-then-`take` ordering (gossip.rs:769-781)
  states exactly why the guard is defused only after the slice persists.
- Runtime independence is enforced in the manifest: `tokio` carries only
  `io-util`, `macros`, `sync` (Cargo.toml:137), and `rt` enters only through
  the `test-internals` feature (Cargo.toml:112).
- The bookmark-then-`watch` lock order is stated and followed at both
  nesting sites (gossip.rs:534-553, 677-716), and the `watch` write lock is
  never held across an await.
- Sweep-reported, not re-verified here: the observers' owned-wait
  `Channel::Waiting` design, the `Extant` token protocol behind
  `try_into_peer`, `OnceLock` memos on shared nodes, the three `# Cancel
  safety` sections on the frame futures, and the negotiated bounds on
  wire-fed buffers.

## Open questions for Finch

1. Finding 3 (destructors under the write lock): the destructor constraint
   must be documented in any case, since `traverse::act` and `traverse::join`
   drop `T` mid-walk. Do you also want (a) the cheap deferral of the pre-image
   drop past `send_if_modified` (internal signature change to `Tree::act` and
   `Tree::join`, removes the bulk deallocation from the lock hold), and (b)
   the fuller evacuation that collects the mid-walk drops into a sink the walk
   hands back? Recommendation: document now and do (a); treat (b) as a design
   proposal.
2. Finding 2 (retire carve-out): is the accurate sentence enough, or do you
   want the cancel-at-each-await retire test alongside it? Recommendation:
   both; the bounded-poll harness in `tests/lifecycle.rs` already does the
   hard part.
3. Finding 4: should `warm_caches` (or a named equivalent) become public API so
   applications can pre-pay the first-greeting materialization? API addition,
   your call; the doc sentence is warranted either way.

## Dropped

- Sweep [3], the `CausalMessages` half: causal.rs:28-32 already states that
  the time to retrieve each message "may burst arbitrarily large, up to the
  total size of the messages stored", which is the per-poll statement at the
  user's altitude; only the greeting half survives as async-hazards-4.
