# H1 triage: receive-side frame length is trusted for allocation up front

Status: triage only, no implementation. Findings verified against this
worktree (branch `link-transport`, base 94c123e0 plus this branch's H2
enforcement commits) by reading the code as text; every claim cites its
file. Framed per the model of record: an authorized peer already holds
write authority, so this is a conformance bug detector against a *buggy*
counterparty, never a security boundary.

## 1. Current behavior, exactly

Two decoders allocate the peer-declared `u32` frame length before any
payload byte arrives, and no receive-side check compares that length to
the negotiated run budget.

- **Control-stream framing** (`src/tree/mirror/framing.rs:86-93`):
  `FrameRead::frame` reads the 4-byte header, then `vec![0u8; len]`,
  then `read_exact` — documented as "peer-supplied and trusted without a
  cap, so this must only run after the preamble validates the
  counterparty". Consumers: the greeting's version and listing frames
  (`src/tree/mirror/streaming/remote/proxy/start.rs:237,255`) and the
  trailing party hand-off (`src/tree/mirror/party.rs:33`).
- **Data-stream codec** (`src/tree/mirror/streaming/remote/codec/decode/async_io.rs:109-116`):
  `supply()` reads the 4-byte run length, then
  `vec![0; u32::from_be_bytes(header) as usize]`, then `read_exact`.
  The synchronous twin (`src/tree/mirror/streaming/remote/codec/decode.rs:106-113`)
  has the identical shape; its consumers (`decode`, `decode_exact`) are
  exercised only by the codec test suites, so parity there is a
  consistency obligation, not an exposure.
- **The budget is encoder-only.** `RunBudget::admits`
  (`codec/budget.rs:120-125`) has exactly one non-test consumer, the
  outgoing adapter's run accumulation
  (`remote/adapter/encode.rs`, the `admits` flush check inside the
  `Supply` arm). Nothing on any decode path consults the negotiated
  minimum; the decoder deliberately "accepts any batching".
- **Every other variable body is tightly capped before allocation.**
  The query listing allocates `count × 17 ≤ 4,369` bytes from a count
  byte (`async_io.rs:101-107`); the signal is one byte; the preamble is
  a fixed 25-byte frame with no length field of its own
  (`src/tree/mirror/handshake.rs`); the epilogue is one marker byte.

Consequence, stated precisely: per receive stream, a buggy sender's
4-byte header commits the receiver to a single allocation of up to the
`u32` ceiling (~4 GiB) with zero payload bytes yet delivered, one frame
in hand per stream (17 data streams per direction plus the control
stream), held for the caller's whole timeout window if the sender then
stalls. Two mitigations already hold and should be stated honestly:
`vec![0u8; len]` lowers to `alloc_zeroed`, which on the platforms we
run is lazily mapped, so *resident* memory tracks bytes actually
transmitted (the sender pays bandwidth for every resident byte); and
the sender must first pass the preamble. What is unbounded today is the
address-space reservation and the trust itself — the public contract
"yours bounds both the frames you build and the frames built for you"
(`src/peer.rs`, `target_message_size` docs) is true of conforming
senders only.

## 2. Fix design

Two independent mechanisms, matching the two exposures.

### 2a. Budget gate on supply runs (data streams)

At `async_io.rs::supply()`, after the run length header and before any
body allocation, gate the declared body length `len` against the
session's negotiated `RunBudget`:

- **Legal:** `SUPPLY_FRAME_OVERHEAD + len <= budget.bytes()` — the
  mirror image of the encoder's `admits` arithmetic
  (`budget.rs:120-125`), so every frame a conforming encoder flushes
  within budget passes by construction.
