# Branch review: link-transport against main

**Date**: 2026-07-23. **Scope**: every change on `link-transport` vs `main`
(merge-base fab65c2c, 106 commits, 226 files, ~25k insertions), reviewed at
HEAD 22b61c02. **Method**: 21 independent review passes (16 area reviews, a
gate run, a mechanical hard-rule sweep, and three negative-space passes over
interaction seams, shared assumptions, and coverage completeness); every
correctness-bearing finding was then adversarially re-verified by one or two
independent skeptics instructed to refute it against the code as it exists on
this branch. 143 raw findings → 39 confirmed (14 major, 21 minor, 4 nit),
1 refuted, 103 documentation/style observations. Line numbers cite the branch
at 22b61c02.

**Gate status**: `just gate` fully green — 866/866 nextest tests (2
self-skipped), doctests, doclint, testdoc, readme-check, clippy `-D warnings`,
and both rustdoc passes clean. Total test wall time 39 s; slowest tests are
`before::bit_flip_rejects_or_decodes_canonically` (23.8 s) and
`capacity_stress_witness_requires_inter_level_fan` (22.6 s) — within bounds,
worth watching.

**Verdict in one paragraph**: the branch is in strong shape. No confirmed
finding is a wrong-behavior bug in a shipped code path; the gate is green; the
deadlock-freedom, batching, negotiation, hashing, and budget-derivation
machinery all survived adversarial reading, and the constants were recomputed
independently and match across code, rustdoc, design docs, and the committed
table. The majors are all of one genre the repo itself treats as first-class:
**claims stated stronger than what is established** — a contract whose clauses
are not jointly sufficient, a conformance suite that checks less (and in one
place more) than the contract it certifies, a memory-model derivation resting
a many-leaf bound on a pairwise lemma, and several doc surfaces that still
describe pre-negotiation or pre-campaign semantics. There is also a residue of
hard-rule violations (ghost references, design-doc citations from code)
concentrated in files that landed after the branch's own prose sweep — the
merge-seam genre.

Numbered findings (R1…) for triage. Severity: **major** = misleading
specification, broken invariant, or missing critical coverage; **minor** =
real but contained; **nit** = taste. "CONFIRMED" means one or two independent
skeptics re-derived the finding from the code; unlabeled entries in §5–§8 are
single-pass documentation/style observations (spot-checked, not adversarially
verified).

---

## 1. Major findings (all CONFIRMED)

### R1 — The Link contract omits the control stream's obligations; the clauses are not jointly sufficient
`src/link.rs:24-50`

The contract section scopes its five clauses to implementations of
`Connector` and `Acceptor`, but a caller building a `Link` also supplies the
control halves (CR/CW), and the deadlock-freedom argument rests on
control-stream properties stated nowhere as obligations: (a) full-duplex
direction independence — a side's control read must progress while its own
control write is blocked; (b) liveness at any positive receiver-paced buffer
capacity. These are load-bearing on this branch: the V2 greeting and the
epilogue are exchanged as concurrent write+read precisely because a greeting
can outgrow any window (`src/tree/mirror/alternating/backend/remote.rs:184-194`
says in as many words that "the greeting fits in the window" is a size
assumption, not a contract; `tests/handshake_liveness.rs` proves greetings
overflow a one-byte window many-fold). Failure scenario: an implementer
follows every stated clause faithfully but builds the control pair over a
direction-coupled carrier (a mutex-guarded duplex, an alternating
request/response transport); the first session whose greeting outgrows the
control buffer deadlocks with both sides blocked in write. The conformance
suite does not catch it either: `check_control`
(`src/conformance/link.rs:135-185`) probes the two directions sequentially
with 30-byte payloads and never writes both directions concurrently, and
`check_sessions`' fixture greeting is small. (Ordered reliable per-direction
delivery, the third load-bearing property, is present descriptively at
`src/link.rs:11-13`; the two above are absent entirely.)

Fix direction: add a control-stream clause to the contract list, and add a
symmetric concurrent-write probe to the conformance suite (a greeting-sized
payload written by both sides simultaneously over a small-capacity control
pair).

### R2 — The conformance suite never exercises the concurrency clause, and its "cannot see" section does not admit it
`src/conformance/link.rs:29`

No probe ever requires more than 2 concurrently open data streams:
`probe_independence` opens its 16 live streams strictly sequentially while the
receiver eagerly drains (only the stalled stream is held across the probe);
`probe_cancellation` holds 2; `check_sessions`' concurrent demand is
unmeasured. The contract (`src/link.rs:41-44`, and `STREAM_COUNT`'s doc at
:73-78) requires admitting 17 concurrent streams per direction and forbids
serializing an open behind unrelated stream progress. A link capping
concurrency anywhere in 3..17 (e.g. QUIC with max concurrent uni streams = 16)
passes the entire suite while violating the contract, and the streaming
protocol really does lazily hold one stream per reply height, so a deep
session can reform the wait cycle in production. Compounding it, the module
doc asserts "a black box bounds three contract clauses" (buffering,
cancellation, failure classification), implying the others are validated.

Fix direction: a probe that holds all `STREAM_COUNT` streams open (some
blocked on backpressure) while requiring the last connect plus a byte on the
last stream to complete; or, minimally, add the concurrency clause to the
"What the suite cannot see" list.

### R3 — The cancellation probe rejects the contract's "or fail it cleanly" arm; the suite is stronger than the spec
`src/conformance/link.rs:501-514`, contract at `src/link.rs:48-50` and :118-120

The contract states a disjunction — a stream mid-delivery across a dropped
accept must "surface from a later `accept` call **or fail it cleanly**" — and
the probe admits only the deliver arm: every realizable observable of "fail
cleanly" (accept `Err`, stream never surfacing after a reset-on-drop, `Rx`
read error, sender-visible reset) either hangs the collecting loop or panics
an `expect`, and the module doc frames every panic as a violated clause.
There is also no defined observable for "fail cleanly" anywhere — accept and
connect errors are restricted to transport failure (`src/link.rs:111-114`) —
so the arm has no legal channel to report through. Either delete the arm from
the contract (making the suite exact) or define its observable and teach the
probe to accept it. As it stands, a QUIC STOP_SENDING-style implementation
following the documented disjunction is reported as nonconforming.

### R4 — Version-bound pricing is stated at derived strength; the pairwise lemma does not cover many-leaf interior joins (and floors are meets)
`src/tree/mirror/streaming/window.rs:233-242` (rustdoc) and :254-255 (inline comment)

`from_budget` prices every held reference at
`version_bound = 2 × (local_max + remote_max)` and justifies it as a
consequence of `before`'s pinned subadditivity lemma. Three independent
passes converged on the same gap, and both skeptics re-derived it: the pinned
lemma (`crates/before/src/version/tests.rs:326-351`, and the arbitrary-pair
variant at ~1106) is strictly pairwise — `|enc(a∨b)| ≤ |enc(a)| + |enc(b)|` —
while an interior node's ceiling is a join over *all* subtree leaves
(`src/tree/typed/untyped.rs:467-490`) and the exchanged bounds are maxima over
*leaf* encodings only. Summed over k leaves the lemma yields k·max, not
`local_max + remote_max`; the rustdoc presents a non sequitur as a derivation.
Sharpening from the interaction pass: a branch **floor** is assembled by
meets, not joins (`untyped.rs:500-527`, `batch &= child.floor()`), and no
meet-size lemma is pinned anywhere — so for floors even the pairwise step is
uncited. The design record is honest about all of this
(design/sync-budget.md §2.2: "the pairwise bound is the *priced* claim",
guarded empirically by the census lemma-slack pin,
`tests/window_census.rs::version_bounds_stay_inside_the_priced_pair_bound`,
with a recorded fallback); the code that owns the number claims more.

