# rumors — project notes

A reference for working in this repo. Keep it accurate: correct it when you
learn the reality differs from what's written here.

## What this project is

`rumors` is a Rust library for **unordered gossip with redaction**: a
CRDT-backed set of messages that any number of peers replicate and keep
convergent. Each peer holds a local `Known<T>` rumor set; peers reconcile by
exchanging only the parts that differ, so two peers that already agree transfer
almost nothing. A message can be *redacted*, and redactions spread
contagiously to every peer the redactor (transitively) gossips with, so one
peer's local decision evicts a message network-wide without consensus.

It offers two equivalent surfaces over the same engine:

- `rumors::Known` — asynchronous (`AsyncRead`/`AsyncWrite` I/O, `async`
  callbacks). The primary surface.
- `rumors::sync::Known` — a synchronous wrapper (`std::io::Read`/`Write`,
  `FnMut` callbacks) that bridges to the async core via `pollster`.

Message values are arbitrary `T: BorshSerialize + BorshDeserialize`; `borsh` is
re-exported so callers derive the traits without a separate dependency.

## Workspace layout

- root crate `rumors` — the gossip set and its wire protocol (`src/`).
- `crates/before` — the **Interval Tree Clock** (ITC) library this is built on:
  `Party` (a forkable/joinable clock identity) and `Version` (a causal
  timestamp). See `crates/before/reference/itc2008.*` for the underlying paper.
- `crates/before-viz` — a visualization of the `before` clocks (wasm/web).
- `tests/` — integration tests: proptest-based wire-equivalence checks
  (`async_wire`, `sync_wire`, `pairwise`, …) plus the `gossip_snapshot` golden
  wire captures.

## Core data model

A `Known<T>` is a `Network` (a 128-bit universe id), a `Party` (its ITC clock
identity), and a `Tree<T>` (its content). It is deliberately **`!Clone`**:
duplicating a live clock would break the linearity ITC requires.

- **`seed()`** mints the single distinguished root party of a *universe* of
  cooperating peers, and a fresh random `Network` id (from `OsRng`) for it.
  Call it once per universe. **`seed_rng(rng)`** is the same but draws the
  `Network` from a caller-supplied RNG (e.g. a deterministic one in tests).
- **`fork()`** is the only way to get another working copy: a *true causal
  fork* that mints a fresh **disjoint** party, inherits the parent's `Network`,
  and shares the current tree copy-on-write. Both halves act independently.
- **`join()` / `join_then()`** reunite a fork: merge the trees and rejoin the
  two parties' clock regions. Disjointness is what keeps every party's history
  causally well-defined no matter how forks and joins interleave.
- **`Network`** guards against a failure disjointness can't catch: two
  *independently-`seed`ed* universes can end up with *coincidentally* disjoint
  parties. Every combining operation — local `join`/`join_then` and remote
  `gossip` — first checks the two networks match, so a shared network is the
  positive proof of common ancestry. A mismatch fails the combination
  (`Err(other)` locally, `Error::NetworkMismatch` over the wire).
- **The one rule for callers:** never let two independently-`seed`ed universes
  gossip — they share no causal history (the `before` crate's Law of
  Disjointness). The `Network` check now enforces this rather than relying on
  the caller. Within a universe, fork/join freely.

`Version` is an ITC event tree: a causal timestamp partially ordered by `<=`
(causal containment), joined by `|` (least upper bound), and advanced by
*ticking* a `Party`. `Key` is an opaque 32-byte message identifier surfaced by
the insert/merge/gossip callbacks; it is stable across peers (a key from one
peer redacts the same message on any other) and is used to `redact`.

## The tree

`Tree<T>` is a **sparse Merkle radix trie** with branching factor 256 and depth
32, with transparent path compression. A leaf's position *is* the hash of its
contents:

```
leaf path = blake3( blake3(version) ‖ blake3(value) )    // 32 bytes
```

Folding the version in means two content-identical messages inserted at
different versions land at distinct paths and get distinct keys. Every node
carries a hash summarizing its subtree, which is what lets reconciliation prune
matching subtrees without descending into them.

## Insert and redact

Both go through `Tree::act`, which applies a batch in one traversal and ticks
the owning party's clock **once per action**:

- **Insert** (`message` / `message_then`): adds a versioned leaf. The per-action
  tick gives content-identical inserts distinct versions (hence distinct keys).
- **Redact** (`redact`): an `Action::Forget(key)` that **removes the leaf
  outright**. Nothing is left in its place — there is no tombstone, marker, or
  deletion record anywhere in this protocol.

