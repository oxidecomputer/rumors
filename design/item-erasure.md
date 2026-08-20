# Item-type erasure at the leaf boundary (part II sketch)

Status: accepted (option B, with the rulings below); implementation in
progress on the `height-erasure` branch. Numbers marked *measured* come
from `cargo llvm-lines --test pairwise` (debug, default features) on
the height-erased tree.

## The problem, precisely

Height erasure removed the per-height axis: the streaming session's
channels, walks, and proxy pumps instantiate once per backend. The axis
it deliberately kept is the payload type `T`. Everything the session
touches is still generic over `T`:

- `Backend<T>::Erased = untyped::Node<T>`: nodes hold `Message<T>` at
  their leaves, so every erased worker, channel, and dispatch arm
  re-monomorphizes per `T`;
- the codec's `Frame<T>` / `LeafRun<T>` and the leaf conversion boundary
  (`Leaf::leaf(Version, Message<T>)`, `Node::message() -> &Message<T>`);
- the tree itself (`untyped::Node<T>`, the traversals, the CRDT ops).

Because this code is generic, none of it is compiled into the `rumors`
rlib: every downstream binary re-instantiates it. Measured: the
`pairwise` test binary carries ~1.04M IR lines after height erasure, and
essentially all of it is generic re-instantiation — the marginal cost of
each additional payload type *or* additional binary. The fleet effect is
the 24 session-exercising test binaries each re-buying the subsystem.

"Compiles only once" means: the session subsystem (and ideally the tree)
becomes non-generic code, codegen'd once into the rlib, with a thin
typed facade per `T`.

## The load-bearing observation: the wire is already erased

On the wire a leaf is `(Version, payload bytes)`: the codec serializes
`Message<T>` to canonical CBOR at encode and deserializes at decode —
`T` exists on the wire only as bytes. Likewise the tree's identity
layer: a leaf's path derives from its version, and its hash commits to
canonical bytes, not to `T`'s shape. The only operations that genuinely
need `T` are the user-facing reads (`Rumors::iter`, `get`) and
insertion.

## Design options

The decision is where `Message<T> ⇄ canonical bytes` conversion lives.

### Option A: erase at the session boundary only

The tree keeps storing `Message<T>`; sessions convert each supplied leaf
to bytes at encode and construct `Message<T>` at decode (what the codec
already does). The session core becomes generic over an opaque
`Payload = raw canonical bytes` instead of `T`.

- Buys: the streaming subsystem compiles once. The tree stays generic.
- Costs: nothing new at runtime (the encode/decode conversions already
  happen at exactly these points).
