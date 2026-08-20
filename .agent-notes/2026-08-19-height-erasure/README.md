# Height erasure for the streaming mirror's plumbing: the record

Status: implemented (the four migration steps below); this note is the
decision and measurement record. The mechanism's documentation of record
is the rustdoc — start at `streaming::erased` and the `materialized` and
`remote` module docs.

## What was built

The streaming (V2) mirror builds its executor-free runtime out of the
type system, and before this work it built it once per trie height: the
height-indexed vocabulary flowed through height-indexed channels,
generators, pumps, and adapters, so each of ~31 heights instantiated the
whole tokio mpsc stack, two async_stream state machines, and per-height
encode/decode adapters — all re-monomorphized per payload type per
downstream binary.

The height parameter was phantom at every layer that matters (one Arc'd
node representation for every height; prefix length as runtime data; a
wire that never sees H), so erasure was re-tagging, not
re-representation: no unsafe, no transmutes, no copies, zero wire-format
change. The byte-pinned snapshots were the acceptance test for every
step and never moved.

What landed, by commit:

1. **The seam** (`streaming: add the height-erasure seam on Backend`):
   `Backend::{Erased, erase, assume}`, the `ErasedNode` observation
   trait, `ErasedPrefix` (length = height witness), implementations for
   `Local` and the three test/conformance wrapper backends.
2. **Channels** (`streaming: erase the materialized channels' payloads`):
   erased channel payloads behind per-edge typed facades.
   Measured: 2,884,887 → 2,609,789 IR lines on `--test pairwise`
   (debug, default features); rows naming tokio's mpsc 632k → 235k.
3. **The walk** (`streaming: erase the materialized walk's workers`):
   the vocabulary itself went erased (`Query<E>`/`Resolution<E>`/
   `Resolve<E>` over `Backend::Erased`), the level loops, answerer,
   resolver, assembler, and deletion filter became shared bodies behind
   thin typed shells, and step 2's facades dissolved into them. Erased
   code reaches the typed `Backend` GATs through the 33-arm
   runtime-to-type dispatch in `erased::ops`, keyed on prefix length.
   Measured: → 1,717,836 lines; materialized-labeled rows 662k → 274k.
4. **The proxy** (`streaming: erase the remote proxy's scopes, adapter,
   and pumps`): `Scope` erased to one type, the adapter's encode/decode
   and the pump/encode workers shared, with `ops::{leaves, assemble}`
   dispatching the two stream-shaped backend ops.
   Measured: → 1,039,534 lines / 41,172 copies.

Cumulative: **2,884,887 → 1,039,534 IR lines (−64%), 109,613 → 41,172
copies** on the `pairwise` binary. (The design's original baseline of
2.04M had drifted to 2.88M before this work began; the drift is
pre-existing growth, not part of this attribution.)

Runtime pin: `gossip_fixed_bidir_insertions/V2/5000`, parent vs.
result, measured back-to-back on ox-east-1 under `pset-run -n 8`
(scheduler-quiet reserved cores; local attempts were discarded as
contended by concurrent builds and are not comparable):

- parent: 54.461–54.553 ms (point estimate 54.506 ms)
- erased: 54.031–54.182 ms (point estimate 54.095 ms)

A −0.75% point delta with nearly touching confidence intervals: no
runtime movement.

Compile cost, measured on the same box (stable 1.96.1, no build cache,
dependencies warm, then `cargo clean -p rumors` and a non-incremental
`cargo build --locked --tests` — the fleet-wide cost of compiling the
crate's own code into every test binary):

- parent: 7,819 CPU-seconds (7,426 user + 392 sys), 423 s wall
- erased: 3,093 CPU-seconds (2,906 user + 186 sys), 271 s wall

−60% CPU, −36% wall. CPU-seconds are the load-tolerant figure; wall
clock is bounded below by the dependency graph's critical path.

## Step 5: the `Tagged<B::Erased, H>` collapse — declined

The optional fifth step (flipping `Backend::Node<H>` to a mirror-layer
`Tagged<B::Erased, H>` wrapper so the GAT and the seam collapse) was
conditioned on the typed facades feeling heavy after 1–4 settled. They
do not: the surviving typed surface is the protocol typestates (which
must stay typed — they are the compile-time schedule proof), one
request-erasure map and one reply re-tag per stage, and the fixed-height
root re-tags. Collapsing the GAT would reshape a public-ish trait to
remove code that no longer shows up in the measurements. Revisit only if
a future backend's `Node<H>` stops being phantom-convertible.

## What the types stopped proving, and what catches it instead

Cross-height pairing inside the walk and proxy is now a
runtime-witnessed property rather than a compile error: every
`ErasedPrefix::assume` debug-asserts its byte length against the claimed
height, `erased::ops` derives its dispatch height from that same length
(coordinate and witness cannot drift), every channel keeps its
`QueueRole` height label, and the behavioral pins (alternating oracle,
violation/capacity suites, byte-pinned wire snapshots) exercise exactly
these pairings. The schedule itself — phase order, role alternation,
bottoming at `Z` — stays compile-time. Peer input cannot reach a
mispairing: wire prefixes decode through height-typed readers.

## Resolutions of the sketch's open questions

- **Coherence tricks on distinct `Node<H>` types**: nothing relied on
  them; the `Leaf` bound at `Z` survives at the typed shells.
- **`future_size.rs`**: verified in a release run after step 4 — all
  three pins pass unchanged. The public futures were already
  type-erased, and the budget is a generous order-of-magnitude
  tripwire.
- **The fused `mirror!`/`seq!` drivers**: still linear-in-height inside
  one function; after erasure they are no longer among the largest
  functions (~5k lines/copy, one copy per peer pairing), so the boxing
  follow-up stays unneeded.

## Incident log

(Retained from the design sketch, as history.)

- **2026-07-17: the lib test binary crossed the memwatch limit.** The
  link axis, not the height axis: every distinct `Link` type driven into
  `remote::Handshaking::start` instantiates the whole proxy tower, and
  the in-crate tests had accumulated fixture-wrapped link types. One
  additional tower instantiation cost +137k IR lines but +0.7 GiB of
  rustc peak memory; the tripwire's default was raised from 8 to 12 GiB.
  The durable fix named then was this height erasure, which shrank every
  tower from the inside.

## What followed: the T axis

Height erasure deliberately kept the payload axis. The follow-on work
erased `T` at the leaf conversion boundary so the subsystem compiles
once into the rlib — its record is
[`2026-08-20-item-erasure`](../2026-08-20-item-erasure/README.md) —
and height erasure shrank what that phase had to move.
