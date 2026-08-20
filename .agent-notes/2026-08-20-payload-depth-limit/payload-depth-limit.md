# The payload nesting-depth limit

The configurable bound on how deeply a message payload's decode may
recurse, enforced identically at both ends, and the send-path shape
built around it. This document records the design and its rulings; the
contracts of record are the rustdoc — start at
`Peer::payload_depth_limit`, whose docs carry every invariant inline,
then `Rumors::batch` for the batch lifecycle.

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
protocol, enforced at three points that share a single computation:

- **Send admission**: `Message::try_new` serializes, then runs the
  peer's minted deserializer — the exact fn every receiver's wire
  ingress runs for the payload type, at the same limit — over the
  just-serialized bytes, and requires the decoded value to equal the
  value sent (by the payload type's own `Eq`, mandated in the payload
  bounds); a payload the decode rejects or misreads is a typed
  `EncodeError` at its author, at the moment of choice (`Depth` for the
  recursion limit, `Roundtrip` for a type whose `Deserialize` rejects
  its own `Serialize` output, `Unfaithful` for an encoding that decodes
  to a different value).
- **Handshake equality**: the V2 greeting carries each side's configured
  limit, and a session proceeds only if the two are exactly equal;
  a mismatch is `Error::PayloadDepthMismatch` on both sides, after the
  greetings and before anything else (the converged-session
  short-circuit included).
- **Wire ingress**: every payload parse in the peer's orbit runs under
  `from_reader_with_recursion_limit` at the configured limit, so
  over-deep *content* from a nonconforming implementation still dies
  typed at decode. The bound governs the decode's recursion, not the
  bytes' shape: byte patterns the engine consumes without recursing
  (deep tag chains in scalar positions, say) decode fine and are
  harmless, so ingress bounds what can be *accepted*, never the
  structure of what can be *sent at* it.

Together: between conforming peers, no V2 session can fail on payload
depth at all — by construction within a decode-engine version, because
admission and ingress are one computation rather than two accountings
held in agreement. The knob's number is engine-defined (the decode
engine's recursion accounting for the peer's type), not a structural
property of RFC 8949 CBOR. Changing the limit is a fleet-coordinated
configuration event, in the same register as changing the selected
protocol.

## The minted codec