Impact bound (why this is a docs-severity major, not a behavioral one): the
shipped `Local` backend prices a node at one pointer regardless of
`version_bound`, so no in-tree path can breach the envelope; the exposure is a
future materializing backend, for which the trait's own docs say an
underpriced node is the one input that breaches *memory* rather than latency.
Per the statement-faithfulness rule, a claim stronger than proven is a
wrong-grade finding regardless.

Fix direction: re-denominate both the rustdoc paragraph and the :254-255
comment at true strength — pairwise lemma pinned; interior many-leaf bounds
*priced* at the pair bound, ceiling/floor as join/meet named honestly; the
census lemma-slack pin named inline as the empirical guard.

### R5 — b05-uniformity-envelope's context header describes an API that no longer exists and denies an adoption that happened
`design/b05-uniformity-envelope.md:3-17`; duplicated at `design/b05-envelope-sim.py:11`

The header (dated 2026-07-22) states "this branch's `window.rs` prices a
node-denominated budget (`Peer::max_in_flight_nodes`), and no charge formula
below is this branch's mechanism … adoption was declined."
`max_in_flight_nodes` exists nowhere on the branch (the sync-budget phases
replaced it the same day the note was imported); the knob is the
byte-denominated `Peer::sync_memory_budget`, and window.rs implements
precisely this note's §7 integer-quantile family (`bernstein`,
`small_mean_quantile`, `jointly_occupied`, `children_quantile`,
window.rs:423-533, in the pair-based A·B adaptation sync-budget §1.2 records).
The contradiction is load-bearing: sync-budget.md:10-12 directs readers to
this very header as the map of what was and was not adopted. Also in the same
block: `design/single-socket.md` is said to "live on the campaign branch, not
here," but the file exists on this branch (and on main). Fix as a dated
amendment to the header.

### R6 — The crate-doc wire-stability policy is contradicted by this branch's own wire changes
`src/lib.rs:219-222`

The crate docs promise "a wire change introduces a new protocol version
rather than silently changing an existing one." This branch changes V2's wire
in place (5-byte version frame → 29-byte greeting prefix + listing frame,
byte-budgeted supply runs, the 0x2e epilogue marker) and changes V1's pinned
bytes too (the lever-E hash change moves the node-hash bytes in
`gossip_snapshot__v1_one_sided_transfer.snap`), while `src/protocol.rs` keeps
V1=1/V2=2. The no-bump choice is deliberate and recorded
(design/streaming-latency-serialization.md ~:1356-1366: nothing has deployed
either version; after first deployment a bump becomes mandatory) — but the
sentence pre-exists on main, the branch creates the contradiction, and the
carve-out must live in lib.rs itself since code prose may not cite the design
doc. Corollary worth one sentence while there: intra-V2 skew (pre-change peer
meets post-change peer) hangs at the greeting rather than failing typed — the
:208-210 "rejected before any peer-declared frame length is trusted" sentence
is literally true only for a *differing version byte*.

