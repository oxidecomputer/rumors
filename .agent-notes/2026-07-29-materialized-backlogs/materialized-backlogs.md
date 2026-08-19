# Materialized backlogs: durable causal subscriptions

*Follow-on to the persistent-storage campaign (branch `persistent-storage`
@ 7a832183, PR #8). The campaign's decision record is retained as the
appendix; its rulings continue to govern.*

## Context

`CausalMessages` delivers the message set in causal order by staging each
ingest pass in an in-memory `BTreeMap<(Rank, Key), (Arc<Version>, Arc<T>)>`
— the traversal reads from storage incrementally, but the causal buffer
materializes the whole undelivered delta in memory, payloads included: a
fresh observer replaying a large persistent store holds the entire set
resident. No per-node monoidal cache can fix this (rank order is a global
linearization; any local child ordering is too coarse — Finch's framing),
so the answer is a *secondary index*: materialized backlogs, stored beside
the tree in the same KV store, rank-ordered by construction, arbitrary in
number, independently consumed, resumable across restarts.

## Rulings (Finch, 2026-07-29/30)

1. **`Rank::{encode, decode}` is the canonical wire representation of
   `Rank`**, added to `before` on the **before-hardening branch**. Laws
   proptested beside `Ord`: strict canonical decode, round-trip,
   byte-equality ≡ value equality, **lexicographic order preservation**
   (x < y ⟺ encode(x) <lex encode(y)), injectivity, **prefix-freeness**
   (sanctions mid-key embedding).
   *Status: done by convergence.* The wire form landed on
   before-hardening and this branch has since been rebased onto it, so
   one `crates/before` serves both and the cherry-pick interregnum this
   ruling provided for never opens. The landed mechanism supersedes the
   sketch the ruling was made against — the decoder is in-house end to
   end (dsi-bitstream's decoders are documented non-total on untrusted
   input, so none can serve a strict seam), and the integral code is an
   inverted-polarity Elias delta rather than a length-recursive
   variant; §A names the landed laws, and the rank rustdoc carries the
   derivation.
2. **Rows are pin-table references**: a backlog row holds a `NodeId`; a
   new **unswept** pin table keeps the referenced leaf record alive
   through tree-side redaction until acknowledged. (Both design slices
   independently found that rows pin the *underlying record* — a pin is a
   liveness reference, never a positional edge — so the reification
   duplication feared at ruling time never occurs: zero payload copies.)
3. **`Backlogs<T>: Store<T>` subtrait, KV-only.** Local peers keep the
   in-memory `CausalMessages` (N observers = N volatile backlogs); it
   remains the delivery-semantics oracle.
4. **Explicit advance, at-least-once**: `pop` reads without removing;
   `advance`/`advance_to(cursor)` acknowledges (deletes rows + unpins) in
   bounded transactions. The opaque `Cursor` is the durable resume token;
   a consumer persisting it atomically with its own effect gets
   exactly-once end-to-end.
5. **Pull-model ingest** (derived, not relitigable without new facts): the
   gossip install enumerates no delta (`join` has no observer seam), so
   each backlog ingests from version-bounded walks over published roots,
   exactly like today's observers.

## Design

### A. `before`: the `Rank` wire form (landed)

The canonical order-preserving byte encoding of `Rank` that this design
requires landed in `before` during the before-hardening campaign and is
in the tree this document sits in. This section is the
requirements-of-record the backlog schema consumes, each requirement
pinned by name in `before`'s law suite; the mechanism — the
prefix-ascending bit stream, its integral and fraction layers, and the
alternatives it refutes — lives in the rustdoc of record (the rank
module documentation and `Rank::encode`), and this document
deliberately does not restate it.

- **Order preservation (lex == Ord)**: x < y ⟺ encode(x) <lex
  encode(y), pinned four ways — `rank_lex_encoding_orders_like_ord`
  (generated rank pairs, equal pairs included),
  `rank_lex_encoding_orders_versions` (oracle-generated versions),
  `rank_encoding_exhaustive_small_scope` (every rank in the small
  scope), and the strictly ascending byte-literal battery
  `rank_encoding_known_values`.
- **Embedding safety (prefix-freedom)**: two encodings order correctly
  even mid-key with arbitrary suffixes riding behind them — exactly
  the `rumors:backlog-rows` composite-key contract (§B). Pinned by
  `rank_lex_encoding_is_suffix_safe` (generated suffixes) and
  `rank_encoding_is_suffix_safe_at_the_padding_seam` (the constructed
  padding-boundary worst case). These pins are a ratchet from a
  refuted draft, not decoration: the adversarial review of the wire
  form constructed an embedding witness that caught a real
  prefix-freedom defect before it froze.
- **Strict canonical decode**: self-delimiting from a reader, with
  every non-canonical genre rejected — witnessed genre by genre in
  `rank_decoding_rejects_each_genre`.
- **Byte-literal goldens**: the wire form is frozen by literal byte
  pins (`rank_encoding_known_values`, `Rank::ZERO` included as a
  pinned literal).
- **Transports**: borsh (serialize is `encode_to`, deserialize is the
  strict decode; self-delimiting is what makes the passthrough sound)
  and serde impls both landed alongside the codec.
- **The composite precedent**: `Ranked` — the rank-then-version-bytes
  composite key whose byte order equals its `Ord` — landed with its
  own embedding and rejection pins
  (`ranked_composite_encoding_is_suffix_safe`,
  `ranked_composite_key_is_suffix_safe_at_the_tiebreak_seam`,
  `ranked_decode_rejects_each_genre`); §B's row key applies the same
  embedding discipline with a different fixed tail (the 32-byte
  `Key`).
- **Cost shape**: encoding size is provenance-linear, pinned by
  `rank_encoding_size_is_provenance_linear`.

### B. Schema (`src/store/schema.rs`, house conventions: `rumors:` prefix,
BE keys, panic-on-undecodable)

Four tables:

- `rumors:backlogs` — directory: key = `BacklogId` BE 8; value = borsh
  `BacklogRecord { frontier: Vec<u8> /* Version::encode: the durable
  ingested frontier */, visible: u64 /* last COMPLETED pass epoch */,
  epoch: u64 /* next/current pass; always > visible */, dropping: bool }`.
- `rumors:backlog-names` — key = name bytes (caller-supplied, opaque);
  value = `BacklogId` BE 8. Name frees immediately on drop; a re-create
  mints a fresh never-reused ID, so stray rows can never alias.
- `rumors:backlog-rows` — key = `BacklogId(8) ‖ epoch(8 BE) ‖
  Rank::encode(var, prefix-free) ‖ Key(32)`; value = borsh
  `RowRecord { node: NodeId }`. Composite ordering is sound because
  `Rank::encode` is prefix-free (the suffix-safety laws named in §A):
  the first differing position
  between two rank encodings lies inside both, so the fixed suffix never
  participates cross-rank; equal ranks compare the 32-byte keys —
  exactly the `(Rank, Key)` order of today's staged map, a linear
  extension of causality (equal ranks are never causally ordered; keys
  are injective over leaves).
- `rumors:backlog-pins` — key = `NodeId(8) ‖ BacklogId(8)`, presence
  row: the durable pin. Node-major so GC's liveness probe is the
  three-line `held()` prefix idiom (refcount.rs:96-106). Within one
  backlog a node has at most one row (re-appends collide on the
  identical row key: idempotent puts), so unpin-on-advance is 1:1 with
  row deletion.

`BacklogId(u64)` mints from the shared `IdAllocator` pre-transaction
(names-before-writes; uniqueness is all that matters). Key splitters
follow `held_key`/`split_held_key`.

**Epoch-major visibility**: rows with `epoch > visible` sit beyond the
delivery horizon with zero marking machinery; the pass-completion txn
sets `visible = epoch`, `frontier |= ceiling`, `epoch += 1` — the single
visibility flip. Pop needs no filtering: the least row for a backlog
either has `epoch ≤ visible` (deliverable) or the backlog is quiet
pending completion. Epoch-major delivery is semantically faithful to
today (a pass opens only after staged drains, so delivery is already
pass-major; the causal sieve means cross-epoch rank interleaving occurs
only between concurrent stragglers, whose order is documented-arbitrary)
— and ingest of the next pass may proceed while the consumer drains
visible epochs.

### C. Custody + GC (`src/store/refcount.rs`)

- `backlog_pinned(txn, node)` prefix probe beside `held()`.
- `queue_if_dead` and `reclaim_step`'s stale re-check widen to
  `strong == 0 && !held && !backlog_pinned`. `advance` unpins and
  immediately `queue_if_dead`s in the same transaction (the one new death
  edge).
- **`recover` does not touch the three backlog tables** — module-doc
  sentence mirroring the canonical-root precedent: backlog pins are
  durable consumer state, not process state. `recover`/`vacuum` resume
  any `dropping` backlog's drain.
- **Audit** (refcount/tests.rs): strong recomputation unchanged (pins are
  not strong edges); reachable-closure becomes closure(canonical root) ∪
  nodes named by `backlog-rows`; new clauses — pins ≡ projection of rows
  on (node, backlog) at every committed prefix (written/deleted in the
  same transactions); every row's node decodes as a leaf; every row's
  backlog exists or is dropping; a sealed backlog's frontier dominates
  its sealed rows' versions. The quiesce clause must NOT demand pins
  empty — undrained backlogs legitimately hold pins across quiescence.

### D. Transactions (`src/store/backlog.rs`, all through `write_upkeep`;
manager state on `Shared` beside `dedup`; budgets ≈ RELEASE_BUDGET = 64)

