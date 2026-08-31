# A CBOR-legible wire protocol, and the observation hook

Status: accepted, in implementation; tracked as
[rumors#35](https://github.com/oxidecomputer/rumors/issues/35), and
where that issue and this document differed, the rulings below resolve
toward the issue. Owner: Finch. Origin: design conversation,
2026-08-19. Builds on the version-keying migration's
uniform-CBOR rulings (payloads and the wire's version atom are already
CBOR; each supply-record body is already a two-item CBOR sequence).

## Goal

Every directed stream of a session — data streams and the control stream
alike — parses as a CBOR sequence with standard tag unwrapping, so that a
tool knowing nothing about rumors can unfold a recorded session into a
legible tree, down to exactly the atoms that are honestly rumors-private.
The concrete payoff: a generic debugger for rumors sessions that needs no
knowledge of the internal format or of the application's message types
(which are the application's own CBOR, legible for free), and an
observation hook that feeds it — whose first consumer is a `tracing`
adapter with deep structural inspection of live sessions.

## Why no stream length is needed

The form is RFC 8742 *CBOR sequences*: concatenated data items, no count,
no total length, no terminator. That is exactly the shape of an unbounded
stream, and it degrades gracefully — a truncated capture is a valid-prefix
sequence. (CBOR's indefinite-length containers also need no length up
front but want a closing break code an aborted session never writes;
sequences are the right choice.)

## The layers, current form → CBOR spelling → cost

| Layer | Today | CBOR spelling | Recurring cost |
|---|---|---|---|
| Frame signal | one dense byte (stream × state, 17 × 10 codes) | unsigned int item carrying the same dense code | +1 byte for codes ≥ 24 (most); see the signal ruling below |
| Frame | signal ‖ raw body | array `[signal]` or `[signal, body]` | +1 byte array head |
| Record framing | u32 BE record header | **tag 63** ("embedded CBOR sequence in a byte string"): each record is `63(bstr(tagged version ‖ payload))` | ≈ 0 (tag 2B + bstr head 1–5B vs flat 4B; often equal, −1 for small records) |
| Record body | CBOR bstr(version) ‖ CBOR(payload) — already a sequence | the version atom gains its tag; payload unchanged | +3 bytes (the version tag) |
| Run length | u32 BE | the whole run is `63(bstr(record*))`; its byte-string head is the run length | ≈ 0 (tag 2B + head 1–5B vs flat 4B) |
| Query child listing | raw `(radix ‖ 24-byte hash)*` | map `{radix: hash}` (+3 B/child; ruled over the +2 B/child alternating array for the canonicality coincidence below) | the one hot cost, **measured** at the calibration cells: the whole change moved the calibrated per-disputed-message intercept 35 → 43 B — +3.9% at the design record (207 → 215 B/message), +8% at mid-size records, +14% at minimal `u64` records |
| Greeting | fixed-offset block + frames | one item, `24(bstr(map))`: a text-keyed map (`{"listing": {radix: hash}, "set_len": …, "version": <tagged atom>, "protocol": "rumors", "max_version_bytes": …, "payload_depth_limit": …, "target_message_size": …}`, keys in deterministic order) behind the embedded-item tag, so the control-stream reader gets the item's length up front. The `protocol` entry is the rumors magic — tag 55799 announces only "CBOR", never whose — the version atom rides tagged, dissolving its old bare-canonical spelling, and the `payload_depth_limit` entry is held to exact equality across the pair (`Error::PayloadDepthMismatch`) | few dozen bytes, once per session |
| Preamble magic | 6 raw bytes opening a 25-byte fixed block | one 30-byte self-described item: `55799(["rumors", version: uint, network: bstr, intent: uint])` — the magic survives as the text item, so "not rumors" still diagnoses at the preamble | once per session |
| Party hand-off | one length-framed frame carrying the party's canonical bytes on the control stream | tagged bstr (the party atom, tagged per the table below) | few bytes, once per hand-off |
| Stream open label | epoch byte ‖ index byte | two leading uint items | +0–1 bytes per stream |
| Epilogue marker | one byte (`.`) | the text item `"."` — one byte dearer than a small int, and a generic tool renders the dot | +1 byte, once per session |

Notes on the spellings:

- **Tag 63 is the load-bearing find.** The u32 record header earns its
  keep by giving O(1) record skip and budget pricing independent of
  payload shape (nested CBOR containers are not O(1)-skippable — their
  headers carry counts, not subtree byte lengths; only strings are).
  Tag 63's byte string preserves both properties exactly, while telling a
  generic tool "unwrap me and parse the inside as a sequence." The
  ledger's charge-before-custody ordering is untouched.
- **The listing map's key order coincides with canonicality.** CBOR
  deterministic encoding mandates ascending keys; the wire's canonical
  form mandates strictly ascending radixes. Under the map spelling the
  two disciplines are one discipline — that coincidence is why the map
  is ruled in over the byte-cheaper alternating array.
- **The wire is deterministic-encoding CBOR, as a stated contract**:
  shortest-form headers everywhere, one spelling per value. This is what
  keeps the byte-pinning snapshot discipline meaningful after the change.
- The existing `record_len` pricing pattern (exact header arithmetic,
  pinned against an actual push) generalizes to every priced length
  above.

## Where the opaque boundary stays, and why

Version and party atoms remain opaque byte strings. Their canonical
bit-level codings are the crate's semantics; re-spelling them as CBOR
structure on the wire would be true structural re-encoding — larger,
slower, and a second spelling of the exact thing the tree pins
byte-for-byte. The generic debugger shows "a 37-byte version atom";
rendering the atom's *meaning* is the public skyline iterator's job
(`design/version-skyline-iterator.md`) — a rumors-aware lens over the
rumors-blind skeleton is one `Plateau` walk away, and the two designs
are deliberate complements.

## The bookmark: fully CBOR-parseable on disk

The stored bookmark follows the same property (ruled 2026-08-19): the
whole file parses as CBOR, not just its payload. The v4 form:

```
55799( [ format_version: int (= 4), integrity: bstr, payload: 24(bstr(map)) ] )
```

where the map is Network (16-byte bstr key) → array of clocks, each
clock a tagged byte string (`CLOCK_TAG(bstr(party ‖ version canonical
bytes))`). The integrity hash covers the encoded format-version item
followed by the encoded tag-24 payload item — every array item except
the integrity item itself, a CBOR-visible region rather than an offset
convention, and coverage no weaker than v3's (which hashed the version
too). The fixed opening (the 55799 tag and array head) sits outside
the hash; corrupting it fails shape validation instead, and the
committed corruption sweep proves rejection stays total over every
byte of the file. The raw magic string has no analogue in a
self-described CBOR file and the v4 format (and public API) carries
none.

`FormatError`'s taxonomy is meaning-named with typed carriers (ruled
2026-08-19: variants say what they diagnose, not what stage tripped):
`NotABookmark` holds a `FrameDefect` naming the "not self-described
CBOR / wrong shape" failure, `Record` carries a typed `RecordDefect`
for a payload that is not the record this codec writes,
`VersionMismatch` and `HashMismatch` are unchanged in meaning, and
truncation is rejected at every strict prefix. This is a
format-version bump under the bookmark's own convention.