Fix direction: scope the claim ("until the first deployed release, a dialect
may change in place; from first deployment onward…").

### R7 — tests/target_message_size.rs documents the opposite of the landed semantics
`tests/target_message_size.rs:3-5`, :20-21, :61-62

The module doc says peers with different targets interoperate "because run
sizing is not wire-visible"; `mixed_targets_interoperate`'s doc calls run
sizing "a local encoder choice, not a negotiated wire parameter"; and
`diverged_pair`'s doc says "both encoders exercise their own setting." All
three are false on this branch: the greeting carries `target_message_size`
(`src/tree/mirror/streaming/remote/proxy/start.rs:225` writes, :258 reads),
the session runs at the exchanged minimum (start.rs:204-206), the
`asymmetric_message_targets_unbatch_the_run` snapshot pins exactly that, and
`src/peer.rs:420-427` documents it. In the (0, DEFAULT) cell the minimum is 0,
so both encoders run unbatched and neither exercises "its own" setting — the
stated coverage is weaker than what the body actually tests. Commit 22b61c02
swept this exact stale claim from peer.rs and missed this file — the
merge-seam genre. Tests pass; the file's spec is inverted.

### R8 — The FAN-channel comment claims "this buffer adds no memory term of its own"; the claim is underived and false in the worst case
`src/tree/mirror/streaming/remote/adapter/decode.rs:115-120`

The reader/assembler channel grew from capacity 1 (main) to FAN=256,
buffering fully **decoded** leaf nodes (owned `Version` + full `Message`
payload, built at :195). The comment says the scope's fan is "already
charged" by the session memory model — but the window prices node
*references* via `Backend::node_bytes`, which deliberately excludes leaf
payloads because "the wire already bounds them through target_message_size"
(`backend.rs:45-47`) — a per-encoded-run bound, while this channel can hold up
to 256 decoded records spanning many frames whenever the assembling backend
lags. Worst case: ~1.1 MB records × 256 ≈ 285 MB of uncharged decoded payload
— **more than the entire ~271 MB default session budget**. Three surfaces
disagree: this comment (no term), `peer.rs:420-423` ("one run's bytes per
message while decoding leaves one at a time"), and `adapter.rs:57-59` (which
honestly lists "the buffered fan of decoded leaves" as a memory term). For
`Local` and small messages the term is negligible; the finding is that the
cost claim is stronger than established.

Fix direction: bound the channel by bytes (or shrink it) and derive the bound,
or restate the comment and the peer.rs Memory section honestly (up to
FAN × max-record bytes between reader and assembler) and say why the envelope
is acceptable.

### R9 — "Replica unchanged on Err, with two qualified exceptions" omits a third: post-commit `Error::Bookmark` when absorbing a retiree
`src/rumors.rs:304-310`; mechanism at `src/peer/gossip.rs:780-819`

`gossip_inner` commits the merged tree *and an absorbed retiring peer's
party* in `send_if_modified` (:780), and only then runs `bookmark_update`
(:817); on failure the session returns `Err(Error::Bookmark)` with the replica
already changed — reconciled messages plus the absorbed identity, un-persisted
(the inline comment at :809-816 says exactly this). The public contract
enumerates exactly two exceptions (Epilogue, bootstrap-fork leak), and
`src/link.rs:162-165` says "'Unchanged' has two qualified exceptions, stated
where they arise" — the third is stated nowhere, including `Error::Bookmark`'s
own docs (`src/error.rs:131-133`). A caller seeing `Err(Bookmark)` after
gossiping with a retiring counterparty believes the replica unchanged and does
not know it now holds the retiree's identity region undurably (a crash then
strands the region). Reachable only with a fallible `Bookmark` impl
(`NoBookmark`'s error is uninhabited), but `Bookmark` is a public trait.
Secondary: the `gossip_when` bullet at rumors.rs:340-344 ("unchanged, unless …
Epilogue") also contradicts the linked session contract, though :353-355's
deference to the link contract partially mitigates the fork-leak half.

### R10 — Rustdoc cites `design/height-erasure.md`: the one hard-rule-2 violation in Rust code
`src/conformance/backend/tests.rs:53-56`

The `Materializing` struct doc reads "every distinct backend type
instantiates the whole height-indexed protocol tower (see
`design/height-erasure.md`), so the honest and lying variants must share one
type." Commit a3da46c4 recorded "design documents are no longer cited from
code prose anywhere"; this file landed after that sweep and regressed it. A
repo-wide grep confirms it is the only `design/` citation left in `.rs` code.
The constraint is already stated inline in the same sentence, so the fix is
deleting the parenthetical. (Non-Rust code carries three more: R40.)

---

## 2. Minor findings (all CONFIRMED)

### R11 — Cancellation probe's teeth are timing-gated on real transports; the limitation is not admitted
`src/conformance/link.rs:484-504`

Deterministic for the in-memory link (connect completes only after the
announce lands, so the first poll dequeues). On a networked link the oneshot
fires when `connect()` returns on the *sender*, up to an RTT before the stream
is acceptable at the far acceptor; the three poll-once-drop cycles then run
back-to-back in microseconds, all observe `Pending`, and a lossy
dequeue-then-await acceptor passes on exactly the transports callers run the
suite for. The "cannot see" section admits only the converse case. Fix:
interleave the poll-drop cycles with retries until at least one `Ready` has
been observed, or extend the "cannot see" bullet.

### R12 — `PRICED_HEADER` and the ledger race across tests under plain `cargo test`; lying-by-default when unset
`src/conformance/backend/tests.rs:62`, stores at :214/:227

Sound under nextest (process-per-test, which the module doc's premise names),
but under plain `cargo test` the two `Materializing` tests run as threads in
one process and whichever stores second retroactively reprices the other's
in-flight session: the `should_panic` test can fail to panic, the conforming
test can spuriously report an underpriced node, and `take_violations` drains
cross-test. The ledger statics have the same exposure (pre-existing genre; the
knob doubles the shared state). Also: the default of 0 means a future test
constructing `Materializing` without storing first silently gets the maximally
lying backend — `ROW_HEADER` is the safe default, with the underpricing test
opting into 0. Fix: restate the one-check-per-process premise at the knob, or
serialize the two tests on a static lock; flip the default.

### R13 — `check_sessions`' oracle is length-only, and its "opens several data streams" sizing claim is unguarded
`src/conformance/link.rs:570-579`, `SESSION_PAYLOADS` doc at :91-94

Convergence is asserted as `snapshot().len() == 96` per side, never content
equality (set equality is equally cheap and strictly stronger — `Snapshot` is
`Eq`). And nothing counts connects, so a future protocol change that lets the
48/48 divergence ride the control stream alone would silently stop exercising
the connector/acceptor in-session while still passing. The suite's own authors
guarded the reordering decorator against exactly this degeneration with a
fired-counter assert (`src/conformance/link/tests.rs:142-146`); the same
discipline is missing here.

### R14 — Bulk-assemble equivalence proptests never compare `version_bytes`
`src/tree/mirror/streaming/backend/local/tests.rs:120-123`, :158-159

`full_height_roundtrip_matches_default` asserts hash/len/ceiling/floor
between `Local`'s bulk `from_sorted_leaves` and the default per-level fold;
`multi_run_grouping_matches_default` asserts hash/len — neither asserts
`version_bytes`, the one aggregate this branch adds, and nothing else covers
the bulk path's propagation (the canonical-construction proptest compares
serialized bytes, and `version_bytes` is not serialized). A future
mis-propagation (say, first child's value instead of max) would pass every
existing test while deflating the greeting's wire bound for every
wire-assembled tree — which the memory budget then prices with. The current
implementation is correct by inspection. Fix: one `prop_assert_eq!` per
property.

### R15 — The backend conformance suite never exercises a backend's overridden `leaves`/`assemble` bulk paths
`src/conformance/backend.rs:235`

`Charged<B>` implements only `node_bytes`/`parent`/`children` and inherits
the trait's default `leaves` (per-level explode) and `assemble` (per-level
fold), so the wrapped backend's own overrides — the paths production actually
runs via the remote adapter (`encode.rs:190`, `decode.rs:221`) — are never
priced, aggregate-checked, or measured. The materialized-materialized session
inside `run()` never invokes the leaves/assemble seam at all; only `corpus()`
does, via the generic fold. A third-party backend whose bulk assemble
over-holds memory or mis-propagates aggregates passes conformance. Contained
today (boundary is crate-internal; `Local` has its own equivalence proptests,
modulo R14). Fix: have `Charged` delegate to the inner backend's overrides and
wrap each yielded node.

### R16 — `REFERENCE_SLOT_BYTES` undercounts the `(u8, Resolve)` slot: 16 B claimed, 24 B actual
`src/tree/mirror/streaming/window.rs:135-138`

`Resolve` (`materialized.rs:212-218`) is
`enum { Ready(Option<B::Node<H>>), Pending }`: `Option<Node>` consumes the
pointer's only null niche, so `Pending` forces an out-of-line tag — `Resolve`
is 16 B and the tuple `(u8, Resolve)` is 24 B, not the documented 16. The 8
B/child undercount propagates into `scope_price`, `SCOPE_ENVELOPE_BYTES`
(whose pin recomputes from the same wrong constant and so cannot catch it),
`DEFAULT_SYNC_MEMORY_BUDGET`, and the documented 22× ratio — roughly a 10%
understatement of the bookkeeping component at fan-heavy depths. (The
suggested Prefix offset does not exist: in situ next to an align-8 `Vec` the
prefix pads to its full 40 B, so `SCOPE_FIXED_BYTES` is exact and the full
undercount applies.) Nothing pins any of these constants to `size_of` of the
real types, though the repo already uses that pattern
(`adapter/tests/parking.rs`). Fix: derive the slot constants from `size_of`
(or pin them to it); `SCOPE_ENVELOPE_BYTES` then fails loudly and re-derives,
as its doc promises.

### R17 — `DISPUTE_WIRE_BYTES = 200` is tagged "Measured: the knee suite's bandwidth-bound cell calibrates it," but nothing measures or pins it
`src/tree/mirror/streaming/window.rs:154-157`

The cited cell (`tests/window_knee.rs:212-231`) asserts only a hop-count
ordering and reasons from a "~100 B floor" comment; no test anywhere computes
a bytes-per-disputed-message figure, and every consumer of the constant is
self-referential (the 22-ratio pin ties it to `SCOPE_ENVELOPE_BYTES` without
validating either against the wire; the operator suite self-calibrates BDP, so
it is structurally insensitive to the constant). A wire-format change lets 200
go stale with no loud failure, and the default budget plus both operator
equations then derive from a stale premise — the forgotten-constant genre
sync-budget §2.4's amendment warns about. If real cost falls below 200, the
design link's BDP-in-messages exceeds what the default admits and the
slowdown-1 promise quietly lapses. Fix: a calibration pin (implied
bytes-per-message brackets 200 over a known divergence/pipe), or re-grade the
doc tag from measured to manual-calibration and name the procedure.

### R18 — Budgets above the u32 framing ceiling wedge sessions mid-run; "Any value is safe, including zero" overclaims
`src/tree/mirror/streaming/remote/codec/budget.rs:62/73`, `src/peer.rs:432`

`RunBudget::from_bytes` stores any usize unclamped; the negotiated minimum
(start.rs:204-206) clamps only at `usize::MAX`; `admits()` happily grows a run
body past `u32::MAX` (buffering >4 GiB in RAM); the flush then fails at
`length_header` → `SupplyTooLarge` → session error, deterministically
re-failing while the >4 GiB single-subtree divergence persists: sync is wedged
until reconfigured. budget.rs:25-28 documents the encoder-side rejection
without reconciling it with the same struct's "Any value … is safe";
peer.rs:432 repeats the unqualified claim. Requires both ends configured above
u32::MAX plus a >4 GiB single-scope supply run — hence minor. Fix: saturate in
`from_bytes` (or the negotiated min) at `u32::MAX − SUPPLY_FRAME_OVERHEAD`,
making the claim true, plus one test in the over-ceiling regime.

### R19 — The liveness matrix claims "every session shape" but omits mutual bootstrap and retire-into-bootstrapper
`tests/handshake_liveness.rs:11-12`

Both omitted shapes have distinct entry-path code (`bootstrap_erased`'s
mutual-bail branch with epilogue certification; the retiree-donates-toward-
bootstrapper cross), and both are tested elsewhere — but only over roomy
default-capacity memory links, never over the 1-byte window this matrix
exists to exercise. Both paths were traced and believed live (greetings ride
the joined proxy accept), so this is a coverage/claim gap, not a suspected
deadlock. Fix: add the four cells or enumerate the covered shapes in the doc.

### R20 — `WindowConfig::default()` flips to the serialization floor under the additive `test-internals` feature
`src/tree/mirror/streaming/window.rs:386` — *promoted by reviewer judgment from the style tier*

`Default` resolves to `Fixed(Window::FLOOR)` when `feature = "test-internals"`
is on, and to `Budget(DEFAULT_SYNC_MEMORY_BUDGET)` otherwise. Cargo features
are additive and unify across a build graph: any crate in a workspace enabling
`rumors/test-internals` (a bench or harness crate) silently puts every
production session that never calls `sync_memory_budget` at the one-slot floor
— up to 484× wire-time slowdown by the branch's own table. Cargo.toml's
comment says "never enable it in an application," but a feature that changes
default *behavior* (not just API surface) violates the additivity convention
that comment cannot enforce. Fix direction: make the test floor an explicit
constructor tests opt into, and keep `Default` unconditional.

### R21 — The repo's only real-network Link instantiation is never run through the shipped conformance suite
`tests/common/tcp.rs`

`conformance::link::check` runs against the in-memory link (several
capacities) and the bench latency link (`tests/latency_link.rs`, whose doc
exists precisely "so the sweep measures the protocols, not an accidentally
nonconforming transport") — but never against tcp.rs, the one instantiation
over real sockets, which `tests/disruption.rs` trusts for process-kill
simulation and whose module doc asserts contract conformance rather than
validating it. A violation there (accept-future drop losing a stream,
half-close subtlety) would surface as a rare disruption-suite hang attributed
to the protocol — the exact misattribution the suite exists to prevent. The
suite runs on the caller's executor, so a `#[tokio::test]` with a loopback
factory is directly feasible. (Seam finding: the conformance pass validated
the suite, the tests pass validated tcp.rs's consumers, neither connected
them.)

### R22 — The pricing suite never checks `node_bytes` monotonicity, the property the quantile evaluation rests on
`src/conformance/backend.rs:274`; contract at `src/tree/mirror/streaming/backend.rs:49-56`

The contract requires `node_bytes` monotone in both arguments because
`from_budget` evaluates it at per-depth quantiles; the suite checks the
pointwise upper bound at each node's actual fan, the aggregate recurrences,
and the census — never monotonicity. The only check anywhere is
`from_budget`'s four-point `debug_assert`, compiled out of release. A
pointwise-honest cost function with a dip between quantile evaluation points
passes conformance while under-pricing every in-flight reference. Fix: a
monotonicity sweep in the suite (grid over child counts × bounds).

### R23 — `from_budget` charges the leaf-request edge at width `n.min(k)` but grants it a capacity that is provably always 1
`src/tree/mirror/streaming/window.rs:291-299` (charge), :324-328 (assignment)

Verified by hand arithmetic (and independently re-derived): `population[32]`
traces to `jointly_occupied(n, pair, 30)` → `small_mean_quantile(pair, 30,
48)`, where `den_bits = 241` and `num_bits ≤ 128` (pair is a product of two
u64s), so the quantile is `Some(0)` unconditionally and `capacities[0] == 1`
for every representable corpus and any budget (every height ≤ 7 floors the
same way; nonzero would need pair ≥ ~2^142). Meanwhile the solve charges
`n.min(k) × LEAF_REQUEST_BYTES` for that edge — up to 2.5 MB at the design
session — justified by a comment claiming a corpus-bounded population. The two
halves of the same function price and grant different widths for the same
edge; the envelope-derivation pin recomputes the charge side only, so it
cannot catch the gap. Direction of error is conservative (never unsafe). Fix:
make charge and assignment use the same bound, whichever semantics is
intended, and re-derive `SCOPE_ENVELOPE_BYTES`.

### R24 — "Exact under redaction" is untested exactly where redaction meets the join path
`src/tree/tests.rs:998`, :1037

The two aggregate proptests structurally cannot generate the arm that makes
the headline claim true through a merge: `version_size_aggregate_is_exact`
forgets on a single tree; `version_size_aggregate_survives_join` joins trees
grown under disjoint parties with no forgets, so the deletion-honoring filter
never drops an incoming argmax leaf during join and the resize-DOWN-through-
merge path has no failing-test witness. (The originally-suspected same-key
divergent-version arm is impossible — keys are content-addressed over
(version, value) — so the gap is arm (b) only.) The maintenance code was
traced and is correct by construction. Fix: interleave forgets of the argmax
on one side before joining, asserting against `naive_max_version_bytes`.

### R25 — `DelayedWire::round_trip` double-counts elapsed time on the running-clock path
`benches/support/latency.rs:400-422`

Returns `wall_start.elapsed() + virtual_elapsed` where the latter is a
`tokio::time::Instant` delta inside `block_on`. Under the paused clock the
components are disjoint (the documented model). Under `new_wall_clock` the
tokio Instant tracks the real clock, so the sum is ~2× actual. Latent today —
the only wall-clock consumer (`benches/window_wallclock.rs`) discards the
return and lets criterion time the closure — but the method's contract is
stated unconditionally, and a caller copying `examples/window_tradeoff.rs`'s
sum-the-returns pattern onto a wall-clock wire silently gets doubled figures.
Fix: store the paused flag and select, or scope the contract to paused-clock
wires.

### R26 — memwatch's swap-abort kill list can orphan a freshly spawned rustc
`tools/memwatch:112-116`

The abort path SIGKILLs the individually enumerated `build_pids` then
`$ROOT`; there is no process-group kill (the child starts as plain `"$@" &`,
no setsid). A rustc spawned by cargo inside the sub-second window between the
`ps` snapshot and the kills survives reparented and keeps compiling unwatched
— on a machine already past the swap limit, exactly the process the backstop
exists to stop. (Verifier correction: the window is intra-tick, sub-second —
not a full 2 s interval.) The removal of the global pkill (38aaad3a) was right;
the residual leak is real. Fix: re-run `own_procs` after killing `$ROOT` and
sweep until empty (bounded retries), or setsid + group kill.

### R27 — The 12→20 GiB memwatch threshold bump accommodated an undiagnosed 14 GiB rustc and outlived the problem it worked around
`tools/memwatch:35-38`

3315928f raised PROC_LIMIT 12→20 (and SWAP 16→20) with a one-line message,
leaving the calibration comment still deriving 12 GiB from the height-erasure
incident log. The pressure it accommodated is recorded only in 38aaad3a's
message ("an 8 GiB gate watchdog took down a legitimate 14 GiB bench rustc"),
and which compile legitimately needs 14+ GiB is recorded nowhere — while
38aaad3a's per-tree scoping removed the very reason a gate's limit had to
clear a concurrent bench build's peak. Net: the monomorphization-bomb detector
trips 67% later than its documented calibration, on the strength of a
workaround whose motivating bug was since fixed properly. Fix: restore the
calibrated 12 (passing an explicit override in the bench recipe — the justfile
currently passes none), or diagnose and record the 14 GiB compile and
re-derive.

### R28 — The `just window-tradeoff` recipe truncates the tracked, rustdoc-included table before the build succeeds
`justfile:211`

`cargo run --release --example window_tradeoff > src/.../tradeoff.md`: the
shell truncates the redirect target before cargo runs, so a compile failure
leaves the tracked file — compiled into `Peer::sync_memory_budget`'s rustdoc
via `include_str!` (`src/peer.rs:394`) — empty, and `include_str!` of an empty
file builds clean under `just gate`, so a distracted commit ships the docs
with the table silently missing. Fix: write to a temp file and `mv` on
success (the repo's own atomic-write practice).

### R29 — tradeoff.md has no drift gate, unlike the README
`justfile:210`

The generator labels the default row from `DEFAULT_SYNC_MEMORY_BUDGET` at
generation time; the constant changed twice on this branch and the table is in
sync today only because ca686533 manually regenerated. README drift gets
`readme-check`; this doc-of-record table (shipped in public rustdoc) gets
nothing, and full regeneration is too slow for the gate. Fix: a cheap
freshness assert (unit test formats the constant and greps the include_str'd
table for its rendered label).

### R30 — The only hashing benchmark was deleted while its measured claim survives
`src/tree/typed/hash.rs:148-155`; deletion in a3da46c4

The "~2× faster" contiguous-preimage claim predates the branch, but the
branch deleted `benches/branch_hash.rs` — the one bench exercising
`Hash::branch`, whose module doc recorded that the measurement applied to the
new preimage layout — and re-established no coverage; no surviving bench
touches `Hash::branch`. A future BLAKE3 or layout change silently invalidates
the figure with no instrument to re-run, and naming the deleted bench would
now be a ghost reference. Fix: reintroduce a small `Hash::branch` bench the
claim can name, or re-grade the comment from measured to derived/expected
without the pinned factor.

---

## 3. Confirmed nits

### R31 — Unchecked u128/usize arithmetic in the window charge can wrap for pathological `node_bytes`
`src/tree/mirror/streaming/window.rs:282`, :297. Requires a backend pricing
nodes near `usize::MAX` and ~2^64-message corpora; every realistic path is far
inside u128 (verified). `saturating_add`/`saturating_mul` make the solve total
in the safe direction (saturation narrows the window).

### R32 — `binding_capacity`'s stand-in session shape does not cover the knee suite's largest sessions
`tests/window_knee.rs:54-67`. The stand-in (COMMON + 32×64 = 4,096) dominates
only for capacities ≤ ~56, while the guard admits up to 256, at which the
largest measured session is 11,264 — the parenthetical "the assertion below
keeps that honest" protects less than it claims. Latent (slope band is wide);
compute per measured divergence as window_operator.rs does, or tighten the
guard.

### R33 — The parking 2 MB pin asserts the decoded skeleton only, but names the encoded+decoded coexistence figure
`src/tree/mirror/streaming/remote/adapter/tests/parking.rs:148-151`. The
assert bounds FAN² × 17 = 1,114,112 B against 2 MiB while the "~2 MB while
encoded and decoded coexist" figure it claims to pin is ≈ 2.23 MB — which
exceeds 2 MiB and is not what the assert computes (widening `Hash` to 24 B
keeps the assert green while the transient grows to ~3.3 MB). Assert the sum
or reword to say the pin covers the decoded half.

### R34 — swarm's shutdown spins forever on `inflight` if a party thread panicked mid-session
`examples/swarm.rs:404-406`; expects at :633/:659. The gossip `.expect`s sit
between the inflight increment and decrement with no drop guard; a session
failure leaks the slot and the `while inflight > 0 { sleep }` loop never
exits after 'q' (the panic hook restores the terminal, so the user sees the
panic, but the process hangs). Contained: in-process links make session
failure a bug by design. Decrement via a drop guard or bound the drain wait.

---

## 4. Hard-rule sweeps

### R35 — Ghost references (rule: no prose about code that no longer exists)

Concentrated in test files and files that landed after the branch's own
prose-sweep commit (44724ad0). Sites, all verified:

- `src/conformance/link/tests.rs:494-527` — the regression section throughout:
  "the suite's *former* soundness holes", "first committed pinned to the
  suite's *unsound* behavior", "The original probe wrote…", "the mux passed";
  also :48-49 "shipped exactly that way once".
- `src/testing/transport.rs:651-655` — ReorderingAcceptor narrates "an earlier
  draft" that degenerated to pass-through (the design lesson is worth keeping;
  restate as a positive invariant); :494 "the coverage the single-pipe wrapper
  provided when every stream shared one pipe" (borderline: wrap_io lives, the
  arrangement doesn't).
- `src/tree/mirror/streaming/remote/streams/tests.rs:337-339` — "recovering
  the deleted demux's frame-after-end detection".
- `src/tree/mirror/streaming/remote/proxy/tests.rs:515` — "under the deleted
  mux/demux session layer this exact shape deadlocked … the original stall
  reproduced from 64-byte to 16 MiB buffers" (the history already lives in
  design/streaming-wire-deadlock.md).
- `src/tree/mirror/streaming/remote/proxy/tests/malformed.rs:22-24`, :87-88 —
  "no longer crosses as a frame", "now that the opening rides the greeting".
- `src/tree/mirror/streaming/remote/proxy/start/tests.rs:6` — "where the
  listing lived before it rode the greeting".
- `src/tree/typed/untyped/tests.rs:398-402` — "under the old per-byte wrap
  rule it held by construction".
- `src/tree/mirror/streaming/remote/codec/budget.rs:64` and
  `tests/target_message_size.rs:54-55` — "the pre-batching one-leaf-per-frame
  wire traffic" (peer.rs:432 already has the positive form).
- `crates/before/src/borsh_impls/tests.rs:162-216` — "the pre-window
  `ReaderCursor`", "Kept verbatim from before the word window", "Replicates
  the pre-window wire pipeline exactly" (×2).
- `crates/before/src/codec/tests.rs:116`, :129 — "(the pre-word
  `encode_int`)", "(the pre-word implementation)".
- `crates/before/src/codec/tree.rs:11-14` — "the wire cursor's old buffer
  growth" (plus an unnamed "measured … at the profile's sampling floor" —
  which profile?).
