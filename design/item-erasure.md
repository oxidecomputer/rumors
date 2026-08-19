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

### Option B: erase the tree's storage too

`untyped::Node` stores `Message<Raw>` (canonical bytes); the typed
facade decodes on read (`iter`/`get` return decoded values or a lazy
view) and encodes on insert.

- Buys: tree + session compile once; the whole gossip stack becomes
  rlib code. The `T`-facade shrinks to (de)serialization at the public
  API.
- Costs: decode-on-read for iteration (each read pays a CBOR decode;
  today reads are free); or cache decoded values (memory). Insertion
  already pays one encode (for hashing) — verify: if hashing already
  encodes, insertion cost is unchanged.
- This is the full "compiles only once".

### Option C: A now, B later

A is strictly smaller and proves the seam; B builds on it. The
measurement after A decides whether B's read-path trade is worth it.

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

1. Is `Message<T>`'s canonical encoding stable enough to be the stored
   representation (option B), or is decode-on-read unacceptable for the
   read path's contract?
2. Should the erased payload be a newtype (`Payload(Bytes)`) with the
   canonicality invariant documented, or the existing
   `Message<RawValue>`-style vehicle if one exists?
3. Facade shape: seal the session core behind non-generic functions in
   the rlib (taking `&mut dyn` link objects — the transport is already
   dyn-erased), or keep generic entry points that immediately erase?