- **Create-or-resume(name)**: allocate `BacklogId` before the txn
  (names-before-writes); txn adopts an existing row or inserts fresh
  (burned IDs read as absent forever). Racing creators serialize; loser
  adopts.
- **Ingest pass**: freeze walk at `start: Excluded(frontier)` over the
  published root + capture ceiling (the causal.rs idiom); per budget of
  ~64 leaves one txn: record id via the new `record_id()` accessor (a
  lookup — published roots are fully persisted, so walk-yielded leaves
  have Stored provenance), put row at `(id, epoch, rank-bytes, key)` +
  pin `(node, id)`; rank encoded ONCE per row (retiring today's
  per-pass, per-observer recompute). Completion txn: `frontier |=
  ceiling`, `visible = epoch`, `epoch += 1` (an empty pass still
  completes — today's empty-ingest checkpoint advance). **Every new pass
  uses a fresh epoch; crashed-pass debris (visible < epoch_row < epoch)
  stays invisible and is swept** at pass-start/vacuum/recover (delete +
  unpin + queue_if_dead) — never resurrected, which is what preserves
  "pre-ingest redactions never fire" across a crash: crashed-pass rows
  were never observed, so a redaction racing the crash must win. Walk
  handles keep records live across each batch (in-process custody;
  recovery cannot run concurrently — ownership contract).
- **Pop**: one read txn — least row in sealed epochs after the cursor
  (prefix probe), fetch the pinned leaf record, decode payload
  (`T: BorshDeserialize`), yield `(Cursor, Key, Arc<Version>, Arc<T>)`.
  Read-only ⇒ trivially cancel-safe and repeatable (at-least-once).
- **Advance_to(cursor)**: bounded txns deleting rows ≤ cursor (the
  opaque cursor IS a row key), each row's pin deleted + `queue_if_dead`
  in the same txn. **Deletion is the delivery cursor** — pop-least
  naturally resumes past deleted rows; no stored cursor row, nothing to
  keep coherent. A crash acknowledges a prefix (at-least-once
  preserved); re-advance is idempotent (absent-key deletes no-op;
  queue_if_dead re-checks).
- **Drop(name)**: set `dropping`; bounded drain of rows + pins (droplist
  drained by write_upkeep piggyback and vacuum); final txn deletes the
  directory row. Durable ⇒ crash resumes at open/vacuum.
- **Crash table** (each sequence bounded single-purpose txns; prefix
  consistency does the rest): partial ingest ⇒ invisible rows, justified
  pins, idempotent re-run; unsealed complete pass ⇒ same; partial advance
  ⇒ acknowledged prefix, no dangling pin or unpinned row; partial drop ⇒
  resumed drain; ambiguous create ⇒ adopt-or-insert retry.

### E. Trait + consumer surface

- `src/tree/backend/backlogs.rs` — `trait Backlogs<T>: Store<T>` (RPITIT
  + Send): `backlog(name, start) -> BacklogState`, `stage(batch)`,
  `seal(id, epoch, ceiling)`, `pop(id, cursor) -> Option<Popped<T>>`,
  `advance(id, cursor)`, `drop_backlog(name)`, `backlogs() ->
  Vec<String>`. The trait is pure storage vocabulary — the walk stays on
  `Store::range`; pass logic lives in the consumer type. `pop` carries
  the crate's first read-side `T: BorshDeserialize` bound, confined to
  backlog methods. Implemented by `KvBackend` only.
- `src/rumors/backlog.rs` — `pub struct Backlog<T, S: Backlogs<T>>`
  adapting `CausalMessages`' three-state owned machine (Ready / Waiting
  on the watch / Ingesting): `async next()`, `try_next()` (TryNext
  parity: Quiet also means fetch-in-flight), `cursor()`, `async
  advance()` / `advance_to(Cursor)`, read-only `Stream` face. `Cursor` is
  an opaque validated newtype over the row key with
  `as_bytes()/from_bytes()` — the durable resume token.
