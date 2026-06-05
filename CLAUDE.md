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

A `Known<T>` is just a `Party` (its ITC clock identity) plus a `Tree<T>` (its
content). It is deliberately **`!Clone`**: duplicating a live clock would break
the linearity ITC requires.

- **`seed()`** mints the single distinguished root party of a *universe* of
  cooperating peers. Call it once per universe.
- **`fork()`** is the only way to get another working copy: a *true causal
  fork* that mints a fresh **disjoint** party and shares the current tree
  copy-on-write. Both halves then act independently and concurrently.
- **`join()` / `join_then()`** reunite a fork: merge the trees and rejoin the
  two parties' clock regions. Disjointness is what keeps every party's history
  causally well-defined no matter how forks and joins interleave.
- **The one rule for callers:** never let two independently-`seed`ed universes
  gossip — they share no causal history (the `before` crate's Law of
  Disjointness). Within a universe, fork/join freely.

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

1. **Handshake** — an 8-byte preamble: `PROTOCOL_MAGIC` (`b"RUMORS"`) + a
   big-endian `u16` `PROTOCOL_VERSION`. A wrong magic/version is rejected
   (`Error::MagicMismatch` / `Error::VersionMismatch`) before any rumor state is
   touched. Both sides write and read it concurrently to avoid deadlock.
2. **Version exchange** — each side sends its latest `Version`. If they're
   equal, the peers have already converged and the session ends immediately
   with no content transfer (this is the fast path, independent of how much
   content they hold).
3. **Trie descent** — otherwise the two sides walk their tries from the root
   downward in lockstep, comparing subtree hashes level by level. Each message
   carries only differences: `providing` (subtrees the peer lacks),
   `requested` (prefixes this side wants), and `uncertain` (prefix→hash pairs
   still to compare). Equal hashes prune whole subtrees, so only genuinely
   divergent regions cross the wire.

After the handshake, each protocol message is one length-delimited frame
(4-byte big-endian length prefix + borsh body), via
`tokio_util::codec::LengthDelimitedCodec`.

The protocol's structure lives in `src/tree/traverse/mirror/`:

- `protocol.rs` — a trait family where each step's associated `Next` type names
  the only legal following call, so the height-indexed phase schedule (Accept →
  Initiator/Responder → OpenInitiator → Exchange (recursive descent) →
  CloseInitiator → CompleteResponder/Initiator) is enforced at the type level.
- `local.rs` — realizes the protocol by traversing the in-memory tree.
- `remote.rs` — realizes the *same* traits as a wire proxy of the counterparty:
  each step serializes the outgoing message and deserializes the reply. Driving
  a `local` against a `remote` is what `gossip` does.

## Testing notes

- Wire-equivalence proptests assert that gossiping over real I/O produces the
  same converged content as an in-process `join`.
- `tests/gossip_snapshot.rs` pins byte-level golden captures of whole sessions
  (see `tests/common/gossip_snapshot.rs` for the recording duplex). These are
  deterministic because both peers run on a single current-thread runtime;
  re-accept them only after a deliberate protocol change.
- Commit `proptest-regressions/**` seed files; never strip them from diffs.