- `crates/before/src/version/tests.rs:1122-1126` — "the definitional
  comparison it replaced".
- `crates/rumormill/src/net/tests.rs:60-61`, :84, :95-97 — "the property that
  failed when `ours` was read from a fresh snapshot", "the fresh-snapshot
  bug/verdict bug".
- `tests/hop_trace.rs:490`, :515 — "one fewer than before the opening question
  moved into the greeting"; assert message "hop count is unchanged" (compared
  to an unnamed prior state).
- `tests/party_conservation.rs:5-12`, :295-297 — the withdrawn version-hop
  design's story told in test rustdoc (the DECIDED/REJECTED exemption covers
  plan documents, not test files; the invariants stand without the narrative).
- `tests/gossip_pipelining.rs:36` — "the pre-window behavior paid one round
  trip per disputed scope" (module doc line 3 already has the positive form).
- `tests/async_wire.rs:6` — "The old in-process `join` is gone" (pre-existing;
  branch edited the surrounding doc).
- `.config/nextest.toml:10-11` — "after its attempt budget was tightened".
- `benches/gossip_fixed.rs:38-40` — measurement-history narration whose
  capacity-one referent no longer ships; `benches/support/grid.rs:24` — "the
  named shapes the old bench hard-coded" (pre-existing, survived the sweep).