- Residue: the tree + CRDT ops still re-instantiate per `T` per binary.
  Measured share (rows naming the tree layers in the height-erased
  `pairwise` binary, overlapping): `tree::typed` 728k of the 1.04M
  total, `typed::untyped` 318k, `tree::traverse` 186k — the tree, not
  the session, is now the larger half of the re-bought code, which
  argues for option B (or C's second phase) rather than stopping at A.

### Option B (recommended; Finch's shape): erase the stored value
behind `Arc<dyn Any>`

`Message<T>` already stores `{ message: Arc<T>, serialized: Bytes }`,
and everything the tree and session do with a payload other than the
typed reads — the hash preimage, wire encode, size accounting — reads
the cached canonical bytes, never the value. So the erased message is
simply

```rust
struct Message /* erased */ {
    message: Arc<dyn Any + Send + Sync>,  // the same allocation,
                                          // unsized: +8 B fat pointer
    serialized: Bytes,                    // unchanged, and already
                                          // everything the erased
                                          // core consumes
}
```

with the typed boundary reading by checked downcast (a `TypeId`
compare; a failed downcast is a *caught* mispairing — stronger
witnessing than the height seam's debug-only prefix asserts). The one
per-`T` residue beyond the facade is a deserialize witness
(`fn(&[u8]) -> Result<Arc<dyn Any + Send + Sync>, _>`) threaded from
peer construction to the wire-decode boundary, which keeps malformed
payloads failing at ingress as `DecodeError::Record` exactly as today.

- Buys: tree + session compile once — the full "compiles only once" —
  with today's runtime behavior preserved at every point: reads free,
  ingress single-decode, hash/encode off the cached bytes.
- Costs: a fat pointer per `Message` handle and a `TypeId` compare per
  typed read; the `gossip_fixed` bench pin guards the claim that this
  is nothing.
- Public API movement, owner-ruled (see the rulings): `Message` itself
  is crate-internal (nothing re-exports it; verified against the
  public rustdoc surface), but two of the crate's faces move. The
  observers already speak owned `(Version, Arc<T>)`, and the
  `Snapshot` faces join them: a coerced `Arc<dyn Any>` is a fat
  pointer sharing the `T` allocation, with no `Arc<T>` object anywhere
  to lend, so the former `&Arc<T>`-lending faces become owned — each
  yielded item one `Arc::downcast::<T>()`, a refcount bump plus the
  `TypeId` check, exactly what a keeper paid before. And `Any` demands
  `T: 'static`, which the insert paths previously did not.

A decode-on-read variant (store the bytes only, decode at the typed
boundary) loses to this on every axis: it saves the fat pointer but
charges every read a CBOR decode that today is free.

### Option C: A now, B later

A is strictly smaller and proves the session seam; B builds on it. But
the measured residue above says the tree is the larger half, so
stopping at A forfeits most of the prize.

## What the types stop proving

`T`'s type safety at the session boundary is today only the guarantee
that both ends of an in-process session speak the same `T`; on the wire
it was never checked beyond CBOR well-formedness. Erasure moves nothing
security-relevant: payload validation stays exactly where it is
(deserialization at the read boundary).

## Acceptance

- Wire snapshots byte-identical (the wire never sees the change).
- The oracle and violation suites as behavioral pins.
- `cargo llvm-lines` per step, plus the new headline number: IR lines of
  a *minimal downstream binary* (one `gossip` call), before and after —
  the "what does the next consumer pay" meter.
- The `gossip_fixed_bidir_insertions/5000` bench as the runtime pin.

## Rulings (owner-resolved)

1. **Witness minting**: at `Peer<T>` construction — one witness per
   peer lifetime, stored alongside the erased tree in the shared inner
   state and threaded to every session. `DeserializeOwned` lives at
   construction; the gossip entry points carry no serde bounds.
2. **Sealing**: non-generic core functions in the rlib, with the
   public API unchanged — byte-for-byte the same signatures. If the
   non-generic sealing turns structurally hairy, back it out and fall
   back to generic shells that erase immediately: legibility outranks
   structural purity here.
3. **Scope**: one stroke — tree and session erase together, staged as
   gate-clean commits (the part I pattern); no transient
   typed-tree/erased-session seam.
4. **Branch**: continues on `height-erasure`, stacking on part I.
5. **Holder shape and the payload faces**: the single coerced
   `Arc<dyn Any + Send + Sync>` (the same allocation, unsized in
   place), with the `Snapshot` faces going owned: `iter`/`range` yield
   `(&Version, Arc<T>)` and `get` returns `Option<Arc<T>>` — `get` no
   longer echoes a version at all, because a leaf's path derives from
   its version, so the hit's version is always the queried one. The
   alternative (an extra box holding a concrete `Arc<T>`, to keep
   lending `&Arc<T>`) was rejected: it charges the gossip path a
   malloc per message at insert and wire ingress to subsidize
   look-only application scans, and the box is pure plumbing whose
   only justification is preserving a signature.
6. **`T: 'static` at the insert paths**: added. Safe type erasure is
   `TypeId`-based and `Any` requires `'static`; gossip already
   demanded it, and the local-only borrowed-payload usage that
   previously compiled (verified by probe) was unintended surface.

## Implementation plan (staged, each commit gate-clean)

1. **The erased `Message`**: the crate-internal `Message<T>` becomes
   non-generic — `{ message: Arc<dyn Any + Send + Sync>, serialized:
   Bytes }` — with typed constructors at the insert boundary and
   checked typed accessors (downcast) at the read boundary. Measure.
   *Measured*: `pairwise` at 1,039,827 lines / 41,186 copies — flat
   against the 1,039,534 / 41,172 baseline, as this stage predicts:
   the payload type leaves storage, but every instantiation still
   exists because the tree and session stay generic over the now-
   phantom `T`. The dedup is stage 2's and 3's to collect.
2. **The erased tree**: `untyped::Node<T>` and the typed veneer drop
   `T` (the stored `Message` was its only occurrence); iterators and
   walks yield erased leaves; `Rumors`/`Snapshot`/observer facades
   downcast at the door. The V1 oracle and the wire codecs sweep along.
   Measure.
   *Measured*: `pairwise` at 719,899 lines / 30,285 copies, from
   1,039,827 / 41,186 — −30.8% per consumer binary — after two moves:
   the tree layers going non-generic (the `join`/`unknown` towers then
   codegen once in the rlib: present in the lib's own measurement,
   absent from the binary's), and the batch-apply entry going
   *monomorphic* (`Vec` in, `&mut dyn FnMut` observer), because a
   generic entry re-instantiated the whole per-height apply tower —
   radix grouping included, ~155k lines — in every consumer despite
   the payload erasure. The lib's own codegen grows 32,262 → ~102k:
   the once-paid residence of what consumers stopped re-buying.
   Measurement lens, corrected en route: `cargo llvm-lines` counts
   only the named target's crate, so the per-binary number IS the
   marginal cost, and substring attribution overcounts (session
   workers' signatures mention tree type names). The residue is the
   streaming session, still generic over `(B, T)` — stage 3's scope,
   and where the trade's verdict lands.
3. **The erased session**: `Backend<T>` drops `T`; the codec's record
   decode keeps the wire bytes and builds payloads through the witness;
   sessions receive the witness from the peer. Measure.
4. **Sealing**: the session core's entry points go non-generic over
   the erased tree, witness, and dyn-erased link (public API
   unchanged). The new meter lands here: IR lines of a minimal
   downstream binary (one `gossip` call) — the "what the next consumer
   pays" number. Measure; bench pin at the end.