- `Rumors` methods bounded `where S: Backlogs<T>` (no existing bound
  moves): `backlog(name)` / `backlog_since(name, since)` (create-or-
  resume; on resume the stored state is authoritative and `since` is
  ignored — documented loudly), `drop_backlog`, `backlogs()`. Reached via
  the watch-borrow backend-clone idiom — this also closes the
  `Peer::open`-returns-no-handle gap with zero plumbing.
- **Single consumer per backlog, enforced in-process**: `Shared` gains an
  `open_backlogs` set; second open returns Busy; handle Drop releases
  (sync mutex). Cross-process exclusion is the store-ownership contract.
- **Quiescence/teardown**: `Backlog` doesn't count against
  `try_into_peer` (observer parity); after quiescence/retire a live
  handle can still drain and advance sealed rows (durable data) but
  ingest ends with the watch; retiring with undrained backlogs strands
  rows — documented, `drop_backlog` as pre-retire hygiene, a retire-time
  sweep noted as future hardening.
- **Semantics parity vs `CausalMessages`** (documented as a decision
  table): carried — at-least-once + skip-free (now across restarts),
  per-pass (Rank, Key) order, cross-pass causality,
  staged-then-redacted-is-still-delivered (via pins: redaction unlinks
  the record from the tree; the pin holds it; delivery reads it; advance
  unpins; GC collects). Changed — resume token is a `Cursor` (sharper: a
  cleanly-advanced row is never redelivered, which the Version
  checkpoint cannot promise); `T: BorshDeserialize`; delivery pays a KV
  fetch. NOT promised: identical order across consumers with different
  pass boundaries (concurrent items may interleave differently — as
  today across replicas).

