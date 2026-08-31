# rumors frame-level fuzz target

Status: spec of record, not yet implemented (2026-07-27).

A libFuzzer target that feeds arbitrary bytes into the wire session layer
as if they came from a peer: the fuzzer plays one end of an in-memory
`Link` while a real, fixed local replica drives `Rumors::gossip` on the
other. It is the missing complement to the existing wire suites: they
disrupt *honest* traffic (severed transports, injected I/O faults,
adversarial chunking), while every byte of the traffic itself is produced
by our own encoders. Nothing today performs coverage-guided search over
the space of *dishonest* bytes against the full public session entry.

## 1. Model framing

The model of record is uniform-hash, authenticated-honest-peer: transport
is pre-authenticated and authorized, and an authorized peer already holds
write authority over the set, so hostile-peer regimes are off-model — no
design or pricing argument may rest on adversary economics. The
violation/fail-fast machinery this target exercises is a conformance bug
detector, not a security boundary (`src/link.rs`, "What securing the
transport means", states the division).

What a finding means under that framing: garbage bytes stand in for the
on-model ways a session receives bytes its peer's honest state never
produced — a nonconforming or buggy counterparty implementation, version
skew that slips past the preamble checks, transport corruption the
carrier's integrity layer failed to catch, or a bug in our own encoders.
The contract the protocol already claims for that regime is *typed
fail-fast*: malformed and mismatched input surfaces as a typed error
(`Error` in `src/error.rs` and the diagnostic taxonomy under it), the
local replica is left unchanged, and the link is poisoned. A finding is
therefore any input under which that surface tears: a panic, a hang, work
or memory grossly disproportionate to the input, or a replica left
incoherent after rejection. It is never priced as an "attack"; the fix it
motivates is a conformance fix.

## 2. Entry point and harness shape

### Entry point decision

The target enters at the outermost public session boundary:
`Rumors::gossip` (`src/rumors.rs`) driven over one end of
`rumors::link::memory_with_capacity` (`src/link.rs`), with the fuzzer's
bytes played from the other end. Rationale:

- **Public-surface rule.** Differential and exhaustive suites run against
  the public API, never internal entries; a fuzz target pinned to the
  private frame codec (`src/tree/mirror/streaming/remote/codec/`) would
  keep passing while the public op's wiring drifted around it. Entering
  at `gossip` fuzzes the preamble (`src/tree/mirror/handshake.rs`), the
  V2 greeting decode, the stream-label validation, the signal/frame
  grammar, the leaf-run record framing, the adapter's scope
  reconstruction, the proxy state machine, and the epilogue — as
  actually composed, in one target.
- **The layers below are already point-tested.** The codec, adapter, and
  proxy carry hand-crafted malformed-wire suites
  (`src/tree/mirror/streaming/remote/adapter/tests/malformed.rs`,
  `remote/proxy/tests/`), which pin *known* violation classes one frame
  at a time. Coverage-guided search adds the compositional cases nobody
  thought to craft; it should add them at the boundary where all the
  layers meet.
- **Depth is a seeds problem, not an entry-point problem.** Almost all
  random inputs die at the 25-byte preamble; recorded honest transcripts
  (section 4) start the search past the handshake, and libFuzzer's
  comparison tracing learns the magic and framing from there.

One target, not a family, to start. The wire dialect fuzzed is
`Protocol::V2` only — V1 is a feature-gated frozen oracle (section 5).
A first input byte selects the local fixture from a small table, so one
binary still explores several tree shapes (open question 2 covers
growing the family to bootstrap/retire pairings).

### Harness shape

The honest side is a real replica, not a scripted responder: a
deterministic fixture `Rumors<u64>` built once per input from a fixed
RNG seed (`Peer::seed_rng`, `src/peer.rs`) and a fixture-selected batch
of sends/redactions, configured with `sync_window_floor()` so the
capacity-one orderings the deadlock argument certifies stay exercised.
Determinism matters twice: inputs replay exactly, and seed derivation is
reproducible (the `insta` wire snapshots pin honest sessions
byte-for-byte, so fixture sessions are known-deterministic).

The fuzz side is the raw other end of the memory link pair, driven
directly as transport halves — it never constructs a `Peer`. The link
pair is the crate's own reference instantiation, already validated by
the `conformance` feature's link suite (`src/conformance/link.rs`), so a
finding is attributable to the protocol, never to the harness transport.
That is the reuse the conformance feature offers here: not the checks
themselves, but the certified in-memory fixture they certify.

The whole input is driven on a deterministic manual poll loop (the
`run_to_quiescence` technique in `src/testing.rs`: a flag waker, so a
`Pending` poll that arranged no wake is a deterministic quiescence
witness, with Tokio's cooperative budget disabled around the subject).
The drive has three phases:

1. **Play.** Concurrently: the honest future runs; the harness writes
   the input's control section into its control half, opens the input's
   scripted data streams (in order, up to `STREAM_COUNT`) and writes each
   section; and the harness continuously drains everything incoming
   (control bytes, accepted streams), discarding it. Draining is
   mandatory — an undrained honest write would backpressure into a false
   stall.
2. **Close.** Entered when the transcript is exhausted *or* the play
   phase goes quiescent with transcript remaining (the honest side is
   waiting for bytes the script will never produce in a form it will
   accept — by contract, a session imposes no deadline against a silent
   peer, so mid-transcript quiescence is not a finding; it just means the
   rest of the script is unreachable). The harness shuts down its control
   write half, drops every scripted stream writer, and drops its
   connector and acceptor, so every wait the honest side holds resolves
   as end-of-stream or a transport error. Incoming drains continue.
3. **Resolve.** With the peer's transport fully gone, the honest
   `gossip` future must resolve. Quiescence *here* is a hang finding;
   fuel exhaustion (a poll cap, section 3) is an unbounded-work finding.

### Input framing convention

The framing is a wire contract with the committed seed corpus, exactly
as `crates/before/fuzz/fuzz_targets/fuzz_decode_ops.rs` documents its
own: a change to the framing means regenerating the seeds with it.

```
[ fixture: 1B | n_streams: 1B (capped at STREAM_COUNT)
| control_len: u16 LE | control bytes (len capped at remaining)
| n × ( stream_len: u16 LE | stream bytes (capped at remaining) ) ]
```

- `fixture` indexes the local-replica table modulo its size (empty tree,
  small tree, tree with redactions — final table fixed at
  implementation).
- Every length is capped at the remaining input, the
  `fuzz_decode_ops` discipline, so no prefix can index out of bounds and
  truncated mutants stay valid inputs.
- The control section rides at the head because its first 25 bytes (the
  fixed preamble) gate everything behind them; keeping them at a
  near-fixed offset is what lets mutation and dictionary entries bite.
- Each scripted data stream is opened eagerly at session start. The
  session pairs streams by their first-bytes label (epoch + stream
  index), never by accept order, so eager opening is contract-clean; the
  label bytes are the seeds' job to supply and mutation's job to break.

## 3. Contracts asserted

Every input, in one process, under libFuzzer:

1. **No panic.** The libFuzzer default; includes panics from any
   internal `unreachable!`/`expect` reachable by wire bytes.
2. **Termination with bounded work.** The poll loop carries a fuel cap:
   `MAX_POLLS = BASE_POLLS + POLLS_PER_INPUT_BYTE × input.len()`.
   Exhausting fuel (a self-waking livelock, or work grossly
   superlinear in the input) panics with a distinguished message — a
   crash finding, following the `before_fuzz::under_heap_cap` doctrine
   that a wrong *cost* must fail like a wrong *answer*. Quiescence in
   the resolve phase (a wait wired to no transport event) likewise
   panics as a hang finding. The denominator is input bytes alone: the
   session against a garbage peer produces no caller-visible output to
   co-denominate. Both constants are measured against honest seed
   replays and pinned with stated headroom, never guessed (open
   question 3).
3. **Bounded memory.** The harness lib mirrors
   `crates/before/fuzz/src/lib.rs`: a `peak_alloc` global allocator and
   an `under_heap_cap` wrapper, cap chosen above the default
   `sync_memory_budget` envelope plus fixture overhead and pinned
   (open question 3). A session that materializes transient state
   grossly disproportionate to its window pricing is a crash finding.
4. **Fail-fast surfaces typed errors.** The session's result type
   already forces this statically: `Result<Gossiped, Error<NoBookmark>>`
   with the taxonomy in `src/error.rs` (`Error::Io`,
   `Error::MagicMismatch`, `Error::VersionMismatch`,
   `Error::NetworkMismatch`, `Error::IntentInvalid`,
   `Error::Epilogue`, `Error::Mirror` wrapping
   `MaterializedError`/`Violation` — `UnaskedReply`,
   `UncontainedSupply`, … — and `RemoteError` over the codec's
   `CodecDecodeError`/`CodecDecodeErrorKind`, the stream layer's
   `StreamError`/`AcceptError`/`SendError`, and the adapter's
   `ReplyDecodeError` with `OversizedVersion`, `LeafOutsideScope`, …). What
   the harness asserts dynamically is the *link consequence* the
   contract attaches: on any `Err`, the link end's
   `SessionState::poisoned()` reads true (via `Link::into_parts`).
5. **Replica coherence after rejection** — pinned to what the session
   contract promises today (`Rumors::gossip` docs; `Link`'s "What a
   session promises"), not an invented ideal:
   - Before the session, capture `snapshot.hash()` and
     `snapshot.latest()` (`src/snapshot.rs`).
   - On `Err` other than `Error::Epilogue`: the replica is unchanged —
     post-session root hash and version byte-equal to the captured pair.
     (Of the contract's three "unchanged" exceptions, only `Epilogue`
     is reachable: the fixture neither bootstraps, donates, nor carries
     a bookmark.)
   - On `Ok` or `Err(Epilogue)` (both mean locally committed): the
     local version never regresses — post-session `latest()` is `>=`
     the captured version. Content retention is deliberately *not*
     asserted: a completed session may honor peer-declared deletions,
     so "no message lost" is not an invariant of this pairing.
   - Serviceability probe: after a committed outcome, gossip the
     post-state replica against a pristine honest copy over a fresh
     link and require `Ok` plus convergence. Committed-with-garbage is
     rare, so the probe's cost is negligible, and it is the strongest
     public-API statement that "the tree the session left behind is a
     tree the protocol can still operate on." On `Err`-unchanged the
     probe is redundant (hash equality already proved the state) and is
     skipped.

## 4. Seed corpus

Seeds are recorded honest transcripts re-framed into the input
convention: for each fixture pairing, run a real honest session over
capture links (the interposition pattern of
`tests/common/gossip_snapshot.rs`, whose `LinkCapture` — exported via
`rumors::testing`, `test-internals` feature — already yields exactly the
sections the framing wants: the peer's control bytes plus each opened
data stream's bytes), then emit `fixture`, `n_streams`, and the
length-prefixed sections. The committed `tests/snapshots/*.snap` wire
snapshots are *not* parsed for seeds — they are a human-readable
rendering — but they are what proves the derivation deterministic, so
the corpus can be gated byte-identically.

Precedent mirrored from `crates/before/fuzz` (its README documents the
scheme): seeds derive from the live API via
`cargo run -p rumors --features test-internals --example fuzz_seeds`
(the feature rides the derivation invocation only — the fuzz target
itself needs no internal features); a committed integration test
(`tests/fuzz_seeds.rs` shape) re-derives and byte-compares the seed
directory so it cannot drift; runs name `seeds/<target>` as the
extra corpus directory so the committed seeds stay pristine while
discoveries land in the git-ignored `corpus/`.

Mutation-friendliness: honest transcripts give the mutator valid
preambles, greetings, labels, and frames to splice; the capped length
prefixes mean truncation and crossover always yield well-formed harness
inputs; single-byte mutations inside a section reach the codec's
"reject immediately after the signal byte" paths (the signal encodes
`state × 17 + stream` with values 170–255 reserved —
`src/tree/mirror/streaming/remote.rs` documents the grammar — so bit
flips there are productive). One seed per fixture plus one truncated
and one label-perturbed variant is the committed minimum; the corpus
grows organically thereafter.

## 5. What this deliberately does not cover

- **Fault injection on honest traffic.** `tests/disruption.rs` with
  `tests/common/fault.rs` (byte-budgeted `Fuse`/`Cut` severing, intra-
  and inter-process, real TCP included) and the `rumors::testing`
  transport adversity (`IoPlan`/`IoFault`: chunking, delays,
  hold-until-flush, typed injected failures at every surface) already
  sweep sessions whose *content* is honest while the transport
  misbehaves, and assert the honest-error classification and global
  party invariants. This target holds the transport honest and makes
  the content hostile; the two sweeps compose, they do not overlap.
- **Link-contract conformance of transports.** The `conformance`
  feature validates caller-built `Link` implementations against the
  contract in `src/link.rs`. The fuzz harness *assumes* that contract
  (it runs on the validated in-memory reference); it never probes it.
- **Honest-peer behavior.** Convergence, redaction, bookmark, and
  causality properties of well-formed sessions belong to the
  differential and property suites (`tests/common`, the V1-vs-V2 oracle
  mirroring in `src/tree/mirror/`); a fuzz input that happens to encode
  an honest session asserts only section 3's contracts.
- **The V1 wire.** `Protocol::V1` is feature-gated off by default and
  frozen as the streaming protocol's behavioral oracle; hardening spend
  goes to the dialect deployments speak (open question 5).
- **Off-model regimes.** Nothing here prices resistance to a motivated
  adversary — spoofing, replay, confidentiality, and tampering are the
  transport's obligations by the model of record, and no finding or
  fix from this target may be argued on adversary economics.
- **Session drivers above `gossip`.** `gossip_when`, multi-session link
  reuse, bookmark persistence, and the bootstrap/retire pairings are
  out of the first target's scope (question 2 stages the pairings in).

## 6. Integration

- **Location:** a detached fuzz workspace at `fuzz/` beside `src/`,
  mirroring `crates/before/fuzz`: empty `[workspace]` table so the
  stable-toolchain gate never builds it, `cargo-fuzz` package metadata,
  `libfuzzer-sys`, `peak_alloc`, and `rumors` as a path dependency with
  default features. Package `rumors-fuzz`; target `fuzz_session`;
  harness lib in `fuzz/src/lib.rs` (poll-fuel driver + heap cap);
  README documenting the framing contract and run lines, as before's
  does.
- **Justfile wiring, mirroring the before recipes:** `fuzz-build`
  gains (or is joined by a sibling for) the rumors workspace built
  under `tools/memwatch` with the nightly toolchain; the `fuzz` smoke
  recipe gains one line running `fuzz_session` for `fuzz_smoke_secs`
  (20s default) with `corpus/fuzz_session seeds/fuzz_session` named in
  that order. `ci` keeps building fuzz targets without running them;
  `all` inherits the smoke through the existing `(fuzz
  fuzz_smoke_secs)` dependency. The gate is untouched.
- **Landing obligations:** committed seed corpus plus its derivation
  example and byte-identity gate test; doc comments on the target, the
  harness lib, and every gate test stating the invariant in English
  (testdoc conventions); the framing documented in the target's module
  doc as the seed contract; measured-and-pinned fuel and heap-cap
  constants with their derivation noted at the constant; any crash
  finding fixed lands its minimized input as a committed seed and, where
  the class generalizes, a point test in the owning layer's malformed
  suite.

## 7. Open questions for the owner

1. **Second, codec-level target?** A `fuzz_frame` target decoding a
   single frame via the internal codec would iterate faster per exec
   but pins an internal entry, which the public-surface ruling forbids
   except as a deliberate documented decision. *Recommendation:* no —
   ship the session-level target; revisit only if coverage reports show
   the codec's decode arms unreached through the public entry, and then
   as an explicit internal-entry decision recorded at the target.
2. **Fixture family scope.** Gossip/remain only, or also the
   bootstrap-provider and retire-absorber pairings (distinct intent
   paths, donation windows)? *Recommendation:* land gossip first;
   add a bootstrap-provider fixture as a follow-up family member once
   the harness is proven — it reuses the framing unchanged, only the
   honest driver differs.
3. **Fuel and heap-cap constants.** `BASE_POLLS`,
   `POLLS_PER_INPUT_BYTE`, and the heap cap must come from measuring
   honest seed replays and the default window envelope, not from this
   spec. *Recommendation:* measure at implementation, set floors with
   ~10× headroom, pin with the derivation noted at the constant.
4. **Where the budgeted poll driver lives.** Reuse
   `rumors::testing::run_to_quiescence` (fixed 1M-poll budget,
   `test-internals` feature) or a harness-local loop with the
   input-proportional fuel? *Recommendation:* harness-local — the fuel
   policy is fuzz-specific, the loop is ~30 lines over public `Future`
   API, and the target then needs no internal features at all.
5. **V1 exclusion.** Confirm the frozen V1 oracle wire stays out of
   scope permanently, not just initially. *Recommendation:* exclude
   permanently; V1's value is as a behavioral oracle for honest
   sessions, and hardening a feature-gated dialect nobody deploys buys
   nothing.
6. **Serviceability probe on committed outcomes.** Keep the
   second-session probe (section 3, contract 5), or is version
   dominance enough? *Recommendation:* keep it — it is the only total,
   public-API check that a committed post-garbage state remains
   operable, and it runs only on the rare committed outcomes.

## Addendum (2026-07-27): the unbounded-allocation objection, answered by the denominator

Raised at spec review: a protocol-valid arbitrary counterparty can
induce large allocation (buffers; a large tree), and long valid
streams necessarily describe large trees — is bounded-cost fuzzing
coherent at all? Resolution, in four parts, now binding on the
implementation:

1. **No expansion mechanisms exist.** The wire has no backreferences,
   no compression, no repeat forms; hash references never materialize
   subtrees (we hold them already or fail fast). Every materialized
   node is paid for by received wire bytes, so allocation is linear in
   what the peer actually sent, and a fuzz input is finite.
2. **Random-valid-stream statistics are critical, not explosive.** The
   version grammar's one-bit flag makes random branching critical
   Galton–Watson (offspring 0 or 2, mean 1): a random valid stream is
   almost surely a finite tree, heavy-tailed (P(size > N) ~ N^(-1/2)),
   and in every case bounded by the input's own length. Conditioning
   on a long valid stream yields a big tree the peer bought
   byte-by-byte.
3. **The real genre is declared-size pre-allocation, promoted to a
   named contract.** The only door to allocation ahead of payment is a
   length-prefixed frame whose declared size is reserved before its
   bytes arrive. Contract (first-class, alongside section 3's):
   **allocation tracks received bytes, never declared sizes** — a
   reservation keyed to a declared length that outruns receipt is a
   finding regardless of whether the stream would eventually have been
   valid.
4. **Caps are denominated per received byte against the documented
   session cost claim** (measured honest-replay headroom, per open
   question 3) — so legitimately expensive valid traffic sits under
   the cap by construction, and a trip is either an amplification bug
   or a wrong complexity claim, both findings. Conceded scope limit:
   cumulative replica growth across an unbounded session is
   legitimate; the harness bounds only per-finite-input totals, which
   is a fuzzer's proper scope.

Consequence for the harness: the finding taxonomy in section 1 gains
"allocation ahead of receipt" as its own class, and the heap cap
assertion runs continuously during the play phase (not only at
resolve), so a declared-size reservation trips at the moment of
over-reservation with the offending frame in hand.