## Tagged atoms: context-free identity for the opaque byte strings

The opaque atoms gain CBOR tags — selectively — so their identity travels
with them rather than living in protocol position (ruled 2026-08-19). A
tagged atom is self-describing anywhere it appears: a wire capture, a
bookmark, a log line, a pasted hex snippet. That turns the generic
debugger's "37-byte atom" into a dispatch point: a thin *rumors lens*
keyed on nothing but a tag table sends version atoms to the public
skyline iterator and renders them semantically, with zero
protocol-position knowledge. Tags are the bridge between the
rumors-blind skeleton and the rumors-aware lens.

**Placement rule (the crux): tags belong to the transport codecs, never
to the serde impls.** A `Version` whose *serde* implementation emitted
tags would stop being format-agnostic — an application payload
containing a `Version`, serialized to JSON, would break on a
CBOR-specific concept tunneled through serde. Instead the wire and
bookmark codecs (already hand-written at the framing layer) write
`tag ‖ untagged-serde-bytes` and hand-read the tag before delegating
decode. `before`'s serde impls stay untagged and backend-agnostic; the
tags are protocol vocabulary, owned where the protocol is spelled.

Consequences for parsing: no wholesale non-serde parser is required —
only the points that already hand-parse read tags. (Two library facts:
`ciborium::tag`'s `Required`/`Accepted` wrappers do tunnel tags through
serde, but as a ciborium-specific magic-newtype mechanism that would
format-lock `before`'s impls — deliberately not used; and
`ciborium::Value` preserves tags natively, so generic consumers get them
for free.)

Tag / don't-tag:

