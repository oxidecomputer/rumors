# Report summaries, positives, open questions, dropped

<!-- source: final/api-core.md -->
# Partition api-core: Public surface core: Peer, Rumors, observers, Batch, Snapshot, errors, tutorial

## Partition summary

This partition is the replica's public face. `Peer` (src/peer.rs) is the `!Clone` identity anchor: the `Network`, the window and run-budget settings, the `watch`-guarded `Inner { party, tree }`, the bookmark mutex, the type-erased payload codec, and the observation attachment. `Bootstrap` and `BookmarkedBootstrap` (src/peer/bootstrap.rs) are the type-state builders behind `Peer::bootstrap`, and `Joined` their outcome. `Rumors` (src/rumors.rs) is the cloneable working handle: a `Peer` plus an `Extant` token whose count lets `try_into_peer` reclaim the anchor; every operation forwards to a `pub(crate)` method on `Peer`. The three set observers, `UnorderedMessages`, `CausalMessages`, and `Changes` (src/rumors/), share one `Channel` type that materializes the `watch` wait as an owned boxed future. `Batch` (src/batch.rs) queues `Action`s and commits them in one `send_if_modified` critical section; `Snapshot` (src/snapshot.rs) is a `Network` plus a structure-shared `Tree<T>`. `Network` reserves its all-zero bootstrap sentinel structurally, `tags.rs` is the CBOR tag table, `Protocol` has one variant, `Error<B>` is a flat taxonomy generic only for its bookmark arm, and `tutorial.rs` is a docs-only page. I read all fifteen partition files in full with line numbers (4039 lines; src/peer/bootstrap/tests.rs, 146 lines, is the only test code), plus the out-of-partition context the findings depend on (src/peer/gossip.rs, src/tree.rs, src/message.rs, src/bookmark.rs, src/observe.rs, src/tree/mirror/handshake.rs, the cited tests, `before`'s `Version`/`Rank`/`Ranked`, and the vendored tokio 1.52.3, bytes 1.11.1, and thiserror-impl 2.0.18 sources at the versions Cargo.lock pins).

The code is in good shape. The `Peer`/`Rumors` XOR is argued at the types rather than in prose, the type-state builder lets each `join` declare only its own outcomes, `Batch` is closed by the borrow checker (two `compile_fail` doctests pin both escape routes), the observers state their checkpoint discipline at the fields and pin it at both boundaries on both faces, every `must_use` names the concrete loss, and the error module opens with a per-variant recovery table at exactly the altitude a caller needs. I checked the contract claims I could reach against the code and found them accurate, with the exceptions below.

The findings cluster in four places. First, two removal commits did not sweep their own remainders: the V1 retirement (368da2a5) left the vocabulary of "selecting" a `Protocol` in the error table, an error message string, and two method docs, plus a `Default` derive nothing calls; and the commit that made `Snapshot::iter` opaque (8dc0596ed) left a `pub use crate::tree::Iter` and an `IntoIterator` impl that name a type no public path reaches. Second, the `Bootstrap` page and its plumbing tests were written for a three-setting builder and not re-read when `observe` (40b1e96a) and `payload_depth_limit` (4356e197) landed. Third, `#[derive]` on generic wrappers imports bounds the representations never use, so `Error<B>` is not `std::error::Error` for a legal non-`Debug` bookmark and `Snapshot<T>` refuses to clone a non-`Clone` payload; the crate already states the counter-principle in two hand-written impls. Fourth, two small concurrency pieces carry more machinery than their invariants need: the three observers each re-spell the same two `Channel` transitions (and `Changes` implements its state machine twice), and `Extant` keeps two counters of one quantity. One verification gap stands out: the `claimed` CAS and the subscribe-before-shed ordering in `try_into_peer` are exercised by no committed test with two concurrent reuniters. The remainder is prose mechanics and small idiom, listed as nits.

Findings are numbered in path-then-line order. One finding (api-core-10) anchors in src/message.rs, outside this partition's file list; the perfapi lens filed it here because `Batch::send` is its only call site in the partition, and the message.rs partition's finalizer may hold a duplicate.


## Positives

- `Bootstrap`/`BookmarkedBootstrap` argue the type-state split at the type (bootstrap.rs:269-273: each state's `join` declares only its own outcomes), and every manual impl carries its reason: the unbounded `Clone` (81-83) because the payload is phantom, the `Debug` showing the bookmark by `type_name` (286-287) because `Bookmark` does not require `Debug`.
- `Batch` is a closed scope handle enforced by the borrow checker: the two `compile_fail` doctests (rumors.rs:320-336) pin both escape routes, and `commit` reads the changed flag straight from `Tree::act` with the no-root-hash-read invariant metered in src/tests.rs:644-655 (with a liveness leg at 626-635).
- `try_into_peer_inner` (rumors.rs:108-131) gets the hard ordering right and says why at the fields: subscribe before shedding, shed before waking. api-core-22 dissolves the need for the second rule; the reasoning as written is correct.
- The observers state the checkpoint discipline once at the field (causal.rs:46-53), mirror it in `UnorderedMessages`, and pin it on both faces at both boundaries; `folding_delivered_versions_can_lose_a_message` in tests/listen.rs is a committed demonstration that the tempting wrong implementation fails. `CausalMessages`' ordering promise rests on a stated theorem (`before::Rank` strictly monotone in the causal order, rank.rs:152-156) plus a two-line cross-pass argument.
- `Network` reserves its all-zero bootstrap sentinel structurally (`from_rng` redraws) and keeps the serde form to one opaque byte string with the rationale beside it (network.rs:27-29).
- error.rs opens with a per-variant table of replica state and the action beyond reconnecting (10-28), exactly the altitude a caller matching on `Error` needs; the "counterparty bug: report it" rows keep the conformance-detector framing of the model of record. `Error::Epilogue`'s doc (124-159) is a clear statement of the two-generals residue and what the exchange protects. `Error::widen` closes the uninhabited arm with `match never {}`, so adding a variant cannot mis-widen silently.
- `Snapshot`'s cheapness claim is mechanically true (a `Root` clone is a `Bytes` refcount bump plus an `Arc` bump; `len` is a stored count; hash and bounds are `OnceLock` memos), and `Snapshot::hash`'s claim that the frontier is excluded is true at the mechanism (`root_hash` hashes nodes only; the ceiling rides outside `Root`).
- Every `must_use` names the concrete loss: `Bootstrap`/`BookmarkedBootstrap` (does nothing until `join`), `Joined` (a peer or the bookmark), `Retire` (leaks the identity), `Unbookmarked` (strands the identity), `gossip_when` (the driver does nothing until polled).
- `Peer` and `Rumors` `Debug` impls print a bounded summary independent of `T: Debug`, the right default for a replica that may hold millions of messages.
- changes.rs:36-43 ("This signal alone does not make a gossip driver") names the concrete deadlock a naive loop produces and points at `gossip_when`: a hazard section that earns its space. rumors.rs:196-207 explains a deliberate API omission (`send` returns no `Version`) with the intended pattern and the batching argument.
- tutorial.rs: every step is a complete program with its exact output, the final program is the cumulative one, and step 5's "one driver per end" is called out as the thing to remember. The `Peer` lifecycle example (peer.rs:47-116) covers all `Retire` outcomes and the mutual-bootstrap bail in one runnable block.
- benches/in_memory.rs covers every in-memory public operation in this partition, and tests/api_send_bounds.rs pins the `Send`/`Sync` contract of the handle types and every async entry point at compile time; api-core-5's acceptance test extends exactly that shape.

## Open questions for Finch

- `Inner.party: Option<Party>`: the `None` state exists only while `Peer::retire` holds the party in flight, yet it forces two unreachable arms (batch.rs:136-139, api-core-2; gossip.rs:825-828, "keeps the arm total without a panic path") that each need a proof, and the two arms chose different shapes (silent `return false` versus adopt-the-donation). Holding the in-flight party in the retire future and keeping `Inner.party` total would delete both. I did not size the change against `PartyGuard` and the bookmark critical sections. Recommendation: rule once for both arms now (api-core-2's `expect` is the cheap consistent fix), and schedule the dissolution as a design item.
- `sync_memory_budget`'s sizing guide (peer.rs:371-465 plus the included table): the rulings of 2026-07-23 and 2026-07-24 placed the derivation in this rustdoc. The prose lens's altitude case for moving "# Choosing a budget" to a docs-only `sizing` page beside `reconciliation`, leaving the method with its ~40-line contract and a link, is worth reconsidering now that the crate has docs-only pages; api-core-15 fixes only what the rulings did not cover. Recommendation: move it; the rulings were about where the derivation lives relative to code, not about which rustdoc page.
- `Protocol` after the V1 retirement: decision 2 keeps the enum public and `#[non_exhaustive]` as wire vocabulary, which closes the lenses' question. The retirement note's status line (README.md:7-8) says the `Protocol` enum "went with" the builders, which the tree contradicts; agent notes are exempt from the prose rules, but a reader following that line will be misled. Recommendation: one-line correction in the note.
- batch.rs:22-26 is your own wording (c927ba04, a GitHub web edit): "concurrent gossip rounds" where the crate elsewhere says sessions, and "holds no lock" in two consecutive sentences. Not swept as a finding because it is your voice; do you want it aligned to "sessions" and the repetition folded?
- lib.rs:45-54 states scaling claims marked "derived" ("run stale in proportion to roughly the square of the bandwidth shortfall", "falling roughly as the inverse square root of the backlog"). I found no committed instrument or pinned measurement in tests/ holding them. Is there one, or should the doc state the qualitative behavior and leave the derivation to a design note?
- Lock scope of `Batch::commit`: the whole `Tree::act` runs inside `send_if_modified`'s write lock, so `snapshot()`, the `Debug` impls, and observer `borrow_and_update` calls block for the commit's duration; a 10^5-message `send_all` is O(N log N) under the lock. Inherent to the design (insert paths derive from post-tick versions read under the lock), so no finding; is a measured figure for the large-batch case wanted before persistent storage lands?
- api-core-17 (first-sentence mood) and api-core-34/35 (`Iter`, `==`) each need a ruling before work starts; my recommendations are in the findings (third person per the doctrine; delete the `IntoIterator` impl; document `==` rather than drop it).

## Dropped

- Structure [30]/[36] (`Changes::next_inner` duplicate) and [41] (enter-the-owned-wait block): duplicates of api-core-33.
- Prose [11], correctness [28], perfapi [39] (Protocol selection ghosts): duplicates of api-core-4; the `Default` derive half of structure [2] is api-core-21.
- Prose [15], perfapi [37] (unnameable `Iter`): duplicates of api-core-34.
- Structure [4] and correctness [26] parts (a) and (b) (`Snapshot` derive bounds and O(n) `Debug`): merged into api-core-5; part (c) (ceiling-inclusive `==`) is api-core-35.
- Correctness [29] (bootstrap plumbing module doc): duplicate of api-core-20.
- Prose [22] part (b) (batch.rs "concurrent gossip rounds", doubled "holds no lock"): owner-authored wording (c927ba04); moved to open questions rather than swept. Part (d) (`cryptographically impossible`) merged into api-core-11.
- Prose [21]'s `static_assertions` comment: merged into api-core-9 with the dependency-placement question the prose lens raised.
- Prose [12]'s proposal to relocate the sizing guide: reopens the 2026-07-23/24 rulings without new evidence; moved to open questions, with the three unruled sub-items kept as api-core-15.
- Prose [16]'s "pick one home" framing: the writing doctrine (writing-style.md:219-230) mandates repeating hazards with a link to one canonical explanation; kept as api-core-27 in the doctrine-compatible form (one canonical, one summary).
- Prose [19]'s direction (align third-person sites to the imperative): the doctrine (writing-style.md:122-124) fixes third person; kept as api-core-17 with the direction reversed and marked owner-gated.
- Perfapi [42]'s mechanism (`-D warnings` missing from the doctest recipe): refuted by the refutation pass (rustdoc's injected `#![allow(unused)]` is why the warnings never fire); kept as api-core-32 with the crate-attribute fix.
- Structure [6]'s variant tally ("eighteen"): the enum has a different count and no tally belongs in the write-up; kept as api-core-7 without the number.
- Lens open question on `Protocol` remaining a one-variant enum: already ruled (retirement note decision 2); recorded under open questions only for the note's own inaccuracy.
- Lens open question on whether `Bootstrap::observe` is retained by the joined peer's later sessions: the correctness lens did not read the callers and neither did I beyond tests/observe.rs:275-289; api-core-20 asks for a direct retention assertion, which settles it.
- `try_into_peer` sitting in the `impl<T, B: Bookmark>` block (rumors.rs:427) though its body needs only `BookmarkError`: below the bar; a `Peer<T, B>` with a non-`Bookmark` `B` is unconstructible, so the stricter bound costs nothing.
- perfapi's wire-ingress mirror of api-core-10 (`Message::from_wire` storing sliced run buffers): out of this partition; belongs to the streaming decoder's reviewer.

<!-- source: final/benches-envelope.md -->
# Partition benches-envelope: Criterion benches and their support, the window trade-off example, the envelope simulator

## Partition summary

This partition is the crate's measurement apparatus. Five Criterion targets sit under `benches/`: `branch_hash.rs` re-measures the feeding-strategy claim behind `Hash::branch`'s one-shot preimage; `in_memory.rs` sweeps the public single-set surface (batch insert, iteration, redaction, causal-delta ranges, both observer faces, point lookup) at three set sizes; `gossip_grid.rs` and `gossip_fixed.rs` reconcile diverged peers through `Rumors::gossip` over a persistent in-memory link, the first across a divergence cube and the second at fixed `N` with a link-latency axis; and `window_wallclock.rs` re-runs three cells of the window suites' virtual-time model on a running clock. Three `#[path]`-included support modules serve them: `support/grid.rs` (the divergence cube and its fixture builder), `support/wire.rs` (a pollster-driven `MemoryLink` pair and the bootstrap fork), and `support/latency.rs` (a delayed-pipe `Link` whose paused-clock runtime turns wire delay into an exact, load-independent hop count; seven `tests/` suites include it too). The two examples are `window_tradeoff.rs`, twelve lines that print `rumors::testing::window_tradeoff_table()` for the rustdoc table the gate byte-compares, and `envelope_sim.rs`, a 1,586-line port of the Python analysis behind the session window's occupancy envelopes, which `window.rs` cites as the certificate that its integer quantiles dominate the exact Chernoff tails.

All 3,337 lines are dev-side code: every file in the partition is bench, support, or example code, held to the test standard; none is production. I read every file with line numbers and corroborated the external anchors the findings rest on (`window.rs`, `hash.rs`, `budget/tests.rs`, `batch.rs`, `peer.rs`, `rumors.rs`, `snapshot.rs`, the pinned criterion 0.5.1 sources, the include sites, the recorded rationales in `.agent-notes/`, and the git provenance of the expiry commits).

The benches and their support are in good shape. `latency.rs` argues its measurement model from mechanism rather than asserting it, refuses to report a virtual figure on a wall-clock wire, and fails loudly off the delay lattice; `grid.rs` states its throughput denominator and enforces the redaction precondition at the one site that consumes it; every Criterion group keeps fixture construction untimed and warms lazy memos explicitly; `branch_hash.rs` prices the digest through the public `MERKLE_HASH_LEN` so the widening moved it automatically; and the trade-off table is a derived artifact with a `just` recipe, a temp-file-then-move write, and a byte-compare in the gate. The residue is small and concentrated in prose that outlived the code it described: a `join` bench deleted in June is still cited, a fixture-discipline section justifies a rebuild policy the benches do not follow with a fork hazard none of them can trigger, and a `Batch` contract is stated backwards. The structural findings are module-layout nits (wire nested under grid, two dead-code-allow conventions, duplicated helpers) and two denominator slips in `in_memory.rs`.

The envelope simulator carries the partition's substantive problems. Its constants say they mirror the crate but encode the 16-byte Merkle hash and the pre-CBOR target message size, so every byte-denominated table it prints describes a wire that does not ship. It pins a flat-solve baseline, labeled "landed" and "default 16 GiB", whose only referent is a branch that never survived. Its depth-0 prefix shift overflows a `u64`. And the dominance sweep that `window.rs` names as its certificate compares the simulator's private copy of the integer envelopes against the simulator's own oracle, never touches the shipped functions, never covers the pair-product generalization, and is run by no recipe, workflow, or test. The math family it certifies is sound and the transcription matches today; what is missing is any mechanical tie between the certificate and the code that claims it.


## Positives

- benches/support/latency.rs:10-62 states the measurement model as mechanism: which component is deterministic and why (the runtime's one thread, no library timers, deadlines on the delay lattice, OS descheduling unable to convince the runtime a pending task is idle), which component moves with load, and which figure assertions may rest on. `round_trip_virtual` refuses to report a virtual figure on a wall-clock wire (475-479) and `hops_on_lattice` fails loudly off the lattice instead of rounding (538-553). The `# Panics` sections are one-line proofs, not labels. I traced the reader and writer state machine (waker set and consumed under the same lock, the equal-deadline timer path, half-close via `writer_gone` with queue drain before EOF, `buffered <= capacity` by construction) and found no lost wakeup, underflow, or busy loop; `tests/latency_link.rs` pins the link against the public conformance suite at zero and nonzero delay.
- examples/window_tradeoff.rs is twelve lines over `rumors::testing::window_tradeoff_table()`, `just window-tradeoff` (justfile:773-775) writes through a temp file and `mv` so a failed build cannot truncate the tracked artifact, and `tradeoff_table_matches_the_derivation` (window/tests.rs:258-263) byte-compares the committed table with a message naming the recipe: a derived artifact with no hand-maintained copy and a closed drift path.
- benches/branch_hash.rs restates `Hash::branch`'s preimage faithfully (I checked every field against hash.rs:162-213), takes the hash width from the public `rumors::MERKLE_HASH_LEN` so the 16-to-24 widening moved it automatically, names the exact claim it keeps re-measurable, and hash.rs:167 cites it back with the measured magnitude.
- benches/support/grid.rs denominates throughput precisely (`Cell::divergence`, 86-95: per-side transfer, not the shared size), documents the disjoint-block redaction scheme and its `common >= 2 * redacted` precondition once in the module doc, enforces it in `cells()` (124), and restates it where the slices are taken (168-169). The corners-of-the-cube list (23-28) makes the classic divergence shapes fall out of the sweep instead of being enumerated.
- Every Criterion group keeps fixture construction untimed (`iter_batched` at `PerIteration`), warms lazy memos explicitly, and says what its throughput denominator means; `gossip_grid.rs:60-72` moves the identical corner to a latency-only group with the reason stated.
- benches/gossip_fixed.rs:27-39 tells the reader how to read the latency lines (intercept is compute, slope over delay is the serialized hop count) and which shape of slope is the regression the session window exists to prevent; the latency sweep's `iter_custom` (192-201) sums only the harness's own reported elapsed, keeping the per-iteration fixture rebuild out of the figure, and the comment at 175-180 explains the sample-size choice in terms of what is bounded.
- benches/in_memory.rs's per-bench docs each say what the column measures and which claim it puts under measurement (`range_delta` at 168-174 is the model), the right altitude for a bench.
- examples/envelope_sim.rs's integer arithmetic is disciplined: `pow256` is the single crossover guard, `bernstein`'s `x = ⌊√(2μT)⌋ + T` certifies `e⁻ᵀ` in the regime every caller is in, `small_mean_quantile`'s bit-length argument is sound, `binom_tail_log` handles the `p = 0` and `a = n` boundaries explicitly, and the `# Numeric representation` paragraph (39-46) says exactly where `u128` gives way to `f64` and why. The `--manifest` mode dumping every deterministic quantity for value-for-value diffing is the right shape for a numerical certificate. The file keeps inline tuple types with `#[allow(clippy::type_complexity)]` (1335, 1390) rather than coining names.

## Open questions for Finch

- Should `examples/envelope_sim.rs` remain the certifying tool of record (ded24eb3a), or should the dominance certificate move into `window/tests.rs` as a differential test over the shipped functions, extended to asymmetric `(A, B)`, with the example's integer copies dissolved? Recommendation: move it; the oracle's independence survives, the gate enforces it, and the pair-product path gains its first bound-level check. If the example stays, the minimal step is `test = true` in Cargo.toml so `test-all` runs the analytic tier.
- Is the flat-solve baseline (`NODE_BYTES`, `k_flat`, `check_landed_replication`) a standing design comparison worth keeping under a positive name, or history to retire? Recommendation: retire it; both agent notes already record the rejected shape, and the tables it feeds are computed for a wire that no longer ships. Same question for the `L(N)` section, which the sync-budget note earmarks for a future transport task: keep it in `.agent-notes/` beside that plan.
- Should the example's `FAN` and `KEY_DEPTH` mirrors reach the crate through `rumors::testing` (both are `pub(crate)` in window.rs), or be declared as the simulator's own model parameters? Recommendation: declare them as the simulator's own; they are structural constants of a radix-256 trie over 32-byte paths and will not move.
- Renaming the `V2` Criterion series ids breaks comparison against any saved local baselines; no committed roster depends on them. Is there bench history worth preserving, or is a clean rename fine pre-release? Recommendation: rename now, before a release makes baselines worth keeping.
- The benches are compiled by the gate (`bench-build`) but never run; a fixture `expect` that starts panicking surfaces only when someone runs `just bench`. Is a `--sample-size 10 --measurement-time 1` smoke of each bench binary worth a `just all` leg? Recommendation: yes, as a cheap liveness check; it would also have caught nothing here, since the benches are sound, so the case rests on the future.
- `Wire` uses 64 KiB pipes (wire.rs:10) while `DelayedWire` in the latency sweep uses 8 MiB (gossip_fixed.rs:87); the zero-latency intercept of the fixed groups and the `d = 0` point of the latency groups therefore measure different harness overheads. Intended (the memory link as a socket-buffer stand-in), or should they share a capacity so the intercepts compare? Recommendation: state the intent at `CAPACITY` either way.
- Crate-wide register: `//` line comments in src use em-dashes 116 times and "knob" appears 36 times, both contrary to the house rules (spaced double-hyphens in code comments; unanchored metaphor as jargon). Both are one decision and, once made, mechanically enforceable by the doc linter. Recommendation: rule once, then sweep; do not fix them piecemeal in this partition.
- `Gossiped.stats` (bytes sent and received, disputed scopes, window granted) is discarded by both wire harnesses. Reporting `Throughput::Bytes` alongside elements would make the gossip benches read as wire efficiency as well as compute. A bench-design choice, not a defect; recommendation: add it when the next bench pass happens.
- window_wallclock.rs:36-38 asserts the 256 KiB budget runs "below and above its serialization knee (1k pipelined, 10k serialized)"; the knee suite computes its own predicted knee from the binding capacity. Recommendation: have the bench doc cite that derivation rather than hard-code the two divergences as the two sides, so a window change cannot leave the claim stale.

## Dropped

- "envelope_sim certifies its own copy" as a high-severity simplification (structure [0]): merged into benches-envelope-32 at medium, per the refutation's reasoning that window.rs carries inline sufficiency proofs; the class is verification-gap, which is what the defect is.
- Correctness [31] and perfapi [38] (the certificate is never executed; the example imports nothing from the crate): duplicates of benches-envelope-32.
- Prose [18] (nothing runs the envelope certification): the enforcement half of benches-envelope-32, merged.
- Structure [1], prose [16], perfapi [39] (mirrored constants): duplicates of benches-envelope-28 (correctness [30] is the most complete report and is the base).
- Structure [2] (flat solve and L(N) model unshipped designs): merged into benches-envelope-29 with prose [17].
- Structure [3] (dead-code convention): merged into benches-envelope-22 with prose [23]; its "nine of ten includers" is corrected to eight of nine.
- Prose [19] (gossip_fixed's stale include comment): merged into benches-envelope-16 as the comment half of the wire-under-grid nesting.
- Perfapi [45] (duplicated helpers across binaries): split across benches-envelope-11 (send_units), benches-envelope-16 (bootstrap_fork), and benches-envelope-33 (the sweep grid).
- Prose [27] (send_units and its rationale stated three times): merged into benches-envelope-11.
- Prose [20] and perfapi [46] (V2 label, unreachable arms): duplicates of benches-envelope-2; the attribution of the unreachable arms to the V1 retirement is corrected, since 818a8707 wrote them with the two-element sweep.
- Perfapi [48] (`--fast` standalone): duplicate of benches-envelope-27.
- Perfapi [47] (branch_hash mirror): duplicate of benches-envelope-1.
- Perfapi [44] (runtime inside the timed body): duplicate of benches-envelope-15.
- Perfapi [49] (OsRng draw inside batch_insert's timed loop): merged into benches-envelope-12, whose `iter_batched` resolution removes both the drop and the syscall; its unmeasured "low-single-digit percent" magnitude is dropped.
- Structure [5]'s stated mechanism (a split removes the dead-code allows): refuted; every includer uses a different subset within each layer. Kept as benches-envelope-20, a nit design proposal whose payoff is legibility.
- Prose [21]'s proposal to rename "receive window": dropped; it is the established networking term and the design record's vocabulary. Narrowed to benches-envelope-4.
- Prose [25]'s em-dash-in-`//`-comment item: dropped as a partition finding; the crate uses em-dashes in line comments throughout (116 sites in src), so it is a crate-wide register decision, raised as an open question. Its "knob" item: crate vocabulary (window.rs:14 "The public knob", 36 uses in src), same disposition.
- Correctness [29]'s numpy claim about the retired Python: out of scope for the tree and unverifiable here (numpy is not installed); the port-bug attribution rests on reading the Python source, and the Rust defect stands on its own.
- The criterion 0.8.2 citations in correctness [34] and [37]: corrected to the pinned 0.5.1 (Cargo.lock:519-520); the mechanics are the same and the verdicts stand.
- `children()`'s `k.max(1)` in branch_hash.rs:50 as a dead guard: below the bar (the range is empty at `k = 0`, so the guard is never exercised and harmless).
- The persistent-wire stream-accounting question from the correctness lens (whether a session can leave an opened-but-unaccepted stream): not a finding; `link.rs` states the contract and `tests/reuse.rs` pins reuse, and no lens found evidence against it.

<!-- source: final/conformance.md -->
# Partition conformance: The public conformance suite for caller-built links and backends

## Partition summary

The partition holds the crate's two validation suites. `conformance::link` (`src/conformance/link.rs`, public under the `conformance` cargo feature) is the black-box suite a deployment runs against its own `Link` transport: seven checks, each consuming a fresh link pair from the caller's factory and probing both directions, that turn violations of the link contract into panics naming the clause (byte assertions) or into hangs the caller's timeout bounds (liveness clauses). Streams are classified in-band by a tag or index byte, never by accept order, so the probes tolerate the reordering freedom the contract grants acceptors, and `check_sessions` runs bootstrap and gossip end to end with a `CountingConnector` floor proving data streams opened in each direction. Its sibling `link/tests.rs` runs the in-memory link through the suite at the default and one-byte windows and under a reversing acceptor, and holds five contract-violating fixtures (a shared-FIFO mux, direction-coupled control halves, a lossy dequeue-then-await acceptor in both directions, a capped connector, a pooled window sized below the bound) to a deterministic `Quiescence::Stalled` verdict.

`conformance::backend` (`src/conformance/backend.rs`, `cfg(test)` and `pub(crate)`) is the storage-pricing suite. A `Charged<B>` decorator keeps every live node handle's measured bytes on a process-global ledger, checks `node_bytes` pointwise where leaves are constructed, where parents are assembled, and along the bulk `leaves` and `assemble` overrides, sweeps the cost function for monotonicity in both arguments, and differences a budgeted session's census peak against a zero-budget floor session's. Its `tests.rs` runs `Local` and a row-store reference backend (`Materializing`) whose process-global knobs give each check a `should_panic` negative control.

The partition is 3565 lines: `conformance.rs` (19), `link.rs` (1082), `link/tests.rs` (1085), `backend.rs` (791), `backend/tests.rs` (588). `link/tests.rs` and `backend/tests.rs` are test code; `backend.rs` is test-only code compiled under `cfg(test)`; `link.rs` is production code shipped behind a feature.

Overall the code is in good shape, and the link suite in particular is a model of how this crate wants a check built: every liveness probe has a committed negative control asserted to fail as `Stalled`, every legal adversity asserts its adversity fired, the "What the suite cannot see" section states the negative space, and the maintainer comments explain why a branch means what it means. No false-fail path against a conforming link was found under the first three lenses, and no panic reachable from a conforming transport was found; the late correctness lens found one false-fail path that turns on a clause the contract does not state (conformance-37). The dominant issues are on the verification side of the backend suite: the census ceiling has no liveness floor, and for `Local` the stated budget resolves to the floor window, so the end-to-end check passes vacuously today while its testdoc says the budget binds; the `children` stream (the population the window prices per depth) and the `parent` Some/None clause the `Backend` docs say this suite convicts are unchecked; and the `assemble` seam is driven only at the root over one run. The remainder is duplication (two near-identical independence probes, a fixture and a helper each with a twin in `crate::testing`), a handful of doc sentences that expired when the code they described moved, and register nits the owner's writing doctrine names.

The correctness lens ran after the other three (its reader hung during the main run and was rerun after finalization), its candidates went through the same refutation and history passes, and the six that survived are conformance-37 through conformance-42: the cancellation probe's unstated connect precondition and its lossy control's window-dependent verdict, a lock-scope nit in the `WindowedTx` fixture, and three unchecked obligations in the backend suite (the decode-slot padding the window delegates to `node_bytes`, the residency the re-tag is trusted to preserve, and the monotonicity grid's gaps); the witness pass had closed before they existed, so each is verified or assessed and none is demonstrated.


## Positives

- Every liveness check in the link suite has a committed negative control asserted to fail with the deterministic `Err(Quiescence::Stalled)` witness rather than a wall-clock timeout (mux coupling, coupled control duplex, lossy acceptor in both directions, capped connector, under-sized pooled window), and every legal-adversity fixture asserts its adversity fired (`reordered > 0`). The suite proves its own teeth instead of asserting them, and `never_binding_pooled_budget_conforms` pins the contract's pool bound from the passing side.
- Streams are classified in-band (`STALLED_TAG`/`LIVE_TAG`, the per-stream index byte), never by accept order, matching the contract's reordering freedom exactly; `reordering_acceptor_passes_independence` pins that the probes tolerate it.
- The stalled receivers in both independence shapes are returned from the receive future so they outlive the sender's `select` (link.rs:566-568, 673-675); without that hold a conforming pipe would report BrokenPipe and the probe would false-fail. The reasoning is written at the site.
- `probe_cancellation`'s ordering (connect all, signal, then write; collect the first delivery with a real accept before any poll-drop cycle) is argued step by step and makes the probe sound on real sockets as well as the in-memory link; `CANCEL_DROP_PATIENCE` expiring weakens rather than fails the probe, and the docs say so.
- `check_sessions` asserts snapshot equality rather than equal sizes and pins a liveness floor (`opened >= 1` per side), so the end-to-end check cannot degenerate into control-only traffic as `SESSION_PAYLOADS` or the tree's depth changes: the meters-need-liveness-floors doctrine applied to the suite itself, which is exactly what the backend suite's census ceiling lacks.
- The "What the suite cannot see" section (link.rs:28-55) states the black-box limits of each probe in terms a transport author can act on; this negative-space section is rare and well done.
- Constants carry the failure they prevent, not just their value: `STALLED_TAG` (in-band classification because acceptors may reorder), `CANCEL_DROP_PATIENCE` (why expiry weakens rather than fails), `SESSION_PAYLOADS` (which assertion keeps it from rotting), `CONTROL_DUPLEX_FILL` (what a pass proves only up to).
- Maintainer comments state the why at branches rather than the next line: why the pressure loop yields (link.rs:495-500), why the cancellation signal precedes the writes (861-866), why the demux lock is held across the routing send (link/tests.rs:230-233), why `ChargedNode` holds an `Option` (backend.rs:151-155), why the knobs are state rather than type parameters (backend/tests.rs:65-72, the rejected alternative named).
- `Measure`'s doc closes the cheapest-passing-artifact path in one sentence: "it must not consult the cost function it validates" (backend.rs:80-81).
- The `Charged` ledger is exactly balanced across wrap, clone, `into_inner`, drop, `erase`, and `assume`, with the erase/assume peak-neutrality argument stated where it holds (backend.rs:285-288). `check_assembled` prices at `run.leaves.min(FAN)` and justifies the cap by monotonicity in fan, while `node_bytes_monotone` sweeps that property before any session runs: the argument and its premise are both enforced.
- The `Knob` design (one backend type shared by accurate and skewed variants, knobs resting at their accurate value, a guard that holds `SERIAL` and restores on drop, a lock that clears poison from `should_panic` tests) makes plain `cargo test` sound, not merely nextest. `leaf_underpricing_fails_at_construction` is non-vacuous by construction: an empty ledger would panic with a message the `should_panic(expected)` does not match.
- `yield_once` keeps the whole suite runtime-agnostic, which is what lets the crate's own tests run it under `run_to_quiescence` and turn a hang into a deterministic `Stalled` verdict.
- No panic in `src/conformance/link.rs` is reachable by a transport that satisfies the contract as written: every `expect` on a connect, accept, write, flush, or read names the clause a violation breaches; the two internal `expect`s (`every slot was filled`, `the sending half signals after connecting`) are structurally unreachable; and `probe_concurrency` looks an in-band index up with `held.get_mut(..).expect(..)` (link.rs:777-779) rather than indexing, so an out-of-range byte fails by name.
- Every early `Done` drop the probes perform (`let (mut rx, _) = accept`, `drop((tx, done))`) is the contract's abort case, and `probe_completed_streams` (link.rs:383-420) is the one place completion is exercised, reading exactly the payload and handing the halves back without probing for end-of-stream: the two legal endings the Completion clause distinguishes are kept distinct in the suite.
- The fixtures are cancel-safe on the clause they test around: `ReversingAcceptor` keeps its batch in `self.held` (link/tests.rs:50-57), so a dropped `accept` future loses nothing, and `MuxAcceptor` releases the demux lock before awaiting the pump (link/tests.rs:437-457). A fixture that violated the cancellation clause while probing independence would confound two clauses. `Serialized::drop` (backend/tests.rs:45-49) drains the violation ledger, so a `should_panic` test that leaves violations behind cannot make its successor fail by name for a lie it never told.

## Open questions for Finch

- Does `MATERIALIZING_BUDGET` (4 MiB) widen the window above the floor at the suite's corpus scale? The flat pre-charge under `Materializing` pricing is `17 × 257 × (node_bytes(0, version_bound) + 40)` with `node_bytes(0, vb) = 32 + 64 + vb`; a rough estimate puts it near 850 KiB, leaving room, but only the floor proposed in conformance-28 says so mechanically. Recommendation: land the floor first and let it answer.
- conformance-7 reopens f5039abb's ruling on the eightfold signatures. My recommendation is the non-breaking loosening (drop `Send` and the unused control-half bounds from the focused checks) plus a one-line comment stating why the shape is otherwise kept; the sealed-trait collapse trades legibility for a one-implementor abstraction, which the doctrine also flags.
- conformance-18 reopens f5039abb's ruling on the `ReversingAcceptor`/`ReorderingAcceptor` pair. Recommendation: unify on `testing::reorder_accepts` if `reordered > 0` and the five `Stalled` verdicts hold under the patient wait; otherwise keep two and have transport.rs's comment state the actual behavioral reason rather than the feature-isolation one.
- conformance-1: ship a `conformance::bookmark` suite, or narrow the module doc's "each caller-implementable boundary" sentence? Recommendation: ship it; the `store` contract's commit-iff-`Ok` clause is exactly the kind of thing a deployment gets wrong over a file or KV store, and the crate names the consequence as unspecified corruption.
- conformance-19, conformance-22, conformance-32: three register rulings (em-dashes in `//` comments, "seam", "honest"/"lying") that are crate-wide or fixture-wide. Recommendation: rule once each; add a `tools/` lint for the dash convention since nothing enforces it today.
- Does `check_control` (sequential ping-pong) earn its place beside `check_control_duplex`? No link that passes the duplex probe and fails the sequential one was constructed; its remaining value is a legible first failure (looped-back halves) before the 32 KiB fill. Recommendation: keep it, and give it the negative control conformance-20 asks for, which is the one thing that would justify it.
- conformance-12: qualify the concurrency docs, or strengthen the probe so elders always backpressure? Recommendation: qualify the docs now (no behavior change); consider the stronger probe as a separate owner decision, since it changes what a pass certifies for every downstream transport.
- conformance-37: does the protocol's deadlock-freedom argument assume `Connector::connect` completes independently of the peer polling `accept`? If yes, the contract in src/link.rs should say so and the cancellation probe's precondition follows from it; if no (the protocol's acceptor loop at streams.rs:694-706 is polled continuously beside the session, and the memory link's backlog comment says the bound never binds), the probe requires more of a transport than the protocol does. Recommendation: state the precondition in the suite's requirements and in `check_accept_cancellation`'s rustdoc, and promote it to a clause only if transports are to be held to what the in-tree instantiations already provide.
- conformance-38: should every link negative control run at both tested capacities (8 KiB and one byte), the way the passing side does in `memory_link_conforms` and `one_byte_windows_conform`? The lossy control's verdict differs by capacity; whether the mux, coupled, capped, and pooled fixtures do is unknown, and a loop over `[MEMORY_STREAM_CAPACITY, 1]` in each control would answer it mechanically. Recommendation: yes; the two capacities are already the suite's own convention for the passing side.
- conformance-40: is the census meant to model slot residency, or node values only? `window.rs` charges `node_bytes + FAN_SLOT_BYTES` per decode slot and delegates alignment padding to the backend's price; the suite measures node values only. Recommendation: fold the slot excess into the leaf comparison, since the obligation is stated on `node_bytes` and the suite is where `node_bytes` is held to account.
- conformance-41: may a backend's `Erased` representation differ in residency from its typed one? The `Backend` trait's clause (streaming/backend.rs:57-60) says the re-tag does not change the value. Recommendation: hold the suite to the clause by measuring after `assume`; `erase` stays trusted until `Measure` gains a method over `Erased`, and the trust is stated at the site.
- Should `conformance::backend`'s module doc gain a "What the suite cannot see" section like `conformance::link`'s (link.rs:28-55)? conformance-40, conformance-41, and conformance-42 each name a limit the backend suite neither checks nor admits; whichever of them is not closed by measurement lands a one-line admission there. Recommendation: add the section in the same commit that closes or admits each.

## Dropped

- Candidate [4] "Testdoc claims drops settle the ledger, but the test observes only the peak": construction was wrong (a no-op `Drop` does fail the test, at the second pair's assertion); the accurate version is conformance-35 (from [18]).
- Candidate [13] "Census budget is asserted to bind in the testdocs but check has no liveness floor": folded into conformance-28, which carries the constructed instance and the dated expiry.
- Candidates [35], [47] dead `Charged` binding: duplicates of conformance-36 (from [3]).
- Candidates [36], [43] manual `Clone` impls: duplicates of conformance-15 (from [9]).
- Candidate [46] vestigial braces: duplicate of conformance-21 (from [8]).
- Candidate [41] eight-bound signatures: merged into conformance-7 with [10]; its verified non-breaking alternative is the recommended resolution.
- Candidate [26] visibility stated three times: merged into conformance-3 with [11] and [12]; its point that the public doc names an unreachable suite is kept.
- Candidate [12] `pub(crate)` wider than use: merged into conformance-3 as "state the placeholder intent", since history records the intent (goes `pub` with the storage boundary) as deliberate and holding.
- Candidate [32] bulk seams' structural clauses: merged into conformance-24 with [29] as one pattern (clauses the `Backend` docs claim convicted but the suite does not check), with the refutation's reframe applied (leaves' doc claims only encoder enforcement).
- Candidate [42] `CONTROL_DUPLEX_FILL` relation held by prose: merged into conformance-5 as site (a), where it becomes the const-assert resolution.
- Candidate [38] `node_bytes_monotone` bounds: subsumed by conformance-27 (from [44]), which covers all five `Measure + Clone` sites.
- Refutation new nit 1 (`check(Local, 64 * 1024)` inline magic number): folded into conformance-28's resolution (name `LOCAL_BUDGET` and size it above the flat term).
- Refutation new item 3 (`check_sessions` has no negative control): folded into conformance-20.
- Refutation new item 4 (em-dash convention breached crate-wide): folded into conformance-19 as its scope and owner-gating.
- Correctness lens open question on `Charged::assemble` discharging leaves on entry and charging the node on exit (a backend's run buffer invisible between them): not a finding; the module's stated accounting premise (the fans' flat pre-charge covers assembly transients) answers it, and the seam is not exercised in-session today (conformance-31). Recorded here so it is not lost.
- Correctness lens open question on `check_control`'s "equal-length payloads" sentence: nothing in the code depends on the equality (each side reads its own expected length); below the bar as a standalone finding, noted under open questions on whether `check_control` earns its place.
- Structure lens open question on `Charged` deriving `Copy` and `Default` unused: harmless mirrors of `Local`'s derives; below the bar.
- Late correctness lens: the refutation pass's new item (`let charged = Charged::<Local>::new(Local); let _ = &charged;` at backend/tests.rs:572-573 constructs a value used nowhere): duplicate of conformance-36.
- Late correctness lens: the refutation pass's new item (the two never-completing `select` arms at link.rs:522 and 632-634 are written in different idioms): already covered by conformance-9 (the drift between the two probes) and conformance-10 (the `contract:` prefix on the `unreachable!`).
- Late correctness lens: the refutation pass's new item (`REFERENCE_SLOT_BYTES` at window.rs:139-156 carries the same padding clause as `FAN_SLOT_BYTES`): folded into conformance-40's resolution, which covers both slot constants.
- Late correctness lens: no candidate was refuted, by reading or by construction; the witness pass had closed before the lens reran, and none of the six survivors was built and run, so none carries `demonstrated` provenance. The constructions each entry states remain to be run.

<!-- source: final/link.md -->
# Partition link: The Link transport contract, the in-memory link, erased links, and the routed (TCP) link

## Partition summary

This partition is the crate's transport boundary. `src/link.rs` states the six-clause contract every transport must satisfy (control duplex, per-stream independence, receiver-paced flow control at any positive capacity, `STREAM_COUNT` concurrency, completion through `Done`, tolerance of dropped accepts), defines the `Connector` and `Acceptor` traits, the `Link` bundle with its sealed `SessionState` (an epoch counter and a poison latch), the `LinkParts` decoration path, and the in-memory reference instantiation (`memory`, `MemoryConnector`, `MemoryAcceptor`). `src/link/erased.rs` is the monomorphization funnel: a `Bundle<H>` keeps a stream half together with its `Done` behind `TxDyn`/`RxDyn`, `DynConnector` erases behind an `Arc`, and `DynAcceptor<'a>` erases behind a `&mut dyn`. `src/link/routed/` adapts accept/connect transports (TCP and everything shaped like it) onto the contract by giving every link stream its own connection: `header.rs` is the 28-byte connect header and the `Addr` boundary with its stock `SocketAddr` instantiation, `router.rs` the per-endpoint listener loop with its routing table, `Registration` drop guard, and count-bounded `Abortable` header reads, `stream.rs` the per-link `StreamConnector`/`StreamAcceptor`, and `endpoint.rs` the `Endpoint`, `Incoming`, `Config`, and error types.

I read all 3204 lines of the partition with line numbers (production: `link.rs` 630, `erased.rs` 163, `routed.rs` 288, `endpoint.rs` 302, `header.rs` 321, `router.rs` 281, `stream.rs` 101; test code: `src/link/tests.rs` 180, `src/link/routed/header/tests.rs` 195, `src/link/routed/tests.rs` 743), plus the neighbors the findings rest on (`peer/gossip.rs` erasure aliases and `for_session` callers, `proxy/start.rs`'s `open`, `remote/streams.rs`'s `AcceptDriver::new` and its test callers, `codec/signal.rs`, `conformance/link/tests.rs`, `tests/common/routed_tcp.rs`, `tests/routed_link.rs`, `codec/encode/async_io.rs`). No cargo, just, or test command was run for this report; every "verified" item below is a grep or a read, and the two constructions were traced, not executed, when it was finalized. The witness pass afterwards ran link-28's memory-network construction: under `pending_headers: 1`, one stalled arrival evicted the pooled connection and the next stream open on a healthy link failed with `BrokenPipe` (`witness/results.md`); the TCP-pool hang variant and the four-peer figure under the default `Config` stay assessed by tracing.

The code is in good shape. The contract states goal beside mechanism for every clause; every `expect`/`unreachable!` in production code is a one-line proof from a check on the preceding lines; cancellation of `connect`, `accept`, `Incoming::accept`, and the router's `select!` arms is safe by construction; the poison latch is set before any wire traffic and cleared only by the funnels; routing revocation falls out of ownership (`Registration`'s `Drop`) with no teardown API; the router's no-await discipline is structural (`try_send`, `try_reserve`, abortable per-connection futures). Test hygiene is strong: every test carries an invariant docstring, the header parser has an every-prefix truncation sweep and a proptest, and the routed suite is shaped as a negative control for each failure mode the module docs list. I found no vestige of the V1 protocol or BLAKE3 here, and the one candidate vestige (the blanket `Acceptor for &mut A`) turned out to have five in-crate consumers.

Three items rise above nits and lows. The routed adapter's single `pending_headers` bound treats two populations with opposite eviction preferences as one, so a pooling `Dial`'s already-admitted idle connections are evicted before a fresh stalled header, and the default is sized for the non-pooling case: a liveness failure reachable by conforming peers at four pooling peers with the default `Config`, which the code's own docs name as a sizing duty rather than closing structurally (link-28). The routed TCP example and the `Conn` docs never mention `TCP_NODELAY`, and the routed shape (unidirectional connections, one to three `write_all` pieces per frame) is the canonical Nagle-plus-delayed-ACK stall (link-14). And the routing table has no type of its own, so the token-claim invariant is spelled twice, two `#[allow(clippy::type_complexity)]` stand in for a newtype, and a maintainer comment's premise is false as written (link-26). The rest are documentation accuracy refinements, dialect, and small API-surface gaps, most owner-gated.


## Positives

- The contract section (link.rs:24-68) states, for every clause, the mechanism, the failure it precludes, and the concrete carrier shape that would violate it (half-duplex turn protocols, close-credited pools, buffered writers), so an implementer knows what to build without reading the protocol; and every clause has a conformance check with a committed negative control.
- Routing revocation is ownership, not protocol: `Registration`'s `Drop` (router.rs:90-94), held by `StreamAcceptor` (stream.rs:66-70), means a dropped link stops routing at that instant with no teardown API, on every early return of `deliver` too; `dropping_a_link_revokes_its_token` pins exactly that.
- `Done<Half>` (link.rs:171-199) is one small mechanism that lets single-use transports write `Done::discard()` and recycling transports hand the connection back, and it composes through the erasure layer without a second concept: `Bundle<H>` keeps the concrete half and its `Done` together so completion through the box reaches the transport's own recovery hook (erased.rs:28-87).
- The router's three disciplines are stated once (router.rs:9-27) and each is visible in the code as written: `try_send` for delivery, `try_reserve` for the backlog, `try_send` for returns, the table mutex never held across an await, and all per-connection I/O inside abortable futures evicted oldest-first by count.
- Every `expect`/`unreachable!` in the partition's production code argues its unreachability from a check on the preceding lines: "length checked above" (header.rs:214-215), "prefix layout fixes the token width" (header.rs:299), "the loop ends at the first vacancy" (router.rs:110).
- `SessionState` is sealed exactly right for a value wrappers must carry: `Copy`, private fields, public getters, `pub(crate)` mutators (link.rs:345-407); the poison latch is set in `begin` before any wire traffic and cleared only by `finish`; and `LinkParts::session`'s doc (link.rs:491-498) explains the hazard of substituting a stale copy.
- The explicit-dispatch comment at erased.rs:158-161 documents a non-obvious recursion trap (method syntax would resolve to the blanket `AcceptDyn` impl) at the one site where a later reader would otherwise reintroduce it.
- Hazard sections are complete where they exist: `Endpoint::new`'s `# Errors` names every arm with its trigger (endpoint.rs:175-190); `Listen`'s `# Cancel safety` matches the router's `select!` re-creation exactly (routed.rs:277-281); `memory_with_capacity`'s `# Panics` carries the one-line reason (link.rs:556-558).
- The advertised name is encoded and length-validated once at construction (endpoint.rs:159-161, 204-207); `stream_header` returns a stack array (header.rs:260-267), so a routed stream open allocates nothing in the adapter itself.
- Test hygiene is strong: every test carries an invariant docstring; `truncation_is_rejected` sweeps every prefix length rather than sampling one; `socket_addr_roundtrips` is a proptest that states its canonicalization caveat rather than overclaiming equality; the routed tests run under `pollster` with the routers as a `select` arm so a router that resolves is itself a failure (routed/tests.rs:38-48), and they are shaped as negative controls for every failure mode the module docs list.
- No vestige of the V1 protocol, BLAKE3, or the height/item erasure work exists in this partition; the link layer is protocol-agnostic and reads as written today.

## Open questions for Finch

- Is a pooling `Dial` a first-class deployment shape for the routed adapter, or an opt-in optimization whose sizing duty the caller accepts? This decides link-28's shape. Recommendation: first-class; the design record already names `Dial` as the pooling boundary, and a recovered-connection budget applied before `READY` is a small change that removes a whole failure class.
- Should the router distinguish a full backlog from a foreign listener with a reason byte in place of the single `ACK` (link-18)? Pre-release the wire is free to change and the byte costs one octet per link. Recommendation: only if a caller wants different retry policy for the two; otherwise document the conflation as deliberate.
- Should `READY` (and `ACK`) become public constants so pooling implementers can validate the byte, or should the public contract state that any single byte is the ready signal (link-15)? Recommendation: state the unspecified-value contract; do not widen the surface.
- Which site is the statement of record for the session promise's three qualified exceptions: `Link#what-a-session-promises` (the anchor every cross-reference targets) or `Rumors::gossip`? 3f33afab deliberately states the third at every reachable surface. Recommendation: keep `Link` canonical, trim `Rumors::gossip` to a one-line summary plus the anchor, and drop the "stated where they arise" clause either way (link-1 handles the clause).
- The "+0.7 GiB of rustc peak memory" figure (erased.rs:6, conformance/backend/tests.rs:67) was ruled inline (punch list item 3) and was measured on stable 1.96.1; the pin is now 1.97.1 and no meter re-measures it. Recommendation: soften to the structural claim ("a second full protocol tower per instantiation") in rustdoc and keep the number in the decision record, or add a memwatch-style recipe that re-measures it on toolchain bumps.
- Keep `LinkParts` for representation freedom, or publish `Link`'s fields and dissolve the twin (link-9)? Recommendation: dissolve, pre-release, unless a representation change for `Link` is on the horizon; either way, state the reason in `Link`'s doc.
- `NonZeroUsize` for `Config`'s counts and `memory_with_capacity` (link-20)? Recommendation: leave as is, consistent with the existing `usize`-plus-check shape and R79's spirit.
- A crate-wide `#[non_exhaustive]` ruling for public enums (link-17): which taxonomies are open? Recommendation: rule once, pre-release, and apply mechanically.
- Should `MEMORY_STREAM_CAPACITY` be public so docs can cite it, or is the fixed size an implementation detail the docs should stop quoting (link-1)? Recommendation: stop quoting it.
- `starved_pool_degrades_latency_not_liveness`'s doc (conformance/link/tests.rs:1026-1027, another partition) points at link.rs:107-113 by description ("the link docs' measured-tolerance sentence"); if link-2's restatement lands, that pointer should name what it pins in its own words.

## Dropped

- Blanket `impl Acceptor for &mut A` has no use site (structure 3): refuted. `AcceptDriver::new` takes `acceptor: A` by value with `A: Acceptor` (streams.rs:660-668) and `streams/tests.rs` passes `&mut b.acceptor` at lines 85, 161, 209, 398, 432, which type-check only through this impl; the history pass's "no consumer" grep missed those sites.
- The scoped-IPv6 refusal rationale is written out at three public sites (prose 25): below the bar. One full statement (header.rs:180-189) plus two abbreviated pointers at the altitude of an error variant and a constructor's `# Errors`, both of which must keep every arm named per b23d19c0.
- `Endpoint::link` over a recycled connection is never exercised (correctness 30b): below the bar. The router-side path after `READY` is identical for `LINK` and `STREAM` headers, so the shape adds little over the existing `STREAM` coverage.
- The "+0.7 GiB" figure at a declaration site (prose 16, figure half): recorded owner ruling (punch list item 3, executed in 33bd7d56); moved to open questions with the toolchain-drift observation.
- "still" at link.rs:150 as a temporal marker (prose 18, one item): misread; "The core still ships no network code" is the logical "nonetheless".
- "item-granular" as undefined jargon (prose 22, one item): deliberate owner vocabulary (5048a368 added the buffering sentence with its rationale; "item" is the CBOR wire's established term).
- Manual `Clone` impls (perfapi 43): duplicate of link-19.
- `Config` zero bounds as runtime validation (perfapi 39): duplicate of link-20.
- "never allocates" testdoc (correctness 32): duplicate of link-12.
- Dialect tells in two public summaries (perfapi 46): duplicate of link-13.
- `STREAM_COUNT` derivation pinned literal-to-literal (correctness 33): merged into link-3.
- `Endpoint::link` under-enumerates `Rejected` (correctness 29b) and `Rejected` conflates busy with foreign (perfapi 38): merged into link-18.
- `entries` doc premise false for the `LINK` arm (prose 23) and `register`'s `Option::take().expect` dance (perfapi 44): subsumed by link-26's `Table` newtype.
- `Connector::connect` lacks `# Cancel safety` (perfapi 41) and `Endpoint::link` lacks it (correctness 29a): merged into link-4.
- `PREFIX_LEN` visibility (structure 8): merged into link-22.
- Control-half erasure aliases live in gossip.rs (structure 10) and the erasure argument written twice (prose 16, duplication half): merged into link-10.
- Encoding attributed to the router at the test site (prose 14, second site): folded into link-23.
- Module-doc "cheap", the blank line at 376/377 (prose 22), "8 KiB" (prose 20), and "stated where they arise" (prose 24 residue): batched into link-1.

<!-- source: final/materialized.md -->
# Partition materialized: The materialized (fixed-memory) mirror: progress, transcript, unknown pruning, work queues, levels, resolver, assembly

## Partition summary

The `materialized` module is the in-process participant of the streaming mirror. `materialized.rs` holds the type-level phase schedule (`Handshaking<B, Start | Connecting | Connected>` into `Descending<B, H>` into `Completing`), the `SupplyLedger`, the `Query`/`Resolution`/`Resolve` item vocabulary, the `yield_resolve_query!` macro that fixes the publication order the deadlock-freedom argument rests on, and the initiator's terminal `absorb` loop. `work.rs` accumulates every independently runnable pump as a boxed task and drives them through `tasks::complete`; `work/levels.rs` holds the five walk bodies (initiator opening, responder opening, internal, leaf-parent, leaf), `work/answer.rs` the three merge-joins that answer one query, `work/resolver.rs` the per-reply reaction loop that classifies counterparty faults into `Violation`s, `work/assembly.rs` the positional upward reassembly, and `work/queues.rs` one named constructor per channel edge with its capacity argument. `unknown.rs` is the deletion-honoring prune, the streaming twin of `traverse::unknown`. `progress.rs` and `transcript.rs` are `#[cfg(test)]` thread-local recorders of publication order and payload-erased wire order that the cross-peer suites validate. `error.rs` defines `Error` and `Violation`, the only two items here that reach the crate's users (re-exported as `MaterializedError` and `MaterializedViolation` through `crate::error`).

I read all 18 files, 4511 lines. Test code: `progress.rs` (a `#[cfg(test)]` module, 535 lines), `progress/tests.rs` (198), `tests.rs` (166), `transcript.rs` (`#[cfg(test)]`, 112), `unknown/tests.rs` (84), `work/tests.rs` (270), and `work/tests/violations.rs` (352), 1717 lines in all; the remaining 2794 lines are production.

The production code is in good shape. I found no in-model correctness bug: every panic is guarded by structure the counterparty cannot influence, the stage heights line up, the stream-termination chain has no cycle, and every peer-controlled datum reaches the walk through a merge-join or a peekable fan. The deadlock-freedom argument in the module doc is precise and is enforced structurally by the macro and re-checked by the test-only trace; the channel constructors each carry their own capacity argument; the height erasure is done with a thin typed boundary. The dominant issues are on the verification side and in prose. The one high finding is that the leaf-height deletion verdict in `unknown` has no committed test that fails when it is inverted: the handoff note from 2026-08-21 records the surviving mutant, no follow-up has landed, and by reading I can say why every fixture, including the module's own oracle proptest, misses the arm. Below that sit a medium simplification (the initiator's terminal state hand-rolls `tasks::complete`), a medium legibility fix in `internal_walk`'s opening hand-off, two medium prose findings (roster IDs and adjudication narrative in the test-only recorders, which the hard rules forbid; public `Violation` docs that name reaction kinds the user cannot reach), and a long tail of low and nit items: duplicated handshake derivations, a vestigial version copy, redundant `Sync` bounds and dead clippy attributes, one ghost reference, one vocabulary collision, and a handful of default-dialect tells.


## Positives

- The deadlock-freedom argument (materialized.rs:26-87) is a model maintainer document: it states the two ordering invariants, derives why one slot suffices, names the independence premise and exactly where it is supplied (the link contract) and refuted (the mux fixture), and isolates the one exception (the assembly fan queue) with its reason. `yield_resolve_query!` then makes the order a structural property of every call site by keeping wire, resolution, and dependent work in one expansion with the `#[cfg(test)]` trace hooks inside it, so the test-only instrument cannot drift from the production order; the remote proxy's `yield_reply_scopes!` mirrors the shape.
- `queues.rs` gives every channel constructor its own cardinality or flow argument, distinguishes the correctness floor (`assembly_level_returns` at `FAN`) from the amortization (`terminal_leaf_resolutions`), and ties the floor to a demonstrating test that exists and is exercised from both sides (`underbuffered_mirror_stalls`, capacity.rs:26; a stall at 253 and completion at 254).
- The erasure boundary is disciplined and cheap: walk bodies take `Replies<E>` and instantiate once per backend, the typed re-tags are `Work::respond`'s exit and the two fixed-height root re-tags (levels.rs:143-146, 257-260), each commented, and the prefix length is the single runtime height witness so coordinate and height cannot drift.
- `Resolver` isolates the counterparty-fault taxonomy in one 130-line type, and `injected_fault_reports_exact_violation` drives every scriptable fault through all 32 walk heights under arbitrary channel schedules asserting the exact public `Violation`.
- `SupplyLedger::charge` handles the wrapped-counter case explicitly, uses one `Relaxed` `fetch_add` with a post-check that is race-correct for two ingestion sites, and returns a vocabulary-neutral error so the walk and the wire decoder each render the overdraw in their own terms; the doc says exactly why two instruments exist.
- Every peer-controlled datum reaches the walk through a merge-join or a peekable fan: no indexing, no arithmetic on wire values except the ledger's checked add. Every panic I traced is guarded by structure the counterparty cannot influence.
- `unknown.rs` states the recursion's depth bound in the module doc (the prefix's remaining height), satisfying the input-controlled-depth rule with an argument rather than a guard, and the span classification prunes whole subtrees without descending; the streaming prune is differential-tested against `traverse::unknown`.
- `transcript.rs` captures the wire transcript at the single funnel every response stream passes through (`pump`) and states the causal-consistency property of the capture point.
- Every test in the partition has a doc comment stating its invariant, and every one I traced against its body (all of progress/tests.rs, materialized/tests.rs, work/tests.rs, violations.rs, unknown/tests.rs) is accurate, with one exception recorded as materialized-22. Several state the deadlock the check prevents, not merely the panic expected (progress/tests.rs:44-47, 60-66, 78-83).
- Branch comments carry the why rather than the next line: levels.rs:94-96, 118-119, 219-223, 232-233, 339-340, 386-390; assembly.rs:81-83; materialized.rs:838-840; answer.rs:149-151.

## Open questions for Finch

- `assert_parent_early` (materialized-22): keep it as a design-space record fed a real captured trace, or dissolve it and let `formal/lean` (`Sched.deadlock_free_d5`) carry the record? My recommendation: dissolve; the model already records the rejected corner, and test code performing a design-doc function is the circular-justification tell.
- `MaterializedError` derives and exhaustiveness (materialized-17): derive `Clone`/`PartialEq`/`Eq` on both inner error enums and decide `#[non_exhaustive]` for both together, or drop the dead `Clone` from `mirror::Error`? My recommendation: derive on both and keep `MaterializedError` exhaustive with the two-variant partition stated in its doc.
- `DEFAULT_TARGET_MESSAGE_SIZE` (materialized-10): relocate to `message.rs` keeping `usize`, or also retype to `u64` to match the greeting field (a public API change)? My recommendation: relocate and retype pre-release, since `DEFAULT_PAYLOAD_DEPTH_LIMIT` already sets the precedent of a default typed as its field.
- Window-stall observability (materialized-2): is a `SessionStats::window_stalls` counter wanted, given that stalls are off-model under uniform hashing and the counter would be a diagnostic for misconfigured budgets and off-model key distributions? My recommendation: yes, as zero-versus-nonzero; it is the one readout that tells a user which way to move `sync_memory_budget`.
- `unreachable_pub` (materialized-6): the partition's `pub` items inside the private `tree` module follow the crate-wide idiom; enabling the lint is a crate-wide decision. My recommendation: leave the idiom and only make the three item types' fields consistent.
- Em-dashes in `//` comments (materialized-38): sweep the 116 crate-wide sites or record a tolerance? My recommendation: sweep once, mechanically, in a dedicated commit.
- A walk-side allocation meter (materialized-30): wanted at all? Without it the vector-capacity reservations should stay recorded candidates rather than land.
- The `#[cfg(test)] progress::` instrumentation appears at about fifteen production call sites plus `trace_id` plumbing. A `#[cfg(not(test))]` no-op stub module would remove the attributes from the call sites at the cost of a zero-sized `trace_id` in release builds. The current form guarantees zero release cost; the trade is taste, and worth a ruling before anyone touches it.

## Dropped

- The prune recursion boxes one future per visited node (perfapi candidate 54): deliberate and documented; bf1a5b4b chose one prefix-guided recursion for one instantiation per backend, measured at -40.5% cumulative llvm-lines, and unknown.rs:17-23 states the rationale; the lens itself proposed no change until a meter exists.
- Greeting construction duplicated in `connect` and `accept` (prose 37, perfapi 57): duplicates of materialized-11, which composes `accept` from the two existing transitions rather than hoisting a helper.
- A third body of the listing derivation the doc calls single (perfapi 56): duplicate of materialized-31; its claim that `fan_listing`'s doc is weakened is refuted (the doc is scoped to the two positionally-paired listings and is accurate).
- Hand-rolled left-only merge (perfapi 55): duplicate of materialized-33.
- `known` and `contained` (correctness 44): duplicate of materialized-26.
- Opaque roster IDs (structure 13, correctness 45, perfapi 61): duplicates of materialized-20; the correctness lens's claim that `B5` resolves to nothing is refuted (it is a named Lean axiom, citable by name).
- Typed tower ghost (prose 25, perfapi 50): duplicates of materialized-25; the hand-maintained-count charge in 25 is folded in.
- The terminal leg denominates one absorbed leaf two ways (correctness 46): duplicate of materialized-15.
- Inconsistent field visibility (structure 19): duplicate of materialized-6.
- Terminal absorb reports extra reactions as `UnfinishedReply` (correctness 40) and opening early supplies skip structural checks (correctness 42): merged into materialized-14 as one pattern (the two legs outside the `Resolver`).
- Terminal absorb clones the supplied leaf (perfapi 53): folded into materialized-14's resolution (matching over the owned `Vec` removes the clone).
- `unwrap_or_default()` undocumented (prose 31): folded into materialized-34; the refutation's reframe (the comment is a benign-default argument, not a strandable path) is adopted there.
- `Descending` encodes a three-state hand-off as two `Option`s (structure 2): merged into materialized-34.
- `#[cfg(test)]` inside a `#[cfg(test)]` module (structure 15): merged into materialized-8 with the dead per-item `type_complexity` allows.
- `Resolver::react`'s return contract undocumented (prose 27): merged into materialized-36 (the named struct carries the doc); the resolver's other undocumented methods and missing module doc moved to materialized-29.
- One invariant two names / `d6` glossed two ways (prose 33), "Seven checks:" (prose 34), `assert_parent_last` cites by quotation (prose 35): merged into materialized-19.
- `initiator()`/`responder()` prelude (structure 4) and `our_version` duplicating `root.ceiling` (structure 5): merged into materialized-11.
- "seam" as a default-dialect tell (prose 36, perfapi 61): withdrawn; the word is anchored in the crate at streaming.rs:19 ("the height-erased seam"). "honest" at tests.rs:156: withdrawn; it is the model's term of art ("authenticated-honest-peer"). Both removed from materialized-23's site list.
- Refutation's new item "connected fault suite cannot reach the terminal phase": folded into materialized-39 as corroboration.
- Refutation's new item "the module's own oracle uses hashed paths": folded into materialized-27.
- Refutation's new item "erased.rs duplicates the receiver-as-stream adapter": folded into materialized-16.
- Five-tuple returns of `initiator_level`/`responder_level` (structure open question): not raised as a finding; the comment at levels.rs:339-340 argues the arity is the dataflow, and a `Stage` struct is taste without a named cost.

<!-- source: final/mirror-common.md -->
# Partition mirror-common: Mirror protocol shared layers: CBOR, framing, handshake, party, the streaming module root, protocol phases, messages, driver, erasure, tasks, stats

## Partition summary

This partition is the session envelope beneath the streaming mirror and the plumbing the mirror's two implementors share. The envelope is four small modules under `src/tree/mirror/`: `cbor` (the canonical CBOR head grammar, a writer and two readers held together by round-trip proptests), `framing` (exact-read payload buffering, the memory policy every declared-length body read funnels through), `handshake` (the fixed 30-byte preamble and its cancel-safe receiver), and `party` (the trailing identity hand-off). The plumbing lives under `src/tree/mirror/streaming/`: `protocol` and `protocol/peer` spell the type-level phase schedule and the 15/14-round `Peer`/`Client`/`Server` chains; `message` is the wire vocabulary and the greeting; `driver` expands the schedule into a body and routes the first error; `erased` is the boundary between the height-typed schedule and the height-erased walk; `tasks` holds the completion helpers; `stats` is the per-session counters that surface publicly as `SessionStats`; and `streaming.rs` elects roles and runs the descent. `mirror.rs` is the module root with the `contained` predicate and the two-sided `Error<C, S>`.

I read all 18 files with line numbers, 3579 lines in total, of which 920 are test code (`cbor/tests.rs`, `framing/tests.rs`, `handshake/tests.rs`, `party/tests.rs`, `mirror/tests.rs`) plus the 36-line inline test block at the bottom of `driver.rs`. I also read the files the findings depend on outside the partition (the codec's `record_prefix` and `lone_record_spans`, the gossip call sites of `reconcile`, `error.rs`, `channel.rs`, `remote/streams.rs`, `.cargo/mutants.toml`, the CBOR-wire review packet and the V1-retirement note) and the pinned library sources behind the one correctness finding (tokio 1.52.3 `io/util/read_buf.rs`, bytes 1.11.1 `BufMut for Vec<u8>`, std 1.97.1 `raw_vec`).

The code is in good shape. Every wire-facing parser is total over its input; every `expect` and `unreachable!` carries a one-line proof or is unreachable by type; the ingress suites are the right instruments (an exhaustive intent-byte sweep, a cut at every preamble prefix, a field-by-field oracle proptest, a chunked-versus-whole-read differential, a canonicality oracle on accepted hand-off bodies). The dominant issues are small and of two kinds. First, one real contract defect: `framing::resume_payload` promises `read_payload`'s exactness for any caller buffer, but `read_buf` fills all spare capacity, so a prefix buffer with capacity beyond `len` over-reads the transport; the single production caller escapes by one byte through std's minimum `Vec` capacity. Second, residue of three recent changes that the retiring commits did not sweep: the V1 retirement left a two-item list with one bullet in public `SessionStats` docs, "we selected" in a Display string, a `[u8; 6]` that used to be `LEGACY_MAGIC.len()`, and `LengthOverflow` in a module that no longer writes a length header; the move of `mirror_connected` to `driver.rs` left a comment naming the old file; and the re-denomination of `PayloadDepthLimit` to recursion steps missed the greeting field. The one owner-gated item reopens a recorded ruling (R2, keep-and-document on the two defensive `PreambleDefect` variants) on new evidence: since the V1 retirement, `Staged::buf` is a `[u8; V2_PREAMBLE_LEN]`, so the typed signature that dissolves both variants is now free. The rest is legibility and idiom, batched as nits.


## Positives

- `cbor.rs`'s module doc argues the rejected alternative as mechanism (a general CBOR reader has no head-level incremental API and buffers past item boundaries), scopes the module to the head grammar alone, and names the round-trip tests as what holds writers and readers together. The tests do that: an inverse-plus-width-plus-untouched-trailing round trip, every wider spelling of every value rejected, a cut at every prefix leaving the input unconsumed, the `[0xd9, 0xd9, 0xf7]` literal pinned to `write_tag`, and an async-versus-slice differential. `HeadBytes` renders heads on the stack, and `tests/encode_alloc.rs` prices that at zero allocations.
- `framing.rs` states its memory policy as a contract (grow only as bytes arrive, doubling from one granule, clamped to `len`, exact consumption), turns it into a session-boundary argument (exact reads make a session boundary a stream position), and proves the growth policy differentially against a whole-read reference across arbitrary read schedules and cut points, with `chunk_boundary_cuts` shared with the codec suite so the boundary roster cannot drift. `tests/decode_alloc.rs` prices the policy with an allocator meter.
- The handshake tests are exemplary: `intent_byte_space_is_exhaustive` sweeps all 256 bytes and classifies each; `every_truncation_boundary_is_typed` cuts at every prefix and checks the reported counts; `arbitrary_preamble_decodes_by_the_oracle` is a field-by-field oracle proptest; `prefix_matches_the_writers` pins the flat `V2_PREFIX` literal to the head writers; `fragmented_exchange_is_symmetric` drives the exchange over a one-byte duplex. `Preamble::decode` diagnoses in a useful order, so a dialect skew reports as `VersionMismatch` carrying the remote's number, never as garbled fields.
- `party.rs` draws the `Error::HandOffTruncated` (the stream stopped) versus `HandOffDefect::Undecodable(Decode::Truncated)` (the body arrived whole, its content is short) distinction carefully in code, docs, and tests; `bytes_after_the_frame_stay_untouched` proves the exact-read contract the epilogue depends on; the body proptest checks that every accepted body re-encodes byte for byte (a canonicality oracle, not a no-panic check); and `decode_party` keeps the `Decode::Io` arm total, with the reason stated.
- `mirror.rs` names `contained` for exactly the partial-order pitfall (`!(a <= b)` is not "strictly above") and its test covers all three regimes, incomparable included.
- `driver.rs`: the manual `Clone` for `ErrorRoute<E, S>` is the right idiom (a derive would demand `E: Clone, S: Clone` the fields do not need); `race_session`'s biased select plus `try_recv` fallback preserves causal priority, and both behaviors have committed tests; `divert` parks after reporting so a producer failure can never masquerade as a completed phase.
- `erased.rs` derives the type-level height from the prefix's byte length so the coordinate and its witness cannot drift apart, keeps `at_height!`'s out-of-range arm unreachable by type (`ErasedPrefix` is an `ArrayVec<[u8; 32]>`), and its module doc says plainly what the types stop proving inside the walk and which suites catch it instead.
- `message.rs`: every `Greeting` field doc says why the field rides the greeting, the listing's cost is stated with its trade, and `initiates` carries a `# Panics` section stating the precondition that `descend`'s equality guard (streaming.rs:197) discharges. `protocol.rs`'s `Initiator` doc explains why the protocol never sends a root hash, and `Responder::Next`'s comment says why its bound is left loose.
- `stats.rs`'s public field docs follow one shape (the mechanism, where the count is taken, when it is zero, and for `disputed_scopes` why the two ends disagree); the `Relaxed` ordering is justified by the single post-completion read; and `tests/session_stats.rs` pins `bytes_sent`/`bytes_received` against an independent transport-level tally, so a misplaced counter would disagree with an oracle.
- `tasks.rs` is a model small module: five items, each used across the walk and the proxy, each with a one-line doc stating its invariant (`cancelled` parks so an error cannot be followed by a successful EOF).
- The two "Defensive-variant exemption" comments (handshake/tests.rs:283-290, party/tests.rs:51-57) say plainly what is untested and why; whatever mirror-common-10 decides, that habit is the right one.

## Open questions for Finch

- mirror-common-10 reopens your R2 ruling (keep-and-document on `NetworkTruncated` and `TrailingBytes`). The new fact since R2: `Staged::buf` is now `[u8; V2_PREAMBLE_LEN]`, so the typed `decode` signature that makes both arms structurally nonexistent is free, and the mutants policy you wrote orders that disposition. Recommendation: take it; the two variants are public surface no peer can produce, and their only test artifact is a comment explaining why they cannot be tested. If you keep them, the `encode -> [u8; V2_PREAMBLE_LEN]` half and the constant-name respelling (mirror-common-2) are still worth doing.
- `define_peer!` (peer.rs:12-106) exists to spell a 16-deep and a 15-deep nested bound whose depth is a function of `Root`'s height. The structure lens sketched height-indexed chain traits in the style `ReplyHeight` already uses (`trait InitiatorChain<B, H: Height>` with a base impl at `S<Z>` ending in `CompleteInitiator` and a step impl at `S<S<H>>`, dually for the responder), which would derive the chain length from the height types and delete the `_` roster. Nobody compiled it; the risk to measure is rustc's handling of a 16-deep associated-type recursion through impl selection versus the macro's fully expanded form. Recommendation: worth one afternoon as an experiment on a branch, abandon on compile-time or diagnostic-quality evidence, never on anticipated complexity.
- `race_session`'s `Ok` arm (driver.rs:79) returns the output without consulting `first_error`, while the `Err` arm prefers a same-poll routed error. The correctness lens could not construct an (`Ok`, routed) case, and the refutation pass agrees it is unreachable today because `divert` parks after reporting, so a routed error never lets its consumer see EOF. Nothing states that invariant at the arm. Recommendation: a one-line comment at the arm; no test, since the state is unreachable.
- `HandOffDefect::UnaddressableLength` (party.rs:32-37) is reachable only on 32-bit targets and carries a no-test exemption (party/tests.rs:51-57). `wasm32-unknown-unknown` is in `rust-toolchain.toml`. Is a 32-bit run of the party ingress suite planned, or is the exemption the intended permanent state? Recommendation: leave it as is unless the wasm target grows a test leg; the exemption says what it needs to.
- `SessionStats` counts bytes but not frames (stats.rs:99-132). An operator tuning `target_message_size` wants mean frame size against the target, which bytes alone cannot give; a `frames_sent`/`frames_received` pair would be counted at the codec's frame boundary (`CountedWrite`/`CountedRead` see writes, not frames, so it needs a hook one layer up) and is non-breaking under `#[non_exhaustive]`. Nothing in the current docs is wrong. Recommendation: only if you have the tuning question yourself; otherwise no.
- A gitignored build directory sits inside the source tree: `src/tree/mirror/streaming/target/doctest-nightly/` (dated Aug 17). The `doctest` recipe uses a relative `--target-dir target/doctest-nightly`, so something ran with that cwd. Harmless to the gate; worth deleting and noting which invocation produced it.

## Dropped

- "The `mirror!` schedule macro is V1-era generality with one remaining caller; inline it" (structure [2]): premise refuted by git (`alternating.rs` never used `mirror!`; the three call sites at aa22c2a2b were segments of one schedule), and the proposed straight-line `seq!` form is the driver's pre-aa22c2a2b shape, which the owner replaced deliberately for one-line-per-phase legibility; no new evidence. Converted to mirror-common-21 (state the hygiene rationale at the site).
- "One `_` per exchange round does not literally hold" (prose [24], sub-claim): refuted; with a round as one two-height descent, 15 `_` are the 15 descents between the initiator's 16 `Reply` nodes and 14 the responder's, matching driver.rs:164-168. The location error survives in mirror-common-33.
- "`Staged::fill` has neither a section nor a statement of cancel safety" (prose [27], second half): refuted; handshake.rs:244 and :264 both state it. The section form and return-arm inventory survive in mirror-common-4.
- "The pinned pairwise lemmas cite an artifact nothing in the tree names" (prose [29]): premise refuted; the lemmas are `before`'s `join_encoding_is_subadditive`/`meet_encoding_is_subadditive` proptests. Reframed to mirror-common-26 (cite by name), severity nit.
- "The cbor proptests' testdocs are inaccurate for excluding major 7" (prose [35], first half): refuted; no writer emits major 7, so "Every head a writer emits" is accurate. The unstated exclusion and the runtime builders survive in mirror-common-5.
- "'Tag' for an ITC bit in party/tests.rs:172-173" (prose [37], third clause): dropped; `before` itself calls the party codec's presence bits tags (crates/before/src/party.rs:122).
- "driver.rs and tasks.rs predate the edition bump" (perfapi [54], inference): wrong; the crate has been edition 2024 since 6b70ee9a3. The redundant import itself survives as mirror-common-18.
- "SessionStats counts bytes but not frames" (perfapi [59]): not a defect against the code; converted to an open question.
- "`payload.capacity() * 2` can wrap on 32-bit targets" (prose lens open question): dropped; a `Vec`'s capacity is at most `isize::MAX`, so `2 * capacity <= usize::MAX - 1` on every target and the multiplication cannot overflow.
- "Add a `race_session` test for a session that completes `Ok` in the same poll a route reported" (correctness [43], suggestion): not a gap; `divert` parks after `route.report`, so the state is unreachable. Kept as an open question about a comment.
- "`mirror::Error<C, S>` derives `Clone` but `MirrorError` is not `Clone`" (perfapi open question): below the bar; an inert derive on a generic type is harmless and costs nothing.
- Duplicates merged: [19], [41], [46] into mirror-common-35; [26], [39], [45] into mirror-common-10; [47] into mirror-common-17; [58] into mirror-common-6; [25], [43], [49] into mirror-common-22; [22] into mirror-common-12; [24], [42], [50] into mirror-common-33; [44] into mirror-common-3; [51] into mirror-common-7; [53] into mirror-common-11; [56] into mirror-common-24; [20], [38], [52] into mirror-common-8; [16] and [48] into mirror-common-16; [57] kept separate from [23] as mirror-common-31 because the fixes differ (rename versus document).
- Out of partition, noted for their owners: the item-level `type_complexity` allows under `streaming/**` (mirror-common-29's related sites); "mid-handshake" at error.rs:21 and :195 (mirror-common-9); the `selected` vocabulary at error.rs:14, :76, :179, :201 and protocol.rs:1 (mirror-common-12); the literal `16`s in network.rs (mirror-common-11); the parking.rs module doc's ≈1.1 MB / ≈2.2 MB figures, which disagree with message.rs:14-17's ≈1.8 MB / ≈3.5 MB while message.rs agrees with the pin.

<!-- source: final/remote-adapter-streams.md -->
# Partition remote-adapter-streams: The remote adapter (decode/encode/scope/errors) and the data-stream supply

## Partition summary

This partition is the two lowest layers of the streaming mirror's remote proxy, at commit 9e5784fb. `adapter/*` converts between the walk's height-erased `Reply<E>` (backend node handles, positional `Match`/`Query`, radix-keyed `Supply`, prefix omitted) and the wire's prefix-free frames. `encode.rs` renders one reply as a one-frame-lookahead stream of `Encoded` frames so the reply-ending `Flow::End` lands on the last frame, flattens each supplied node through `Backend::leaves` into byte-budgeted `LeafRun`s, and attaches each newly asked `Scope` to the frame whose successful write makes it publishable (`Encoded::write_with`). `decode.rs` reads exactly one reply (or the initiator's single opening-supply reply, incrementally, in `early_supplies`), recomputes every supplied leaf's path from its version, enforces scope containment, strict path order, strictly ascending run radices, and the peer's greeting-declared `max_version_bytes` and `set_len` in `SupplyRuns::observe` and the `SupplyLedger` charge, hands each leaf to `Leaf::leaf`, and streams leaves through a FAN-slot channel into `Backend::assemble` while retaining only a reply skeleton that `reify` fills afterwards. `scope.rs` is the retained question (parent prefix plus positional radices; the prefix's byte length is the height witness). `error.rs` is the typed rejection taxonomy. `streams.rs` binds the protocol's logical streams one to one onto a `Link`'s transport streams: `StreamSender` connects lazily on its first frame and writes a two-head CBOR label, `AcceptDriver` is the sole acceptor reader that validates labels and delivers streams into take-once oneshot claim slots, `StreamReceiver` claims lazily on first poll and yields frames until the `End::Stream` control, and every incoming failure is published to a one-slot `ErrorRoute` and parked, with a separate deposit slot for the acceptor's own transport failure that the session terminal consumes.

I read all seven files in full with line numbers, 2542 lines. `streams/tests.rs` (571 lines) is the test code; `decode.rs` also carries the `#[cfg(test)]` `fan_probe` module (lines 550-598) and `encode.rs` a `#[cfg(test)]` `Encoded::into_parts`. The lens reports, refutation pass, and history pass were read whole; every anchor and excerpt below was re-checked against the files at this commit, and the related sites outside the partition (proxy/work/encode.rs, pump.rs, work.rs, prefix.rs, erased.rs, backend.rs, local.rs, convert.rs, conformance/backend.rs, window.rs, cbor.rs, link.rs, codec/*.rs, stats.rs, observe.rs, the adapter test suites, and the agent notes the history pass cited) were read where a verdict rests on them.

The partition is in good shape. The types do the protocol's work where it matters: `Encoded::write_with` makes "wire before internal publication" the only order the API admits; `ReplyFrame` keeps the stream lifecycle control out of `StreamSender::frame`'s signature; the take-once claim slots make head-of-line coupling structurally absent. Peer input is judged in one place (`SupplyRuns::observe`) with typed errors, the declared-`set_len` charge lands before custody on both decode paths, and nothing reachable from wire bytes panics. The maintainer prose states invariants and the why at the branches that need them, and the hazards that bite (`StreamSender::frame`'s cancel safety, `AcceptDriver`'s detection latitude, the supply-failure deferral) are argued where the code is. Every testdoc in `streams/tests.rs` is accurate except one.

The dominant issues are residue and duplication, not misdesign. Two structural findings matter most: the height-erasure commit collapsed `Scope` to one type but left the `Q`/`N`/`D` type parameters and four copies of the scope-derivation rule behind (F5), and `read_early` re-implements `read_reply`'s frame and record loops, including a verbatim five-line comment, when `read_reply` over an empty root scope already produces its exact rejections, which is how the encode side already handles early supplies (F3). One owner-gated structural item: `ReplyFrame`'s exclusion is a runtime `TryFrom` on frames the adapter never produces, leaving a public error variant no session can fire (F19). One owner-gated design question: the decode channel's capacity is justified as "load-bearing for liveness" in two places without a mechanism, and none is derivable from the code (F6). The verification gaps are real but bounded: the label parser's rejection arms have no committed test, `early_supplies`' post-error withholding is not pinned, and the backend-contract asserts are promised in docs but never demonstrated to fire; two of these were already dispositioned in the scope-A mutation campaign and not yet landed. The rest are low-severity state-machine simplifications in `streams.rs`, doc lines one layer behind today's code, and idiom nits.


## Positives

- `Encoded::write_with` (encode.rs:26-36) makes the walk's "wire before internal publication" liveness rule the only order the API admits: the question is unreachable until the writer's future resolves `Ok`. The module doc (adapter.rs:37-40) says exactly why.
- The adapter module doc's closing section "Why this is sufficient" (adapter.rs:69-74) names the four protocol properties the conversion rests on in four clauses and states that the adapter adds no identity or ordering authority of its own. Negative-space specification a maintainer needs and rarely gets.
- `SupplyRuns::observe` (decode.rs:487-541) is the single place peer-supplied leaf records are judged; every wire-reachable failure there is a typed `DecodeError` variant carrying the offending bytes, and nothing from the wire reaches a panic. The declared-`set_len` charge lands before `Leaf::leaf` takes custody on both decode paths, and `malformed.rs` proves it with a node census.
- Positional overruns fail eagerly at the offending frame (`UnpositionedMatch`/`UnpositionedQuery` before the skeleton grows; decode.rs:335-339 states why) and a new supply run requires a strictly larger radix, so the decoded skeleton is bounded by the fan by construction.
- The encoder serializes each leaf straight out of the borrowed node (encode.rs:197-206) with no `Version` clone and no `Arc` bump, and the comment says why the span rides a local; `LeafRun` stays encoded on both sides so the decode bound is one run's bytes (decode.rs:356-358), and `Frame`s move rather than clone along the whole path.
- `Scope` (scope.rs) survived height erasure as a small, fully documented type whose parent prefix is the height witness, stated at the type that carries it (scope.rs:5-7); each method carries a one-sentence contract and nothing more.
- `StreamSender::frame` (streams.rs:157-168) carries a `# Cancel safety` section that states not just "not cancel safe" but the exact peer-side consequence (a truncated label read as a failed supply, misattributed session-wide) and what the caller must do.
- `AcceptDriver`'s type doc (streams.rs:643-650) names the two fates of an unasked stream, admits neither is prompt, and says why that is detection latitude and not a safety gap (never absorbable; memory bounded by the link stream's own buffers).
- The supply-failure deferral (`AcceptDriver::run` docs 678-693, the deposit slot 500-506, `queued_supply_closed` 559-577) is a subtle design argued carefully in prose, and `supply_failure_reaches_the_awaiting_receiver` and `supply_failure_after_delivery_lets_the_session_finish` pin both the report-and-deposit and the park paths deterministically.
- Every failure path in `read_frames` is report-then-park with the transport half retained until teardown (streams.rs:485-492 says why the hand-back is a framing judgment), so a consumer never observes a truncation as a clean end; `truncated_stream_is_reported_not_ended` pins it with the origin checked in full and says why (tests.rs:362-364).
- `label()` (streams.rs:57-68) is declared the canonical spelling and the capture harness's `stream_label` (codec/capture.rs:117-122) parses through the same head grammar, so the wire label has one definition and one decoder; I checked the claim.
- `ERROR_ROUTE_CAPACITY` (streams.rs:580-582) is a named constant whose comment states the invariant it encodes (the first failure is the cause, later ones cascade), not just its value.
- The `debug_assert!` at decode.rs:348-355 justifies itself the way a guard should: it names the concrete failure the wire cannot produce but in-process construction can, and what the loop below would do wrong without it. The comment at decode.rs:179-181 correctly attributes the unpositioned-form rejections on the opening stream to in-process construction only.
- `fan_occupancy.rs` is a model liveness-floor meter: it asserts the reader/assembler channel reaches exactly FAN + 1 on both decode paths under an eager source and stays far under it under a paced source, so the probe is shown live rather than constant.

## Open questions for Finch

- Finding 19: delete `ReplyFrameError` and `RemoteError::ReplyFrame` (public taxonomy) in favor of infallible `ReplyFrame` constructors yielded by `render`? Recommendation: yes, pre-release; the variant has no producer and the doc's "statically" then becomes true at the adapter as well as at `frame()`.
- Finding 6: is the FAN channel capacity liveness-critical by a mechanism I could not derive from the joined drive? Recommendation: construct the sub-FAN run first (one test-only parameter); if it completes, rewrite the two comments to the amortization/read-ahead rationale and treat the pull-based reader as a measure-first experiment gated on a persistent backend actually wanting read-ahead. The answer decides whether the ~205 KB supply-decode pre-charge is a floor or a cost of the current shape.
- Finding 22: should `SendError` (and the adapter's `EncodeError<E>`) follow `StreamError`/`DecodeError` into `#[non_exhaustive]`, or is their closed variant set a deliberate contract to state at the declaration? Recommendation: open them now, pre-release, matching 1e458d69's criterion (variant sets that grow as enforcement grows).
- Finding 11: is the illumos gate run meant to be a property of the tree? Recommendation: either name it in the justfile or CI so the ten comment copies become checkable, or drop the platform roster from the comment and state the lowering-independent mechanism.
- Finding 21: add `frames_sent`/`frames_received` to `SessionStats`? Recommendation: yes; it is the direct feedback for `target_message_size` tuning and costs one `Relaxed` increment per frame at an existing choke point.
- Finding 7: crate-wide dash rule in `//` comments (169 sites): sweep, or record a tolerance where the rule lives? Recommendation: one mechanical sweep, since the rule is already applied to messages.
- Finding 17: the literal 17 in prose spans streams.rs, remote.rs, and codec.rs and was written fresh again in 3327a92b; cite `Stream::COUNT`/`STREAM_COUNT` by name everywhere? Recommendation: yes, in one cross-file pass.
- Cross-cutting, from the refuted candidate [8]: `AcceptError::Label { detail: &'static str }` follows the codec's convention (`DecodeErrorKind::Malformed`, `FrameShape`, eleven literal details in codec/decode.rs and frame.rs). Should present-but-malformed wire items become enums crate-wide, or stay strings? Recommendation: leave as is unless a consumer needs to match; it is a crate-wide question, not a partition inconsistency.
- `Backend` is crate-private today, which is what makes the encoder/decoder contract asserts (finding 14) programmer-error panics. If the trait is ever exposed for out-of-crate storage backends, those asserts need `# Panics` sections on the trait methods or conversion into typed errors; worth recording wherever the trait's visibility is decided.

## Dropped

- [8] `AcceptError::Label` carries a stringly-typed defect: refuted; the codec's sibling taxonomy models the same kind of defect with `detail: &'static str` at eleven sites (codec/error.rs:133, 142; codec/decode.rs:222-330; codec/frame.rs:216, 392), so this follows a crate convention; raised as an open question instead.
- [13] "the existing ... fold" is a dated qualifier: merged into remote-adapter-streams-1 (same sentence as [19]).
- [18] adapter doc describes `Reply<B, H>`: merged into remote-adapter-streams-1.
- [19] module doc names `Convert::assemble`: merged into remote-adapter-streams-1.
- [31] per-record ingest duplicated and drifted: merged into remote-adapter-streams-3; the refutation showed the `debug_assert!` asymmetry is a copy made without it (8e1ed47a7 precedes 55d76d5cf), not drift, and the finding text says so.
- [36] per-record ingestion block duplicated: duplicate of [1]; merged into remote-adapter-streams-3.
- [24] verbatim-duplicated comment blocks: the decode.rs charge-comment pair and the encode.rs "Symmetric with decode" pair dissolve with findings 3 and 5; the remaining `OversizedVersion` restatement is remote-adapter-streams-8.
- [35] session constants threaded positionally: decode-side half merged into remote-adapter-streams-4; the stream-side half kept there in reframed form (a receiver-scoped context; sender and receiver contexts overlap only partially).
- [16] hand-maintained "17": duplicate of [14]; merged into remote-adapter-streams-17.
- [38] qualified paths, repeated bounds, function-local imports: merged into remote-adapter-streams-18 minus the `futures::Stream` item, which must stay qualified because `codec::Stream` is imported at streams.rs:54.
- [39] `claims()` detours through a `Vec`: duplicate of [10]; merged into remote-adapter-streams-26.
- [32] label test pins one literal: duplicate of [17]; merged into remote-adapter-streams-29.
- [26] "smuggle" register transplant: folded into remote-adapter-streams-19's resolution (the doc block is rewritten or deleted there); the wording fix stands even if the deletion is declined.
- Refutation's new item, decode.rs:88 `pin!` also redundant: folded into remote-adapter-streams-3.
- Cross-partition ghosts surfaced while verifying [22] (codec.rs:32 "The session demultiplexer", codec/signal.rs:31 "Logical streams multiplexed"): listed as related sites in remote-adapter-streams-15 for the codec partition's owner, not filed here.
- The open question from the correctness lens about `early_supplies`' doc ("a later group's bulk never gates an earlier group's absorption") and the pruned-radix latency case: below the bar; the doc describes groups on the wire, and the pruned case is a latency observation with no stated contract breached.
- The correctness lens's note that decode.rs:299's `unreachable!` rests on an unlisted `assemble` clause: below the bar as a separate item; an assembler that stops pulling early yields no node for later runs, which "never skipped" (backend.rs:191-192) already forbids. The demonstration gap is covered by remote-adapter-streams-14.

<!-- source: final/remote-adapter-tests.md -->
# Partition remote-adapter-tests: The remote adapter test suites

## Partition summary

This partition is the unit specification of the reply/frame adapter in `src/tree/mirror/streaming/remote/adapter/{decode,encode,scope,error}.rs`: the lossless boundary between erased protocol replies (`Reply<E>` holding node handles, no prefix) and prefix-free wire frames (leaf runs, positional `Match` and `Query`). `tests.rs` (80 lines) holds the shared fixture vocabulary: `LeafCase`, `hash`, `leaf_run`, `unbounded`, `runtime`. `properties.rs` (1148) sweeps six laws over every concrete type-level height 0..32 under proptest through a recursive dispatch macro. `malformed.rs` (719) pins the typed rejections and a few admitting boundaries. `opening.rs` (326) covers the greeting-borne opening reply and the `early_supplies` stream. `runs.rs` (296) states the encoder's byte-budget batching contract as a proptest family plus three point witnesses. `backend_errors.rs` (213) is a per-height, per-operation injected-failure matrix. `parking.rs` (238) and `fan_occupancy.rs` (183) pin the memory-model premises: a parked reply costs handles, not subtrees, and the reader/assembler channel peaks at exactly `FAN + 1`. All 3203 lines are test code; I read every line of every file, plus the production sites each finding leans on (decode.rs, encode.rs, failing.rs, frame.rs, message.rs, queues.rs, work.rs, pump.rs, window.rs and its tests, hash.rs, local.rs, before's gamma and literal codecs).

The test design is strong. Several suites are built so that the wrong implementation yields a different typed error rather than a passing run: the eager-rejection test with two `Continue` frames, the sentinel-frame technique proving a decode consumes nothing of the following reply, the paced negative control that keeps the fan-occupancy pin from passing vacuously, the census liveness floor in the set-length test, and an injected-failure matrix that asserts the backend's operation history rather than only the error variant. Everything is deterministic: current-thread runtimes, in-process frame vectors, thread-local probes, no clocks. No residue of the V1 protocol or BLAKE3 remains in these files, and no proptest seed exists for them, so there is nothing stale to sweep. The lenses that read this partition independently found no harness bug that masks a failure, and I confirm that reading.

The dominant issues are maintenance shape and drifted prose. The same fixture searches, tail-flagging loops, node-at-height traits, dispatch macros, and the three ingress premises (`u64::MAX`, `unbounded()`, `PayloadCodec::new::<u64>(PayloadDepthLimit::default())`, spelled 36, 36, and 40 times) are repeated across the seven files; `properties.rs` states each of its six laws twice, once for `Z` and once for `S<H>`, with the bodies textually identical apart from which adapter entry they call. Two documentation-of-record numbers have expired: `parking.rs`'s module doc states megabyte figures two format changes behind its own pin constant and `message.rs`, and `runs.rs`'s `MAX_RECORD_LEN` derivation describes the retired borsh framing while nine of the ten records in the largest committed fixture exceed the "upper envelope" it claims (verified by an offline model of the framing that reproduces `before`'s committed encoding vector). The one substantive verification gap is that `read_early`, a second hand-written copy of the frame grammar, has only three of its rejection arms exercised on its own path; the mutant campaign recorded that gap with a disposition that has not landed.


## Positives

- `leaf_query_matrix_is_exhaustive` (malformed.rs:190-283) enumerates the 2×2×2 cells and asserts `checked == 8`, so the doc's "all eight" is a count the test enforces rather than maintains by hand; a shrunk loop domain fails it.
- `an_unpositioned_match_is_rejected_in_both_directions` (malformed.rs:82-137) is built so that a late, whole-reply rejection would surface as `TruncatedReply` while an eager one yields `UnpositionedMatch`, both frames being `Flow::Continue`; the error type alone distinguishes the property under test.
- The sentinel-frame technique (properties.rs:154-173, 289-311, 419-443; backend_errors.rs:124-159) proves a decode consumes nothing of the following reply, including after an injected failure; this is the property the pump's positional pairing rests on.
- fan_occupancy.rs pairs each `FAN + 1` equality pin with a paced negative control (161-183) and a non-emptiness check, on a thread-local probe whose current-thread FIFO premise is written where the probe lives (decode.rs:550-560); the pins cannot pass vacuously and no timing is involved.
- backend_errors.rs asserts the `Failing` history equals exactly the operations up to and including the injected one, at every height and every fail-after index in both directions, so post-failure work is observable and forbidden rather than argued.
- runs.rs's proptest (152-217) states the greedy law completely: every non-final run is full because the successor's first record would not have fit, no run is empty, only a lone record overflows, and records round-trip in order.
- parking.rs's supply test uses an independent oracle (Merkle hash and leaf count of each assembled node against the source fan) over a 512-leaf tree, and its disputed-reply pin sums a measured half and a derived half with a deliberately tight ceiling, which has in fact been re-derived at both format changes.
- The `dispatch_height!` ladder under proptest gives full type-level coverage of the 32 heights without 32 hand-written tests, and `foreign_supply_is_rejected_at_every_scopable_height` documents the one principled exclusion (height 31) in both the impl and the testdoc.
- tests.rs keeps the fixture vocabulary small: `Version::try_from(u64)` is infallible, so the `expect` message at line 72 is a true one-line proof.
- No residue of the V1 protocol or BLAKE3 was found anywhere in the partition, and no proptest seed exists for these suites.

## Open questions for Finch

- Bundle the three ingress premises in production? `decode_reply`, `decode_leaf_reply`, and `early_supplies` (decode.rs:68-75, 203-210, 231-238) each take `version_bytes: u64`, `ledger: SupplyLedger`, and `codec: PayloadCodec` positionally, and `proxy/work.rs:54-65` holds the same three as fields and re-passes them at each call. A crate-internal struct for "what the peer's greeting declared" would shrink the signatures, remove a `u64`-first ordering hazard, and let the tests' helper (finding 2) be one `Ingress::unbounded()`. The module is private (`mod adapter;`), so this is a design change rather than a public-API change. Recommendation: yes, alongside the test-side helper.
- Retire the leaf/non-leaf entry split? The `*_leaf_reply` entries differ from the general ones only in the question closure (decode.rs:243-258, encode.rs:80-142), and production selects them from statically leaf-typed code (pump.rs). The split forces the properties.rs duplication (finding 20); `decode()` computes `scope.parent().height() - 1` at decode.rs:277 unguarded, so handing `decode_reply` a leaf-parent scope underflows. Recommendation: keep the split as a mirror of the type-level phase schedule (the rationale at adapter.rs:22-26 is sound) and add a `debug_assert!(scope.parent().height() > 1)` in the non-leaf entries so the misuse fails loudly; fix the duplication test-side.
- Add a leaf-construction injection point to `Failing`? The adapter is the only place `Leaf::leaf` is fallible and awaited on the ingress path (finding 4). Recommendation: yes; it is the custody-transfer moment and `Failing` is in-crate test infrastructure, so the cost is small.
- For the decode.rs reviewer: `SupplyOrder`'s comparison `previous >= radix` at decode.rs:531 can only fire with `previous == radix` (the `LeafOrder` check at 516-527 runs first and strict path ascent under one parent makes the radix non-decreasing), so the `>` half is a dead branch and the variant doc's "preceded" appears unreachable. Recommendation: compare with `==` and state the invariant inline, or make the impossibility an assert, per the mutants ladder.
- For the window reviewer: `supply_decode_envelope_matches_the_charge` (window/tests.rs:239-247) recomputes the flat term with its own `(FAN as u128 + 1)` rather than reading it off `from_budget` (window.rs:397-399), so the solve's factor can drift from the constant without failing it; the shared constant in finding 9 would close this too.
- Does the fan-occupancy equality pin (as opposed to the ceiling) rest on tokio's cooperative-budget behavior? The blind-spots lens read `join` as polling the reader first, with a budget-exhausted assembler unable to drain in the same poll, so the peak is exactly `FAN + 1` under every schedule the current runtime produces. Recommendation: add one sentence at the probe stating which scheduling premise the equality rests on, so a future runtime change reads as a scheduling change, not a code regression.

## Dropped

- `Encoded::write_with` has no test (blind-spots [15]): refuted. Its body is `write(frame).await?; Ok(question)`; an `Err` carries no `Q`, so "release the question on a failed write" is unrepresentable; dropping the write's `Result` is an `unused_must_use` warning under `-D warnings`; proxy/tests/failures.rs injects write faults full-stack and requires the typed transport error, and `context_registration_is_causal` pins the ordering over conforming runs. A direct unit test of a two-statement function adds a point check over covered ground.
- `assert_eq!(checked, 8)` is tautological (blind-spots [21a]): below the bar. The counter guards the loop domains (a shrunk `[false, true]` fails it), and the doc's "all eight" is thereby a mechanically enforced count, which is the shape the doctrine asks for.
- Delete the `eprintln!` in parking.rs (structure-prose [12], one item): refuted. It is the readout both re-pin commits quote; folded into finding 18 as "state its purpose at the site".
- `Stream::new(11)` needs a justifying comment (structure-prose [12], one item): refuted. The comment directly above it (parking.rs:184-185) justifies it, and `Stream::COUNT = 17` keeps every label a one-byte CBOR head.
- A miscounted underscore silently drops a height from the dispatch macros (api-economics [27]): refuted. `for height in 0..32` requests every height and the fall-through arm panics; the duplication itself is folded into finding 10 with the `seq!` shape as its resolution.
- Bundle the ingress premises in production (api-economics [25], production half): out of partition; recorded as an owner-gated open question.
- Retire the `*_leaf_reply` entries or debug_assert the height (api-economics [26], production half): out of partition; recorded as an owner-gated open question.
- Leaf-fixture searches re-derived per file (api-economics [29]): duplicate of finding 10.
- parking.rs figures stale (blind-spots [14], api-economics [23]): duplicates of finding 17.
- `MAX_RECORD_LEN` borsh-era (structure-prose [5], blind-spots [19]): duplicates of finding 22.
- properties.rs impl duplication (api-economics [26], test half): duplicate of finding 20.
- backend_errors doc overclaim (blind-spots [18]) and tests.rs map omission (api-economics [31]): duplicates of findings 4 and 1.
- Positive law in malformed.rs (api-economics [30]): duplicate of finding 11 (the two seeds proposed opposite deletions; the finding names the trade-off).
- Import layout (api-economics [32]): duplicate of finding 5.
- `read_early` rejections untested (blind-spots [16]): duplicate of finding 15.
- Erased aliases (structure-prose [7]): duplicate of finding 19, with its incorrect "never existed" premise replaced by the verified history.
- `LeafCase` fold (blind-spots [21d]): duplicate of finding 3.
- `SupplyOrder`'s dead `>` half in decode.rs (refutation, new 1): production, out of partition; recorded as an open question for the decode.rs reviewer, with the test-side half in finding 13.
- `supply_decode_envelope_matches_the_charge` recomputes the factor by hand (refutation, new 3): out of partition (window); recorded as an open question for the window reviewer.

<!-- source: final/remote-capture-atlas.md -->
# Partition remote-capture-atlas: The remote module root, the wire capture renderer, the codec test suite and its error atlas

## Partition summary

The partition is the root of the wire-bound proxy and the test instruments that sit beside its codec. `remote.rs` (94 lines) declares the proxy's submodules, carries a long module doc restating the frame grammar, and forwards a shelf of test-gated names from `codec` to `remote`; `remote/error.rs` (19 lines) is the flat alias layer that gives the adapter and codec error types unambiguous names (`ReplyDecodeError`, `CodecDecodeError`) for `crate::error` to re-export. `codec/capture.rs` (731 lines, compiled only under `test` or `test-internals`) is the CBOR reflection renderer: it parses each observed hook item with the codec's own canonical head grammar into a `Node` tree and renders it as annotated diagnostic notation, and its module doc argues that this rendering is injective on wire bytes and therefore a byte pin without a hexdump. The three remaining files are test code: `capture/tests.rs` (354 lines) pins the renderer's commitments, `codec/tests.rs` (739 lines) holds the frame round-trip properties, the 340-placement atlas snapshot, the bounded exhaustive corpus manifest, and the transport read-plan meter, and `codec/tests/error_atlas.rs` (747 lines) witnesses every frame-stream error variant with wildcard-free `describe_*` matches as the compile-time tripwire. Total read: 2,684 lines, of which 1,840 are test code and 731 are test-gated infrastructure; production code proper is 113 lines.

The quality is high. The renderer's depth budget is one counter spanning structural descent and embedded byte-string re-parses; I checked the stated invariant (render depth never exceeds parse depth) at every `render_*` call site and it holds, and three committed tests drive it through both the control-item path and the harness-reachable frame path. The error atlas closes coverage from both ends and names its own blind spot. The read-plan meter holds a formula, the reader, and pinned reference numbers to each other. Every panic in the renderer is a capture-harness contract whose message says why it cannot be the peer's fault.

The dominant issue is one real hole in the renderer's stated contract: a map key that is a container or a protocol-named tag renders as a literal `…`, so two wire byte strings differing only inside such a key render identically, contradicting the injectivity claim the module doc rests the snapshot discipline on. The prior review assessed injectivity sound by hand and took no action; the hole is where that hand analysis stopped, and it argues for the generative injectivity test the suite lacks. The second substantive issue is a ghost in `remote.rs`'s module doc describing a query listing spelling the wire has not had since the deterministic-CBOR rewrite, which is the cost of that doc restating its children's contracts. Everything else is small: a test-only wrapper around a constant already reachable by name, one type carrying two names in one namespace, a frame-to-signal projection written three times, a test-side copy of the signal roster, several testdocs that overstate their bodies, and a handful of idiom and prose nits.


## Positives

- capture.rs: the depth budget is one counter spanning structural descent and embedded byte-string re-parses (`MAX_DEPTH` at 341; `render_embedded_as` re-parses at the consumed depth, 647), the invariant is stated once at `render_node` (447-455), and I checked it at every `render_*` call site (281, 295, 459, 476, 486, 490, 539, 567, 570, 597, 607, 647, 685): it holds. Three committed deep-input tests drive it, including `deep_payload_through_the_frame_path_falls_back` (capture/tests.rs:338-354), which shows that a payload the depth limit admits can still demand 640 unfold levels. This is the no-input-controlled-recursion rule done exactly right.
- error_atlas.rs: coverage is enforced from both ends. Wildcard-free `describe_*` matches (488-497, 568-576, 592-658) make a new variant a compile error until described; `atlas_covers_every_error_variant` (136-153) makes a described-but-unwitnessed variant a test failure, with failure messages naming the table to edit; and the module doc (18-23) names the one hole neither half can see instead of claiming totality.
- codec/tests.rs: the read-plan meter holds three things to each other: a formula over the frame's wire shape (`read_plan`, 620-638, documented in English that matches the code line for line), the reader's actual transport reads (`decode_counting`, 643-656), and pinned numbers at reference shapes (`read_plan_at_reference_shapes`, 696-739), so neither the formula nor the reader can drift alone.
- codec/tests.rs: the exhaustive corpus constants are asserted, and the arithmetic checks by hand: 2 flows x (2 + 1 + 256 + 32,640) + 2 = 65,800 frames per stream, x17 = 1,118,600.
- capture/tests.rs: `supply_reflection_localizes_the_field_that_moved` (46-89) tests the renderer's purpose (a one-field change moves exactly one rendered line carrying the exact value) rather than its output text, and the fixture rationale at 68-71 (a non-flat event tree so containment cannot match vacuously) is exactly the "why" a future reader needs.
- capture.rs: every capture-integrity panic names why it is the harness's fault, not the peer's (module doc 42-47, `label_item` 127-132, `render_frame` 252-267), and the one `expect` inside the parser (`parse_major_seven`, 399) carries a one-line proof pointing at the caller's peek. `parse_major_seven` (398-432) gets the RFC 8949 corners right: one-byte simples below 32 rejected as non-canonical, float widths 25..=27 with exact bits, reserved 28..=30 and indefinite 31 both refused.
- remote/error.rs is a clean, flat alias layer: nineteen lines, no logic, one renaming convention (`Codec*`, `Reply*`) that resolves same-named adapter and codec types without leaking module paths, consumed by name from src/error.rs:44-50.
- The totality witness is enforced, not merely stated: tests/common/gossip_snapshot.rs:350 and 363 call `assert_items_account_for` per directed stream before anything is rendered, so the rendering-as-byte-pin license is actually held.

## Open questions for Finch

- Injectivity pin form (finding 17): an inverse parser from the rendering back to bytes pins injectivity outright but makes the rendering grammar load-bearing; a leaf-mutation property is cheaper and weaker. Recommendation: the inverse parser; it also catches the float-width and trailing-byte survivors the mutants note lists, and the grammar is already stable enough to pin.
- Layering of the test-internals shelf (finding 3): `codec` is `pub(crate)` and src/tests.rs:281 bypasses `remote` to reach `codec::greeting::encode_greeting`. Either make `codec` private and add a `remote`-level re-export for the greeting encoder, or drop the shelf and let `testing.rs` import from `remote::codec`. Recommendation: keep the shelf, make `codec` private, and route src/tests.rs through `remote`; one door is easier to read from `tests/common`.
- `MAX_DEPTH` (finding 10): keep 64 and document, or derive from `DEFAULT_PAYLOAD_DEPTH_LIMIT` plus the frame's structural overhead so every admissible payload renders as a tree. Recommendation: keep 64 and document; deriving deepens `render_node`'s recursion to 256-plus frames for a legibility gain no fixture needs.
- `LinkCapture`'s home: the crate never reads it (its only in-crate appearances are the definition and three re-export hops; `tests/common/gossip_snapshot.rs` constructs it and re-aliases it as `CapturedLink`). It stays crate-side because design/rumors-frame-fuzz.md section 4 plans to consume it from a separate fuzz crate through `rumors::testing`. If that plan proceeds, the placement is right; if it is dropped, move the type to `tests/common`. Recommendation: leave it until the fuzz target lands or is abandoned, then revisit.
- Em-dashes in `//` comments: two sites in this partition (error_atlas.rs:77, 588), but 116 across `src/`, so the crate's de facto convention is the em-dash in line comments and AGENTS.md rules on nothing here; the spaced double-hyphen rule is your global doctrine. Recommendation: decide once, crate-wide, and if double-hyphens win, sweep mechanically rather than per partition.
- `#[non_exhaustive]` on `DecodeSignalError` and `LeafRunError` (finding 35): both arguably meet 1e458d69's "grows as enforcement grows" criterion. Recommendation: mark both; they are decode-side validation taxonomies, and a downstream wildcard arm is the correct response to a new one.
- `render_frame` (capture.rs:252) reads the frame array's head and asserts its major but never holds `head.value` (the arity) against the number of body items it parses at 277-287, and the module doc's list of capture-integrity panics (42-46) does not mention arity. Recommendation: add the assertion; the decoder's `FrameArity` check on the real path means a mismatch is a harness bug, which is exactly what the renderer panics on.
- Wall time of `bounded_corpus_manifest_snapshot` (finding 28): unmeasured. Recommendation: time it once and record the number; if it is under a few seconds, close the finding.
- The `Greeting`/`Listing` exemption entries (error_atlas.rs:99-111) are deliberate promotion tripwires per 151135c5 and cannot fire on today's code. Recommendation: keep them (the cost is two entries and the module doc explains them) and fix the line-89 summary as finding 32 proposes.
- Two out-of-partition pointers the lenses raised that I did not verify: the perfapi lens reports that `DEFAULT_TARGET_MESSAGE_SIZE`'s export site (remote.rs:92) is imported by materialized.rs, inverting the layer order streaming.rs:10-24 states; and the prose lens reports that AGENTS.md's renderer-vocabulary re-accept witness speaks of "hexdump line sequence" while the renderer emits a value tree. Both belong to other partitions or to AGENTS.md; forwarded, unverified.

## Dropped

- `LinkCapture` is defined in the crate but consumed only by `tests/common` [6]: deliberate and holding per design/rumors-frame-fuzz.md section 4 (a fuzz crate needs it through `rumors::testing`); moved to open questions.
- Two atlas exemptions assert the absence of markers no described variant can render [7], [22]: the `Greeting`/`Listing` entries were added deliberately in 151135c5 as promotion tripwires with the reason stated inline and at module doc lines 6-12; deleting them reopens a recorded decision without new evidence. The constructive half (tie `SupplyTooLarge` to a constructed instance; fix the line-89 summary) survives as finding 32.
- The re-export layer is circular justification [5]: reframed by the refutation pass; a curated shelf is the same pattern codec.rs:80-84 uses for `capture`, and the layering choice is taste. The mechanical cfg merge survives as finding 3; the layering question is under open questions.
- The glob doubles the codec and adapter `DecodeError`s too (sub-claim of [4]): refuted; `codec` is `pub(crate) mod` and `adapter` is private, neither glob-exported, so only `proxy::Error` is doubled (finding 5).
- The arity-out-of-range branch has no witness anywhere [34]: refuted; `frame_shape_is_enforced` (decode/tests.rs:182-192) feeds `[0x81]` and `[0x84]` and asserts `FrameShape`. The unpinned `detail` string survives as finding 33.
- codec/tests.rs lacks a module doc (half of [40]): below the bar; no rule requires test-module docs (the gate's `testdoc` checks functions), and signal/tests.rs lacks one too.
- Em-dashes in two `//` comments (half of [29]): crate-wide convention question (116 sites), not a partition nit; moved to open questions. The two adverbs survive as finding 18.
- `cbor::TAG_SELF_DESCRIBED` left module-qualified (part of [9]): reframed by the refutation pass as a short qualified path through the imported `self`, a consistency nit folded into finding 26's optional step.
- Duplicates merged: [21], [30] into finding 13; [47] into 29; [46] into 4; [43], [52] into 5; [23], the count halves of [24] and [40] into 1; [32] into 2; [48] and the tag half of [41] into 26; the major half of [41] into 11; [50] into 15; [49] into 14; [36] into 10; [17] and the roster half of [24] into 23; [51] into 8.
- Refutation-pass new item: a single container-keyed listing entry renders `… =>` with no annotation: folded into findings 13 and 14.
- Refutation-pass new item: streams.rs:3 also hand-states "17-per-direction": folded into finding 1 as a related site.

<!-- source: final/remote-codec.md -->
# Partition remote-codec: The remote wire codec: budgets, decode (sync and async), encode, errors, frames, greeting, signals

## Partition summary

The remote codec is the frame grammar every logical stream of the streaming mirror speaks. A frame is one CBOR array item, `[stream, state]` or `[stream, state, body]`: the opener's two unsigned ints name the logical stream and the signal's state code, validated against a per-speaker phase schedule (`signal.rs`); a body is either a canonical `{radix: hash}` listing map (queries) or a tag-63 byte string holding a run of leaf records (supplies), the records kept encoded and decoded lazily (`frame.rs`). `budget.rs` derives the default run budget in closed form and defines the one whole-frame boundary (`covers`, with `admits` as `covers` of the grown body) that the encoder's flush rule and both decoders' ingress gate share. `encode.rs` renders the fixed heads on the stack and `encode/async_io.rs` writes the pieces straight to the transport; `decode/async_io.rs` is the production reader, which reads exactly (never a byte of the next frame), batches the opener and the listing entries into bulk reads, and holds supply frames to the negotiated budget from the first record's heads before buffering the body. `decode.rs` holds the shared validators plus a `#[cfg(test)]` synchronous decoder that the tests hold the async reader to through `decode_both`. `greeting.rs` spells the tag-24 greeting item, and `error.rs` is the typed taxonomy that reaches users as `rumors::error::{CodecDecodeErrorKind, ...}`.

I read all sixteen partition files with line numbers, 5072 lines in total, of which the six `tests.rs` siblings are test code (`budget/tests.rs` 91, `decode/tests.rs` 1108, `encode/tests.rs` 236, `frame/tests.rs` 182, `greeting/tests.rs` 213, `signal/tests.rs` 168: 1998 lines). Production code is `codec.rs`, `budget.rs`, `decode.rs`, `decode/async_io.rs`, `encode.rs`, `encode/async_io.rs`, `error.rs`, `frame.rs`, `greeting.rs`, and `signal.rs`, though about 330 of those lines are `#[cfg(test)]` or `test-internals` scaffolding. To settle points I also read `src/tree/mirror/framing.rs`, `src/tree/mirror/streaming/remote/error.rs`, the export list in `src/error.rs`, parts of `cbor.rs`, `observe.rs`, `peer.rs`, `link.rs`, `height.rs`, `error_atlas.rs`, the review packet under `.agent-notes/2026-08-20-cbor-wire-review/`, and the pinned-version sources of `ciborium` 0.2.2, `bytes` 1.11.1, `tokio` 1.52.3, and the 1.97.1 standard library.

The code is in good shape. The invariants that matter are each enforced in one place and pinned by a committed test: one ingress gate for both listing surfaces (`ListingBuilder`), one budget boundary for encoder and decoders, closed-form wire constants each pinned against an actual encode, a lazy run whose per-frame memory bound is one run's bytes, an over-budget gate that decides legality before buffering (with the truncated-after-heads case in the proptest that distinguishes an early decision from buffer-then-check), and a stated ingress spelling boundary protected by three tests that say so in their docs. Every `expect` and `unreachable!` in production code carries a one-line proof that is true of the surrounding code, both async entry points document cancel safety concretely, and the error strings read as plain English.

The dominant issues are three. First, today's three commits (the two-item opener, the bulk reads, the stack-rendered heads) left small residues: a public type leak (`Signal` through `InvalidSignalPlacement::signal`), four spellings of the opener length, hand-maintained counts in the module doc, and two latent correctness defects in the bulk-read paths that the sync-oracle differential cannot see because no committed test drives a failing `AsyncRead` or a listing head defect through the async reader. Second, the error taxonomy is uneven: the frame decoder flattens typed head and listing defects into static strings while every sibling taxonomy keeps them typed, a public `GreetingError::Order` variant is unreachable through any public path, and most variants of the public enums carry no doc. Third, one contract breach: the over-budget lone-record read resumes into a `Vec` whose slack capacity lets `read_buf` take bytes past the declared run, breaching the exactness guarantee two docs state; a conforming encoder cannot produce the triggering record, so it is latent, but the clause is real and the clamp is cheap. The rest is simplification and prose: a greeting parser whose roster-loop shape forces eight panic sites, hand-written derives left from the erased type parameter, a stale public variant doc, and a handful of register tells.


## Positives

- `Heads<const N: usize>` (encode.rs:46-78) renders the fixed heads on the stack with a capacity derived from the grammar's maxima, so body-free frames, the frames a session writes most often, allocate nothing; the claim is enforced by the committed allocator meter in `tests/encode_alloc.rs`, not asserted in prose.
- `ListingBuilder` (frame.rs:421-486) is one ingress gate for both listing surfaces: the async reader, the sync oracle, and the slice parser (hence the greeting) all drive the same `key`/`value_head`/`entry` discipline, so the deterministic-key-order rule and the canonical-child-order rule are enforced once as one rule, and `ListingBuilder::new` rejects an oversized map head before any entry is read.
- The budget algebra is closed under one boundary: `admits(body, record)` is `covers(body + record)` (budget.rs:147-162), so the encoder's flush rule and the decoder's ingress gate cannot drift; every wire constant is derived from the head grammar, and each closed form is pinned against an actual encode (`full_fan_frame_len_matches_an_actual_encode`, `record_len_matches_an_actual_push` sweeping head-width regimes, `admission_charges_the_frame_envelope`).
- `LeafRun` stays encoded on both sides of the wire (frame.rs:58-71): the encoder borrows it and the decoder validates framing once and yields records lazily, so the per-frame bound is the run's bytes; `a_zero_length_record_is_structurally_valid` pins the laziness rather than relying on it.
- The over-budget ingress gate (async_io.rs:429-463) decides legality from the first record's heads alone before buffering the body, and `multi_record_frames_are_held_to_the_run_budget` includes the truncated-after-heads case that distinguishes an early decision from a buffer-then-check implementation; `overbatched_supply_rejects_without_buffering_its_body` prices the premise under an allocation ceiling.
- `FrameRead`'s `# Reads` section (async_io.rs:50-63) states the bulk-read invariant in one paragraph, and the code visibly honors it through `Pending` and `Exact::head`; both async entry points carry accurate `# Cancel safety` sections (async_io.rs:108-115, encode/async_io.rs:54-60) naming the hazard and the two safe disciplines.
- The module doc's ingress-boundary statement (codec.rs:10-16) is protected by three contract tests (decode/tests.rs:467-563), each ending "Flipping this to rejection is a deliberate contract change, not drift", so prose and tests say the same thing and the tests would catch the prose going stale.
- `decode_both` (decode/tests.rs:855-881) is a tidy differential harness: two decoders, identical classification required, I/O text differences deliberately elided by `kind_signature`, and the testdocs say "in both decoders" where it applies.
- Every `expect` and `unreachable!` in production code carries a one-line proof that is true of the surrounding code (frame.rs:265, :268; async_io.rs:376, :523; codec.rs:116, :206, :212, :220, :239), and every peer-declared count or length is bounded before it drives a loop or an allocation.
- The `OpeningSupplies` arm of `validate` (signal.rs:329-331) is a model for branch commentary: the rule, its exception, and the reason in three lines.
- No error, assert, or log string in the partition contains an em-dash or a colon-fronted fragment; the Display strings read as plain English sentences.

## Open questions for Finch

- Is the error taxonomy diagnostic-only, or are `Signal`, `Flow`, `End`, and the stream-index error public vocabulary (remote-codec-30)? Recommendation: diagnostic-only: narrow the accessors to `pub(crate)`, rename the index error to a struct, give `Origin` and `InvalidSignalPlacement` a `Display` that avoids `Debug`, and enable `unnameable_types` so the class cannot recur.
- Should `DecodeErrorKind` carry typed `HeadError` and `ListingIssue` sources (remote-codec-9)? Recommendation: yes; it dissolves `listing_issue`, `head_detail`, the atlas exemption, and remote-codec-10 in one change, and matches the greeting's typed surface.
- Reopen B2 (remote-codec-24)? The ruling declined enforcement because the detector would never fire against an existing encoder; the per-record allocation and 4 KiB memset were not before you. Recommendation: enforce (hand-parse the atom as the greeting does), name the ruling and the two flipped pins in the commit.
- Where should the exactness clamp live (remote-codec-14)? Recommendation: in `resume_payload` (framing.rs), since its doc already promises exactness without a capacity precondition, so every present and future caller inherits it.
- Keep the sync oracle? Its independent coverage is narrower than its lines suggest (opener assembly and the whole-body read), but remote-codec-10 is exactly the class only an independent implementation catches, and retiring an instrument requires the replacement to demonstrate coverage first. Recommendation: keep it, give the struct a doc stating its role, and dissolve the shared fragments (remote-codec-8).
- `checked_run_len` at the encoder (remote-codec-23): a typed `EncodeErrorKind::SupplyTooLarge` that only a programmer error in the accumulator can reach, exempted from the atlas as unconstructible. Typed error or assertion? Recommendation: keep it typed (it is also the length conversion the run head needs) and let the doc name what it catches.
- Em-dashes in `//` comments: 116 sites across src, 5 in this partition, against 2 sites using ` -- `. CLAUDE.md prefers the double-hyphen. Recommendation: one crate-wide sweep or an amendment to the rule; fixing the five here alone would make the codec the outlier.
- `FAN` and `MAX_QUERY_CHILDREN` (remote-codec-3): one radix fan spelled as two constants that `budget.rs` multiplies together. Same quantity, or deliberately distinct (a fan of reactions versus a fan of listed children)? Recommendation: define one from the other and say why at the declaration.
- The `#[non_exhaustive]` rule (remote-codec-18): state it in the tree, and rule on `EncodeErrorKind`, `DecodeLeafError`, and `LeafRunError`. Recommendation: open (they grow with enforcement).
- A public getter for the effective run budget and a stat for the negotiated minimum (remote-codec-5)? Recommendation: add the getter and the `MAX_RUN_BUDGET_BYTES` re-export now; defer the stat until a user asks.
- `read_greeting` reads a peer-declared byte-string length through `read_payload` with no cap (greeting.rs:273). Memory tracks receipt, and under the honest-peer model the greeting is bounded by the peer's own version size, which is legitimately unbounded. Recommendation: no change.
- The two unaddressable-length `Shape` details (greeting.rs:171-172, :268-271) are exempt from construction tests only by a free comment at greeting/tests.rs:197-203; the atlas's `EXEMPT_MARKERS` covers `GreetingError` wholesale. Recommendation: acceptable as is (no 32-bit test host exists; the gate's wasm32 target builds the fuzz-fit guest, not tests), or move the two markers into `EXEMPT_MARKERS` if you want every exemption mechanical.

## Dropped

- Supply frames cost three transport writes plus a flush (perfapi 48): refuted; the piecewise writes exist for `FramePart` attribution, the sign is workload-dependent, and the reader's own resolution was to leave it unless a profile shows otherwise.
- Run buffers grow by amortized doubling and are never reused (perfapi 50): refuted; std's standard policy, both alternatives retain memory the budget doc says is released, and the resolution was measure-first with no measurement.
- Em-dashes in `//` comments (prose 30): the premise that the codebase's comment dash is the double-hyphen is false (116 em-dash sites against 2); a crate-wide style decision, moved to open questions.
- Dissolve the sync decoder oracle (perfapi 46): a design proposal with a concrete counterexample against it (remote-codec-10 is a defect only an independent implementation surfaces) and a recorded retention rationale; moved to open questions.
- Greeting truncation split across `Head(Truncated)`, `Shape("... is truncated")`, and `Listing(Truncated)` (refutation's new observation 3): below the bar; the greeting arrives whole through a declared length, so "cut short" inside it is a shape defect of the item, not a transport truncation a matcher would want to group.
- Narrow `parse_listing_map` and `write_listing` to `pub(super)` (part of structure 14): impossible; the `pub(crate) use` re-export at codec.rs:99 requires `pub(crate)` on the definitions (E0364). The remaining visibility items survive in remote-codec-21.
- The two-definition state roster (structure 10, first half): doctrine-compliant as written (a quantity computed two ways with a committed test comparing them; the table is what the snapshot pins); the residue survives as remote-codec-31.
- perfapi 42: duplicate of structure 0, merged into remote-codec-30 with its `Display`-through-`Debug` observation.
- correctness 37 and perfapi 53: duplicates of structure 2, merged into remote-codec-27.
- correctness 36: duplicate of structure 3, merged into remote-codec-28.
- perfapi 44: duplicate of structure 4, merged into remote-codec-22.
- prose 17, correctness 40, perfapi 43: duplicates of structure 5, merged into remote-codec-20.
- perfapi 52: the stream-count half of structure 11, merged into remote-codec-3 with structure 6 and 7.
- prose 18 and 31: merged into remote-codec-1 with structure 15.
- prose 20: duplicate of structure 16, merged into remote-codec-29.
- prose 21 and 23: merged with prose 22 into remote-codec-17.
- prose 25: merged with prose 26 into remote-codec-19.
- structure 12: reframed to a missing rationale sentence (remote-codec-32); the separation is deliberate and its reason recorded in 40b1e96a and observe.rs:8.

<!-- source: final/remote-proxy-tests.md -->
# Partition remote-proxy-tests: The remote proxy test suites and harness

## Partition summary

This partition is the wire proxy's own test tier: the only place in the crate where `remote::Handshaking` is driven across a `Link` with in-crate fixtures (outside it, `RemoteHandshaking` is constructed only by `streaming.rs`, `peer/gossip.rs`, and `peer/gossip/tests.rs`). `proxy/tests.rs` is the hub: it builds two materialized participants and two wire proxies over in-memory links, in an asymmetric arrangement (`reconcile`) and in the production arrangement where both local participants are the client of their own proxy (`reconcile_symmetric_accepts`), and holds the wire result to the in-process protocol (`reconcile_locally`) under channel schedules, backend fault injection, and an accept-reordering decorator. `tests/harness.rs` supplies the reusable two-proxy driver (`drive`) plus three link decorators: per-endpoint I/O adversity through `testing::wrap_link`, a one-frame mutation script (`ScriptedWrite`) that locates the state item by parsing the wire's own head grammar, and a received-greeting rewriter (`RewriteRead`); it also owns the role-election predicate `left_initiates`. The six sibling files partition the adversities: `transport.rs` (fragmentation, delay, flush buffering), `failures.rs` (typed transport faults and their stacking with backend faults), `malformed.rs` (reserved, misplaced, unordered, duplicated frames), `declarations.rs` (greeting size words the traffic does not honor), `containment.rs` (version containment over the wire), and `greeting.rs` (the greeting-carried opening listing). `start/tests.rs` pins and fuzzes the greeting ingress on the internal `receive` entry, with the reason for the internal entry stated at the top of the file; `work/tests.rs` reaches the `Work` executor's error-selection races deterministically.

I read all ten partition files with line numbers, 3380 lines in total, every one of them test code compiled under `#[cfg(test)]` into the library's unit-test binary; I also read the production sites the findings rest on (`codec/signal.rs`, `codec.rs`, `codec/greeting.rs`, `tree.rs`, `typed/node.rs`, `proxy/work.rs`, `proxy/start.rs`, `proxy/state.rs`, `streaming.rs`, `streaming/driver.rs`, `remote/streams.rs`, `link.rs`, `testing/transport.rs`, `tree/arb.rs`, `tools/testdoc`, `.config/nextest.toml`) and ran `tools/testdoc` against a scratch probe. The harness discipline is the partition's strength and it is consistent: `run_to_quiescence` wraps every session but two, so a stall is a named `Quiescence::Stalled` failure rather than a hang; every frame mutation asserts `script.fired()`; the transport-fault property predicts from a clean run whether its fault can fire and asserts `report.injected` matches that prediction; the channel-instrument test asserts every `QueueKind::PROXY` edge was exercised. Role-sensitive fixtures route through `message::initiates` rather than byte-order guesses, and the docs say why. I found no harness bug that would mask a failure.

The dominant issues are of two kinds. First, coverage shape: no frame ever crosses a link on a logical stream with index two or greater anywhere in the crate (every proxy-tier fixture is content-addressed, and every committed wire snapshot reaches stream 1 at most), so the leaf-tier decode pump and the terminal stream grammars are validated against live traffic by nothing, while ready-made deep fixtures with their oracles already exist in `tree::arb`; and the harness's default arrangement puts the right-hand proxy in the protocol Client position, which production never does, so most adversity coverage runs one endpoint through `Connect`/`CompleteConnect` impls with no production caller. Two smaller gaps follow the same pattern: the backend-fault property discards the unfaulted endpoint's tree, and the `Work::execute` accept arm is never resolved through `execute`. Second, accretion: the two-proxy topology is hand-rolled seven times, the left-is-`Server`/right-is-`Client` error projection seven times, and several small helpers, a constant, and the n-versus-m fixture two or more times each. The prose is mostly accurate and candid; the residue is five greeting-ingress test names from the retired two-frame greeting, a fuzz doc whose reach expired with the CBOR format change, and a few test docs narrower than their bodies. One gate finding falls out of the reading: `tools/testdoc` does not recognize `#[pollster::test]`, so the doc requirement is unenforced for twelve tests here and seventeen elsewhere.

Provenance for the file: most of the partition's structure comes from two commits with no argued rationale, `77674c9c` (empty body) and `83edcd94` (a work-in-progress commit), which is why most history verdicts are "no rationale found" rather than "deliberate". Where a rationale exists and holds (the zero-inversion tripwire, `cbc4a0aa`; the remote proxy's symmetric entry points, `cbfe1aff`), the findings below are reframed to what the rationale does not cover and marked owner-gated.


## Positives

- Liveness is judged everywhere but two tests by `run_to_quiescence`, and every call site either `.expect`s the poller's result or maps it to `TestCaseError`, so a stall is a named `Quiescence::Stalled` failure rather than a hang.
- Every adversity carries a liveness witness: `Script::fired()` is asserted in all five malformed-frame tests; `transport_failures_are_exact_and_fail_fast` (failures.rs:168-272) predicts fault reachability from the clean run's own counts (`after < completed(clean_report, fault)`) and asserts `report.injected == should_inject.then_some(expected_fault)`, so the fault provably fires iff predicted; `instrumented_channels_cover_every_proxy_edge` asserts each `QueueKind::PROXY` was exercised. The api-economics lens checked the reachability predicate against `testing/transport.rs`'s fire-in-place-of-the-next-success rule for all five operations and found it agrees.
- The differential design matches the design of record: wire sessions are held to the in-process protocol (`reconcile_locally`) and, in the greeting suite, to `Tree::join`; `wire_reconciliation_matches_local` layers trace validity and one-slot channel bounds on top of result equality; the unfaulted counterparty in the transport property is bounded, not condemned, and any completion must equal the oracle exactly.
- Role-sensitive tests derive the initiator from the production `message::initiates` through `harness::left_initiates`, and harness.rs:512-520 says why byte-order guesses would rot, so fixtures survive changes to version encoding and content addressing.
- `ScriptedWrite` locates the frame's state item by parsing with the wire's own head grammar (`cbor::read_head`, harness.rs:162-184) rather than byte offsets, and every scripted test asserts `script.fired()`, so a selector that stops matching fails loudly instead of degrading to pass-through.
- `understated_target_message_size_fails_the_session` constructs its multi-record run by pigeonhole (`BULK_MESSAGES = FAN + 1`, declarations.rs:59-62), matching the budget rule that admits a lone record at any budget; the test cannot pass for the wrong reason. `understated_set_len_fails_the_session`'s doc (222-228) explains why only election-preserving rewrites model a real under-declaring peer.
- `start/tests.rs` states at the check site (lines 9-14) why it tests the internal `receive` entry, exactly the sanctioned form for an internal-entry check, and every rejection test names the exact typed error and its cause.
- `work/tests.rs` reaches the terminal's three attribution arms (deposit outranks consequence, queued stream-granularity report outranks deposit, backend error survives) deterministically by constructing the dead supply before the first poll, with docs that state the attribution contract each one pins.
- The reordered-accepts property is candid that its adversity never fires and turns that into a tripwire; whatever its disposition, the doc does not oversell the coverage.

## Open questions for Finch

- The literal `17` transport capacity in failures.rs: history says it predates `STREAM_COUNT` by a day, so the coincidence is accidental. Recommendation: replace with `TRANSPORT_CAPACITY` rather than coining a second named capacity.
- `wide_symmetric_accepts_reordered_match_local`: keep the zero-inversion tripwire as is, reduce it to one deterministic case over `early_first_child_dispute_pair`, or restructure the driver so accepts can genuinely batch? Recommendation: one deterministic case plus the helper-doc fix; the structural argument makes one case as strong a tripwire as 48, and the decorator's genuine coverage stays with the conformance `ReversingAcceptor` tests.
- Where should a shared `disjoint_pair(n, m)` fixture live: `proxy/tests/harness.rs` (this partition only) or `tree::arb` beside `nth_party` (the sibling suites build the same shapes)? Recommendation: `tree::arb`.
- Once the harness runs production's arrangement by default, `impl Connect for Handshaking` on the remote proxy has one caller: containment.rs's server-position wire check. Keep an impl with no production caller for one test, or dissolve it and let the in-process twin carry position-independence? Recommendation: dissolve, unless the wire path is judged to add something the in-process twin cannot show.
- Should codec.rs re-export `Signal` under `cfg(test)` so the harness selects frames by name, or is the byte-level harness preferred as deliberate independence from the roster? Recommendation: re-export; the roster snapshot already owns the numbers.
- The "lie"/"deceived" vocabulary is crate-wide (43 sites outside this partition, including the `GreetingLie` test type). Recommendation: one prose commit sweeping to the "mis-declared" register 408ede87 chose, with the type renamed in the same pass.
- The blind-spots lens notes that `symmetric_accepts_with_distinct_payloads_are_live` is the only proxy-tier test with non-unit payloads, and no multi-record run under transport adversity carries a non-unit payload. Recommendation: low priority; payload identity is pinned end to end by the public suites and at the codec tier; revisit if a payload-parameterised generator lands for other reasons.
- The seed file `proptest-regressions/tree/mirror/streaming/remote/proxy/tests.txt` records shrinks under the generators of their day; the blind-spots lens believes two entries predate the `schedule` draw. Tuple draws are prefix-stable so the seeds still replay. Recommendation: leave them; seeds are proptest's, and the comments are informational.

## Dropped

- [15] Fault-surface roster hand-enumerated where the proptest derives it by rule: refuted. The proptest also hand-enumerates its five operations in `prop_oneof!`; the deterministic list is an expectation list of the sanctioned tamper-evident form, and `completed()` matches `IoOperation` exhaustively so a new operation fails to compile in this file.
- [44] `PayloadCodec::new::<T>(PayloadDepthLimit::default())` 67 times: folded into remote-proxy-tests-5 (the topology consolidation leaves one site here; a `pub(crate)` convenience on `PayloadCodec` is a crate-wide question outside the partition).
- [2] n-versus-m fixtures, [3] duplicated helpers, [41] election predicate spelled twice: merged into remote-proxy-tests-4.
- [22], [37] greeting test names: duplicates of remote-proxy-tests-1.
- [24], [33] reordered wide property: duplicates of remote-proxy-tests-6.
- [26], [29] join witness placement: duplicates of remote-proxy-tests-20.
- [27] duplicated helpers and magic capacities: duplicate of remote-proxy-tests-4, -17, and -23.
- [28] test docs wider than assertions: duplicate of remote-proxy-tests-11 and -13.
- [30] Merkle-hash comparison: duplicate of remote-proxy-tests-15.
- [31] topology hand-rolled: duplicate of remote-proxy-tests-5; its sub-claim that the two alias families "name the same shape" is corrected (they differ in backend error type, `Failure<Infallible>` versus `Infallible`).
- [32] endpoint projection: duplicate of remote-proxy-tests-16.
- [21], [36] state codes: duplicates of remote-proxy-tests-21, whose resolution takes [36]'s re-export.
- [43] unexplained capacities: duplicate of remote-proxy-tests-17.
- [45] work/tests.rs module doc and `31`: duplicate of remote-proxy-tests-13 and -17.
- Blind-spots open question on `ScriptedConnector` preserving the inner `Done`: below the bar today (the memory connector returns `Done::discard()` itself); the captured-`Done` mechanism it would enable is what remote-proxy-tests-25 asks for.

<!-- source: final/remote-proxy.md -->
# Partition remote-proxy: The remote proxy: start, state, work (encode, progress trace, pump, queues)

## Partition summary

The remote proxy is the wire-bound participant in a streaming mirror session. `start.rs` runs the greeting exchange over a `Link`'s control halves (stamping the local codec's payload-depth limit into the outgoing greeting, requiring equality with the peer's before the equal-versions short-circuit), resolves the window and run budget from both greetings, elects roles, and allocates the claim table, error route, and accept driver. `state.rs` is the typestate chain `Connected` -> `Descending<H>` -> `Completing`, binding one incoming logical stream (a lazily claimed receiver) and one outgoing stream (a lazily connected sender) per stage and threading a one-slot `Scope` queue between stages. `work.rs`, `pump.rs`, and `encode.rs` turn each stage into two independently runnable tasks: an encoder that pairs a dequeued scope with the local walk's reply, flushes the whole wire reply, and only then publishes the derived questions; and a decoder that pairs each flushed question with the peer's reply, yields the reply into a one-slot relay, and only then publishes the derived scopes. The one irregular stage is the initiator's first descent, where an `Early` cursor lazily claims the opening-supply stream and splices each early-shipped root child, exploded to its children, into the responder's empty pairing reply. `Work::execute` drives everything under a biased `select!` whose post-select attribution makes a dead stream supply outrank its own symptoms while exempting typed backend errors. `queues.rs` names the two window-sized scope edges; `progress/` is a trace of the proxy's ordering-critical publications, a ZST outside `cfg(test)`.

I read all eleven partition files, 2504 lines. Test code: `work/progress/trace/tests.rs` (124 lines) and `work/progress/trace.rs` (197 lines, compiled only under `cfg(test)`); the sibling `tests.rs` files (`proxy/tests.rs`, `start/tests.rs`, `work/tests.rs`) are outside the partition list, and I read the portions the findings depend on. I also read the cross-partition sites the candidates rest on: the driver's election and dispatch (`streaming.rs`, `driver.rs`), the protocol traits, `streams.rs`'s error enums and `AcceptDriver::run`, the codec's placement grammar (`signal.rs`), the adapter's decode entries, the gossip-layer error lift, `window.rs`, and the walk's counterparts.

The production code holds up under every lens: I found no correctness defect. Ordering is enforced structurally (`Encoded::write_with` releases a question only after its frame flushes; `yield_reply_scopes!` binds the yield and the scope publication in one expansion, with the reason stated at the macro); every panic site is discharged by the schedule rather than by input shape; the attribution table in `execute` has a committed witness for every arm; and completeness is checked in both directions at every stage boundary. The two medium-severity structural findings are in the handshake-to-session hand-off, which does more than it needs to: it duplicates the post-exchange tail between `complete_connect` and `accept`, threads the same session facts through four positional argument lists, and elects the initiator a second time, independently of the driver, then reconciles the two elections with an enum, two `unreachable!`s, and four `debug_assert_eq!`s. The one medium verification finding is that the ordering trace has no liveness floor and passes vacuously on an empty trace. The rest is maintenance debt of the duplication kind (the same rationale restated at several sites, three near-identical decode loops, a ten-site erase-and-box expression under two aliases) plus public-rustdoc inaccuracies on `RemoteError` and a handful of register and naming nits. Nothing vestigial from the V1 or BLAKE3 removals survives here; one arm (`TerminalQuery` on the decode side) was born unreachable from wire bytes.


## Positives

- `Progress` (progress.rs) is a zero-cost instrumentation seam done right: a `Copy` type that is a ZST outside `cfg(test)`, passed by value everywhere, with the trace machinery behind `#[cfg(test)] mod trace`; both ledgers (`assert_valid`, `assert_registration_causality`) have committed `should_panic` demonstrations in trace/tests.rs for six of their eight assertion paths, and both run inside the proptests over arbitrary divergence and adversarial channel schedules.
- `yield_reply_scopes!` (proxy.rs:18-38) keeps publish-after-yield in one place and states exactly why it must be a macro: the `yield` has to be lowered by `async_stream` before expansion. The rejected alternative is named at the declaration, and the walk's analogue is cross-referenced.
- `encode::write_encoded` releasing the question only after `Encoded::write_with` reports a successful flush turns the wire-before-publication liveness rule into the natural API order; the ordering is enforced by types and dataflow, not by discipline.
- `Work::execute`'s attribution logic (work.rs:197-268) is argued arm by arm in its doc and the code matches every clause; work/tests.rs pins the racing-consequence, queued-`SupplyClosed`, backend-exemption, and parked-protocol cases, and the contract those arms implement is stated on `Error` itself (error.rs:8-15), so code and promise are checkable against each other.
- Every panic site is discharged by construction rather than by input shape: `stream_at`'s expect and `Claims::take`'s expect are exercised for all seventeen (speaker, height) pairs synchronously by every divergent session, so a wrong mapping would fail every proptest at once; `Early::advance_to`'s expect follows from `armed()` at its single call site; `parent.pop()` at pump.rs:208 only ever sees height-31 scopes.
- The erased-body/typed-boundary discipline (pump.rs module doc; `Work::respond` as the single re-tag point) is applied uniformly: every pump body instantiates once per backend, and the typed heights survive only as `QueueRole` labels and prefix lengths.
- The listing-based `early` predicate in `encode::opening` (encode.rs:146-152) is a single linear merge over two already-sorted listings, and it is the right design: it lets the responder distinguish "all pruned away" (an empty reply on an open stream) from "no exclusive children" (no stream); the proxy's early splice matches the walk's `early_survivors` contract, so the join oracle is a meaningful differential check across the two implementations.
- Completeness is checked in both directions at every stage boundary: `UnansweredRemoteQuery`/`UnaskedLocalReply` for the local side, `reject_extra` and `Early::finish` for the remote side, `ExtraOpening`/`MissingOpening` for the opening.
- `Early`'s doc (pump.rs:404-412) is a model maintainer comment: it states the pairing invariant (both sides ascend in radix order, one lookahead slot suffices), names both failure directions, and derives the lazy-claim consequence. `encode::opening`'s doc explains why publish-before-supply is required rather than narrating the two calls beneath it.
- `payload_depth_limits_match` (start.rs:248-257) documents the rejected alternative (negotiation) and the concrete reason it fails; `run_budget` saturates on a 32-bit `usize` instead of panicking (start.rs:277), the preferred behavior at a tolerated corner.
- `queues.rs` justifies two one-line constructors by being the one place each edge's capacity rationale lives, with the occupancy derivation deferred rather than restated.
- No design-doc or agent-note citation anywhere in the partition, and no em-dash in any error, expect, or assert string (both grep-verified).
- The surrounding suite is strong where it counts: `run_to_quiescence` is a deterministic deadlock witness, the proptests run under adversarial channel schedules and one-byte transports, `transport_failures_are_exact_and_fail_fast` sweeps every I/O surface, and `declarations.rs` covers the greeting-declaration matrix in both election directions.

## Open questions for Finch

- Arity bundling (remote-proxy-4). The inline rationale "one premise per argument" (start.rs:338-339, work.rs:101-102) argues against a named bundle for the greeting-derived premises, while four `too_many_arguments` allows on one dataflow and a duplicated `budget` field argue for one. Recommendation: after remote-proxy-7 collapses `connected`/`open` into the role entry points, name the surviving premise set (`Negotiated { window, budget, peer_version_bytes, peer_set_len, peer_listing, codec }`) and drop `Session::budget`; the "premise" reading survives as the struct's field list.
- `send_or_cancel` (proxy.rs:11-16) parks forever when the consumer is gone, whereas the walk's `pump` returns `Ok(())` on the same condition (materialized/work.rs:147-149). The plausible reason is that proxy tasks own transport streams whose early drop would present the peer with a torn stream (`cancelled()`'s doc: "Retain cancellation-sensitive resources until their owner is dropped"). Recommendation: state that reason in `send_or_cancel`'s doc; if it is not the reason, the two participants should agree.
- `Progress` is a ZST field threaded through production signatures, while the walk uses `#[cfg(test)]` parameters and a `trace_id` field. Recommendation: keep the proxy's shape (it costs nothing and avoids `cfg` in signatures) and consider moving the walk toward it; no action in this partition.
- `Descending.early: Option<StreamReceiver<A::Rx>>` is `Some` only for the first initiator-representing stage and `debug_assert!`ed absent thereafter (state.rs:375-378). A type-level marker on the first `Descending` deletes the `Option` and the assert at the cost of a second `Reply` impl. Recommendation: fold into remote-proxy-7's refactor if the second impl stays small; otherwise leave the `Option` and delete the assert alone.
- Error taxonomy (remote-proxy-2, remote-proxy-3): nest the local-protocol variants under one `LocalProtocol` variant and move `PayloadDepthMismatch` to a handshake-level result, or take the doc-only route within the PR #38 parsimony directive. Recommendation: the structural route, pre-release, since both changes make the public enum say what the model of record says (a conformance-bug detector naming whose bug it caught).
- The illumos gate run (remote-proxy-22) is a real practice recorded only in commit eb4e0e1ba's message. Recommendation: one line in AGENTS.md's Commands section naming it, so the eight comments can cite something.
- `futures-util` is a direct dependency alongside `futures` (Cargo.toml:139-140), and start.rs:219 uses `futures_util::future::try_join` where work.rs imports from `futures`. Crate-wide; recommendation: one dependency, `futures`.
- `Handshaking`'s `Connect`/`CompleteConnect` impls (proxy as client, receive-before-send) are exercised only by the test topology; production always makes the proxy the server. Two proxy-clients paired would both wait to receive first. Recommendation: a one-line doc on the impls saying the client role is a test topology.
- Em-dashes in `//` comments (remote-proxy-5) are crate practice at 73 sites. Recommendation: one crate-wide sweep, or a decision that comments may keep them.

## Dropped

- [12] Session::incoming/outgoing docs name a nonexistent parameter: duplicate of [21]; merged into remote-proxy-8.
- [20] Payload-depth rationale at four sites: folded into remote-proxy-4, whose shared helper leaves one call site; the history pass corrected the message.rs:108-112 cite (it is `EncodeError`'s admission doc, not a fifth copy).
- [27] `too_many_arguments` allow placement: folded into remote-proxy-4 (the sites it names are the ones that refactor reshapes).
- [38] Session premises through four allow sites, budget duplicated: overlaps [1]; the arity part contests an inline rationale, so it is an open question rather than a finding; the `budget` duplication is noted in remote-proxy-4.
- [39] Fully qualified channel::Sender, [40] stray PayloadCodec import group, [41] pin!(Box::pin): duplicates of [8] and [9]; merged into remote-proxy-11.
- [42] "the peer's payload codec": folded into remote-proxy-10 as the wording to use when the codec doc collapses to one home; history shows "peer" means the `Peer` type.
- [43] Per-reply backend and ledger clones: folded into remote-proxy-24 as a related cost; the adapter signature is another partition's.
- [26] three of five word choices dropped: "codec seam" is anchored vocabulary in stats.rs:6, 29, 122 (refuted); "unsound" is the owner's ruling wording (71de90c11, payload-depth decision record); "load-bearing" is crate idiom at eleven sites. "acknowledged" and "register" survive as remote-proxy-17.
- [18] "five copies drift" framing: reframed by the refutation pass (the other restatements sit at distinct altitudes and show no drift); the arm-comment gap survives as remote-proxy-13.
- [13]'s attribution of "greeting frames" to 4dd2053c9: corrected by blame to 0aa29ed94; the finding survives in remote-proxy-1.
- [32] downgraded from verification-gap to a documentation nit (remote-proxy-26): the walk's supply-ordering violations and the ledger catch the duplication one layer down.
- [34] mechanism corrected by the refutation pass (the `scopes` ledger window opens at one derived scope, not two); survives as remote-proxy-20.
- perfapi's `#[must_use]` on `Handshaking` builders: below the bar; neither type is reachable outside the crate.
- Refutation's new observations 1 and 2 (`pub` constructors with two callers; `debug_assert!(early.is_none())`) folded into remote-proxy-7; observation 3 (`debug_assert!(asked.is_empty())`) into remote-proxy-25; observation 4 (copy count) into remote-proxy-22; observation 5 into remote-proxy-3 and remote-proxy-29.

<!-- source: final/session-bookmark.md -->
# Partition session-bookmark: Gossip session driver, bookmark persistence, reconciliation docs, wire observation, message encoding

## Partition summary

This partition is the layer between a `Peer` and the streaming mirror protocol. `src/peer/gossip.rs` erases a caller's `Link` into type-erased parts, runs the preamble, holds the bookmark-and-snapshot critical section, hands reconciliation to the boxed, `inline(never)` `Reconciliation::reconcile` and `bootstrap_reconcile`, performs the party hand-off under `PartyGuard`, commits, and exchanges the epilogue; it also carries the `gossip_when` unfold driver and the `bootstrap`, `gossip`, and `retire` funnels. `src/bookmark.rs` pairs a caller's raw byte store (`Bookmark`) with the in-memory identity record (`Bookmarked`: `reclaim`, `slice`, `record`, and a staged-then-committed write-suppression token), and `src/bookmark/format.rs` is the self-describing, hash-checked CBOR frame it persists. `src/observe.rs` is the rumors-blind wire hook with its crate-internal `Attachment`, `SessionHandle`, and `CaptureRead`; `src/message.rs` is the type-erased payload with its cached CBOR bytes and the `PayloadCodec` fn-pointer pair that keeps sessions non-generic; `src/reconciliation.rs` is a public explanation page with no code.

I read all ten files in full, 4910 lines. Test code is `src/peer/gossip/tests.rs` (385), `src/bookmark/format/tests.rs` (509), `src/observe/tests.rs` (105), and `src/message/tests.rs` (315); `src/reconciliation.rs` (249) is documentation only. I traced every panic site in the production files to a guarding branch or a crate-bug precondition and found none reachable from wire, payload, bookmark bytes, or storage failure; the frame reader is total, the epilogue distinguishes EOF from a wrong item, payload decode is depth-bounded with trailing bytes rejected, and the party donation leaves the guard only after its slice is durable and immediately before `party::send`.

The code is in good shape, and several pieces are exemplary: the monomorphization boundary is placed and priced in its own docs; `PartyGuard` and the donation ordering make identity duplication structurally impossible on every exit path; the bookmark's staged and committed tokens are a careful cancel-safety design with named hazard sections; the frame format and its test suite (every single-byte corruption, every truncation prefix, trailing bytes, non-canonical spellings, a rumors-blind parse, hex-first pins with the re-accept rule stated on the test) are the standard the rest of the crate should be held to; send-side admission is literally the receiver's decode.

The dominant issues are second-order. First, prose that drifted behind code changes: the V1 retirement left two-protocol dispatch prose in `gossip.rs`; the epilogue marker widened to two bytes while its test docs still say one and the first byte is never swept; `Bookmark::load`'s public "once per Peer" is false after a failed store; `slice`'s doc claims a `watch` section its only caller deliberately omits; `is_current`'s doc says "exactly" where the code compares own-party projections; `reconciliation.rs` derives the stream count wrongly against `STREAM_COUNT`'s own doc. Second, structure that could be simpler with no behavior change: `gossip_inner` hand-builds eleven `(Intent, Err(..))` tuples because its return type defeats `?`; `Bookmarked` proves a three-field "loaded" invariant with three `expect`s; the `Persist` trait and `Bookmark::store`'s lent-writer closure both outlived the two-face (async plus blocking) design that justified them; the reclaim-if-stale block is duplicated; `try_from_arc` erases and downcasts a value it could keep typed. Third, a handful of API-surface questions that are the owner's to rule on: `Message` is `pub` with user-voice docs but unreachable; `EncodeError` lacks `#[non_exhaustive]` where its siblings have it; `store`'s shape; whether the observer needs an end-of-session hook.


## Positives

- The monomorphization boundary is placed and priced in its own docs: `DynRead`'s doc (gossip.rs:56-70) states the cost model (one vtable call per stream open and per poll beneath frame buffering), and `Reconciliation::reconcile`'s doc (1114-1126) explains why both the boxed `dyn` coercion and `inline(never)` are needed, so neither can be removed by accident.
- `PartyGuard` and the donation ordering (gossip.rs:765-802) make identity duplication structurally impossible: the party is sliced out of the durable record while still held by the guard, taken out of the guard only immediately before `party::send`, and every failure after that point reports `Intent::Retire`; the drop path handles both the fork and the taken-whole case, and the comment explains why the recovery join cannot live inside a `debug_assert!`.
- `Bookmarked`'s staged-then-committed suppression token (bookmark.rs:248-268, 343-361) is a careful cancel-safety design: the token moves only on `write`'s `Ok`, a failed write resets record, stage, and token so the next use reloads the disk, and `# Cancel safety` and `# Errors` sections appear on a private method. `tests/bookmark_transmit_window.rs` constructs the in-flight-write race deterministically.
- `Bookmark::store`'s contract (bookmark.rs:104-120) names atomicity as a safety obligation the crate cannot check and spells out the consequence (re-issuing causal coordinates the network durably holds): the right altitude for a caller-implemented trait.
- The frame format (format.rs) is fully CBOR-parseable, deterministic, and totally shape-checked, with three error enums that each tell the reader what to conclude (corruption, logic error, foreign file). Its test suite is the standard the rest of the crate should be held to: every single-byte corruption, every truncation prefix with the exact `len`, trailing bytes, non-canonical spellings per head, version rejection unshadowed by the hash via the `frame_as` split (a test-design decision stated at the code, 231-236), a rumors-blind CBOR parse, and hex-first pins whose re-accept rules are stated on the tests themselves (tests.rs:465-496). `version_item_len()` derives an offset from the constant so a version bump cannot skew the flips, and says so.
- The epilogue mirrors the preamble as a concurrent `try_join` write-then-read, so it cannot deadlock at any positive transport capacity (pinned with `duplex(1)`), and it distinguishes a cut (`UnexpectedEof`) from a protocol violation (`InvalidData`), with the offending bytes named in hex in the error message.
- `observe.rs` keeps the wire hook rumors-blind (no protocol type in the signature, one whole CBOR item per call); the module doc's four named contract bullets (Ordering, Never block, Coverage, Cost) are the right altitude for a hook consumer, and the cost claim is true at every site (`CaptureRead` wraps the reader only when `observe.attached()`). `SessionInfo`'s doc explains a deliberate omission (no session number) with the alternative the reader would otherwise ask for.
- `message.rs` makes send-side admission literally the receiver's decode (`try_from_arc` runs `decode_exact` at the same limit), so the two verdicts cannot drift; the depth case stays typed end to end (`PayloadDecodeError::Depth` to `EncodeError::Depth`); every `expect`/`panic!` message in the file is a one-line proof; `PayloadCodec` is `Copy` (two fn pointers and a limit), so per-message wire decode is a direct call with no `dyn` dispatch; `PayloadDepthLimit` is a complete newtype with a saturating internal conversion whose bound is argued in one sentence.
- gossip.rs's two critical-section comments (645-670, 805-871) state the safety obligations and the three-term wake predicate with the reason for each term, price the frontier comparisons, and name the test suite that pins the no-loop property; `Retire`'s variant docs tell the caller exactly what survived and what to do with the link; `impl From<()> for Gossip` lets `rumors.changes()` plug straight into `gossip_when`.
- gossip/tests.rs drives the misdeclaring bootstrap claimant through the crate's own protocol machinery rather than hand-forged bytes (191-223), pinning the rejection on both sides that can face a claimant, with the provider's content, party, and link state asserted afterwards.
- No em-dash appears in any error, panic, or assert string in the partition (grep-verified).

## Open questions for Finch

- Is `Message` intended to become public API? If yes, re-export it deliberately and keep the constructors' docs at user altitude; if no, `pub(crate)` plus a crate-wide `unreachable_pub` warning closes the class (finding 44). My recommendation: `pub(crate)`, since `Snapshot` yields `Arc<T>` and nothing in the public surface needs the erased type.
- Should "V2" survive as the name of the current dialect in session prose (gossip.rs:51, 1276; tests.rs:6, 184), given the recorded ruling that the `Protocol` enum stays public as wire vocabulary? Recommendation: keep the enum and the wire-facing names, drop the qualifier from prose that describes behavior rather than the wire ("the epilogue", "the preamble"), and rename the two `v2_` tests now (finding 12).
- `Bookmark::store`: byte parameter or lent writer (finding 23)? And if bytes, should `load` become byte-shaped for symmetry, since the crate `read_to_end`s anyway? Recommendation: owned bytes for `store`; keep `load` reader-shaped for file-handle implementors, decided once.
- Reclaim timing (finding 21): re-word the public doc to the code's rule (a), or extend the persist gate so a hearsay-only advance that newly dominates a stranded identity triggers reclaim (b)? Recommendation: (a), which is consistent with the never-write-on-hearsay ruling `tests/bookmark_when.rs` pins; either way, add the test.
- bookmark.rs:456-464: `reclaim` retains overlapping clocks the fully grown party does not `covers`. In a well-formed universe I could not construct a stored clock whose party strictly exceeds the grown live party (a crashed incarnation's identity is never absorbed elsewhere; own aliases are kept in step by `slice`). Is this branch a deliberate safety net for the documented shared-bookmark misuse (84-91)? If so, the comment should say so; if not, it may be dissolvable. Recommendation: state the rationale at the site.
- "Seam" crate-wide (finding 5): keep as the crate's established term or replace with "boundary"/"layer" everywhere including `SessionStats`? Recommendation: replace, following the `mint` purge precedent, in one sweep.
- The birthday-floor sentence (finding 34): restore the "Off-model note" label per the recorded ruling, or delete it? Recommendation: restore the label; the accident bound already carries the acceptance.
- Do you want a session-end observer hook (finding 38) and an offline bookmark inspector (finding 20)? Both are small, both are public API. Recommendation: the hook yes (rumors-tracing needs it for span status); the inspector when an operator tool first needs it.
- Two reconciliation drivers (finding 15): unify into one `Reconciliation` with an admission parameter, or keep `bootstrap_reconcile` for the readability of the bootstrap path on its own? Recommendation: unify; the asymmetric `.stats(..)` is the first drift.
- `Bookmarked` has no sibling `tests.rs` (only `format/tests.rs`); its token state machine (staged/last/slice interplay) is exercised only through `Peer` in the integration suites. Do you want a direct property test of `Bookmarked` (reclaim/slice/record/write sequences against a model), or is public-API coverage the intended stance? Recommendation: a small model-based proptest in `src/bookmark/tests.rs`, since finding 25's refactor is exactly where such a test earns its keep.
- `PayloadDepthLimit::new(0)`: ciborium's recursion check fails when the counter is already zero, so a zero limit refuses every send with `EncodeError::Depth` and fails every non-converged session at ingress. Consistent with the stated semantics; should `new`'s doc name the degenerate value? Recommendation: one sentence.
- gossip.rs:710-711 returns `guarded.party.is_some()` from the pre-session `send_if_modified` (waking watchers when a party is taken or forked), while `bookmark_update`'s closure (551-552) argues a party-only change moves no observable frontier and owes no wake. Both are harmless under `Changes`' frontier compare; which rule is intended? Recommendation: adopt 551-552's rule at both sites and say so once.

## Dropped

- [39g] AGENTS.md pointer in format/tests.rs:490-492: owner-ratified cross-reference (66d782e5) placed so tamper sweeps find the sanctioned exception; deliberate and documented.
- [24] reconciliation.rs:246 `Protocol::V2` link and observe/tests.rs:58 "observed V2 session": accurate names of the live public variant (the test asserts `protocol: Protocol::V2`), not residue; folded into finding 12's owner-gated vocabulary question.
- [39i] reconciliation.rs:202 "Why 17?" and 214 "twiddle its thumbs": Finch's own register (77334965); taste, no cost named; the grammar item at 233-235 is kept in finding 14.
- [21] "inline comment inverts the predicate": overstated; the comment omits the party-changed arm rather than stating the complement; kept as "incomplete" in finding 26.
- [41] duplicate of finding 18 (epilogue marker docs and first-byte sweep).
- [45] duplicate of finding 22 (`load` once per Peer), whose construction is carried forward.
- [37], [56] PartyGuard items: duplicates of finding 17; [56]'s turbofish folded into finding 15.
- [35] duplicate of finding 19 (test helper braces and doc).
- [48] duplicate of finding 47 (`try_from_arc` round trip), framed as per-send cost; the cost statement is carried forward.
- [47] duplicate of finding 44 (`Message` visibility), whose `try_new` doc correction is carried forward.
- [23], [24] duplicates of finding 12 (V1 residue).
- [39b] duplicate of finding 2 (glued imports).
- [18] severity medium: lowered to low; the epilogue comparison is a whole-array `!=`, so the untested first byte cannot currently diverge.
- [20], [21] severity medium: lowered to low; private prose with the correct statement one grep away.
- Correctness lens open question on `!party.covers` retention, perfapi open question on `PayloadDepthLimit::new(0)`, and the 710-711 versus 551-552 wake rule: not findings (no contract breached); carried as open questions.
- perfapi's `SessionStats` "Two deliberate boundaries" count: outside this partition (src/tree/mirror/streaming/stats.rs); left for that partition's reviewer.
- perfapi's "no bench covers the send path with a non-unit payload": an instrument proposal, not a defect in this partition; noted under finding 47's optional meter.

<!-- source: final/streaming-backend-window.md -->
# Partition streaming-backend-window: Streaming backends (local, adversarial), channels, leaf conversion, the window, and the failing/faulting test doubles

## Partition summary

This partition is the streaming mirror's materiality and sizing layer. `backend.rs` defines what a session node is and what holding one costs: the `Backend` trait (erase/assume, `node_bytes`, `parent`, `children`, and the two bulk overrides `leaves`/`assemble`), the `Node`/`ErasedNode`/`Leaf` observation traits, the `NodeStream` alias, and the generic `Root<B>`. `backend/local.rs` is the only production implementation, mapping the traits onto the crate's `Arc`-handled typed tree and overriding the bulk paths so path-compressed spines are walked and rebuilt without per-virtual-level work; under `cfg(test)` every `Local` operation is wrapped by the poll scheduler in `local/adversarial.rs`. `convert.rs` is the level-by-level default chain those overrides are held equivalent to (explode to leaves through `children`, fold back up through `parent`). `window.rs` turns one byte budget plus the two greetings' set sizes and version-size bounds into per-height channel capacities, using integer Chernoff and Poisson-type occupancy envelopes and a binary search over a saturating charge; its constants are pinned by recomputation in `window/tests.rs`. `channel.rs` names the session's bounded edges and swaps in an instrumented, schedulable Tokio wrapper (`channel/instrumented.rs`) under test. `testing/failing.rs` and `testing/faulting.rs` are the fault-injecting backend and protocol decorators.

I read all thirteen files in full, 3588 lines. Production code is `backend.rs` (414), `backend/local.rs` (244), `convert.rs` (146), `window.rs` (763), and the `cfg(not(test))` half of `channel.rs` (90). Test code is `backend/local/adversarial.rs` (157), `backend/local/tests.rs` (188), `channel/instrumented.rs` (318), `convert/tests.rs` (104), `window/tests.rs` (414), `testing.rs` (13), `testing/failing.rs` (301), and `testing/faulting.rs` (436).

The production code is in good shape. Every panic site I traced is either provably unreachable or a backend-contract enforcement with its proof stated; wire input cannot reach `from_sorted_leaves` unsorted or uncontained because the decoder rejects those shapes as session errors first; the window solve is total and floor-preserving under saturating arithmetic; the integer-envelope inequalities check out (I re-derived `bernstein`, `small_mean_quantile`, and the bit-length figures with a replica); and every quoted figure the prose carries is pinned by a committed recomputation. The differential discipline is applied where it matters most: `Local`'s bulk overrides are held to the default chain by observational-equivalence proptests over both deep-spine and wide-fan shapes.

The findings are mostly residue and drift. Three retirements left prose behind: the two-backend session design (convert.rs's module doc describes a converter that no longer exists; `Root`'s manual `Clone` cites a `T` parameter erasure removed; a pass-through generator in `assemble` is the shell of a removed watermark filter), a scheduling axis that was never wired (`Role`), and one testdoc whose figures were re-pinned beneath it without the prose moving. Two verification gaps stand out: the integer-envelope dominance that underwrites the 2^-40 claim is certified only by an example no recipe runs, and that example by its own header checks a different family than the shipped pair-based functions; and a 22-line comment proves the leaf-request term identically zero where a one-line proptest could pin it. The rest is test-scaffold duplication, hand-counted byte constants beside siblings whose docs forbid hand counting, and altitude and dialect nits in otherwise careful prose.


## Positives

- backend/local/tests.rs holds `Local`'s bulk `leaves`/`assemble` overrides to the level-by-level `Convert` default by observational equivalence over generated leaf runs shaped to stress both compression (tiny-alphabet deep spines) and fan (full-alphabet wide fans), comparing hash, len, floor, ceiling, and the otherwise-unserialized `version_bytes` aggregate per node, and round-tripping the leaves both ways. The module doc states exactly why the overrides exist and what keeps them equivalent. This is the differential-oracle discipline applied at the right boundary.
- window.rs derives `REFERENCE_SLOT_BYTES` and `FAN_SLOT_BYTES` from `size_of` of the real slot types with the rationale inline, and local.rs:110 pins the `Local` handle at pointer size with a compile-time assertion the window's per-reference price rests on: layout facts the compiler checks rather than prose.
- window/tests.rs pins every figure the prose quotes to a recomputation (`scope_envelope_matches_the_derivation`, `supply_decode_envelope_matches_the_charge`, `tradeoff_table_matches_the_derivation`, `default_crossover_matches_the_solve`), each assertion message naming the doc site to update; `pathological_pricing_saturates_to_the_floor` demonstrates the saturating solve at `u64::MAX` corpora under `usize::MAX` pricing. The one drift I found (finding 37) is in a testdoc, not in any pinned figure.
- `Window::from_budget` is total and floor-preserving: saturating arithmetic where a u64 population times a near-`usize::MAX` price passes u128, plain multiplication only where provably below 2^70, and a binary search that terminates at capacity one when even the floor exceeds the budget. The integer-envelope arithmetic checks out line by line: `small_mean_quantile`'s bit-length test is a strictly stronger form of `num * 2^(t+2) < 256^j`, `bernstein`'s `(t - 2) s` slack argument is correct, and the bit-length figures in the `from_budget` comment (241 and 249 denominator bits) match my replica.
- `WindowConfig::default` is unconditional on cargo features, with the reason stated at the site (features are additive and unify across a build graph) and pinned by `default_is_the_budget_unconditionally`; `Window::FLOOR` is an explicit test opt-in rather than a build-shape accident.
- backend.rs:77-88 (`Backend::assume`) follows its unspecified-behavior clause with a one-line proof of why only programmer error can reach it, naming the witness (an erased prefix's byte length is its height) and why peer input cannot reach a mispairing. `Leaf::leaf` carries uniform `# Errors` and `# Cancel safety` sections, and the cancel-safety text reasons through the reclaimable-garbage case rather than asserting safety. `Backend::node_bytes` names the one asymmetric failure mode precisely and points at the suite that convicts a violating implementation.
- The wire path cannot reach `from_sorted_leaves` with unsorted, uncontained, duplicate, or oversized-version leaves: the decoder rejects those as session errors before a leaf enters assembly, so the backend's preconditions hold by construction and its panics are backend-contract enforcement with stated proofs, exactly as backend.rs promises.
- The test doubles compose cleanly: `Failing` layers keep independent countdowns and distinguish their errors (`failing_backends_compose`); `Faulting` injects both reply corruptions and greeting lies, and the greeting-lie suite checks the tolerated (inflated) directions as well as the detectable (shrunken) ones, so the guards are tested for false positives too; every `with_*` scope in the instrumented channel and the adversarial scheduler restores prior state through a `Drop` guard so nested scopes unwind correctly.
- convert/tests.rs:74-78 calls out the one case a naive fold misses (the final group, flushed at input end) and builds its expected value through `Backend::parent` directly, not through the fold under test.

## Open questions for Finch

- Which two containers does `SCOPE_FIXED_BYTES` price (finding 26)? My layout reading: `Query<E>` plus `Resolution<E>` gives exactly 128 today, `Query<E>` plus the proxy `Scope` gives 136 because of `Scope::next`, and a buffered scope has all three. Recommendation: name `Query` and `Resolution` in the doc, derive both constants from `size_of`, and state in one sentence whether the proxy `Scope` is covered by the "overlapping views" argument at window.rs:401-406 or should be priced. If the derived value is 128 nothing re-pins.
- Is a persistent `Backend` still the plan (finding 1)? Recommendation: add the one-sentence "crate-internal today" status to backend.rs now, and schedule the trait-shape pass (owned `Vec` in `parent`, the `pub(crate)` alias in `assemble`'s signature, the `Error` bounds) for the publication commit rather than now.
- `examples/envelope_sim.rs` (finding 32): gate leg, or supersede with an in-tree dominance proptest over the shipped pair-based functions and retire it? Recommendation: the in-tree proptest, because the example's own header says it certifies a different family; then retire the example under the same discipline as landing an instrument, once the proptest demonstrably catches a lowered quantile.
- `Local::assemble`'s run buffer (finding 10): price it in prose or rebuild incrementally? Recommendation: prose now, stating the per-leaf bookkeeping as a `size_of` expression and that it falls under the budget's "replica itself" exclusion; build the radix-stack builder only if a census or allocation meter over a large single-run supply shows it matters.
- The leaf-request term (finding 30): keep it per 655d2ae9's symmetry ruling with the committed pin, or delete it? Recommendation: keep, pin, and cut the comment to the invariant.
- `TAIL_DEPTH_CAP = 40` (window.rs:608-611) never binds: every caller passes `j <= 32`. Is it deliberate future-proofing against a `KEY_DEPTH` change (then its doc should say so), or dissolvable? Recommendation: dissolve, or tie it to `KEY_DEPTH` with a stated reason.
- Bare `pub` versus `pub(crate)` under the private `mod tree` (finding 2): enable `unreachable_pub`, or state the convention? Recommendation: enable the lint; it makes the API surface legible from the keyword.
- Moving the flushed-question derivation onto `queues::local_questions` (finding 24) reverses b76a31f3's placement. Recommendation: move it; the file-path citations are the cost of the current placement, and queues.rs already defers to it.
- Inline `mod tests {}` in test-only modules (finding 14): move to siblings, or exempt? Recommendation: exempt test-only modules explicitly in AGENTS.md; a `tests.rs` beside a module that is itself `cfg(test)` is ceremony.
- backend/local/tests.rs:104-105 says the `version_bytes` aggregate is "not serialized, so this equivalence is its only bulk-path coverage", but the root's aggregate is serialized in the greeting. Is "not serialized" meant per node? Recommendation: reword to "no node's wire form carries its aggregate" so the coverage claim reads precisely.

## Dropped

- `impl Stream for` the instrumented `Receiver` has no consumer (candidate 8): refuted. erased.rs:143-144 declares `#[cfg(test)] type ReceiverStreamOf<E> = Receiver<E>;` and `ReplyResultStream::poll_next` polls it through `Stream::poll_next` (erased.rs:133-138); common.rs:52 calls `rx.map(Ok)` on the instrumented receiver under `cfg(test)`. Deleting the impl breaks the test build.
- Two hand-rolled run-grouping loops share one shape (candidate 12): refuted. The flushes differ in kind: `fold_parents` awaits a fallible `backend.parent` that may return `None` and must be filtered (convert.rs:104-119, 132-135), while `Local::assemble` builds synchronously and infallibly (local.rs:211-214). A generic `runs_by` would leave each caller a tail comparable in length to the loop it replaces.
- Candidates 17, 36, 50 (testdoc figures): duplicates of streaming-backend-window-37.
- Candidate 22 (convert doc): duplicate of streaming-backend-window-19.
- Candidates 41, 56 (rosters): duplicates of streaming-backend-window-16; candidate 27's channel.rs half is folded there and its testing.rs half is streaming-backend-window-21.
- Candidate 19 (ghost `T`): duplicate of streaming-backend-window-5.
- Candidate 42 (`Role` heights): duplicate of streaming-backend-window-12, which carries its height-convention observation.
- Candidates 32, 55 (scheduler duplication and cap) and 13 (`RoleStats` merge): folded into streaming-backend-window-13.
- Candidate 39 (leaf-term pin): duplicate of streaming-backend-window-30, with the refutation pass's sign correction applied.
- Candidates 20, 44 (hand-counted constants): duplicates of streaming-backend-window-26.
- Candidate 52 (positional `u64`s): duplicate of streaming-backend-window-28.
- Candidates 46, 58 (pass-through generator): duplicates of streaming-backend-window-20.
- Candidate 57 (capacity clamp): duplicate of streaming-backend-window-31.
- Candidate 37 as a medium correctness finding: reframed to documentation at low (streaming-backend-window-10); no committed promise is falsified because the budget's documented scope excludes the replica.
- Candidate 51's "decide the seam's status": reopens a recorded deferral without new evidence; narrowed to the status sentence and the trait-shape notes in streaming-backend-window-1.

<!-- source: final/streaming-tests.md -->
# Partition streaming-tests: The streaming protocol test suites: fixtures, skeleton, local equivalence, faults, capacity, stats, wedge

## Partition summary

This partition is the in-process test suite for the streaming mirror, nine files under `src/tree/mirror/streaming/tests.rs` and `tests/`, 3075 lines, all test code (`#[cfg(test)] mod tests` under `streaming.rs`). `tests.rs` holds the shared harness: two `Local`-backed endpoints at `WindowConfig::FLOOR` driven through `mirror` under the closed-world poller `run_to_quiescence`, with optional channel and backend poll schedules, a progress trace validated on every run, and a payload-erased reply transcript; `Tree::join` is the differential oracle. `fixtures.rs` builds deterministic trees with hand-placed paths (one-sided pairs, prefix-cell pairs, pyramids, the full-depth comb, and the three-tree `Divergence` generator). On top sit six suites: `capacity.rs` pins channel-capacity boundaries and parent-delay stall probes; `faults.rs` injects reply violations, greeting lies, and backend failures through the connected driver in both orientations; `stats.rs` pins the session counters against a prefix-closure oracle; and `skeleton.rs`, `wedge.rs`, `local_eq.rs`, and `announced.rs` bridge real sessions to the Lean model (the `Mux.wedge` witness, the `viewEnc`/`LocalEq` projection, and the payload-independence reconstruction).

The engineering is strong where it is aimed. Every test carries a doc comment; the oracles are independent and derived rather than hand-tallied (join for content, prefix closure split by depth parity for disputes, the transcribed Lean literal for the wedge); the capacity witness is pinned from both sides of its boundary (stalls at 253, completes at 254) with liveness floors on every queue role; the skeleton decoder audits every publication against the model's count and parity laws as a total check; and determinism is used uniformly (no sleeps, runtimes, or spawned processes; thread-local instruments scoped by `with_*` guards). The Rust mirrors of the Lean definitions match the Lean source, and the deviations from it are recorded at the site.

The defects concentrate in three places. First, one harness weakness that masks failures: the stall probes in `capacity.rs` collapse completion, violation, and poll-budget exhaustion into one boolean, so every "must complete" assertion passes on a protocol error or a livelock. Second, verification gaps at the edges of what the suite claims: the whole-subtree shed count is never observed above one anywhere in the tree, the fault-injection step range excludes the leaf-height terminal phase on both sides (and a committed seed records a failure at exactly that step), and the join-oracle differential omits the deep-spine generator built for divergence below the root. Third, prose: two doc comments describe code that no longer exists, `wedge.rs` names seed paths the gate now forbids, and the bridge docs identify what they test by campaign roster tags (`B5`, `T3`, `F4`, `finding #7`, `Bridge N`) whose definitions live only in `.agent-notes/` and `formal/doc/`. The rest is legibility work on a test harness that grew by accretion: the session body is spelled nine times behind a six-function ladder, the fixture file hand-rolls what its own builders express, and one family-shaped test drives `TestRunner` by hand and so forfeits shrinking and seed persistence.


## Positives

- Every test in the partition carries a doc comment, and most state their invariant precisely enough to check body against doc line by line (verified: I read all nine files; the two inaccuracies found are streaming-tests-5 and -13).
- The oracles are independent and derived rather than hand-tallied: `join_oracle` (tests.rs:152-161) is the single differential for content, stated with its justification and reused by `capacity.rs`; `stats.rs` derives its expected dispute counts as the prefix closure of an antichain of cells split by depth parity, cross-checks the oracle's own arithmetic (`by_depth == [1, 2, 4]`, sum 7), and adds the conservation law `live_after == live_before + gained - shed`.
- `capacity_stress_witness_requires_inter_level_fan` (capacity.rs:171-195) is an adequacy demonstration of the kind the doctrine asks for: it shows the return queue needs the fan (stalls at 253, completes at 254, `high_water >= 254`) rather than asserting that an oversized constant works; `capacity_stress_covers_every_queue_role` pairs every capacity ceiling with liveness floors (`channels > 0`, `sends > 0`, `receives > 0`, `blocked_send_polls > 0`, more than one typed height observed).
- `skeleton::decode` (335-479) does not merely rebuild the skeleton; it audits every publication against the model's count and parity laws as a total check over the trace, naming the offending scope, and `assemble` enforces the Lean `wellFormed` conjuncts on the way in. `skeleton::announced` reconstructs the skeleton from the payload-erased reply transcript with no tree access, an oracle that shares no code with the walk.
- The Lean mirrors are faithful and their deviations are recorded at the site: `asks`, `viewEnc` (tokens 2/3/4, the R child emitted only to the asker), `LocalEq` minus the vacuous `fan`/`capLevel` conjuncts (skeleton.rs:21-27), the `wedge` literal (matched scope for scope against `Instances.lean:56-67` today), and `Skel::max_fan` reflecting both `wellFormed` bounds; every cited Lean name exists under `formal/lean` (the lenses verified by grep; I confirmed `def wedge`).
- The fault matrix in `faults.rs` runs every injected violation and every greeting lie in both driver orientations and asserts error routing by side; `materialized_backend_failures_are_fail_fast` pins the exact failing operation identity through sibling cancellation; the join oracle and the redaction fixture run in both argument orientations, exercising the error-flip path of `descend` as well as the direct one.
- Determinism is used uniformly: every session runs under `run_to_quiescence` on the test thread, thread-local instruments scope cleanly through nested `with_*` closures, every convergence harness calls `trace.assert_valid()`, and grep finds no sleeps, runtimes, `#[ignore]`, `TODO`, debug prints, dead feature gates, or BLAKE3/alternating-protocol identifiers in code (verified).
- `fixtures.rs` states the invariants that make the oracles exact (disjoint parties keep extras concurrent; slot columns never collide; both remotes advertise a joined ceiling and equal set sizes so the local role is provably identical across the two sessions), and `view_projection_is_sound` asserts that premise rather than assuming it; `wedge_trees` asserts its own election precondition with an actionable message.
- The committed seeds carry provenance annotations ("minted against a deliberately neutralized containment check during mutation review; it passes on real code"), making the adequacy discipline visible in the artifact.

## Open questions for Finch

- Seed disposition (streaming-tests-20, and the orphaning that streaming-tests-17's enumeration would cause for faults.txt lines 7, 8, 12, 15): the AGENTS.md rule says never strip a seed. Recommendation: remove lines 7-8 in a commit whose message names the orphaning (they never matched a live strategy), and when the rosters are enumerated, retire the seed lines that name enumerated parameters in the same way, leaving the `materialized_backend_failures_are_fail_fast` seeds (lines 9-10) in place.
- The Lean wedge literal (streaming-tests-28): a mechanical comparison or an accepted manual discipline? Recommendation: a Lean-emitted expectation file (the `muxprobe-expected.tsv` precedent) read by `wedge_generator_matches_the_lean_literal`, so the Rust literal can be deleted rather than maintained.
- Every local-walk-versus-`Tree::join` check in `src/` runs at `WindowConfig::FLOOR`; wider windows are exercised by the proxy harness and the integration window suites. Does any wide-window run compare against `Tree::join` rather than pairwise convergence alone? Recommendation: one wide-window arm in `streaming_matches_join_oracle`, since a positional-pairing bug that manifests only with several scopes in flight has no differential oracle today (outside this partition to confirm).
- No test in this partition drops the `mirror` future mid-session. The backend contract documents cancel safety for `Leaf::leaf`; where is the walk's behavior under cancellation pinned? Recommendation: if nowhere, a walk-tier pin that cancels at a drawn poll count and checks the local root is unchanged.
- The em-dash convention (streaming-tests-4) is codebase-wide (roughly 116 `//` sites under src/) and sourced from the global doctrine rather than AGENTS.md. Recommendation: a one-line `tools/` lint wired into the gate, plus one sweep, rather than partition-by-partition fixes.
- `tree::Root::len()` and `ceiling()` accessors (`pub(crate)`) for the three test sites that re-derive them: recommendation is no; the in-suite consolidation in streaming-tests-24 removes the duplication without an API change.

## Dropped

- [0]'s related site tests.rs:190-193 ("the closing request for it prunes"): ordinary English, not a retired identifier (refuted).
- [17]'s site tests.rs:232 (function-local `use materialized::{Error, Violation}`): defensible, since hoisting an unaliased `Error` into a file that imports `mirror::Error as MirrorError` would want an alias (refuted).
- [47]'s premise that `tree::Root`'s private fields force two spellings of the length and election: descendant-module visibility already lets the tests read the fields (refuted); the duplication itself survives as streaming-tests-24.
- [50]'s option to dissolve `terminal_errors_preempt_parked_peers`: it pins the crate-owned `Client`/`Server` routing that `descend`'s equal-version path relies on (refuted); relocation survives as streaming-tests-2.
- [23]'s claims that redaction below the root is "never sampled" and that the pruned-to-nothing reply is unexercised: depth-1 redaction is reached by hash collision and the root-fan corner is exercised (reframed); the depth-with-redaction gap survives as streaming-tests-7.
- [24]'s claim that depth reaches the streaming walk solely through hand fixtures: `scheduled_structured_disputes_match_oracle` draws pyramids to depth 6 and boundary fans to depth 31 against the oracle (reframed); the missing deep-spine arm survives as streaming-tests-6.
- [26]/[41]'s coverage-risk framing for the sampled rosters: the per-run miss probability is about 2% for one of 64 cells and negligible for the lie cells (reframed); enumeration survives as streaming-tests-17 on determinism, legibility, and cost.
- [18] (long lines in `proptest!` bodies): folded into streaming-tests-3, whose helper is the fix.
- [51] ("knobs", "genuine"): folded into streaming-tests-8 as dialect sites.
- Duplicates merged: [29], [38] into streaming-tests-27; [39] into streaming-tests-5; [31], [37] into streaming-tests-8; [27] item 1 and [36] into streaming-tests-18; [27] items 2-3 and [46] into streaming-tests-14; [27] item 4, [28], [43] into streaming-tests-23; [7], [40] into streaming-tests-3; [44] into streaming-tests-22; [47] into streaming-tests-24; [34], [45] into streaming-tests-12; [42] into streaming-tests-16; [49] into streaming-tests-9; [48] into streaming-tests-21.
- The blind-spots open question on `Faulting`'s radix-0xff assumption (faulting.rs:230-232): outside the partition; the debug check belongs in faulting.rs.
- The refutation pass's new observations 3 (driver.rs inline `mod tests`) and 4 (roster tags in transcript.rs and progress.rs): outside the partition; referenced from streaming-tests-2 and -8 so one sweep covers them.

<!-- source: final/swarm-example.md -->
# Partition swarm-example: The swarm example and its convergence tests

## Partition summary

`examples/swarm.rs` (1800 lines) is an interactive demonstrator and the crate's documented self-measurement tool. `main` seeds one `Rumors<Vec<u8>>`, forks it by real bootstrap sessions (`bootstrap_fork`) into N party-disjoint replicas, one OS thread each, and hands the directory to a coordinator thread that grows the swarm by fork and shrinks it by `Peer::retire` over an in-memory wire. Each party thread loops: serve inbound sessions from its inbox, obey membership commands, initiate a Poisson-scheduled `Rumors::gossip` over a `memory_with_capacity` link pair when it can claim itself and a peer with compare-and-swap on a per-party `engaged` flag, and otherwise churn under a steady-state controller (`p_add = T / (T + L)`) fed by an `UnorderedMessages` observer that replays the set into a per-thread redaction pool. A metrics layer decorates the link halves through `Link::into_parts`/`LinkParts::into_link` with byte- and direction-flip-counting `AsyncRead`/`AsyncWrite` wrappers; a ratatui UI or a headless sampler reads windowed rates off the atomic counters. `examples/swarm/tests.rs` (149 lines, test code) is one seeded, single-threaded test that drives three forked parties through a target drop and raise and judges each party's four-sample settled mean against a half-to-double band; `Cargo.toml` registers the example with `test = true`, so `just test` runs it.

The concurrency core is in good shape and argued at the site. The rendezvous is a one-slot protocol whose wait-for-graph argument is stated once in the module doc and whose `wind_down` doc gives the one-paragraph proof the code then follows; `InflightGuard` names the concrete failure it prevents and the ordering it relies on; the shutdown handoff is SeqCst on both sides and deadline-bounded with a diagnostic instead of a hang. Every `expect` on a session carries a pointer to the one place that argues why the example panics where an application would match on the error, and every claim I cross-checked against the crate (the `Error` variants' semantics, `SessionState` riding through decoration, `try_into_peer` resolving immediately, the CBOR encoding of `Vec<u8>`) holds. The controller's stale-discard argument is precise, and the test that pins it says which defect it catches and why three parties is the smallest swarm that exposes it.

The dominant issue is an instrument the transport change left behind: the "roundtrips/sync" row counts write-then-read flips on the control stream, which since the per-stream link landed carries only the session envelope (preamble, greeting, epilogue), so the row reports a small divergence-independent constant while the module doc promises the session's request-to-response turns. About ninety lines (`Rounds`, `RoundState`, `CountRead`, the `rounds` wiring, the row) exist to print that constant, and the `Gossiped` the example already receives carries the per-session measures the readout wants. The second substantive issue is the payload type: `Vec<u8>` encodes as a CBOR integer array (about 1.9 wire bytes per random payload byte, with per-element encode and decode on every send and receive) where `bytes::Bytes`, the idiom the crate's own tests use, encodes as a byte string. The rest is a set of low findings a careful maintainer would schedule (CLI input reaching `assert!` and a library `# Panics` precondition, a terminal-restore gap on the error arms, two headless rows that include the warmup window, a data struct carrying display strings, a comment ghost of the retired `Key` vocabulary, docs that describe state the code does not have) and a batch of nits: orphaned lines from a mechanical summary split, em-dashes in `//` comments, moralized adjectives, duplicated rationales, an unreachable drain kept "for good measure", and a `Snapshot` name collision that forces qualified paths.

Total lines read: 1949 (`examples/swarm.rs` 1800; `examples/swarm/tests.rs` 149, test code). Cross-checked in `src/link.rs`, `src/tree/mirror/streaming/remote.rs`, `remote/streams.rs`, `remote/proxy/{start,work}.rs`, `src/peer/gossip.rs`, `src/rumors/unordered.rs`, `src/message.rs`, `src/lib.rs`, `tests/dispute_wire.rs`, `Cargo.toml`, `justfile`, `tools/doclint`, the vendored `ciborium-0.2.2`, and the link-transport review ledger. No cargo, build, or test command was run.


## Positives

- `InflightGuard` (328-351) is a guard by the doctrine's own standard: its doc names the concrete, constructible failure it prevents (shutdown spinning on a slot leaked by a panic unwinding out of `gossip`) and the ordering it relies on (reserve before the final `running` check), and `main`'s drain (489-503) is SeqCst on both sides, deadline-bounded, and reports a wedged session instead of hanging.
- The rendezvous argument is stated once as a wait-for-graph acyclicity claim (58-67), `wind_down`'s doc (626-631) gives the one-paragraph proof that a successful claim means nothing is owed, and the code follows the proof line by line; the serve-before-lock loop at 639-653 is exactly right.
- `steady_state_op`'s doc (796-803) explains why the stale-entry discard is load-bearing for the fixed point (pool inflow is every party's inserts; drain is only own draws), and the test doc (tests.rs:74-79) says why three parties is the smallest swarm that exposes the defect and why two would hide it: a testdoc that tells the reader what the test would miss.
- Byte accounting is designed so each byte is counted once, on its writer, and the rule is stated at every decoration site (762-765, 1155-1156, 1179-1191); `initiator_link`'s note (1197-1199) that `SessionState` rides through unchanged restates `LinkParts.session`'s contract accurately.
- The error-handling comment at 736-742 teaches what a networked caller must do, and every claim in it checks out against the crate's `Error` docs; every session `expect` in the file points back to it.
- Headless mode (1252-1265) reuses the UI's `Snapshot`/`compute` path, so scripted and on-screen numbers are one readout by construction.
- The `#[path = "swarm/tests.rs"]` comment (1781-1783) and the `Cargo.toml` `[[example]]` comment each state exactly what the code cannot show; the Poisson draw (842-849) handles `ln(0)` explicitly and cannot yield a negative `Duration`; the test judges a mean over settled samples with the oscillation rationale in the body (tests.rs:121-127).

## Open questions for Finch

- Roundtrips row (swarm-example-16): drop it and surface `Gossiped.stats` (swarm-example-18), relabel it as control-stream turns, or count data streams opened per session? My recommendation is (a): the descent's per-session measures already exist in the library, and a hop count derived from I/O flips has no denominator the transport still supports. If a depth or hop measure is wanted, the natural home is a `SessionStats` field counted where a `StreamSender` connects on its first frame (the perfapi lens's proposal); that is a library API addition for the partition that owns `stats.rs`, whose doc header at stats.rs:33 says "Two deliberate boundaries:" and lists one.
- Module doc lines 47-49 explain the above-target population offset as "the messages still in flight between views", the propagation-lag mechanism the link-transport ledger records as refuted by direction; the ruling (2026-07-24) closed the item with "no doc reword". Two lenses flagged the disagreement between prose and ledger. If the ruling meant to cover the controller rather than the sentence, the smallest fix is to cut the final clause after "rides above the target", stating the observation without a mechanism. Recommendation: cut the clause; otherwise leave per ruling.
- `bootstrap_fork` exists in three cargo-isolated copies (examples/swarm.rs:166, tests/common/wire.rs:199, benches/support/wire.rs:40). Is a shared helper under the `test-internals`-gated `testing` module wanted, or is the triplication accepted as the cost of the cargo boundary? Recommendation: accept it for now; note it where the copies live.
- Em-dashes in `//` comments (swarm-example-11) are a workspace pattern (160 sites under src/, tests/, benches/), not a swarm one. Do you want a doclint-style rule so the sweep happens once and stays swept? Recommendation: yes, as its own small task.
- The controller test's red measurement (stall at 72 against 20) lives only in d987a2be's message; no committed known-bad controller demonstrates the test fails it. Recommendation: do not add the scaffolding for an example's test; the commit record suffices here.
- tests.rs calls itself deterministic and the reasoning holds by reading (content-determined merge, tree-order observer, `OsRng` only for the network id), but the tokio runtime is built without `rng_seed`. If determinism is a contract rather than an observation, pinning the seed is cheap. Recommendation: pin it when touching the test.
- `MAX_PARTIES` (64) bounds the UI dial but not `--parties` (swarm-example-8). Share the ceiling, or leave the CLI unbounded for scripted headless runs on large machines? Recommendation: share it, and raise the constant if big runs are wanted.

## Dropped

- `Donation` is a one-field struct with no behavior (structure [9]): refuted; the name carries the role in four signatures and its doc states a contract specific to the artifact; the history pass notes the second field was removed by 45835294, but a named hand-off type is ordinary, and the lens's own confidence was low.
- `Rounds` uses a `Mutex` where atomics suffice (structure [2]): merged into swarm-example-16 (moot under deletion; stated for the relabel branch).
- `CountRead.rounds` is an `Option` never `None` (structure [3]): merged into swarm-example-16.
- CLI bounds enforced by `assert!` (structure [7]): strict subset of swarm-example-8.
- `TEST_DUPLEX_CAPACITY` hand-synchronizes with the clap default (structure [12]): merged into swarm-example-12.
- Orphaned doc lines (prose [25]): duplicate of swarm-example-7.
- Moralized adjectives, narrower (prose [27]): duplicate of swarm-example-4; the finalizer sides with cutting "a real application" too.
- Coordinator "claim race" comment (prose [18]), `SwarmPeer`/`Command` docs (prose [19]), `Metrics` doc (prose [20]), `try_initiate` return contract (prose [21]): merged into swarm-example-6 as one doc-drift pattern.
- "Drain any straggler for good measure" (prose [23]) and `wind_down` drain unreachable (correctness [40]): duplicates of swarm-example-14.
- `mod tests` placement and aliases after first use (prose [32]): duplicate of swarm-example-28.
- Headless rows include warmup (correctness [43]): duplicate of swarm-example-23.
- Roundtrips constant and `Rounds` machinery (perfapi [45]): duplicate of swarm-example-16.
- `CountConnector` discards `Done` (perfapi [47]): duplicate of swarm-example-22.
- Redundant `Version` clone (perfapi [48]): merged into swarm-example-13.
- Local `Snapshot` shadows `rumors::Snapshot` (perfapi [49]): duplicate of swarm-example-25.
- `SessionStats` lacks a depth measure (perfapi [50]): converted to an open question; the resolution is a library API addition in another partition's files and is conditional on how swarm-example-16 resolves.
- Module-doc readout wording "on the initiator's I/O" (prose [24], item 4): resolved by swarm-example-16.
- "Never run by the gate" (correctness [36], one clause): corrected; `Cargo.toml` sets `test = true` and `just test` runs `cargo nextest run --workspace`, which builds the example's test binary. The untested-surface claim survives as swarm-example-3.
- "current can never change" (correctness [39], one clause): corrected per the refutation; `grow`/`shrink` draw a random party each retry. The pacing-comment claim survives in swarm-example-20.
- Panic-safety benefit of a `Claim` guard (structure [10], one clause): dropped; a panicked initiator's thread is dead and later claimants back off by design. The legibility proposal survives as swarm-example-15.

<!-- source: final/testing-infra.md -->
# Partition testing-infra: The crate-internal test scaffolding (testing module, memnet, transport) and the lib-level tests

## Partition summary

The partition is the crate's own test scaffolding plus the crate-level tests that need private access. `src/testing.rs` (442 lines) is a `doc(hidden)` facade compiled under `cfg(any(test, feature = "test-internals"))`: one-line delegations that hand `pub(crate)` instruments (codec frame builders, framing chunk constants, window constants, the node census, version-bound walks) to the integration suites; `run_to_quiescence`, the closed-world deterministic poller that turns a wire stall into a `Quiescence::Stalled` error; and `window_tradeoff_table`, the renderer of the sync-budget table the rustdoc includes. `src/testing/memnet.rs` (140 lines) is a channel-backed named-listener network for the routed link. `src/testing/transport.rs` (888 lines) is the adversity layer: `IoPlan`/`IoFault`/`wrap_link` with fragmentation, self-waking delays, flush buffering, and byte- or operation-counted faults on every surface, plus `ReorderingAcceptor`, a batch-reversing acceptor. `src/tests.rs` (690 lines) holds party-linearity tests across bootstrap and retire, retire sessions severed at exact frame boundaries through a hand-rolled `Fuse`, link-poisoning tests, an uncontained-supply test at the `Rumors` tier, and the root-hash read meter pins. All 2160 lines read are test code; none ships in an application build.

The instruments that matter are in good shape. `run_to_quiescence` distinguishes a self-wake from a stall without wall-clock guessing and disables tokio's cooperative budget, with a committed test for each of those two properties. The fault injector's rule that a fault fires in place of the next successful operation keeps every threshold a function of the clean run, and the reason is stated at each site. `ReorderingAcceptor` says why a drain-only design degenerates under the deterministic scheduler and instructs consumers to assert its counter both ways. Every facade entry names the suite it serves and why the constant is read from the code rather than transcribed. `memnet` is small and exact. Every test carries a doc comment, and the severing tests explain in English why each outcome is the only correct one.

The dominant issues are duplication that the scaffolding was built to dissolve, and prose that expired when the wire changed. `src/tests.rs` carries a third byte-budgeted write fuse (`Fuse`/`FusedConnector`/`fused_link`) that `testing::wrap_link` provides; `party_of` and `with_messages` have byte-identical twins in `src/peer/gossip/tests.rs`; `window_tradeoff_table` re-implements `Window::widest`, hard-codes the private `KEY_DEPTH`, and defines a second `DESIGN_SESSION_MESSAGES`. The CBOR respelling of the wire (commit 4dd2053c) moved the preamble to 30 bytes, the greeting to one item, and the epilogue marker to two bytes, and updated the constants and bodies in `src/tests.rs` but not the comments beside them, so the `PREAMBLE_LEN` doc sums to 25, `greeting_frame_len` describes two frames, and the epilogue-severance test says "minus one byte" where the budget is minus two. Three verification gaps remain: the absorber's party is never read after any failed hand-off, `ReorderingAcceptor`'s inversion has no committed demonstration that it ever fires, and `Quiescence::PollBudget` is never observed.


## Positives

- `run_to_quiescence` (src/testing.rs:366-394) is a precise deterministic deadlock witness: a self-wake flag distinguishes progress from a stall without wall-clock guessing, `tokio::task::coop::unconstrained` closes the inherited-budget false-stall hole, and each property has a committed test (`observes_wake_contract`, `ignores_tokio_cooperative_yields`). Twenty-three files across src/ and tests/ call it, and .config/nextest.toml's liveness rationale rests on it.
- The fault semantics in src/testing/transport.rs are documented with their reason, not just their rule (51-65, 567-569, 615-618): read faults fire on payload-bearing reads and accept faults on successful accepts so the structural EOF probes and the final parked accept of a clean session cannot trip an operations-counted threshold. `every_transport_fault_surface_is_reachable` (proxy/tests/failures.rs:278-287) lists every meaningful operation/unit pair.
- `wrap_link` chains `Done` through the adversity wrappers (transport.rs:587, 633) so completion semantics survive decoration; `AdversarialRead` reads through a bounded sub-`ReadBuf`, advances only on success, and discards beyond-budget payload with the failing connection (309-329).
- `ReorderingAcceptor` (transport.rs:648-720) states why a drain-only design degenerates under the deterministic scheduler, bounds its wait in peer progress rather than wall time, and `reorder_accepts`'s doc demands the counter be asserted both ways.
- Every meter-facing facade entry names the suite it serves and why the constant is read from the code rather than transcribed (`frame_payload_chunk_len`, `supply_frame_head`, `lone_record_run`); `window_tradeoff_table`'s claim that the committed table is byte-compared is true and enforced (`tradeoff_table_matches_the_derivation`, window/tests.rs:258-263).
- src/testing/memnet.rs is small and exact: named constants for buffer and backlog, a dial that never waits on accept pace with the routed adapter's requirement stated as the reason (112-115), and a poison-tolerant registry lock whose critical sections are single map operations (84-91).
- The severing tests in src/tests.rs land cuts on exact protocol boundaries and state in English why each outcome is the only correct one (408-417, 507-512); `severed_epilogue_marker_is_uncertain` validates its own placement from below (an under-count changes the error class), and the fuse budgets are computed from the codec's own head writers so a greeting respelling breaks them loudly.
- The root-hash meter has a liveness leg (`root_hash_read_meter_is_live`) beside its two zero-ceiling pins, and `uncontained_supply_fails_gossip_and_poisons_the_link` verifies its own poisoning premise (`!contained(...)`) before asserting on the session.
- Every test in the partition carries a doc comment, and the docs are overwhelmingly accurate to the bodies.

## Open questions for Finch

- `ReorderingAcceptor` (testing-infra-12): a unit-level witness that the patience wait produces a batch of two, keeping the proxy-tier `== 0` tripwire cbc4a0aa2 ruled on, or dissolve the decorator and let the conformance suite's `ReversingAcceptor` carry the inversion proof? Recommendation: the witness plus the doc correction; it costs one test and preserves the recorded decision.
- Should `testing` export a `block_on` (`run_to_quiescence(..).expect(..)`, `#[track_caller]`) with `MAX_POLLS` stated in its doc, dissolving the copy at tests/common/wire.rs:39-42 and settling per suite whether the poller or a runtime is the right driver? Recommendation: yes; it also answers whether `MAX_POLLS = 1_000_000` is sized against the largest closed-world session the suites run, which no comment states today.
- `tests/common/fault.rs` (another partition) is the third fuse; the one capability it has that `IoPlan` lacks is coupled supply refusal (a severed direction also fails connect/accept). Grow `IoPlan` to per-direction faults with that coupling so fault.rs can dissolve into `wrap_link`, or leave it and note the gap in its header? Recommendation: grow `IoPlan` in a later pass; note the gap meanwhile.
- The `features` recipe (testing-infra-13): add the `test-internals`-alone leg? It is outside the gate, so this is a recipe choice rather than gate policy. Recommendation: add it; it is one line and the duplication it justifies is otherwise unverified.
- `IoFault` shape (testing-infra-8): reshape as an enum per surface (changes the `testing` surface the integration suites construct) or document the ignored unit? Recommendation: the enum; `failures.rs`'s hand-avoidance disappears and `State` simplifies with it (testing-infra-9).

## Dropped

- [2] `greeting_frame_len`/`party_frame_len` re-derive wire lengths: refuted. The `Greeting { .. }` struct literal fails compilation on a new field rather than drifting; the three tests self-check the sum to within one byte (testing-infra-21 covers the remaining slack); a measured clean-run offset cannot produce the divergent descent test's boundary; the cited doctrine governs differential suites, not these point tests.
- [4] "the dependency runs the other way": refuted by the cfg gates (lib.rs:306-321); the import at conformance/link/tests.rs:24 sits inside a `cfg(test)` module where both features are on. The surviving residue (path citation, "duplicated") is testing-infra-1; the unbuilt seam is testing-infra-13.
- [11] `drop(survivor)` deletions: superseded by testing-infra-22, which replaces those lines with assertions; the two remaining asserts are testing-infra-10.
- [31] latent `Done` bug in `FusedConnector`: tempered and folded into testing-infra-20; `fused_link` accepts only `MemoryLink`, whose connector returns `Done::discard()` (link.rs:604), so nothing is lost today.
- [35] "never exercised": tempered into testing-infra-13; the conformance-alone direction is built (justfile:522), the test-internals-alone direction is not.
- [37] "subsumed by tests/lifecycle.rs": refuted (distinct cancellation point) and "needs no in-crate access": refuted (`Peer::gossip` is `pub(crate)`); reframed as testing-infra-23 (move, not delete). Its module-doc half is testing-infra-15.
- [0], [21], [33]: merged into testing-infra-16. [1], [20], [31]: merged into testing-infra-20. [5], [6], [22], [36]: merged into testing-infra-2. [7], [25] (Flush arm): merged into testing-infra-9. [12], [25], [38], [39]: merged into testing-infra-8. [8], [34]: merged into testing-infra-5. [10], [30], [40] (imports): merged into testing-infra-3. [15], [26], [40] (wrap_io, literals): merged into testing-infra-11. [13], [27], [41]: merged into testing-infra-6. [14], [37] (module doc): merged into testing-infra-15. [24], [32]: merged into testing-infra-18. [28], [42]: merged into testing-infra-7. [29]: merged into testing-infra-10.
- The refutation pass's observation about proxy/tests.rs:513-515 ("proven instead by the conformance suite's `ReversingAcceptor` tests"): outside the partition's files; recorded as a related location of testing-infra-12, whose resolution rewrites it.

<!-- source: final/tests-bookmark.md -->
# Partition tests-bookmark: Integration tests: bookmark attach, causality, transmit window, when

## Partition summary

The four suites pin the identity bookmark, the crate's durable checkpoint of a peer's `Party` and `Version`, from four angles. `tests/bookmark_attach.rs` (210 lines) holds three point tests on `Peer::bookmark` after construction: a pristine seed writes nothing at attach, a failed persist hands the peer back for retry, and a failed attach never reclaims a stranded region into the returned peer. `tests/bookmark_causality.rs` (1357 lines) is a deterministic, plan-driven fleet simulation: a `World` of `Node`s over `FlakyInMemoryBookmark`, wire faults from `common::fault`, crashes and retirements, judged by an `EmissionLog` that rejects any later durable emission whose version is `<=` an earlier one, plus post-heal convergence, party disjointness, and (reliable variant) identity-space coverage. `tests/bookmark_transmit_window.rs` (654 lines) builds a `GatedBookmark` that parks inside the durable write so a concurrent `send` becomes a deterministic interleaving point, and pins that the persisted record dominates every own-party event a session transmits, including after a cancelled persist and across donation-persist aborts. `tests/bookmark_when.rs` (778 lines) instruments the `Bookmark` trait with a logging `Probe` and checks the read/write call schedule against an operation-semantics `Model` that deliberately shares none of the crate's suppression arithmetic. All 2999 lines are test code; I also read the harness modules they lean on (`tests/common/{wire,sim,flaky,fault,mod}.rs`) and the crate sources their claims cite.

The substance is strong. The properties are the right ones, the oracles are independent of the implementation where independence matters, the proptests carry negative controls and vacuity guards, the two committed seeds match their explicit reconstructions byte for byte, and every suite drives the public API plus the gated `test-internals` aliases only. `GatedBookmark` and the red-first history of the transmit-window pins are instruments-before-cures done exactly right. The `retire` step's refusal to swallow the absorber's result, with the decode-failure classification stated inline, is the standard the rest of the harness should be held to.

The dominant issues are two harness blind spots and a maintenance-shape problem. First, the causality suite's recycle oracle only sees the degenerate recycle (a version equal to or below an earlier one); the recycle the bookmark exists to prevent produces a version that compares `Greater` or incomparable, and since redactions are untracked the destroyed message is invisible too, so the proptest's headline claim is judged by an oracle blind to its primary failure class (the crate itself is covered by the transmit-window pin). Second, the same suite's `bootstrap_into` and `gossip` swallow session errors, including server-side panics, in regimes where any error is unconditionally a crate bug, and its "fully deterministic" claim is false because `Peer::seed()` draws `Network` ids from `OsRng` and the ids decide the tie-break. Third, `common::wire`'s drivers are fixed at `NoBookmark`, so every suite re-implements bootstrap-serving and pair-gossip (losing the drained-control assertion the shared drivers enforce), re-declares `LINK_BUF` and the heal cap, and repeats its preludes. The rest is prose: past-tense bug narrative, em-dashes in `//` comments, fragment lines left by the first-sentence split, and a few dialect tells.


## Positives

- tests/bookmark_causality.rs:696-716: `retire` refuses to swallow the absorber's result and splits decode failures (`InvalidData`, `HandOffMalformed`) from injected disruptions with the reasoning inline. This is the standard the rest of the harness should be held to (finding 12 asks only that it be applied uniformly).
- tests/bookmark_causality.rs:121-158 and 1221-1239: the recycle check runs as the durable set grows so a failure lands on the most-shrunk witness, and a `should_panic` negative control proves the verifier rejects a recycled coordinate.
- tests/bookmark_causality.rs:895-951: `assert_no_leak` is stated as the exact dual of live-party disjointness (claimed zero times versus claimed twice), with a clear argument for why checkpointed-but-not-live regions count as held.
- tests/bookmark_when.rs:40-55 and 261-371: the `Model` predicts I/O from operation semantics alone and never from the crate's suppression arithmetic, so the proptest is a differential oracle rather than the implementation compared with itself; per-step deltas plus a global read-once/read-before-write check.
- tests/bookmark_transmit_window.rs:57-144: `GatedBookmark` turns a durable-write/commit race into a deterministic, replayable interleaving point with `Notify`. The history is instruments-before-cures done right: 077b64db committed `record_dominates_the_transmitted_frontier` red, and ccd88401 committed `cancelled_persist_never_suppresses_the_next_update` "red before this fix" (both commit messages verified); the latter exercises the cancel-safety clause of `Bookmarked::write` through the public API only.
- proptest-regressions/bookmark_causality.txt holds both seeds and they match the explicit reconstructions at tests/bookmark_causality.rs:1276-1286 and 1297-1325 exactly (verified by comparing the plans); the comment at L1263-1267 gives a real reason (strategy range changes re-map cut offsets) for keeping both.
- All four suites drive the public API plus the gated `test-internals` aliases (`dangerously_alias_party`, `sync_window_floor`); no internal protocol entry is exercised, so coverage cannot drift when the public wiring changes.
- tests/bookmark_attach.rs:132-138 and 139-210: `failed_attach_does_not_reclaim_into_an_unbookmarked_peer` constructs the exact recycle shape (a fresh fork attached to a store still holding a dominated previous incarnation over a failing write) rather than arguing it, and asserts both the live party and the on-disk record.
- The runtime choice is stated and accurate where it is stated: tests/bookmark_causality.rs:48-50 correctly contrasts its current-thread runtime with tests/disruption.rs's multi-thread runtime (disruption.rs:44-49 verified), and the three suites that spawn tasks each say why; the when suite uses the quiescence-checked `block_on` so a protocol stall fails at its source.
- tests/common/flaky.rs:210-215 (the harness these suites rest on): `FlakyInMemoryBookmark::store` fails before touching the durable bytes, modelling exactly the atomic-commit obligation the `Bookmark` trait places on implementors, so recovery paths are tested against the contract rather than a lenient stand-in.

## Open questions for Finch

- Generalizing `common::wire`'s drivers (finding 3): generalize `gossip_pair_async`, `bootstrap_fork_configured`, and `sim::quiesce` over `B: Bookmark` (touching a module every suite shares), or land a bookmark-specific driver module beside them? Recommendation: generalize; the `Rumors<T, B>` parameter already exists, and one driver family keeps the drain assertion in one place.
- `LINK_BUF` conventions (finding 2): attach, retire, bootstrap, gossip_when, reuse, and party_conservation use 64K; `common::wire` and the other bookmark suites use 8K. Is 64K a deliberate convention for bootstrap-carrying sessions? Recommendation: import common's 8K in the bookmark suites now; if 64K is load-bearing for bootstrap payloads, name it once in `tests/common` with bootstrap.rs's rationale and have those six suites import it (cross-partition).
- The destruction-class oracle (finding 9): extend the causality simulation with a redaction ledger and a survival check, or accept that the random simulation pins only the degenerate recycle and leave the destruction class to constructed schedules in the transmit suite? Recommendation: the ledger, because the proptest's headline claim is the recycle property and it should be judged by an oracle that can see the primary class; the mutant demonstration then earns its place as a committed check.
- A yielding store in the causality `World` (raised by blind-spots, no finding): the random simulation cannot reach the mid-persist-send cause because `FlakyInMemoryBookmark::store` completes without yielding and the `World` never sends concurrently. Recommendation: not now; the transmit suite owns that class by construction, and the causality module doc should say the mid-persist window is out of its scope.
- Shape of finding 21: negative-space (keep the drift alarm, delete the arm, add the M0 survival witness) or positive-path (make M1 durable and assert it survives)? Recommendation: negative-space; it is accurate about the schedule and costs three lines.
- Suite renames (finding 23): `bookmark_io_schedule.rs` and `bookmark_persist_coverage.rs`? Owner taste; recommendation: rename, since both collisions are with live crate vocabulary.
- The post-donation re-record (finding 24): `Bookmarked::slice` could stage `(sliced party, recorded version)` as the token so the redundant write disappears while fork-then-absorb still differs from the token; the version would have to be the one already in the record, never the live one, or the transmit-window invariant is breached. This reopens ccd88401's "token commits only with the write" design. Recommendation: leave the policy and fix the doc; raise the alternative with the production reviewer only if the extra write per donation matters.
- One binary or four (raised by api-economics, no finding): each `tests/*.rs` links `common` separately; merging the four bookmark suites into `tests/bookmark.rs` with submodules would save compile and link time and relocate the seed to `proptest-regressions/bookmark/causality.txt` under the `tests/main.rs` anchor. Recommendation: measure link time first; do not act on this review alone.

## Dropped

- [0] `bootstrap_into` discards the serving side's outcome: merged into tests-bookmark-12 (same site; that finding covers `gossip` and `revive` too).
- [1], [19], [29] the dead destruction arm (three lenses): merged into tests-bookmark-21, keeping structure-prose's framing because it names the missing M0 witness.
- [2], [30] suite-local drivers (two lenses): merged into tests-bookmark-3, with blind-spots' floor omission [27] and the refutation's hash-only heal fingerprint folded in.
- [3] trailing assert cannot fire: merged into tests-bookmark-14 with [16] (same principle, two files); the refutation rejected [3]'s proposed replacement floor and so do I.
- [4], [37] `LINK_BUF` and heal cap (two lenses): merged into tests-bookmark-2, with transmit's `before::Version` import folded in.
- [5], [27] (staging half), [38] (transmit items): merged into tests-bookmark-20.
- [6], [38] (when items): merged into tests-bookmark-25.
- [10], [25] (provenance half), [35]: merged into tests-bookmark-16.
- [15], [24] (when and transmit items), [25] (the re-records half), [26]: merged into tests-bookmark-4 as one pattern (assertions weaker than their docs or intent).
- [34] `select!` nondeterminism: merged into tests-bookmark-8 as the secondary, undemonstrated source; the `OsRng` source is the definite falsifier.
- [39] `Snapshot::hash` newtype: reframed by the refutation to a test-side alias or inference; the test-side item is folded into tests-bookmark-18 and the API question is below the bar for this partition (no test fights the type once `reference` infers).
- [40] `let _ = self.label;` in tests/common/flaky.rs:199: verified (the field is read by the `Debug` impl at L172-178 and `tests/common/mod.rs:33` allows dead code), but the file is outside this partition; route to the tests/common finalizer.
- Refutation NEW 5 (a scattered seed file at 077b64db^): nothing to do at HEAD; `tests/seed_liveness.rs` guards the shape.
- Blind-spots open question on a yielding store, api-economics open questions on `sync_window_floor` everywhere and on merging binaries: not findings; the floor pin is a crate-wide convention (db2718d4) already ruled, and the other two are recorded above as owner questions.

<!-- source: final/tests-common.md -->
# Partition tests-common: The integration-test common library: sim, schedules, faults, oracles, wire, snapshots, seed liveness

## Partition summary

`tests/common` is the library every category binary under `tests/` builds on. It has three layers. The generators produce inputs that are valid by construction: `action.rs` draws single-peer insert/redact sequences, `schedule/arb.rs` draws multi-peer schedules (with a membership alphabet of mid-schedule bootstraps and retirements) while a shadow simulator tracks what each peer has observed so that every emitted `Redact` names a message its peer holds, `overlap.rs` draws schedules in which hand-driven sessions are opened, parked at chosen poll prefixes, and closed, and `window.rs` draws the per-peer window configuration the suites sweep. The executors run those inputs against real peers: `schedule/executor.rs` one session at a time over in-memory links under the closed-world poller (`wire.rs`), `sim.rs` all at once on a multi-thread runtime over links that `fault.rs` severs at chosen byte offsets, with a value ledger and a custody chain that decide which invariants are sharp. The oracles and instruments are kept structurally independent of the merge machinery: `oracle.rs` is a `BTreeMap` keyed by event index and reads redaction as absence, `peer.rs` keeps an observation log by pull-draining snapshots, `gossip_snapshot.rs` captures every wire byte for the `insta` pins and holds the public observation hook to that capture, `shape.rs` stages deterministic tree shapes, `tcp.rs` and `routed_tcp.rs` instantiate the link contract over sockets, `flaky.rs` fails bookmark storage on a schedule, and `tests/main.rs` plus `tests/seed_liveness.rs` anchor and audit where proptest persists its seeds.

I read all twenty files in full with line numbers, 5189 lines, every one of them test code, and verified the candidate findings against the crate sources they depend on (`src/rumors.rs`, `src/rumors/unordered.rs`, `src/snapshot.rs`, `src/peer/gossip.rs`, `src/link.rs`, `src/testing.rs`, `src/testing/transport.rs`, `src/tree/typed/hash.rs`, `src/tree/typed/untyped/fan.rs`, `src/bookmark/format.rs`, `src/protocol.rs`, `Cargo.lock`) and against git history where a finding turns on provenance. No cargo, just, or test command was run; nothing below rests on a build.

The harness is in good shape. I found no harness bug that masks a failure: every liveness bound panics at its source, every successful in-memory session asserts a drained control stream, the fault engine's error classifier rejects decode-class errors as protocol bugs (keeping the disruption engine a conformance-bug detector, as the model of record requires), the value oracle is gated on possible identity loss rather than on the presence of faults and has a committed tripwire, the schedule shadow is meta-tested against the live executor, and the seed sweep guards its own vacuity and carries fixtures for both verdict classes. Constants carry their rationale where they are declared, and where they are measurements they name the pin that re-derives them.

The dominant issues are housekeeping that a blanket `#![allow(dead_code, unused_imports)]` lets accumulate (two dead imports, a dead field, a no-op statement, a stray scope, all from named commits), duplication between sibling modules and against the test binaries (a second shadow simulator, a fingerprint tuple in twelve places, a bootstrap handshake reimplemented in four suites), two documentation claims that overstate what the code does (an "exact sequence" only the set of which is pinned; a fork "captured" at `Open` that the live session performs at its first poll), one module doc that motivates a number by a data structure removed from the tree the same day it was written, and two coverage duals the fault plans never construct (the donor side of a bootstrap, the absorber side of a retirement). Two findings are medium; nothing is high.


## Positives

- oracle.rs:22-29 implements `Default` by hand so `Oracle<T>` does not acquire a spurious `T: Default` bound: a manual impl with a reason to exist. The oracle is a `BTreeMap` keyed by event index and reads redaction as absence through the public `Snapshot::iter` (73-88), sharing no code with the merge.
- fault.rs draws every stream of a direction on one shared budget and fails the connector and acceptor alongside the writers and readers (198-241), with comments at 202-204 and 228-230 naming the deterministic path each refusal reaches (`SendError::Connect`, the parked accept driver); the `Done` threading at 208-213 and 234-239 preserves the connector's recycle contract instead of discarding it.
- wire.rs:65-93 makes `assert_control_drained` a post-condition of every successful in-memory session (gossip_pair_async:156, bootstrap_fork_configured:262, the executor's retire arm at executor.rs:275), turning a latent control-stream desynchronization into a failure at the session that caused it; gossip_snapshot.rs:449-471 reproduces the same invariant from its log.
- gossip_snapshot.rs:333-387 renders pins from the public observation hook while holding it to the transport capture as a totality oracle (`assert_items_account_for` per stream plus the observed-versus-wire stream count), and the doc at 333-337 says exactly what licenses the pin.
- sim.rs:90-104 derives `MAX_CUT` from a measurement with a two-sided pin (`max_cut_spans_the_envelope_session`, tests/disruption.rs:432) via `fault::metered`, which shares the counters the cuts spend, so generated cuts provably reach every byte of the envelope session.
- sim.rs states its loss accounting (53-71), the custody chain (`lost_custody`, 214-243), and the premise that gates `assert_value_oracle` (961-974) at the declaration sites; the classifier (361-375) admits only truncation and severed-transport kinds and rejects decode-class errors as protocol bugs, keeping the engine on-model; tests/disruption.rs:172 carries the tripwire that the oracle bites known-bad mechanisms; the clippy allow at 597-600 carries its reason.
- schedule/arb.rs keeps generated schedules valid by construction through a shadow simulator that is meta-tested against the live executor (tests/shadow_validity.rs), for both alphabets, comparing sets for a stated reason.
- window.rs puts `Floor` first so failures shrink toward the deadlock-certified baseline (23-26), and each budget endpoint constant names the pin in tests/window_sweep.rs that keeps the sweep from degenerating (85-99); that file exists.
- overlap.rs states why its link buffer is 48 bytes (46-57) and why sides are polled separately (67-77); `SESSION_POLL_BOUND` turns a protocol hang into a failure at its source.
- peer.rs:169-173 and sim.rs:905 turn non-termination into a named failure whose message says what class of bug it indicates.
- flaky.rs designs `FaultFeed` so shrinking toward empty is shrinking toward fault-free (116-122), and `store` fails before touching the durable bytes (210-215), modeling the atomicity the crate's recovery relies on.
- seed_liveness.rs reconstructs proptest's persistence resolution in reverse, guards its own vacuity (220-223), commits a fixture for each verdict class it produces (298-357), and its stated provenance matches Cargo.lock; tests/main.rs is an empty binary whose doc says exactly why it exists and what enforces it.

## Open questions for Finch

- Test-support crate (finding 8): should `tests/common` become a path dev-dependency crate? Recommendation: measure `cargo build --tests --timings` at HEAD and on a branch first; adopt only if the number is material, since finding 10 restores import linting without restructuring.
- `Rumors::send` returning the `Version` it stamped (finding 2): the state-machine argument at src/rumors.rs:190-203 stands; the batching argument does not bind the single-message method. Recommendation: keep the ruling, consolidate the harness's recovery routine, and record that the friction was weighed.
- The overlap harness pins every peer at the floor (overlap.rs:209 and `bootstrap_fork`) while every other schedule engine sweeps windows. Recommendation: give `arb_overlap_schedule` a `WindowAssignment` with `Floor` as the shrink target, unless fork/install overlap is argued regime-independent at the site.
- `Retire::Retired` with absorber `Err` (sim.rs:807-815): with faults only on the retiree's side this arm looks unreachable (`Retired` means the retiree read the absorber's post-commit marker), and no committed seed shows it firing. Recommendation: adopt finding 25, which makes it reachable, and keep the arm; otherwise state at the site that it is conservative and unexercised.
- Cancellation: overlap.rs:81 says dropping an unfinished `Session` models a cancelled one, but no executor drops one, and tests/lifecycle.rs cancels only gossip/gossip. Recommendation: a deterministic cancel-at-every-poll-prefix sweep over a serving bootstrap (guard snap-back before `party.take()`, leak after) and over a retirement, the dual of the cut sweep in finding 22, unless it already exists with forged peers in src/tests.rs.
- Newcomers born mid-chaos (sim.rs:744-750, 768-771) are neither probed by `probe_disjointness` nor given observers until the audit after the chaos phase. Recommendation: hand each newcomer to the prober through a shared `Mutex<Vec<Rumors<u64>>>` as soon as `run_boot` returns, so disjointness is probed over the window in which the fresh fork exists.
- Vocabulary rulings (findings 23 and 24): "honest" as a second sense and em-dashes in `//` comments are both crate-wide. Recommendation: rule each once and sweep mechanically rather than fixing the partition's instances alone.
- The stale seed (finding 32): delete or re-derive? Recommendation: delete; the failure it recorded is fixed, and a fresh regeneration under today's strategy is a different case.
- `WindowChoice::Floor` versus `Budget(1 << MIN_BUDGET_EXPONENT)`: tests/window_sweep.rs pins the latter to one slot. Recommendation: keep both; `Floor` exercises the explicit `sync_window_floor` configuration path, the budget arm the solver path that reaches the same regime.

## Dropped

- pairwise::gossip_converges subsumed by async_wire (api-economics [41]): anchored in tests/async_wire.rs and tests/pairwise.rs, outside this partition; route to the partition owning the test binaries, with the history pass's caveat that proptest-regressions/async_wire.txt holds four cc seeds that must be re-homed into pairwise.txt or seed_liveness fails.
- sanity::arbitrary_schedules_dont_panic re-executes the multi_peer generator (api-economics [42]): anchored in tests/sanity.rs, outside this partition; route likewise.
- "seam" as jargon (part of structure-prose [19]): established crate vocabulary, 48 uses under src/; replacing it in tests/common alone would split the vocabulary. The rest of [19] survives as finding 27.
- Replace the overlap redact guard with `assert!(observed)` (blind-spots [26] resolution): refuted; the guard is load-bearing for a by-design imprecision (finding 12 explains the mechanism and asks for a meter instead).
- Dead `Protocol` import ([30]), unread `ByteMeter.read` ([31]), `let _ = self.label;` ([32]), residue list ([35]), no-op constructs ([16]), `ByteMeter.read` ([1]), suppression leftover ([2]): merged into finding 10.
- Named CBOR constants ([23]), self-check independence ([33]), transcribed internals ([43]): merged into finding 6.
- `fork_tree` duplicate ([5]) and second shadow ([36]): merged into finding 14.
- Pass-through wrapper ([9], [47]), bootstrap helper copies ([10]), retire-into and Option bootstrap ([37]): merged into finding 30.
- `metered` duplication ([45]): duplicate of finding 4. Duplicate bounds ([46]): duplicate of finding 17. Composition map ([48]): duplicate of finding 9. Convergence loop copies ([38]): duplicate of finding 18, and its tests/lifecycle.rs:50-58 citation is the `divergent_pair` fixture, not a loop.
- Refutation's new item 2 (Open doc versus the live fork point): merged into finding 12 as its mechanism.

<!-- source: final/tests-disruption-handshake.md -->
# Partition tests-disruption-handshake: Integration tests: disruption, gossip_when, pipelining, hop trace, handshake and its liveness

## Partition summary

The partition is six integration binaries, all test code, 3439 lines read in full with line numbers (tests/disruption.rs 1009, tests/gossip_when.rs 982, tests/hop_trace.rs 647, tests/handshake_liveness.rs 431, tests/handshake.rs 276, tests/gossip_pipelining.rs 94). To settle points I also read tests/common/wire.rs, the cited ranges of tests/common/sim.rs, tests/common/fault.rs, tests/common/peer.rs, tests/common/action.rs, tests/reuse.rs, tests/lifecycle.rs, tests/window_corners.rs, tests/session_stats.rs, benches/support/latency.rs and wire.rs, src/testing.rs, src/peer/gossip.rs (the driver's idle select, the epilogue, and Drive's Drop), src/peer/bootstrap.rs, src/tree/mirror/handshake.rs and its tests, src/tree/mirror/streaming/window.rs, src/link.rs, src/network.rs, src/protocol.rs, src/peer.rs, src/tree/typed/hash.rs, src/error.rs, tools/testdoc, .config/nextest.toml, Cargo.toml, and the git history of 368da2a5, 212c6914, 6d48d8dc, 2c73d032, 3327a92b.

disruption.rs drives the plan-based chaos engine in tests/common/sim twice: in-process on a multi-thread runtime with in-memory wires cut at byte offsets, and inter-process by re-executing the test binary as TCP children. Around the two proptests it carries the value-oracle adequacy tripwires, the custody-transitivity regressions, the two-sided pin that derives MAX_CUT from a metered envelope session, and four reconstructed counterexample plans. gossip_when.rs pins the whole contract of the cue-driven driver with hand-fed cue streams (reduction to one-shot gossip, suppression exactness, unconditional probes, transitive relay of content and of a redaction frontier, the clean-end and error terminals, poison fail-fast, poll cancel-safety) plus two proptests (severed connections; chaotic tick and commit interleavings). handshake.rs hand-transcribes the 30-byte preamble and drives one-shot gossip against a fake peer for each rejection diagnosis. handshake_liveness.rs runs nine session shapes over the one-byte in-memory link under the closed-world quiescence poller. gossip_pipelining.rs and hop_trace.rs measure serialized wire hops in exact virtual time over the delayed-pipe link, the latter with a byte-level tracer that prints the critical path.

The quality is high where the doctrine puts the most weight. Known-bad mechanisms are constructed and shown to fail the checks they exist for; the fault range is derived from a meter using the counters the cuts spend, and pinned from both sides; deadlocks in the liveness matrix are witnessed deterministically rather than by wall clock; hop fixtures self-check their shape before any hop arithmetic runs; every test has a substantive doc comment and assert messages say what a failure means. The dominant issues are of three kinds. First, two harness holes: the inter-process parent swallows panics from its serving tasks (a JoinSet never joined, then aborted), and the gate's testdoc never sees `#[pollster::test]`, so nine tests here and 29 crate-wide are unchecked. Second, determinism drift in gossip_when.rs: five negative assertions rest on 100 ms wall-clock windows while the same file already uses the deterministic stall witness three functions away. Third, residue and duplication: two ghost references survive the V1 retirement, a hand-maintained count is stale, the pipelining and hop-trace fixtures are byte-identical while one doc claims otherwise, `send_random` exists in seven copies, and the handshake suite's six fake peers read and discard the bytes its module doc says it is an oracle for. No finding reveals a production bug.


## Positives

- disruption.rs value_oracle_tripwires_catch_known_bad_mechanisms (159-219) and value_oracle_survives_committed_retire_chain (268-350) construct both known-bad mechanisms (a suppressed redaction, a dropped insert) and show them failing the checks in the same run that shows the uncorrupted ledger passing: the adequacy discipline done exactly right.
- disruption.rs max_cut_spans_the_envelope_session (352-448) derives MAX_CUT from a metered envelope session using the same counters the cuts spend, pins it from both sides, derives the envelope's value count from the generator's own exported bounds (ENVELOPE_VALUES_PER_SIDE), and states the dominance premise honestly (exact on value count, representative on version shape).
- disruption.rs custody_chain_loss_is_transitive (221-248) records the concrete reviewed counterexample as the test's invariant, so the transitive custody rule carries its motivation.
- handshake_liveness.rs runs every session shape under the closed-world quiescence poller, so a deadlock surfaces as a deterministic stalled-poll failure with the cell name attached; the seasoning fixture self-checks that the greeting version dwarfs the window (GREETING_FLOOR) without pinning greeting bytes; cells assert liveness and convergence only; and every successful session is held to the clean-drain invariant at its boundary.
- hop_trace.rs turns virtual time into an exact instrument (bucket k is hop k), documents the run command for reading the trace, and transfer_pair (528-602) asserts its staged shape (two leaves under one root radix, ballast elsewhere, smaller set initiates) before any hop arithmetic runs.
- handshake.rs transcribes the preamble layout by hand and says why (10-12): an oracle that must not derive its expectations from the code under test.
- gossip_when.rs dropping_next_futures_loses_nothing (574-614) states why it uses a minimal executor (it pins the driver's no-Tokio promise) and runs poll cancel-safety under the quiescence detector; a_driver_on_a_poisoned_link_fails_fast proves the fail-fast without a counterparty; severed_connections_fail_loudly_and_recover asserts the epilogue's certification whenever any Ok appears.
- common::wire::assert_control_drained is applied at every successful session boundary across the partition, turning latent control-stream desynchronization into an immediate failure at the session that caused it.
- Every `#[test]` in the partition carries a doc comment that states a behavior, and assert messages say what a failure means rather than restating the condition.

## Open questions for Finch

- gossip_pipelining's loose bound (`< 24` hops) beside hop_trace's exact pin (`== 7`) on the same fixture: HOP_BUDGET's doc and 814f07ad record it as a regime separator meant to survive a deliberate re-pin of the exact figure. Keep it as such, or dissolve the binary into hop_trace? Recommendation: keep the bound, share the fixture, and add the instrument equality and the measured floor leg (findings 10 and 31); if kept, say in the module doc that the bound is a regime check that does not move when the exact pin is re-accepted.
- `Bootstrap::join` returns `Result<Option<Peer>>` while `BookmarkedBootstrap::join` returns a typed `Joined`; the test corpus double-`expect`s at seven sites (gossip_pipelining.rs:79-81, hop_trace.rs:487-489, handshake_liveness.rs:255-257 and 335-337, disruption.rs:931-935, common/wire.rs:257-259, common/sim.rs:511-513). 55e34738 recorded the asymmetry as deliberate ("nothing can fail after the session"); the join doc states what `Ok(None)` means but not why the shape is an Option. Recommendation: state the type-shape rationale inline in `join`'s doc and leave the API; a reshaping toward a shared outcome vocabulary is an owner decision the seven sites do not by themselves justify.
- Is the OS-process boundary in disruption.rs load-bearing? The library holds no process-global state the children could share, and a dying peer's ConnectionRefused can be produced in-process by dropping a listener. Recommendation: state in the module doc what the boundary proves (real sockets, process death mid-session, a separately-initialized runtime); if nothing, the exec protocol could dissolve into an in-process TCP variant of run_plan.
- Should ChildPlan gain a redaction step so the TCP leg covers deletion honoring and the value ledger (finding 4)? Recommendation: yes; a child redacting one of its own sends before the final gossip, with the parent asserting the redaction's absence, is small and closes the leg's only structural blind spot.
- Can gossip_when's two proptests run under `run_to_quiescence` instead of the reused tokio runtime with 10 s watchdogs, making a wedge a deterministic Stalled rather than a 10 s wait? `Op::Pump` uses `tokio::task::yield_now`, which may need a runtime. Recommendation: try it; if yield_now works without a runtime the DEADLINE watchdogs on those tests can go too.
- Finding 7's severity: the rubric puts a harness bug that masks failures at high; the refutation pass argued medium because the masked class is confined to the TCP serving path. I kept high because that path is the engine's unique coverage. Recommendation: fix regardless; the severity only affects scheduling.

## Dropped

- [43] `Bootstrap::join` returns `Result<Option<Peer>>`: deliberate and recorded (55e34738); the seven double-expect sites are not new evidence; moved to open questions.
- [47] `sync_window_floor` on every alice in handshake.rs: refuted; pinning the floor is the documented suite convention (window.rs:549-553, wire.rs:209-212) and needs no comment at the sites. The seed_rng half survives in finding 32.
- [51] Heal loop never re-checks convergence after its final mesh round (tests/common/sim.rs:891-905; same shape at tests/common/peer.rs:155-160): out of this partition's file list; a false failure only, never a masked pass. Relay to the tests/common reviewer.
- [42] HOP_BUDGET's floor regime is argued, not constructed: merged into finding 10 with the refutation's correction that window_corners measures the floor (budget 0 floors every capacity at one) on a different fixture.
- [4], [40] Dissolve gossip_pipelining into hop_trace: the bound's coexistence with the exact pin is a recorded decision that holds; the actionable residue (shared fixture, instrument equality, measured floor) lives in findings 10 and 31 and the dissolve question is in open questions.
- [18]'s 'real TCP', 'real peer', 'real parallelism', 'real sessions' sites: refuted; each contrasts with a simulated or hand-driven counterpart and carries mechanism.
- Refutation new item 3 (a note at window_corners.rs that budget 0 is floor-equivalent): out of this partition's file list; worth a one-line note there or in `Peer::sync_memory_budget`'s docs.
- Duplicates merged: {0, 24, 38} into finding 14; {1, 41} into 9; {2, 44} into 4; {3, 40} into 31; {4, 28, 40, 42} into 10; {5, 29, 39} into 22; {6, 39} into 21; {8, 34, 45} into 20; {10, 46} into 30; {11, 12, refutation new 4} into 12; {15, 35} into 6; {19, 27} into 5; {21, 47} into 32; {25, 50, refutation new 1} into 17; {48, refutation new 2} into 11.

<!-- source: final/tests-lifecycle.md -->
# Partition tests-lifecycle: Integration tests: bootstrap, retire, lifecycle, membership, multi-peer, pairwise, partition, reuse, redaction

## Partition summary

This partition is the crate's public-API behavioral suite for the replica lifecycle, at commit 9e5784fb. Fifteen integration binaries (3171 lines, all test code) cover: bootstrap over the wire and every arm of the bookmarked builder's `Joined` outcome (`tests/bootstrap.rs`), with byte pins in `tests/bootstrap_snapshot.rs`; retirement outcomes, content survival, and a plain-gossip differential oracle (`tests/retire.rs`, `tests/retire_redaction.rs`), with byte pins in `tests/retire_snapshot.rs`; the session promise, link poisoning, and the post-commit `Epilogue` residue (`tests/lifecycle.rs`); connection reuse and the epoch wrap (`tests/reuse.rs`); the per-universe `Network` guard (`tests/network.rs`); single-peer batch semantics (`tests/single_peer.rs`); the algebraic laws of one gossip session (`tests/pairwise.rs`); redaction corners (`tests/redaction.rs`); and the schedule-executor family checked against the spec-shaped oracle (`tests/multi_peer.rs`, `tests/partition.rs`, `tests/membership.rs`, `tests/sanity.rs`). To settle the findings I also read the harness these suites stand on (`tests/common/wire.rs`, `action.rs`, `peer.rs`, `oracle.rs`, `mod.rs`, `window.rs`, `tests/async_wire.rs`, about 890 lines, plus cited ranges of the schedule executor, the shadow generator, `flaky.rs`, and `sim.rs`) and the crate sources the testdocs cite.

The suite is in good shape. Every session runs over in-memory links under the closed-world poller (`common::wire::block_on` is `run_to_quiescence`), so a protocol stall fails deterministically at the offending poll. Every successful session in the partition ends in `assert_control_drained`, and the assert carries its own committed negative control. Several tests assert their own premises (the zero-stream check in `reuse.rs`, the budget placement in `lifecycle.rs`, the empty-store precondition in `bootstrap.rs`), and `membership.rs` pins the liveness of its generated dimension. The oracle is pure data that never invokes the merge. Every test carries an English testdoc. No residue of the V1 protocol, BLAKE3, or the height and item erasure appears in these files; there are no ignored tests, no commented-out code, and no debug prints.

The dominant issues are prose written on one side of a design boundary and carried across it unchanged, and harness plumbing re-spelled per binary. Four boundaries explain nearly every stale sentence: retire reconciling before it relinquishes the party (fedb3ecb2), the callback API's removal and the shared-state port (db32b94d9, 1d3a3df4d1), the `rumors::sync` surface's removal (83edcd944), and the per-stream link (b3b877d9bf). The concrete casualties are a `retire.rs` header and testdocs describing a domination precondition and declines the `Retire` contract does not have, a `partition.rs` module doc naming an `on_message` callback and a `key` that exist nowhere in the crate, a `redaction.rs` testdoc claiming the public docs are silent where `Rumors::redact` speaks, a `single_peer.rs` testdoc citing a batch-docs promise that was deleted, and `async_known` plus a section header still qualified against a sync surface that is gone. On the structural side, the "serve a bootstrap" and "retire over a link" sessions are written out ten times across the partition and its neighbors instead of once in `common::wire`, and one copy has already drifted (the bookmarked join and the mutual bookmarked bail skip the drain check every other driver runs). Three verification gaps are worth scheduling: nothing pins that redactions stay in the generated populations (the membership alphabet has exactly this floor; the redaction dimension does not), two `redaction.rs` tests assert quantities a redaction cannot change while leaving the documented no-op contract unpinned, and the epoch-wrap test never reads the public `SessionState::epoch()` it exists to exercise.


## Positives

- Session-boundary hygiene is uniform: every successful in-memory session in the partition ends in `assert_control_drained` (the two exceptions are tests-lifecycle-7), and reuse.rs:228-267 commits the negative control that plants one byte and requires the assert to fire, so the gate is itself gated against rot.
- The `Joined`-arm tests in bootstrap.rs (234-411) each name and run their negative control (empty-store precondition, retry through the returned bookmark against a live provider, the fault-free identical join), so the arm reached is attributable to the injected condition.
- lifecycle.rs's `a_lost_epilogue_marker_is_distinguished_and_post_commit` (166-234) measures the clean byte schedule on a byte-identical probe pair and replays one byte short; its error-class and content assertions would fail differently under a misplaced cut, so the test validates its own instrument. The cancellation test (73-94) brackets the cut both ways: each poll asserted pending, then stream counters prove the descent had begun.
- `membership_population_contains_churn` (membership.rs:99-135) is a liveness floor on a generated dimension under the deterministic runner, with a doc stating exactly what would rot without it: the pattern tests-lifecycle-15 asks to extend.
- `empty_sessions_advance_epochs_in_lockstep` (reuse.rs:153-168) asserts its own premise (zero data streams in the converged session) through the wrapped links' counters instead of assuming it, and says why.
- Deterministic floor legs sit beside every swept leg (membership.rs:85-96, multi_peer.rs:168-180), keeping the capacity-one orderings the deadlock argument certifies exercised on every run rather than with generated probability.
- Content checks go through the `readout` / `readout_multiset` lens rather than party state, and retire.rs's wire-equivalence properties (352-443) use the crate's own plain gossip as the differential oracle: an oracle-shaped family stated as a property.
- The schedule executor never swallows an outcome: every `Retire` variant other than `Retired` panics with a message naming why a clean wire forbids it (executor.rs:266-273), and `quiesce_refs` (peer.rs:140-174) stops on identical fingerprints, the fixed point itself, with a bounded loop whose panic names the two things non-convergence could mean.
- single_peer.rs pins the batch lifecycle through the public API at every exit (Ok, user Err, depth Err, panic), including the admission-stops-at-rejection count and the one-tick-per-commit observer check.
- The snapshot suites fix the A/B party convention and its exceptions at the top of the module (bootstrap_snapshot.rs:14-21, retire_snapshot.rs:17-25), exactly what a reader of a hexdump needs.
- No leftovers of the V1 protocol, BLAKE3, or the height and item erasure; no ignored tests, commented-out code, or debug prints anywhere in the partition.

## Open questions for Finch

- reuse.rs runs under `#[tokio::test]` with a 10 s deadline (tests-lifecycle-27). Is real-executor coverage the intent? Recommendation: convert to `block_on` per round; if the executor is the point, say so in the module doc and keep the deadline.
- `Batch` once promised strictly increasing per-action versions and no longer does (tests-lifecycle-32). Restore the clause as a public promise, or reword the testdoc? Recommendation: reword; restore only if users are meant to rely on within-batch ordering, since a batch's versions carry no input-order correspondence at recovery time.
- `Peer::seed_rng` and `warm_caches` are `#[doc(hidden)] pub` with no `cfg` (tests-lifecycle-10). Document or gate? Recommendation: gate both under `test-internals` unless deterministic `Network` seeding is a capability you want application test suites to have; if it is, document the shared-`Network` hazard.
- The dead `# shrinks to n = 1` seed line (tests-lifecycle-1). Recommendation: remove it in a commit naming the deleted property, and extend `seed_liveness.rs` to match `cc` parameter names against live `proptest!` signatures so the class cannot recur.
- multi_peer's one-named-test-per-invariant policy (80a3155f41) versus fusing the two implied readout checks (tests-lifecycle-9). Recommendation: fuse the two implied checks into the canonical-map test with distinct messages, and state the policy in the module doc for the tests that remain separate.
- `Rumors::send` returns no `Version`, and the test corpus recovers it six ways (tests-lifecycle-31). Recommendation: keep the decision and add the recommended recovery idiom to `send`'s docs; revisit only if application code shows the same six shapes.
- Rename the harness `common::peer::Peer` (for example `SimPeer`) to release the forced `rumors::Peer::` qualifications in redaction.rs and sanity.rs (tests-lifecycle-12)? Recommendation: yes; it is a mechanical rename inside tests/.
- Are the pairwise laws deliberately restricted to disjoint-content populations, with the schedule engines the designated home for shared-then-redacted content (tests-lifecycle-17)? Recommendation: yes, and write that division into pairwise.rs's header; extending the laws' population is optional.
- Should retire-into-bootstrap enter a generative alphabet (party_conservation's `Op`, or the membership executor), or is the point test plus the snapshot pin sufficient (tests-lifecycle-23)? Recommendation: add the party assertion to the point test first; a generative arm is a nice-to-have.
- None of the schedule-executor binaries set a `ProptestConfig`, so each property runs the default 256 cases at up to 8 peers and 50 events, while session_overlap (48), disruption (8), and party_conservation (32) budget explicitly. Chosen or inherited? Recommendation: if 256 is fine on the gate's wall clock, leave it and say so in one module doc; otherwise budget the swept legs and keep the floor legs at 256.

## Dropped

- Harness self-tests are scattered across three behavioral suites [18]: refuted; each placement has a stated local charter (sanity.rs names "degenerate inputs", reuse.rs's negative control guards the invariant reuse.rs exists to pin, membership.rs's floor guards its own generator), and no doctrine rule requires centralizing them; taste with a defensible status quo.
- The window sweep never configures the newcomer's side of a bootstrap session [41]: refuted by src/peer/bootstrap.rs:123-128, which documents that the window setting has nothing to bound during a join (an empty replica disputes no subtrees); the sweep's asymmetric-window claim concerns reconciliation sessions, where it holds.
- tests/common/mod.rs's composition map omits `overlap` and `shape` [52]: verified true, but out of this partition (tests/common); route to the harness partition so it is not lost.
- Em-dashes in `//` comments at reuse.rs:144 and 156 [17]: below the bar; the repo holds no such convention (the justfile's comments use em-dashes throughout and tools/ has no dash lint), so this is Claude's own writing discipline, not a finding against the tree. The "silently destroy" tell from the same candidate is folded into tests-lifecycle-4 and the scare quotes into tests-lifecycle-12.
- `durable_bookmark` re-spells the flaky-bookmark construction [19]: below the bar; two sites share the shape (bootstrap.rs:200-204 and bookmark_attach.rs:54-56, the latter outside the partition), the constructor would live in tests/common, and the refutation showed bookmark_causality.rs's per-peer labels would not use it.
- wire_bootstrap duplicates bootstrap_fork_configured [0]: merged into tests-lifecycle-3.
- retire.rs's LINK_BUF cites other suites' headroom [1] and LINK_BUF redefined per binary [45]: merged into tests-lifecycle-2.
- seeded() copied into five binaries [2], [50]: merged into tests-lifecycle-8 (four identical copies; opening_supply's is a distinct fixture).
- retire_redaction.rs restates a retire.rs claim [3], [39]: merged into tests-lifecycle-26 and tests-lifecycle-25.
- partition.rs cites on_message [5], [32], [36]: merged into tests-lifecycle-19, which adds the `key` ghost.
- redaction.rs testdoc says the docs are silent [6], [43] and the no-op tests are vacuous [24]: merged into tests-lifecycle-21.
- batch docs promise [7], [33]: merged into tests-lifecycle-32.
- Lenses re-implemented per suite [10]: fingerprint and canonical map in tests-lifecycle-14; version-by-payload in tests-lifecycle-31.
- retire.rs duplicate oracle block and subsumed point tests [11]: merged into tests-lifecycle-25.
- Twin bootstrap bodies [12] and local pair builders [20]: merged into tests-lifecycle-6.
- reuse.rs tokio and deadline [13], [42]: merged into tests-lifecycle-27.
- sanity.rs brace blocks [14]: merged into tests-lifecycle-13 with `dup` and single_peer.rs's `sync_window_floor`.
- async_known relic [15], [44]: merged into tests-lifecycle-24, which adds the line-112 section header and the seed-call-site doc inaccuracy.
- Import idioms [16], [51]: merged into tests-lifecycle-12 and tests-lifecycle-13.
- Epoch-wrap constants hand-computed [21] and epoch never read [26]: merged into tests-lifecycle-28.
- retire header misplaces party accounting [49] and the bootstrapper cross omits party checks [29]: merged into tests-lifecycle-23.
- "Serve a bootstrap" spelled five times [37]: merged into tests-lifecycle-3.

<!-- source: final/tests-observation.md -->
# Partition tests-observation: Integration tests: causal and unordered observers, changes, observe hooks, listen, session overlap and stats, party conservation

## Partition summary

This partition is the public-surface behavioral suite for everything an application observes of a replica. Three set observers are driven step by step with `now_or_never`: `UnorderedMessages` (tests/listen.rs), `CausalMessages` (tests/causal.rs), and `Changes` (tests/changes.rs). The wire observation hook is held differentially against the recording-link capture in both directions (tests/observe.rs). The per-session counters are re-checked where an application reads them, with the byte counters compared against an independent transport-level tally (tests/session_stats.rs). The overlapped-session regime has a total deterministic sweep and a generated twin (tests/session_overlap.rs), the schedule generator's shadow simulator has a meta-test against the executor (tests/shadow_validity.rs), the ITC identity algebra is pinned over lifecycle schedules (tests/party_conservation.rs), and one regression pin covers the stale-floor hazard (tests/stale_floor.rs). All 3230 lines read are test code; every file in the partition is a test binary under tests/, and I also read the shared harness they lean on (tests/common/wire.rs, oracle.rs, overlap.rs, schedule/executor.rs, peer.rs, sim.rs) and the production docs the tests pin.

The suite is in good shape. Invariants are stated in English and the bodies check them; families are proptests with committed seeds; the oracles are independent of the code under test (a transport capture for the hook, a transport tally for the byte counters, a BTreeMap oracle for the overlap fleet, `Party::seed()` for the identity fold); every session in the partition runs under `run_to_quiescence`, so a stall fails at the poll instead of hanging; and every in-memory session that owns both link ends finishes with `assert_control_drained`. The negative control in listen.rs and the restart-shaped at-least-once tests in causal.rs are model examples of constructing the counterexample rather than arguing about it.

The dominant issues are residue of mechanical passes that each stopped one step short: fourteen `§6.n` tags from a plan file deleted from the tree; "borrowed faces" and "lent" vocabulary from the dissolved lending API; braces left around single `send_all` calls after the batch-closure rewrite; a first-sentence split that left two-word fragments on their own lines. On structure, the two observer suites carry the same `Step`/`step`/`drain`/`live_map` helpers where the public `try_next` and `oracle::readout` already exist; a retire driver and a 64 KiB link constant are copied across six suites with a rationale the harness itself falsifies (the schedule executor retires at 8 KiB). Three verification gaps deserve scheduling: the gate's `testdoc` regex does not recognize `#[pollster::test]`, so seven tests in changes.rs sit outside doc enforcement (verified by running the tool); the session-stats conservation property never generates a redaction, so its `shed` term is identically zero; and the overlap generator's shadow has no validity meta-test while its executor degrades a mismatch to a skipped event. One contract disagreement is an owner ruling: causal.rs pins a replica-independent delivery order for concurrent messages that the public `CausalMessages` doc deliberately declines to promise.


## Positives

- Every wire session in the partition that owns both link ends finishes with `assert_control_drained` (listen.rs:263, party_conservation.rs:113, session_stats.rs:39, and every `common::wire` driver), turning a latent control-stream desynchronization into a failure at the session that caused it.
- Every session runs under `common::wire::block_on`, which is `run_to_quiescence(..).expect(..)`: a wire stall fails at the stalled poll with a verdict rather than hanging (changes.rs is the one exception, tests-observation-8).
- session_stats.rs's `CountingWrite`/`CountingConnector` (161-211) tally at the transport layer, so `bytes_sent` is checked against a number the crate's own counters cannot influence; the label exclusion is accounted for explicitly; the inline `Link<...>` return type with `#[allow(clippy::type_complexity)]` (215-227) follows the doctrine of not coining a synonym to appease the lint.
- listen.rs's `folding_delivered_versions_can_lose_a_message` (464-526) is a committed negative control: it searches deterministic universes for the path-before-version shape, shows the fold loses a message, and shows `checkpoint()` re-delivers it. That is the argument for the API shape, built rather than stated.
- causal.rs's `restart_replays_every_unhandled_message` (536-633) and `final_pop_checkpoint_still_replays_the_last_message` (651-682) model the persist-after-delivery crash protocol on both observer faces, round-tripping the checkpoint through `as_bytes`/`Version::decode` and resuming against a replica rebuilt from a survivor: the realistic restart shape.
- changes.rs's `gossip_frontier_only_advance_ticks_the_observer` (87-137) names the contract clause, the mechanism that makes the case special (`Tree::join`'s changed flag excludes ceiling-only advances), and why `now_or_never() == None` there would be a lost wakeup rather than a pending one.
- observe.rs holds the hook's view against an independent transport capture byte for byte in both directions and separately proves observed-versus-unobserved wire identity over whole universes (366-403), explaining at the fixture (259-260) why the network id must be drawn from a fixed seed.
- session_overlap.rs's sweep is total: it calibrates the poll ceiling on the same fleet shape (81-93), sweeps every redaction target and every parking prefix, and collects violations instead of stopping at the first, so a regression reports its whole footprint. tests/common/overlap.rs:46-57 states why a 48-byte link buffer is the point of the harness.
- party_conservation.rs states four invariant families with the model each rests on (12-37), checks disjointness pairwise so a violation names the pair (66-67), gets the index arithmetic after `fleet.remove(r)` right (178-180), and places the reduced-case-count rationale as a comment on the `proptest_config` it constrains (386-392).
- shadow_validity.rs exists at all: a meta-test that the generator's validity guarantee holds against the executor for both alphabets, compared set-wise with the reason stated (25-26).
- Every proptest in the partition whose per-case cost is a wire session states why its case count is pared (observe 24, session_overlap 48, session_stats 24, fleet-scale 32).

## Open questions for Finch

- Causal delivery order (tests-observation-3): promote the replica-independent `(rank, canonical bytes)` order into the public `CausalMessages` contract, or keep the public under-promise and re-label the two tests as internal-order pins? Recommendation: promote. The order is a function of the set alone, the field doc already relies on it, and a user building a replicated log on `CausalMessages` will want to rely on it too; the materialized-backlogs note treats global rank order as load-bearing.
- `encoded_bits` and the `meter` dev-feature (tests-observation-35): drop the redundant assertions and the feature, or keep the size check as an implementation-independent statement of the fragmentation bound? Recommendation: drop; `Party`'s doc commits to byte-level equality by design, and a dependency feature whose only consumer is an implied assertion is the circular-justification tell.
- Seed comments (tests-observation-38): keep the third shadow_validity `cc` line (with its stale note removed) or strip it; delete or reword the six "minted" comments? Recommendation: keep the hash, delete every comment; provenance lives in git.
- `converged_trio` (tests-observation-22): was the default window on `a` needed to reproduce the discovering incident, or is it an omission? Recommendation: if the incident reproduced only at the default window, say so in the doc and keep it; otherwise pin the floor for uniformity.
- tests/stale_floor.rs: the same observable is a committed property in tests/bootstrap.rs (`bootstrap_reproduces_a_fork`, 82-87), which predates your 2026-07-23 ruling to keep the pin. Fold the newcomer-side check (`b_has`) into that property and delete the file, or keep the named pin with its doc fixed (tests-observation-26)? Recommendation: fold and delete, since the property already asserts the provider side over arbitrary histories; but this reopens a ruling and I have no new evidence beyond the duplication you presumably knew of.
- Does any retire path actually need a 64 KiB link (tests-observation-32)? The schedule executor and bookmark_causality.rs retire at 8 KiB by reading, and the link contract promises flow control at any positive capacity. I could not run the suite to confirm; if a retire session stalls at 8 KiB under `run_to_quiescence`, that is a production finding, not a test-constant one.
- `Changes` yields on `latest()` changing (src/rumors/changes.rs:90-93, 147-150) while the gossip commit notifies on `peer_retiring || tree_changed || ceiling_advancing` (src/peer/gossip.rs:865-870). A content change with no frontier advance would be a lost tick. The blind-spots lens and I could not construct one (every gain brings a version outside the local frontier; every shed requires the peer's frontier to dominate the leaf, which the same join brings in). If you see a path, `Changes` under session overlap is untested; if not, one sentence in the `Changes` field doc stating why `latest()` suffices would close the reading.
- tests/common/peer.rs's `drain` reimplements the `UnorderedMessages` pass via `Snapshot::range(since(checkpoint))`; every schedule and overlap suite validates against this reimplementation, not the public observer. Is a one-property bridge wanted (drain a harness `Peer` and an `UnorderedMessages` on the same replica in lockstep and assert set-equal observations), so the public observer cannot drift from the harness's model of it unnoticed? Recommendation: yes, one property in listen.rs.
- Should the overlap shadow snapshot at the first `Step` (the real fork point, after the preamble exchange) rather than at `Open` (tests-observation-28)? Answering this properly wants the meta-test in place first.
- listen.rs:5-6 points readers to the `Snapshot::range` differential in src/tree/tests.rs, but the test there (`range_and_freeze_match_the_naive_filter`, 886) exercises `Tree::range`/`Tree::range_owned`, internal entries. Whether a public-surface differential for `Snapshot::range` exists is a question for the tree partition's reviewer; the pointer here should name whichever test covers the public method.
- With `Protocol` down to one variant, does `SessionInfo.protocol` stay on the hook surface? The V1-retirement note keeps it as wire vocabulary; this is a production API decision outside the partition, and tests-observation-13 follows either way.
- The total sweep in `overlapped_install_never_loses_innocent_messages` runs on the order of 25 × (session_polls + 1) fleets, each with two bootstraps, one 48-byte-buffer overlapped session, one gossip, and a convergence of at least three sessions; no committed wall-time figure exists for it, and nextest's 180 s terminate is the only guard. Worth one measured number in a note, not a code change.

## Dropped

- [26] retire_ends_the_observer doc (blind-spots): duplicate of tests-observation-17.
- [37] observer helpers (blind-spots) and [41] (api-economics): duplicates of tests-observation-2.
- [3] gossip_pair and [4] corpora (structure-prose): merged into tests-observation-12 with [45].
- [36] LINK_BUF (blind-spots) and [42] retire driver (api-economics): merged into tests-observation-32 with [8].
- [43] pollster (api-economics): merged into tests-observation-8 with [13].
- [46] § tags (api-economics): duplicate of tests-observation-15; its count of thirteen corrected to fourteen.
- [48] braces (api-economics): duplicate of tests-observation-16.
- [52] Protocol::V2 (api-economics) and [53] opened predicate: duplicates of tests-observation-13 and -10.
- [54] imports (api-economics): merged into tests-observation-21 with [17].
- [47] LABEL_LEN (api-economics): merged into tests-observation-24 with [21].
- [49] seed comments (api-economics), [33] and [35] (blind-spots): merged into tests-observation-38; [35]'s claim that the "mint" purge did not reach proptest-regressions/ is corrected (2c73d032 exempted the file deliberately); [33]'s attribution to ce3664dd is corrected to 390160a76.
- [51] converged_trio window (api-economics): duplicate of tests-observation-22 with [38].
- [24]'s sub-claim that a KV-backed backlog could invert rank within a pass: contradicted by the materialized-backlogs note ("rank-ordered by construction"); the finding survives without it as tests-observation-3.
- [0]'s sub-claim that the repeated §6.6 and §6.9 tags are ordering errors: the recovered plan carries a "Negative control" under item 6 and a "Variant" under item 9, so the doubles are faithful to the plan; the finding survives as orphaned tags (tests-observation-15).
- [32]'s resolution to delete tests/stale_floor.rs: reopens Finch's 2026-07-23 ruling, and the bootstrap.rs family pin it cites predates that ruling, so it brings no new evidence; moved to open questions, with the doc-quantifier defect kept as tests-observation-26.
- [39]'s step (a) "assert instead of guard" as an immediate change: refuted in part (the shadow can over-approximate before the fork point is fixed); retained as the conditional third step of tests-observation-28.
- [22]'s generation-time framing: not load-bearing (`Index::index(n)` also takes its bound at execution time); the shrinking argument survives as tests-observation-6.
- [6]/[52] owner-gating: dropping the vacuous assertion does not touch the `SessionInfo.protocol` field the retirement note keeps, so tests-observation-13 is not owner-gated.
- Refutation new item 3 (three private `LINK_BUF = 8 * 1024` copies in the bookmark suites): out of partition; listed as a related sweep in tests-observation-32's resolution.
- Refutation new item 4 (executor.rs's per-variant retire diagnostics): folded into tests-observation-32 as the diagnostic to keep.
- Refutation new item 5 (trailing commas inside `tokio::join!(..., )` at observe.rs:283, listen.rs:261, retire.rs:60): below the bar; rustfmt does not reflow macro bodies and the commas carry no cost.
- observe.txt seed shrinking to an all-empty universe (blind-spots open question): no comment, no defect; nothing to do unless the seed-comment ruling in tests-observation-38 adopts a uniform note.

<!-- source: final/tests-resource-link-window.md -->
# Partition tests-resource-link-window: Integration tests: allocation meters, message size, async wire, latency, opening supply, routed and TCP links, window suites

## Partition summary

This partition is the crate's instrument bench. Two allocator meters (`tests/decode_alloc.rs`, `tests/encode_alloc.rs`) price the wire codec's declared-length reads and frame writes in bytes and allocation events through `rumors::testing` entries. `tests/target_message_size.rs` captures the wire to prove the run-batching setting reaches the greeting and binds both encoders. `tests/async_wire.rs` holds concurrent `Rumors::gossip` to a union oracle. Three conformance drivers run concrete transports through the public `rumors::conformance::link::check`: the bench harness's delayed pipe (`tests/latency_link.rs`), the per-session TCP link (`tests/tcp_link.rs`), and the routed adapter over sockets and the in-memory network (`tests/routed_link.rs`, which adds a pooling dialer and a three-node mesh beside a stalled header). `tests/opening_supply.rs` pins question ownership on the wire. The window family (`window_census`, `window_corners`, `window_knee`, `window_operator`, `window_sweep`, and the ignore-gated `tradeoff_probe`) holds the sync-memory-budget derivation against measured node residency and exact virtual-time hop counts read off `benches/support/latency.rs`'s paused-clock wire, which seven of these binaries compile in by `#[path]`.

I read all fourteen files in full with line numbers, 3,111 lines, every one of them test code; I also read the src and tests/common sites each finding cites, the bench support module, `.config/nextest.toml`, the justfile and CI recipes, and the refutation pass's run log. I ran no cargo, just, or test command myself; the one test run cited below was performed by the refutation pass and I read its log.

The quality is high where it matters most. Every test carries a doc comment, and in every file the body asserts what the doc states, with one exception discussed below. The meters pair ceilings with liveness floors and calibrate their own harness overhead; the capture suites carry negative controls that prove their comparators can see a difference; the knee suite places each measured cell against the derivation for its own session shape and asserts the landing; the sweep suite pins the liveness of a generator dimension. The dominant defects are structural and prosaic. One is a genuine verification gap: `window_census`'s headline admittance test runs both arms of its differencing at the identical all-ones window, because its "tight" budget sits below the derivation's flat pre-charge, so its central assertion is `X <= X + admitted` today. The rest are duplication (ten copies of one bootstrap fixture, three copies of one capture parser, two copies of the allocator scaffolding) and hand-transcribed numbers that drifted from the constants they restate (`28 + m` where the crate ships 43; a nextest comment describing a wall-clock bound removed five weeks ago).


## Positives

- decode_alloc.rs is a model meter: every ceiling has a liveness floor (`framing_full_delivery_meters_at_least_payload`, `supply_full_delivery_meters_at_least_payload`); the non-power-of-two `HONEST_ODD_LEN` defeats the doubling-overshoot masking a power-of-two length would allow, with the reason stated at 35-41; the allocation-event ceiling catches a per-granule reservation policy whose byte reading would look identical; the typed-error assertions keep the zero-delivered ceilings from passing on an early reject, and lines 124-126 say exactly why.
- encode_alloc.rs calibrates its own harness overhead with a committed test (`harness_allocations`) so the frame constants state the writer's allocations alone, and asserts written length positive so an exact-count pin cannot pass vacuously.
- target_message_size.rs pairs every count comparison with a negative control: `sync_memory_budget_is_not_wire_visible` proves the comparator can see a difference through a wire-visible setting (337-346), and `nonzero_minimum_binds_both_encoders` states the rejected reading and carries per-direction margin self-checks (238-253) so the tuple equalities cannot hold vacuously.
- window_knee.rs couples derivation to measurement: each cell is placed by monotone search against the binding capacity of its own session shape, and the landing assertions (136-141, 162-167) fail loudly if a derivation change displaces a cell, instead of silently measuring a shape the prediction does not cover; every constant carries its rationale and headroom argument.
- window_sweep.rs pins the liveness of a generator dimension in all three ways it could rot (arm coverage, width actually granted, budget actually reaching the solve), which is rare and exactly right.
- benches/support/latency.rs argues the measurement model once and enforces it: `hops_on_lattice` refuses off-lattice drift instead of rounding, `round_trip_virtual` refuses to report a virtual figure on a running clock, and latency_link.rs pins both contracts plus run-to-run determinism, so the window suites' load-independence claim rests on committed checks.
- The conformance drivers run each transport through the public suite, in both seat orientations where construction paths differ (routed_link), under explicit timeouts, with the reason real sockets need real time stated once and well (tcp_link.rs:7-11); `pooled_mutual_sessions_converge` names the regression it pins and why the multi-thread flavor is load-bearing (218-225).
- opening_supply.rs stages its disputed-sibling shape deterministically and self-checks the landed shape before asserting on the wire, so fixture drift fails at the fixture rather than as a mysterious count.
- window_corners.rs's real-clock leg asserts only the never-undercharge direction and states why that is the load-immune one (206-212).

## Open questions for Finch

- tradeoff_probe (finding 18): wire it (a `just tradeoff-probe` recipe in the CI `instruments` job beside `worst-cases-pin`) or retire it? Recommendation: wire it. Its design-corpus and design-record cells are covered by nothing else, and `Peer::sync_memory_budget`'s public rustdoc already quotes its figure; a scheduled run makes that citation honest at the cost of ~11 s release per run.
- Shared measurement fixture (finding 22): route the window suites through `tests/common` (8 KiB `LINK_BUF`, `run_to_quiescence`, `assert_control_drained`) or through a lighter `#[path]`-included support module beside latency.rs? Recommendation: `tests/common` with a capacity parameter on the fork helper; history shows compile weight was never the recorded reason for avoiding it, and the stall detector is the point.
- Replacement `TIGHT_BUDGET` for window_census (finding 20): any value works once the fixture asserts its own liveness; the number should come from a measurement, not from me. Recommendation: pick the smallest power of two whose capacities sum exceeds 33 at 21,024 messages a side, and record the measured `windowed - floor` as the new floor's value.
- Is `cargo test` (non-nextest) a supported entry for this crate (finding 19)? If yes, the census lock is a correctness fix; if the justfile is the only sanctioned entry, it is a consistency nicety. Recommendation: add the lock either way; it costs three lines and matches the sibling meters and R12's precedent.
- Exact hop pins versus headroom bands (finding 24). Recommendation: keep the bands, drop the recorded measurements from the comments, and let the `eprintln!` lines be the record; the bands' rationale is already inline.
- Outside this partition but raised from it: `Peer::sync_window_floor` derives the same all-ones table as `sync_memory_budget(0)` (verified by reading `from_budget`); the R20 ruling asked for an explicit opt-in, not a solve-independent variant, and no record argues `Fixed` over `Budget(0)`. Recommendation: keep `Fixed` for solve-independence and say so at the declaration (peer.rs:472-479, window.rs:512-518), so the next reader does not re-derive the equivalence.
- Outside this partition, API surface the tests lean on: `Peer::seed_rng` is public but `#[doc(hidden)]` with no cfg gate (peer.rs:210-213), and `run_to_quiescence` lives behind `test-internals`, whose manifest warning ("never enable it in an application") was justified by a behavior R20's fix removed. Recommendation: document `seed_rng` with the two-universes caveat, and consider a documented home for the stall detector; both are owner decisions for the api-core partition.
- gossip_pipelining.rs (another partition) is a one-cell knee test at `DEFAULT_SYNC_MEMORY_BUDGET` with its own fixture copies and a V1 ghost at line 7 ("through both protocol implementations"). Recommendation: give window_knee's `diverged` a budget parameter and land the production-budget cell there, deleting the file and updating nextest.toml's list.

## Dropped

- [20], [40] "28 + m stale" (blind-spots, api-economics): duplicate of tests-resource-link-window-29; their arithmetic corrections (51 B, not 52; the probe's 5,431 and the 512 MiB are accurate today) are folded in.
- [25], [41] "nextest comment stale": duplicate of tests-resource-link-window-1.
- [29], [54] "tradeoff_probe unscheduled": duplicate of tests-resource-link-window-18.
- [31], [43], [51a] "fixtures duplicated" and [12] "binding_capacity duplicated": merged into tests-resource-link-window-22.
- [48] "three capture parsers" and its QueryEmpty liveness point: merged into tests-resource-link-window-15 and -12.
- [34], [53] "HeadError pin placement": duplicate of tests-resource-link-window-8.
- [51b] "merge the allocator binaries": folded into tests-resource-link-window-5 as one of two resolutions.
- [47] composite (census runner, literals, floor): split into tests-resource-link-window-19, -21, -20.
- [32], [44] "double pair build" and [28] "probe rounds hops": merged into tests-resource-link-window-26.
- [26], [45] "len-only convergence": duplicate of tests-resource-link-window-27.
- [38], [55], [51c] "routed_link braces and repetition": merged into tests-resource-link-window-13.
- [57] "target_message_size builders": duplicate of tests-resource-link-window-14.
- [35], [49] "path_radix shadow" and [36] "absence assertion": merged into tests-resource-link-window-11 and -12.
- [33] "census literals": duplicate of tests-resource-link-window-21.
- [39] "caller comment" and [46] "caller comment plus attribute deletion": comment half merged into tests-resource-link-window-2; the attribute-deletion half is refuted: benches/gossip_fixed.rs:56-57 includes latency.rs with no module-level allow and uses only `DelayedWire::new`, so the per-item allows are load-bearing under `just clippy`'s `-D warnings`.
- [58] "import spacing": duplicate of tests-resource-link-window-17.
- [52] "growth race unwitnessed": duplicate of tests-resource-link-window-28.
- [19] "async_wire bodies identical": folded into tests-resource-link-window-3 as the fallback resolution if the file is kept.
- [21]'s sub-claim "no test anywhere pins that node_census counts": refuted; src/tree/mirror/streaming/remote/adapter/tests/malformed.rs:647-656 asserts the meter alive. The suite-local gap survives as tests-resource-link-window-20.
- [50] "sync_window_floor equals sync_memory_budget(0)": out of partition (src/peer.rs, src/tree/mirror/streaming/window.rs); recorded ruling R20 holds for the opt-in; raised as an open question.
- [59] "seed_rng doc(hidden)" and [60] "run_to_quiescence feature placement": out of partition (src/peer.rs, Cargo.toml features, src/testing.rs); feature requests, not defects; raised as open questions.
- Refutation-pass note "gossip_pipelining.rs:7 V1 ghost": out of partition; flagged in the open questions for whoever owns tests/gossip_pipelining.rs.
- [56]'s companion suggestion to pin exact counts everywhere: kept only as the owner-gated nit tests-resource-link-window-24, since 814f07ad1's band design is recorded and its rationale is inline.

<!-- source: final/tests-wire-format.md -->
# Partition tests-wire-format: Integration tests: wire snapshots, CBOR evolution, dispute wire, wire legibility, snapshot liveness, payload depth, future size, send bounds

## Partition summary

This partition is the wire-format and payload-contract instrument layer of the integration suite. `tests/gossip_snapshot.rs` stages twelve gossip scenarios over the recording link in `tests/common/gossip_snapshot.rs` and pins every wire byte with insta, staging hash-dependent tree shapes deterministically through `tests/common/shape.rs` (send a pool, search the created versions for the required path shape, redact the rest). `tests/wire_legibility.rs` states the CBOR-sequence promise as a proptest over random gossip, bootstrap, and retire sessions, checked by a walker that knows only `ciborium::Value`. `tests/snapshot_liveness.rs` reverses insta's path resolution to convict any committed `.snap` no live test generates. `tests/dispute_wire.rs` pins the affine per-message wire law at three record sizes with exact integer quotients over seeded corpora, guarded by a negative control. `tests/cbor_evolution.rs` and `tests/payload_depth.rs` exercise the payload contract peer to peer: name-keyed decoding, clean failure on undecodable payloads, the default depth boundary for two type shapes, the `Some(None)` faithfulness case, and the placement of the depth-mismatch abort. `tests/future_size.rs` and `tests/api_send_bounds.rs` are static guards on the public futures' size and `Send`-ness. All eight files are test code; I read 2454 lines across them, plus the harness modules they drive (`tests/common/{wire,gossip_snapshot,shape,mod}.rs`), the gate recipes, and the src sites the prose cites.

The suite is strong where it matters most. Every shape-staged fixture asserts the tree shape it landed before the byte comparison, so a pin cannot degrade into a weaker wire form unnoticed; two fixtures add in-test liveness floors on the frame sequence; the calibration suite proves its counter alive and bounds the truncation remainder before pinning exact figures; the snapshot harness renders from the public observer hook while holding every hook item to the transport capture as a totality oracle; and sessions run under the closed-world quiescence poller almost everywhere, so a protocol stall fails at its source rather than at nextest's timeout.

The dominant defects are of two kinds. First, one instrument is dead: `tests/future_size.rs` is compiled out under `debug_assertions`, and no gate leg, CI job, coverage run, or mutants campaign runs rumors tests at a profile where they are off, so its three budget tests have never executed under any committed check; its module doc meanwhile names types and an erasure site that no longer exist. Second, a cluster of prose has outlived the code it describes and was not re-read at the streaming swap, the CBOR respelling, or the V1 retirement: V1 phase names in `gossip_snapshot.rs`, a 25-byte preamble the same test's snapshot renders as 30 bytes, a test doc claiming the crate documents evolution rules the owner removed from `lib.rs` by hand, a design-cell doc contradicting `window.rs` about what derives from `DISPUTE_WIRE_BYTES`, and an `api_send_bounds.rs` module doc whose "every" the API outgrew in June. The remainder is harness duplication (`seeded` in ten suites, a cross-typed bootstrap hand-rolled five times, three observer recorders, two `FixtureTree`s), a handful of under-specified assertions, and idiom nits.


## Positives

- gossip_snapshot.rs's shape-staged fixtures (`colliding_pair` 107-122, `bulk_initiator_ships_opening_supplies` 275-302, `early_supplies_honor_redactions` 361-379, `shared_subtree_dispute_pins_a_nonempty_query` 560-596) assert the tree shape they landed before the byte comparison, and two of them add in-test liveness floors on the frame sequence (305-319, 382-388) or the nonempty listing (599-607): the cheapest passing artifact (a re-accepted degraded capture) is excluded by construction, not by convention. `early_supplies_honor_redactions` also checks post-state on both replicas (389-404); it is the pattern the rest of the roster should follow (tests-wire-format-5).
- tests/common/shape.rs stages hash-derived shapes deterministically (pool, search, redact) with no hand-picked versions and no mocked hash, and states plainly why its searches cannot flake; its panics say "enlarge the pool".
- tests/common/gossip_snapshot.rs renders from the public observation hook while holding every hook item to the transport capture with `assert_items_account_for` and asserting the control drain in both directions (459-471): the instrument enters through the public door and the byte-pin claim keeps a totality oracle.
- dispute_wire.rs's negative control (343-371) is a textbook liveness floor: it proves the counter alive on a session that disputes nothing and bounds the fixed overhead below one byte per message, which is what licenses exact integer pins; the three-cell affine design with a separately named residual makes framing drift visible at every record size.
- wire_legibility.rs states its claim as a family (a proptest over sessions), keeps the walker to `ciborium::Value` and standard tag semantics only, and uses `#[allow(clippy::type_complexity)]` on `corpora()` rather than coining a type name, as the owner's doctrine asks. No proptest seed belongs to it: it has never failed.
- snapshot_liveness.rs guards its own vacuity (230-237), refuses to skip anything under a snapshots directory (a pending `.snap.new` convicts), and documents its substring-match residual plainly (37-42).
- payload_depth.rs's `mismatched_limits_abort_both_sides_at_the_handshake` pins the abort's placement (no election, no data stream, on a converged pair) rather than only the error variant, and `a_sender_exits_typed_when_its_counterparty_aborts_on_decode` uses the closed-world poller so a hung sender is a failure, not a parked process; cbor_evolution.rs's failure-path tests assert "error, never a panic, nothing moved" on both sides.
- The closed-world `block_on` (`run_to_quiescence`) is used consistently in six of the seven session-driving files, so protocol stalls fail at their source instead of at nextest's 180-second timeout.

## Open questions for Finch

- future_size.rs (tests-wire-format-26): lift the cfg and pin a budget measured in both profiles, or add a release-profile leg for this one binary? Recommendation: lift the cfg. The cfg's original premise (debug-only boxing in the traverse trait dispatch) left the tree in June; a dev-profile budget keeps the gate's build set unchanged and keeps the guard inside the mutants observer, which runs dev/test. Either way the number needs a fresh measurement before it lands.
- cbor_evolution.rs (tests-wire-format-9): re-add the unknown-field and `#[serde(default)]` rules to lib.rs's compatibility paragraph, or drop the test doc's attribution? Recommendation: re-add them, because "may I add a field to my type?" is the first evolution question a user asks and the tests already pin the answer; but this reverses your own edit in 3d16765f9 ("Update lib.rs"), so if that removal was a ruling not to promise serde's field semantics, choose the other branch and the test doc follows.
- Liveness floors in gossip_snapshot.rs parse the renderer's text (161-229), as REVIEW.md item 14 specified. The parsers have been re-shaped three times for renderer changes, and AGENTS.md sanctions renderer-vocabulary re-accepts independent of the wire. Would you want a `rumors::testing` accessor exposing decoded frame signals so the floors read structure instead of text? Recommendation: leave as specified unless a renderer-vocabulary re-accept actually lands; it is a design proposal, not a defect.
- `Peer::seed_rng` is `#[doc(hidden)]` (src/peer.rs:212-213), yet it is the precondition for every wire pin in this partition, and a downstream crate writing its own insta pins over rumors sessions has the same need with no documented entry. Against documenting it: a caller-supplied RNG lets two processes mint the same `Network` each holding `Party::seed()`, which is exactly the two-universes hazard AGENTS.md forbids. Recommendation: keep it hidden and, if the need is real, offer a documented constructor that takes a `Network` value rather than an RNG, with the safety rule restated there.
- tempfile as a dev-dependency (tests-wire-format-21): it is already in the lockfile transitively. Recommendation: yes.
- A stream-index-2 snapshot fixture (tests-wire-format-7) costs a birthday pool on the order of 2^12 sends per run. Recommendation: add the cheap depth floor to `deep_trie_divergence` now; for the deeper labels, a codec-level unit pin of the stream-label encoding for indexes 2..16 is the lower-cost route if one does not already exist, with the module doc saying where the pin lives.

## Dropped

- [22] 'V2' qualifiers on the only protocol: refuted. `Protocol::V2 = 2` is public API, the literal `2` crosses the wire in the preamble (snapshot line 10), and the crate docs make per-version pinning first-class; "a V2 session" is a present-tense fact, not V1 residue.
- [12] Liveness floors re-parse rendered text: deliberate and specified in REVIEW.md item 14 (2026-08-20), implemented verbatim; the finding brings coupling cost but no new evidence, so it moves to open questions as a design proposal.
- [36] snapshot_liveness does not name the cfg'd-out generator as a residual: below the bar; the doc's closing clause ("pairs names with sources, not assertions with snapshots") already generalizes the residual, and no such generator has snapshots today.
- [45] Every byte-level suite depends on the doc(hidden) `Peer::seed_rng`: an owner decision about public API in src/peer.rs, not a defect in this partition; moved to open questions with the two-universes hazard as the case against documenting.
- [34] Depth-limit boundaries beyond the default unexercised: partly refuted (the raised boundary is pinned at src/message/tests.rs:191-200); the surviving zero and u64::MAX corners are tests-wire-format-22.
- [16] eprintln! diagnostics and rename-only wrapper: reframed by the refutation pass (the prints are the only success-path readout of a calibration suite; the wrapper is a pass-through); both folded into tests-wire-format-16.
- [0], [24], [38]: duplicates of tests-wire-format-26.
- [3], [25], [39]: duplicates of tests-wire-format-25.
- [2], [40]: duplicates of tests-wire-format-3.
- [1], [26], [41]: duplicates of tests-wire-format-6 (the 25-byte and 'u64 throughout' sub-claims of [26] and [41] live in tests-wire-format-3 and tests-wire-format-1; [26]'s 'now that' lives in tests-wire-format-15).
- [7]: folded into tests-wire-format-1.
- [4], [42]: duplicates of tests-wire-format-9; [42]'s serializer half and [5], [31] are tests-wire-format-14.
- [6], [33], [49]: duplicates of tests-wire-format-27.
- [8], [43], [47]: merged into tests-wire-format-11.
- [9], [44], [46], [10]: merged into tests-wire-format-2.
- [55]: split among tests-wire-format-8 (braces), tests-wire-format-2 (prefix collector), and tests-wire-format-17 (discarded corpora); [17]: duplicate of tests-wire-format-8; [18]: duplicate of tests-wire-format-17.
- [15], [32], [48]: merged into tests-wire-format-16.
- [20], [35], [53]: duplicates of tests-wire-format-10 (count corrected to 22 sites).
- [21], [52]: merged into tests-wire-format-24.
- [23], [54]: merged into tests-wire-format-15.
- [11]: tests-wire-format-23. [13]: tests-wire-format-20 (count corrected to three of nine). [14], [56]: merged into tests-wire-format-21. [19]: tests-wire-format-13. [27]: tests-wire-format-5. [28]: tests-wire-format-7 (mechanism corrected to a three-byte shared prefix on both sides). [29]: tests-wire-format-18. [30]: tests-wire-format-12. [37]: tests-wire-format-4. [50]: tests-wire-format-19. [51]: tests-wire-format-28.

<!-- source: final/tree-core.md -->
# Partition tree-core: The sparse Merkle radix trie: Tree, its tests and generators, the traversals (act, join, unknown)

## Partition summary

This partition is the in-memory content tree and everything that mutates it. `src/tree.rs` wraps a `Root` (a `Version` ceiling riding beside an optional height-32 typed node) in a `Tree<T>` facade whose `T` is a `PhantomData<fn() -> T>` witness over erased `Message` storage. It exposes the read faces (`hash`, `get`, `iter`, `range`, `range_owned`, `len`, `latest`, `earliest`), two commit paths (`act`, which ticks and keys a local batch and hands it to the private `react`; and `join`, the in-memory merge every gossip commit takes), and two test-only thread-local instruments (a root-hash read meter and a panic fuse). `src/tree/traverse/` holds three height-inductive traversals as polymorphic-recursive traits over the `S<H>`/`Z` ladder: `act` (batch apply with a per-key observer that decides the changed flag and the ceiling movement), `join` (lockstep merge with pointer-or-hash pruning), and `Unknown` (the deletion-honoring filter over memoized `[floor, ceiling]` spans that `join`'s one-sided arms call). `mod tree` is private (`src/lib.rs:322`); only `MERKLE_HASH_LEN` is re-exported and `Iter` leaks through `Snapshot`'s `IntoIterator`, so nearly every `pub` doc here is maintainer-facing and was judged at that altitude. `arb.rs` is the generator and fixture module every tree-adjacent suite in the crate imports; `tests.rs` is the tree's property suite.

The production code is structurally sound. No traversal recurses on input-controlled depth (all three recurse on the type-level height, bounded at 32 by construction); the one production `assert!` (version reuse, act.rs:169-176) and the one `unreachable!` (same-position leaves, join.rs:230-232) each carry a one-line argument that holds against `Hash::leaf`'s suffix-only preimage and `Node`'s pointer-or-hash equality; both commit sections defend panic atomicity by statement order with fuse-injected and destructor-source pins for each; and the changed flags are decided by the traversals, stated in both directions, with the one conservative case constructed rather than argued. No lens found a reachable panic, overflow, or torn state from any input the crate admits. `reference_hash` in `tests.rs` is an independent oracle, and `arb.rs` justifies each fixture by the shape it reaches and why version addressing cannot reach it otherwise.

The dominant issues are prose that outlived three events and a few fixed-sign performance deletions. Version addressing (961f63c6) left content-addressed language in `arb.rs` and content-era helper signatures and docs in `tests.rs`; the V1 mirror's retirement (368da2a50) left `Levels`, the zipper, `join_matches_mirror`'s transitive-coverage claim, and the "same filter" premise in three module docs; the sync-then-erasure commits (262568f9e, b524e406) left `T: Send + Sync` bounds and a hand-written `PartialEq` with nothing to avoid. The SHA3 swap (4f18c347) broke a re-derivation discipline d800957e8 had followed for the geometry fixture's "attempt 1581". On the code side, `act` sorts its action list at every one of the 32 heights and re-sorts each touched fan on reassembly where one stable sort at entry would do; `join`'s divergent arm clones the fan three times over and hand-rolls a merge that `itertools::merge_join_by` (already used in `materialized/work/answer.rs`) spells in a `match`; and `Tree::hash` clones the whole root pair to borrow a field. The one verification finding of substance is that `join_associative`'s doc claims redaction-associativity coverage that no suite provides, and that no property in the tree states join's survivor formula directly, so the three algebraic laws would pass with the leaf verdict inverted.

Lines read: 4067 across the nine partition files, plus the out-of-partition lines each finding cites. Test code: `src/tree/tests.rs` (1897), `src/tree/arb.rs` (699), `src/tree/traverse/join/tests.rs` (52), `src/tree/traverse/unknown/tests.rs` (156), and the `#[cfg(test)]` `meter` and `panic_injection` modules inside `src/tree.rs` (616-712). Production: the rest of `src/tree.rs`, `traverse.rs` (19), `act.rs` (194), `join.rs` (238), `unknown.rs` (94). No cargo, just, or test command was run; every claim is by reading, grep, or read-only git.


## Positives

- The commit sections in `Tree::react` and `Tree::join` (tree.rs:507-541, 576-613) defend panic atomicity by construction (the walk runs on an O(1) structural clone, the ceiling folds into a local, the commit is replace, assign, then drop) and state the hazard concretely: an emptied root under a live ceiling, "byte-for-byte the shape of 'everything was redacted'". Each is pinned from three directions: caller-stream unwind, fuse-injected mid-walk unwind after copy-on-write work has begun (with the arming depth derived from the fire-point arithmetic, tests.rs:1655-1659), and the one caller-reachable source, a panicking `T` destructor on a last-handle drop, with the caught panic's message checked so the pin provably exercises that source (tests.rs:1683-1794). Every test the comments name exists.
- The changed flags are decided by the traversals, not by hashing, so no root hash is read inside a critical section; both directions are stated with their differing promises (tree.rs:399-417, 552-565), the one conservative case is constructed rather than argued (`act_changed_flag_is_conservative_only_in_a_poisoned_store`), and the biconditionals are pinned at the root fan and at full depth (`join_changed_flag_tracks_the_root_hash{,_at_depth}`, `deep_divergent_join_changed_flag_is_exact`), with `ceiling_only_join_reports_unchanged` pinning the deliberate exclusion of ceiling movement from join's flag.
- No traversal in the partition recurses on input-controlled depth: `act`, `join`, and `Unknown` are polymorphic recursions over the Peano height that bottom out at `Z` by type, and `Path::pop`'s index `32 - S::<H>::HEIGHT` is in `0..=31` by construction. The one production `assert!` (act.rs:169-176) is justified as a trust-boundary detector with both legs pinned by `#[should_panic]` tests and the identical-reinsert idempotence pinned separately; the one `unreachable!` (join.rs:230-232) carries a valid argument from `Hash::leaf`'s suffix-only preimage.
- `traverse::act`'s `# Panics` section (act.rs:30-38) is close to the one-line proof the doctrine asks of an assert (fresh ticks dominate the ceiling; party linearity; no wire-derived leaf on this path), and the monomorphization boundary is a stated decision at the code that enforces it (act.rs:24-28: a concrete `Vec` and `&mut dyn FnMut` so the per-height tower compiles once).
- `reference_hash` (tests.rs:107-181) is an independent ground truth: literal tag bytes, its own compression rule, its own preimage assembly, re-deriving the canonical shape from the sorted path set without calling back into the implementation. `tree_shape_is_canonical_in_the_leaf_set` routes one leaf set four ways (single batch, shuffled split with a redacted detour, disjoint join, bulk `from_sorted_leaves`) and pins the hash's version-set purity and the leaf view's payload mapping as separate facets.
- `arb.rs` justifies every generator and fixture by the shape it reaches and why version addressing cannot reach it otherwise; `arb_deep_divergent_pair`, `leaf_parent_dispute_pair`, and `leaf_parent_redaction_pair` construct the hash-prefix-collision analogue deliberately so the divergent descent and the `S<Z>` arms are reached at every depth; `early_first_child_dispute_pair` cross-checks its path simulation against the built trees (397-409) so the search cannot disagree with the builder unnoticed; `nth_party`'s disjointness invariant is stated (10-19) and checked mechanically.
- `unknown/tests.rs` is a model meter test: a retained known-worse shape as the cost oracle, a verdict-equality assertion before the cost comparison, a liveness floor on both counters, and a `MEASURED` line for the record.
- join.rs's module doc (1-35) is a model maintainer doc: the four-case analysis, why equal hashes mean equal version sets, and a candid complexity note about enumerating a divergent branch's full fan; unknown.rs's fused-classification comment (37-62) explains each `Dominance` verdict as a prune decision and why the floor-first exit is the common case.

## Open questions for Finch

- `Tree::react` documents a general versioned-apply contract (concurrent and tied versions, order-of-specification tie-breaking) that only tests exercise; its sole production caller `act` supplies one party's ascending chain. Should `react` stay a generic versioned-apply (then tree-core-30's storage-site fix and a per-key ceiling across Forgets are the right repairs), or should it collapse into `act`'s commit section with the fixtures reaching `traverse::act` directly as `arb.rs` already does, and the multi-action semantics documented once at the leaf level? Recommendation: keep `react` as `act`'s commit section, document it as such, and let tree-core-14 remove the middle spelling.
- The larger `act` redesign behind tree-core-27 (recurse on `&mut [(Path, Version, Action)]` slices with a depth index as `from_sorted_leaves` does, and build a fresh subtree under an absent child with `Node::from_sorted_leaves` in one shot instead of 30 levels of `branch`/`beneath`) changes the fuse fire-point structure that `act_mid_walk_unwind_leaves_tree_byte_identical` derives its arming depth from. Recommendation: land the single-sort tier first (fixed sign, measured with the existing benches), and decide the redesign only on those measurements.
- tree-core-5 reopens 8dc0596ed's opacity decision on `Snapshot::iter`. Recommendation: re-export `Iter` and return it concretely, keeping `typed::Iter` private inside it; that meets the "hide engine internals" goal and matches std's `IntoIterator for &Vec` convention.
- tree-core-6: keep `latest`/`earliest` as named and state the asymmetry at the tree level, or rename the ceiling accessor (`frontier`) so the pair can be symmetric? Recommendation: state it at the tree level now; the rename is worth doing before the first release if at all.
- After the SHA3 swap, does `early_first_child_dispute_pair`'s search still terminate at attempt 1581, or at another attempt within 2048? One test run answers it; tree-core-24 asks that the prose stop carrying the number either way.
- The `.agent-notes/2026-08-21-unknown-pruning-survivor/` handoff (a mutant inverting the streaming filter's leaf verdict survives the suite) is out of this partition, but its hypothesis that fixtures never build a mixed-knowledge parent at height 1 also describes `traverse::unknown`'s `Z` arm here, which only `leaf_parent_redaction_pair` reaches. A shared generator forcing `S<Z>` parents with interleaved known, unknown, and deleted leaves would serve both partitions; should it live in `src/tree/arb.rs`?
- The associativity-under-redactions law (tree-core-34) holds under the in-model invariant that a ceiling containing a leaf's tick contains the whole leaf version (a consequence of ceilings advancing only by own ticks and joins of whole ceilings). Is that invariant stated anywhere a maintainer would find it (crate docs or `reconciliation`)? If not, the new property's testdoc is a reasonable home for it.

## Dropped

- Candidate 14 (prose) and 52 (perfapi), "attempt 1581" in `ATTEMPTS`'s doc: duplicates of tree-core-24.
- Candidate 40 (correctness) and 46 (perfapi), `Tree::hash` clones the whole `Root`: duplicates of tree-core-7.
- Candidate 41 (correctness), bounds hygiene: its derived `Debug`/`Eq` half is tree-core-2; its `Send + Sync` and `Root: PartialEq` halves are tree-core-4.
- Candidate 53 (perfapi), `Root`'s hand-written `PartialEq`: duplicate of tree-core-4.
- Candidate 54 (perfapi), `react`'s `M` generic: duplicate of tree-core-14.
- Candidate 17 (prose), `traverse.rs` ghost `Levels` and misdescribed trio, and 18 (prose), the zipper: merged into tree-core-26 as one pattern (ghosts of the V1 mirror).
- Candidate 30 (prose), comments that narrate the next line: duplicate of tree-core-28.
- Candidate 34 (correctness), `join_associative` testdoc: duplicate of tree-core-34.
- Candidate 37 (correctness), observer contract and "once per changed key": merged into tree-core-11, which states its rule; the refutation pass's new item on the `# Panics` reach is folded in there as well.
- Candidate 42 (correctness) and 51 (perfapi), the "2-3x" figure: duplicates of tree-core-10.
- Candidate 43 (correctness), `TwoPass` framing, and 24 (prose), "now rejects": merged into tree-core-35 as temporal framing in test prose.
- Candidate 47 (perfapi), join clones the fan three times: merged into tree-core-31 with candidate 3; one rewrite satisfies both.
- Candidate 48 (perfapi), act's reassembly pays `Fan::from_iter`'s sort fallback: merged into tree-core-27 as the second redundant sort in the same walk.
- Candidate 31 (prose), `naive_max_version_bytes` is a renamed call: merged into tree-core-19.
- Candidate 32 (prose), missing module docs and inline `mod test`: merged into tree-core-25.
- Candidate 21 (prose), `idx`'s doc: merged into tree-core-18 with candidate 20 as the same pattern (helper preconditions that are false).
- The refutation pass's new item 2 (`react`'s "causally latest wins" fails for `[Forget(v2), Insert(v1)]`): folded into tree-core-30 as the second construction rather than a separate finding.
- The refutation pass's new item 3 (`streaming/stats.rs:45` and `streaming/tests/fixtures.rs:322` say "content-addressed"; `streaming/tests.rs:154-156` repeats the shared-filter claim): out of this partition; noted as related sites under tree-core-23 and tree-core-1 for the streaming partition to pick up.
- Nothing was dropped as refuted: the refutation pass confirmed every candidate and reframed one (35, now tree-core-33 at low severity).

<!-- source: final/tree-typed.md -->
# Partition tree-typed: The typed tree layer: hashing, heights, nodes, paths, prefixes, the untyped view, fans, iteration

## Partition summary

This partition is the content tree's storage core and its height-typed veneer. `untyped::Node` is one `Arc<NodeInner>` shape for leaves and branches alike, carrying a deepest-first compressed prefix and three lazy memos: the single-preimage SHA3-256 hash, a `[floor, ceiling]` `Span`, and the maximum version-encoding width. `Fan` is a sorted `SmallVec` of `(radix, Node)` pairs with two inline slots. iter.rs holds a borrowing frontier walk (`Iter`, `Range`) and an owned constant-state spine walk (`RangeOwned`). Above that, height.rs gives Peano heights `Z`/`S<H>` with numbered aliases for the erased dispatch table, and `Node<H>`, `Children<H>`, `Path<H>`, `Prefix<H>`/`ErasedPrefix` re-tag the untyped values so traversals monomorphize per level. hash.rs fixes the 24-byte Merkle width, the 32-byte path width, and the tagged, length-delimited preimage layout. `mod tree` is private and `typed` is `pub(crate)`, so the only public surface here is `MERKLE_HASH_LEN`.

The code is in good shape. The two non-obvious tricks (the `PhantomData<fn() -> H>` auto-trait shortcut, and hash agreement resting on canonical shape rather than on the hash construction) are each explained and tested adversarially; the census funnel, the bulk `from_sorted_leaves` constructor, and the fan's inline-size pin state what they serve outside themselves; every panic site traces to programmer error, with the one trust boundary (`from_sorted_leaves`'s preconditions) enforced upstream by the decoder; recursion depth is structural everywhere. Verification is strong where it matters: literal-byte preimage pins, an independent `reference_hash` over the decomposition API, a virtual-level canonicity proptest against the bulk constructor, and a differential fan suite that checks handle identity.

The dominant issues are retirement residue in prose rather than design debt. The V1 protocol retirement and the leaf-preimage change left four sites describing a node codec that no longer exists, a `LEAF_TAG` doc that lists a field the preimage no longer commits, and a crate-private copy of the 24-byte width argument that has drifted from the reconciliation doc and now prices an actor the same docstring says contributes nothing. Below those: two mechanisms for one goal (`Height`'s supertraits beside hand-rolled impls that claim to avoid them), the number 32 hand-maintained three ways with no `Root::HEIGHT` tie, a handful of one-caller wrappers and bypassed impls that would read better dissolved, a performance contract in `Fan::from_iter`'s doc that the commit path violates, and a set of nits (expect messages, long qualified paths, a sentinel-encoded cursor, testdocs carrying a stale record width).

Lines read: 4164 across the fourteen partition files (2736 production, 1428 test: hash/tests.rs, height/tests.rs, path/tests.rs, untyped/fan/tests.rs, untyped/tests.rs), plus the external anchors each finding cites (act.rs, encode.rs, frame.rs, decode.rs, backend.rs, local.rs, tree.rs, lib.rs, reconciliation.rs, conformance.rs, testing.rs, both unknown.rs, join.rs, erased.rs, the bench header, the mutants config, the two agent-notes rulings, and the pinned smallvec and tinyvec sources). No cargo or just command was run; every "verified" below means grep, git, or a read of the cited lines.


## Positives

- The `PhantomData<fn() -> H>` auto-trait shortcut is explained where the chain originates (height.rs:6-12), extended at height.rs:16-21 to why `S<T>`'s impls are hand-rolled (a derive would reintroduce the `T: Trait` bound), and pinned for size and alignment in height/tests.rs. This is how a non-obvious design decision should be documented; finding 13 is only about the second copy.
- `Hash::branch`'s canonicity section (hash.rs:143-156) locates hash agreement in canonical shape rather than in the hash construction, and `every_virtual_level_hashes_canonically` (untyped/tests.rs:470-485) pins exactly that claim at every virtual level, mid-spine included, against the from-scratch bulk constructor.
- hash/tests.rs `prefix_len_separates_boundary_shifts` (71-115) constructs the collision pair the length tag exists to prevent and asserts its premise (the untagged preimages coincide) before asserting the conclusion: a model adversarial pin. `saturated_fan_count_uses_the_high_byte` pins the one reason the count is a `u16`.
- The fan differential suite (fan/tests.rs:46-97, 107-132) checks handle identity via `ptr_eq` in both iteration directions, all 256 point lookups, and every successor probe against a `BTreeMap` oracle after every step; `size_hint` exactness is checked under consumption from both ends. `fan_is_forty_bytes` and `node_inner_stays_within_budget` make per-node cost a reviewed number with the growth rule stated in the testdoc.
- `Node::from_inner` (untyped.rs:190-199) is the single construction funnel that makes the test-only `census` an exact residency count, and its doc names what it serves outside itself: checking the session window's in-flight-reference bound against reality.
- `from_sorted_leaves`'s strict-ascent precondition is enforced at the trust boundary (decode.rs:508-527 rejects non-ascending and out-of-scope leaves before assembly), so its debug asserts are correctly scoped as programmer-error guards; the `Option` slots let the recursion move nodes out of a shared slice without cloning, and `branch_at` strictly increasing toward 32 bounds the recursion structurally.
- `RangeOwned`'s constant-state spine walk (one `Level` per materialized branch, siblings never enumerated, `successor` by binary search) is documented at the type with the memory argument the session window relies on; `Walk::step`'s two-ended re-push order (iter.rs:129-133) is stated precisely enough to check by reading.
- The `NodeInner`/`Children` field docs state the memo-invalidation invariants exactly (hash resets on any prefix or children change; bounds and `version_bytes` reset on children change only), and every mutation site (`into_children`, `beneath`, `from_sorted_leaves`) carries the matching reset with a comment saying why.
- `# Panics` sections with one-line reasons at `Hash::leaf`/`branch`, `ErasedPrefix::push`/`pop`, and `Leaf::value`; `ErasedPrefix::assume`'s doc traces why a cross-height re-tag is programmer error and never peer input.
- fan.rs's module doc argues the data-structure choice from the population shape and says where structural sharing actually lives and why `Fan` is deliberately not a persistent map: a design defended once, in writing, at the type.
- The `PartialEq` comment at untyped.rs:686-693 explains why hash equality is content equality and names the one scenario in which it would not be, tying it to the linearity invariant.

## Open questions for Finch

- `Hash` derives `Default` (the all-zero digest); its only users are ten test sites under mirror/streaming/remote, and `root_hash`'s comment warns that the empty tree must not hash as that value. Recommendation: drop the derive and spell `Hash([0; MERKLE_HASH_LEN])` in the tests, so a production `Default::default()` on a digest cannot compile.
- `ErasedPrefix::assume` checks the length-vs-height witness only in debug builds (prefix.rs:45-49). In release, a cross-height re-tag to `Prefix<Z>` yields fewer than 32 bytes and `From<Prefix> for Path` (`into_inner`, prefix.rs:113-117) zero-fills the tail, a misplaced leaf rather than a crash. Programmer-error only, and all tests run in debug. Recommendation: promote the O(1) witness to a release `assert!`; the check is cheap and the failure mode is silent misplacement.
- `Root`'s two-row `S<` ruler (height.rs:127-133): pedagogy worth keeping, or `pub type Root = H32;`? Recommendation: the alias plus `const _: () = assert!(Root::HEIGHT == PATH_LEN);`; if the ruler stays, the assert alone closes the miscount (finding 12).
- Which copy of the `PhantomData<fn() -> H>` argument to keep (finding 13): the owner last re-justified the node.rs copy at 8f87ddd01. Recommendation: move that wording onto `S` in height.rs, where the chain originates, and point the three types at it.
- The supply path's bare-leaf rebuild (`Leaf::into_node`, iter.rs:345-362) costs one `Arc<NodeInner>` allocation, a `Version` clone, and a `Message` clone per supplied leaf, to uphold `Node<Z>`'s bare-leaf invariant that only `from_sorted_leaves` asserts (untyped.rs:288-291) and the supply encoder never needs (encode.rs:204-206 reads `span()` and `message()` only). The rationale is stated in code and holds, so it is not filed as a finding. Question: is the invariant worth the per-leaf allocation on the wire path, or should `Backend::leaves`'s item be a leaf handle distinct from `Node<Z>` (touches the `Backend` trait)? Recommendation: measure with `benches/gossip_fixed.rs` on a supply-heavy fixture before deciding.
- `Prefix<H: Height = Z>` defaults to leaf height while `Path<H: Height = Root>` defaults to root height (prefix.rs:18, path.rs:15). Both are internal; the asymmetry surprises a maintainer. Recommendation: document why leaf-height prefixes are the common case (they are the leaf keys), or drop the `Prefix` default.
- Why does the gate's `-D warnings` not flag the caller-less typed `compressed_prefix_len` (node.rs:236-242)? I could not compile. If it is a dead-code-lint gap for `pub` items in a private subtree, finding 1's `unreachable_pub` plus `pub(crate)` convention is the fix; if something reaches it, finding 14's dead-wrapper claim should be re-checked by compiling.
- Handoffs out of partition: reconciliation.rs:164-165 ("a leaf's digest commits its address and its version") carries the same drift as finding 5 in the public doc of record; tree.rs:36 ("the node serializer relies on") is the same ghost as finding 19; tree.rs:277 clones the `Root` pair per hash read (finding 18).

## Dropped

- [42] `MAX_BRANCHING` never binds: deliberate and documented. The constant is the alphabet guard for `btree_set(any::<u8>(), 1..=max_n)` (a budget above 256 would otherwise ask for more distinct bytes than exist), and its doc hedges "subject to the leaf budget". The residual gap (no 256-child fan through `Node::hash` against `reference_hash`) is below the bar: the saturated count is pinned at `Hash::branch` and the fan's spilled storage by the differential suite.
- [44] supply walk rebuilds a bare leaf per compressed leaf: the rationale is stated in code (iter.rs:345-351) and holds; the encode.rs:197-200 comment is accurate about the encoder's own reads. Converted to the fifth open question.
- [52] `Path::pop` and `Prefix::pop` return their tuples in opposite orders: refuted. `Path::pop` removes the leading byte and returns `(byte, rest)`, the shape of `split_first`; the two prefix `pop`s remove the trailing byte and return `(rest, byte)`, the shape of `split_last`. The byte sits in the tuple where it sits in the path.
- [32] the bench magnitude in `Hash::branch`'s comment: reopens the sealed R30 ruling (.agent-notes/2026-07-23-review-link-transport: "bench named in the claim", the factor re-denominated for SHA3 at 4f18c347) with no new evidence. Its "17-byte" half is kept as finding 9.
- [14] the `step(back: bool)` half: a documented private parameter; taste, not a finding. The sentinel half is finding 34.
- [28] untyped.rs:237 `expect("non-empty prefix")`: sits directly under its guard at 230 and names the establishing fact; not a defect. The other two messages are finding 22.
- [33] the "names the tree" half: `crate::reconciliation` is public and describes the tree, so the concept is already established for the user. The fragment half is finding 2.
- [9] the "undercounts its walks" clause: the module doc counts shells of the shared frontier engine, and `RangeOwned` has its own spine engine; kept only as a completeness item within finding 32.
- Refutation's new item on "seam" (node.rs:151 "The streaming mirror's erasure seam"): the streaming layer's established name for `erased`, used across the crate; whether it is defined where a reader first meets it belongs to the streaming partitions.
- Duplicates merged: [20], [34], [47] and the refutation's "node serializer" item into finding 19; [36] into 5; [16] into 3; [51] and [4] into 12; [48] into 16; [38] and [50] into 35; [40] and [54] into 32 and 22; [12] into 14; [30] into 9; the refutation's release-mode duplicate-path item into 27.

<!-- source: sweeps-final/api-audit.md -->
# Sweep api-audit: Public API surface audit

## Method and coverage

The sweep enumerated the public surface from the rendered rustdoc and read
every item's definition. This finalization pass disputed each of its
nineteen findings against the tree at 9e5784fb (working tree clean):

- Read every cited site with line numbers (`awk`/`sed -n` ranges over
  `src/error.rs`, `src/snapshot.rs`, `src/bookmark.rs`, `src/protocol.rs`,
  `src/peer.rs` whole, `src/rumors.rs` whole, `src/peer/gossip.rs`,
  `src/peer/bootstrap.rs`, `src/link.rs`, `src/link/routed.rs`,
  `src/link/routed/endpoint.rs`, `src/link/routed/stream.rs`, `src/tree.rs`,
  `src/message.rs`, `src/observe.rs`, `src/rumors/{unordered,causal,changes}.rs`,
  `src/tree/mirror.rs`, `src/tree/mirror/handshake.rs`, the streaming
  `remote/error.rs`, `codec/error.rs`, `codec/signal.rs`, `codec/frame.rs`,
  `adapter/error.rs`, `materialized/error.rs`, `proxy/error.rs`,
  `streams.rs`, `stats.rs`, `window.rs`, `Cargo.toml`, the `justfile`, and
  `tests/api_send_bounds.rs`).
- Extracted the `impl` headers from the rendered pages under
  `target/doc/rumors/` (index mtime 2026-09-01 19:17, after HEAD's commit
  time 18:37; the tree is clean, so the pages render this commit) with a
  short Python scan, and checked which pages exist under `error/`.
- Grepped for use sites (`fn rumors`, `into_rumors`, `select`,
  `missing_docs`, `warm_caches`/`seed_rng`, `non_exhaustive`, `Debug for`,
  `tests/` inside doc comments, doc-comment `.send(..);` statements,
  `.flip()`, the `pub` constructors' callers).
- Checked history with `git log -S` for `BookmarkError`, `repr(u16)`,
  `Two deliberate boundaries`, `peer.rumors()`, `# Choosing a budget`,
  the `Iter` re-export, and `warm_caches`; read the commit message of
  1e458d69 (the `non_exhaustive` policy) and the notes under
  `.agent-notes/2026-09-01-v1-retirement/`,
  `.agent-notes/2026-08-20-cbor-wire-review/`,
  `.agent-notes/2026-07-22-sync-budget/`.

What this pass could not see: no cargo, rustc, or `cargo doc` invocation
was permitted, so the derive-bound claims rest on the rendered pages plus
the standard semantics of `#[derive]` (a bound on every type parameter),
and the availability of the `unnameable_types` lint on the pinned
toolchain was not checked. No test invocation was needed: none of the
surviving findings is a correctness claim.


## Positives

- The outcome types that carry a live `Peer` (`Retire`, `Joined`,
  `Unbookmarked`) are `#[must_use]` with messages that say exactly what
  dropping loses (gossip.rs:92, 148; bootstrap.rs:364): the compiler
  enforces the identity-custody model at the API edge.
- `Bootstrap` versus `BookmarkedBootstrap` is a typestate that gives each
  `join` only its own outcomes (bootstrap.rs:262-273), so a persist failure
  after a successful session cannot be mistaken for success.
- `Peer`, `Rumors`, and `Bootstrap` implement `Debug` by hand, print a
  summary, and say why (peer.rs:182-193, rumors.rs:79-90, bootstrap.rs:96-
  105); `Bootstrap`'s manual `Clone` carries a two-line comment naming the
  derive-bound trap (bootstrap.rs:81-84). The standard api-audit-1 asks the
  rest of the crate to meet is one the crate already wrote down.
- The `error` module doc is a table mapping every top-level variant to the
  replica's state and the caller's next action (error.rs:10-28).
- `Link`'s "What a session promises" (link.rs:279-321) states the
  `Ok`/`Err`/cancellation contract, names the three qualified exceptions to
  "unchanged", and puts the timeout with the caller; the model-of-record
  statement at link.rs:115-139 is careful to say the validation machinery
  is not a security boundary.
- `Gossiped`, `SessionStats`, `SessionInfo`, `StreamInfo`, and `StreamId`
  are `#[non_exhaustive]` data carriers with public fields, and commit
  1e458d69 records a coherent rule for which enums are open.
- `PayloadDepthLimit` is a proper newtype with `const fn new`/`get`,
  `Display`, `Default`, and a named default constant; `Network` is opaque,
  `Copy`, totally ordered, and serializes as its 16 raw bytes.
- The conformance suite's `yield_once` (conformance/link.rs:680-696) avoids
  `tokio::task::yield_now`, so the suite runs under any executor;
  `link::memory` is deterministic and closed-world.
- The tutorial is a sequence of complete programs with their exact output,
  and the crate-level example is shown whole; both compile as doctests.
- `Rumors::batch` documents and demonstrates with two `compile_fail`
  doctests that the scope handle cannot escape the closure
  (rumors.rs:320-336).
- `Snapshot::range` takes anything `Into<causally::Query>` and states that
  a small causal delta costs work proportional to the delta
  (snapshot.rs:114-161).
- The V1 retirement note records its rulings, the verification lattice,
  and an honest "what is genuinely lost" section; the retirement itself
  left few ghosts (api-audit-5, api-audit-17 are the residue).

## Open questions for Finch

1. Decision 2 of the V1 note keeps `Protocol` public as wire vocabulary.
   Does that ruling also keep `Error::VersionMismatch.local_protocol` as
   the enum (so api-audit-5 is purely a prose rewrite), or should the
   variant carry the local wire version as a number like `remote_version`?
2. Should `BookmarkError` be folded into `Bookmark` (one trait for
   implementors, `B: Bookmark` everywhere), or is there a reason for the
   split that should be written at the trait? No rationale was found in
   git or the notes.
3. Is the full depth of `rumors::error` (roughly thirty codec, adapter,
   stream, and proxy types) meant to be public API, or should the public
   surface stop at a concrete `MirrorError` with the generic sum, its
   `Infallible` slots, constructors, and schedule helpers kept
   crate-private? api-audit-13 and api-audit-14 hinge on this.
4. Does the operator sizing guide belong on `Peer::sync_memory_budget`
   (where the 2026-07-23 amendment placed it), or in an explanation module
   beside `reconciliation` now that the crate has that shape?
5. Should `Rumors` expose direct `len`/`is_empty`/`latest` readers and
   `Snapshot` a `contains(&Version)`, or is "read through a snapshot" the
   intended contract, to be stated in the `Rumors` type doc? The tutorial
   reads `snapshot().len()` five times, which suggests the direct readers
   would be used; the cost of the current path is one `Arc` bump.

## Dropped

- Sweep [18] (`Rumors` has no `len`/`is_empty`/`latest`, `Snapshot` no
  `contains`): a design question with no named cost beyond an `Arc` bump;
  moved to open question 5.
- Sweep [7]'s citations of link.rs:49-50 and link.rs:167-168: those docs
  already state provenance without a path, the form the resolution asks
  for.
- Sweep [7]'s citations of adapter/error.rs:84 and 99 (`max_version_bytes`,
  `set_len`): those are the greeting's wire field names (greeting.rs:16,
  19), observable through the `observe` hook, not private items.
- Sweep [9]'s `Config` example: a public-field struct with `Default` is
  the std idiom; `#[non_exhaustive]` would remove literal construction and
  `..Config::default()` already survives new fields.
- Sweep [9]'s claim that `EncodeError`'s docs describe admission as a
  growing set: message.rs:108-139 describes one mechanism (decode and
  compare) with three outcomes; the finding was reframed to the unrecorded
  policy (api-audit-10).
- Sweep [10]'s newtype-versus-`usize` asymmetry: `PayloadDepthLimit`
  crosses the wire and is compared in the handshake; the byte budgets are
  local-only, so the shapes differ for a reason the code shows.
- Sweep [17]'s mechanism (the doctest leg lacking `-D warnings`): rustdoc
  injects `#![allow(unused)]` into doctests, which command-line lint levels
  do not override; reframed in api-audit-18.

<!-- source: sweeps-final/async-hazards.md -->
# Sweep async-hazards: Async and concurrency hazards

## Method and coverage

This is the verification pass over the async-hazards sweep at commit
9e5784fb4dce977cfbdfd1619886d1482b5ce764 (working tree clean, checked with
`git rev-parse HEAD` and `git status --short`). For each of the sweep's six
findings I opened every cited site with line numbers (`awk`/`sed -n`) and
compared the sweep's quotations against the file; every quotation matched.
Beyond the cited lines I read:

- `src/peer/gossip.rs` 525-890 and 1367-1404 (`bookmark_update`,
  `bookmark_donate`, `gossip_inner` end to end, `PartyGuard`), `Retire` at
  gossip.rs:87-134, `Peer::retire` docs at peer.rs:281-290.
- `src/tree.rs` 490-620, `src/tree/traverse/act.rs` 140-170,
  `src/tree/traverse/join.rs` 60-72, `src/tree/tests.rs` 1685-1700 and the
  four unwind-atomicity test names at 1505, 1554, 1697, 1746.
- The pinned dependency source: Cargo.lock pins `tokio 1.52.3` (the sweep
  checked 1.53.1); I read `send_if_modified`, `send_modify`, and both
  `borrow` methods in `~/.cargo/registry/src/*/tokio-1.52.3/src/sync/watch.rs`
  (lines 1104-1112, 1171-1211, 629-638, 1253-1260).
- `src/tree/mirror/streaming/materialized.rs` 468-595,
  `materialized/common.rs` 1-60, `backend/local.rs` 125-150,
  `backend.rs` 390-414, `materialized/work.rs` 80-153, `tasks.rs` 1-59,
  `materialized/work/levels.rs` 105-125, 200-220, 380-460,
  `remote/proxy/work.rs` 120-140.
- `src/rumors/causal.rs` 15-110 and 165-200; `src/lib.rs` 225-270 and
  307-349 (module and re-export roster: `mod tree;` is private, only
  `MERKLE_HASH_LEN` and `SessionStats` are re-exported from it).

Mechanical checks: `grep -rn 'select!'` over `src/` (six `tokio::select!`
sites plus one doc mention, matching the sweep's roster);
`grep -rn 'poll_fn\|impl.*Stream for\|impl.*Drop for\|impl.*Future for'`;
`grep -rn 'send_if_modified\|send_modify'` (four production sites:
batch.rs:132, gossip.rs:538, 682, 816, plus `send_modify` at gossip.rs:1388);
`grep -rn 'spawn'`, `warm_caches`, `before any wire traffic`,
`recoverable only through` over `src/`, `tests/`, `.agent-notes/`;
`git log -S` for both disputed phrases; `git show 3f33afabf:src/peer/gossip.rs`
to check the await order at the commit that introduced the bookmark gate;
`git merge-base --is-ancestor f6cf25791 3f33afabf` (the preamble commit is an
ancestor of the bookmark-gate commit). I read the two review packets'
mentions of "wire traffic" and "carve-out"
(`.agent-notes/2026-07-23-review-link-transport/review-link-transport-branch.md`
lines 170-190, 600-630, 740-765): both concern other matters (a ghost
reference, and R54's epilogue-confirmation carve-out), so no prior ruling
covers these findings.

I ran none of the two permitted `cargo nextest` invocations: no existing test
settles any of the claims, and constructing the deadlock in finding 3 needs a
new test file, which I may not write. Every claim below is therefore
"verified" only in the sense of source reading and mechanical grep/history
checks; nothing was executed. After this report was finalized, the second
witness pass ran finding 3's construction as a new integration binary
(tests/zz_witness_drop_reentry.rs, deleted after the run): `redact` did not
return within 10 s, and the entry below carries the outcome and its raised
severity. I did not re-verify the sweep's positives that
lie outside the cited files (the observers' owned-wait design, the `Extant`
token protocol, the wire-fed buffer bounds); those are carried as
sweep-reported.


## Positives

- Every `tokio::select!` in production source is cancellation-clean by
  construction (verified at tasks.rs:24-36, driver.rs:73-74 with `biased;`
  toward the error route, and the roster of six sites matches the sweep's).
  `tasks::complete` races pinned `&mut` futures and awaits the loser
  afterwards, so no task's progress is lost.
- The commit critical sections are panic-atomic by statement order alone
  (tree.rs:531-539, 602-611) and that property is pinned by four committed
  tests, two of which exercise the real mid-walk destructor source
  (`act_destructor_unwind_leaves_tree_byte_identical`,
  `join_destructor_unwind_leaves_tree_byte_identical`).
- `PartyGuard` (gossip.rs:1379-1404) makes the speculative fork's recovery a
  drop guard, so every error and unwind path re-joins it without a per-path
  case, and the `bookmark_donate`-then-`take` ordering (gossip.rs:769-781)
  states exactly why the guard is defused only after the slice persists.
- Runtime independence is enforced in the manifest: `tokio` carries only
  `io-util`, `macros`, `sync` (Cargo.toml:137), and `rt` enters only through
  the `test-internals` feature (Cargo.toml:112).
- The bookmark-then-`watch` lock order is stated and followed at both
  nesting sites (gossip.rs:534-553, 677-716), and the `watch` write lock is
  never held across an await.
- Sweep-reported, not re-verified here: the observers' owned-wait
  `Channel::Waiting` design, the `Extant` token protocol behind
  `try_into_peer`, `OnceLock` memos on shared nodes, the three `# Cancel
  safety` sections on the frame futures, and the negotiated bounds on
  wire-fed buffers.

## Open questions for Finch

1. Finding 3 (destructors under the write lock): the destructor constraint
   must be documented in any case, since `traverse::act` and `traverse::join`
   drop `T` mid-walk. Do you also want (a) the cheap deferral of the pre-image
   drop past `send_if_modified` (internal signature change to `Tree::act` and
   `Tree::join`, removes the bulk deallocation from the lock hold), and (b)
   the fuller evacuation that collects the mid-walk drops into a sink the walk
   hands back? Recommendation: document now and do (a); treat (b) as a design
   proposal.
2. Finding 2 (retire carve-out): is the accurate sentence enough, or do you
   want the cancel-at-each-await retire test alongside it? Recommendation:
   both; the bounded-poll harness in `tests/lifecycle.rs` already does the
   hard part.
3. Finding 4: should `warm_caches` (or a named equivalent) become public API so
   applications can pre-pay the first-greeting materialization? API addition,
   your call; the doc sentence is warranted either way.

## Dropped

- Sweep [3], the `CausalMessages` half: causal.rs:28-32 already states that
  the time to retrieve each message "may burst arbitrarily large, up to the
  total size of the messages stored", which is the per-poll statement at the
  user's altitude; only the greeting half survives as async-hazards-4.

<!-- source: sweeps-final/clippy-pedantic.md -->
# Sweep clippy-pedantic: Pedantic and nursery clippy lints, judged

## Method and coverage

The sweep ran one clippy invocation with `-W clippy::pedantic -W clippy::nursery`
over the rumors package at commit 9e5784fb (log:
`<session scratchpad>/sweeps/clippy-pedantic.log`,
exit 0) and parsed it into deduplicated (lint, file, line) tuples: 2383
crate-wide, 1103 under rumors' own `src/`, `tests/`, `benches/`, and
`examples/`, the remainder in `crates/before` and `crates/suanpan` (out of
scope, excluded). Five lint groups (`use_self`, `redundant_pub_crate`,
`missing_const_for_fn`, `cast_precision_loss`, `cast_possible_truncation`)
account for 64% of the in-scope hits; the sweep judged all five not worth
adopting and I concur with each ruling (reasons under Dropped and Positives).

This finalization pass opened every line range the seventeen sweep findings
cite and quoted from the file, never from the sweep's report. Beyond reading:

- Grep for use sites where a claim depends on them (`link_header`'s callers,
  `MAX_QUERY_CHILDREN`'s type, the receivers of `backend()` in both `work.rs`
  files, `let ... else` and `<'_` counts in `src/`).
- `git log -L` on `pump.rs:175` and `header.rs:248`, `git log -S` on the
  traverse globs; grep of `.agent-notes/` for rulings on casts, pedantic
  lints, globs, and lifetimes. The 2026-08-20 CBOR wire review prescribes
  `usize::try_from`/`u8::try_from` at codec sites (REVIEW.md:473, 581);
  nothing recorded touches lifetimes, globs, receivers, or pedantic lints.
- Read proptest 1.11.0's `prop_assert_eq!` from the cargo registry to settle
  three `redundant_clone` hits.
- Enumerated all twelve `cast_possible_truncation` hits in non-test library
  code and classified each: the five in finding 1; `cbor.rs:111-119`
  (match-arm bounded); `codec.rs:150` (a deliberate wrapping fill);
  `budget.rs:58` (inside a `const` initializer, where `TryFrom` is not
  callable, so `as` is the only spelling); `window.rs:653` (a byte depth
  bounded by the tree height).
- Zero test invocations: no finding rests on a runtime correctness claim.

What this pass could not see: whether any proposed rewrite compiles (no
build permitted), so the `&self` and `'_` changes are compile-unverified;
whether rustdoc actually renders `observe.rs:182` as two code spans (assessed
from rustdoc's inline-code styling, not rendered); the remaining
`redundant_clone` hits beyond the eleven examined.

Under this lens the partition is clean. Nothing surfaced is a lossy cast, a
truncation, or an overflow reachable from wire, payload, or environment
input. One finding at severity low touches signature semantics (`&mut`
receivers that never mutate); one at low is a five-site idiom straggler the
crate's own recorded direction already names; the rest are nits.


## Positives

- `await_holding_lock` (pedantic) produced zero hits anywhere in the run log
  (verified: `grep -c` over the log is 0): no std mutex guard is held across
  an `.await` in rumors.
- `future_not_send` did not fire on the public conformance entry
  `conformance::link::check` (`src/conformance/link.rs:158`, `pub async fn`);
  its five hits are the crate-private, test-only backend gate
  (`src/conformance.rs:19`: `#[cfg(test)] pub(crate) mod backend;`) and two
  test bodies in `src/link/routed/tests.rs`. A library user can spawn the
  public suite on a multi-threaded runtime.
- Every narrowing `as` in non-test library code (twelve
  `cast_possible_truncation` hits, all enumerated above) sits behind a range
  check, a match arm, a `const` bound, or a deliberate wrapping intent; no
  lossy cast is reachable from wire, payload, or environment input.
  `streams.rs:729-731` already writes the `try_from` shape finding 1 asks the
  stragglers to adopt, and the 2026-08-20 CBOR wire review recorded that
  direction.
- Match arms name the enum explicitly rather than `Self::` (a rough regex
  over `src/` counts 154 explicit `=> Type::Variant` arms to 13 `=> Self::`),
  a consistent house style; `error.rs:295`
  (`handshake::Error::Io(error) => Error::Io(error),`) shows the explicit
  name disambiguating two `Error` types in one arm.
- The manual `Clone` impls that `expl_impl_clone_on_copy` flags carry their
  rationale in place (`path.rs:61`: `// Manual copy/clone impls so we don't
  require unnecessary bounds on `H`:`; `prefix.rs:202` likewise); the derive
  the lint suggests would regress it.
- `#[must_use = "..."]` is applied with a stated reason at eight sites, e.g.
  `rumors.rs:616` (`"the driver does nothing until the returned stream is
  polled"`) and `tree.rs:694` (`"dropping the guard disarms the fuse"`), so
  the lone `return_self_not_must_use` hit on `Speaker::other` is rightly not
  adopted.
- `let ... else` is the crate's idiom for a diverging bind (107 sites in
  `src/`); finding 9's two library sites are the only stragglers.

## Open questions for Finch

1. `header.rs:248-254`: minimal `u8::try_from(..).expect(..)` beside the
   kept `debug_assert!`, or a validated-length newtype for the encoded
   advertised name that `Endpoint` constructs at `endpoint.rs:205` and
   `link_header` accepts, deleting assert and cast together? The newtype is
   the types-first answer and the change is crate-private; recommendation:
   the newtype.
2. `adversarial.rs:101`: `cx: &Context<'_>` (accurate) or keep
   `&mut Context<'_>` (the universal poll-adjacent convention)?
   Recommendation: keep `&mut`, and note the lint as a false positive by
   convention if the rest of finding 2 lands.
3. Lint policy: the stragglers in findings 3, 4, 9, 10 stay fixed only if
   something enforces them. `[lints.clippy]` in `Cargo.toml` with
   `elidable_lifetime_names`, `redundant_closure_for_method_calls`,
   `manual_let_else`, and `match_wildcard_for_single_variants` at `warn`
   would ride the existing `-D warnings` gate. Adopt, or leave pedantic
   lints as a periodic sweep? Recommendation: adopt those four; they had
   zero false positives in this run.
4. Carried from the sweep: the CBOR additional-information values 24..27
   recur at `cbor.rs:83, 112-122, 172-184, 273-276`; named constants tying
   the head grammar's functions together is a taste call the sweep raised
   and I did not evaluate further.

## Dropped

- Sweep [2] `used_underscore_binding`/`unused_self` in `progress.rs`: the
  underscore prefix is Rust's spelling for a parameter unused under some
  cfg, and the `#[cfg(test)]` on the next line names the cfg; neither
  `#[cfg(not(test))] let _ = (..)` nor `#[cfg_attr(not(test),
  allow(unused_variables))]` reads better, and the cost of the current form
  is nil. The lint is pedantic because this pattern is idiomatic.
- Sweep [7] `range_plus_one` in `cbor.rs`: `bytes[1..1 + extension]` shows
  the slice length (`extension` bytes after the initial byte) directly,
  which is the quantity a reader of a head parser checks; the inclusive
  form moves that arithmetic into the reader's head. No nameable cost to the
  current form.
- Sweep open-question rulings on `use_self`, `redundant_pub_crate`,
  `missing_const_for_fn`, `cast_precision_loss`, `cast_sign_loss`,
  `cast_possible_wrap`, `or_fun_call` (the six `ok_or` sites),
  `semicolon_if_nothing_returned`, `option_if_let_else`,
  `single_match_else`, `similar_names`, `too_many_lines`,
  `significant_drop_tightening`, `large_types_passed_by_value`,
  `needless_pass_by_value`, `match_same_arms`, `manual_assert`,
  `naive_bytecount`, `suspicious_operation_groupings`, `option_option`,
  `while_float`, `many_single_char_names`, `suboptimal_flops`,
  `duration_suboptimal_units`, `missing_fields_in_debug`,
  `collection_is_never_read`, `match_wild_err_arm`, `ref_option`: concur
  with the sweep's not-adopted ruling on each; none names a cost.
- `budget.rs:58` `radix as u8`: inside a `const` initializer, where
  `TryFrom` is not callable; not a straggler of finding 1.
- Sweep [3]'s five relating-lifetime sites (gossip.rs:242, 269, 1186;
  backend.rs:198; local.rs:191): the named lifetime relates an input to a
  `BoxFuture<'a, _>` or `impl .. + 'a` output; keep.
- Sweep [15]'s hits at tests/bootstrap.rs:71, 104, backend/local/tests.rs:129
  (`prop_assert_eq!` moves both operands; the clone is required),
  tests/network.rs:36 (the clone is the test's subject), and probably
  tests/observe.rs:334, 340 (outer binding reused at 378-393): lint false
  positives.

<!-- source: sweeps-final/deps.md -->
# Sweep deps: Dependency and feature audit

## Method and coverage

This pass disputed the sweep's eleven findings against the tree at
9e5784fb4dce977cfbdfd1619886d1482b5ce764 (working tree clean, verified with
`git rev-parse HEAD` and `git status --porcelain`). For every finding I opened
the cited lines with line numbers (`cat -n`, `sed -n`), re-ran the use-site
census with `/usr/bin/grep -rn` over `src/`, `tests/`, `benches/`,
`examples/`, and every member `Cargo.toml` (`grep` on this machine is aliased
to ugrep, which rejects the bounded-repetition patterns; the census used the
system grep throughout), and read the registry sources the claims rest on:
static_assertions 1.1.0 (`assert_eq_size.rs`, `assert_eq_align.rs`), futures
0.3.32 and futures-util 0.3.32 (the versions Cargo.lock resolves), tokio
1.52.3, and serde 1.0.228. For the docs.rs finding I ran `strings` over
`librustc_driver-*.dylib` in the pinned nightly (nightly-2026-06-30) and in
stable 1.97.1 (the `rust-toolchain.toml` channel) and extracted the
removed-features neighborhoods. History came from `git log -S` on each
originating token (`static_assertions`, `forbid(unsafe_code)`,
`doc_auto_cfg`, `futures-util`, `tokio-stream`, `pub use ::before`,
`seed_rng`, `nightly_toolchain :=`, `toolchain: nightly`) and from a grep of
`.agent-notes/` and `design/` for every one of those tokens; the two prior
review packets record no ruling on any of them. The workspace-table census
(every `[workspace.dependencies]` key counted against `workspace = true`
inheritors across the root and member manifests) found no orphan.

What this pass could not see: no cargo, rustdoc, or `cargo tree` run was
permitted, so the docs.rs breakage is inferred from the compiler's embedded
feature table rather than demonstrated, and dependency-graph claims rest on
the manifests and Cargo.lock rather than on a resolved tree. Neither of the
two permitted `cargo nextest` invocations was spent: no finding turned on a
runtime behavior a test could settle. The `.claude/worktrees/` directory
holds another agent's worktree; it was excluded from every census and left
untouched.


## Positives

- The workspace dependency table is the single version of record and the census confirms it: every `[workspace.dependencies]` key has at least one `workspace = true` inheritor across the root and member manifests (verified by script), and `tools/manifestlint`'s header states both the rule and the reason cargo cannot check it itself.
- The tokio normal-dependency feature set (`io-util`, `macros`, `sync`) matches what the shipped library uses: `tokio::select!` at six non-test sites (tasks.rs:26, materialized.rs:841, driver.rs:73, remote/proxy/work.rs:215, peer/gossip.rs:966, link/routed/router.rs:149), and no `tokio::task`, `tokio::spawn`, `tokio::time`, or `tokio::runtime` outside doctests and test scaffolding (verified by grep). The crate docs' runtime-independence claim is true of the manifest.
- `conformance = []` adds no tokio features; the suite's only mention of `tokio::task::yield_now` is the contrast sentence at conformance/link.rs:684, and the `features` leg (justfile:522) checks the feature alone.
- `height.rs:166` already states a type-level fact with `const _: () = assert!(...)`: the idiom deps-1 asks for is in the file.
- The ciborium exact pin (Cargo.toml:36-46) and the cbor-diag fork comment (26-35) state the invariant and the bump procedure at the right altitude; the `[lib] bench = false` comment (97-106) names the concrete cost it avoids.
- The sibling crates' `[package]` tables are complete and consistent (description, MPL-2.0, `publish = false` where test-only), which is what makes deps-12 a one-line fix rather than a decision.
- The justfile's nightly-pin comment (23-38) gives the reproducibility argument in full and names the paired bump procedure; deps-7 is about ci.yml not having followed it, not about the argument.

## Open questions for Finch

1. decode.rs builds its leaf channels on raw `tokio::sync::mpsc` (lines 83 and 288) rather than channel.rs's `channel(QueueRole, ..)`, so in tests that edge escapes the instrumented channel's capacity limits, delay schedule, and coverage assertions (`QueueKind::PROXY` lists three proxy edges; this one is not among them). Is that deliberate (the adapter's internal fan-out is not a protocol edge) or an omission the streaming partition should pick up? It bears on which option deps-2 takes.
2. rand is on the 0.8 line workspace-wide while 0.9 changed the core traits. Nothing in rumors forces a bump; `seed_rng` (deps-5) is the one place the choice leaks to users, and gating it makes the bump a private matter.
3. The `meter` feature reaches `before::meter` two ways (rumors' feature forwards to `before/limb-meter` and `before/scan-meter`, each implying `before/meter`; the dev-dependency lights `before/meter` directly for `encoded_bits`). Sound, but one sentence at Cargo.toml:117-122 saying which route each consumer relies on would save the next reader the derivation.
4. deps-3's construction command is one nightly rustdoc; running it before any of this lands settles the medium confidence and shows whether docs.rs has ever built this crate's docs.
5. For deps-7: do you want CI to read the nightly pin from the justfile (a `just --evaluate nightly_toolchain` step), or to carry a second copy with the two sites naming each other?

## Dropped

- The sweep's claim under the `pub use ::before;` finding that "docs.rs renders before's surface under rumors": rustdoc renders an external-crate re-export as a `pub use before;` line linking to that crate's own docs and does not inline it without `#[doc(inline)]`; the semver coupling and duplicate spellings stand, the rendering claim does not (assessed; no rustdoc run permitted).
- The sweep's count of tokio-stream's footprint as "four sites": decode.rs:8 (uses at 84 and 404) is a fifth, on raw tokio channels, and changes what dissolving the dependency costs (folded into deps-2).
- The sweep's forwarding surface for the production `Receiver` wrapper, which listed `try_recv` at driver.rs:80 and streams.rs:570: both are raw `tokio::sync::mpsc` receivers outside channel.rs, and the instrumented `Receiver` exposes only `recv`; the wrapper forwards `recv` alone (folded into deps-2).
- The `pub use ::before;` finding's medium severity: lowered to low, because the re-export has the standard version-pinning rationale for `before` types in rumors' public signatures and the actionable remainder is recording the choice.

<!-- source: sweeps-final/fresh-eyes.md -->
# Sweep fresh-eyes: Fresh-eyes user: a scratch application from the public docs alone

## Method and coverage

The sweep read the crate as a first-time user (README, crate docs, tutorial,
every `pub` item's rustdoc) and wrote a 500-line application against those
docs alone; its artifacts sit in the shared scratchpad under
`sweeps/fresh-eyes/` (`check-1.log`, `run.log`, `scratch-app/`). This pass
disputed each of its ten findings against the tree at 9e5784fb (clean
working tree, verified with `git rev-parse HEAD` and `git status`):

- Opened every cited site with line numbers (`sed -n` ranges, `cat -n`) and
  quoted from the file, never from the sweep's report.
- Grepped use sites for every identifier a finding turns on (`STREAM_COUNT`,
  `Protocol`, `selected`, `BufReader`, `Bookmark for`, `fn rumors(`,
  `unnameable_types`, `SCOPE_ENVELOPE_BYTES`, `seam`).
- Read history where a finding might be a recorded ruling: `git show
  368da2a5` (the V1 retirement) and its message, `git log -S` on the
  affected phrases, `git show b16a800b5:src/peer.rs` (the routed-link
  commit), and the notes `.agent-notes/2026-09-01-v1-retirement/README.md`,
  `.agent-notes/2026-08-20-cbor-wire-review/REVIEW.md` (behavioral note
  N1), and `.agent-notes/2026-07-22-sync-budget/sync-budget.md`.
- Corroborated the sweep's two compiler probes against its own log:
  `check-1.log:77` carries `error[E0603]: module `snapshot` is private` and
  `check-1.log:88` carries `error[E0599]: no method named `protocol` found
  for struct `Peer<T, B>``. Its runtime observations (`run.log`) were read,
  not reproduced.
- Ran no cargo, just, or test command. Both permitted `nextest` invocations
  went unused: no finding rests on a runtime claim that reading did not
  settle (the one candidate, the stream count, is two literal constants and
  a pin test read in `src/link/tests.rs`).

What this pass could not see: docs.rs rendering. Every "reachable from the
public API" claim is inferred from `pub`/`pub(crate)` visibility and `mod`
privacy in `src/lib.rs`, not from a rendered page.

Outcome: all ten sweep findings survive. Two are widened (fresh-eyes-2 gains
sites and a recorded-ruling history; fresh-eyes-5 gains its cause), two are
reframed (fresh-eyes-7's mechanism and history; fresh-eyes-8's class), and
one nit is new (fresh-eyes-11). No correctness defect was found under this
lens.


## Positives

- The public docs alone were enough to write a complete application: the
  sweep's custom `Bookmark`, three-level wire `Observer`, routed TCP
  `Dial`/`Listen` pair, stoppable `gossip_when` drivers, and all three set
  observers compiled on the first `cargo check`, with the only errors being
  its two deliberate probes (I read `check-1.log`: exactly two `error[` lines,
  at 77 and 88).
- Every `#[must_use]` on an outcome type says what a silent drop loses; I
  verified the messages at bootstrap.rs:58, 278, 364, gossip.rs:92, 148, 912,
  and rumors.rs:616. Line 364's "every `Joined` variant carries a peer or
  the bookmark; dropping it loses one or the other" is the model.
- `Changes`' docs pre-empt the naive `loop { changes.next(); gossip() }`
  deadlock and route the reader to `gossip_when` (src/rumors/changes.rs:37-
  43, verified). The `Error` table (error.rs:10-28) gives one actionable
  sentence per variant, and the two sites this pass found wanting
  (fresh-eyes-2, fresh-eyes-10) are wording, not structure.
- The V1 retirement's prose re-denomination was thorough: the commit message
  for 368da2a5 enumerates the surfaces it re-stated, and the leftovers found
  here (fresh-eyes-2's "select" vocabulary, fresh-eyes-5's count) are the
  residue of a sweep that caught nearly everything. The retirement note's
  numbered rulings made attributing each leftover to a decision immediate.
- The sweep's runtime observations (`run.log`, read not reproduced): both
  ends' drivers ended cleanly on stream end and on remote hang-up; concurrent
  one-shot `gossip` on both ends merged into one session with both sides
  reporting `Led::Local`, as rumors.rs and gossip.rs:184-185 say; one side's
  `bytes_sent` equalled the other's `bytes_received` over TCP (69/69);
  redactions issued on each side were honored on both in one session.

## Open questions for Finch

1. Bookmark size (the sweep's question, answered by reading): Bob's record
   was 68 bytes after his bookmarked join and 68 bytes after three driven
   sessions carrying his own sends. gossip.rs:681-715 re-records and writes
   the bookmark at every session start whenever the live `(party, version)`
   differs from what was last persisted (`is_current` false), and a send
   advances `inner.tree.latest()`, so those writes did run; the equal size
   means the advanced version encoded to the same width. Assessed from code,
   not run. If you want certainty, one `eprintln!` in the sweep's
   `FileBookmark::store` settles it.
2. fresh-eyes-4: which shape do you want, `Snapshot::iter` returning a
   nameable `Iter` re-exported at the root, or an `impl` return with a
   nameable `IntoIter`? Either way I recommend `#![warn(unnameable_types)]`
   as the committed check.
3. fresh-eyes-7: should the routed adapter wrap its own read halves in
   `BufReader`, since it owns the split, or stay a pass-through and document
   the option?
4. fresh-eyes-9: return the whole `BookmarkedBootstrap` from `Bailed`/
   `Failed`, or document the clone-before-bookmark pattern?
5. src/tutorial.rs:29 instructs `rumors = "0.1"`; Cargo.toml is 0.1.0. Fine
   for a crate that publishes as 0.1; worth a line if the tutorial is read
   before publication.

## Dropped

- Candidate: reconciliation.rs:237-249 "Why streaming, not level-synchronous exchange" as a ghost of V1. Dropped: the V1-retirement note rules it "the design rationale it is: the level-synchronous shape described as the naive alternative", and the text names no removed code; only its `Tree::join` sentence is flagged, under fresh-eyes-3.
- Candidate: dissolve the one-variant `Protocol` enum under Principle 3. Dropped: commit 368da2a5's message keeps it deliberately ("a knob with one position; it returns with a V3"); fresh-eyes-2 targets the vocabulary only.
- Candidate: run `stream_count_matches_the_codec` to settle fresh-eyes-1 mechanically. Dropped: both constants are literal `17`s read at their definitions, and the machine is shared with other reviewers.
- The sweep's classification of fresh-eyes-8 as feature-gap. Reframed to documentation rather than dropped; the shipped-impl half stays as an owner option.

<!-- source: sweeps-final/inventory.md -->
# Sweep inventory: Allow attributes, panic sites, and visibility inventory

## Method and coverage

The sweep ran three mechanical inventories over the 113 production source
files under `src/` (every `tests.rs` sibling and `tests/` directory
excluded; `cfg(test)`, `test-internals`, and `conformance` regions noted per
site), with scripts and raw output under
`scratchpad/sweeps/inventory/` (`inventory.sh` for allow/expect attributes,
panic macros, asserts, `as` casts, division, and indexing; `visibility.py`
for module reachability, pub-in-private items, and `pub(crate)` referencer
counts). Its counts: 39 allow sites and no `#[expect]`; 315 panic-macro,
`unwrap`, and `expect` lines and 136 assert lines, of which 88 production
panic sites remain after removing doc examples, non-panic `Reader::expect`
calls, and test regions; 92 `as` casts; 10 division sites; 98 indexing
sites; 46 crate-root re-exports over 9 publicly reachable modules.

This pass disputed every finding against the tree at
9e5784fb4dce977cfbdfd1619886d1482b5ce764. For each one I opened the cited
lines with line numbers, grepped for use sites across `src/`, `tests/`,
`benches/`, `examples/`, and `crates/`, ran `git blame` at the five sites
where history could carry a rationale (snapshot.rs:7, streaming.rs:43,
levels.rs:67, protocol.rs:10, bookmark.rs:302), and grepped `.agent-notes/`
and `design/` for rulings on every topic (pub-in-private, `unreachable_pub`,
`tree::Iter`, `warm_caches`, `seed_rng`, `static_assertions`,
`type_complexity`, `missing_const_for_thread_local`, `KEY_DEPTH`, poison
handling, `ensure_loaded`, `LEAF_TAG`, the greeting roster). The two review
packets (`2026-08-20-cbor-wire-review`, `2026-07-23-review-link-transport`)
verify the greeting key order and the bulk-assemble equivalence tests but
rule on neither the parse shape nor the `from_sorted_leaves` asserts; the
`2026-07-18-node-hash-preimage` note states the leaf preimage as
`LEAF_TAG ‖ prefix_len ‖ prefix`, corroborating inventory-9. I counted
column-0 `pub` items under `src/tree/` and `src/message.rs` independently
(218 in non-`tests.rs` files before subtracting re-exported names and
`cfg(test)`-only files) to check inventory-10's order of magnitude, and
counted parameters at all seven `too_many_arguments` sites for inventory-12.

I used neither of the two permitted test invocations: no finding is a
correctness claim a test run would settle. What this pass could not see:
clippy and rustdoc were not run, so lint-scoping and threshold claims
(inventory-3, inventory-12) rest on the documented semantics plus one
in-tree corroboration (a seven-parameter function with no allow passes the
`-D warnings` gate), and the illumos clippy misfire behind inventory-13's
rationale is unverified either way.

Twenty findings survive; two are reframed (inventory-4, inventory-6) and
one component is dropped (see Dropped).


## Positives

- Zero caller-, environment-, or wire-reachable panic sites across the 88 production panic-macro sites and 8 release asserts the sweep inventoried. I re-verified the wire-facing parsers named for this sweep: `parse_greeting` returns `GreetingError` on every deviation (greeting.rs:122-203), `read_head` returns `HeadError` for truncation, reserved, indefinite, and non-shortest heads (cbor.rs:166-197), and the leaf decoder returns `OversizedVersion`, `LeafOutsideScope`, `LeafOrder`, and `SupplyOrder` before any run reaches `from_sorted_leaves` (decode.rs:497-540).
- Compile-time pins where they matter, both verified: `const _: () = assert!(std::mem::size_of::<typed::Node<Z>>() == std::mem::size_of::<*const ()>());` at local.rs:110 with its reason stated ("the window's per-reference price rests on it"), and `const _: () = assert!(H0::HEIGHT == 0 && H32::HEIGHT == 32);` at height.rs:166.
- Mutex poison handled deliberately with a stated rationale at router.rs:56-69 and observe.rs:332-343 (production) and memnet.rs:84-91 (test-internals): each says why a single-operation critical section cannot be torn.
- Allows that carry their reasons, verified: `too_many_arguments` at proxy/work.rs:101-102 and proxy/start.rs:338-339 each state the trade ("one premise per argument"), and every other `too_many_arguments` allow sits on a function that clippy would flag.
- The `PartyGuard` drop-recovery comment at gossip.rs:1389-1392 explains why the join runs unconditionally rather than inside a `debug_assert!`: a maintainer-altitude comment stating what the code cannot show.
- Sweep-reported, not re-verified by this pass: recursion discipline throughout (iterative leaf walks, typed traversals bottoming out at `Z`, erased `unknown` bounded by remaining height, the capture renderer's `MAX_DEPTH`); saturating or checked arithmetic on peer-declared quantities (window.rs:432-435, frame.rs `record_len`/`push`, `SupplyLedger::charge`, `PayloadDepthLimit::recursion_limit`); and all 17 `Stream::at_height` pairings the proxy states request resolving against the stream-height table.

## Open questions for Finch

1. `seed_rng` (peer.rs:212-213) is hidden but is also the natural deterministic-seeding entry a downstream test suite would want. Gate it behind `test-internals` (with a private inner function for `seed`), or un-hide and document it? Recommendation: un-hide it; deterministic seeding is a legitimate testing need for users, and the `#[doc(hidden)]` is the only thing making it look internal.
2. The observers' `channel: Option<Channel<T>>` (unordered.rs:40, causal.rs, changes.rs): should `None` mean "ended" (a fused stream that returns `Poll::Ready(None)` forever after close), which dissolves the four `expect("channel state present")` and three `unreachable!("matched Ready above")` sites in one design move? Today a closed observer restores `Ready(rx)` and re-polls open a fresh pass. Recommendation: yes if the fused semantics are acceptable; otherwise the receiver-clone transition removes the `Option` without changing behavior.
3. The `Backend` trait's generality (erase/assume, `node_bytes` pricing, `leaves`/`assemble` overrides, the conformance backend suite) and the release asserts enforcing its contract in adapter/encode.rs:223-262 and adapter/decode.rs:427-439 serve backends that do not yet exist; `Local` is the sole implementor and the trait is unreachable from outside the crate. Designed-ahead, or machinery awaiting a constraint? A design-lens question the inventory cannot settle.
4. The thirteen `missing_const_for_thread_local` allows rest on a claim that clippy misfires on illumos's fallback-TLS lowering. Neither the sweep nor this pass ran clippy on illumos; does the pinned toolchain still exhibit it? If not, the allows and their rationale can go entirely rather than being consolidated.
5. `Stream::at_height` returns `Option` with exactly one production caller expecting `Some` (state.rs:90). A type-level height-to-stream mapping would make the schedule/stride agreement a compile-time fact; the snapshot tests pin it today. Design proposal, not a defect.
6. The debug-only `debug_assert!(false, ..)` guards at batch.rs:137 and gossip.rs:1395 degrade to a no-op (a dropped batch, a leaked fork) in release if the Peer/Rumors exclusivity or party linearity were violated; both are type-enforced, so this is the most benign behavior available, but you may prefer an explicit error path for the batch case.

## Dropped

- Sweep [4]'s `StreamReceiver` component (streams.rs:305-311, 378-396): the `start`/`frames` pair of `Option`s is the consume-once idiom absent a placeholder value for `ReceiverStart` (a `oneshot::Receiver` has none); no panic-free spelling exists without a dependency, and `frames`'s absence carries meaning for `finish`. Not a finding.
- Sweep [4]'s claim that a take-and-restore `match` removes the observers' `expect("channel state present")`: moving the payload out of an `Option` still leaves a `None` arm, so the panic relocates unless `None` is given a meaning. Reframed into inventory-6's design option and open question 2.
- Sweep [5]'s resolution as written for `seed_rng`: `Peer::seed` calls it (peer.rs:207), so the gate cannot be applied directly. Reframed in inventory-4.
- Sweep [6]'s class `verification-gap`: nothing is unverified; the finding is about assert discipline and debug cost. Reclassified as `idiom` in inventory-7.

<!-- source: sweeps-final/module-graph.md -->
# Sweep module-graph: Module dependency graph and layering

## Method and coverage

The sweep built the module graph of `src/` with a script (`scratchpad/sweeps/module-graph/`: `modgraph.py` extracts every `mod` declaration and every `use crate::`/`super::`/`self::` path and resolves each to a module; `analyze.py` computes SCCs, top-level layers, and sibling-subtree cycles). I did not re-run the script; I read its `analysis.txt` and then opened every site every finding cites, at HEAD `9e5784fb4dce977cfbdfd1619886d1482b5ce764` in a clean tree.

Mechanical checks I ran myself, read-only:

- `grep` for every use site of `Inner`, `Protocol`, `children_of`, `SupplyLedger`, `DEFAULT_TARGET_MESSAGE_SIZE`, `alternating`, `Levels`, `untyped::Range`, `traverse::act`, `traverse::unknown`, `mirror::remote`, `mpsc`, and the `materialized::channel` shim's consumers.
- A shell loop over every non-test `.rs` under `src/` printing the first non-blank line where it is not `//!`: 27 files, matching the sweep's count.
- A grep for brace-bodied `mod` declarations, and a brace-matching script for their spans: six inline test modules (matching the sweep) and, beyond the sweep's three, two more inline production modules (`untyped::census`, 27 lines; `decode::fan_probe`, 37 lines) plus `height::sealed` (6 lines, the sealed-trait idiom, exempt).
- `git ls-tree HEAD~60 src/tree/mirror/` (shows `alternating.rs` beside `streaming.rs`), `git log --grep=alternating`, `git log -1 368da2a5` and `c13c21b4`, `git log --follow` on the shim and glob files, `git blame` on `remote.rs:85`.
- `.agent-notes/2026-09-01-v1-retirement/README.md` for the rulings on `Protocol`; `.agent-notes/2026-08-20-cbor-wire-review/REVIEW.md` for the error re-export routing ruling; grep of both review packets for anything on the sites below.
- `tools/` and the `justfile`: nothing mechanical checks test-module placement, `//!` presence, or the `#[cfg(test)]` convention; `doclint` checks summary length and `include_str!` separation only; there is no `[lints]` table anywhere in the workspace.

Not run: cargo, just, or any test (no finding rests on a correctness claim). Not seen: `benches/`, `examples/`, and the sibling crates, except where a finding names them.


## Positives

- Every `.rs` file under `src/` is declared exactly once (183 files, 182 `mod` declarations plus the root); no orphan files, no duplicate declarations. (The sweep's count from its `mod_decls.txt`; I did not recount it.)
- The top-level layering reads cleanly once the two single-site upward edges of module-graph-1 are set aside: `message`/`tags`/`network`/`protocol` at the bottom, `link` and `observe` above, `tree` above those, `batch`/`bookmark`/`snapshot`, then `peer`/`rumors`/`error`.
- `streaming.rs:1-40` is a real orientation map: each child module is named with the one reason it is separate, and it says where to start reading.
- `handshake.rs:170-198` defines a local `pub(crate) enum Error` that `error.rs:292-314` lifts by an exhaustive `From` impl: the right layering shape and the template for module-graph-1.
- `define_peer!` (`protocol/peer.rs:12-106`) plus the type-level height descent in `protocol.rs:163-203` make the exchange schedule a compile-time fact: a wrong round count cannot build. (This is what lets module-graph-5 drop the sweep's proposed const-assert.)
- The cargo features each gate something real: `conformance` gates `pub mod conformance` and the link suite; `test-internals` gates `testing`, the capture renderer, the node census, and the window introspection; `meter` gates exactly the metering test modules. Every gate's comment in `Cargo.toml:108-122` says who lights it and why.
- `testing.rs`'s forwarders each name the integration suite that consumes them (`tests/decode_alloc.rs`, `tests/encode_alloc.rs`, `tests/dispute_wire.rs`), so the `test-internals` boundary documents itself even where its contents should split (module-graph-7).
- `tree::mirror::cbor` states why it is hand-written and exactly what it owns, and `bookmark::format` reuses that boundary without pulling in the wire protocol.
- `budget.rs:64-71` derives `DEFAULT_TARGET_MESSAGE_SIZE` from the wire constants rather than measuring it, and says so; `window.rs:139-156` prices slots by `size_of` the real types for the same reason. Both are the "no hand-maintained number" rule applied to memory pricing.
- The V1 retirement is recorded end to end: the plan note carries the three rulings, `368da2a5` names everything deleted and simplified, and `c13c21b4` names what it re-denominated and what it deliberately left. The residue in module-graph-4 is small against that.

## Open questions for Finch

1. `window ↔ materialized`: `window.rs:154-156` prices `REFERENCE_SLOT_BYTES` by `size_of::<(u8, Resolve<...>)>` (the sweep attributed this to `FAN_SLOT_BYTES`, which uses only `typed` types). Is that pricing edge intended as the one deliberate mutual dependence in the streaming core? If so, a sentence at `window.rs:123` closes the question, and module-graph-2's other moves can proceed independently.
2. Is the closed-receiver divergence between the walk's `pump` (returns `Ok(())`) and the proxy's `pump` (parks via `send_or_cancel`) a decision? (module-graph-13)
3. `streaming` as a path segment: fold into `tree::mirror`, rename to what it now distinguishes, or keep and re-word `mirror.rs:3`? (module-graph-9)
4. `adapter/decode.rs:83` and `:288` build their reader/assembler fan edges with raw `tokio::sync::mpsc::channel(FAN)` and a private `cfg(test)` `fan_probe`, outside `streaming::channel`'s named-queue instrumentation. Deliberate (a transient edge inside one decode call, not a protocol edge the schedule tests perturb), or an edge the `with_schedule` machinery should also reach? I did not pursue this beyond noting it; it is a test-coverage question more than a graph one.
5. `unreachable_pub`: worth the crate-wide diagnostic sweep it implies, or is normalizing the ten `pub mod` sites by hand the right size? (module-graph-12)

## Dropped

- Sweep [1] resolution (c), "move `DEFAULT_TARGET_MESSAGE_SIZE` to `message.rs`": the constant is `FAN * FULL_FAN_QUERY_FRAME_LEN`, derived from codec frame constants at `budget.rs:52-71`; moving it would invert a `codec → message` edge. Replaced by the thread-through or document options in module-graph-2.
- Sweep [1] resolution (b) as written, "move `SupplyLedger` whole": `absorb` (`materialized.rs:245-250`) returns `materialized::Error` and constructs `Violation::OverdrawnSupply`; only `new`/`charge` can move, with `absorb` staying as an inherent impl in `materialized`. Reframed in module-graph-2.
- Sweep [3]'s option to relocate `Protocol` into `tree::mirror::handshake`: the V1 plan ruled the enum stays public as wire vocabulary, and moving its definition behind the same root re-export changes nothing a reader sees. Kept only the prose fix (module-graph-4).
- Sweep [4]'s "derive both sites from one constant or add a `const _: () = assert!(...)`": the type-level height descent (`protocol.rs:163-168`, `:198`) already makes a mismatched round count a compile error. Kept only the location and layout corrections (module-graph-5).
- Sweep [11]'s "the ten `pub mod` sites under src/tree are the expected hits" for `unreachable_pub`: the lint fires on every `pub` item unreachable from the crate root, so the hit count is far larger. Reframed in module-graph-12.
- Sweep open question's attribution of the `Resolve` pricing edge to `FAN_SLOT_BYTES` (`window.rs:139-142`): it is `REFERENCE_SLOT_BYTES` at `:154-156`. Corrected in open question 1.
- A suspected ghost link `[`queues`]` at `remote/proxy/work/pump.rs:25`: `proxy/work.rs:41` declares `mod queues;`, so the link resolves. Not a finding.

<!-- source: sweeps-final/prose-hygiene.md -->
# Sweep prose-hygiene: Ghost references, temporal language, dialect tells

## Method and coverage

This is the verification pass over the prose-hygiene sweep at commit
9e5784fb4dce977cfbdfd1619886d1482b5ce764 (verified: `git rev-parse HEAD`
matches, `git status --porcelain` is empty). Every one of the sweep's twelve
findings was disputed by opening the cited lines with line numbers (`sed -n`
piped through `cat -n`), re-running the greps that produced the counts, and
checking history with `git show --stat`, `git log -S`, and `git log
--format=%ci` where a finding rested on a deletion or an ordering of commits.

Mechanical checks run for this pass:

- Deleted-identifier ghost hunt: extracted every `fn|struct|trait|enum|type|
  mod|const|macro_rules!` name on the deletion side of `git show 368da2a5`
  (the `Protocol::V1` retirement; 110 files, 7611 deletions), kept the
  CamelCase and underscore names (202), dropped those with a surviving
  definition anywhere in `src`, `tests`, `benches`, `examples`, or `crates`,
  and grepped the rest across the crate's prose. Surviving ghosts:
  `DecodeNode`, `serialize_to`, `crate::tree::wire` (src/tree/typed/node.rs),
  `Levels` (src/tree/traverse.rs and tests/future_size.rs), `Below`
  (tests/future_size.rs). No `V1`, `LEGACY_MAGIC`, `Alternating`, or
  `capture_*_v1` reference survives.
- Roster-tag anchoring: for each letter-number tag cited from code (`B5`,
  `F4`, `T3`, `D5`, `d5`, `d6`, `finding #6`, `finding #7`, `Bridge 1/2/3`,
  `charter`) grepped `formal/lean/**/*.lean`, `formal/MODEL.md`,
  `formal/PROGRESS.md`, `formal/PLAN.md`, and `.agent-notes/` for a home.
- Lean names cited from code (`wc_impossibility`, `viewEnc`, `LocalEq`,
  `wedge`, `asmResList`, `fan`, `capLevel`): each resolves to a definition,
  theorem, or structure field under `formal/lean`.
- Em-dash census by register: 1394 lines in `///` or `//!` rustdoc, 169 in
  plain `//` comments, 0 trailing after code, 4 inside string literals
  (assert messages), 95 in `#` comment lines across the justfile (57),
  .cargo/mutants.toml (20), .github/workflows/ci.yml (9), Cargo.toml (5),
  .config/nextest.toml (3), .github/workflows/pages.yml (1).
- "seam": 66 lines, 47 of them in rustdoc; no gloss or definition anywhere.
- Snapshot format: `grep -lE '^[0-9a-f]{20,}$'` over every `.snap` finds a
  standalone hex line only in the two bookmark pins; none of the 22 files
  under tests/snapshots has one.
- Display-string pin check for the `VersionMismatch` message: `find tests src
  -name '*.snap' | xargs grep -l 'speaks rumors protocol'` and a grep over
  `tests/` return nothing, so rewording it moves no snapshot.
- Sweep positives spot-checked: `blake` 0 hits, `mint` (word) 0, `TODO|FIXME|
  HACK` 0; adapter/tests/malformed.rs:190's "All eight leaf-query paths" is
  pinned by `assert_eq!(checked, 8)` at :282.

No test invocation was run: none of the findings is a correctness claim.
Not seen: the ~1394 rustdoc em-dashes were counted, not read; the
"silently" and "genuine(ly)" families the sweep left open were not read
per site here either.


## Positives

- The V1 retirement's prose pass held almost completely: across the crate,
  the only surviving references to deleted identifiers are the three sites
  in finding 1 and the selection framing in finding 2 (verified by the
  deleted-identifier grep described under Method). No `V1`, `LEGACY_MAGIC`,
  `Alternating`, or `capture_*_v1` reference survives in prose.
- The BLAKE3 to SHA3 swap left zero `blake` hits; "mint" is fully purged;
  there is no TODO, FIXME, or HACK anywhere in scope (all verified).
- No src/ or tests/ prose cites `.agent-notes/`, `design/`, `formal/MODEL.md`,
  or `formal/PROGRESS.md`; the only pointers are AGENTS.md (the sanctioned
  guidepost) and the one justfile line in finding 5 (verified).
- Every Lean citation checked (`wc_impossibility`, `viewEnc`, `LocalEq`,
  `wedge`, `asmResList`, `fan`, `capLevel`, `B5`) resolves under formal/lean,
  and each rides with its invariant restated inline (verified).
- Hand-maintained counts are rare and disciplined: "All eight leaf-query
  paths" (adapter/tests/malformed.rs:190) is pinned by `assert_eq!(checked, 8)`
  at :282 (verified).
- src/reconciliation.rs:237-249 keeps the rejected level-at-a-time wire
  shape as design rationale without naming the retired protocol: a model for
  retaining a rejected alternative with no ghost reference (verified by
  reading).
- "Tripwire" is anchored at src/tree/mirror.rs:9-13 in terms consistent with
  the model of record (per the sweep; not re-read here).

## Open questions for Finch

1. A gitignored build directory sits inside the source tree at
   src/tree/mirror/streaming/target/doctest-nightly/ (verified present and
   ignored via `.gitignore:3:target/`). It is residue of running the nightly
   doctest recipe with the shell's cwd inside that module. Not tree content,
   so not a finding; delete at your discretion, and consider an absolute
   `--target-dir` in the `doctest` recipe.
2. Terms the sweep judged exempt but that sit on the brief's list: "the
   walk" (defined at src/tree/mirror/streaming.rs:4-7), "tripwire" (anchored
   at src/tree/mirror.rs:9-13), "dial" (the `Dial` trait), "silently" (81
   sites, about twenty read by the sweep with the mechanism beside the
   adverb), "genuine(ly)" (90 sites, mostly a meaningful contrast). Do you
   want the pure-intensifier subset of "genuine(ly)" swept regardless? It
   needs a per-site read neither pass did.
3. Rustdoc carries 1394 true em-dashes. The doctrine permits them in
   rendered prose "sparingly"; this pass did not treat volume as a finding,
   but the density is a taste question for a dedicated prose pass.
4. Finding 6's fix rewrites a hard rule's witness. The factual correction is
   plain; whether the renderer-vocabulary re-accept class should survive at
   all, now that the render is CBOR notation whose annotations are the
   `/ comment /` layer, is your call.
5. `Protocol` is a `#[non_exhaustive]` enum with one variant and a
   `#[default]`; finding 2 stands independently of whether you keep it.

## Dropped

- Sweep [3]'s `B5` sites (announced.rs:2, skeleton.rs:18 and :506,
  transcript.rs:11): `B5` is a named Lean axiom (formal/lean/StreamingMirror/
  Mux/Causal.lean:20 and :162), so the citation is in the permitted form.
- Sweep [3]'s "charter locality" (skeleton.rs:19): "charter" is a Lean-anchored
  term (Charters.lean; `c1_charter`, "charter-local" in Mux/Statement.lean),
  not a register transplant.
- No other finding was dropped; all twelve survive, with findings 1 and 2
  extended by one site each, finding 4 narrowed, and finding 7's line
  numbers corrected.

<!-- source: sweeps-final/suite-economics.md -->
# Sweep suite-economics: Test suite timing and economics

## Method and coverage

The sweep's run of record is one `cargo nextest run -p rumors --all-features`
at 9e5784fb (log: `scratchpad/sweeps/suite-economics/run.log`): 836 tests
across 60 binaries, 22.07 s wall, 2 skipped (`disruption::sim_child` and
`tradeoff_probe::tradeoff_closed_form_validation_run`, both `#[ignore]` by
design), no SLOW, retry, or flaky markers. Load averages were 9.65/7.33/6.99
at run start and 19.09/9.70/7.85 at run end, and the build waited on the
shared build-directory lock, so all timings are indicative.

This final pass re-derived every number it relies on from the sweep's own
artifacts rather than its prose: the 836 per-test rows in `all-times.txt`
sum to 295.142 s; the per-binary table (`per-binary.txt`) has no
`future_size` row; the slowest-test ranking matches the sweep's. Every cited
site was read with line numbers; every call site of
`early_first_child_dispute_pair` was enumerated by grep (15 calls in 10
tests, one more than the sweep counted); the justfile, ci.yml, nextest.toml
and mutants.toml were grepped for release-profile flags; git log and blame
were consulted for arb.rs, future_size.rs, nextest.toml, window_corners.rs
and capacity.rs; and `.agent-notes/` was grepped for recorded rationale
(the height-erasure, item-erasure and parent-placement notes and the
2026-07-23 review packet all bear on findings below).

One of the two permitted test invocations was used:
`cargo nextest run -p rumors --all-features -E 'binary(future_size)'`
reported `Starting 0 tests across 1 binary (59 binaries skipped)` and exited
4 with `error: no tests to run` (log:
`scratchpad/final-sweep-suite-economics/future-size-run.log`; load 4.51
before and after). The second invocation was not needed: nothing else in
dispute is settled by a run that needs no file change.

Not visible to this pass: the fixture search's current winning attempt (no
test prints it), the `[4, 256]` witness variant (needs a file change), and
the per-case runtime construction cost in `tests/disruption.rs`.


## Positives

- The suite is fast and quiet under load: 836 tests in 22 s wall on a
  machine whose load average rose from 9.6 to 19 during the run, with no
  SLOW markers, retries, or flaky results. The virtual-time discipline in
  `benches/support/latency.rs` (compute costs zero virtual time; assertions
  on the hop lattice) is what lets the window pins pass interleaved with
  everything else, exactly as `.config/nextest.toml:28-32` predicts.
- `.config/nextest.toml:14-24` is a model timeout paragraph: it names the
  one collision the budget knowingly accepts (a shrink phase outrunning
  terminate-after), states why raising the budget is the worse trade, and
  records the recovery procedure (`PROPTEST_MAX_SHRINK_ITERS`,
  `PROPTEST_MAX_SHRINK_TIME`).
- Reduced case counts carry their rationale at the declaration:
  tests/party_conservation.rs:386-392 (population scale as the sampling
  axis), src/tree/mirror/streaming/remote/proxy/tests.rs:524-529 (wide
  generator plus decorator latency, trigger geometry pinned separately),
  src/tree/mirror/streaming/tests/capacity.rs (`arb_stress_widths`:
  structured fan-out without exponential cases). This is the right shape
  for suite-economics decisions, and the finite-space proptests above are
  the sites that lack it.
- Cargo.toml:174-187 optimizes only the two kernel crates in the dev
  profile and explains why debug assertions must stay on (the envelope pins
  count work the `debug_assert!` comparisons perform): a deliberate,
  documented trade of compile time for test time. `.cargo/mutants.toml:26-37`
  states the matching campaign profile and why a release campaign would be
  a weaker observer.
- tests/common/wire.rs:26-32 reuses one current-thread runtime per test
  thread across proptest cases, with the reason stated; tests/main.rs plus
  tests/seed_liveness.rs make proptest seed persistence mechanically
  enforced rather than a convention held in memory.
- tests/disruption.rs's inter-process simulation is cheap (0.66 s for the
  generated cases plus the reconstructed counterexamples) while exercising
  real TCP, real process boundaries, and the exit-code loss protocol; the
  value-oracle tripwires (`value_oracle_tripwires_catch_known_bad_
  mechanisms`) commit the known-bad demonstrations the doctrine asks for.

## Open questions for Finch

1. future_size (suite-economics-1): a release leg for the one binary (a
   release build of rumors and dependencies per gate) or a debug-profile
   budget measured once (cheap; the order-of-magnitude argument holds under
   either profile)? Recommendation: the debug budget, keeping a release
   constant beside it only if the release number is wanted on record.
2. The fixture search (suite-economics-2 and -3): an executable
   `HINT_ATTEMPT` constant converts the prose number into a checked one and
   removes about 24 s of CPU per pass; the alternative is deleting the number
   and leaving the search as it is. Recommendation: the hint, measured after
   the SHA3 swap.
3. The capacity witness (suite-economics-7): does the `[4, 256]` pyramid
   reproduce the high-water mark and the 253/254 boundary? One measured run
   answers it and either shrinks the suite's critical path about 8x or
   produces the sentence the test is missing.
4. Binary layout (suite-economics-8): is the per-category layout meant to
   hold even for the window family's seven `latency.rs` re-inclusions and
   the five single-test binaries, or is consolidating those within the
   intent? Recommendation: consolidate the window family and the
   single-test files, leave the schedule-engine suites.

## Dropped

- Sweep finding [9] (a fresh 16-worker runtime per proptest case in
  disruption): the cost is unmeasured and plausibly small (thread spawn and
  join per case), a fresh runtime per case also isolates any task a faulted
  session leaves behind from the next case, and the construction needs a
  file change; a candidate, not a finding of record.
- Sweep finding [10] (`floor_overhead_is_bounded_by_content` inlines the
  `overhead` helper body): merged into suite-economics-5, whose
  window_census resolution removes the copy.

<!-- source: sweeps-final/verification-infra.md -->
# Sweep verification-infra: Verification infrastructure audit

## Method and coverage

This is the verification pass over the sweep's eighteen findings, at commit
9e5784fb4dce977cfbdfd1619886d1482b5ce764 (working tree clean). For each
finding I opened the cited sites with line numbers, checked the claim
against the code, and looked for a recorded rationale in git history and
`.agent-notes/`.

Read in full, with line numbers: `justfile` (1042 lines),
`.github/workflows/ci.yml` (230), `tests/future_size.rs` (91),
`tools/testdoc` (151), `tools/digestshare` (108), `.config/nextest.toml`,
`rust-toolchain.toml`, `tests/gossip_pipelining.rs` (94). Read in part:
`tools/mutantcheck` (header, lines 1-90, and 130-145), `tools/covcheck`
(195-225), `.cargo/mutants.toml` (1-60), `tools/covcheck-expected.json`
(1-12), `tools/mutantcheck-expected.json` (1-20), `design/rumors-frame-fuzz.md`
(1-30, 265-335), `src/lib.rs` (20-60), `tests/tradeoff_probe.rs` (1-35,
180-195), `tests/window_knee.rs` (1-40 plus a grep of its test roster),
`AGENTS.md` (140-175), `tests/dispute_wire.rs` (1-40), `Cargo.toml`
(94-110, 170-200), `results/mirror-complexity.tex` (1-12, 34-42, 96-106),
`tests/routed_link.rs`, `tests/changes.rs`, `tests/handshake.rs`,
`src/link/routed/tests.rs`, `src/link/routed/header/tests.rs` (110-135),
`.agent-notes/2026-08-21-cbor-wire-mutants-scopeA/README.md` (1-60 and a
grep), `.agent-notes/2026-08-21-unknown-pruning-survivor/README.md` (1-30),
one snapshot (`tests/snapshots/bootstrap_snapshot__empty_provider.snap`,
1-30).

Mechanical checks I ran (read-only): `grep` for every `nextest run` line in
the justfile and every `--release`/`--cargo-profile release` line in the
justfile's and ci.yml's whole history; `grep -c proptest` over every file
in `tests/` and over `src/conformance/**`; a Python scan of every
`proptest! {` block under `src`, `tests`, `examples`, `benches` counting
depth-1 `fn` items and whether each carries `#[test]` and a `///` line
(118 found, 0 missing either); `grep -l '│' tests/snapshots/*.snap` (0 of
22) and a grep for hexdump-shaped lines (0); `find .claude -name '*.rs' |
wc -l` (543); `git log -S`/`-L` on the `ci:` and `gate-lints:` recipe
lines, `manifestlint`, `bench = false`, `nightly_toolchain`, the
install-action tool list, `tools/digestshare`, `tests/future_size.rs`,
`design/rumors-frame-fuzz.md`, and `results/`; `gh run list`/`gh run view`
on the CI workflow for the last eight main runs and the failing job's log
at HEAD; a scratch file under my own scratchpad directory run through
`python3 tools/testdoc` to demonstrate the `proptest!` blind spot.

Not done, and therefore not claimed: I ran no `cargo`, `just`, build, or
test command (the two permitted `cargo nextest run` invocations were not
needed; every correctness-adjacent claim here settles by reading). I did
not reproduce `just --list`, so the listing fragments in
verification-infra-13 are assessed from the file's comment layout and
just's documented rule, with the sweep's reported output as unverified
corroboration. I did not run `tools/testdoc` on the foreign worktree under
`.claude/worktrees/`. I did not build `tests/future_size.rs` under the
release profile, so whether its budget holds today is unknown (open
question). The `before` meter failure that reds CI's coverage job is out
of this partition; I recorded what the logs show and stopped.


## Positives

- `tools/testdoc`'s `--self-test` (lines 81-103) pins eight lexical cases
  including the `//!`, `////`, and interleaved-attribute forms, and a
  missing root is a usage error rather than a clean sweep (lines 119-126);
  the gate leads with the self-test so a checker bug names itself.
- `gate-streams` (justfile:409-500) records an ok/failed marker per stream
  before narrating, and a stream with neither marker fails the gate with
  its partial log replayed; an OOM-killed stream cannot read as a pass.
- `tools/mutantcheck`'s header (lines 47-84) states its model of record
  (net movement per pattern, with the count-preserving-swap residual named),
  its dialect boundary, its dedup rule, its tool-version provenance, and
  its liveness floors — a checker whose limits are written where its
  claims are.
- `tools/covcheck` is tamper-evident in both directions (a new uncovered
  line fails; a stale entry fails until the pin tightens) and refuses a
  report with no branch instrumentation (lines 195-204); the justfile's
  coverage section (1014-1015) states why it is not a global threshold.
- Every `proptest!` block fn in src, tests, examples, and benches (118)
  carries an explicit `#[test]` and a `///` doc, so the convention closes
  testdoc's block-form blind spot today (verified by scan).
- `tests/seed_liveness.rs` skips `.claude` (line 35), the right call that
  testdoc has yet to make.
- `design/rumors-frame-fuzz.md` states the model framing (conformance bug
  detector, not a security boundary) in its first section and was
  re-anchored to the wire change landed today (3327a92b), so it is a
  maintained spec rather than a stale one.
- `tests/dispute_wire.rs`'s module doc (lines 1-36) states exactly what its
  pins establish and what they do not, including the truncation residual
  its negative control bounds.

## Open questions for Finch

1. Coverage job red at HEAD (verification-infra-2): `before::meter
   masked_cmp_hole_envelope` fails only under llvm-cov instrumentation
   (peak heap 1156 B against a 480 B pin; the same test passes in the `ci`
   job at the same commit), and the only change under `crates/before`
   between the last green coverage run (3327a92b) and this red is
   `Cargo.lock`. Is a process-global heap meter meaningful under
   `-C instrument-coverage`, and if not, should the coverage legs exclude
   the meter suites or the meter tolerate instrumentation? A `before`
   question, out of this partition, but it blocks a green main.
2. Does `tests/future_size.rs` pass under the release profile today? It has
   never run in any wired leg (verification-infra-1), so the answer is
   unknown; the first release run may itself be red.
3. Is `results/` in the review's scope? It holds the only derivation
   artifact for the crate doc's headline numbers (verification-infra-6), is
   dated 2026-06-12, BLAKE3-denominated, and cites two removed files
   (`results/ANALYSIS.md`, `results/mirror-complexity.md`). Re-denominate,
   move the derivation into a test, or excise: an owner call.
4. Should the coverage pin's scope extend to rumors' streaming kernel and
   bookmark format (verification-infra-4)? The instrumented run already
   covers the workspace; the cost is curation.
5. The fuzz design doc's five open questions (section 7) still await
   rulings before implementation (verification-infra-5); the 2026-07-22
   note records the deferral, not a schedule.
6. Is `just all` meant to be the full local pre-push ladder? If so it
   should absorb the coverage legs (and perhaps the instruments job's
   legs); if not, the header's "Everything" and "exactly as GitHub CI
   builds them" wording needs narrowing (verification-infra-2).
7. Mutation campaign cadence (verification-infra-3): the scope-A note's
   confirming re-run at the branch tip has no recorded outcome and scope B
   never ran; a campaign is hours on ox-east-1. Named hand-run recipe, or
   a scheduled remote run with its record in `.agent-notes/`?

## Dropped

None dropped outright. Three reframed rather than dropped: the coverage-legs
finding (the recipes exist; the gap is composite membership, severity
lowered to medium), the mutation-campaign finding (a campaign ran and was
disposed by hand; the gap is cadence, the outstanding confirming re-run,
and the absent recipe), and the hex-line witness (a ghost reference to the
retired hexdump render, not merely an unmechanized convention).
