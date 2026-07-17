# Height erasure for the streaming mirror's plumbing

Status: sketch, not implemented. Companion to the compile-time work of
2026-07-16 (transport erasure at the session boundary; `protocol-v1`
feature gate). Numbers below are *measured* with `cargo llvm-lines` on
`--test pairwise` (debug profile, default features) unless marked
*derived*.

## The problem, precisely

The streaming (V2) mirror gets fixed memory and pipelining without an
executor: every logical stream keeps moving under plain polling, with no
`tokio::spawn`. It builds that runtime out of the type system — and it
builds it **once per trie height**. The height-indexed vocabulary
(`message::Reply<B, T, H>`, `Query<B, T, H>`, `Resolution<B, T, H>`)
flows through height-indexed channels, generators, pumps, and adapters,
so each of the ~31 heights instantiates:

- ~6 typed mpsc channels (`materialized/work/queues.rs`), each dragging
  tokio's whole mpsc stack (~1k IR lines per payload type);
- two `async_stream` generator state machines (`work/levels.rs`,
  the proxy pumps), each with its own drop glue;
- per-height encode/decode adapters and reply pumps on the proxy side.

Measured, in one test binary after V1 gating: **2.04M lines of LLVM IR
total**, of which ~696k is streaming-marked symbols, ~446k is
`tokio::sync::mpsc` + `async_stream` machinery instantiated on the
height axis, and a large share of the remaining `core`/`alloc` glue and
12k+ `drop_in_place` symbols is induced by the same height-indexed
types. All of it re-monomorphizes per payload type `T`, per downstream
binary (24 test binaries exercise sessions).

Boxing cannot fix this: `BoxResponses` already caps how deeply the
stream *types nest* (the one genuinely exponential hazard, long since
fixed), but boxing does not reduce the instantiation *count* — every
boxed stream still has a height-indexed item type.

## The load-bearing observation: the runtime is already erased

The height parameter is phantom at every layer that matters:

- `typed::Node<T, H>` wraps `untyped::Node<T>` (one `Arc`'d
  representation for every height; the prefix length is runtime data) —
  `src/tree/typed/node.rs`, `src/tree/typed/untyped.rs`.
- `Prefix<H>` is `PhantomData<fn() -> H>` over an
  `ArrayVec<[u8; 32]>` whose length (`32 - H::HEIGHT`) is runtime data —
  `src/tree/typed/prefix.rs`.
- The wire never sees `H`: the V2 codec's runtime signal byte
  (`state * 17 + stream`) names the logical stream on every frame — no
  longer a demux key, since under the Link transport nothing
  multiplexes; it is per-stream redundancy validated to exact equality
  with the claimed label (`streaming-wire-deadlock.md` §8.6) — and the
  channel layer already threads `H::HEIGHT` as a runtime `u8` for
  diagnostics
  (`QueueRole::new(kind, H::HEIGHT)` in
  `streaming/materialized/work/queues.rs`).

So height erasure is **re-tagging, not re-representation**: every
conversion is a `PhantomData` swap over the value the program already
holds. No `unsafe`, no transmutes, no copies, and — because the encoder
never consumed `H` — **zero wire-format change**. The byte-pinned
snapshots must not move; that is the acceptance test for every step.

## The design

Keep the typed phase schedule as the interface; erase everything that
flows *through* it.

### What stays typed (the guarantees we keep)

`protocol.rs`'s traits (`Initiator`, `Responder`, `Reply`,
`CompleteResponder`, `CompleteInitiator`, `ReplyHeight`), the
`Descending<B, T, H, R, W>` typestates, and the `mirror!` driver
schedule. These are what prove, at compile time, that phases run in
order, each side speaks when it must, and the descent bottoms out at
`Z` instead of trusting a depth counter. They are also *cheap*: each
state is a small struct and a thin method once the workers beneath them
are shared.

### The erased seam on `Backend`