### F. Test program (ruling-9 exhaustiveness)

- **Differential oracle**: `Backlog` on `KvBackend<Memory>` vs
  `CausalMessages` on the same peer, drained in lockstep so pass
  boundaries coincide ⇒ byte-identical `(Key, Version)` sequences (the
  replica-independence pin generalized). Proptest over the existing
  causal action alphabet; committed seeds.
- **Crash battery**: every `Memory` committed prefix reopens; resume from
  the stored cursor; laws: delivered ∪ remaining ⊇ final live set;
  cleanly-advanced rows never redeliver; frontier never regresses;
  extended storage audit (C) green at every prefix.
- **Redaction lifecycle**: stage → redact → deliver → advance → quiesce +
  vacuum → record gone. The pin table earns its existence here. Plus the
  crash-boundary twin: a redaction after a COMPLETED ingest still
  delivers (pin holds across restart); a redaction racing a CRASHED pass
  never delivers (debris swept, not resurrected).
- **Multi-backlog independence**: N backlogs at staggered cursors;
  advance/drop on one never perturbs another (a shared record survives
  until its last pin releases).
- **Fault/cancellation batteries**: inject_abort / commit-then-error
  swept over stage/seal/advance positions; futures dropped at every poll
  depth; the advance-interleaves-ingest schedule swept (disjoint epochs,
  but the transactions interleave).
- **Visibility adequacy witness**: a pass whose key order inverts rank
  order (two-party concurrent sends), paused mid-pass ⇒ pop must be
  Quiet; mutation-verify by removing the sealed-epoch filter (string
  swap, restore verified by git diff).
- **Embedding adequacy**: the composite-key ordering law over generated
  rank pairs + keys, with the witness that a naive (non-prefix-free)
  fraction encoding fails it.
- **before laws** (§A): landed with the wire form, gated in `before`.
- **Measurement**: `bench_causal_replay`/`bench_causal_delta` vs a new
  backlog-drain bench, on ox-east-1 under `pset-run` — the memory claim
  (peak per pass = one stage budget, not the set) verified with a
  large-set replay; the KV-fetch delivery cost quantified honestly.

## Deliverable: a design document, not an implementation (Finch,
2026-07-30)

This design is FILED for later implementation as
`design/materialized-backlogs.md` on the `persistent-storage` branch,
joining the design/ corpus (sync-budget, height-erasure,
node-hash-preimage, streaming-wire-deadlock). The document carries: the
context (why no node-cached monoid can causally order leaves — the
secondary-index argument), the five rulings as DECIDED records, the full
design (sections A–F above, re-edited to the design-doc genre: reads as
written today, cites code by path, is never cited FROM code per the
house rule), the implementation phases below as its roadmap section, the
verification program, costs, and open questions. One commit on
persistent-storage; `just gate` (doc linters run on prose) before it.

