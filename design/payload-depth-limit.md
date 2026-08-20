# The payload nesting-depth limit

The configurable, symmetric bound on how deeply a message payload's CBOR
encoding may nest, and the send-path shape built around it. This
document records the design and its rulings; the contracts of record are
the rustdoc — start at `Peer::payload_depth_limit`, whose docs carry
every invariant inline, then `Rumors::batch` for the batch lifecycle.

## The problem

CBOR payloads decode through a recursion-limited reader: a nesting bound
must exist, or a deeply nested value overflows the decode stack. But a
fixed, implicit, decode-only bound has a failure mode of its own: the
serializer accepts a value of any depth, so a payload nested past the
decoder's bound is accepted and stored locally while every transfer of
that leaf to any peer fails — a deterministic gossip wedge on a
locally-legal input, persisting for as long as the divergence does — and
the payload-facing documentation named no limit at all.

## The design

One limit, `PayloadDepthLimit`, a peer setup value like the selected
protocol, enforced at three points that share a single accounting:

- **Send admission**: `Message::try_new` serializes and runs an O(n)
  iterative depth scan over the produced bytes (the wire's head grammar
  with an explicit stack of remaining-child counts, so input-controlled
  depth never recurses); an over-deep value is a typed
  `PayloadDepthError` at its author, at the moment of choice.
- **Handshake equality**: the V2 greeting carries each side's configured
  limit, and a session proceeds only if the two are exactly equal;
  a mismatch is `Error::PayloadDepthMismatch` on both sides, after the
  greetings and before anything else (the converged-session
  short-circuit included).
- **Wire ingress**: every payload parse in the peer's orbit runs under
  `from_reader_with_recursion_limit` at the configured limit, so a
  nonconforming implementation's over-deep supply still dies typed at
  decode.

Together: between conforming peers, no V2 session can fail on payload
depth at all. Changing the limit is a fleet-coordinated configuration
event, in the same register as changing the selected protocol.

The scope accounting is deliberately not transcribed into prose or
constants anywhere: the committed differential proptest
(`depth_scanner_agrees_with_the_decoder`, `src/message/tests.rs`) holds
the send-side scanner and the decoder to the same accept/reject verdict
at the limit and on either side of it, over arrays, maps, and tags mixed
(the decoder's big-integer form included). That test is what keeps
"symmetric" true against either side drifting.

## The minted codec

The limit rides a `PayloadCodec` minted at `Peer` construction: a small
`Copy` struct pairing a payload serializer and deserializer (both fn
pointers, generic over the payload type only at the mint) with the
`PayloadDepthLimit`, threaded everywhere the bare minted deserializer
traveled before. What this provides: the limit cannot be missed — every `Message`
creation and every ingress parse in the peer's orbit goes through the
one codec value, so no creation or ingress site can carry a different
bound — the payload type's serde bounds concentrate at construction
(`Serialize` joins `DeserializeOwned` there and drops from
`Rumors::send`/`Batch::send`), and the greeting reads the limit off the
codec sessions already carry. The accepted cost: `Peer` construction
demands `Serialize` even for a peer that never sends, symmetric with
already demanding `DeserializeOwned` for a peer that never receives
(forwarding needs neither bound, since gossip re-supplies cached bytes).

## The closure-scoped batch

Fallible send forced the batch lifecycle question, and the answer
reshaped `Batch`: `Rumors::batch` runs a synchronous closure over
an exclusive `&mut Batch` scope handle and commits everything queued iff
the closure returns `Ok`. The scope type has no `Drop` impl, no `Clone`,
no `Default`, no public constructor; the higher-ranked closure bound
(with the result and error types quantified outside it) keeps the handle
from escaping, pinned by two `compile_fail` doctests. The lifecycle
collapses to one sentence — the batch commits iff the closure returns
`Ok`, all-or-nothing — and each case of the former drop-driven lifecycle
follows from that shape: a send error propagated out commits nothing; a
user `Err` commits nothing (a deliberate abort the RAII design never
offered); a panic unwinds past the commit call; and async cancellation
cannot observe a half-built batch, because a cancellation lands between
polls and the whole closure runs inside one poll. Batching's efficiency
gain — one tree traversal, one commit, one wakeup — is untouched, and
batches nest (building holds no lock; inner commits land before the
outer batch).

## Decision record

- **Symmetric, configurable, exchanged for equality** (Finch): the
  recursion limit becomes a peer setup value, enforced symmetrically
  (send-side admission and decode-side ingress judge the same bound),
  carried in the greeting, and required to match exactly; a mismatch in
  either direction is an unconditional, typed abort at the handshake.
  Negotiating down is unsound: a peer whose session limit dropped below
  its own configured limit may already hold messages deeper than the
  negotiated bound, content it is then not allowed to gossip — so any
  negotiation scheme merely relocates the failure to mid-session,
  conditional on which leaves actually differ. Parameter equality trades
  that for a deterministic fail-fast on mixed configurations at every
  pairing. Structurally the limit is a property of the *shared set* —
  every replica must be able to hold and forward all content — so it is
  Network-like (pairwise equality, transitively fleet-wide agreement),
  not `target_message_size`-like (a per-session resource trade where the
  minimum is safe).
- **Eager, fallible send** (Finch): `send` creates the `Message` at
  invocation and returns the typed error on a depth violation; a
  `Serialize`-impl failure keeps the documented panic contract
  (programmer error), while depth is data-driven and therefore an error.
- **A failed batch commits nothing** (Finch): cancel-on-error,
  all-or-nothing, earlier-queued actions included.
- **The closure scope is the no-await mechanism** (Finch): batch state
  cannot exist across an await point by language rule — a synchronous
  closure body cannot await. Rejected fiats: `!Send` binds only futures
  that must be `Send` and misstates the type; `#[must_not_suspend]` is
  unstable on the pinned toolchain; clippy's
  `await-holding-invalid-types` binds only in-repo runs. Commit-on-`Ok`
  inverts the old "performance optimization, not an atomicity
  guarantee" into a stated guarantee.
- **The minted codec** (Finch, endorsed with the fn-pointer refinement):
  serde bounds concentrate at construction; a plain fn pointer cannot
  capture a runtime value, so the concrete shape is the codec struct
  with the limit as data.
- **The 256 default** (Finch): exactly the decode bound the previous
  code enforced implicitly (the decoder's `from_reader` default), so a
  fleet upgrading together sees no acceptance change on existing
  content; the only new rejections are send-side (landing on the author
  of an over-deep value) and the handshake mismatch (landing on mixed
  configurations).
- **The V1 carve-out** (Finch): the frozen V1 greeting cannot carry the
  parameter, so V1 sessions keep decode-side-only enforcement, and
  content-conditional failure remains possible on the legacy dialect;
  the knob's rustdoc states it.

## Where the pieces live

- The knob and its full contract: `Peer::payload_depth_limit`,
  `Bootstrap::payload_depth_limit` (rustdoc).
- The codec and the scan: `src/message.rs` (`PayloadCodec`,
  `Message::try_new`, `depth_within` and its differential test).
- The greeting entry and the equality check:
  `src/tree/mirror/streaming/remote/codec/greeting.rs` and
  `src/tree/mirror/streaming/remote/proxy/start.rs`; the wire-format
  ruling's row lives in `design/cbor-legible-wire.md`.
- The batch lifecycle: `Rumors::batch` and `Batch` (rustdoc), pinned in
  `tests/single_peer.rs`.
- The peer-to-peer boundary pins: `tests/payload_depth.rs`.