- **Tagged**: version atoms and party atoms wherever the protocol spells
  them (supply records, the greeting, the party hand-off, the
  bookmark's stored clocks).
  Their contexts are diverse, and their per-instance cost (+3 bytes for
  a first-come-first-served-range tag) lands on payload-dominated paths
  or once-per-session surfaces.
- **Untagged**: hashes inside listings — one context,
  position-determined, and +3 on a 25-byte child is ~12% on the
  dispute-heavy path, the one place bytes are dear. Structure already
  names them. Signals and counts likewise: position suffices.

Tag numbers come from the IANA first-come-first-served range (32768+
per RFC 8949 §9.2 — everything below is standards-action or
specification-required; numbers through 65535 still encode in 3
bytes). The honest path is registering a small contiguous block (FCFS
registration is lightweight); squatting risks a generic tool someday
rendering these atoms with someone else's semantics. The provisional
block is based at 53845 (hex D255, the ASCII bytes "RU" with the FCFS
high bit set), chosen against the live registry's unassigned space:
party, version, then clock (the bookmark's stored party ‖ version
concatenation, tagged whole rather than split), with a small reserve.
Until registration lands, the numbers live in one pinned constant
table, and the capture renderer learns their names (the sanctioned
renderer-vocabulary re-accept class).

## The signal-redundancy ruling

Within one recorded directed stream the signal's stream component is
constant, so signals *could* re-base to state-only codes (≤ 9, always one
CBOR byte). But the dense code's redundant stream component is what the
`Mislabeled` check validates against the transport label — a conformance
bug detector with committed fault-matrix coverage. Ruling: keep the
redundancy and pay the byte (codes ≥ 24 cost two).

## The observation hook

The capture path is a public hook — the `observe` module's three
traits, scoped peer, session, directed stream (ruled 2026-08-19; the
module's rustdoc is the documentation of record, this section the
design rationale):

- **A handler attaches at construction** (`Peer::observe`, and
  `Bootstrap::observe` for the joining session that runs before the
  peer exists — attaching an `Arc` field is what retired the builder's
  `Copy`; its retry affordance survives as `Clone`). For each session
  the peer enters (gossip, and equally bootstrap and retire — a
  capture that skips session kinds is a debugger with blind spots),
  the handler is asked for a **per-session sub-handler**. The
  sub-handler's creation carries what identifies the session — kind
  and protocol — and its lifetime is the session's. It deliberately
  carries no session number: numbering is the observer's own concern,
  counted inside its handler with its own synchronization, exactly
  like message interleaving.
  The role election is deliberately *not* part of that identity: it
  does not exist yet at session start. It is delivered by a dedicated
  notification when decided (after the greetings, before any data
  stream), and never for an equal-versions session, which elects no
  one.
- **The per-session handler yields a per-directed-stream handler** for
  each directed stream as it opens: the control stream's two
  directions at session start, each data stream when first spoken or
  read (a stream the session never uses yields no handler). The
  creation call carries the directed-stream identity — control, or
  data with its speaking role and wire index, plus the direction; the
  handler's lifetime is the stream's.
- **The per-stream handler is invoked once per protocol message, in
  stream order.** Invocation order within one directed stream is that
  stream's message order; **the hook imposes no cross-stream
  synchronization** — streams pump concurrently, and the library does
  not make every observed frame contend on a session-wide point just to
  serialize observation. A consumer that wants the observed
  interleaving reconstructs it without a lock from a session-scoped
  atomic ordinal in its own per-session handler, stamping each message
  as it arrives. A slow handler back-pressures only its own stream;
  the hook must still never block on protocol progress, documented at
  the hook.
- **The per-message payload is the message's wire bytes, not parsed
  values**: `bytes: &[u8]`, **exactly one CBOR item** per invocation —
  a data frame, or one control item (preamble, greeting, hand-off,
  epilogue; the two-byte stream-open label is addressing, not an
  item). Received items are captured off the transport, never
  re-encoded; only complete, accepted items are delivered. Two
  deliberate choices here: borrowing keeps the hot path zero-copy
  (attachment costs one branch per frame unattached, one contiguous
  materialization per frame attached), and bytes-not-types keeps the
  hook *itself* rumors-blind — no protocol type appears in its
  signature, so the hook's API is stable across wire evolution and its
  consumers parse with any CBOR library (or none). Stream identity
  lives at the per-stream handler's creation rather than on every
  message, so the message call carries only what varies per message.
- Attachment is dynamic (`Arc<dyn …>` held as an `Option`), not a
  generic parameter on `Peer`: one branch per frame when unattached,
  and the public type stays unparameterized. An observability surface
  does not warrant monomorphization.
- **Only the CBOR dialect is observable**: `Protocol::V1` sessions are
  not observed, because the frozen legacy wire is not a CBOR sequence
  and cannot honor the one-item contract.

## The two consumers

Both consumers are in scope for the implementation lane (ruled
2026-08-19); each is the dogfood proof, from a different angle, that
the hook's bytes-only signature suffices.

**The snapshot extractor.** A CBOR reflection renderer that supplants
the opaque hexdump wire snapshots with human-readable introspection:
the snapshot suites capture sessions through the public hook and pin a
deterministic rendering of every observed item — structure unfolded
(ints, text, arrays, maps, embedded items), tagged atoms named from
the tag table, opaque byte strings as full hex. The byte-pinning
discipline survives the legibility twice over: because the wire is
deterministic-encoding CBOR as a stated contract, a rendering that
shows every item's complete content is injective on wire bytes — two
different byte streams cannot render identically, with anything the
walk cannot vouch for falling back to explicit exact hex — and the
harness holds the rendered items to the transport capture as a
totality oracle (label plus concatenated items reproduce every wire
byte). So the legible snapshot still pins the wire, while a reviewer
can finally *audit* what moved and why.

**The tracing adapter.** A separate crate (or feature-gated module) so
the core keeps its dependency surface: sessions open `tracing` spans
(session identity as span fields), every observed frame is an event
within its span, and the CBOR structure maps to structured fields —
ints and text directly, maps by key, atoms as lengths-plus-hex. Because
the wire is CBOR all the way down, the adapter is a *generic*
CBOR-to-tracing bridge plus a thin naming layer; deep inspection of
application payloads comes free, since they are the application's own
CBOR.

## The committed contract

- **The rumors-blind render test, stated as a property over arbitrary
  sessions**: randomized peer contents and payloads drive real sessions
  of every kind the capture harness can pair, every directed stream's
  capture is parsed with a generic RFC 8742 walk plus standard tag
  unwrapping (55799, 63, 24; unknown tags tolerated), and the proptest
  asserts everything parses with no bytes outside CBOR items. The
  parser knows nothing of rumors. This is the tamper-evident form of
  the legibility promise; prose claims of legibility are decoration
  without it.
- **Capture validity is a two-property contract, and both properties
  are permanent** (ruled 2026-08-19): the render proptest above keeps
  its transport-level capture forever — it is the proof that
  *external* capture (a third party tapping the wire, no rumors code
  involved) yields valid CBOR — while the observation hook carries its
  own *internal*-capture properties: every hook invocation is exactly
  one CBOR item, and concatenating a stream handler's items
  byte-equals that directed stream's transport capture. The
  whole-stream property implies the per-item property only if the hook
  is bug-free; the pairing tests exactly that implication. Neither
  test supplants the other.
- The full snapshot corpus re-accepts as one deliberate, owner-ruled
  pre-release format change, named in the re-accepting commit.
- Re-derived (never transcribed) readings: the dispute-wire closed form
  and crossover, the window-solve constants, digestshare, decode-alloc
  meters, and the affected wasm32 wire-door pins.

## Cost summary

An M/L codec lane, comparable to the borsh→CBOR wire migration: the
codec layer (signal/frame/streams/greeting/bookmark format) rewritten,
hand-parsed as today (delegating to ciborium is *not* required — the
structural validation and exact pricing stay first-class), plus the hook
threading through the session drivers, plus the re-accept and re-pin
wave. Recurring wire cost, measured at the calibration cells: +8 B per
disputed message end to end (the calibrated intercept moved 35 → 43 B),
which is +3.9% at the design record size and grows toward +14% only for
minimal `u64`-record corpora; essentially zero relative cost on bulk
supply (+3 B per record for the version tag, ≈0 for the framing). A
one-off whole-session check (measured, not an instrument: the
fixed-corpus gossip bench's 5,000-shared / 2,500-per-side-differing
cell, single-byte payloads — the metadata-dominated worst corner)
read +16.4% total session bytes, about 51 KB on a 308 KB session,
falling toward the +3.9% figure as payloads approach the design
record size. The
committed snapshot corpus — toy sessions dominated by their per-session
constants — stands at 9,853 wire B carrying 1,776 digest B (74 digests
at 24 B, 18.0%, measured by `tools/digestshare` over the reflection
renders); against the pre-conversion corpus's 5,273 B that is an 87%
growth, a denominator that overweights the once-per-session surfaces
by design, and the per-message figures above are the hot-path claim.
(The greeting's `payload_depth_limit` entry accounts for 23 B per
greeting of that figure: the corpus measured 8,887 wire B without it.
The remaining growth over the figures previously quoted here predates
that entry; the corpus had moved under the quoted numbers, and both are
re-measured here.)
What does not change: session semantics, the
deadlock-freedom argument (framing-independent; the hook adds
observation, never a protocol dependency), and every validation
property, re-denominated.

## Sequencing

After the version-keying branch merges: same review season, one format
era. Pre-release is the cheap moment for a wire change; once a release
ships, this is a new protocol version by the hard rules. The render
test lands with the codec lane itself; the two consumers (the snapshot
extractor and the tracing adapter) land in the same lane, downstream of
the codec and the hook.

## Decision record

- 2026-08-19 (Finch): the +1 byte per record for CBOR-legible record
  bodies is accepted; the record body is a CBOR *sequence*, deliberately
  not an array-wrapped tuple.
- 2026-08-19 (Finch): pursue full stream legibility — structural CBOR
  for listings and greetings; payloads legible via their containing
  records; the generic-debugger use case is the design's purpose.
- 2026-08-19 (Finch): the on-disk bookmark format becomes fully
  CBOR-parseable under the same property.
- 2026-08-19 (Finch, shape; Claude, refinements): the observation hook —
  Peer-attached handler, per-session sub-handler carrying session
  identity, per-message bytes-level invocation; the tracing adapter is
  the first consumer.
- 2026-08-19 (Finch): the opaque atoms gain CBOR tags, placed in the
  transport codecs (never the serde impls), per the tag/don't-tag table
  above.
- 2026-08-19 (Finch): keep the signal's redundant stream component and
  pay the byte; the `Mislabeled` conformance check keeps its wire-side
  witness.
- 2026-08-19 (Finch): listings are spelled as the map, for the
  canonicality coincidence; cost still measured at the calibration
  cells before pinning.
- 2026-08-19 (Finch): the hook is three-level (peer → session →
  directed stream), per the issue's framing; the library imposes no
  cross-stream ordering cost — a consumer wanting the interleaving
  reconstructs it from its own session-scoped atomic ordinal.
- 2026-08-19 (Finch): the greeting map carries a `protocol` entry as
  the rumors magic, since the self-described CBOR tag alone does not
  name the protocol.
- 2026-08-19 (Finch): the implementer picks a provisional contiguous
  tag block from the FCFS range, pinned in one constant table; the
  IANA registration (or renumbering to the registered block) is
  Finch's, before first release.
- 2026-08-19 (Finch): both consumers are in scope for the
  implementation lane — the CBOR reflection snapshot extractor
  supplants the opaque hexdump wire snapshots, and the tracing adapter
  lands alongside rather than trailing.
- 2026-08-19 (Finch): capture validity is a permanent two-property
  contract — the transport-captured whole-stream render proptest
  (external capture) and the hook's per-item plus concatenation
  differential (internal capture); neither migrates into the other.
- 2026-08-19 (Claude, implementer resolutions within the hook ruling,
  each flagged for review): the role election rides a dedicated
  session-handler notification rather than session-creation identity
  (it does not exist at session start); `Protocol::V1` sessions are
  unobserved (the frozen wire cannot honor the one-item contract);
  and the `Bootstrap` builder's `Copy` gave way to `Clone` so the
  handler can attach before the peer exists.
- 2026-08-19 (Finch): a one-off whole-session bytes measurement
  (before vs after the conversion, mirroring the fixed-corpus gossip
  bench shape) runs as a final bandwidth sanity check; deliberately
  not landed as an instrument.
- 2026-08-19 (Finch): error taxonomies name what they diagnose. The
  bookmark's variants are meaning-named (`NotABookmark`, `Record`),
  and the handshake's failure cases are enumerated as typed public
  variants — `PreambleMalformed` carrying a `PreambleDefect`,
  `PreambleTruncated` carrying byte counts — with no collapse into
  `io::Error`.
- 2026-08-19 (Finch): `PROTOCOL_MAGIC` is V1 vocabulary, exposed only
  under the `protocol-v1` feature; the V2 wire's protocol identity is
  the preamble's `"rumors"` text item and the greeting's `protocol`
  entry. A V2 endpoint still recognizes the legacy bytes internally,
  to diagnose cross-dialect pairings.
- 2026-08-19 (Finch): session numbering is consumer-side, like
  message interleaving — the hook's session identity carries no
  ordinal; an observer wanting "the peer's Nth session" counts inside
  its own handler with its own synchronization.
- 2026-08-20 (Finch): the greeting map gains a `payload_depth_limit`
  entry, held to exact equality across the pair — the payload depth
  limit is a property of the shared set (every replica must hold and
  forward all content), so it is Network-like, never negotiated:
  negotiating down is unsound, since a peer may already hold messages
  deeper than a negotiated bound. A deliberate pre-release wire
  format change, with its snapshot corpus re-accepted in the
  implementing commit; the full design (symmetric enforcement, the
  minted codec, the closure-scoped batch) lives in
  `design/payload-depth-limit.md`.
