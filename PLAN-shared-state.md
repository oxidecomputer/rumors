# Plan: single-lock shared state for `Known`

Status: draft for review. Not yet implemented.

## 1. Problem and the one law

Two confirmed bugs (retire-vs-snapshot region duplication, fixed in `89e4478`;
stale-snapshot bootstrap floor, reproducer in `tests/stale_floor.rs`) and one
reasoned sibling (snapshot absorbs a retiree; originator gains region without
floor) are all instances of a single defect: **the party and the tree are
sampled at different times.** The party rides a shared `Arc<RwLock<Party>>`
(live); the version floor and content ride a tree cloned at `rumors()` time
(frozen).

The law that kills the whole family:

> Every transfer of id-space pairs, in one critical section, the region with
> the exact tree state the recipient causally inherits. The party never
> leaves the lock except by subtraction (fork, retire's take) or returns
> except by disjoint addition (absorb, reclaim); ticks read the party inside
> the same critical section that installs the minted version; the lock is
> never held across wire I/O.

## 2. Verified code facts the design rests on

- **Bootstrapper's floor = provider's greeted ceiling.** The reconciled root's
  ceiling is `our_version | their_version` (`mirror/local.rs:591,616,1054`),
  where `our_version` is captured from the root at `Exchange::start`
  (`mirror/local.rs:204`). Nothing folds served *leaf* versions into the
  ceiling on this path. Consequence: the served tree, the greeted version,
  and the forked region must all come from one lock acquisition. Serving
  content fresher than the greeting would put leaves above the newcomer's
  ceiling — it could then re-mint their heights. **Fork-at-clone is the only
  sound timing** (see §4).
- **Tree mutations suspend only at user callbacks.** `Tree::act`, `react`,
  and `join` (`src/tree.rs`) are async solely to await the per-leaf observer
  futures; the traversals themselves never yield (stack growth via `stacker`
  is synchronous). With `Ready`-returning collectors, the futures complete in
  one poll — drivable via `now_or_never` inside a lock guard with no real
  suspension. Collector output reaches user callbacks only after the guard
  drops (`step`'s mint phase); `step`'s drain phase streams lock-free
  against the owned observed tree (Appendix A).
- **`Tree::join` is a lattice merge with deletion-honoring** (version-bound
  filtering), already the in-process equivalent of the wire protocol. It is
  the correct write-back operation under any interleaving.
- **The wire format does not change.** Fork timing, locking, and write-back
  discipline are all peer-local. No `PROTOCOL_VERSION` bump; the
  `gossip_snapshot.rs` / insta pins must remain byte-identical (treat any
  diff as a bug in the port).
- **`before` does not change.** `Party::{fork, join, seed}` suffice; the last
  live `dangerously_alias` call in rumors is already gone (`89e4478`); the
  one in dead-code `bookmark.rs:159` is dealt with in Phase 5.

## 3. Target design

```rust
/// The shared pool: the replica every handle gossips against. The only
/// place a Party lives. All methods are synchronous and do no locking
/// themselves; the caller holds the guard.
struct State<T> {
    party: Party,
    tree: Tree<T>,    // the pool: everything replicated, observed or not
}

pub struct Known<T, S = Facts<T>> {
    network: Network,                 // immutable, Copy
    pool: Arc<RwLock<State<T>>>,
    tier: S,
}

/// Canonical tier (payload-carrying type-state): privately owns the
/// *observed* tree — the unique, race-free exactly-once cursor.
/// `Known<T, Facts<T>>` is !Clone.
pub struct Facts<T> { observed: Tree<T> }

/// Gossip tier: no private state. `Known<T, Rumors>` is Clone; dropping
/// one loses nothing.
pub struct Rumors;
```

**Two tiers, one law.** The pool is the single-lock design, unchanged:
party and tree in one lock, the Lease, fork-at-clone, atomic absorb —
every soundness argument in §§2–5 applies to it verbatim. Above it, the
Facts privately owns a second, COW-structure-shared tree: the observed
set. Gossip (from either tier) checks into the pool, always silently.
Content crosses pool → observed in exactly one place — **`step` on the
`&mut` Facts, the application's reducer turn**: it first drains all
pending pool state into the observed tree (firing the callback per leaf
new to the Facts, applying remote redactions), then mints the supplied
messages into both trees (firing the callback per mint). One callback
site in the entire API. That single crossing is what makes **exactly-once
observation** hold: the dedup cursor is an exclusively-owned tree behind
`&mut`, not a shared structure behind a lock, so no race can
double-observe and no buffer is needed to dedup. The type-state is
restored *not* for clock safety (the lock is still the linearizer for the
pool) but because exactly-once requires a unique observer.

Unification dividends: the mint-without-processing and
consume-without-processing footguns of a `message`/`observe` pair are
unexpressible (every mutation point is the processing point; the callback
is mandatory); a turn's mints tick above the pool ceiling it just
drained, so outputs causally dominate inputs; and `bootstrap` collapses
too — the received tree lands only in the pool, `observed` starts empty,
and the first `step` replays history through the same reducer that
handles live traffic (`bootstrap_then` is removed). When nothing is
pending, the drain is O(1) via the dominance fast path (§5), so
`step(msgs, cb)` costs what `message_then` did.

Three properties fall out of the tree-diff observation for free:
self-echo dedup (a mint returning via gossip is already observed);
robustness to failed sessions (a dead session checked nothing into the
pool, so nothing is observed — the at-least-once wart of the streaming
design disappears); and redaction-before-observation never fires (a
message redacted in the pool before any `step` never existed, from the
application's view).

Memory: the observed tree shares structure with the pool (COW);
divergence is exactly the not-yet-observed gossip, bounded by `step`
cadence — the application's queue, not a hidden buffer.

Never stepping is protocol-harmless (every wire obligation lives in the
pool; a pure relay node is a legitimate mode) with two local, documented
consequences: (a) the frozen observed tree pins at most its own nodes as
pool churn erodes COW sharing — bounded by its size at the last `step`,
not by traffic; (b) **remote redactions take local effect only at
`step`** (the drain's join honors deletion via the pool ceiling), so a
never-stepping Facts retains redacted content in its own view
indefinitely. Local `redact` is immediate (applied to both trees).
Document (b) loudly; the mitigation is a periodic empty `step([], cb)`.

### The `Lease`: the misuse-proof core

All session-facing state checkout goes through one linear type, defined next
to `State` (one module owns every party mutation in the crate):

```rust
/// A consistent (content, floor, region) triple checked out of the shared
/// state in one critical section. Linear: the lease ends in exactly one of
/// check-in or Drop, either of which rejoins an unsurrendered fork; the
/// fork exits only via `surrender_fork` (to the wire) or that reclaim.
/// Not Clone; no accessor exposes the fork.
struct Lease<T> {
    tree: Root<T>,          // COW clone @ checkout
    fork: Option<Party>,    // subtracted from the shared party @ checkout
    shared: Arc<RwLock<State<T>>>,  // for check-in / Drop reclaim
}
```

- `State::lease()` (write lock): clone the tree, `party.fork()` — the only
  producer of a wire-eligible region for serving.
- `Lease::surrender_fork() -> Outbound`: moves the fork out for
  `remote::send_party`; `Outbound` is a private newtype, the only type
  `send_party` accepts, constructible only here and by retire's take. After
  surrender, Drop has nothing to reclaim — the two-generals commitment
  boundary (frame send) is preserved exactly as today.
- `Lease` check-in (write lock, one critical section): `tree.join(session
  result)` *and* `party.join(unused fork)` *and* `party.join(absorbed
  retiree region)` as applicable — region and floor land together (kills
  bug B).
- `Lease::drop`: reclaims an un-surrendered fork under the lock. This makes
  cancelled gossip futures safe for free: cancellation before surrender
  reclaims; cancellation during the party-frame send is past surrender and
  commits (leak, never duplicate), identical to retire's model.

Why this is hard to misuse: `Party` never appears in the public API (already
true — keep it that way); inside the crate, the only constructors of a
sendable region are `lease()`/retire's take, both of which *subtract under
the lock atomically with the tree state they pair with*; `Lease` is `!Clone`
with private fields, so a region cannot be duplicated, double-sent, or
separated from its floor without writing new code in the one module that owns
the laws. Overlap is then impossible by construction, not by review.

### Operation map

| Operation | Lock discipline |
|---|---|
| `step` drain phase (`&mut` Facts) | read lock pool: COW-clone the tree; unlock; stream the diff into the owned observed tree (`Tree::join` with `on_recv`), installing per leaf as each callback returns — no lock held, awaits free (Appendix A) |
| `step` mint phase (`&mut` Facts) | chunked: per ≤K messages — write lock pool: `tick(&state.party)` + `act` (driven sync, collect ≤K); unlock; then per leaf: await its callback, `react` it into the owned observed tree (no lock); repeat (Appendix A) |
| `redact` (`&mut` Facts) | single silent chunk applied to both trees (write lock for the pool; owned observed needs none) |
| wire `gossip` (either tier, silent) | `lease()` at start (clone + speculative fork); session runs lock-free on the lease; check-in at end (join tree, reclaim or surrender fork, absorb retiree region) — always the elided walk |
| serve a bootstrapper | the leased fork is surrendered as the trailing frame; the served tree/greeting are the lease's — consistent by construction |
| absorb a retiree | received region joins the shared party *in the same critical section* as the session tree joins back |
| `bootstrap` (client) | no state yet; wire protocol unchanged; received tree becomes the pool, observed starts empty (the first `step` replays it) |
| `retire` (Facts) | `Outstanding` check (`Arc::strong_count == 1`) stays; then `Arc::into_inner` gives the pool `State` by value — the party take becomes a true move, no `Option`, no alias; ships the pool (a superset of observed); on `Recovered`, rebuild the `Arc` (replaces today's root-backup dance) |
| `join` (cross-set local merge, silent) | injects the other's pool clone into *our pool*, unobserved (observation still happens only at `step`); two locks never held together |
| Facts reads (`iter`/`len`/observed `latest`) | owned observed tree: plain borrows, no lock |
| pool reads (`pool() -> View`, `Eq`) | read lock; clone out what's needed |

### `step` mechanics

The mint phase is a chunk loop (bounded memory, backpressure into the
caller's iterator — Appendix A): pull ≤K messages; write lock →
split-borrow `State { party, tree }` → `tree.act(|b| b.tick(party), chunk,
collector)` driven via `now_or_never().expect("collector callbacks never
suspend")` → unlock → then, per collected `(Key, Version, Message)` leaf:
await the user's `on_message` future, `react` that leaf into the owned
observed tree (versioned insert, no lock) → next chunk. Versions tick from
the *pool* ceiling (the replica's causal frontier), never the observed
ceiling. `redact` (no callback) is a single silent chunk applied to both
trees; session check-in is a single silent `tree.join`. Add a small
`drive_sync` helper documenting the never-suspends contract; a debug
assertion makes a violation loud.

Cancellation: both phases install each leaf into the observed tree only
*after* its callback returns (callback-then-install — install-then-callback
would risk observed-but-never-fired messages, silent under-delivery).
Dropping a `step` future mid-turn therefore re-fires at most the single
in-flight message on the next `step` (at-most-twice under cancellation;
exactly-once otherwise) — and a re-fired *mint* arrives as
`Source::Gossip`, since it is drained back out of the pool. Document both;
do not `mem::take` the observed root across user awaits.

## 4. Why fork-at-clone, not fork-at-handshake or fork-last

The newcomer's anti-collision floor is the provider's greeted ceiling (§2).
Let T0 = lease checkout (clone + greeting version), Tf = fork time. Any mint
by any clone in (T0, Tf] may land in the to-be-forked interval at
a height above the greeted ceiling; the newcomer, whose floor is the T0
ceiling, can then re-mint that height → duplicated version for distinct
content → silent loss via deletion-honoring. Sound iff **Tf = T0**.

- Fork-last (today): window = the whole session → `tests/stale_floor.rs`.
- Fork at handshake (when we learn the peer bootstraps): window = clone→
  handshake. Smaller, still unsound.
- Re-snapshot + fork at handshake: serves leaves above the greeted ceiling —
  unsound the other way around (the newcomer holds leaves above its floor and
  can re-mint *their* heights).
- **Fork-at-clone (chosen):** every wire-gossip lease forks speculatively;
  unused forks are reclaimed at check-in or Drop. Region and floor coincide;
  mints after T0 happen in the post-subtraction remainder and cannot touch
  the newcomer's interval. Cost: one ITC fork + join per ordinary gossip
  session (cheap bit-tree ops; fork→join round-trips to canonical form — the
  reconstitution tests already pin that normalization).

## 5. Write-back is a lattice join; dominance is a fast path

"Write back iff the lock's version doesn't dominate ours" with *replacement*
loses data under concurrency: S1 merges X; S2 (holding Y, not X) sees an
incomparable current version, replaces, and X is gone. Check-in therefore
**always joins**. The dominance check survives as an optimization only: if
the shared version dominates the session's, the join is provably a no-op
(deletion-honoring makes dominated-and-absent mean redacted) and can be
skipped without taking the merge walk.

## 6. Public API decisions (recommendations)

Breaking changes are acceptable (pre-1.0, own crate). Each phase lands the
matching `sync::Known` parity and doc updates.

1. **`gossip` takes `&self`, returns `Result<(), Error>`, and is silent:
   `gossip_then` is removed.** The consume-and-return dance existed to
   freeze the tree during a session; nothing needs freezing now, and the
   Facts may mint while sessions run. Sessions carry no user callbacks, so
   they always take the elided discovery walk (cheaper), run at wire speed
   into the pool, and impose no observation semantics. Available on both
   tiers.
2. **The Facts/Rumors type-state is restored — for exactly-once, not for
   clock safety.** The lock remains the linearizer for the pool; what the
   lock cannot provide is a race-free observation cursor (a consistent
   global cursor behind the lock forces buffering or stalls — the rejected
   designs). The `!Clone` `Facts<T>` carries the owned observed tree as
   type-state payload; `Rumors` is a unit, and `Known<T, Rumors>` is
   `Clone` (gossip workers, no private state). `rumors()` returns as the
   handle constructor. Origination returns to `&mut self`, Facts-only.
   `strong_count` on the pool `Arc` still backs `Outstanding`.
3. **One mutation-observation entry point: `step(messages, on_message)`**
   — the reducer turn, replacing `message`/`message_then`/`observe`/
   `observe_then` (and `bootstrap_then`; §3). Drain phase first (fire per
   leaf new to the Facts, apply remote redactions), then mint phase (fire
   per minted message, surfacing its `Key` for later redaction). Catch up
   without minting via `step([], cb)`. The callback is mandatory: minting
   without processing and consuming without processing are unexpressible.
   Naming: `step` (the reducer-loop convention); on-theme alternatives
   `converse`/`turn` considered. The callback carries a provenance
   discriminant (decided) — `FnMut(Key, &Version, &Arc<T>, Source)` with
   `Source::{Local, Gossip}` — since the one callback now sees both own
   mints and drained gossip, and apps (optimistic rendering, own-key
   bookkeeping) need the distinction. Callback-free
   `join(&self, other)` survives as the demoted cross-set convenience —
   it injects into the *pool*, unobserved, preserving the single crossing
   (its content surfaces at the next `step`).
4. **Exactly-once is robust, not just true under concurrency** (Appendix
   A): observation is decoupled from sessions, so failed sessions observe
   nothing (no at-least-once wart), redaction-before-observation never
   fires, and a `&mut` receiver makes double-observation unconstructible —
   a callback cannot even reentrantly `step` (the borrow forbids it).
   Mint chunks bound memory with iterator backpressure; the drain streams
   leaf-by-leaf (Appendix A). Sole qualifier: cancelling a `step` mid-turn
   may re-fire its one in-flight message (§3 mechanics).
5. **Read surface splits by tier.** Facts reads come off the owned
   observed tree with plain borrows — `iter()`/`latest()` survive with
   today's signatures, no lock, no indirection. Pool reads on either tier
   go through `pool() -> View<T>`: a frozen COW clone taken under a brief
   read lock, carrying `iter/len/hash/latest/earliest`. Two frontiers are
   deliberately visible (observed vs. pool); their naming is settled by
   the split itself — `facts.latest()` vs. `facts.pool().latest()`.
6. **`Eq`:** compare pools (replica equality, network-guarded), via
   cloned-out views — never two locks at once. Observed-tree equality is
   visible through Facts reads. Clones of one pool are trivially equal —
   document.
7. **Poisoning:** keep `std::sync::RwLock` + `.unwrap()` (house style): a
   panic inside a critical section is a crate bug; poisoning propagates it
   rather than gossiping from a half-mutated state.

## 7. What deliberately does not change

- Wire format, `PROTOCOL_VERSION`, all insta snapshot pins.
- `before`'s public API (frozen per its CLAUDE.md).
- The network/universe check at every combining operation.
- Retire's `Outstanding` refusal — its justification shifts from soundness
  (the lock now linearizes transfers) to content completeness (don't ship a
  tree that in-flight sessions are about to grow), and it makes retire's
  `Arc::into_inner` a true move.
- The two-generals commitment model and `RetireError` variants.
- Linearity of parties as the root invariant; redaction via version bounds.

## 8. Phases (dependency-ordered; each ends `just gate`-clean and committed)

**Phase 0 — pins and groundwork.** (a) Test pinning "bootstrapper ceiling ==
provider's greeted version" — documents the floor source §2 rests on. (b)
`drive_sync` helper + test that `act`/`join` complete in one poll with
`Ready` callbacks. (c) Assert wire snapshots green as the baseline.

**Phase 1 — consolidate: one lock, `State`, `Lease` (the big commit).**
Move the tree into the lock beside the party; introduce `State`/`Lease`/
`Outbound` in one module with the law in its module docs; sessions become
clone-out → run → check-in; mints run under the lock. Fork stays *late* in
this phase (semantics-preserving for the protocol) — this commit is the
structural move only; the type-state keeps today's shape (unit markers)
until Phase 4 gives `Facts` its payload. Public fallout already unavoidable
here: snapshots go live as pool-sharing handles (doctests and tests that
stage "snapshot, mint, gossip to teach the snapshot" — the lib.rs gossip
examples, `tests/handshake.rs`, `pairwise` snapshot tests — need restaging
on bootstrap forks), and `iter()` can no longer borrow through the lock —
introduce `View`/`pool()` here as the interim read surface (`Facts::iter()`
returns in Phase 4 over the observed tree). Largest, riskiest phase; review
with the adversarial-rounds protocol.

**Phase 2 — fork-at-clone.** Move the fork into `lease()`; reclaim at
check-in/Drop; surrender for bootstrappers. **Un-ignore
`tests/stale_floor.rs` — it must pass.** Add the cancellation test (drop a
serving gossip future mid-session; party reconstitutes).

**Phase 3 — atomic absorb.** Retiree's region joins the shared party in the
same critical section as the session tree joins back. Regression test for
bug B's transitive scenario (4 peers: C mints → D learns C → a clone of F
absorbs C's retirement → F mints → F↔D full sync → nothing lost). Plus the
private-party test: absorb-then-retire reconstitutes the seed region.

**Phase 4 — API surface: the two-tier observation model.** §6 items:
`Facts<T>` gains its observed-tree payload; `step` replaces `message`/
`message_then`/`observe`/`observe_then`/`bootstrap_then` (drain phase then
chunked mint phase, `Source` discriminant); `gossip_then` is removed
(sessions silent, always the elided walk); `gossip(&self)`; demoted
cross-set `join`; `Facts::iter()` returns over the owned observed tree;
`Eq` on pools. Tests: exactly-once (mint → gossip out and back → a later
`step` does not re-fire the self-echo); failed-session isolation (severed
gossip → `step` drains nothing); mint-chunking backpressure (the caller's
iterator is pulled lazily); redaction-before-`step` never fires; cancelled
`step` re-fires at most its in-flight message; bootstrap replay arrives
through the first `step` with `Source::Gossip`. Retire internals simplify
(`Arc::into_inner(state)`, rebuild-on-`Recovered`; the root-backup dance
and the severed-wire tests' expectations get re-derived — outcomes must
stay identical). Full doc overhaul: crate docs, `Known` docs, mirror
module docs' party-handoff sections, CLAUDE.md orientation note.

**Phase 5 — sweep.** `sync::Known` parity audit (each phase should have
kept it green; this is the systematic file-by-file pass). `bookmark.rs`:
rework its aliasing to read under the lock or delete the module (recommend
delete; it's dead and predates the design). Grep gate: `dangerously_alias`
count in rumors must be zero.

**Phase 6 — property tests.** New proptest file driving one universe
through arbitrary interleaved operation sequences (`step`s with arbitrary
message batches, wire gossip between sets on either tier, bootstrap
serves, retires), asserting after every operation and at quiescence:
(i) live parties pairwise disjoint (in-crate test); (ii) every minted,
never-redacted message present everywhere after full pairwise sync (the
anti-stale-floor invariant); (iii) retiring everything reconstitutes
`Party::seed`; (iv) **exactly-once**: every message fires each Facts'
callback at most once, exactly once if never redacted (with
`Source::Local` at its minting Facts, `Source::Gossip` elsewhere), and
never if redacted before that Facts stepped. Because every critical
section is atomic, logically interleaved sequences cover the concurrency
state space — no true parallelism needed in the generator. Commit all
`proptest-regressions`.

## 9. Risks and open questions

- **Phase 1 blast radius**: the doctest/test restaging is wide; budget for
  it (every example built on "snapshot then teach it over the wire").
- **`now_or_never` contract**: if any traversal ever gains a real suspension
  point, mint-under-lock breaks loudly (expect). The Phase 0 test pins it.
- **Lock hold time**: blake3 hashing and merge walks run under the write
  lock. Acceptable for a local mutator (only same-universe ops contend);
  benches in `just all` will say if not.
- **`Lease::drop` takes a lock in Drop**: fine (sync lock, never held across
  await by Law); document that a `Lease` must not be dropped while its own
  `State` lock is held — structurally guaranteed since sessions don't hold
  guards.
- **Greeted-version vs. served-tree consistency** is now load-bearing in two
  places (floor §4, snapshot pins §7); the Phase 0 pin test is the tripwire.
- **`large_futures` lint**: session futures shrink if anything (lease holds
  the root by value as today); keep the boxed entry points.
- Resolved during review: `join`'s mismatch error is the unit
  `NetworkMismatch` (§6.3, Appendix B); `pool()` views expose no frozen
  `gossip` (frozen gossip is exactly the bug class being deleted).
- Resolved: the single mutation-observation entry point is
  `step(messages, on_message)` (drain then mint; the lineage ran
  `join_then` → `observe`/`observe_then` → `step` as the argument and then
  the mint/observe split fell away). The callback carries
  `Source::{Local, Gossip}` — decided.

## Appendix A (logically §3.5): observation — exactly-once at the Facts

Observation is decoupled from gossip entirely: sessions are silent and
check into the pool with `None` callbacks (every session takes the elided
discovery walk). Exactly **one** callback site remains —
`step(messages, on_message)` on the `&mut` Facts, the reducer turn — with
two phases, both streaming with zero buffering. The callback is
`FnMut(Key, &Version, &Arc<T>, Source) -> Fut` with
`Source::{Local, Gossip}` (decided): one function sees both own mints and
drained gossip, and applications need the distinction.

- **Drain phase** (`Source::Gossip`) — everything learned over the wire
  (or via cross-set `join`): brief read lock to COW-clone the pool tree;
  release; `Tree::join` the clone into the owned observed tree with
  `on_recv` streaming per leaf new to the Facts, installing per leaf as
  each callback returns. No lock held during user awaits; backpressure
  lands on the application's own consumption; the frontier captured is
  the clone's, so leaves arriving mid-walk are the next turn's news.
  Empty pending state short-circuits via the dominance fast path (§5).
- **Mint phase** (`Source::Local`) — chunked: pull ≤K messages; write
  lock pool: tick + insert, collect ≤K; unlock; then per leaf, await its
  callback and `react` it into the observed tree (callback-then-install,
  as in the drain); repeat. O(K) memory; the caller's iterator is not
  pulled until the prior chunk's callbacks complete. K=1 is exact
  streaming; a modest default keeps `act`'s single-traversal win;
  per-chunk commit is blessed by `act`'s "morally associative" contract.
  Each mint ticks above the pool ceiling the drain just consumed:
  outputs causally dominate inputs.

`bootstrap` lands the received tree in the pool only; `observed` starts
empty, so the first `step` replays history through the same reducer that
handles live traffic (`bootstrap_then` is removed).

Why exactly-once holds, and robustly: the dedup cursor is the observed
tree — exclusively owned, behind `&mut`, so double-observation is
unconstructible rather than merely synchronized away (a callback cannot
even reentrantly `step`; the borrow forbids it). Self-echoes (a local
mint returning via gossip) are already in the observed tree and the diff
skips them. A failed session checked nothing into the pool, so `step`
drains nothing — no at-least-once wart. A message redacted in the pool
before any `step` never fires; one redacted after observation is silently
retracted from the observed tree at the next `step` (a future `on_redact`
hook could surface this; out of scope). Sole qualifier: a `step` future
cancelled mid-turn re-fires at most its one in-flight message — a
re-fired mint arriving as `Source::Gossip` (§3 mechanics).

Rejected designs, for the record: buffer-at-check-in (worst case doubles
memory, kills backpressure for a future streaming mirror exchange);
streaming `gossip_then` on the lease (constant memory and wire
backpressure, but only per-session at-least-once semantics — no global
dedup point exists outside a lock). The two-tier design is the synthesis:
constant memory *and* exactly-once, paid for with one COW tree and an
explicit observation call.

Bonus defect retired: today `gossip(self)` drops the `Known` on any wire
error (`lib.rs` `gossip_inner`, `?` paths) — for a set with no handles,
a TCP reset leaks its party region. `gossip(&self)` makes wire failure
non-destructive structurally.

## Appendix B: target async API surface

```rust
/// A local set of rumors: a shared gossip pool plus, on the canonical
/// tier, the privately-owned observed tree.
pub struct Known<T, S = Facts<T>> { /* network, pool: Arc<RwLock<State<T>>>, tier: S */ }

/// Canonical tier (type-state with payload): the observed tree, the
/// exactly-once cursor. `Known<T, Facts<T>>` is !Clone.
pub struct Facts<T> { /* observed: Tree<T> */ }
/// Gossip tier: no private state. `Known<T, Rumors>` is Clone.
pub struct Rumors;

impl<T> Clone for Known<T, Rumors>;  // another gossip worker; loses nothing

// ── canonical tier ───────────────────────────────────────────────────────

impl<T> Known<T, Facts<T>> {
    pub fn seed() -> Self;
    pub fn seed_rng<R: RngCore + ?Sized>(rng: &mut R) -> Self;

    /// Acquire a rumor set from an established peer. The received tree
    /// lands in the pool; `observed` starts empty, so the first `step`
    /// replays history through the same reducer that handles live
    /// traffic. (No `bootstrap_then`.)
    pub async fn bootstrap<R, W>(
        read: &mut R,
        write: &mut W,
    ) -> Result<Option<Self>, Error>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync,
        R: AsyncRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send;

    /// Mint a gossip handle sharing this set's pool. Handles gossip; they
    /// cannot originate, observe, or retire.
    pub fn rumors(&self) -> Known<T, Rumors>;

    /// The reducer turn — THE one callback site in the API. Phase 1
    /// drains pending pool state into the observed tree (fires per leaf
    /// new to this Facts with `Source::Gossip`; applies remote
    /// redactions); phase 2 mints `messages` (fires per mint with
    /// `Source::Local`, surfacing its `Key`). Exactly-once per Facts;
    /// self-echoes deduped by the diff. `step([], cb)` catches up without
    /// minting. Streams both phases: zero buffering, backpressure on the
    /// caller (drain: its own consumption; mint: lazy iterator pull).
    pub async fn step<'a, I, OnMessage, OnMessageFut>(
        &'a mut self,
        messages: I,
        on_message: OnMessage,
    ) where
        T: BorshSerialize + Send + Sync + 'a,
        I: IntoIterator<Item = T> + Send, I::IntoIter: Send,
        OnMessage: FnMut(Key, &Version, &Arc<T>, Source) -> OnMessageFut + Send + 'a,
        OnMessageFut: Future<Output = ()> + Send + 'a;

    pub fn redact<I: IntoIterator<Item = Key>>(&mut self, redacted: I)
    where T: Send + Sync;

    /// Cross-set convenience: inject a *different* same-universe set's
    /// pool into ours, unobserved (its content surfaces at the next step).
    pub fn join<S2>(&self, other: &Known<T, S2>) -> Result<(), NetworkMismatch>
    where T: Send + Sync;

    /// Consumes the Facts; `Outstanding` refuses while any rumors() handle
    /// exists. Ships the pool (a superset of observed).
    pub async fn retire<R, W>(
        self,
        read: &mut R,
        write: &mut W,
    ) -> Result<Option<Self>, RetireError<Self>>
    where /* as bootstrap */;

    // Observed reads: plain borrows off the owned tree — no lock, no View.
    pub fn iter(&self)
        -> impl ExactSizeIterator<Item = (Key, &Version, &Arc<T>)>
               + DoubleEndedIterator + Send + Sync
    where T: Send + Sync;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn hash(&self) -> [u8; 32];
    pub fn latest(&self) -> &Version;            // observed frontier
    pub fn earliest(&self) -> Option<&Version>;
}

// ── both tiers ───────────────────────────────────────────────────────────

impl<T, S> Known<T, S> {
    /// Synchronize with a remote peer, silently: learnings land in the
    /// pool, surfacing at the Facts' next `step`. Does not consume self;
    /// cancel-safe (dropping the future reclaims the lease). There is no
    /// gossip_then.
    pub async fn gossip<R, W>(&self, read: &mut R, write: &mut W) -> Result<(), Error>
    where /* as bootstrap */;

    pub fn network(&self) -> Network;

    /// Frozen snapshot of the *pool* (COW clone under a brief read lock):
    /// replica-level stats and iteration, observed or not.
    pub fn pool(&self) -> View<T>;
}

/// Two rumor sets are equal when they share a universe and their *pools*
/// hold the same observations right now (compared via cloned-out views;
/// never two locks at once). A handle is trivially equal to its Facts.
impl<T, S, U> PartialEq<Known<T, U>> for Known<T, S>;
impl<T: Debug, S> Debug for Known<T, S>;

// ── frozen read view ─────────────────────────────────────────────────────

pub struct View<T> { /* network + frozen tree (COW clone) */ }

impl<T> View<T> {
    /// Borrowing is back: a View is owned and immutable.
    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (Key, &Version, &Arc<T>)>
           + DoubleEndedIterator + Send + Sync
    where T: Send + Sync;

    pub fn network(&self) -> Network;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn hash(&self) -> [u8; 32];
    pub fn latest(&self) -> &Version;            // borrows: frozen
    pub fn earliest(&self) -> Option<&Version>;
}

impl<T> Clone for View<T>;
impl<T: Debug> Debug for View<T>;
impl<T> PartialEq for View<T>; impl<T> Eq for View<T>;

// ── callback provenance and errors ───────────────────────────────────────

/// Which phase of a `step` delivered the message: minted by this Facts
/// (`Local`, the iterator you passed) or drained from the pool (`Gossip` —
/// wire sessions, cross-set `join`s, bootstrap replay).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source { Local, Gossip }

pub use mirror::remote::Error;                  // unchanged
pub enum RetireError<K> { Recovered { error, known: K }, Uncertain { error },
                          Outstanding { known: K } }
// shape unchanged; docs reword "snapshots" → "handles" in Phase 4

/// `join` consumes nothing now, so a mismatch hands nothing back.
#[derive(Debug, thiserror::Error)]
#[error("peers descend from different seeds and share no causal history")]
pub struct NetworkMismatch;
```

Gone from the surface, deliberately:
- `message`, `message_then`, `observe`, `observe_then`, `bootstrap_then`,
  `gossip_then` — all collapsed into `step` (plus silent `bootstrap`):
  one callback site, one observation semantics, no way to mint without
  processing or consume without processing.
- `gossip(self) -> Result<Self, _>` — the consume-and-return dance existed
  to freeze the tree during a session; nothing freezes anymore (and a wire
  error no longer destroys the `Known`).
- `join(&mut self, other: Known<T, Rumors>) -> Result<(), Known<T, Rumors>>`
  — the consuming/hand-back shape; `join` consumes nothing and feeds the
  pool, not the observed tree.
- Observation and reads on `Rumors` handles — they are gossip workers;
  only `network()`, `gossip()`, and `pool()` remain on that tier.
- The frozen-snapshot meaning of `rumors()` — frozen reads are `pool()`
  `View`s; for gossip there is deliberately no frozen variant.