```rust
pub trait Backend<T: ...>: ... {
    type Node<H: Height>: ...;            // unchanged
    /// One runtime representation shared by every height's `Node<H>`.
    type Erased: Clone + Send + 'static;
    /// Forget a node's height tag. Free for `Local` (a field move).
    fn erase<H: Height>(node: Self::Node<H>) -> Self::Erased;
    /// Re-tag an erased node at height `H`.
    ///
    /// Contract: `assume::<H>(erase::<H>(n)) == n`. Debug builds check
    /// the runtime height (prefix length) and panic on a cross-level
    /// value; release builds trust the erased module's internal
    /// invariant (see "What the types stop proving").
    fn assume<H: Height>(erased: Self::Erased) -> Self::Node<H>;
}
```

For `Local`: `Erased = untyped::Node<T>`, and both conversions are the
existing phantom wrap/unwrap. A hypothetical backend with genuinely
different per-height representations supplies an enum. (`Backend` is
public; adding an associated type is a breaking change — acceptable
pre-1.0, and the crate docs already reserve API reshaping.)

`Prefix<H>` gets the same pair (`Prefix::erase() -> ErasedPrefix`,
`ErasedPrefix::assume::<H>()` with a `debug_assert_eq!(len,
32 - H::HEIGHT)`), where `ErasedPrefix` is the `ArrayVec` it already
is.

### The erased vocabulary and channels

One module (say `streaming/erased.rs`) defines the height-free frames:

```rust
struct Reply<B, T>      { replies: Vec<Reaction<B, T>> }
enum   Reaction<B, T>   { Supply(u8, B::Erased), Match, Query(Vec<(u8, Hash)>) }
struct Query<B, T>      { prefix: ErasedPrefix, ours: Vec<(u8, B::Erased)> }
struct Resolution<B, T> { prefix: ErasedPrefix, resolved: ... }
```

`queues.rs` mints channels of *these* — one mpsc instantiation per
`(B, T)` instead of per `(B, T, H)`. The typed facade is a pair of
one-line wrappers:

```rust
struct TypedSender<M, H>(Sender<Erased>, PhantomData<fn() -> H>);
```

whose `send` erases and whose `recv` re-tags. The per-height residue is
these adapters and the `BoxResponses<H>` map-streams at the schedule
boundary — one small map per height instead of the full machinery.

### The erased workers

`internal_level<H>` (and `leaf_parent_level`, `leaf_level`,
`answer::internal`, `Resolver`, `children_of`, `assemble`, the
`unknown` walk, the `convert` folds, and the proxy's
`encode`/`decode`/`internal_replies` pumps) each become a thin typed
shell — capture `H::HEIGHT` as a `u8`, erase the inputs, delegate,
re-tag the outputs — over one shared worker. Two workers keep a runtime
branch the types used to make static:

- the leaf boundary: leaves carry `Message<T>` and content-addressing;
  the erased worker branches where `untyped::Children::Leaf` already
  discriminates, and `height == 0` must agree with that discriminant
  (debug-asserted);
- the `unknown` recursion: a loop over runtime height replaces 31
  `BoxFuture` instantiations (that module already boxes each level, so
  this is strictly less machinery).

### What the types stop proving, and what catches it instead

Today, sending a level-5 resolution into a level-6 queue is a compile
error. After erasure it is a bug class again — confined as follows:

1. **Locality.** The typed facade is the only entry to the erased
   module; a mispairing can only be authored inside it. The audit
   surface is one module, not the codebase.
2. **Runtime witnesses.** Every `assume` debug-asserts the prefix
   length; `QueueRole` already labels every channel with its height, so
   a violation names itself.
3. **The oracle and the violation suite.** The alternating oracle tests
   (`streaming/tests`), the adversarial-schedule backend, and
   `work/tests/violations.rs` exercise exactly these pairings; the
   byte-pinned wire snapshots pin the external behavior.
4. **Unchanged threat model.** Peer-controlled input was always
   validated at runtime (the typed layer never protected the wire);
   the schedule itself — phase order, role alternation, bottoming at
   `Z` — stays compile-time.

