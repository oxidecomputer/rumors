# Item-type erasure at the leaf boundary (part II sketch)

Status: sketch, not implemented — the "phase 2" the height-erasure work
names. Written after height erasure landed; numbers marked *measured*
come from `cargo llvm-lines --test pairwise` (debug, default features)
on the height-erased tree.

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
- No public API movement: `Message` is crate-internal (nothing
  re-exports it; verified against the public rustdoc surface), and the
  public observers speak owned `(Version, Arc<T>)` on every face (the
  former lending forms dissolved when `Version` went CoW), so the
  typed boundary is one `Arc::downcast::<T>()` — a refcount bump plus
  the `TypeId` check — folded into the clone each yielded item already
  pays. `T: Serialize` migrates to the insert boundary and
  `DeserializeOwned` to witness minting; the public bounds stay
  equivalent.

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

## Open questions for review

1. Witness minting site: at `Peer<T>` construction (one witness for the
   peer's lifetime, threaded through sessions), or per gossip call?
   Construction seems right — it is also where `DeserializeOwned`
   naturally lives.
2. Facade shape: seal the session core behind non-generic functions in
   the rlib (taking `&mut dyn` link objects — the transport is already
   dyn-erased), or keep generic entry points that immediately erase?
3. Does the tree erase in the same stroke (it must for the full win —
   `untyped::Node<T>`'s only `T` is the stored `Message`), or does a
   first landing keep a typed tree and erase at the session boundary
   (option A) to de-risk?