- **Also legal (the documented overhang):** a single-record run. The
  encoder's minimum-one-record rule ships a record larger than the
  budget alone in its own frame (`budget.rs` module docs, "exceeding
  the budget by exactly that record's overhang"), so an over-budget
  length is conforming iff the run is exactly one record. Detectable
  from the first 4 body bytes: read the first record header and require
  `LENGTH_HEADER_LEN + first_record_len == len`. Cost: one extra
  4-byte read, paid only on over-budget frames.
- **Otherwise:** a new typed
  `CodecDecodeErrorKind::RunOverBudget { budget: usize, declared: usize }`
  (`codec/error.rs`, beside `QueryOutOfOrder`/`InvalidRun`), surfacing
  through the stream receiver's error route like every codec decode
  failure and terminating the session.

Threading the budget to the decoder: the codec `FrameRead`
(`async_io.rs:21-31`) gains the budget at construction;
`receive_stream` builds it (`streams.rs:392`) from `ReceiverStart`, so
`StreamReceiver::new` (`streams.rs:293-300`) takes the budget and the
proxy's `Session::incoming` (`proxy/state.rs:50-58`) supplies it from
the `Work` it already owns (`work.rs`, field `budget`). The sync twin
(`decode.rs`) takes the same parameter for parity; its only callers are
tests.

Zero-budget consistency: a zero negotiated budget makes every run
over-budget, and the encoder under a zero budget emits exactly
single-record runs, so the gate admits precisely the traffic the
encoder produces (`tests/target_message_size.rs`'s
`zero_target_still_converges` must stay green and becomes the
end-to-end witness).

### 2b. Incremental buffer growth (allocation tracks received bytes)

For any frame that may legally be large, replace
`vec![0; len]` + `read_exact` with a shared chunked reader: allocate
`min(len, CHUNK)` (e.g. 64 KiB), `extend` per chunk as bytes arrive, up
to `len`. Where it applies:

- **Supply runs over the budget** (the single-record overhang, 2a):
  within-budget runs may keep today's single up-front allocation — the
  gate has already bounded it by `budget.bytes()`, so the fast path for
  all conforming traffic is unchanged.
- **Greeting version frame** (`start.rs::receive`, first frame): read
  *before* any budget is negotiated, and its version component has no
  static honest ceiling (a causal version's encoding grows with
  history), so the only sound bound is the `u32` framing ceiling —
  chunked growth makes the allocation track receipt.
- **Greeting listing frame** (second frame): this one *does* have a
  static honest ceiling — Borsh `Vec<(u8, Hash)>` with at most 256
  distinct radices is `4 + 256 × 17 = 4,356` body bytes — so cap the
  declared length there outright and fail `HandshakeDecode` on a larger
  declaration; no chunking needed.
- **Party hand-off frame** (`party.rs:33`): a Borsh `Party`, small in
  honest traffic but without a pinned ceiling; chunked growth.
- **Preamble and epilogue:** fixed 25 bytes and 1 byte, no length
  field, nothing to do.

### 2c. Wire-visible behavior

None, confirmed. The gate and the growth policy consume bytes exactly
as framed today (the overhang check reads the first record header,
which is part of the body it would read anyway); no byte is written
differently, so `tests/gossip_snapshot.rs` and every insta wire
snapshot are untouched. Conforming senders never produce a rejected
frame (2a mirror-arithmetic argument, plus the property pin in §4).
The one snapshot that changes is the codec *error atlas*
(`codec/tests/error_atlas.rs`), a test-side inventory that gains the
new variant — a deliberate test re-accept, not a wire change.

## 3. Cost and risk

- **Performance.** Conforming path: one integer comparison per supply
  frame; allocation unchanged (single up-front `vec` bounded by the
  budget). Over-budget single-record path: chunked growth pays
  amortized-doubling copies on a rare giant-record path that today pays
  one `alloc_zeroed` — acceptable, and it is the path whose allocation
  we specifically want proportional to receipt. Greeting/party frames:
  chunked reads on frames that are tens of bytes to ~4 KiB in honest
  traffic — negligible, and the listing cap removes a chunking case
  entirely.
- **Framing headroom interaction** (`codec/budget.rs`). The gate must
  reuse `SUPPLY_FRAME_OVERHEAD` and `RunBudget::bytes()` rather than
  restating the arithmetic: budgets saturate at `MAX_RUN_BUDGET_BYTES
  = u32::MAX − SUPPLY_FRAME_OVERHEAD` at construction, so
  `SUPPLY_FRAME_OVERHEAD + len` cannot overflow `usize` for any legal
  header value, and every within-budget flush the encoder can produce
  is representable. The risk to manage is *drift between the encoder's
  `admits` and the decoder's gate* — two computations of one quantity —
  pinned by the §4 property test, per the two-ways/one-pin practice.
- **Existing pins on today's behavior.** None found: no test asserts
  the up-front allocation or the decoder's acceptance of over-budget
  runs (the decode suites' `FailingReader` exercises error context
  only, `codec/decode/tests.rs:405-427`), and the window census suite
  measures conforming sessions, which the fix leaves byte-identical.
  So the fix breaks no pinned behavior — which is itself the H1 red
  flag the metering practice says to close *first* (§4, tests 1-2 land
  before the fix).
- **Docs.** `src/peer.rs` (`target_message_size`, "bounds … the frames
  built for you") becomes an enforced claim and should name
  `RunOverBudget` when the fix lands (the audit's H3). Note: the main
  tree carries uncommitted doc edits; peer.rs prose changes should be
  coordinated at cherry-pick time.

## 4. Pinning tests the fix must land with (red first)

1. **Codec acceptance tripwire (red → green).** A two-record run whose
   frame exceeds a small `RunBudget` decodes silently today — commit
   that pin, then flip it to assert `RunOverBudget { budget, declared }`.
   Sibling green case: a single-record run over the same budget decodes
   (the overhang stays legal), and a two-record run exactly at the
   budget decodes.
2. **Allocation-shape tripwire (red → green).** A counting `AsyncRead`
   that records each requested read size, fed a supply header declaring
   `u32::MAX` and then pending forever: today the first post-header
   request is the full declared size — commit that pin, then flip it to
   assert no request exceeds `received + CHUNK`. Same shape for
   `framing::FrameRead` with a huge greeting declaration ending in EOF:
   typed `UnexpectedEof`, requests capped.
3. **Encoder/decoder mirror property.** For arbitrary leaf corpora and
   budgets, every frame the encoder flushes at budget `b` passes the
   decode gate at budget `b` — the anti-drift pin for the two
   `SUPPLY_FRAME_OVERHEAD` computations.
4. **Full-stack session pin (red → green).** The greeting-rewrite
   harness this branch added for H2
   (`proxy/tests/harness.rs::reconcile_rewritten_greetings`) does this
   in one line: rewrite the `target_message_size` word one side
   *receives* down to zero, so the victim negotiates a zero budget
   while the peer batches at its honest minimum. Today the session
   converges (red pin); with the fix, the victim reports
   `RunOverBudget` and both endpoints terminate. (Requires extending
   `GreetingRewrite` with a `target_message_size` constructor — the
   third word, one line.)
5. **Zero-budget interop stays green.** `zero_target_still_converges`
   (`tests/target_message_size.rs`) already witnesses that honestly
   negotiated zero budgets converge; it must not regress, proving the
   gate admits exactly the encoder's zero-budget traffic.

## 5. Effort estimate

Well over the implement-now threshold (~50 lines), so not implemented
here. Estimate: ~150 lines of mechanism (chunked-read helper shared by
`framing.rs` and `async_io.rs`, the budget gate and its single-record
peek, budget threading through `StreamReceiver`/`Session::incoming`,
the sync-twin parity change, the new error variant and its atlas
entry) plus ~200 lines of tests across the five pins, two snapshot
re-accepts confined to test-side atlases, and doc updates in
`framing.rs`, `codec.rs`, `budget.rs`, and `peer.rs`. Roughly one
focused day including the gate runs; the riskiest seam is the
mirror-arithmetic property (item 3), which is also the first thing to
write.