The implementation phases, recorded in the doc for whoever picks it up:

- **B0** — before: `Rank::{encode, decode}` + laws + byte pins; borsh
  passthrough. *Done by convergence*: the wire form landed on
  before-hardening, and this branch's rebase onto it is the
  convergence the phase provided for — one `crates/before`, no
  cherry-pick leg. §A names the landed pins.
- **B1** — schema + custody: four tables, codecs, splitters, round-trip
  proptests; `pinned` probe, GC widening, recovery/vacuum drains
  (dropping + invisible epochs), audit extension; raw-schema crash
  battery.
- **B2** — the manager + trait: `store/backlog.rs` transactions,
  `Backlogs` trait, `KvBackend` impl (+ `record_id()` accessor),
  `open_backlogs`, droplist; fault battery; conformance-law additions.
- **B3** — consumer + API: `rumors/backlog.rs`, `Rumors` methods, the
  lockstep differential oracle, crash/cancel batteries, adequacy
  witnesses (visibility filter, embedding law), multi-backlog
  independence, redaction lifecycles (both crash-boundary variants).
- **B4** — docs (crate storage section, trait/type docs, the
  CausalMessages-vs-Backlog decision table, cross-pointers,
  `just readme`), benches on ox-east-1 under pset-run, final review
  round.

## Verification (of this plan's deliverable)

The design doc lands gate-clean (prose linters); its content is the
synthesis above, reviewed against both design-fork reports for fidelity;
no code changes. Implementation verification (the batteries, mutation
checks, gates) is specified inside the doc for the future implementer.

## Risks

- Delivery order is pass-boundary-dependent across consumers (as across
  replicas today) — docs must not overpromise; the differential oracle
  controls boundaries explicitly.
- The `T: BorshDeserialize` read bound is new; the not-chosen fallback
  (inline payload copies) must not drift back in silently.
- Rank key size is provenance-linear (pinned by
  `rank_encoding_size_is_provenance_linear`) — a known linear cost,
  stated in the codec docs, never a surprise.
- `BacklogRecord` carries no format version (policy parity with
  `CanonicalRoot`); additive borsh evolution only.
- The parent campaign's pre-merge item is resolved: task #16 (the A1
  interning back-out call) was ruled 2026-07-30 and the interning is
  retired — the appendix decision record has the ruling and its basis.

---

# Appendix: persistent-storage campaign decision record (2026-07-28/29)

Rulings 1-9c and the campaign close-out record are preserved from the
original plan. Summary of standing rulings that govern this plan too:
uniform async API; backend-choice durability above a crash-consistency
floor; store-allocated node IDs + deferred GC; `node_bytes` excludes
payloads; party clock lives in the store (shrinks recorded pre-wire,
clock: None clears, restart never joins two identity records);
record-decode failures panic; eager child maps in handles; exhaustive
failure-path testing (fault + cancellation batteries, mutation-verified
instruments); height erasure deferred; imbl is test-only with its diff
denylisted (upstream defect #161 pinned, mitigated on both branches);
A1 version-interning: RETIRED (owner ruling 2026-07-30, resolving task
#16 as the provisional clause anticipated) — before's #120 made
`Version` Bytes-backed, so a plain `Version::clone` is the same O(1)
refcount bump the `Arc<Version>` handles bought (pinned in before by
`join_all_equal_operands_is_clone_cheap`'s flatness leg), no cross-node
dedup or version pointer identity ever relied on the shared handles,
and the interning's whole remaining economy was a 40-byte handle copy
per yield; the read surfaces yield owned `Version`s again. The Kv
closures-may-rerun clause has its ratchet (task #17, discharged):
`Memory::retrying` runs every transaction closure on the
optimistic-conflict schedule — execute, discard, execute again — and
the backend batteries (the Store differentials, the crash and fault
sweeps, the session census) run over it, so a backend closure leaking
an effect outside its transaction argument diverges under the committed
assertions; the schedule lives as a `Memory` instrumentation face
rather than a wrapper Kv type because a distinct store type would
instantiate the whole height-indexed protocol tower again (the
conformance knobs record the measured cost of each extra
instantiation). The campaign landed as PR #8
(20 commits, tip 7a832183): Store/Backlogs-style capability layering,
commit-lock write protocol, Kv/Memory/conformance::kv, KvBackend with
pending-node custody and recovery, Peer::seed_in/open, read paths
recovered to 1.1-1.4x on delta shapes.