- `tools/memwatch:35` — "12 GiB" comment above a default of 20 (see R27).
- Soft: `src/tree/typed/hash.rs:60` "load-bearing, not legacy" (negative
  framing answering a question only history raises); `AGENTS.md:21` "the mux
  it replaced" (arguably a sanctioned pointer at the decision record — 35
  lines above the rule that bans the phrasing; author's call).

### R36 — Design-document citations from code (rule: state the constraint inline; docs cite code, never the reverse)

- `src/conformance/backend/tests.rs:55` — the Rust-code instance (R10).
- `src/conformance/link/tests.rs:228` — "this is the §2 coupling" (resolves
  only against streaming-wire-deadlock.md); :502/:521/:545/:563 — "Regression
  (hole H2a…H2d)": identifiers defined nowhere in the codebase.
- `src/tree/arb.rs:189` — "supplies §7 item 3's *budget* only": an unnamed
  design doc's section, ambiguous across three docs with a §7.
- `tests/hop_trace.rs:1` — "the §9-item-3 instrument", likewise dangling.
- `.config/nextest.toml:6` — "See design/streaming-wire-deadlock.md section 7"
  (lines 1-5 already state the constraint).
- `Cargo.toml:76` — "(design/streaming-latency-serialization.md §9)" in the
  bench-profile comment (the inline rationale already stands alone).