The limit rides a `PayloadCodec` minted at `Peer` construction: a small
`Copy` struct pairing a payload serializer and deserializer (both fn
pointers, generic over the payload type only at the mint) with the
`PayloadDepthLimit`, threaded everywhere the bare minted deserializer
traveled before. What this provides: the limit cannot be missed — every `Message`
creation and every ingress parse in the peer's orbit goes through the
one codec value, so no creation or ingress site can carry a different
bound — the payload type's serde bounds concentrate at construction
(`Serialize` and `Eq` join `DeserializeOwned` there and drop from
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
- **Admission is the receiver's exact codepath** (Finch, ruled on the
  adversarial review's critical finding): the feature first shipped
  send-side admission as a byte-level structural scan, differentially
  tested against decoding the bytes as `ciborium::Value` — and the
  review refuted that oracle by construction. The decode engine's
  recursion accounting is *type-dependent*: decoding a serde enum
  prices its variant scope (a unit variant costs one step a `Value`
  decode of the same bytes does not), so no byte-level oracle can equal
  the receiver's `T`-decode, and an enum payload at exactly the limit —
  the crate docs' own recommended versioning shape — was admitted at
  send and wedged every receiver. The ruling: admission *is* the
  ingress computation — `Message::try_new` runs the minted deserializer
  over the just-serialized bytes and discards the value — deleting the
  scan and its differential outright, since with one computation there
  is no second accounting to keep in agreement. Corollaries: the knob's
  number is engine-defined, not RFC-structural; a payload type whose
  `Deserialize` cannot read its own `Serialize` output now fails typed
  at the author (`EncodeError::Roundtrip`) instead of at every
  receiver. Residual, stated: two binaries whose ciborium versions
  account recursion differently could still diverge on acceptance at
  equal limits — a fleet upgrades its decode engine in coordination,
  like the limit itself.
- **Faithful encoding, checked by `Eq`** (Finch): payload types must be
  `Eq` — table stakes for any wire message type — and every send
  requires the value decoded from the just-serialized bytes to equal
  the value sent, rejecting inequality as the typed
  `EncodeError::Unfaithful`. The deep why: `rumors` exists to
  synchronize causal messages so a fleet can replicate a
  causally-convergent state machine from the stream; a message that
  decodes to anything other than what was meant violates that premise,
  allowing any state machine driven by consumption of the stream to
  diverge arbitrarily. The runtime check guarantees no implementation
  can pollute the set with a value replicas would read differently.
  Considered and rejected: a byte-fixpoint check (re-serialize the
  decoded value, compare bytes) needs no `Eq` bound but cannot catch
  the known lossy class — `Some(None)` serializes to CBOR null and
  decodes as `None`, and re-serializing that `None` is byte-identical,
  so the divergence is invisible in byte space; the value space is
  finer than the byte space, so value equality is the only total
  instrument. The check is send-side only (ingress holds no original to
  compare against), and the bound is `Eq` rather than `PartialEq` by
  design: equality must be an equivalence relation for the check to be
  total and never spurious, which excludes NaN-capable float fields
  from payload types.
- **ciborium pinned exactly (`=0.2.2`)** (Finch): the workspace pins its
  CBOR engine to one exact version. Within a binary, admission and
  ingress run the same compiled deserializer, so their symmetry is exact
  by construction; across a mixed-version fleet, the no-failure
  invariant holds only if every build shares one recursion accounting.
  ciborium documents no accounting contract — the pricing of variant
  scopes, tags, and containers is an implementation detail free to move
  between releases — so the exact pin is what turns "same engine" from
  an assumption into a property of the build. The pin's own manifest
  comment carries the condensed rationale and procedure.
- **The engine bump playbook** (Finch): a ciborium bump is a
  deliberate, fleet-coordinated event, never a routine dependency
  refresh. Procedure: build the workspace at the old and the candidate
  versions and run an accept/reject verdict differential over a seeded
  corpus of payloads at and around the limit. Identical verdicts make a
  pure bump. Differing verdicts mean the accounting moved: the bump
  then ships with a greeting accounting stamp, so mixed fleets fail
  fast at the handshake instead of content-conditionally mid-session.
  A verdict-*strictening* bump additionally requires re-validating
  stored payloads before gossiping — a replica may hold content the new
  accounting rejects, which it would then not be allowed to forward:
  the negotiate-down unsoundness argument, applied across time instead
  of across peers.
- **Vendor evaluation: cbor2 rejected** (Finch): cbor2 (ldclabs) was
  evaluated as an alternative engine — public configurable decode
  recursion limit, active maintenance, deterministic-encode mode — and
  rejected on supply-chain provenance: its implementation is a
  from-scratch rewrite first committed 2026-06-12, roughly two months
  old at evaluation, published atop an older repository's history;
  insufficient provenance and maturity for this dependency's trust
  position. Successor criteria, should ciborium's dormancy ever become
  a liability: understood (source-verified) recursion accounting,
  genuine provenance and maturity, serde round-trip fidelity, and a
  configurable decode limit. Encoder byte-compatibility is explicitly
  not a criterion: payload spelling carries no identity, and snapshots
  re-accept deliberately.
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
- The codec and the admission decode: `src/message.rs` (`PayloadCodec`,
  `Message::try_new`, `EncodeError`).
- The greeting entry and the equality check:
  `src/tree/mirror/streaming/remote/codec/greeting.rs` and
  `src/tree/mirror/streaming/remote/proxy/start.rs`; the wire-format
  ruling's row lives in `design/cbor-legible-wire.md`.
- The batch lifecycle: `Rumors::batch` and `Batch` (rustdoc), pinned in
  `tests/single_peer.rs`.
- The peer-to-peer boundary pins: `tests/payload_depth.rs`.