### What it buys (derived)

- mpsc/`async_stream` machinery: ~446k lines ÷ ~31 heights → ~15–30k.
- Streaming-marked symbols: the per-height workers dominate the ~696k;
  shared workers leave roughly 150–250k (schedule shells + one worker
  set + leaf specifics).
- Drop glue and `core`/`alloc` glue shrink proportionally (they are
  per-type artifacts of the erased population).

Estimate: `pairwise` drops from ~2.04M to roughly **0.7–1.0M lines**,
and — more important for the fleet — each *additional* payload type or
test binary re-buys only the residue, not the tower. The axis this does
NOT remove is `T` itself: compiling the whole subsystem once, inside
`rumors`, additionally requires content erasure at the leaf (the
conversion boundary already named in the streaming module docs — leaves
cross the wire as canonical borsh bytes today). That is phase 2,
orthogonal, and composes: height erasure shrinks what phase 2 would
move.

## Migration plan (each step lands gate-clean, snapshots byte-identical)

1. **Seam.** Add `Backend::Erased` + `erase`/`assume`, `ErasedPrefix`,
   and the `Local` impl. No caller changes; pure addition.
2. **Channels.** Erase `queues.rs`/`channel.rs` payloads behind
   `TypedSender`/`TypedReceiver`. Measure (expect the mpsc block to
   collapse).
3. **Materialized workers.** One erased worker behind
   `internal_level`/`leaf_parent_level`/`leaf_level`;
   erase `answer`/`Resolver`/`assemble`/`unknown`. The oracle suite is
   the behavioral pin. Measure.
4. **Proxy workers.** Erase the adapter encode/decode and reply pumps
   (the codec beneath them is already runtime-indexed). Wire snapshots
   are the pin. Measure.
5. **Optional cleanup.** If the facades feel heavy, consider flipping
   `Backend::Node<H>` to a mirror-layer `Tagged<B::Erased, H>` wrapper
   so the GAT and the seam collapse into one shape. Bigger surface
   change; only worth it once 1–4 have settled.

## Incident log

- **2026-07-17: the lib test binary crossed the memwatch limit.** The link
  axis, not the height axis: every distinct `Link` type driven into
  `remote::Handshaking::start` instantiates the whole proxy tower, and the
  in-crate tests had accumulated fixture-wrapped link types (memory,
  adversarial, scripted; a reordering acceptor tipped it over). Measured
  on the lib test target (stable 1.96.1, all features): one additional
  tower instantiation cost +137k IR lines but **+0.7 GiB of rustc peak
  memory** — the cost is type-tree and collector state, not codegen
  volume — and under incremental CGU partitioning (every tower lands in
  the proxy module's CGU) the same delta took the compile from ~7.3 GB to
  the 9.4 GB memwatch kill. Fixed by `Link::into_erased` (test-only,
  `src/link/erased.rs`): fixtures now funnel into one owned erased
  carrier, mirroring the session funnels' erasure, dropping the tower
  population from 7 to 4 (`internal_replies` 210 → 120 copies; peak
  9.4+ → 7.7 GB cold-incremental). The erasure must be *owning*: a
  borrowed carrier leaves the concrete connector alive in the caller and
  the peer's supply never closes, which stalls every supply-closure test.
  The residual four towers are the funnels' borrowed carrier plus the
  payload/backend axes — exactly what this document's height erasure
  would shrink from the inside.

## Open questions

- Does any code rely on `B::Node<H>` being *distinct types* per height
  for coherence tricks (blanket impls keyed on `Node<Z>: Leaf<T>`)? The
  `Leaf` bound at `Z` survives (the typed shells still see `Node<Z>`),
  but step 5 would need care here.
- `future_size.rs` pins session-future sizes; the erased state machines
  are smaller, so the pins move once (deliberately).
- The fused `mirror!`/`seq!` drivers stay linear-in-height inside one
  function (~5k IR lines/copy). If they remain the largest single
  functions after erasure, box each phase step — a small, independent
  follow-up.