- `tools/memwatch:35` — "see design/height-erasure.md, incident log
  2026-07-17".

---

## 5. Documentation-accuracy findings (single-pass, spot-checked)

Link and conformance:

- **R37** `src/link.rs:62` — "Network instantiations … live in sibling crates"
  names crates that do not exist (workspace has no network binding; the design
  doc says, accurately, that deployments implement `Link` themselves).
- **R38** `src/link.rs:360` — `MEMORY_STREAM_CAPACITY` (8 KiB) claims it
  "keeps honest sessions off the backpressure path"; a single default-budget
  supply frame is ~1.06 MiB and squeezes through in ~136 refills — bulk
  catch-up lives on the backpressure path. Liveness is unaffected; the
  rationale is false as stated.
- **R39** `src/link.rs:109` (and module bullet :16-17) — `accept` promises
  "in arrival order" while :55 disclaims any order and the suite blesses a
  batch-reversing acceptor as conforming. The method doc is the surface an
  implementer reads.
- **R40** `src/link.rs:202` — `SessionState`'s `pub` fields let safe code
  clear the poison latch and forge epochs, which the docs themselves forbid
  ("Never mirror or reconstruct…"); a fault-injection wrapper rebuilding
  `LinkParts` with the obvious literal does it by accident. Constructor +
  getters would make the invariant type-enforced.
- **R41** `src/link.rs:48` — the cancellation clause ties accept-drops to
  "session teardown", but the suite (and the in-crate reordering harness)
  drops them mid-session; the suite enforces slightly more than the contract
  states.
- **R42** `src/link.rs:82` — "hands an owned clone to each stream producer"
  describes the erased layer's Arc sharing, not per-producer clones; an
  implementer could build per-clone isolation semantics on that reading.
- **R43** `src/link/erased.rs:10` — the per-open cost claim omits the second
  allocation (`Box::new(tx/rx) as DynTx/DynRx`) beside the `Box::pin`.
- **R44** `src/lib.rs:191` — links feature-gated `conformance::link`; with no
  `[package.metadata.docs.rs]` block a default-features docs.rs build emits a
  broken link and hides the advertised module. `src/lib.rs:132` — `[`Link`]`
  brackets inside a doctest comment render literally.
- **R45** `src/conformance/link/tests.rs:78` — "a real error resurfaces from
  the next accept call" holds for the MemoryAcceptor it wraps, not for the
  generic parameter it is written over (one-shot errors are swallowed).

Window, budget, pricing:

- **R46** `src/tree/mirror/streaming/remote/proxy/work/pump.rs:8-11` — module
  doc still claims all three dataflow channels are one-slot; only `responses`
  is, the other two are window-wide (that is the branch's headline change).
- **R47** `src/peer.rs:392` and `design/sync-budget.md:164` — "pins the
  default as the inverse form's design-point value **exactly**" / "an
  identity, pinned to the byte": true for the unrounded quotient only; the
  published `22 × BDP / slowdown` form gives 275,000,000 vs the default
  271,187,500 (22 = div_ceil(4339/200)). ~1.4% loose, conservative direction;
  the "exactly" needs the quotient caveat both places.
- **R48** `src/peer.rs:348` — "plus a few MB of in-hand reply batches" carries
  no provenance; with up to 17 streams per direction the naive worst case is
  nearer ~19 MB per direction. Derive (state the stream-count assumption) or
  call it an envelope.
- **R49** `src/tree/mirror/streaming/window.rs:503` — `child_slots_quantile`'s
  recorded derivation inverts an inequality (`1-(1-p)^N ≥ 1-e^{-Np}`, not ≤);
  the code is correct, its validity resting on the unexplained `+1` slack and
  the Bernstein margin (verified numerically by the finder). The comment
  should record the actual argument.
- **R50** `src/conformance/backend.rs:222` — the accounting premise says leaf
  *payloads* are out of scope, but `ChargedNode::leaf` exempts the whole leaf
  node (handle + resident version bytes), so the depth-32 leaf-reference term
  the window prices is never census-checked. Under-describes a deliberate
  choice (suspected); one sentence fixes it.
- **R51** `src/tree/mirror/streaming/backend.rs:91-105` — `Backend::leaves`
  states no contract for the three properties the encoder enforces by panic
  (containment under the prefix, strict path order, nonemptiness —
  `adapter/encode.rs:224,250-263`); `children` and `assemble` document theirs.

Greeting, codec, API:

- **R52** `src/tree/mirror/streaming/message.rs:30` (also streaming.rs:12-14,
  handshake.rs:5-8) — every altitude's greeting summary says "version plus
  root-fan listing", omitting `set_len`/`max_version_bytes`/
  `target_message_size` — the negotiation surface two commits exist to add.
  Field docs are complete; the extracted first sentences are the spec.
- **R53** `src/peer/gossip.rs:563` — the staged-buffer doc says it may hold
  "the remote's greeting"; `Staged` only ever holds the 25-byte preamble, a
  distinction this branch's own handshake vocabulary now makes sharp.
- **R54** `src/rumors.rs:288-290` and `Retire::Retired`
  (`src/peer/gossip.rs:95-97`) — "the peer has confirmed" is stated
  unconditionally; the epilogue that backs "confirmed" is V2-gated, and a V1
  peer gets Ok/Retired on local completion with no confirmation. link.rs:156
  carries the carve-out; the two public surfaces that promise confirmation
  don't.
- **R55** `src/peer.rs:129` — "Bootstrapping without consensus" instructs
  comparing "the `remote_min_ticks` reported in the error": the field is
  `remote_min_events` (ghost name, pre-existing but now load-bearing), and the
  section was not updated for the branch's new
  `NetworkMismatch::local_min_events`, whose field docs cite this section as
  the canonical dominance rule.
- **R56** `src/tree/mirror/streaming/remote/codec/tests/error_atlas.rs:1` —
  "every codec error reachable without resource exhaustion" overclaims:
  `DecodeLeafError::{Version, Message, TrailingBytes}` are reachable from
  cheap wire bytes yet absent from the 47-witness snapshot.
- **R57** `src/tree/mirror/streaming/remote/codec/frame.rs:281` —
  `validate_children`, the canonical-order gate shared by two independent
  ingress points and newly re-exported, is the file's one undocumented pub
  item (strictness and error contract unstated).
- **R58** `src/tree/mirror/streaming/remote/codec/decode/async_io.rs:35` —
  `FrameRead::frame`/`FrameWrite::frame` lack `# Cancel safety` sections; the
  current safety rests on an undocumented usage invariant (every caller
  retains the in-flight future or drops the whole session — verified, no live
  bug).
- **R59** `src/tree/mirror/streaming/remote/streams.rs:521` — AcceptDriver's
  unasked-stream story says detection is "late, not prompt"; one class (a
  live-but-never-polled claim receiver) is *never* detected, not late.
  :136 — `StreamSender::frame` spans three await points and a mid-open
  cancellation poisons the peer's whole stream supply (classified
  SupplyFailed); no `# Cancel safety` section says so.