Redaction propagates entirely through the **version vector**. A forget ticks
the party so the resulting version *strictly dominates* the forgotten insert's
version. During reconciliation, the mirror protocol's *deletion-honoring
inference* reads that version dominance to distinguish "this peer deleted the
message" from "this peer never saw it" — the two are indistinguishable when the
versions compare equal, which is exactly why the strict tick is load-bearing.
So: redaction is "delete the leaf and advance the clock," and both convergence
and deletion-honoring are driven by **version bounds**, never by retained
state. When reasoning about how a redaction reaches a peer, think version
ceilings/floors, not markers.

## Gossiping: the mirror protocol

`Known::gossip(read, write)` reconciles two peers over a byte stream; both ends
must drive it concurrently. A session is:

1. **Preamble** — a raw 8-byte prefix: `PROTOCOL_MAGIC` (`b"RUMORS"`) + a
   big-endian `u16` `PROTOCOL_VERSION`. A wrong magic/version is rejected
   (`Error::MagicMismatch` / `Error::VersionMismatch`) *before* the
   length-delimited codec ever trusts a peer-supplied frame length, so a garbage
   peer cannot induce a huge-frame allocation. Both sides write and read it
   concurrently (`futures_util::future::try_join`) to avoid deadlock. This is
   the only raw (non-framed) exchange in a session.