- **R60** `src/tree/mirror/streaming/remote.rs:14` — "each stream carries
  exactly one placement of that grammar" misuses "placement" as minted three
  sentences earlier (a stream carries many placements; the stream *component*
  is what's singular).
- **R61** `README.md:180`, :191 — a dead relative hyperlink
  (`Link#what-a-session-promises`) and an unstripped `[`Link`]` shortcut link
  render broken/literal on GitHub; root causes are two gaps in tools/readme's
  stripping regexes (no `#fragment` alternative in RUST_PATH; SHORTCUT's
  lookahead treats a prose colon as a definition marker). readme-check passes
  because the derivation is faithful to the same regexes.
- **R62** `crates/before/src/codec/gamma.rs:99-102` — the decline-reason doc
  says a wider-than-window code spills "to `Base::Big` past u64"; 65–127-bit
  codes decode in the bit loop's u64 fast path to `Base::Small`; only k ≥ 64
  spills. Doc-only (behavior differentially pinned).
- **R63** `src/tree/typed/hash.rs:60` — "Both tags are load-bearing" overstates:
  under the current layout field lengths already separate every leaf preimage
  from every branch preimage (verified by construction); the tags are
  defense-in-depth, which is worth saying instead.

---

## 6. Design-document internal findings

- **R64** `design/node-hash-preimage.md:3` — the status line pins the
  implementation to 51f6ecd1, which is not an ancestor of this branch (it
  lives only on wave1/integration); the landed commit is 5a6dd8a2 (hardened by
  0dd2743e). :113 cites `from_sorted_run`; the code item is
  `from_sorted_leaves` (markdown gets no link resolver; this is the backtick
  rot the tooling section warns about).
- **R65** `design/review-packet-link-transport.md:3` — the packet's pin commit
  (069ec491) and its ~40 per-series pointers are not ancestors of
  link-transport; every hash dangles once wave1/integration is deleted or
  GC'd. :195 — the "measured outcome of the whole campaign" table's
  pre-campaign baseline is the post-§5.4 state, excluding the campaign's own
  largest gain (the 80.3→31.6 ms conversion-boundary fix is inside the
  packet's own Series B): measured from the true start, V2 I=5000 is −78%,
  not −39%. Conservative, but a reviewer reconciling tables will trip on it.
- **R66** `design/single-socket-retrospective.md:102` — the harvest pointer
  `harvest/single-socket` dangles (no such ref; the archive branch
  wave1/integration exists — b9175099 fixed that one and missed this).
- **R67** `design/streaming-latency-serialization.md:461-468` vs
  `design/sync-budget.md:74-79` — the same suite's wave-model figures disagree
  ~5× between the two records (0.045–0.054 vs 0.0072–0.0127 hops/message; 40
  vs 63 against 39 vs 94) because the derived binding capacities moved between
  runs; both are honest, neither notes the configuration changed. One
  cross-note fixes it.
- **R68** `design/sync-budget.md:394` — "no `NODE_BYTES` token survives
  anywhere" is true of src/ only; the token lives on in
  streaming-latency-serialization.md:443 and throughout b05-envelope-sim.py.
  The audit-trail sentence should state its actual scope.
- **R69** `design/b05-uniformity-envelope.md:385` — "sweep-verified to
  dominate … over N ∈ {2 … 2⁵⁰}" reads as exhaustive; the sim sweeps 13
  sampled N values (the structural argument likely closes the gap; the
  provenance wording overstates what the sweep certifies).
- **R70** `src/tree/mirror/streaming/window/tradeoff.md:11` (and generator
  header, `examples/window_tradeoff.rs:56`) — a 0.9× cell in a table whose
  preamble declares every cell a worst-case slowdown factor, shipped into
  public rustdoc via include_str: run-to-run noise (compute cancels only in
  expectation), acknowledged in sync-budget §1.4 but not where the reader
  sees the number. Fix in the generator (clamp at 1.0× with a noise note, or
  state the noise floor in the preamble).

---

## 7. Style nits

- **R71** The greeting's 24-byte size-word prefix is spelled as bare literals
  at **three** co-evolving sites: `proxy/start.rs:222-261` (offsets 0/8/16/24
  in send and receive), `codec/capture.rs:122` (`FRAME_LEN + 24`), and
  `src/tests.rs:277` (`greeting_frame_len()`'s `+ 24`, which feeds the
  severance-fuse budgets). One named constant makes the layout change in one
  place; the third site is the one a two-site fix would miss.
- **R72** `LABEL_LEN = 2` has three private copies (streams.rs:56,
  codec/capture.rs:22, proxy/tests/harness.rs:37).
- **R73** `streams.rs:470` — error_route's load-bearing one-slot capacity is a
  bare `1` (the replaced code named the analogous HANDOFF_CAPACITY).
- **R74** `streams.rs:221` — `StreamError::Mislabeled` carries raw u8s where
  both values are validated `Stream`s at the sole report site.
- **R75** `window.rs:417/478/480/491` — the 2⁻⁴⁸ union-tail level (and its
  derived t=34, depth cap 40) recurs as bare literals across four sites with a
  cross-site consistency obligation (change 48 and 34 silently stops being
  48·ln 2).
- **R76** `proxy/tests.rs:502` — em-dash in an assert message (house rule:
  colons, for terminal compatibility).
- **R77** `benches/window_wallclock.rs:33` — the CELLS comment describes a
  2×2 grid; the table is three cells over two budgets.
- **R78** `src/tree/mirror/alternating/message/tests.rs:241` — comment cites
  `mirror::wire_snapshot`; the module is `mirror::alternating::wire_snapshot`
  (propagated from the pre-existing line 10; the sibling new file gets it
  right).

---

## 8. Questions for the author

- **R79** `src/peer/gossip.rs:253-257`, :312-313 — bootstrap, the one session
  that transfers the whole set, runs with the hardwired default window and
  message-size target on both participants; `Peer::bootstrap` exposes no
  builder, so a memory-constrained newcomer cannot lower its own bootstrap
  session's advertisement (the knob takes effect only from the first
  post-bootstrap session). Behaviorally inert for the window (empty local
  tree; supplies stream outside it) — but is the target being unconfigurable
  intended? (Two passes raised this independently.)
- **R80** `src/link.rs:61` — the QUIC 1:1 guidance: does the Independence
  clause need a connection-level flow-control caveat? If the connection window
  is smaller than the sum of per-stream buffers, unread bytes on a parked
  stream can block writes on another — which reforms the coupling the clause
  exists to exclude. (Suspected, not traced to a concrete deadlock.)
- **R81** `src/tree/mirror/streaming/remote/adapter/decode.rs:166` — a
  positional Query past the scope's children is rejected at decode, but a
  Match past them is accepted (`let _ = scope.next()`) and caught only after
  the entire reply decodes (`Violation::UnexpectedMatch`), so one reply can
  grow the skeleton Vec unboundedly first. Off-model (fail-fast is a bug
  detector) — is the asymmetry intended?
- **R82** `design/tracecheck.md:206`, :257, :282 — the plan doc conditions
  trace-validation obligations on "once the single-socket transport lands."
  This branch replaced the mux with the Link transport; is the trigger
  tripped, or mooted now the mux is deleted? Un-adjudicated either way.

---

## 9. Examined and dismissed

- The occupancy model's cross-peer independence premise (window.rs:36) was
  challenged — two honest replicas ingesting the same external feed minting
  the same content address with concurrent versions would be a jointly
  occupied population the birthday envelope doesn't bound — and **refuted**:
  keys are content-addressed over (version, value), so identical payloads
  under concurrent versions mint *different* keys; the premise holds within
  the model of record.
- One verifier argued R4 should grade minor on runtime-impact grounds (Local
  prices a pointer; no shipped path can breach). Kept major per the
  statement-faithfulness rule; noted so triage sees both framings.

## 10. Verified sound (positive assurance)

Each item below was explicitly checked and found correct — not merely
unflagged.

- **Link contract consumers**: each of the five stated clauses maps to a real
  assumption in streams.rs; the flow-control any-positive-capacity claim
  carries genuine measured provenance (the liveness matrix at a one-byte
  window with a fixture self-checked to overflow it); `STREAM_COUNT = 17`
  agrees with the codec's height schedule and is pinned. The erased layer has
  no behavioral drift: cancellation semantics, error passthrough, vectored
  writes, `&self`-concurrency via Arc all preserved, and the explicit-UFCS
  recursion guard in erased.rs is correct and necessary (the naive call
  stack-overflows at first poll).
- **Streams/session replacement**: lazy establishment real on both sides; the
  2-byte label protocol validated in order epoch→index→slot with all four
  violation classes reachable and tested; epochs advance in lockstep and burn
  on failed sessions; end-of-stream discipline (End control then EOF) enforced
  exactly; the clean-drain invariant is genuinely gated (with a negative
  control proving the drain assert catches a planted leftover byte).
- **Greeting**: minimum semantics symmetric (same `min` over the same greeting
  pair in both proxy entry impls); the 24-byte prefix cannot be omitted (short
  frame is a typed error, pinned); zero and u64::MAX degrade correctly; the
  joined send/receive is deadlock-free at a one-byte control window under a
  deterministic scheduler that fails on any wedge, on the accept path that
  production topology uses; every truncation point mid-greeting yields a typed
  error, not a hang.
- **Batching**: run-boundary correctness has a real prop test (greedy flush,
  budget compliance except lone oversized records, exact round trip); the
  oversized-leaf edge cannot loop (first record always pushed; budget-0
  pinned); split runs reassemble to one supplied node and re-encode
  canonically.
- **Budget arithmetic**: `DEFAULT_SYNC_MEMORY_BUDGET`'s const expression
  divides exactly and matches its documented premises;
  `SCOPE_ENVELOPE_BYTES = 4,339` pinned by exact recomputation including the
  end-to-end BDP-in-flight assertion; the operator equations' algebra
  re-derived by hand and correct as published; every headline constant
  recomputed independently matches across code, rustdoc, design docs, and the
  committed table (4,339; 200; 22; 1,114,624; 271,187,500; 12,500,000); no
  zero-capacity channel reachable (capacities clamped ≥ 1; the field private).
- **Hashing (lever E)**: the preimage layout is injective on (kind, compressed
  span, ordered child records) — length-tagged prefix, big-endian u16 count,
  fixed 17-byte records, tag-separated kinds; the suffix-only leaf commitment
  is sound (a root-anchored chain commits all 32 path bytes exactly once); the
  32-byte lone-leaf-root prefix bound matches the encoding.
- **Wire snapshots**: every regenerated snapshot maps to a commit that
  declares the protocol fact it re-encodes; V1 snapshots moved only under the
  declared hash change (framing intact, only hash bytes differ).
- **before codec**: `decode_int_window` is accept-exact and
  reject-conservative (zeros past the stream can only lengthen the apparent
  prefix into the fallback, never shorten it into a bogus accept); the word
  encode path is byte-identical to the per-bit emit; fast/slow equivalence is
  prop-pinned differentially over boundary-biased inputs.
- **Pricing/aggregate maintenance**: every branch construction flows through
  `Node::branch` or `from_sorted_leaves`, both recomputing `version_bytes` as
  the max in hand; act/join/deletion-filter reassemble touched spines through
  `Node::branch`; the aggregate is a max, so no accumulation overflow exists.
- **Test harness**: fault.rs injects on every Link surface with no silently
  absorbed path (the one inherent case covered by offset sampling by design);
  tcp.rs is contract-sound in the respects checked (self-labeling makes
  arbitrary accept order legal; ephemeral ports; listeners die with links) —
  R21 asks for the suite run that would make this vacuous-proof;
  wire.rs's no-waker drain probe is sound for the memory link, with a negative
  control.
- **API surface**: `target_message_size` has exactly one stated semantics (the
  exchanged minimum) at every surface except the R7 test file; the poison
  latch is airtight across every error/cancellation path in gossip, retire,
  bootstrap, and the driver (including zero-bytes-consumed and
  drop-with-staged-bytes).
- **Mechanical sweeps**: the headline numbers are named constants with
  provenance-labeled derivations; every `unwrap`/`expect` on a wire-input path
  in new non-test code was adjudicated unreachable-or-correct; both lockfiles
  agree; both new proptest-regressions files are committed and well-formed.
- **Gates**: fully green (header block above).

## 11. Residual risks and test gaps

Beyond the numbered findings: (a) the contract/conformance pair is the
branch's public promise, and its two strongest findings (R1, R2) are exactly
the two ways a caller-built link can pass validation and still deadlock or
serialize in production — worth fixing before any external caller builds a
link; (b) the memory model's honest status is "priced, empirically pinned"
rather than derived (R4/R16/R17/R23 all reduce to this), which is fine so long
as the code says so — the fix is prose plus pins, not a redesign; (c) V1↔V2
and skew behavior is tested for the shapes enumerated, but intra-V2 dialect
skew (R6 corollary) hangs rather than rejects and is untestable until a
version-bump policy exists; (d) nothing on the branch runs the conformance
suite against a transport with real asynchrony between connect-return and
accept-availability (R11/R21 jointly), which is the regime QUIC/TCP callers
will actually inhabit.