2. **Connect phase (the greeting)** — each side then exchanges one
   length-delimited `message::Handshake` frame carrying its `Network` (16 raw
   bytes), its latest `Version`, and its `Intent` (a one-byte enum: `Remain`
   for gossip/bootstrap, `Retire` ⇒ "after we reconcile, I will hand you my
   party"). This single frame validates and dispatches:
   - **Network match**: two real (non-`ZERO`) networks that differ descend from
     unrelated `seed`s and are rejected as `Error::NetworkMismatch`. The all-zero
     `Network` is the placeholder a *bootstrapping* peer sends (it has no
     universe yet); a `ZERO` on either side suppresses the check and signals
     bootstrap (see below). The check lives in `remote::Exchange::accept`, the
     one step that holds both greetings at once — each peer runs its own driver,
     so both check independently, and `accept` sends our greeting *before* it can
     error so a mismatch can't deadlock.
   - **Version compare**: if the two versions are equal the peers have already
     converged and the session ends with no content transfer (the fast path,
     independent of how much content they hold).
   - **Retire / bootstrap dispatch**: `Intent::Retire` on *both* sides (mutual
     retire), or `Intent::Retire` meeting a `ZERO` network (retiree vs.
     bootstrapper), ends the session with no descent — neither counterparty can
     absorb a party. A single `Intent::Retire` against an ordinary peer
     proceeds to the descent like any gossip, after which the retiree ships its
     party as one trailing frame and the absorber joins it. A single `ZERO`
     network triggers the bootstrap party hand-off (the same trailing frame,
     opposite direction) after reconciliation.
3. **Trie descent** — otherwise (versions differ, not both-`ZERO`, not a
   declined retirement) the two sides walk their tries from the root downward in
   lockstep, comparing subtree hashes level by level. Each message carries only
   differences: `providing` (subtrees the peer lacks), `requested` (prefixes
   this side wants), and `uncertain` (prefix→hash pairs still to compare). Equal
   hashes prune whole subtrees, so only genuinely divergent regions cross the
   wire.

After the preamble, every protocol message — the greeting included — is one
length-delimited frame (4-byte big-endian length prefix + borsh body), via
`tokio_util::codec::LengthDelimitedCodec`.

The protocol's structure lives in `src/tree/traverse/mirror/`:

- `protocol.rs` — a trait family where each step's associated `Next` type names
  the only legal following call, so the height-indexed phase schedule (Connect/
  Accept → Initiator/Responder → OpenInitiator → Exchange (recursive descent) →
  CloseInitiator → CompleteResponder/Initiator) is enforced at the type level.
- `local.rs` — realizes the protocol by traversing the in-memory tree.
- `remote.rs` — realizes the *same* traits as a wire proxy of the counterparty:
  each step serializes the outgoing message and deserializes the reply. Driving
  a `local` against a `remote` is what `gossip` does. Also home to the raw
  `preamble` exchange and the fork-last party hand-off (`send_party` /
  `recv_party`) that bootstrap and retire complete with.
- `mirror.rs` — `connect_phase` runs the greeting exchange and hands back a
  `Phase` (`Converged`/`Diverged` plus the peer's `Handshake`) for the caller to
  dispatch on; `descend` runs the steady-state recursive descent.

## Bootstrapping a party from a remote peer

A process holding *nothing* (no network, no party, no tree) can join a universe
with the associated function `Known::bootstrap(read, write)` (returns
`Result<Option<Known<T>>, Error>`; `sync` mirror too). It declares itself by
greeting with the all-zero `Network` placeholder and an empty tree; the outcome
is decided by which side(s) sent the placeholder:

- **both bootstrapping** (both `ZERO`) → neither has state to give, both return
  `Ok(None)`;
- **neither** (both real networks) → ordinary mirror-gossip, unchanged
  (mismatched networks are rejected at the greeting);
- **exactly one** → the already-bootstrapped side (running `gossip`) *serves*.
  Bootstrapping is **not** a separate bulk transfer: the empty bootstrapper just
  runs the ordinary mirror descent, pulling all of the provider's content through
  the usual `providing` channel. The descent moves content but not parties, so
  afterward the provider forks its party and sends the give-half as a single
  trailing frame (`send_party`); the bootstrapper reads it (`recv_party`),
  adopts the provider's `Network` (learned in the greeting), and assembles a
  `Known` causally equivalent to a local `fork`.

The send order (descend, **then** fork+party last) is load-bearing: a failure
during the descent never costs a party region, and the residual leak window is
one tiny frame. No acknowledgement could close that window (two-generals), so
forking last is the structural minimum and costs no extra round-trip. See the
module docs on `send_party` in `remote.rs`.

## Retiring: handing a party back

`Known::retire(read, write)` is the inverse of bootstrap: a peer **leaves** a
universe by handing its ITC party to a peer that reclaims the id-region rather
than leaking it. It returns `Result<Option<Self>, RetireError<Self>>`:

- `Ok(None)` — **retired**: the peer handed its party over and dropped itself.
- `Ok(Some(self))` — **declined, unchanged**: handed back intact to retry
  elsewhere.
- `Err(RetireError::Recovered { error, known })` — the session failed strictly
  *before* the trailing party frame was sent, so the peer provably does not
  hold the party: `known` is the intact retiree (content as of the start of
  the session), ready to retry. Nothing was lost.
- `Err(RetireError::Uncertain { error })` — the session failed while sending
  the party frame itself: the peer *may* hold the party, so the retiree is
  consumed (keeping a copy could duplicate the region).

A retire session **begins with a round of gossip**: retiree `R` and absorber
`N` run the ordinary mirror descent (a no-op if their versions already match),
so `N` comes to **causally dominate** `R` by construction before the party
changes hands. No prior synchronization is required, and nothing `R` held is
lost — its novel content (and redactions) reach `N` through the descent exactly
as plain gossip would carry them. Content `R` learns from `N` in return is
dropped along with `R`. Declines remain only for counterparties that
structurally cannot absorb a party: a peer that is itself retiring, or a
bootstrapper (decided at the greeting, no descent).

The party itself never rides the greeting: `R` greets with `Intent::Retire`
(announcement only), and ships `self.party.dangerously_alias()` as a **single
trailing frame** on the same framed writer the descent used — bootstrap's
fork-last structure, in the opposite direction (both directions reuse
`send_party` / `recv_party` in `remote.rs`). Sending the party last keeps the
id-region out of limbo for the whole reconciliation: a failure anywhere before
that frame returns `RetireError::Recovered` with the retiree intact, and only
a failure on the frame itself (`RetireError::Uncertain`) forces the
two-generals assumption — `R` must treat the party as delivered and drop its
copy, so the worst case is a *leaked* region, never a *duplicated* one.
Exactly one side ever treats the alias as live: `N` joins it on receipt; `R`
drops its copy by dropping itself. `before::Party::dangerously_alias` is the
named linearity escape hatch this relies on (it deliberately violates
disjointness; retire is responsible for keeping exactly one copy live). Plain
`gossip` **transparently absorbs** a retiree (reconciles, then reads and joins
the trailing party frame), exactly as it transparently serves a bootstrapper,
so the counterparty needs no special call — its `gossip_then` callbacks
observe whatever the retiree contributes. The retiree itself discards what it
learns, so there is still no `retire_then` callback variant. The severed-wire
behavior is pinned by fault-injection tests in `src/tests.rs`
(`severed_descent_recovers_the_retiree`, `severed_party_frame_is_uncertain`),
which cut the duplex at exact byte offsets with a budgeted `Fuse` writer.

## Testing notes

- Wire-equivalence proptests assert that gossiping over real I/O produces the
  same converged content as an in-process `join`.
- `tests/gossip_snapshot.rs` pins byte-level golden captures of whole sessions
  (see `tests/common/gossip_snapshot.rs` for the recording duplex). These are
  deterministic because both peers run on a single current-thread runtime;
  re-accept them only after a deliberate protocol change.
- Commit `proptest-regressions/**` seed files; never strip them from diffs.
