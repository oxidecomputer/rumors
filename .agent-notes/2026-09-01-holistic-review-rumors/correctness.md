# Correctness

This document collects every finalized finding of the rumors review whose primary class is correctness: what is wrong, or could be wrong under some input, schedule, or cancellation, in the `rumors` crate at commit 9e5784fb4dce977cfbdfd1619886d1482b5ce764. Ids take the form `<partition or sweep key>-<n>`; the full record for each, with its lens history and refutation notes, lives in `evidence/partitions/<key>.md` or `evidence/sweeps/<key>.md` beside this file. Severities are the finalizers' own and are reproduced unchanged, with one exception stated at the entry (async-hazards-3, raised from medium to high after the second witness pass demonstrated the hang): *high* marks a defect in shipped behavior or a harness bug that masks failures; *medium* a breached contract clause whose consequence is bounded today, or a liveness failure reachable by conforming peers; *low* a breach reachable only through programmer error, a test-only entry, or a documentation promise the code does not keep; *nit* a tolerated corner where the doctrine still asks for the most benign behavior. Provenance is stated per entry: *verified* means the finalizer ran a command or checked the claim mechanically (grep, arithmetic, a read of pinned library sources); *assessed* means the claim rests on reading; *demonstrated* means a constructed test was run against this commit in a scratch worktree (sources restored afterwards) and its code and output are reproduced verbatim in the entry. Every primary anchor and every quoted evidence line below was re-read at the commit for this document; none needed correction. The async-hazards sweep contributes one entry (async-hazards-3); the inventory sweep filed no correctness-class finding, and its correctness-adjacent material appears under Positives (no wire-reachable panic) and Open questions (the debug-only guards).

## Highest-value items

1. The wire capture renderer collapses any container-shaped or protocol-tagged map key to a bare `…`, so two different byte streams render identically, which is exactly what its injectivity contract says cannot happen; demonstrated on two supply records differing only inside a tuple key (remote-capture-atlas-13).
2. The streaming suite's stall probes reduce completion, protocol violation, and poll-budget exhaustion to one boolean, so every "must complete" assertion passes when the session dies with a violation; demonstrated with an injected `UnexpectedQuery` under all five probe schedules (streaming-tests-11).
3. The inter-process disruption parent never joins its serving `JoinSet` and aborts the accept task, so a panic in a TCP serving session leaves the test passing; demonstrated with a panic on the fourth connection's serving path (tests-disruption-handshake-7).
4. User payload destructors run inside the `watch` write-lock critical section at both commit sites, so a `T: Drop` that touches the same replica re-enters the lock from the thread holding it, and every `snapshot()` blocks for the cascading deallocation; demonstrated through the public API alone in the second witness pass, where a `redact` whose payload destructor called `snapshot()` entered the destructor and had not returned after ten seconds (async-hazards-3).
5. The routed adapter's single `pending_headers` bound evicts a pooling dialer's already-admitted idle connections before fresh stalled headers, and the default is sized for the non-pooling case; demonstrated on the in-memory network as a `BrokenPipe` on the next stream open of a healthy link after one eviction under `pending_headers: 1`, with the TCP-pool hang variant assessed by tracing (link-28, owner-gated).
6. `resume_payload` promises `read_payload`'s exactness but offers the reader all spare capacity of a caller-supplied buffer, so a prefix with capacity beyond `len` consumes the next frame; demonstrated for both clauses of the contract (mirror-common-8).
7. The codec's over-budget lone-record path is that caller: a one-content-byte record followed by another frame makes the async reader eat the second frame while the sync oracle stops exactly; demonstrated through `decode_both` (remote-codec-14).
8. The envelope simulator's "mirror the crate" constants encode the 16-byte digest and the pre-CBOR target message size, so every byte-denominated table it prints describes a wire that does not ship (benches-envelope-28).
9. The swarm example's "roundtrips/sync" row counts direction flips on the control stream, which under the per-stream transport carries only the session envelope, so the row reports a small constant labeled as protocol work (swarm-example-16).
10. The docs.rs configuration enables a rustdoc feature the toolchain records as removed, so the build docs.rs would run fails with E0557, and no gate or CI leg compiles the `--cfg docsrs` path; demonstrated under the pinned nightly (deps-3).
11. `Z::act` stores the running join of every action's version at a key rather than the applied action's version, so the path/version coupling the tree rests on holds only by a caller-side chain property stated nowhere near the storage site, and `react`'s documented tie-break is false for a Forget followed by a causally earlier Insert (tree-core-30).
12. The walk's opening and terminal legs classify malformed replies under `Violation` variants whose documentation describes a different fault, and the responder's early supplies skip the structural checks every solicited supply gets (materialized-14).

## Crate-wide patterns

- **One exact-read defect at two altitudes.** `framing::resume_payload` (src/tree/mirror/framing.rs:100-106) reserves only when `len() == capacity()` and otherwise hands `read_buf` the whole spare capacity, so any caller whose buffer carries capacity beyond `len` over-reads the transport. mirror-common-8 states the contract breach at the callee; remote-codec-14 traces the production caller (src/tree/mirror/streaming/remote/codec/decode/async_io.rs:458-462, 474), which is safe today by an unstated one-byte margin resting on std's minimum `Vec` capacity. Both are demonstrated; one clamp in the callee resolves both.
- **Debug-only guards that degrade to wrong release behavior.** Three sites express an unreachability argument as a `debug_assert!` followed by code that misbehaves when the argument fails: src/batch.rs:136-139 (api-core-2, a silently dropped batch), src/link/routed/header.rs:248 and :254 (link-25, a truncated length byte), and examples/envelope_sim.rs:807-812 (benches-envelope-34, a shift by 64 that panics in debug and masks to zero in release). The inventory sweep names a fourth sibling no finding covers, the `debug_assert!(false, ..)` guard at src/peer/gossip.rs:1395. The doctrine's sanctioned response to programmer error is a panic carrying the proof, in every build profile.
- **Harnesses that read a failure as success.** Two high-severity entries share one shape: an outcome the harness collapses so that the intended failure is indistinguishable from completion. src/tree/mirror/streaming/tests/capacity.rs:26-36 folds `Ok(Err(violation))` and `Err(Quiescence::PollBudget)` into "did not stall" (streaming-tests-11); tests/disruption.rs:733-765 spawns serving sessions into a `JoinSet` nothing joins and aborts the accept task with its result discarded (tests-disruption-handshake-7). tests-bookmark-12 (test-quality) records the same shape in the bookmark causality harness.
- **Async-reader defects the sync-oracle differential cannot see.** `decode_both` holds the async reader to the sync decoder, but no committed test drives a failing `AsyncRead` or a listing head defect through the async path, so three divergences stand unobserved: a widened listing head yields a different `detail` string on each path (remote-codec-10, src/tree/mirror/streaming/remote/codec/decode.rs:320-323 against decode/async_io.rs:531-538); a transport error arriving after a partial opener fill is dropped and the outcome depends on what the transport does next (remote-codec-11, async_io.rs:152-161); and the lone-record over-read above (remote-codec-14). Each resolution adds the fixture that makes the differential see the case.
- **Instruments in the examples that outlived a format or transport change.** examples/envelope_sim.rs:55-71 carries hand-copied constants that expired at the digest widening and the CBOR respelling (benches-envelope-28); examples/swarm.rs:720-722 and :109-110 describe a roundtrip count the per-stream transport made a constant (swarm-example-16); examples/swarm.rs:1273-1284 recomputes two headless rows over a span that includes the warmup window the doc says is discarded (swarm-example-23). In each case the printed number describes something other than what the label says, and the crate exports the constants that would have kept the first one's values current.
- **Documentation promises stronger than the code.** src/tree.rs:297-313 says `warm_caches` forces "every" memo and forces three of four (tree-core-8); src/bookmark.rs:61-63 promises reclaim at the first dominating gossip where the code reclaims at the first unsuppressed pre-session checkpoint (session-bookmark-21); src/tree.rs:461-463 promises "the causally latest action wins" for a sequence the implementation does not honor (tree-core-30); src/tree/mirror/streaming/materialized/error.rs:25-27 documents `UnfinishedReply` as too few reactions where `absorb` reports it for too many (materialized-14). Each entry says whether the doc should be re-stated to what the code does or the code brought to the doc.

## Crate root and public surface (lib, peer, rumors, batch, snapshot, network, tags, protocol, error, tutorial)

### deps-3: the docs.rs configuration names a feature gate the toolchain records as removed, and no gate or ci leg builds under `--cfg docsrs`
- Where: src/lib.rs:297-299 (related: Cargo.toml:89-94, justfile:256-258, justfile:268-270, justfile:1000)
- Class / severity / confidence: correctness / medium / medium
- Provenance: assessed (read; `strings` over `librustc_driver-66ceb92b8eba7452.dylib` in nightly-2026-06-30 and over the 1.97.1 stable dylib; no rustdoc run was permitted)
- Demonstration: demonstrated by construction after finalization; the test code, command, output, and explanation from `witness/results.md` follow the entry verbatim.
- Verification: confirmed as far as static evidence permits; history: no-rationale-found (dfd19c447, 2026-07-24, "round 4: the style doctrine, applied" added both lines with the message fragment "the Cargo features section and docsrs auto-cfg"; Rust 1.92.0 predates that commit, so the attribute has never been accepted by a then-current nightly)
- Owner-gated: no

The crate enables `doc_auto_cfg` under `cfg(docsrs)`, and Cargo.toml's docs.rs metadata passes `--cfg docsrs`. The removed-features table embedded in both the pinned nightly and the 1.97.1 stable compiler carries an entry with reason "merged into `doc_cfg`" at version 1.92.0; `doc_auto_cfg` is the rustdoc feature RFC 3631 merged into `doc_cfg`, and the same binaries carry the successor syntax's diagnostics ("`#[doc(auto_cfg)]` is experimental"). The string pool does not place the table's name column beside its reason column, which is why the mapping is inferred rather than read, and why confidence is medium. On a post-1.92 nightly, which is what docs.rs builds with, `#![feature(doc_auto_cfg)]` is an error (feature has been removed). Locally, `docs` and `docs-internal` run stable `cargo doc` without `--cfg docsrs`, and no ci leg passes it either, so nothing in the tree ever compiles the docsrs path.

Evidence:

    src/lib.rs
       297	// docs.rs builds pass `--cfg docsrs` (see Cargo.toml's docs.rs metadata), so
       298	// every feature-gated item self-labels its gate there; inert on stable builds.
       299	#![cfg_attr(docsrs, feature(doc_auto_cfg))]

    Cargo.toml
        89	# docs.rs renders every feature-gated module the crate docs advertise
        90	# (`conformance`), matching the gate's all-features rustdoc passes. The
        91	# `docsrs` cfg turns on lib.rs's `doc_auto_cfg`, so gated items self-label.
        92	[package.metadata.docs.rs]
        93	all-features = true
        94	rustdoc-args = ["--cfg", "docsrs"]

    justfile
       258	    RUSTDOCFLAGS="-D warnings --html-in-header {{ justfile_directory() }}/crates/before/docs/fuelscape-header.html" cargo doc --workspace --all-features --no-deps

    strings, nightly-2026-06-30 librustc_driver (removed-features neighborhood, one line of the pool)
    ...replaced by `CoercePointee`1.84.0renamed to `diagnostic_on_unmatched_args`CURRENT_RUSTC_VERSIONmerged into `doc_cfg`1.92.0merged into `#![feature(rustdoc_in...

    strings, 1.97.1 librustc_driver
    ...replaced by `CoercePointee`1.84.0merged into `doc_cfg`1.92.0merged into `#![feature(rustdoc_i...

Resolution: Change lib.rs:299 to `#![cfg_attr(docsrs, feature(doc_cfg))]` (under RFC 3631, `doc_cfg` labels gated items automatically; `#[doc(auto_cfg = false)]` opts out) and reword Cargo.toml:91 and lib.rs:297-298 to name `doc_cfg`. Add a leg to `ci` (and consider `gate`; it is one nightly rustdoc of one crate): `RUSTDOCFLAGS="--cfg docsrs -D warnings" cargo +{{ nightly_toolchain }} doc -p rumors --all-features --no-deps --target-dir target/doc-docsrs`. Acceptance: a committed justfile leg runs rustdoc for rumors under the pinned nightly with `--cfg docsrs` and `-D warnings`, `just ci` includes it, and lib.rs and Cargo.toml name the gate that leg accepts.
Construction: at HEAD, `RUSTDOCFLAGS="--cfg docsrs" cargo +nightly-2026-06-30 doc -p rumors --all-features --no-deps`. Expected: `error[E0557]: feature has been removed` naming `doc_auto_cfg`, with the note "merged into `doc_cfg`". After the fix the same command succeeds and `conformance` renders with its feature label. This one command settles the medium confidence either way.

Demonstration (deps-3), reproduced verbatim from `witness/results.md`:

Outcome: demonstrated

Verdict: CONFIRMED

Test code:

```rust
// No test file: the demonstration is a rustdoc build under the pinned
// nightly the gate names (nightly-2026-06-30), with the docs.rs cfg the
// Cargo.toml metadata passes. Run at HEAD, no source edits:
//   RUSTDOCFLAGS="--cfg docsrs" cargo +nightly-2026-06-30 doc -p rumors --all-features --no-deps
// The attribute under test is src/lib.rs:299:
//   #![cfg_attr(docsrs, feature(doc_auto_cfg))]
```

Command:

```text
RUSTDOCFLAGS="--cfg docsrs" cargo +nightly-2026-06-30 doc -p rumors --all-features --no-deps
```

Output:

```text
 Documenting rumors v0.1.0 (/Users/oxide/src/rumors/.claude/worktrees/wf_866cfd1a-b96-194)
error[E0557]: feature has been removed
   --> src/lib.rs:299:29
    |
299 | #![cfg_attr(docsrs, feature(doc_auto_cfg))]
    |                             ^^^^^^^^^^^^ feature has been removed
    |
    = note: removed in 1.92.0; see <https://github.com/rust-lang/rust/pull/138907> for more information
    = note: merged into `doc_cfg`

error: Compilation failed, aborting rustdoc

error: could not document `rumors`
```

Explanation:

DEMONSTRATED. Under the pinned nightly-2026-06-30 (the toolchain the gate's nightly legs name) with `--cfg docsrs` (the exact cfg Cargo.toml's [package.metadata.docs.rs] rustdoc-args passes, and what docs.rs builds with), the crate fails to document: `error[E0557]: feature has been removed` at src/lib.rs:299, naming doc_auto_cfg, with the note 'removed in 1.92.0 ... merged into `doc_cfg`' — precisely the finding's prediction. So the docs.rs configuration is broken: the very build docs.rs runs would error out, and nothing in the gate or ci ever compiles the `--cfg docsrs` path, so the breakage is invisible to the workspace's own verification. Resolution is owner-gated in spirit (it changes how the published docs render) but is a straightforward fix — RFC 3631 stabilized doc_auto_cfg into the always-on doc_cfg, so on a post-1.92 compiler the `#![feature(doc_auto_cfg)]` gate must be dropped (and the auto-cfg labelling is then on by default under a supporting rustdoc). I did not attempt the post-fix build (would require editing lib.rs), but the E0557 note text ('merged into doc_cfg') is the compiler's own directive.

Synthesis note: the finalizer's medium confidence rested on an inferred mapping in the compiler's string pool; the constructed run below observed `error[E0557]: feature has been removed` at src/lib.rs:299 under the pinned nightly, which settles the claim. Severity and the recorded confidence are left as filed.

Cross-references: deps-7 (documentation) concerns the CI nightly pin the new `--cfg docsrs` leg would ride on.

### api-core-2: `Batch::commit`'s no-party arm degrades to a silent drop in release builds
- Where: src/batch.rs:133-139 (related: src/peer/gossip.rs:696-711, src/peer/gossip.rs:825-828, src/peer/gossip.rs:1384-1404, src/peer.rs:177-180, .cargo/mutants.toml:15-24)
- Class / severity / confidence: correctness / low / medium
- Provenance: assessed (read)
- Seen by: correctness (structure and perfapi raised it as an open question); refutation: confirmed; history: no rationale found; the mutants policy of record points the other way
- Owner-gated: no

The arm is argued unreachable, and I agree with the argument: `Inner.party` is `None` only between `inner.party.take()` in `Peer::retire` (gossip.rs:696-702) and `PartyGuard::drop`'s restore (gossip.rs:1384-1404), and `retire` consumes the `Peer` while the `Peer`/`Rumors` XOR keeps any `Batch`-creating handle from coexisting with it. But the code expresses that proof as a debug-only assert followed by `return false`: were the arm ever reached, a closure that returned `Ok` would have its whole batch discarded with no error, no observer wake, and no panic. The doctrine's sanctioned response to programmer error is a panic carrying the proof; a silent no-op is the one behavior never sanctioned, and the crate's own mutants policy (step 2 of the disposition ladder) says a structurally necessary but unreachable branch asserts the impossibility at the site. tokio's `send_if_modified` (watch.rs:1180-1194) catches the closure's panic, drops the write lock, and resumes unwinding, so an `expect` cannot poison the channel.

Evidence:

    133	            // The party is present on every reachable handle: `retire`
    134	            // consumes the `Peer`, and the `Peer`/`Rumors` XOR keeps a
    135	            // retiring set's handles from coexisting with it.
    136	            let Some(party) = inner.party.as_ref() else {
    137	                debug_assert!(false, "no party to tick in a `Batch` commit");
    138	                return false;
    139	            };

Resolution: Replace the `let ... else` with `.expect("a Batch commits through a live Peer or Rumors handle; the Peer/Rumors XOR keeps one from coexisting with a retirement's in-flight party")`. The structural alternative, making `Inner.party` total by holding the in-flight retirement party elsewhere (it would also delete the `None => inner.party = Some(party)` arm at gossip.rs:825-828), is a larger design change; see the open questions. Acceptance: no `debug_assert!(false, ..)`-plus-fallback pattern remains in `Batch::commit`; the panic message states the XOR argument; `just test` still passes (the arm is unreachable, so no test changes).
Construction: In src/tests.rs, where `Inner`'s fields are `pub(crate)`: `let peer = Peer::<u64>::seed(); peer.inner.send_modify(|i| i.party = None); peer.send(1).unwrap(); assert_eq!(peer.snapshot().len(), 0);`. At HEAD this passes in release (the send reports `Ok` and commits nothing) and panics in debug; after the fix it panics in both.

Cross-references: the open question on `Inner.party` below; the inventory sweep's open question 6 names the sibling `debug_assert!(false, ..)` guard at src/peer/gossip.rs:1395 (a leaked fork in a release build), which no finding of record covers.

### api-core-11: `Network::from_rng` loops without bound on a degenerate RNG, and its doc calls the handled case impossible
- Where: src/network.rs:57-71 (related: src/peer.rs:210-213)
- Class / severity / confidence: correctness / nit / medium
- Provenance: assessed (read)
- Seen by: correctness (the loop), prose (the wording); refutation: confirmed; history: no rationale found (byte-identical to its origin in 0f4461932)
- Owner-gated: no

The retry is unbounded. Through the hidden `Peer::seed_rng<R: RngCore + ?Sized>` a caller-supplied RNG that fills zeros (a stub, an exhausted or mis-seeded generator) hangs peer construction forever rather than failing; the 2^-128 argument holds for a uniform RNG only. A degenerate RNG is programmer error, for which the sanctioned response is a diagnosable panic, never a hang. The doc's "cryptographically impossible" argues from likelihood about a case the loop correctly handles. Gating `seed_rng` (api-core-12) shrinks the input space to the crate's own tests, which is the cheaper fix if taken.

Evidence:

    59	    /// Re-draws in the (cryptographically impossible, `2^-128`) event of the
    60	    /// all-zero value, keeping [`BOOTSTRAP`](Self::BOOTSTRAP) reserved as the
    61	    /// unambiguous bootstrap sentinel.
    62	    pub(crate) fn from_rng<R: RngCore + ?Sized>(rng: &mut R) -> Self {
    63	        loop {
    64	            let mut bytes = [0u8; 16];
    65	            rng.fill_bytes(&mut bytes);
    66	            let network = Network(bytes);
    67	            if !network.is_bootstrap() {
    68	                return network;
    69	            }
    70	        }
    71	    }

Resolution: Bound the retry to a small constant and `panic!` with a message naming the RNG as the fault, or document on `seed_rng` that a non-uniform RNG may not terminate; reword 59 to "in the negligible (2^-128) event". Acceptance: `from_rng` has no unbounded loop, or `seed_rng`'s doc states the RNG requirement; the doc no longer calls a handled case impossible.

Cross-references: api-core-12 (vestigial) and deps-5 (api-surprise) both concern gating `Peer::seed_rng`, the only path by which a caller-supplied RNG reaches `from_rng`; the open question on `seed_rng` below collects the reports' divergent recommendations.

## Session and bookmark (peer/gossip, bookmark, reconciliation, observe, message)

### session-bookmark-21: The `Bookmark` doc promises reclaim at the first dominating gossip; the code reclaims at the first unsuppressed pre-session checkpoint, and no test pins the timing
- Where: src/bookmark.rs:61-66 (related: src/peer/gossip.rs:682-694, src/bookmark.rs:285-298, src/bookmark.rs:412-417, src/peer.rs:250-253, tests/bookmark_when.rs:12-28, tests/bookmark_causality.rs:905-951)
- Class / severity / confidence: correctness / low / high
- Provenance: assessed (read the gate at gossip.rs:685, the predicate at bookmark.rs:296, the pre-session version read at gossip.rs:684, and `assert_no_leak`'s counting of checkpointed regions at tests/bookmark_causality.rs:905-914)
- Seen by: correctness; refutation: confirmed (adds a second gap: even an unsuppressed checkpoint uses the pre-session version, so the session that first achieves dominance never reclaims); history: deliberate and holds for the code (Finch's ruling, 9cf90dfb, "Only persist bookmark when there have been *local* changes", with the reclaim-delay consequence acknowledged in Finch's own `record` doc at 414-416, "rather than stranded until the next event"); the public trait sentence is from the July doc passes and is the drift
- Owner-gated: yes: choosing between re-wording the public doc to the code's rule and changing the persist gate, which relaxes a pinned schedule

The trait doc says a prior incarnation's identity is reclaimed "at the first gossip that causally dominates everything that incarnation had itself recorded". `reclaim` runs only inside the pre-session checkpoint (gossip.rs:682-694), which is skipped whenever `is_current(party, version)` holds, and `is_current` compares own-party projections only (bookmark.rs:296). A session that advances the frontier purely by learning other parties' events leaves the projection unchanged, so a gossip that newly dominates a stranded incarnation's own writes does not reclaim it; the identity waits for this peer's next local send or redact, or its next restart. Further, the checkpoint reads the pre-session version (684), so even an unsuppressed checkpoint cannot reclaim on the session that achieves dominance. Safety is unaffected (a late reclaim never recycles a coordinate), and the code's rule is deliberate; the public contract is what drifted. The verification gap: `assert_no_leak` deliberately counts checkpointed-but-unreclaimed regions as accounted for, and `tests/bookmark_when.rs` models writes, not reclaims, so nothing would fail if either the doc's rule or the code's rule were wrong.

Evidence:

    61	/// The prior incarnation's identity is then reclaimed out of the record at
    62	/// the first gossip that causally dominates everything that incarnation
    63	/// had itself recorded: its own writes, not everything it had observed.

    685	                    if !bookmark.is_current(party, &version) {

    296	            p == party && v / p == version / p

Resolution: Owner decision between two consistent states. (a) Recommended, consistent with the recorded ruling: re-word bookmark.rs:61-66 to the actual gate ("reclaimed at the first session whose pre-session checkpoint is not suppressed after the frontier dominates it: the first session after attach, or after any own event or party change following the dominating gossip"); peer.rs:250-253's "behind that path's persist gate" is already accurate. (b) Make the pre-session gate also fire when the record holds a clock with `own_version() <= version` that the live party does not cover; this relaxes the never-write-on-hearsay rule `tests/bookmark_when.rs` pins, so its model needs a `pending` rule for reclaimable clocks. Under either choice, add a test asserting the persisted record's contents after a hearsay-only dominating gossip. Acceptance: a committed test constructs the scenario below and its expectation matches the documented rule; the trait doc and the code state the same rule.

Construction: Seed S; bootstrap Q from S with a `FlakyInMemoryBookmark` (or `Probe`) store; Q sends m and gossips it to a third peer R only; drop Q's peer (crash), keeping its store. Bootstrap a new incarnation P from S (which lacks m) with the same store via `Bootstrap::bookmark`; gossip P with S (the attach-time checkpoint reclaims nothing: Q's own version is not dominated). Gossip P with R so P learns m and now dominates Q's own version. Under the doc's rule, `persisted_record(store)` no longer holds Q's clock after that session; under the current code it still does, and only a later `P.send(..)` followed by another gossip removes it. Assert whichever rule the owner selects.

Cross-references: the open question on reclaim timing below restates the two consistent states.

## Link

### link-28: The router's single count bound evicts a pooling dialer's already-admitted idle connections first, and the default is sized for the non-pooling case
- Where: src/link/routed/router.rs:159-163 (related: src/link/routed/router.rs:135-141, src/link/routed/router.rs:151, src/link/routed/router.rs:213-215, src/link/routed/router.rs:229-231, src/link/routed/endpoint.rs:42-56, src/link/routed/endpoint.rs:63-65, src/link/routed.rs:74-79, src/link/routed.rs:246-250, src/link/routed/tests.rs:498-596, tests/common/routed_tcp.rs:49-86)
- Class / severity / confidence: correctness / medium / high
- Provenance: demonstrated on the in-memory network (witness pass, after finalization: with `pending_headers: 1`, one completed stream, and one raw dial stalled mid-header, the next stream open on a healthy link failed with `BrokenPipe`); the TCP-pool hang variant and the four-peer figure under the default `Config` are assessed by tracing (a completed inbound stream's `Done` returns its connection at router.rs:229-231, the drive loop pulls it at 151, `deliver` writes `READY` at 213-215 before the header read begins, so the connection then sits in `order` beside fresh mid-header arrivals and is evicted oldest-first at 159-163). The test code, command, output, and explanation from `witness/results.md` follow the entry verbatim.
- Seen by: correctness (26, and 30c for the missing witness); refutation: confirmed step by step, severity unchanged, plus two new observations folded in below; history: already-known (4be4b830 added both the recovered-connection population and the "Size generously" paragraph; b2675dbe made `READY` precede the header read; landed via PR #13 with no owner ruling; `DEFAULT_PENDING_HEADERS`'s rationale dates from b16a800b and was never re-denominated)
- Owner-gated: yes: design change to the adapter; the design record's DECIDED 2026-07-29 entry deferred pooling to the `Dial` without pricing this interaction

The router keeps one bound (`pending_headers`) over two populations with opposite eviction preferences: fresh connections stalled mid-header (the population the bound exists to shed) and recovered idle connections a pooling `Dial` has already admitted, because `READY` is written before the header read. Eviction is oldest-first, so admitted idle connections predating a fresh stalled arrival go first, and each link can park up to `STREAM_COUNT` recovered connections per session, so `DEFAULT_PENDING_HEADERS = 64` overflows at four pooling peers in ordinary operation. An evicted admitted connection is discovered only by the next stream drawn on it: over memory the `STREAM` header write fails with `BrokenPipe` and `connect` errors on a link whose peer is healthy; over TCP a small stream's header, label, frame, and end control can all land in the socket buffer before the RST, so the dialing session sees no failure and the peer's session waits for a stream that never arrives, until the caller's timeout. `Config::pending_headers`'s own doc names that outcome and defers it to the caller; `DEFAULT_PENDING_HEADERS`'s rationale speaks only of "simultaneous dials". Two riders: the module doc at routed.rs:77-79 says a dead pooled connection "surfaces as the recycled stream's transport failure and heals at session granularity", which understates the hang its own `Config` doc names; and the `returns` channel's comment at router.rs:135-140 describes a bound ("bounded by the same budget as the pending reads it feeds") that never constrains the idle population, because the drive loop frees each slot as it pulls the connection. Correct at all scales: peer count is an input, and the crate's own rule for resource pressure is "degradation is latency, never deadlock". A documented sizing duty is a hand-maintained invariant where a structural one is available: a pool should never be able to admit a connection the router will not serve.

Evidence:

    159	        if order.len() >= pending_headers
    160	            && let Some((_, oldest)) = order.pop_front()
    161	        {
    162	            oldest.abort();
    163	        }

    213	    if recovered {
    214	        conn.write_all(&[header::READY]).await?;
    215	    }

    135	    // Connections coming back from completed streams, to await their
    136	    // next header beside fresh arrivals. The queue is bounded by the
    137	    // same budget as the pending reads it feeds, and its send never
    138	    // blocks: a return finding it full is dropped, the eviction the
    139	    // pending bound would deal it anyway. (Declared before `pending`,

    (endpoint.rs)
    52	    /// expects. Eviction of an idle recovered connection is silent: no
    53	    /// invalidation reaches the dialer's pool, and the next stream
    54	    /// drawn on the dead entry fails, or hangs to the caller's session
    55	    /// timeout. Size generously.

    63	/// Default [`Config::pending_headers`]: comfortably past a full
    64	/// session complement of simultaneous dials from several peers.
    65	const DEFAULT_PENDING_HEADERS: usize = 64;

    (routed.rs)
    77	//!   the caller's [`Dial`] — as does discovering that a pooled
    78	//!   connection died while idle, which surfaces as the recycled
    79	//!   stream's transport failure and heals at session granularity.

Resolution: separate the two populations. Give recovered connections their own bound and apply it before `READY` is written: a return that finds the recovered budget full is dropped pre-`READY` (the dialer's pool then never admits it, since the pre-`READY` drop path already exists in embryo at router.rs:229-231), and an admitted connection is thereafter closed only by the dialer or by transport failure, never by count. The smallest form is a counter of recovered connections currently in `pending`, checked before the `READY` write. Keep oldest-first count eviction for fresh mid-header connections only, whose dialer is still inside its open and observes the drop as a failed open. Make routed.rs:77-79 and endpoint.rs:52-55 agree whichever design is chosen. If the owner prefers one bound, at minimum re-denominate `DEFAULT_PENDING_HEADERS`'s rationale for the pooled idle population and land the constructed test as the documented failure's witness. Acceptance: a committed test with a pooling `Dial` in which recovered idle connections exceed `pending_headers` completes a later session with no failed or hung stream (new design), plus a negative control showing a pre-`READY` drop leaves the pool without that connection; or, under the current design, the constructed test pins the failure and the default's doc states the pooled sizing rule.
Construction: in src/link/routed/tests.rs with the `PoolingDial` fixture (498-578) and `settle()` (582-596): build endpoint `a` with `Config { pending_headers: 1, ..Config::default() }` and `b` with the default; `establish(&b, "a", ..)`; `transfer_completed(&at_b, &mut at_a, ..)` once; `settle()` so `a`'s router writes `READY` and `admit` pools the connection; dial one raw connection to `a` with `net.dial()` and write `b"ROU"` (stalls mid-header; as the newer arrival it evicts the recovered connection, the oldest, dropping the router's `DuplexStream` end); then `at_b.connector.connect()`: the pool hands out the dead connection, the `STREAM` header write returns `BrokenPipe`, and `connect` returns `Err` on a link whose peer is healthy. For the hang variant, the same shape with `PoolingTcpDial` (tests/common/routed_tcp.rs:49-86), a small mutual gossip session, and `tokio::time::timeout`.

Demonstration (link-28), reproduced verbatim from `witness/results.md`:

Outcome: demonstrated

Test code:

```rust
/// Witness (link-28): a recovered idle connection a pooling dial has
/// already admitted is evicted by a later stalled arrival under the
/// single pending-header bound, and the eviction surfaces as a failed
/// stream open on a link whose peer is healthy.
///
/// Asserts the healthy behavior (the next open succeeds and the stream
/// delivers): a failure here demonstrates the finding.
#[test]
fn witness_evicted_pooled_connection_fails_the_next_open() {
    use crate::testing::run_to_quiescence;

    let net = MemoryNet::new();
    let dial = PoolingDial::new(&net);
    let a_name = MemoryName::new("a");
    let config = Config {
        pending_headers: 1,
        ..Config::default()
    };
    let (a, mut a_incoming, a_router) =
        Endpoint::new(net.listen(&a_name), a_name, dial.clone(), config)
            .expect("a valid construction");
    let b_name = MemoryName::new("b");
    let (b, _b_incoming, b_router) =
        Endpoint::new(net.listen(&b_name), b_name, dial.clone(), Config::default())
            .expect("a valid construction");
    let outcome = run_to_quiescence(drive(routers(a_router, b_router), async {
        let (linked, arrival) =
            futures::join!(b.link(MemoryName::new("a")), a_incoming.accept());
        let at_b = linked.expect("establishment succeeds");
        let (_info, mut at_a) = arrival.expect("the router delivers the link");

        // One completed stream: b's write half is recycled toward the
        // pool, a's router takes the read half back and writes READY.
        transfer_completed(&at_b, &mut at_a, b"paid by a dial").await;
        settle().await;
        let established = dial.fresh_dials();

        // A newcomer stalls mid-header at a's router: under the count
        // bound of one it displaces the oldest pending read, which is
        // the recovered connection awaiting its next header.
        let mut stalled = net.dial().dial(&MemoryName::new("a")).await.expect("dial");
        stalled
            .write_all(b"ROU")
            .await
            .expect("a partial magic writes");
        settle().await;

        // The next open draws the pooled connection.
        let (mut tx, done) = match at_b.connector.connect().await {
            Ok(opened) => opened,
            Err(error) => panic!(
                "next stream open on a healthy link failed: kind {:?}, {error}",
                error.kind()
            ),
        };
        assert_eq!(
            dial.fresh_dials(),
            established,
            "the open drew the pooled connection"
        );
        let open = async {
            tx.write_all(b"after the stall").await.expect("payload writes");
            tx.flush().await.expect("payload flushes");
            done.complete(tx);
        };
        let read = async {
            let (mut rx, done) = at_a.acceptor.accept().await.expect("stream arrives");
            let mut received = vec![0u8; b"after the stall".len()];
            rx.read_exact(&mut received)
                .await
                .expect("the completed stream delivers its bytes");
            done.complete(rx);
            received
        };
        let ((), received) = futures::join!(open, read);
        assert_eq!(received, b"after the stall");
        drop((stalled, a, b));
    }));
    outcome.expect("the scenario runs to completion without stalling");
}
```

Command:

```text
cargo nextest run -p rumors --all-features --no-run   # then:
cargo nextest run -p rumors --all-features --no-fail-fast --success-output immediate --failure-output immediate -E '(kind(lib) & test(witness_)) | (binary(disruption) & test(/^reconstructed_child_retire_cut_at_first_byte$/))'
```

Output:

```text
        FAIL [   0.014s] (2/6) rumors link::routed::tests::witness_evicted_pooled_connection_fails_the_next_open
  stderr ───

    thread 'link::routed::tests::witness_evicted_pooled_connection_fails_the_next_open' (239157197) panicked at src/link/routed/tests.rs:795:27:
    next stream open on a healthy link failed: kind BrokenPipe, broken pipe
```

Explanation:

Appended to src/link/routed/tests.rs (the file's PoolingDial fixture, transfer_completed, settle, drive, routers reused; the file's establish() helper is monomorphic over MemoryDial, so establishment is inlined as the existing completed_streams_reuse_their_connection test does). Ran once (after one compile fix; no behavioral iteration). VERIFIED BY RUNNING: with a's Config { pending_headers: 1 }, after one completed stream and a settle, a raw dial writing b"ROU" (a stall mid-header) followed by a settle made the very next at_b.connector.connect() return Err(BrokenPipe) on a link whose peer endpoint, router, and listener were all live; the panic site (tests.rs:795:27) is the Err arm of the connect match, before the fresh_dials assertion. A BrokenPipe on the STREAM-header write is only producible on the pooled tokio duplex whose router-side end was dropped (a fresh duplex to the live listener cannot fail that way), which is the eviction of the recovered connection by the newer stalled arrival under the oldest-first count bound at router.rs:159-163, admitted into the pool by the READY byte written at router.rs:213-215 before the header read. ASSESSED BY READING, not run: the TCP hang variant (PoolingTcpDial + gossip session + timeout). Not constructed within the cap: it needs a new integration binary plus timing-dependent socket behavior (whether the small stream lands before the RST), and endpoint.rs:52-55 already concedes 'fails, or hangs to the caller's session timeout'; the memory-net run demonstrates the mechanism the finding rests on. Polarity: the test asserts the healthy behavior, so its failure is the demonstration.

Synthesis note: the constructed run demonstrates the memory-network variant (a failed stream open on a healthy link); the TCP hang variant remains assessed by reading, as the demonstration's explanation states.

Cross-references: link-14 (performance) concerns the same routed TCP shape; link-18 (documentation) the `Rejected` conflation in the same router. The open question on pooling below decides the resolution's form.

### link-25: `link_header` truncates the advertised-name length with a debug-only guard
- Where: src/link/routed/header.rs:247-256 (related: src/link/routed/endpoint.rs:204-207, src/link/routed/endpoint.rs:259)
- Class / severity / confidence: correctness / nit / high
- Provenance: assessed (read; the single caller passes the construction-validated `encoded`)
- Seen by: correctness (27); refutation: confirmed; history: no rationale (b23d19c0 moved validation to `Endpoint::new` and left the debug guard)
- Owner-gated: no

The length byte is written with `as u8` behind a `debug_assert!`; in release a caller passing more than 255 bytes would emit a truncated length and a header the peer parses as a shorter name followed by garbage. Today the only caller passes validated bytes, so the breach is programmer error, but the guard that says so is not total, and an `as` cast that can truncate is a wrong-behavior path rather than an argued unreachability.

Evidence:

    248	    debug_assert!((1..=MAX_ADDR_LEN).contains(&addr.len()));

    254	    bytes.push(addr.len() as u8);

Resolution: `bytes.push(u8::try_from(addr.len()).expect("the endpoint validates the advertised name's length at construction"));` and drop the `debug_assert!`; or carry the validated encoding as a newtype so the bound is in the type. Acceptance: no `as u8` in header.rs; the `expect` message names the construction-time check.

Cross-references: clippy-pedantic-1 (idiom) lists this cast among the range-checked `as` narrowings, and that sweep's open question 1 proposes the validated-length newtype.

### link-29: The router's read-id counter should wrap rather than overflow
- Where: src/link/routed/router.rs:165-166 (related: src/link/routed/router.rs:143-147, src/link/routed/router.rs:152-155)
- Class / severity / confidence: correctness / nit / high
- Provenance: assessed (read)
- Seen by: correctness (28); refutation: confirmed; history: no rationale
- Owner-gated: no

`next_id += 1` on a `u64` panics in debug and wraps in release after 2^64 arrivals. That is the tolerated infeasible-work corner, but even there the doctrine picks the most benign behavior; wrapping is correct because ids need only be distinct among the at most `pending_headers` live entries in `order` (saturating would collide every id at `u64::MAX` and make `retain` retire all of them).

Evidence:

    165	        let id = next_id;
    166	        next_id += 1;

Resolution: `next_id = next_id.wrapping_add(1);` with a one-line comment that at most `pending_headers` ids are live at once. Acceptance: no unchecked increment on the id counter.

## Tree core

### async-hazards-3: User payload destructors run inside the watch write-lock critical section at both commit sites
- Where: src/tree.rs:531-539 (related: src/tree.rs:602-611, src/tree.rs:507-513, src/tree/traverse/act.rs:150-159, src/tree/traverse/join.rs:64-66, src/batch.rs:132-144, src/peer/gossip.rs:816-871, src/peer.rs:678-680, src/lib.rs:238-264)
- Class / severity / confidence: correctness / high / high
- Severity: raised from medium to high in reconciliation after the second witness pass demonstrated a caller-reachable hang in production code; the sweep and the refutation pass had rated it medium.
- Provenance: demonstrated (second witness pass: through the public API alone, `redact` of a payload whose `Drop` calls `snapshot()` on a clone of the same `Rumors` handle entered the destructor and had not returned after 10 s, `snapshot()` never returning; before the pass, verified by reading `send_if_modified` in the pinned tokio 1.52.3 source, where the closure runs under `self.shared.value.write()`, `Peer::snapshot`, which takes `self.inner.borrow()`, a `read()` on the same lock, and the mid-walk drop sites in `traverse::act` and `traverse::join`)
- Verification: reframed: the pre-image drop is one of three destructor sources inside the critical section, not the only one, so deferring it shrinks the lock hold but does not lift the re-entrancy constraint; history: no-rationale-found (`.agent-notes/` mentions of "critical section" and "destructor" are in the streaming-latency design note and concern other matters)
- Owner-gated: no

`Batch::commit` and the session commit call `Tree::act`/`Tree::join` inside
`watch::Sender::send_if_modified`, which holds the channel's `RwLock` write
guard for the whole closure. Inside that closure, `T` destructors run in three
places: the mid-walk drop of a causally-prior action's message
(act.rs:150-159, `continue`), the mid-walk drops deletion honoring and the
duplicate-subtree arm perform in `traverse::join` (join.rs:64-66), and the
commit point's `drop(pre_image)`, where everything the batch or merge
displaced becomes uniquely held (tree.rs:531-539, 602-611). A `T: Drop` that
touches the same replica (`snapshot()`, `send()`, any `inner.borrow()`)
re-enters the `RwLock` from the thread holding its write guard: std's `RwLock`
either deadlocks or panics there. Separately, the lock hold includes the
cascading deallocation of everything a `redact_all` or deletion-honoring join
removed, during which every `snapshot()` and observer poll on other threads
blocks. `Tree` is lock-agnostic and says only that the drop "runs user code";
the two callers that own the lock say nothing, and "Choosing a payload type"
(lib.rs:238-264) places no constraint on `T`'s destructor.

Evidence:

    src/tree.rs
    531	        // The commit point: the walk returned without unwinding. Both fields
    532	        // are assigned before the pre-image drops, because that drop runs
    533	        // user code — everything the batch displaced becomes uniquely held
    534	        // here, so its cascading `T` destructors run now, and a panicking
    535	        // destructor must find the tree already consistent. The defense is
    536	        // nothing subtler than statement order: replace, assign, then drop.
    537	        let pre_image = std::mem::replace(&mut self.root.root, new_root);
    538	        self.root.ceiling = new_ceiling;
    539	        drop(pre_image);

    src/tree.rs (the mid-walk sources, as the code itself states them)
    509	        // our own bug. Unwind sources survive inside this walk: the leaf
    510	        // level drops causally-skipped action messages and batch-internal
    511	        // displaced inserts mid-walk, and on the wire-apply path those
    512	        // messages are freshly deserialized, so the drop is the last handle
    513	        // and runs `T`'s destructor.

    src/tree/traverse/join.rs
    65	    // is the unwind-source region of `Tree::join`'s commit section (deletion
    66	    // honoring and the duplicate-subtree drops run `T` destructors), and its

    src/batch.rs
    132	        inner.send_if_modified(|inner| {
    ...
    143	            inner.tree.act(party, actions)
    144	        });

    src/peer/gossip.rs
    816	        self.inner.send_if_modified(|inner| {
    ...
    869	            let tree_changed = inner.tree.join(merged);

    ~/.cargo/registry/src/*/tokio-1.52.3/src/sync/watch.rs
    1177	            let mut lock = self.shared.value.write();
    1178	
    1179	            // Update the value and catch possible panic inside func.
    1180	            let result = panic::catch_unwind(panic::AssertUnwindSafe(|| modify(&mut lock)));

    src/peer.rs
    678	    pub(crate) fn snapshot(&self) -> Snapshot<T> {
    679	        Snapshot::new(self.network, self.inner.borrow().tree.clone())
    680	    }

Resolution: Two steps, the first required regardless of the second. (1) State
under "Choosing a payload type" that `T`'s destructor may run inside the
replica's commit critical section on any `send`, `redact`, or gossip commit,
must not block, and must not touch a `Rumors`, `Snapshot`, or observer of the
same replica (re-entering the replica's lock deadlocks). Add a matching
maintainer comment at batch.rs:132 and gossip.rs:816 naming that user code
runs under the write guard. (2) Have `Tree::act` and `Tree::join` hand the
pre-image root back to the caller (return it alongside the changed flag) so
`Batch::commit` and `gossip_inner` drop it after `send_if_modified` returns;
`Tree` is private to the crate, so this is an internal signature change, and
the replace-assign-then-drop unwind argument is unchanged because the tree is
consistent before the drop either way. This removes the bulk deallocation
from the lock hold; the mid-walk drops remain inside it, which is why step (1)
stays. Evacuating those too (collect skipped and duplicate messages into a
sink the walk hands back) is a design proposal for the owner. Acceptance: the
payload-type section names the destructor constraint; `drop(pre_image)` (or
its equivalent) executes outside both `send_if_modified` closures, checked by
a test whose `T: Drop` records whether `inner.borrow()` succeeds during the
drop (it must, once the drop is outside the lock).
Construction (for the hazard as it stands): a payload type whose `Drop` reads
a thread-local `Rumors<T>` handle and calls `snapshot()`. Seed, `into_rumors`,
store the handle in the thread-local, `send(value)`, drop every `Snapshot`,
then `redact(&version)`: `Batch::commit` -> `send_if_modified` (write guard
held) -> `Tree::act` -> `drop(pre_image)` -> `T::drop` -> `snapshot()` ->
`inner.borrow()` -> `RwLock::read` on the thread holding `write`. The test
hangs or panics with std's re-entrant-lock diagnostic.

Demonstration (async-hazards-3), reproduced verbatim from `witness/results.md` (second pass):

Outcome: demonstrated

Test code:

```rust
// tests/zz_witness_drop_reentry.rs (new integration test binary; deleted after the run)

//! WITNESS (async-hazards-3): a payload destructor that reads its own replica
//! re-enters the watch channel's `RwLock` from the thread holding the commit's
//! write guard.
//!
//! The sequence: `send(value)`, drop every `Snapshot`, then `redact(&version)`.
//! `Batch::commit` runs `Tree::act` inside `watch::Sender::send_if_modified`
//! (write guard held); the commit point drops the pre-image, the displaced
//! payload becomes uniquely held, and its `Drop` calls `snapshot()`, which is
//! `inner.borrow()`: a read acquisition on the same lock, same thread.

use std::cell::RefCell;
use std::sync::mpsc;
use std::time::Duration;

use rumors::{Peer, Rumors};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
struct Payload(u64);

thread_local! {
    /// The replica handle the destructor reads through, installed only once
    /// the message is in the tree (so the send-time round-trip copy's drop
    /// reads nothing).
    static HANDLE: RefCell<Option<Rumors<Payload>>> = const { RefCell::new(None) };
}

impl Drop for Payload {
    fn drop(&mut self) {
        // `try_with`: tolerate TLS teardown and the not-yet-armed case.
        let _ = HANDLE.try_with(|slot| {
            if let Some(rumors) = slot.borrow().as_ref() {
                eprintln!(
                    "WITNESS async-hazards-3: Payload::drop({}) calling snapshot()",
                    self.0
                );
                let snapshot = rumors.snapshot();
                eprintln!(
                    "WITNESS async-hazards-3: snapshot() returned inside the destructor, len={}",
                    snapshot.len()
                );
            }
        });
    }
}

/// WITNESS: `redact` of a payload whose destructor calls `snapshot()` on the
/// same replica must return; a hang (or a re-entrant-lock panic) is the
/// hazard the finding describes.
#[test]
fn zz_witness_payload_drop_reenters_replica_lock() {
    let (done_tx, done_rx) = mpsc::channel::<&'static str>();
    let worker = std::thread::spawn(move || {
        let rumors = Peer::<Payload>::seed().into_rumors();
        rumors.send(Payload(7)).expect("payload encodes");
        let version = {
            let snapshot = rumors.snapshot();
            let (version, _arc) = snapshot.iter().next().expect("one message is held");
            version.clone()
        };
        HANDLE.with(|slot| *slot.borrow_mut() = Some(rumors.clone()));
        done_tx.send("armed").unwrap();
        rumors.redact(&version);
        done_tx.send("redacted").unwrap();
    });
    assert_eq!(
        done_rx.recv_timeout(Duration::from_secs(10)).ok(),
        Some("armed"),
        "worker did not arm within 10s"
    );
    match done_rx.recv_timeout(Duration::from_secs(10)) {
        Ok(message) => {
            eprintln!(
                "WITNESS async-hazards-3: worker reports `{message}`: no re-entry hazard observed"
            );
            worker.join().expect("worker exits cleanly");
        }
        Err(mpsc::RecvTimeoutError::Timeout) => panic!(
            "WITNESS async-hazards-3: redact() did not return within 10s: the destructor's \
             snapshot() blocked on the lock the commit holds"
        ),
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            let outcome = worker.join();
            panic!("WITNESS async-hazards-3: worker died before reporting: {outcome:?}");
        }
    }
}
```

Command:

```text
cargo nextest run -p rumors --all-features --no-capture --no-fail-fast -E 'test(zz_witness_deep_fixtures_over_wire) | test(zz_witness_payload_drop_reenters_replica_lock)'   (run 5; log run5_proxy_drop.log; source saved as zz_witness_drop_reentry.rs)
```

Output:

```text
WITNESS async-hazards-3: Payload::drop(7) calling snapshot()
thread 'zz_witness_payload_drop_reenters_replica_lock' (241377500) panicked at tests/zz_witness_drop_reentry.rs:78:49:
WITNESS async-hazards-3: redact() did not return within 10s: the destructor's snapshot() blocked on the lock the commit holds
test zz_witness_payload_drop_reenters_replica_lock ... FAILED

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.01s

        FAIL [  10.017s] (2/2) rumors::zz_witness_drop_reentry zz_witness_payload_drop_reenters_replica_lock

(The line `WITNESS async-hazards-3: snapshot() returned inside the destructor, ...` never appeared.)
```

Explanation:

Verified by running, through the public API only (`Peer::seed`, `into_rumors`, `send`, `snapshot`, `iter`, `redact`). The destructor was entered from inside `redact` (the `calling snapshot()` line printed), `snapshot()` never returned, and `redact` had not returned after 10 s: the read acquisition in `Peer::snapshot` (`self.inner.borrow()`, src/peer.rs:678-680) blocked behind the write guard `Batch::commit`'s `send_if_modified` holds around `Tree::act` and its `drop(pre_image)` (src/batch.rs:132-144, src/tree.rs:537-539). The send-time round-trip copy's drop did not interfere because the handle is installed only after the message is in the tree. The observed form is a hang, not a panic, consistent with a blocking (non-diagnosing) re-entrant read on the watch channel's std `RwLock` (assessed, not verified: tokio is built here without the `parking_lot` feature, and std's lock does not detect re-entrancy). The test's own 10 s timeout is what turned the hang into a failure; without it the test would hang until nextest's 180 s terminate budget.

Cross-references: the api-core partition's lock-scope open question (below) asks for a measured figure for the same critical section; async-hazards-2 (documentation) concerns the retire cancellation carve-out on the same session path.

### tree-core-30: `Z::act` stores the running join as the leaf's version rather than the applied action's, and `react`'s "causally latest action wins" holds only for causally ascending sequences
- Where: src/tree/traverse/act.rs:179-182 (related: src/tree/traverse/act.rs:145-159; src/tree.rs:283-284, 447-451, 461-465)
- Class / severity / confidence: correctness / low / high
- Provenance: assessed (traced: `greatest_version` joins every action's version at the key, line 148, before the skip; an Insert stores that join, line 181, while the path was derived from the action's own `version`, tree.rs:449; under `act` the two coincide because one party's ticks from the ceiling form a chain and an insert is the first action at its fresh path)
- Seen by: correctness (38); refutation: confirmed, and its new item 2 (the `[Forget(v2), Insert(v1)]` counterexample to the `react` contract) is folded in here; history: no-rationale-found (052d1f95b changed `Node::leaf(version, value)` to `Node::leaf(greatest_version.clone(), value)` in a WIP commit with no comment, and no later commit discusses it)
- Owner-gated: no

The invariant the tree rests on, "the set is the tree" (tree.rs:16-19) and `get`'s premise that "the hit's version is the queried one" (283-284), requires the stored version to be the one the path was derived from. The storage site does not say so: it stores `greatest_version`, the join over every action at the key including skipped ones, and the coupling holds only by a caller-side chain property stated nowhere near it. Through the private `react` (test-only today) the two decouple: a Forget then an Insert at one path carrying concurrent versions stores their join, which hashes to neither path, so `iter` yields a version `get` cannot find. The same trace refutes `react`'s doc (tree.rs:461-463): `[Forget(v2), Insert(v1)]` at one key on a fresh node with `v1 < v2` sets `node = None`, then the Insert's skip test compares `v1` against `Version::default()` and passes, so the causally earlier insert lands, stamped with the forget's version `v2`; the doc predicts the forget wins. Finished code should be obviously, reviewably correct at the site where the property lives.

Evidence:

    145            for (_, version, action) in actions {
    146                // Join by reference: `version` is still needed for the causality
    147                // comparison just below, and the join doesn't consume it.
    148                greatest_version |= &version;
    ...
    179                node = match action {
    180                    Action::Forget => None,
    181                    Action::Insert(value) => Some(Node::leaf(greatest_version.clone(), value)),
    182                };

    src/tree.rs
    461        /// If multiple actions refer to the same leaf of the tree, the causally
    462        /// latest action wins, with order of specification breaking concurrency
    463        /// and version ties. Each item is keyed by its version-derived path, so

Resolution: Store the applied action's version: `Some(Node::leaf(version.clone(), value))` (move `version` into the arm; it is not needed after the comparison), keeping `greatest_version` for the observer only; this is equivalent under `act` and makes the path/version coupling hold at the storage site. Then either qualify `react`'s doc ("for a causally ascending sequence at each key, which `act` guarantees") or track the per-key ceiling across a Forget so the stated rule holds for arbitrary sequences. Acceptance: the leaf's stored version is by construction the one its path was derived from; existing `act`/`react` suites unchanged; a committed test pins the two constructions below. Construction: (1) tree holds a leaf at synthetic path `p` with version A@1; `tree.react([(p, B@1, None::<Message>), (p, C@1, Some(msg))])` with A, B, C disjoint parties: the Forget at B@1 is concurrent with A@1, not `<`, so the leaf is removed; the Insert at C@1 lands on `None`; the stored version is `B@1 | C@1`. Assert `tree.iter().next().unwrap().0 == &C@1`, which fails at this commit. (2) fresh tree, `p = Path::for_leaf(&v1)`, `v1 < v2` on one party: `tree.react([(p, v2, None::<Message>), (p, v1, Some(msg))])`. The doc predicts an empty tree; `tree.len()` is 1 and the stored version is `v2`, so `Path::for_leaf(stored) != p`.

Cross-references: tree-core-14 (modularity) and the open question on `react`'s role below decide whether the storage-site fix or a collapse into `act`'s commit section is the repair; tree-core-34 (test-quality) is the neighbouring gap in join's algebraic laws.

### tree-core-8: `warm_caches` says it forces every memo but skips `version_bytes`, which the greeting reads
- Where: src/tree.rs:297-313 (related: src/tree/typed/untyped.rs:147-158, src/tree/mirror/streaming/backend.rs:400-413, src/tree/mirror/streaming/materialized.rs:521 and 577, src/snapshot.rs:163-169, benches/gossip_fixed.rs, benches/gossip_grid.rs)
- Class / severity / confidence: correctness / low / high
- Provenance: verified (read `untyped.rs:147-158`: a branch carries `bounds`, `version_bytes`, and the hash as `OnceLock`s, and `version_bytes` "forces" the bounds; grep shows `Root::<B>::max_version_bytes` at `backend.rs:408-411` reads `node.version_bytes()` for the greeting at `materialized.rs:521,577`; `warm_caches` forces `hash`, `ceiling`, `floor` only)
- Seen by: perfapi (50); refutation: confirmed; history: deliberate-but-expired (611b325de forced the three memos that existed; 206c288ae made `version_bytes` a lazy `OnceLock` without extending `warm_caches`)
- Owner-gated: no

The doc says the method forces "every lazily-memoized structural value"; a branch carries four (hash, the bounds span, `version_bytes`) and the body forces three. `version_bytes` is production-reachable: the streaming greeting reads it once per tree lineage, so the gossip benches that call `warm_caches` (through `Rumors::warm_caches`) before timing still pay one O(branches) fold inside their first timed session. Criterion's warm-up likely absorbs it, so the practical harm is small; the defect is a calibration helper whose contract ("every") its body does not meet. Statement faithfulness: never stronger than shown.

Evidence:

    297        /// Forces every lazily-memoized structural value — the observable hash
    298        /// and the ceiling/floor version bounds — for the whole tree.
    ...
    307        pub fn warm_caches(&self) {
    308            if let Some(root) = &self.root.root {
    309                let _ = root.hash();
    310                let _ = root.ceiling();
    311                let _ = root.floor();
    312            }
    313        }

    src/tree/typed/untyped.rs
    155            /// Like the bounds span (which it forces), this must be reset
    156            /// whenever the branch's children change, but not when its
    157            /// prefix does.
    158            version_bytes: OnceLock<usize>,

Resolution: Add `let _ = root.version_bytes();` (which forces the bounds span, so the `ceiling`/`floor` calls become redundant and can go), and word the doc as the list of memos it forces so a future memo cannot fall outside "every" unnoticed; mirror the wording at `snapshot.rs:163-165`. Acceptance: a test calls `warm_caches` then observes the memo set (below). Construction: add a `#[cfg(test)]` probe on `untyped::Node` returning whether the branch's `version_bytes` `OnceLock` is populated (`get().is_some()`); build a tree of at least two leaves, call `warm_caches()`, assert the probe is true. The assertion fails at this commit and passes after the fix.

Cross-references: async-hazards-4 (documentation) is the production face of the same memo: the first greeting materializes `version_bytes` inside one poll.

## Mirror common (including the streaming module root's test suites under src/tree/mirror/streaming/tests/)

### streaming-tests-11: Stall probes report violations and livelocks as completion
- Where: src/tree/mirror/streaming/tests/capacity.rs:26-36 (related: capacity.rs:191-194, 198-218, 238-242, 265-277, 303-329; src/testing.rs:345-394)
- Class / severity / confidence: correctness / high / high
- Provenance: assessed (read `run_to_quiescence`'s signature `Result<F::Output, Quiescence>` with `Quiescence::{Stalled, PollBudget}` at testing.rs:345-394; `F::Output` here is the session's `Result<(Root, Root), MirrorError<..>>`; the `matches!` arms admit only `Err(Quiescence::Stalled)`)
- Demonstration: demonstrated by construction after finalization; the test code, command, output, and explanation from `witness/results.md` follow the entry verbatim.
- Seen by: api-economics ([35]); refutation: confirmed and proposed lowering to medium (a test-probe weakness rather than a production defect, and the probed shapes are compared to the oracle at default capacity by other tests); history: no rationale found (the probes are the "[checked]" evidence the parent-placement note rests on; the positive `stalls` assertions are unaffected and the negative ones are the gap, so a three-way probe strengthens the recorded argument)
- Owner-gated: no

The severity stays at high because the review's rubric names "a harness bug that masks failures" as high, and this is one. `underbuffered_mirror_stalls` and `shape_stalls` collapse three outcomes (`Ok(Ok(_))` completion, `Ok(Err(violation))`, `Err(Quiescence::PollBudget)`) into `false`, so the five assertions documented as "must complete" (`!stalls_under_any_schedule(..)` at 265-269, 274-277, 308-312, 323-329 and `!underbuffered_mirror_stalls(a, b, 254)` at 191-194) pass if the session dies with a protocol violation or exhausts the million-poll budget. The completion arm also never compares the reconciled roots to `join_oracle`, and the other oracle comparisons run at the default capacity, not at capacity 1 or 254, so a regression specific to the constrained capacity is invisible. The refutation's point that no production defect is known is true and does not change the classification: the cheapest passing artifact is not the intended one.

Evidence:

        30	    with_kind_capacity(QueueKind::AssemblyLevelReturns, capacity, || {
        31	        matches!(
        32	            run_to_quiescence(drive_streaming(client, server)),
        33	            Err(Quiescence::Stalled)
        34	        )
        35	    })

       265	    assert!(
       266	        !stalls_under_any_schedule(&internal_fan(3), 1),
       267	        "fan = cap + 2 must complete: the model's tighter pdelay boundary \
       268	         is not realizable in the sequential encoder"
       269	    );

Resolution: Replace the boolean probes with a three-way outcome: `fn outcome(pair, capacity, schedules) -> Result<Result<(Root, Root), MirrorError<..>>, Quiescence>` (or the `Outcome` of the `LocalSession` builder in streaming-tests-3), with `stalls(o) = matches!(o, Err(Quiescence::Stalled))` and `completes(o)` returning the roots so the caller can compare them to `join_oracle`. Rewrite `stalls_under_any_schedule` as `any(stalls)` and add `completes_under_every_schedule` as `all(completes)` for the negative sites; treat `Ok(Err(_))` and `Err(Quiescence::PollBudget)` as failures with their own messages. Acceptance: inject `Fault::Reply(Violation::UnexpectedQuery)` into the `internal_fan(3)` session (or temporarily make the completion path return `Ok(Err(..))`) and confirm `parent_delay_single_parent_boundary` fails; restore and confirm it passes; every `!stalls` site is a `completes` site whose roots are compared to `join_oracle`.

Construction: In `parent_delay_single_parent_boundary`, wrap the server in `Faulting::new(server, 0, Some(Fault::Reply(Violation::UnexpectedQuery)))` inside `shape_stalls`; today the test still passes because `Ok(Err(..))` counts as "did not stall".

Demonstration (streaming-tests-11), reproduced verbatim from `witness/results.md`:

Outcome: demonstrated

Test code:

```rust
/// Witness (streaming-tests-11): the stall probes' boolean collapses a
/// session that dies with a protocol violation into "did not stall".
///
/// The fan = cap + 2 shape whose completion
/// `parent_delay_single_parent_boundary` asserts, with the server
/// wrapped to commit `UnexpectedQuery` in its first reply: the probe's
/// predicate answers `false` (so the existing "must complete" assertion
/// passes) while every run's actual outcome is a violation, never a
/// completion. This test passes exactly when the finding holds.
#[test]
fn witness_stall_probe_reads_a_violation_as_completion() {
    use crate::tree::mirror::streaming::{Fault, Faulting, materialized::Violation};

    let cells: Vec<Vec<u8>> = (0..3u8).map(|radix| vec![0, radix]).collect();
    let pair = divergent_cells_pair(&cells, 1, LeafOrder::Outside);
    let mut outcomes: Vec<String> = Vec::new();
    let stalls = probe_schedules()
        .into_iter()
        .any(|(channel_schedule, backend_schedule)| {
            let (a, b): (StreamingRoot<Local>, StreamingRoot<Local>) =
                (pair.0.clone().into(), pair.1.clone().into());
            let client = Handshaking::start(Local, a).window(WindowConfig::FLOOR);
            let server = Handshaking::start(Local, b).window(WindowConfig::FLOOR);
            let server =
                Faulting::new(server, 0, Some(Fault::Reply(Violation::UnexpectedQuery)));
            with_kind_capacity(QueueKind::AssemblyLevelReturns, 1, || {
                with_schedule(channel_schedule, || {
                    with_local_schedule(backend_schedule, || {
                        let outcome = run_to_quiescence(drive_streaming(client, server));
                        let stalled = matches!(outcome, Err(Quiescence::Stalled));
                        outcomes.push(match outcome {
                            Ok(Ok(_)) => "completed".to_string(),
                            Ok(Err(error)) => format!("violation: {error:?}"),
                            Err(quiescence) => format!("{quiescence:?}"),
                        });
                        stalled
                    })
                })
            })
        });
    eprintln!("witness: probe predicate stalls = {stalls}; run outcomes = {outcomes:?}");
    assert!(
        !stalls,
        "the probe predicate reads the faulting session as 'did not stall' \
         (the existing 'must complete' assertion form): {outcomes:?}"
    );
    assert!(
        outcomes
            .iter()
            .all(|outcome| outcome.starts_with("violation: ")),
        "every run actually died with a violation, none completed: {outcomes:?}"
    );
}
```

Command:

```text
cargo nextest run -p rumors --all-features --no-fail-fast --success-output immediate --failure-output immediate -E '(kind(lib) & test(/^tree::mirror::streaming::tests::capacity::witness_stall_probe_reads_a_violation_as_completion$/)) | (binary(disruption) & test(/^reconstructed_child_retire_cut_at_first_byte$/))'
```

Output:

```text
        PASS [   0.017s] (1/2) rumors tree::mirror::streaming::tests::capacity::witness_stall_probe_reads_a_violation_as_completion
  stderr ───
    witness: probe predicate stalls = false; run outcomes = ["violation: Client(Violation(UnexpectedQuery))", "violation: Client(Violation(UnexpectedQuery))", "violation: Client(Violation(UnexpectedQuery))", "violation: Client(Violation(UnexpectedQuery))", "violation: Client(Violation(UnexpectedQuery))"]
```

Explanation:

Appended to src/tree/mirror/streaming/tests/capacity.rs; it is shape_stalls/stalls_under_any_schedule from that file with the server wrapped in Faulting::new(server, 0, Some(Fault::Reply(Violation::UnexpectedQuery))) and the run_to_quiescence outcome recorded beside the collapsed boolean. Reverse polarity, as a missing-check finding: the test PASSES exactly when the finding holds. Ran twice (the first run passed silently; the second added only the eprintln so the passing run leaves visible evidence; no logic changed). VERIFIED BY RUNNING: under all five probe schedules at capacity 1 on the internal_fan(3) shape, `matches!(run_to_quiescence(..), Err(Quiescence::Stalled))` was false, the exact form the 'must complete' assertions at capacity.rs:265-269, 274-277, 308-312, 323-329 and 191-194 consume, while every run's real outcome was Ok(Err(Client(Violation(UnexpectedQuery)))): the session died with a protocol violation and the probe read it as completion. The PollBudget collapse and the absence of any join_oracle comparison in the completion arm were assessed by reading capacity.rs:26-36 and 198-218, not separately constructed.

Cross-references: streaming-tests-3 (simplification) proposes the `LocalSession` builder whose `Outcome` the resolution can adopt; streaming-tests-6 and streaming-tests-7 (verification-gap) are the neighbouring oracle-coverage gaps in the same suite.

### mirror-common-8: `resume_payload`'s exactness contract is false for admitted inputs: spare capacity beyond `len` over-reads the transport, and a prefix longer than `len` returns `Ok` over-long
- Where: src/tree/mirror/framing.rs:84-110 (related: src/tree/mirror/framing.rs:11-16, src/tree/mirror/framing.rs:74, src/tree/mirror/streaming/remote/codec/decode/async_io.rs:449-462, src/tree/mirror/streaming/remote/codec/decode/async_io.rs:473-487, src/tree/mirror/streaming/remote/codec/frame.rs:155-170, src/tree/mirror/streaming/remote/codec/frame.rs:324-329, src/tree/mirror/streaming/remote/codec/budget.rs:104-112, src/tree/mirror/framing/tests.rs:66-104)
- Class / severity / confidence: correctness / medium / high
- Provenance: assessed (read; the mechanism confirmed in the pinned library sources: tokio 1.52.3 `read_buf.rs:47-67` hands `poll_read` the whole `chunk_mut`, and bytes 1.11.1 `buf_mut.rs:1623-1636` returns `capacity - len` bytes for a `Vec<u8>`; std 1.97.1 `raw_vec/mod.rs:158-166` gives `Vec<u8>` a minimum non-zero capacity of 8; the construction below was not run)
- Demonstration: demonstrated by construction after finalization; the test code, command, output, and explanation from `witness/results.md` follow the entry verbatim.
- Seen by: prose, correctness, perfapi; refutation: confirmed (with the same library reading, against tokio 1.53.1 and bytes 1.12.1; the lockfile pins 1.52.3 and 1.11.1, whose code is identical at these lines); history: not already known (REVIEW.md:1158-1162 dismissed `reserve_exact` overshoot inside the loop, never a caller-supplied buffer's capacity; ed0f1775e states no precondition)
- Owner-gated: no

The doc at :92-94 says growth, exactness, and error behavior are `read_payload`'s, and `read_payload` (:74) promises "Never consumes a byte beyond `len`". The loop reserves only when `payload.len() == payload.capacity()` and otherwise calls `read.read_buf(&mut payload)`, which offers the reader every spare byte of capacity, not `len - payload.len()`. A caller-supplied prefix whose capacity exceeds `len` therefore consumes bytes belonging to the next frame, which the module doc (:11-16) makes the property that keeps a session boundary a stream position. The doc also says the prefix's bytes "count toward `len`" but not that they must not exceed it; a prefix longer than `len` skips the loop and returns `Ok` with an over-long buffer. The one production caller (`record_prefix` → `resume_payload`, async_io.rs:473-487 and :460) is safe by a one-byte margin it does not state: `Vec::new()` plus two head writes yields length 3 with capacity 8 (std's minimum), so over-read needs `len < 8`, while the smallest lone record is `RECORD_TAG_LEN` (2) + a one-byte length head + at least 6 bytes of content (a 3-byte version tag head, a 1-byte length head, at least one version byte since the empty version pads to one byte, at least one payload byte), so `len >= 9`; and `lone_record_spans` (frame.rs:324-329) establishes the prefix is at most `len` before the call. Correct for all inputs: a documented guarantee must hold for every input the signature admits or state its precondition; "our one caller happens to pass a full buffer" is the "we are the only writer" premise the doctrine warns about, and here it rests on std's allocation policy and constants in two other modules.

Evidence:

    84	/// Continue an exact `len`-byte payload read into `payload`, whose
    85	/// existing bytes — a prefix the caller already consumed from the same
    86	/// source — count toward `len`.
    ...
    92	/// where a read-then-splice would briefly hold the payload twice. Growth,
    93	/// exactness, and error behavior are [`read_payload`]'s (it is this
    94	/// function from an empty buffer).
    ...
    100	    while payload.len() < len {
    101	        if payload.len() == payload.capacity() {
    102	            let target = (payload.capacity() * 2).max(PAYLOAD_CHUNK_LEN).min(len);
    103	            payload.reserve_exact(target - payload.len());
    104	        }
    105	        if read.read_buf(&mut payload).await? == 0 {
    106	            return Err(std::io::ErrorKind::UnexpectedEof.into());
    107	        }
    108	    }
    109	    Ok(payload)

Resolution: Make the claim true by construction: bound each read by the bytes still owed, `use bytes::BufMut;` (already a dependency, Cargo.toml:126) and `read.read_buf(&mut (&mut payload).limit(len - payload.len())).await?` (`Limit<&mut Vec<u8>>` is `BufMut`). Keep the growth policy. State the remaining precondition (`payload.len() <= len`) in the doc and check it with an O(1) `debug_assert!` or return `InvalidData` (the sanctioned place for a runtime assert: a cheap spot check at a contract boundary). In framing/tests.rs, add a differential proptest for `resume_payload`: for `prefix_len <= len` and arbitrary prefix capacity, `resume_payload(rest, prefix, len)` equals `read_payload(prefix ++ rest, len)` byte for byte, truncation classification included, with trailing bytes left unread; the suite currently exercises `read_payload` only (:87, :115, :135). Acceptance: a committed test passes `resume_payload` a 3-byte prefix in a `Vec::with_capacity(64)`, `len = 9`, and 16 trailing transport bytes, and asserts the returned payload is exactly 9 bytes and the cursor stops at the trailing bytes; it fails at this commit and passes after the change; `chunked_read_matches_whole_read_reference` still passes.
Construction: in `src/tree/mirror/framing/tests.rs`: `let len = 9; let mut transcript = pattern(len, 5); transcript.extend_from_slice(b"next-frame-bytes"); let mut prefix = Vec::with_capacity(64); prefix.extend_from_slice(&transcript[..3]); let mut cursor: &[u8] = &transcript[3..]; let decoded = pollster::block_on(resume_payload(&mut cursor, prefix, len)).unwrap(); assert_eq!(decoded, &transcript[..len]); assert_eq!(cursor, b"next-frame-bytes");`. At this commit `payload.len()` (3) differs from its capacity (64), so no reserve runs, `chunk_mut` offers 61 bytes, and the slice reader copies all 22 remaining bytes in one `read_buf`: `decoded` has 25 bytes and `cursor` is empty. For the second clause: `resume_payload(&mut cursor, vec![0u8; 12], 9)` returns `Ok` with 12 bytes.

Demonstration (mirror-common-8), reproduced verbatim from `witness/results.md`:

Outcome: demonstrated

Verdict: CONFIRMED

Test code:

```rust
// Appended to src/tree/mirror/framing/tests.rs (uses the existing
// `pattern` helper and the `pub(crate)` `resume_payload`):
/// WITNESS mirror-common-8 (first clause): a caller-supplied prefix Vec
/// whose spare capacity exceeds `len` makes `resume_payload` read past
/// `len`, consuming the following frame's bytes. Asserts the contract
/// (exact `len` bytes, cursor left at the next frame); it FAILS if the
/// over-read finding is right.
#[test]
fn witness_mirror_common_8_over_reads_spare_capacity() {
    let len = 9;
    let mut transcript = pattern(len, 5);
    transcript.extend_from_slice(b"next-frame-bytes");
    let mut prefix = Vec::with_capacity(64);
    prefix.extend_from_slice(&transcript[..3]);
    let mut cursor: &[u8] = &transcript[3..];
    let decoded = pollster::block_on(resume_payload(&mut cursor, prefix, len)).unwrap();
    assert_eq!(decoded, &transcript[..len], "resume_payload read past len");
    assert_eq!(cursor, b"next-frame-bytes", "resume_payload consumed the next frame");
}

/// WITNESS mirror-common-8 (second clause): a prefix already longer than
/// `len` skips the loop and returns Ok holding the whole over-long
/// buffer. Asserts an exact read never returns more than `len` bytes; it
/// FAILS if the over-long finding is right.
#[test]
fn witness_mirror_common_8_over_long_prefix() {
    let mut cursor: &[u8] = b"unused";
    let decoded = pollster::block_on(resume_payload(&mut cursor, vec![0u8; 12], 9));
    let returned = decoded.as_ref().map(Vec::len).unwrap_or(0);
    assert!(
        returned <= 9,
        "resume_payload returned an over-long payload of {returned} bytes",
    );
}
```

Command:

```text
cargo nextest run -p rumors --all-features -E 'test(witness_)'
```

Output:

```text
// First clause (over-read past len):
    thread '...witness_mirror_common_8_over_reads_spare_capacity' panicked at src/tree/mirror/framing/tests.rs:154:5:
    assertion `left == right` failed: resume_payload read past len
      left: [5, 36, 67, 98, 129, 160, 191, 222, 253, 110, 101, 120, 116, 45, 102, 114, 97, 109, 101, 45, 98, 121, 116, 101, 115]
     right: [5, 36, 67, 98, 129, 160, 191, 222, 253]

// Second clause (over-long prefix returns Ok):
    thread '...witness_mirror_common_8_over_long_prefix' panicked at src/tree/mirror/framing/tests.rs:167:5:
    resume_payload returned an over-long payload of 12 bytes
```

Explanation:

DEMONSTRATED, both clauses. First clause: with a prefix Vec of capacity 64 holding 3 bytes and len=9, the loop's reserve guard (`payload.len() == payload.capacity()`, i.e. 3 == 64) is false, so no clamp-to-len reserve runs; read_buf then offers the reader all 61 spare bytes and the slice reader copies all 22 remaining transcript bytes. `decoded` is 25 bytes (the 9-byte payload `[5,36,67,98,129,160,191,222,253]` PLUS the entire next frame `next-frame-bytes` = `[110,101,120,116,45,102,114,97,109,101,45,98,121,116,101,115]`), and the cursor is left empty — the exactness contract ('Never consumes a byte beyond len', inherited from read_payload) is violated for this admitted input. Second clause: a prefix already longer than len (12 > 9) skips the `while payload.len() < len` loop entirely and returns Ok holding all 12 bytes — an over-long payload an exact read must never produce. Both are as the finding predicts. The finding's own note that the single production caller (record_prefix -> resume_payload) is safe by an unstated capacity margin (a length-3 Vec at std's minimum capacity 8, versus a smallest lone record of 9 bytes) is a code-reading observation I did not exercise; the demonstrated defect is in resume_payload's contract for the general admitted-input class, matching the finding's severity framing.

Cross-references: remote-codec-14 (this document) is the production caller's instance of the same defect; one clamp in `resume_payload` resolves both.

## Materialized

### materialized-14: The opening and terminal legs classify and check counterparty faults differently from the `Resolver`, contradicting the `Violation` docs
- Where: src/tree/mirror/streaming/materialized.rs:884-896 (related: src/tree/mirror/streaming/materialized/work/levels.rs:204-248; src/tree/mirror/streaming/materialized/error.rs:25-33; work/resolver.rs:66-109; work/tests/violations.rs:246-297; src/tree/mirror/streaming/tests/faults.rs:41-46, 66-67; src/tree/mirror/streaming/testing/faulting.rs:213-241; src/tree/mirror/streaming/materialized/tests.rs:49-86)
- Class / severity / confidence: correctness / low / high
- Provenance: assessed (read; the classification arms and the `Resolver` arms were compared by reading, and grep confirms no test drives `absorb` or `responder_level` with the malformed shapes)
- Seen by: correctness, perfapi (terminal), correctness (opening); refutation: confirmed, with corroboration that the connected suite's `0..=15` step range cannot reach the terminal reply phase on the depth-32 comb and that `Faulting` can already script all eight reply-shaped violations while `arb_connected_violation` samples two; history: no rationale found (the `absorb` arms are verbatim from 7126e489 under a comment stating the acceptance condition, not the taxonomy; the opening loop is from 55d76d5c with `UnexpectedQuery` as its catch-all). One recorded constraint: 50c8b0a3 records "a pre-existing liveness gap at the position", a violation raised in the responder's early-supply loop before the stage's one reply yields stalls a wire session instead of aborting typed, and the probing test was held back; every new opening-leg violation proposed here fires at that position.
- Owner-gated: no

Three related gaps. (a) `absorb` maps `[Match]`, `[Query(_)]`, and every reply with two or more reactions to `UnfinishedReply`, whose public doc is "The reply ended before reacting to every listed child"; a leaf request lists no children and these replies have too many reactions, not too few. The `Resolver` maps the same shapes to `UnexpectedMatch`, `UnexpectedQuery`, and `InvalidSupply`. (b) `responder_level` reports a missing opening `Query` and any non-`Supply` trailing reaction as `UnexpectedQuery` ("A positional `Query` after every held child has been answered"), which is false for a leading or trailing `Match`. (c) The responder's early supplies get the containment and ledger checks but not the structural checks every solicited supply gets (strictly ascending radices, radix not already held; resolver.rs:74-83): an out-of-order, duplicated, or locally-held early radix is charged to the ledger, exploded, and parked in `supplied` where no root-level request will claim it, so its `messages_gained` credit never lands; and the opening reads exactly one reply (levels.rs:207) without the trailing `requests.next().await.is_some()` check every descending walk performs, so a second opening reply is never `UnaskedReply`. All of this is off-model for security (an honest initiator emits none of these shapes) and in-model for the taxonomy: `error.rs:10-12` promises "exactly the semantic faults", `MaterializedViolation` is public, and the exact-variant proptest never drives either leg. On the wire path the adapter already rejects non-ascending supplied paths (adapter.rs:73; `LeafOrder`), so (c) is narrower over a wire than in process.

Evidence:

    884	        let supply = match replies.as_slice() {
    885	            [] => None,
    886	            [Reaction::Supply(radix, leaf)] if *radix == expected => {
    887	                if !contained(leaf.span().hi(), &their_version) {
    888	                    return violation(Violation::UncontainedSupply);
    889	                }
    890	                ledger.absorb(leaf.len() as u64)?;
    891	                stats.gained(1);
    892	                Some(leaf.clone())
    893	            }
    894	            [Reaction::Supply(_, _)] => return violation(Violation::InvalidSupply),
    895	            _ => return violation(Violation::UnfinishedReply),
    896	        };

    211	            let Some(Reaction::Query(theirs)) = reactions.next() else {
    212	                return violation(Violation::UnexpectedQuery)?;
    213	            };
    214	            let mut early = Vec::new();
    215	            for reaction in reactions {
    216	                let Reaction::Supply(radix, node) = reaction else {
    217	                    return violation(Violation::UnexpectedQuery)?;
    218	                };

    25	    /// The reply ended before reacting to every listed child.
    26	    #[error("reply failed to cover every listed radix")]
    27	    UnfinishedReply,

Resolution: In `absorb`, classify by the first offending reaction (`Match` -> `UnexpectedMatch`, `Query` -> `UnexpectedQuery`, a second reaction after a supply -> `UnexpectedSupply` or `InvalidSupply` by radix), and match over the owned `Vec` rather than `as_slice()` so the accepted leaf moves instead of cloning (`Some(leaf)`, removing the one `Arc` clone per requested leaf at 892). In `responder_level`, report a missing opening query as `UnfinishedReply` (or a documented new variant) and a non-`Supply` trailing reaction as `UnexpectedMatch`; check early radices strictly ascending and absent from `fan`, reporting `InvalidSupply`/`UnexpectedSupply`; add the trailing `requests.next()` check. Alternatively route both legs through `Resolver::react` so one classifier serves every height. Before landing the opening-leg changes, resolve the 50c8b0a3 liveness gap (a violation before the opening's one yield must abort typed over a wire). Then add `Completing`- and opening-leg injections to `violations.rs`, and widen `arb_connected_violation` to every variant `Faulting` can script. Acceptance: each malformed terminal and opening reply shape maps to the `Violation` whose doc describes it, pinned by committed injections that fail on the current arms; no `.clone()` in `absorb`; the four `terminal_absorb_*` tests still pass.

Construction: Reuse `absorb_scripted` in `materialized/tests.rs`, replacing the single `Reaction::Supply(0, ..)` with each of `[Match]`, `[Query(vec![])]`, `[Supply(0, leaf), Supply(0, leaf)]`, and `[Supply(1, leaf)]` against expected radix 0; on HEAD the first three report `UnfinishedReply`. For the opening, build `Work::new(Local, Window::FLOOR, Recorder::default()).responder_level(..)` with `fan = [(3, node)]` and an opening reply of `[Query([(3, h)]), Supply(3, other)]` (held radix) or `[Query([]), Supply(9, n), Supply(4, n)]` (descending): on HEAD both are accepted and the supplies sit unclaimed in `supplied`. A leading `[Match]` opening reply reports `UnexpectedQuery` on HEAD.

Cross-references: materialized-39 (test-quality) corroborates that the connected fault suite cannot reach the terminal reply phase.

## Remote codec

### remote-codec-14: The over-budget lone-record body read can consume bytes of the next frame
- Where: src/tree/mirror/streaming/remote/codec/decode/async_io.rs:458-462 (related: async_io.rs:473-486; src/tree/mirror/framing.rs:84-110; src/tree/mirror/streaming/remote/streams.rs:485-492; src/tree/mirror/streaming/remote/codec/decode/tests.rs:1001-1059)
- Class / severity / confidence: correctness / medium / high
- Provenance: assessed (read; the capacity arithmetic traced through the pinned toolchain's `alloc/raw_vec/mod.rs:158-166` (`min_non_zero_cap(1) == 8`), `bytes` 1.11.1 `BufMut for Vec<u8>::chunk_mut` at buf_mut.rs:1623-1628 (exposes the whole spare capacity), and `tokio` 1.52.3 `AsyncRead for &[u8]` at async_read.rs:102 (`min(self.len(), buf.remaining())`); not constructed or run)
- Demonstration: demonstrated by construction after finalization; the test code, command, output, and explanation from `witness/results.md` follow the entry verbatim.
- Seen by: correctness (32); refutation: confirmed, severity argued down to low on consequence; history: the review packet's "considered and dismissed" entry on `resume_payload` rests on capacity being "clamped to `len` on every growth step", which covers growth inside the loop and not the capacity the caller's `prefix` already carries, so the finding is not already dismissed
- Owner-gated: no

The over-budget supply path builds `prefix` from `Vec::new()` with two `cbor::write_head` calls (each an `extend_from_slice`), which std grows to the one-byte-element minimum capacity of 8, and hands it to `resume_payload`, whose loop reserves only when `len() == capacity()` and otherwise calls `read_buf` into the whole spare capacity. When the declared run `len` is below that capacity (a lone record of 1 to 4 content bytes: `len` 4 to 7 with a 3-byte prefix), the first `read_buf` may take up to `capacity - len` bytes belonging to the next frame. The frame is then judged with foreign trailing bytes (`LeafRun::from_encoded` sees a second, bogus record and fails `NotARecord`) and the next frame's bytes are gone. Two docs promise the opposite: `read_payload`'s "Never consumes a byte beyond `len`", which `resume_payload` inherits, and `FrameRead`'s "a valid frame is consumed exactly and no byte of the next frame is touched"; `streams.rs` hands the transport half back through `into_inner` on the strength of that exactness. A conforming encoder cannot produce such a record (its smallest is at least 8 bytes: a 2-byte record tag, a 1-byte length, the 3-byte version tag, a 1-byte version head, and at least one payload byte), and the triggering record fails at `records()` regardless, so today the breach shows only as a misclassification (`InvalidRun(NotARecord)` where the sync oracle says `Ok`) on a frame that fails anyway. I keep it at medium because the clause breached is the exactness contract the frame-boundary hand-off rests on, the callee's documentation promises it without stating a capacity precondition, and the clamp is cheap and protects every future caller.

Evidence:

    458	            // Legal lone record: resume the body read behind the heads
    459	            // already consumed, in the same single buffer.
    460	            resume_payload(&mut *self.exact.read, prefix, len)
    461	                .await
    462	                .map_err(|source| classify(FramePart::SupplyRun, source))?

    474	        let mut prefix = Vec::new();

    framing.rs:100	    while payload.len() < len {
    framing.rs:101	        if payload.len() == payload.capacity() {
    framing.rs:102	            let target = (payload.capacity() * 2).max(PAYLOAD_CHUNK_LEN).min(len);
    framing.rs:103	            payload.reserve_exact(target - payload.len());
    framing.rs:104	        }
    framing.rs:105	        if read.read_buf(&mut payload).await? == 0 {

Resolution: Clamp the read in `resume_payload` so no iteration can fill past `len`: read through `(&mut payload).limit(len - payload.len())` (`BufMut::limit` caps `chunk_mut`), or shrink so `capacity() <= len` before the loop; the callee is the right home because its documentation already promises exactness without a capacity precondition, and `record_prefix` should not have to know std's minimum capacity. Then add to `overbatched_corners_classify_exactly` (or a sibling) a case decoding, through `decode_both`, a zero-budget frame carrying `raw_record(&[0x00])` followed by a second frame's bytes, asserting the first decode is `Ok` and, through `FrameRead`, that the second frame then decodes cleanly; also feed the same bytes through a chunked reader so the resume path runs under partial delivery. Acceptance: the new test fails at HEAD (the async reader returns `InvalidRun(NotARecord { remaining: 3, .. })` while the sync oracle returns `Ok`, so `decode_both` panics on disagreement) and passes after the clamp; `supply_full_delivery_costs_at_most_payload_plus_chunk` and `overbatched_supply_rejects_without_buffering_its_body` in `tests/decode_alloc.rs` still pass.

Construction: Bytes: opener `[0x83, 0x09, 0x07]` (stream 9, `Signal::Supply(Flow::End)`), then `[0xd8, 0x3f, 0x44]` (tag 63, byte string of 4), then the run body `[0xd8, 0x3f, 0x41, 0x00]` (one record of 1 content byte), then a trailing frame `[0x82, 0x09, 0x09]` (`End::Stream` on stream 9). Budget `RunBudget::from_bytes(0)`. Async: `covers(4)` is false (envelope 10 + 4 > 0); `len` 4 is not below 3; `record_prefix` yields `prefix = [d8 3f 41]` (len 3, capacity 8) and record content 1; `lone_record_spans(4, 1)` is `2 + 1 + 1 == 4`, true; `resume_payload` sees `3 < 4` and `3 != 8`, so `read_buf` offers a 5-byte chunk and the slice reader fills it with all 4 remaining bytes `[00 82 09 09]`; the payload is 7 bytes; `from_encoded` parses record one, then reads `0x82` (major 4) where a tag belongs and returns `NotARecord { remaining: 3, .. }`. Sync: `read_exact` takes exactly 1 byte and returns `Ok` with a one-record run. `decode_both` hits its `(Err, Ok)` arm and panics; alternatively call `FrameRead::frame` twice and observe the second call return `Ok(None)` (EOF) instead of `Some((stream 9, Frame::End(End::Stream)))`.

Demonstration (remote-codec-14), reproduced verbatim from `witness/results.md`:

Outcome: demonstrated

Test code:

```rust
/// Witness (remote-codec-14): the over-budget lone-record body read
/// consumes no byte beyond the declared run.
///
/// Stream 9, `Supply(End)`, declaring a four-byte run that is exactly
/// one record of one content byte, followed by a second frame
/// (`End(Stream)` on stream 9). Under a zero budget the lone-record
/// path is taken. Asserts the exactness contract: the async reader
/// accepts the frame as the sync oracle does and rests at the second
/// frame's first byte.
#[test]
fn witness_over_budget_lone_record_read_stays_within_its_frame() {
    let stream = stream(9);
    let zero = RunBudget::from_bytes(0);
    // A lone record of one content byte: tag 63, bstr(1), 0x00.
    let lone = raw_record(&[0x00]);
    assert_eq!(lone, [0xd8, 0x3f, 0x41, 0x00]);
    let mut encoded = supply(stream, Flow::End, &lone);
    assert_eq!(&encoded[..3], [0x83, 0x09, 0x07]);
    assert_eq!(&encoded[3..6], [0xd8, 0x3f, 0x44]);
    let trailing = bare_frame(stream, Signal::End(End::Stream));
    assert_eq!(trailing, [0x82, 0x09, 0x09]);
    encoded.extend_from_slice(&trailing);

    for speaker in SPEAKERS {
        // The sync oracle: one frame, the trailing frame untouched.
        let mut rest = encoded.as_slice();
        let from_sync = decode(speaker, zero, &mut rest);
        assert_eq!(
            rest,
            trailing.as_slice(),
            "the sync oracle rests at the next frame; it decoded {from_sync:?}"
        );
        let from_sync = from_sync.expect("the sync oracle accepts the lone record");

        // The async reader over the same bytes.
        let mut reader = FrameRead::new(speaker, zero, encoded.as_slice());
        let from_async = pollster::block_on(reader.frame());
        let remaining = reader.into_inner();
        assert_eq!(
            remaining,
            trailing.as_slice(),
            "the async reader consumed bytes of the next frame; it decoded {from_async:?}",
        );
        assert_eq!(
            from_async.expect("the async reader accepts the lone record"),
            Some(from_sync),
        );
    }
}
```

Command:

```text
cargo nextest run -p rumors --all-features --no-fail-fast --success-output immediate --failure-output immediate -E '(kind(lib) & test(witness_)) | (binary(disruption) & test(/^reconstructed_child_retire_cut_at_first_byte$/))'
```

Output:

```text
        FAIL [   0.012s] (1/6) rumors tree::mirror::streaming::remote::codec::decode::tests::witness_over_budget_lone_record_read_stays_within_its_frame
  stderr ───

    thread 'tree::mirror::streaming::remote::codec::decode::tests::witness_over_budget_lone_record_read_stays_within_its_frame' (239157196) panicked at src/tree/mirror/streaming/remote/codec/decode/tests.rs:1148:9:
    assertion `left == right` failed: the async reader consumed bytes of the next frame; it decoded Err(DecodeError { origin: Stream { speaker: Initiator, stream: Stream(9) }, kind: InvalidRun(NotARecord { remaining: 3, detail: "record does not open with the embedded-sequence tag" }) })
      left: []
     right: [130, 9, 9]
```

Explanation:

Appended to src/tree/mirror/streaming/remote/codec/decode/tests.rs, reusing its raw_record/supply/bare_frame helpers; the byte-level asserts confirm the constructed wire is exactly the finding's bytes ([0x83,0x09,0x07] [0xd8,0x3f,0x44] [0xd8,0x3f,0x41,0x00] then [0x82,0x09,0x09]). Ran once, no iteration. VERIFIED BY RUNNING (Speaker::Initiator; the loop failed on its first speaker, and the code path is speaker-independent): the sync oracle's two assertions passed first (it consumed exactly the 4-byte body and rested at [0x82,0x09,0x09], returning Ok), then the async FrameRead::frame returned Err(InvalidRun(NotARecord { remaining: 3, .. })) and reader.into_inner() was empty: all three bytes of the following End(Stream) frame were consumed. This is the predicted breach of read_payload's 'Never consumes a byte beyond len' (framing.rs:74, inherited by resume_payload) and FrameRead's 'no byte of the next frame is touched' (async_io.rs:54-55). ASSESSED BY READING: the cause is record_prefix's Vec::new() growing to capacity 8 across two write_head calls and resume_payload's `len() == capacity()` reserve guard letting read_buf fill spare capacity past len (framing.rs:100-106); consequence today is bounded to misclassification because a conforming record is at least 8 bytes, as the finding states.

Cross-references: mirror-common-8 (this document) is the callee's contract; the open question on where the clamp lives below.

### remote-codec-10: The async and sync decoders spell a non-canonical listing entry head differently, and no differential test drives listing defects through the async reader
- Where: src/tree/mirror/streaming/remote/codec/decode.rs:320-323 (related: src/tree/mirror/streaming/remote/codec/decode/async_io.rs:531-538; decode.rs:198-202 and :336-360; src/tree/mirror/streaming/remote/codec/decode/tests.rs:565-629 and :844-850)
- Class / severity / confidence: correctness / low / high
- Provenance: assessed (read; both paths traced to their detail strings, and `kind_signature` read to confirm `Malformed` is compared through `{:?}`)
- Seen by: correctness (34); refutation: confirmed and reframed (kind agrees, detail differs); history: introduced today by 18527932's `partial_head`, whose message claims the sync-oracle differential "hold[s] as [it was]"; true only because no `decode_both` test constructs a listing head defect
- Owner-gated: no

Inside a listing, the async reader classifies a widened, indefinite, or reserved entry head through `partial_head` into `listing_issue(ListingIssue::Head(e))`, which emits `detail: "listing head is not canonical"`; the sync oracle reads the same head through `self.head(FramePart::QueryChildren)`, then `head_error` and `head_detail`, emitting "head not in shortest form" (or the indefinite/reserved text). Both are `Malformed { part: QueryChildren }`, so the kind agrees, but `decode_both` compares the full `Debug` and would panic on this input, which no committed test constructs: `unordered_query_is_rejected` and `empty_query_listing_is_rejected` run through `decode_exact` alone. The `FrameRead` doc promises a defect "is classified exactly as a reader fetching one head at a time would classify it"; the oracle is that reader, and the differential exists to hold the two together. An oracle that would disagree, with no test asking it, is the blind spot.

Evidence:

    320	        ListingIssue::Head(_) => DecodeErrorKind::Malformed {
    321	            part: FramePart::QueryChildren,
    322	            detail: "listing head is not canonical",
    323	        },

    async_io.rs:535	        Err(error) => Err(listing_issue(super::super::frame::ListingIssue::Head(
    decode.rs:358	        cbor::HeadError::NotShortest => "head not in shortest form",
    decode/tests.rs:578	        let error = decode_exact(speaker, RunBudget::default(), &encoded).unwrap_err();

Resolution: Make `listing_issue(ListingIssue::Head(head))` use `head_detail(head)` so the two paths agree byte for byte (remote-codec-9 subsumes this by typing the source). Then route `unordered_query_is_rejected` and `empty_query_listing_is_rejected` through `decode_both`, and add a proptest over listing-head spellings (widened key `[0x18, r]` for r < 24, widened value head `[0x59, 0x00, 0x18]`, indefinite `0x5f`, reserved `0x1c`) through `decode_both`. Acceptance: before the one-line fix, the widened-key case panics inside `decode_both` ("the two decoders classify the failure differently"); after it, every listing-defect test passes through `decode_both`, and the atlas snapshot for `query/listing-key` is unchanged (a `Shape` defect, untouched).

Construction: Query frame on stream 5: opener `[0x83, 0x05, Signal::Query(Flow::Continue).state()]` (state 4), map head `0xa1`, key `[0x18, 0x05]` (radix 5 in widened form), value head `[0x58, 0x18]`, 24 digest bytes. `decode_both(speaker, RunBudget::default(), &bytes)`: the sync path yields `Malformed { part: QueryChildren, detail: "head not in shortest form" }`, the async path `Malformed { part: QueryChildren, detail: "listing head is not canonical" }`; `kind_signature` formats both with `{:?}` and the assertion at decode/tests.rs:871-875 fails.

Cross-references: remote-codec-9 (simplification) subsumes the one-line fix by typing the defect sources; remote-codec-8 (simplification) and the open question on keeping the sync oracle bear on whether the differential this finding relies on survives.

### remote-codec-11: The opener bulk read drops a transport error that arrives after a partial fill
- Where: src/tree/mirror/streaming/remote/codec/decode/async_io.rs:152-161 (related: async_io.rs:249-267, :50-63, :293-311)
- Class / severity / confidence: correctness / low / high
- Provenance: assessed (read; the control flow traced from `fill` through `Pending`/`Exact::head`)
- Seen by: correctness (33); refutation: confirmed; history: today's 18527932; the inline comment covers only the clean-close case, and the atlas's `Read` witnesses drive only the sync oracle, so the claim was never exercised on the async path
- Owner-gated: no

`Exact::fill` returns `Arrived { filled, failure: Some(e) }` when the transport errors mid-fill, but `read_frame` consults `failure` only when `filled == 0`. With one or two opener bytes in hand and a failure recorded, the error object is discarded and `Exact::head` re-reads the transport for the missing head; the outcome then depends on what the transport does after an error: a sticky error reproduces `Read { part: Signal }` with the second error object, an EOF misclassifies as `Truncated { missing: Signal }`, and a transport that resumes delivering decodes the frame as if nothing failed. The struct doc promises classification "exactly as a reader fetching one head at a time would classify it", which would report the first error at the signal item.

Evidence:

    151	    let mut opener = [0u8; OPENER_LEN];
    152	    let arrived = exact.fill(&mut opener).await;
    153	    if arrived.filled == 0 {
    154	        return match arrived.failure {
    155	            None => Ok(None),
    156	            Some(source) => Err(direction(DecodeErrorKind::Read {
    157	                part: FramePart::FrameHead,
    158	                source,
    159	            })),
    160	        };
    161	    }

    255	                Err(source) => {
    256	                    return Arrived {
    257	                        filled,
    258	                        failure: Some(source),
    259	                    };

Resolution: Carry `arrived.failure` into the wire-order judgment: parse the bytes in hand and, at the first head that needs more bytes, return `Read { part, source: failure }` instead of re-reading (for example, give `Exact` a pending-failure slot that `fill_exact` surfaces before touching the transport). Add the async failing-reader fixture of remote-codec-15 with a non-sticky shape (`[0x82]`, then `Err(Other)`, then EOF). Acceptance: the non-sticky fixture yields `Read { part: Signal, source: Other }` in both decoders (today the async reader yields `Truncated { missing: Signal }`), and the sticky-error and full-delivery cases are unchanged.

Construction: An `AsyncRead` that serves `[0x82]` on the first poll, `Err(io::ErrorKind::Other)` on the second, and `Ok(())` with nothing filled thereafter. Drive `FrameRead::new(Speaker::Initiator, RunBudget::default(), reader).frame()`. Trace: `fill` returns `{ filled: 1, failure: Some(Other) }`; the `filled == 0` gate is not taken; the frame head parses from the byte in hand (arity 2); the stream `Pending` takes an empty `rest`, so `Exact::head` calls `fill_exact(1 byte)`, which reads EOF and returns `short(Signal)` with `failure: None`, so `Truncated { missing: Signal }`. Expected by the one-head-at-a-time contract and by the sync oracle over a `FailAfterReader::new(bytes, 1)`: `Read { part: Signal }`.

Cross-references: remote-codec-15 (verification-gap) supplies the async failing-reader fixture the resolution asks for.

## Remote capture and codec tests

### remote-capture-atlas-13: Container and named-tag map keys render as a bare `…`, breaking the renderer's injectivity contract
- Where: src/tree/mirror/streaming/remote/codec/capture.rs:470 (related: src/tree/mirror/streaming/remote/codec/capture.rs:523, src/tree/mirror/streaming/remote/codec/capture.rs:20-22, src/tree/mirror/streaming/remote/codec/capture.rs:46-47, src/tree/mirror/streaming/remote/codec/capture.rs:705-716, src/tree/mirror/streaming/remote/codec/capture/tests.rs:7-8)
- Class / severity / confidence: correctness / high / high
- Provenance: assessed (traced by reading: `scalar` returns `None` for `Node::Array(_) | Node::Map(_)` at line 705 and for the six protocol-named tags at 708-716; both call sites substitute `"…"`; the frame path reaches the generic map arm for a supply record's payload per the trace in finding 10. Verified mechanically that no committed snapshot contains `…`: `grep -rl '…'` over tests/snapshots, both codec snapshot directories, and src/bookmark returns nothing, so the fix moves no pin)
- Demonstration: demonstrated by construction after finalization; the test code, command, output, and explanation from `witness/results.md` follow the entry verbatim.
- Seen by: structure, prose, correctness; refutation: confirmed (all three lenses traced independently; the refutation pass adds that a single container-keyed listing entry renders `… =>` with no annotation at all, since the order check is pairwise); history: no rationale found (the elision was written in ef6569c4 with no comment; REVIEW.md item 12 recorded "Injectivity: assessed sound, modulo B1. No action beyond B1" from hand construction, so this overturns a recorded assessment rather than reopening a ruling)
- Owner-gated: no

When a map key is an array, a map, a protocol-named tag, or a generic tag over a container, `scalar(key)` is `None` and the entry renders its key as `…`, discarding the key's content; the same elision sits in `render_listing` at line 523. Two wire byte strings that differ only inside such a key render identically, which is exactly what the module doc says cannot happen, on the input class (application payload) the doc promises "only ever fall[s] back explicitly". Application payloads are arbitrary serde CBOR, so any type with tuple or struct keys reaches this path through the public `send` API. Principle: correct for all inputs, and the cheapest artifact that passes the snapshot must be the intended bytes: a payload-key change in a fixture would pass the snapshot unseen. The renderer is test-only infrastructure, and no committed fixture carries a container key today, so the hole is latent; the contract clause and the masking potential are what set the severity.

Evidence:

    470	                let key = scalar(key).unwrap_or_else(|| "…".into());

    523	            other => scalar(other).unwrap_or_else(|| "…".into()),

    20	//! the determinism contract a complete value tree has exactly one
    21	//! encoding, so the rendering is injective on wire bytes: two different
    22	//! byte streams cannot render identically. Wherever the walk cannot

    46	//! harness, not the peer, is broken. Application payload bytes are the
    47	//! application's own CBOR and only ever fall back explicitly.

    705	        Node::Array(_) | Node::Map(_) => return None,

Resolution: Never elide. When `scalar(key)` is `None`, render the entry in block form: a `key =>` line preceded by `render_node(key, Naming::Plain, deeper, depth + 1, out)` (or a `/ key /`-annotated block), then the value block or inline value. Apply the same at line 523 (a listing key that is not a uint is already off-grammar; render it fully and let a key-shape verdict carry the diagnosis, see finding 14). `Node` does not retain its byte span, so block-form keys are the local fix; routing container-keyed maps through `fallback` with exact bytes is the alternative if span retention is added. Then tighten the module doc's fallback list to match. Acceptance: the construction below fails before the fix and passes after; `grep -n '"…"' capture.rs` is empty; the existing snapshots are byte-identical (`just test-all`).

Construction: In capture/tests.rs, using the existing `run_lines` helper, build two `LeafRun`s each holding one record: `run.push(&Version::new(), &Message::new(BTreeMap::from([((1_u8, 2_u8), 7_u8)])))` and the same with `((3_u8, 4_u8), 7_u8)`. `Message::new` serializes through `ciborium::ser::into_writer` (message.rs:285-288), and serde serializes a tuple key as a two-element CBOR array, so each payload is a map with an array key; every integer is a one-byte head, so both encodings have equal length and every rendered header byte count matches. Assert `run_lines(&a) != run_lines(&b)`. Today both render the entry as `… => 7` and the assertion fails. A second case for the named-tag branch (lines 708-716): keys `Tag(VERSION_TAG, Bytes([0xe0]))` versus `Tag(VERSION_TAG, Bytes([0x40]))` built through `cbor::write_head` and rendered via `render_item`, which also collapse to `… => ...`.

Demonstration (remote-capture-atlas-13), reproduced verbatim from `witness/results.md`:

Outcome: demonstrated

Verdict: CONFIRMED

Test code:

```rust
// Appended to src/tree/mirror/streaming/remote/codec/capture/tests.rs
// (uses the existing `run_lines` helper, `LeafRun`, `Version`, `Message`):
/// WITNESS remote-capture-atlas-13: two supply records whose only
/// difference is the content of an array-shaped map key render to
/// identical line sets, because the renderer collapses any container
/// key to a bare `…`. Asserts the injectivity contract (distinct wire
/// bytes render distinguishably); it FAILS if the collapse finding is
/// right.
#[test]
fn witness_capture_atlas_13_container_key_collapses() {
    use std::collections::BTreeMap;
    let mk = |key: (u8, u8)| {
        let mut run = LeafRun::new();
        run.push(&Version::new(), &Message::new(BTreeMap::from([(key, 7u8)])))
            .expect("one small record fits any run");
        run_lines(&run)
    };
    let a = mk((1, 2));
    let b = mk((3, 4));
    assert_ne!(a, b, "container-key records must render distinguishably");
}
```

Command:

```text
cargo nextest run -p rumors --all-features -E 'test(witness_)'
```

Output:

```text
    thread '...witness_capture_atlas_13_container_key_collapses' panicked at src/tree/mirror/streaming/remote/codec/capture/tests.rs:373:5:
    assertion `left != right` failed: container-key records must render distinguishably
      left: ["63(<< / embedded sequence, 1 item(s), 13 bytes /", "  63(<< / embedded sequence, 2 item(s), 10 bytes /", "    53846(h'e0') / causal version, event tree: 0 /", "    {", "      … => 7", "    }", "  >>)", ">>)"]
     right: ["63(<< / embedded sequence, 1 item(s), 13 bytes /", "  63(<< / embedded sequence, 2 item(s), 10 bytes /", "    53846(h'e0') / causal version, event tree: 0 /", "    {", "      … => 7", "    }", "  >>)", ">>)"]
```

Explanation:

DEMONSTRATED. Two supply records carry distinct wire bytes — payloads `A1 82 01 02 07` (map {[1,2]:7}) versus `A1 82 03 04 07` (map {[3,4]:7}), differing in the array key's element bytes — yet render to byte-identical line vectors. The map-key render path (`scalar(key).unwrap_or_else(|| "…".into())`, capture.rs:470) returns None for a Node::Array key (scalar() returns None for Array/Map, capture.rs:705) and emits `… => 7` for both, discarding the key's content. Both encodings are equal length (every integer a one-byte head), so even the `{} bytes` annotations match (both '2 items, 10 bytes'). This violates the module doc's injectivity contract ('two different byte streams cannot render identically', lines 20-22) on the application-payload input class the doc says only ever falls back explicitly (lines 46-47). The path is reachable through any serde payload with tuple/struct keys via the public send API (Message::new serializes tuple keys as CBOR arrays). Matching the finding: the renderer is test-only and no committed fixture carries a container key, so the hole is latent — the demonstration required constructing such a fixture.

Cross-references: remote-capture-atlas-17 (verification-gap) is the generative injectivity test whose constructed run found this hole after five cases; remote-capture-atlas-14 (simplification) covers the listing-key verdict the resolution defers to.

## Integration tests (tests/common, then the suites)

### tests-disruption-handshake-7: Serving-task panics are swallowed in the inter-process parent
- Where: tests/disruption.rs:733-765 (related: tests/disruption.rs:819-828; tests/common/sim.rs:466-478)
- Class / severity / confidence: correctness / high / high
- Provenance: assessed (read; the construction below was not run because no file may be modified)
- Demonstration: demonstrated by construction after finalization; the test code, command, output, and explanation from `witness/results.md` follow the entry verbatim.
- Seen by: api-economics [36]; refutation: confirmed, with a recommendation to lower to medium because the masked class is confined to TCP serving in one eight-case proptest and four reconstructed plans; history: no rationale found (the abort-and-discard wind-down is original to 9eadfc68)
- Owner-gated: no

The accept task spawns each serving session into a JoinSet that nothing ever `join_next`s; at wind-down the accept task is aborted and its result discarded. A panic inside `handle.gossip` on the serving path therefore unwinds the spawned task, prints to captured stderr, and the test passes: `serve_errors` is not incremented (the increment sits on the `Err` arm, which a panic never reaches), the handle clone drops during unwind so `try_into_peer` succeeds, and a mid-gossip panic leaves the replica unchanged so no post-run invariant can see it. The intra-process engine propagates panics by `.expect`ing each session task's JoinHandle (sim.rs:476-477); the inter-process engine, whose unique coverage is exactly the TCP serving path, does not. I keep the rubric's classification (a harness bug that masks failures) at high; the refutation's narrowing argument is stated so the owner can calibrate.

Evidence:

       733	        tokio::spawn(async move {
       734	            let mut sessions = tokio::task::JoinSet::new();
       ...
       744	                sessions.spawn(async move {
       ...
       754	                    if let Err(e) = handle.gossip(&mut link).await {
       755	                        serve_errors.fetch_add(1, Ordering::Relaxed);

       819	    // Wind down: stop the prober and the accept loop (dropping its
       820	    // `JoinSet` aborts any straggling serve task), then reclaim the
       ...
       827	    accept.abort();
       828	    let _ = accept.await;

    tests/common/sim.rs
       476	    assert_honest_gossip(&task_a.await.expect("session task A"));
       477	    assert_honest_gossip(&task_b.await.expect("session task B"));

Resolution: Hold the JoinSet outside the accept task (or have the task return it), and after the children are reaped and before reclaiming the Peers drain it: `while let Some(result) = sessions.join_next().await { result.expect("serving session task"); }`. Every serving task has completed by then (every child has exited), so the drain is immediate. End the accept loop by closing the listener rather than by abort, or check `JoinError::is_panic` on the aborted task's result. Acceptance: with `panic!("probe")` inserted as the first statement of the serving task body in a scratch copy, inter_process_disruption_upholds_party_invariants and each reconstructed_* test fail; today they pass.
Construction: In a scratch worktree, add `panic!("probe");` at the top of the closure passed to `sessions.spawn` at line 744; run `cargo nextest run -E 'binary(disruption)' -E 'test(reconstructed_child_retire_cut_at_first_byte)'`; observe PASS with the panic message only in captured stderr.

Demonstration (tests-disruption-handshake-7), reproduced verbatim from `witness/results.md`:

Outcome: demonstrated

Test code:

```rust
// VARIANT A, the finding's literal construction (tests/disruption.rs, the
// closure passed to sessions.spawn in run_proc_plan):
                sessions.spawn(async move {
                    // WITNESS PROBE (tests-disruption-handshake-7), literal
                    // construction: a panic as the serving task's first statement.
                    panic!("witness probe: serving-task panic");
                    #[allow(unreachable_code)]
                    // A child that dies before the listener-port swap is the
                    // same honest severance as one that dies mid-session.
                    let mut link = match tcp::link(socket).await {

// VARIANT B, conditional construction (same file; `ordinal` captures the
// accept-order index so the panic lands on the serving path of the fourth
// connection, after the port swap):
                let handle = casts[next % casts.len()].clone();
                let ordinal = next;
                next += 1;
                let serve_errors = Arc::clone(&serve_errors);
                let dishonest = Arc::clone(&dishonest);
                sessions.spawn(async move {
                    // A child that dies before the listener-port swap is the
                    // same honest severance as one that dies mid-session.
                    let mut link = match tcp::link(socket).await {
                        Ok(link) => link,
                        Err(_) => {
                            serve_errors.fetch_add(1, Ordering::Relaxed);
                            return;
                        }
                    };
                    // WITNESS PROBE (tests-disruption-handshake-7), conditional
                    // construction: a panic on the serving path after the
                    // port swap, on the fourth connection: the child's faulted
                    // retire (write cut at byte 0), whose failure the child
                    // attributes to its own injected cut.
                    if ordinal == 3 {
                        panic!("witness probe: serving-task panic on connection {ordinal}");
                    }
                    if let Err(e) = handle.gossip(&mut link).await {
```

Command:

```text
# Variant A:
cargo nextest run -p rumors --all-features --no-fail-fast --success-output immediate --failure-output immediate -E '(kind(lib) & test(witness_)) | (binary(disruption) & test(/^reconstructed_child_retire_cut_at_first_byte$/))'
# Variant B:
cargo nextest run -p rumors --all-features --no-fail-fast --success-output immediate --failure-output immediate -E '(kind(lib) & test(/^tree::mirror::streaming::tests::capacity::witness_stall_probe_reads_a_violation_as_completion$/)) | (binary(disruption) & test(/^reconstructed_child_retire_cut_at_first_byte$/))'
```

Output:

```text
# Variant A (literal):
        FAIL [   0.034s] (4/6) rumors::disruption reconstructed_child_retire_cut_at_first_byte
  stderr ───

    thread 'tokio-rt-worker' (239157183) panicked at tests/disruption.rs:747:21:
    witness probe: serving-task panic
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

    thread 'reconstructed_child_retire_cut_at_first_byte' (239157178) panicked at tests/disruption.rs:815:22:
    child 0 exited abnormally (Some(101)): an invariant violation or panic in the child process

# Variant B (conditional):
        PASS [   0.035s] (2/2) rumors::disruption reconstructed_child_retire_cut_at_first_byte
  stderr ───

    thread 'tokio-rt-worker' (239179846) panicked at tests/disruption.rs:761:25:
    witness probe: serving-task panic on connection 3
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

Explanation:

Two runs, one per variant. VERIFIED BY RUNNING, variant A (the finding's literal construction: panic as the serving task's first statement, before tcp::link): the test FAILS, so the construction's predicted 'observe PASS' is refuted as written; but the failure is 'child 0 exited abnormally (Some(101))' from the parent's exit-code check at disruption.rs:815, i.e. the child process noticed its bootstrap connection die at its own `tcp::link(socket).await.expect("swap listener ports")` and panicked; the parent itself observed nothing of the serving-task panic. VERIFIED BY RUNNING, variant B (panic on the serving path after the port swap, on accept-order connection 3, which for this plan (boot clean, one clean session, one clean final gossip, then retire with write cut at byte 0 and a clean retry) is the child's faulted retire, whose failure the child attributes to its own injected cut): the test PASSES with 'witness probe: serving-task panic on connection 3' present only in captured stderr. That is the finding's mechanism: the JoinSet is never join_next'ed and the accept task is aborted with its result discarded (disruption.rs:733-765), so a panic in a serving session is invisible to the test. INFERRED, not observed: in the unmodified test connection 3's serving session ends in an honest I/O error and increments serve_errors (so possible_losses = 1); with the panic in its place serve_errors stays 0 and the stricter assert_party_invariants(.., 0) still passed because the child retired cleanly on its retry, so the panic also silently altered the loss accounting. Note for the durable record: an unconditional first-statement panic is not a faithful probe because the child's own expectations on clean connections surface it via exit codes; a panic mid-serving on a connection the child expects to fail is.

Synthesis note: the entry's construction predicts a passing run for a panic placed as the serving task's first statement; the constructed run shows that probe fails for an unrelated reason (the child process notices its bootstrap connection die and exits abnormally), while a panic on the serving path after the port swap passes with the message only in captured stderr. The finding stands on the second variant; the construction sentence should be read with that correction.

Cross-references: tests-bookmark-12 (test-quality, high) is the same class of swallowed session outcome in the bookmark causality harness; tests-disruption-handshake-4 (documentation) concerns the same inter-process engine.

## Benches and examples

### benches-envelope-28: The "mirror the crate" constants are hand-copied literals that have rotted twice
- Where: examples/envelope_sim.rs:55-71 (related: examples/envelope_sim.rs:66, examples/envelope_sim.rs:105, examples/envelope_sim.rs:139-141, examples/envelope_sim.rs:164, examples/envelope_sim.rs:184-196, examples/envelope_sim.rs:1174, src/tree/typed/hash.rs:12, src/tree/typed/hash.rs:79, src/tree/mirror/streaming/remote/codec/budget/tests.rs:9, src/tree/mirror/streaming/window.rs:132, src/tree/mirror/streaming/window.rs:137, src/tree/mirror/streaming/window.rs:275, src/lib.rs:340-348, src/link.rs:169)
- Class / severity / confidence: correctness / medium / high
- Provenance: verified (hash.rs:12 `pub const MERKLE_HASH_LEN: usize = 24;` and hash.rs:79 `CHILD_RECORD_LEN = 1 + MERKLE_HASH_LEN`; budget/tests.rs:9 `assert_eq!(DEFAULT_TARGET_MESSAGE_SIZE, 1_830_400)`; window.rs:275 `DEFAULT_SYNC_MEMORY_BUDGET: usize = 512 * 1024 * 1024`; `1_114_624 = 256 × (256 × 17 + 2)` by arithmetic; `git log -1 --date=short 2d1e6ea51` is 2026-08-18 and `git show --stat 2d1e6ea51 -- benches examples` is empty; the example was last touched by dfd19c447 on 2026-07-24; the Python source at envelope-sim.py:70 reads `Hash = [u8; 16]`; lib.rs:340-348 re-exports `DEFAULT_SYNC_MEMORY_BUDGET`, `DEFAULT_TARGET_MESSAGE_SIZE`, `MERKLE_HASH_LEN`; link.rs:169 `pub const STREAM_COUNT`; window.rs:132 `FAN` is `pub(crate)` and window.rs:137 `KEY_DEPTH` is private, and `src/testing.rs` exposes neither)
- Seen by: structure, prose, correctness, perfapi; refutation: confirmed; history: deliberate-but-expired (correct mirrors at 3824c7754 on 2026-07-23; expired at 2d1e6ea51, which lists `DEFAULT_TARGET_MESSAGE_SIZE 1114624 -> 1638912` among its moved readings but names no example, and again at 4dd2053c9 for the CBOR wire)
- Owner-gated: no

The block says `LISTING_ENTRY_BYTES` mirrors the crate's 17-byte `(u8, Hash)` entry and `DECODE_SLACK_BYTES` mirrors `DEFAULT_TARGET_MESSAGE_SIZE`. Neither holds: the entry is 25 bytes since the digest widened to 24, and the target message size is pinned at 1,830,400; `1_114_624` factors as the 16-byte-entry, pre-CBOR full-fan frame. Every byte-denominated output (the `K_flat`/`K_sharp`/`K_int` tables, envelope-at-K, `L(N)`, heavy counts, the sensitivity section) is computed for a wire that does not ship, under a comment saying otherwise. Principle 5 and the no-hand-maintained-counts rule: four of the mirrored values are public re-exports the example could import, as `branch_hash.rs:29` already does for `MERKLE_HASH_LEN`. The count-level dominance sweep is constant-free and unaffected. Note the coupling: `check_landed_replication` (184-196) pins `k_flat`, which reads `LISTING_ENTRY_BYTES` through `PARKED_REPLY_SKELETON_BYTES` (66, 139-141) and `DECODE_SLACK_BYTES` through `STEADY` (105, 164), so importing the constants moves that pin (see benches-envelope-29).

Evidence:

    55	// ---------------------------------------------------------------------
    56	// Model constants. FAN/DEPTH/LISTING_ENTRY_BYTES/STAGES mirror the
    57	// crate (radix-256 tries over 32-byte content addresses, 17-byte
    58	// `(u8, Hash)` listing entries, `Stream::COUNT` = 17 streams);
    59	// DECODE_SLACK_BYTES mirrors `DEFAULT_TARGET_MESSAGE_SIZE`. The rest
    60	// parameterize the reference flat solve and the reply containers.
    61	// ---------------------------------------------------------------------
    62	
    63	const FAN: u128 = 256;
    64	const DEPTH: i32 = 32;
    65	const LISTING_ENTRY_BYTES: u128 = 17;
    66	const PARKED_REPLY_SKELETON_BYTES: u128 = FAN * FAN * LISTING_ENTRY_BYTES;
    67	const CHILD_CONTAINER_BYTES: u128 = 32;
    68	const STAGES: u128 = 17;
    69	const DECODE_SLACK_BYTES: u128 = 1_114_624;
    70	const DEFAULT_BUDGET: u128 = 16 * (1 << 30);
    71	const DEFAULT_N: u64 = 1 << 40;

Resolution: Derive every constant the crate exports: `const LISTING_ENTRY_BYTES: u128 = 1 + rumors::MERKLE_HASH_LEN as u128;`, `const STAGES: u128 = rumors::link::STREAM_COUNT as u128;`, `const DECODE_SLACK_BYTES: u128 = rumors::DEFAULT_TARGET_MESSAGE_SIZE as u128;`. `FAN` and `DEPTH` mirror `pub(crate)` items that `rumors::testing` does not expose, so either expose them there or state at the declaration that they are the simulator's own model parameters. Rewrite the banner to say which constants are imported and which are the simulator's own. Re-run the certification and re-pin or re-label `check_landed_replication` in the same change (benches-envelope-29). If benches-envelope-32 dissolves the example, this dissolves with it. Acceptance: `grep -n '= 17;\|1_114_624' examples/envelope_sim.rs` returns nothing; no numeric literal in the file duplicates a value the crate exports; the analytic certification passes.
Construction: Add at the top of `main`: `assert_eq!(LISTING_ENTRY_BYTES, 1 + rumors::MERKLE_HASH_LEN as u128); assert_eq!(DECODE_SLACK_BYTES, rumors::DEFAULT_TARGET_MESSAGE_SIZE as u128);`. Both fail today (17 against 25; 1,114,624 against 1,830,400).

Cross-references: benches-envelope-29 (vestigial) moves with this fix, as the entry notes; benches-envelope-32 (verification-gap) is the missing mechanical tie between the certificate and the shipped functions, and the open question on the example's future decides whether this entry is fixed or dissolved.

### swarm-example-16: "roundtrips/sync" counts control-stream flips, which no longer carry the descent
- Where: examples/swarm.rs:720-722 (related: examples/swarm.rs:109-110, examples/swarm.rs:1049-1080, examples/swarm.rs:1122-1149, examples/swarm.rs:1155-1156, examples/swarm.rs:1179-1224, examples/swarm.rs:1299, examples/swarm.rs:1705, src/link.rs:11-13, src/tree/mirror/streaming/remote.rs:8-10, src/tree/mirror/streaming/remote/proxy/start.rs:142, src/tree/mirror/streaming/remote/proxy/start.rs:172, src/tree/mirror/streaming/remote/proxy/start.rs:217-219, src/peer/gossip.rs:932-933, src/peer/gossip.rs:1248-1249)
- Class / severity / confidence: correctness / medium / high
- Provenance: verified (grep of every `control_read`/`control_write` use outside `src/link`: the greeting exchange in `proxy/start.rs` at 142/172 and 217-218, the handshake and epilogue plumbing in `gossip.rs` at 932-933 and 1248-1249; `proxy/work.rs` only threads the halves through at 205-206 and 237, so no descent frame touches the control stream; `Rounds` is attached only to the control halves at 1207-1215, `CountConnector` passes `rounds: None` at 1172, incoming data streams are unwrapped per 1183-1186; `git blame -L 720,722` attributes the comment to b3b877d9, the per-stream transport commit). The per-session magnitude (two or three flips depending on the greeting role and envelope chunking) is assessed from the exchange shapes, not run.
- Seen by: correctness, perfapi (plus the structure lens's `Rounds` Mutex and `CountRead` Option candidates, folded here); refutation: confirmed (both; magnitude corrected to "2 or 3, role-dependent"); history: deliberate but expired (`Rounds` was designed in e8442e6a for one duplex carrying the whole session; b3b877d9 moved the descent onto data streams and confined `Rounds` to the control halves with a rationale its own transport docs already contradicted)
- Owner-gated: no (which option to take is the owner's taste; see the open question)

The readout promises the mean number of request-to-response turns per session, but `Rounds` sees only the control halves, and under the per-stream transport the control stream carries exactly the session envelope: preamble, greeting (the initiator's opening question rides inside it), and epilogue. Every question, confirmation, and supply of the descent rides one of seventeen lazily opened data streams, which by the example's own design carry no `Rounds`. The number reported is therefore a small constant independent of tree divergence, labeled as protocol work, in the tool the module doc calls "the scripted way to measure the swarm itself". Roughly ninety lines (`Rounds`, `RoundState` behind a `Mutex` in a file whose convention is lock-free atomics, `CountRead` with an `Option` that is never `None`, the `rounds` fields and wiring, the row and its `String` formatting) exist to print that constant. Machinery outlived the constraint that justified it, and the module doc and the comment describe a measurement the code does not perform.

Evidence:

    720	    // Roundtrips are write→read flips on the *control* stream — the session's
    721	    // request→response turns — so only its halves carry the `Rounds`; the
    722	    // data streams tally bytes but never roundtrips.
    109	//! - **roundtrips/sync** — mean number of request→response turns per session,
    110	//!   counted from write→read direction flips on the initiator's I/O;

    src/link.rs
    11	//! - the **control stream**: one persistent bidirectional byte stream,
    12	//!   carrying every session's framing (preamble and greeting through
    13	//!   closing epilogue) in order, for the life of the link;

    src/tree/mirror/streaming/remote.rs
    8	//! The transport is a [`Link`](crate::link): 17 logical streams in each
    9	//! direction, each carried by its own independently flow-controlled
    10	//! transport stream, lazily established as the descent needs it.

    1052	struct Rounds {
    1053	    inner: std::sync::Mutex<RoundState>,
    1124	struct CountRead<R> {
    1125	    inner: R,
    1126	    rounds: Option<Arc<Rounds>>,

Resolution: Pick one. (a) Delete `Rounds`, `RoundState`, `CountRead`, the `rounds` fields, the `roundtrips` counters, the row in both readouts, and the module-doc bullet, and surface `Gossiped.stats` instead (swarm-example-18). (b) Re-denominate to something the transport exposes: count data streams opened per session in `CountConnector::connect` (one per descent level that produced replies) and rename the row; or (c) relabel the row as control-stream turns and say what it does not count. Under (b) or (c), the kept counter needs no `Mutex` (an `AtomicBool::swap` plus an `AtomicU64::fetch_add` is the same state machine without a lock or an `unwrap`) and `CountRead.rounds` should be a bare `Arc<Rounds>`. In every case rewrite 109-110 and 720-722 to describe the quantity computed. Acceptance: the header's readout list and the `Rounds` comment describe exactly what is measured; a session over a large divergence and a session between converged replicas produce visibly different values in any row that claims to measure protocol work, or the row is gone.

Cross-references: swarm-example-18 (feature-gap) is option (a)'s replacement readout.

### benches-envelope-34: prefixes() shifts a u64 by 64 at depth 0: a debug-mode panic and a wrong release-mode row
- Where: examples/envelope_sim.rs:807-812 (related: examples/envelope_sim.rs:840, examples/envelope_sim.rs:887-896, examples/envelope_sim.rs:1414-1420, Cargo.toml:174-192)
- Class / severity / confidence: correctness / low / high
- Provenance: assessed (read against the Rust reference: a shift by at least the operand's bit width is arithmetic overflow, a panic with overflow checks on and a masked shift with them off; the Python original at envelope-sim.py:718 evaluates the same shift under numpy, where an unsigned shift by the width yields 0, so the port introduced the bug; I could not run the example)
- Seen by: correctness; refutation: confirmed, severity lowered from medium to low (the vacated assertion at 1420 could not have failed at `j = 1`, where the envelope is the deterministic depth-1 slot cap of 256 and `listed` is structurally at most 256; what remains is a wrong printed row and a debug-mode panic in an example nothing runs); history: no-rationale-found (a port artifact; the port's byte-identical check covered the deterministic manifest, which never calls `prefixes`)
- Owner-gated: no

`prefixes(keys, 0)` computes `shift = 8 * (8 - 0) = 64` and evaluates `k >> 64` on a `u64`. `trie_stats` calls it for `j = 0` unconditionally (840; the loop at 842-866 then never reads the depth-0 entry), and `two_corpus_stats` calls `prefixes(&a, j - 1)` at `j = 1` (891). With debug assertions the documented `--full` run panics before the first Monte Carlo table prints; in release (Cargo.toml sets no `[profile.release]` override) the shift masks to zero, every key becomes its own "prefix", `a_up.binary_search(&(p >> 8))` searches random keys for small values, `listed` at `j = 1` reads 0 instead of about 256, and the assertion at 1420 passes vacuously. Correct at all scales, for all inputs: a panic reachable from an argument the code itself passes, and a release result that differs from the debug result, is incorrect behaviour, not a tolerated corner.

Evidence:

   807	fn prefixes(keys: &[u64], j: i32) -> Vec<u64> {
   808	    let shift = 8 * (8 - j as u32);
   809	    let mut v: Vec<u64> = keys.iter().map(|&k| k >> shift).collect();
   810	    v.dedup(); // input sorted ⇒ prefixes sorted
   811	    v
   812	}

Resolution: Make the depth-0 case explicit: `if j == 0 { return vec![0]; }` before the shift (or `k.checked_shr(shift).unwrap_or(0)`); start `trie_stats`'s range at 1 since nothing reads the depth-0 entry; keep the `two_corpus_stats` call at 891, which needs the depth-0 case handled. Then run `--full --fast` once in a debug profile and once in release and confirm the two-corpus table's `j = 1` "listed meas" column is about `256 · p_occ(N, 1)²`-scale, not 0. Acceptance: `cargo run --example envelope_sim -- --full --fast` completes in a debug profile; the `j = 1` row shows a non-zero `listed meas` consistent with `joint pred`; a unit assertion that `prefixes(keys, 0) == [0]` for a non-empty corpus is committed.
Construction: Debug profile: `cargo run --example envelope_sim -- --full --fast` panics at line 809 with "attempt to shift right with overflow" during the brute-force section. Release profile: insert `assert_eq!(prefixes(&keys, 0), vec![0]);` after `let keys = draw_keys(n_brute, &mut rng);` in `monte_carlo` and observe it fail, or read the printed `j = 1` "listed meas" (about 0.0) against the expected hundreds.

### swarm-example-8: Command-line input reaches `assert!` and a library `# Panics` precondition unvalidated
- Where: examples/swarm.rs:417-418 (related: examples/swarm.rs:171, examples/swarm.rs:199-202, examples/swarm.rs:227-228, examples/swarm.rs:1254, examples/swarm.rs:1310, examples/swarm.rs:1358, examples/swarm.rs:1377-1385, examples/swarm.rs:1431-1435, examples/swarm.rs:1543, src/link.rs:556-560)
- Class / severity / confidence: correctness / low / high
- Provenance: assessed (read)
- Seen by: structure, correctness; refutation: confirmed (the structure candidate is a strict subset); history: no rationale found (asserts and `MAX_PARTIES` original to e8442e6a)
- Owner-gated: no

A CLI flag is environmental input, and the doctrine forbids panics reachable from it. `--parties 1` and `--message-size 0` panic via `assert!` with a backtrace instead of a usage error; `--duplex-capacity 0` passes through to `memory_with_capacity`, whose documented panic fires inside the first `bootstrap_fork`; `--parties 100` is accepted although the UI's ceiling is `MAX_PARTIES = 64`, so the first press of the right arrow drops the dial to 64 through `bump`'s `.min(max)`; `--refresh-ms 0` gives `ui_loop` a zero `sample_interval`, so every wake resamples over a `f64::MIN_POSITIVE` window while the headless path clamps with `.max(1)`; and `args.headless_secs * 1000` overflows `u64` in a debug build for absurd but accepted values. The `Args` doc states the bound in prose ("Must be at least 2.") that the parser does not enforce.

Evidence:

    417	    assert!(args.parties >= 2, "need at least 2 parties to gossip");
    418	    assert!(args.message_size > 0, "message size must be positive");
    1254	    let windows = (args.headless_secs * 1000 / args.refresh_ms.max(1)).max(2) as usize;
    1310	const MAX_PARTIES: u64 = 64;
    1543	    let sample_interval = Duration::from_millis(args.refresh_ms);

    src/link.rs
    559	pub fn memory_with_capacity(capacity: usize) -> (MemoryLink, MemoryLink) {
    560	    assert!(capacity > 0, "a link stream must buffer at least one byte");

Resolution: Validate at parse with clap value parsers: `value_parser = clap::value_parser!(u64).range(2..=MAX_PARTIES)` on `parties` (and make the field a `u64` to match `Controls`, or state that unbounded CLI counts are wanted for scripted runs), `.range(1..)` on `message_size`, `duplex_capacity`, and `refresh_ms`; compute `windows` with `saturating_mul` or from a `Duration`; delete the two asserts and the prose bound. Acceptance: `cargo run --example swarm -- --parties 1` (and `--duplex-capacity 0`, `--refresh-ms 0`) prints a clap usage error; no `assert!` on `args` remains; the UI's and the CLI's bounds are the same constant.
Construction: `cargo run --example swarm -- --duplex-capacity 0` today panics at src/link.rs:560 from inside `bootstrap_fork` rather than at argument parsing.

Cross-references: swarm-example-12 (idiom) covers the hand-synchronized `TEST_DUPLEX_CAPACITY`; the open question on sharing `MAX_PARTIES` below.

### swarm-example-20: After a party thread dies, the coordinator's retry paths spin without the tick they promise, and `shrink` can drop a live party
- Where: examples/swarm.rs:961-961 (related: examples/swarm.rs:899-911, examples/swarm.rs:1005-1012, examples/swarm.rs:577-583, examples/swarm.rs:79-80, examples/swarm.rs:86)
- Class / severity / confidence: correctness / low / high
- Provenance: assessed (read)
- Seen by: correctness (the pacing comment); refutation: reframed ("current can never change" is wrong: `grow`/`shrink` draw a random party each retry and succeed as soon as a live one is drawn, so the busy loop is brief), and the `shrink` state loss raised as new; history: no rationale found (loop and comments original to e8442e6a)
- Owner-gated: no

Both consequences live in the regime after a party thread has panicked, which the example treats as a bug and the process survives (a spawned thread's panic does not abort the process). First, `grow` and `shrink` return immediately when a party's `control` sender fails, and `run_coordinator` sleeps only in the balanced branch, so the retry is immediate: a busy loop until a live party is drawn, while the comments promise a "next tick" that does not exist on these paths. Second, `shrink`'s short-circuit at 1005-1009 and the destructure at 1010-1012 can leave party `a` wound down with no one listening: if `a`'s send succeeds and `b`'s fails, or `a` replies and `b` does not, the function returns and drops `a_rx` (or `da`); `a` meanwhile sets `engaged` permanently, its `reply.send` fails, `run_party` returns, and `a`'s `Rumors` and party region are dropped unretired while its `SwarmPeer` stays in the directory as a dead entry. The module doc's "the directory stays a partition of the seed's party space" and "reclaimed rather than leaked" then no longer hold, and the comment at 1008 describes the trigger but not this consequence.

Evidence:

    961	        return; // parent already gone; try again next tick
    1005	    if a.control.send(Command::WindDown { reply: a_tx }).is_err()
    1006	        || b.control.send(Command::WindDown { reply: b_tx }).is_err()
    1007	    {
    1008	        return; // a party already gone; try again next tick
    1009	    }
    1010	    let (Ok(da), Ok(db)) = (a_rx.recv(), b_rx.recv()) else {
    1011	        return;
    1012	    };
    580	                    let donation = wind_down(&runtime, &net, &me, &inbox, rumors);
    581	                    let _ = reply.send(donation);
    582	                    return;

Resolution: Move the 20 ms sleep to the loop's tail so every iteration paces, or evict an entry whose `control.send` fails from the directory so `current` reflects the live set; in `shrink`, when one wind-down succeeds and the other fails, relaunch the wound-down survivor with `launch_party` (its `Donation` is in hand or arriving) rather than dropping it. Rewrite or delete the two comments to match. Acceptance: with one party thread killed, the coordinator's CPU stays near zero, no `Rumors` is dropped unretired, and the comments match the mechanism.
Construction: add a test-only `Command::Panic` (or temporarily `panic!` in `run_party` for `id == 1`), start with `--parties 3`, dial parties down; observe `shrink` returning through 1008 or 1011 and a party's `Donation` dropped, with the directory length still 3.

### swarm-example-23: Headless latency and roundtrip rows include the warmup window because `Stats` carries display strings
- Where: examples/swarm.rs:1273-1284 (related: examples/swarm.rs:1249-1251, examples/swarm.rs:1414-1425, examples/swarm.rs:1449-1460, examples/swarm.rs:1291-1295, examples/swarm.rs:1697-1701)
- Class / severity / confidence: correctness / low / high
- Provenance: verified (grep for `u64::MAX` and `"--"` lists the duplicated sentinel blocks at 1291-1295 and 1697-1701 and the `String` fields at 1418/1422; the span mismatch by reading 1249-1251 against 1281-1284)
- Seen by: structure (the `Stats` types), prose, correctness (the span); refutation: all three confirmed, root shared; history: no rationale found (`latency: String`/`roundtrips: String` original; d987a2be's headless code works around them with the comment at 1273-1275 rather than changing them)
- Owner-gated: no

`run_headless`'s doc says the first window is discarded as warmup "so the summary reflects steady state", and the rate rows honor that; but latency and roundtrips are recomputed from `end = Snapshot::take(net)`, cumulative since process start, so those two rows include the warmup window and everything before sampling began. The root cause the comment names is that `Stats`, the one readout definition shared by UI and headless mode, stores `latency` and `roundtrips` as formatted `String`s and `latency_best_nanos` as a raw value with a `u64::MAX` sentinel, so the headless path cannot average them and the sentinel-to-"--" block is written out twice, character for character. The comment states an artifact of the field types where the statistical reason the span recompute is the right estimator (a session-weighted mean, not a mean of window means) is what the code cannot show.

Evidence:

    1249	/// Sample the same windowed statistics the UI renders, for `headless_secs`,
    1250	/// then print one summary line per readout row to stdout. The first sampling
    1251	/// window is discarded as warmup so the summary reflects steady state.
    1273	    // Latency and roundtrips are formatted strings in `Stats`; recompute the
    1274	    // means from the raw counters across the whole measured span instead. The
    1275	    // best is the fastest session seen in any window.
    1281	    let end = Snapshot::take(net);
    1282	    let syncs = end.syncs.max(1);
    1283	    let latency = Duration::from_nanos(end.sync_nanos / syncs);
    1284	    let roundtrips = end.roundtrips as f64 / syncs as f64;
    1418	    latency: String,
    1422	    roundtrips: String,
    1291	        if best == u64::MAX {
    1292	            "--".to_string()
    1697	                if stats.latency_best_nanos == u64::MAX {
    1698	                    "--".to_string()

Resolution: Make `Stats` carry numbers: `latency: Option<Duration>`, `roundtrips: Option<f64>` (if the row survives swarm-example-16), `latency_best: Option<Duration>` converted from the sentinel once in `compute`; one `fn or_dash(Option<Duration>) -> String` at both display sites. In `run_headless`, keep the snapshot taken at the end of window 0 and compute the span mean from `end` minus it, so the denominator matches the doc; replace the comment with the estimator argument. Acceptance: `Stats` has no `String` field and no sentinel; the sentinel-to-"--" block appears once; every headless row is computed over the same span, and the doc's warmup claim holds for every row.
Construction: run `--headless-secs 3 --refresh-ms 1000`; the "over N sessions" count printed on the latency row is `end.syncs` (cumulative) and exceeds the sum of the post-warmup windows' session deltas.

### swarm-example-27: The terminal is not restored on the `?` arms between enable and disable
- Where: examples/swarm.rs:1521-1531 (related: examples/swarm.rs:1510-1519)
- Class / severity / confidence: correctness / low / high
- Provenance: assessed (read)
- Seen by: correctness; refutation: confirmed (also notes the hook is never restored after `run_ui` returns); history: no rationale found (byte-for-byte original; R34 noted the hook covers panics without examining the error arms)
- Owner-gated: no

`run_ui`'s doc promises restoration "on any exit path", and the panic hook covers panics, but the error arms do not: if `execute!(stdout, EnterAlternateScreen)?` or `Terminal::new(backend)?` fails, the function returns with raw mode enabled; if `disable_raw_mode()?` fails, `LeaveAlternateScreen` is skipped. `main` then runs the shutdown sequence and exits with the user's terminal in raw mode. The restore code already exists twice (hook and happy path) and belongs in one place.

Evidence:

    1510	/// Run the terminal UI until the user quits. Sets up and tears down raw mode
    1511	/// and the alternate screen, restoring the terminal on any exit path.
    1521	    enable_raw_mode()?;
    1522	    let mut stdout = io::stdout();
    1523	    execute!(stdout, EnterAlternateScreen)?;
    1524	    let backend = CrosstermBackend::new(stdout);
    1525	    let mut terminal = Terminal::new(backend)?;
    1529	    disable_raw_mode()?;
    1530	    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

Resolution: A `TerminalGuard` whose `Drop` runs `disable_raw_mode()` and `LeaveAlternateScreen` (errors ignored), created right after `enable_raw_mode()` succeeds; the hook and the happy path both use it, `show_cursor` stays on the happy path. Restore the previous panic hook when `run_ui` returns, or say why it stays. Acceptance: an injected failure at `EnterAlternateScreen` or `Terminal::new` leaves the terminal in cooked mode; the restore code appears once.
Construction: run the example with stdout closed (`cargo run --example swarm 1>&-`): `enable_raw_mode` succeeds on the controlling tty, the `execute!` write to stdout fails and returns through `?`, and the shell prompt that follows is in raw mode.

### benches-envelope-31: joint_int's expect argues a bound that fails for j = 16 at N ≥ 2⁶²
- Where: examples/envelope_sim.rs:607-608 (related: examples/envelope_sim.rs:128-133, examples/envelope_sim.rs:295, examples/envelope_sim.rs:579-591, examples/envelope_sim.rs:691-705)
- Class / severity / confidence: correctness / nit / high
- Provenance: verified (by arithmetic against 579-591: at `n = 2⁶²`, `nn = 2¹²⁴`, `num_bits = 125`, `den_exp = 128`; the first branch fails because `125 > 128 − 48 − 2 = 78`; `den_bits = 129 < num_bits + 5 = 130`, so `small_mean_quantile` returns `None`; `pow256(16)` is `None` at 128-133, and the `expect` fires. At `n = 2⁶¹` the second branch returns `Some`, so the threshold is exactly `n ≥ 2⁶²`)
- Seen by: correctness; refutation: confirmed; history: no-rationale-found (a port artifact; the comment transcribes the Python's informal premise)
- Owner-gated: no

The comment proves `256^j` fits `u128` from "8j is within nn's bit length", but the function's declared `u64` domain admits inputs where `small_mean_quantile` returns `None` and `pow256` is `None` together. It is unreachable only because the hard-coded sweep grids stop at `2⁵⁰` and `occ_hi`'s `assert!(n < (1 << 60))` at 295 runs earlier in the sweep, neither of which the message names. Doctrine: every `expect` message is a one-line proof of why it cannot fire; this one's premise is incomplete for the function's domain.

Evidence:

   607	    // Non-sub-unit mean ⇒ 8j is within nn's bit length ⇒ 256^j fits.
   608	    let slots = pow256(j).expect("bulk regime keeps 256^j within u128");

Resolution: Assert the `N < 2⁶⁰` premise at `joint_int`'s entry (matching `occ_hi`) and cite it in the message, or fall back to the corpus cap `u128::from(n)` when `pow256` is `None`, which is the value the deterministic cap gives anyway. Acceptance: the message (or a guarding assert) names a premise that covers every `u64` the function accepts.
Construction: Call `joint_int(1 << 62, 16)`: `small_mean_quantile(2¹²⁴, 128, 48)` returns `None`, `pow256(16)` is `None`, and the `expect` panics.

## Positives

- No panic is reachable from wire, payload, bookmark bytes, storage failure, or environment input anywhere in production code. The inventory sweep classified all 88 production panic-macro sites and 8 release asserts and found none caller- or wire-reachable; the session-bookmark, materialized, remote-codec, tree-core, and tree-typed partitions each traced every panic site in their files to a guarding branch or a crate-bug precondition, and every `expect` and `unreachable!` in production carries a one-line proof that is true of the surrounding code (materialized, remote-codec, link, tree-core, tree-typed).
- Peer input is judged in one place and bounded before it drives anything: `SupplyRuns::observe` is the single ingress for supplied leaf records, every wire-reachable failure there is a typed error, the declared-`set_len` charge lands before custody, and the wire path cannot reach `from_sorted_leaves` with unsorted, uncontained, duplicate, or oversized leaves because the decoder rejects those shapes first (remote-adapter-streams, streaming-backend-window, tree-typed). Every peer-controlled datum reaches the walk through a merge-join or a peekable fan, with no indexing and no arithmetic on wire values except the ledger's checked add (materialized).
- The commit critical sections in `Tree::react` and `Tree::join` are panic-atomic by statement order alone (replace, assign, then drop), and the property is pinned from three directions, including the one caller-reachable unwind source, a panicking `T` destructor on a last-handle drop, with the caught panic's message checked (tree-core, async-hazards). async-hazards-3 concerns the lock hold around those sections, not their atomicity.
- Identity duplication is structurally impossible on every exit path of a retirement: `PartyGuard` makes the speculative fork's recovery a drop guard, the party is sliced out of the durable record while still held, and it leaves the guard only immediately before `party::send` (session-bookmark, async-hazards). `Network` reserves its all-zero bootstrap sentinel structurally (api-core).
- Every `tokio::select!` in production is cancellation-clean by construction, the `watch` write lock is never held across an await, the bookmark-then-`watch` lock order is stated and followed at both nesting sites, and clippy's `await_holding_lock` fires nowhere in the crate (async-hazards, clippy-pedantic). Cancellation of `connect`, `accept`, `Incoming::accept`, and the router's `select!` arms is safe by construction, and the poison latch is set before any wire traffic (link).
- The deadlock-freedom argument is enforced structurally: `yield_resolve_query!` keeps wire, resolution, and dependent work in one expansion so the publication order cannot drift from the argument, every channel constructor carries its capacity argument, and the one-slot sufficiency is re-checked by a test-only trace on every run (materialized). `Encoded::write_with` makes "wire before internal publication" the only order the proxy's API admits (remote-adapter-streams, remote-proxy).
- Liveness is judged deterministically: `run_to_quiescence` distinguishes a self-wake from a stall without wall-clock guessing, disables tokio's cooperative budget, and turns a wire stall into a named `Quiescence::Stalled` verdict; twenty-three files across src/ and tests/ use it, and every successful in-memory session in the integration suites ends in `assert_control_drained` (testing-infra, tests-lifecycle, tests-observation, tests-common).
- No traversal recurses on input-controlled depth: `act`, `join`, and `Unknown` recurse on the type-level height bounded at 32, the erased `unknown` prune is bounded by the prefix's remaining height, and the capture renderer's depth budget is one counter spanning structural descent and embedded re-parses, checked at every `render_*` call site (tree-core, tree-typed, materialized, remote-capture-atlas).
- Arithmetic on peer-declared quantities is saturating or checked: the window solve is total and floor-preserving under saturating arithmetic (the integer envelope inequalities were re-derived with a replica), `SupplyLedger::charge` handles the wrapped counter explicitly, and every narrowing `as` in library code sits behind a range check, a match arm, a `const` bound, or a deliberate wrapping intent (streaming-backend-window, materialized, clippy-pedantic).
- The wire-facing parsers are total over their input: every truncation boundary of the preamble and the party hand-off is typed, the CBOR head reader rejects every wider spelling of every value, and `Preamble::decode` diagnoses in an order that reports a dialect skew as `VersionMismatch` rather than garbled fields (mirror-common). `resume_payload` is the one exception this document records.
- The known-bad-mechanism discipline is applied where it matters for correctness: the value-oracle adequacy tests construct a suppressed redaction and a dropped insert and show them failing; `folding_delivered_versions_can_lose_a_message` shows the tempting wrong observer implementation losing a message; every liveness check in the link conformance suite has a committed negative control that fails as `Stalled` (tests-disruption-handshake, api-core, conformance).
- The fresh-eyes application's runtime observations agreed with the documented contracts: both ends' drivers ended cleanly on stream end and on remote hang-up, concurrent one-shot `gossip` on both ends merged into one session with both sides reporting `Led::Local`, one side's `bytes_sent` equalled the other's `bytes_received` over TCP, and redactions issued on each side were honored on both in one session (fresh-eyes).

## Open questions for Finch

1. **`Inner.party: Option<Party>` (api-core-2).** The `None` state exists only while `Peer::retire` holds the party in flight, yet it forces two unreachable arms (src/batch.rs:136-139 and src/peer/gossip.rs:825-828) with different shapes (a silent `return false` versus adopt-the-donation). Rule once for both arms now, taking api-core-2's `expect` as the cheap consistent fix, and schedule making `Inner.party` total (holding the in-flight party in the retire future) as a design item. Recommendation: the `expect` now; the dissolution later.
2. **Is a pooling `Dial` a first-class deployment shape for the routed adapter (link-28)?** This decides whether the recovered-connection population gets its own bound applied before `READY`, or the sizing duty stays with the caller and the constructed test pins the failure. Recommendation: first-class; the design record already names `Dial` as the pooling boundary, and a pre-`READY` budget is a small change that removes a whole failure class.
3. **Reclaim timing (session-bookmark-21).** Re-word the public `Bookmark` doc to the code's rule (reclaim at the first unsuppressed pre-session checkpoint after dominance), or extend the persist gate so a hearsay-only advance that newly dominates a stranded identity triggers reclaim, relaxing the never-write-on-hearsay rule `tests/bookmark_when.rs` pins. Recommendation: re-word, consistent with the recorded ruling; either way, land the constructed scenario as a test.
4. **Where the exactness clamp lives (mirror-common-8, remote-codec-14).** Recommendation: in `resume_payload`, whose documentation already promises exactness without a capacity precondition, so every present and future caller inherits it; state the remaining precondition (`payload.len() <= len`) in the doc and check it.
5. **Keep the sync decoder oracle?** Its independent coverage is narrower than its lines suggest, but remote-codec-10 is exactly the class only an independent implementation catches, and retiring an instrument requires the replacement to demonstrate coverage first. Recommendation: keep it, give the struct a doc stating its role, and dissolve the shared fragments (remote-codec-8).
6. **Destructors under the write lock (async-hazards-3).** The destructor constraint must be documented in any case, since `traverse::act` and `traverse::join` drop `T` mid-walk. Do you also want (a) the cheap deferral of the pre-image drop past `send_if_modified` (an internal signature change to `Tree::act` and `Tree::join` that removes the bulk deallocation from the lock hold), and (b) the fuller evacuation that collects the mid-walk drops into a sink the walk hands back? Recommendation: document now and do (a); treat (b) as a design proposal. The api-core partition asks a companion question: whether a measured figure for a large `send_all` under the lock (O(N log N) for 10^5 messages) is wanted before persistent storage lands.
7. **`Tree::react`'s role (tree-core-30).** `react` documents a general versioned-apply contract that only tests exercise; its sole production caller `act` supplies one party's ascending chain. Keep it generic (then the storage-site fix and a per-key ceiling across Forgets are the repairs), or collapse it into `act`'s commit section and document the multi-action semantics once at the leaf level. Recommendation: keep `react` as `act`'s commit section, document it as such, and store the applied action's version at the leaf regardless.
8. **`Peer::seed_rng` (api-core-11).** Three reports recommend three things: the inventory sweep would un-hide and document it (deterministic seeding is a legitimate testing need); the deps sweep and api-core-12 would gate it behind `test-internals`; the tests-wire-format partition would keep it hidden and offer a documented constructor taking a `Network` value instead of an RNG, because a caller-supplied RNG lets two processes create the same `Network` each holding `Party::seed()`, the two-universes hazard AGENTS.md forbids. Recommendation: the `Network`-taking constructor with the safety rule restated there, and gate `seed_rng`; whichever way, `from_rng` should bound its retry or state the RNG requirement.
9. **The envelope simulator's future (benches-envelope-28).** Should `examples/envelope_sim.rs` remain the certifying tool of record, or should the dominance certificate move into `window/tests.rs` as a differential test over the shipped functions? And should its `FAN` and `KEY_DEPTH` mirrors reach the crate through `rumors::testing` or be declared the simulator's own parameters? Recommendation: move the certificate in-tree and declare the two structural constants as the simulator's own; if the example stays, import every constant the crate exports and set `test = true` so the analytic tier runs.
10. **The swarm roundtrips row (swarm-example-16).** Delete it and report `Gossiped.stats` instead (swarm-example-18), relabel it as control-stream turns, or count data streams opened per session. Recommendation: delete; the per-session measures already exist in the library, and a hop count derived from I/O flips has no denominator the transport still supports.
11. **`MAX_PARTIES` and the CLI (swarm-example-8).** Share the UI's ceiling with `--parties`, or leave the CLI unbounded for scripted headless runs on large machines? Recommendation: share it, and raise the constant if big runs are wanted.
12. **Severity of tests-disruption-handshake-7.** The rubric puts a harness bug that masks failures at high; the refutation pass argued medium because the masked class is confined to the TCP serving path. Recommendation: fix regardless; the severity only affects scheduling.
13. **The docs.rs leg (deps-3).** The constructed run settles that docs.rs's own build fails at HEAD. Land the `doc_cfg` fix, and decide whether the nightly `--cfg docsrs` rustdoc joins `gate` as well as `ci` (it is one nightly rustdoc of one crate). Recommendation: both; the gate is where a contributor would see it.
14. **Cancellation coverage.** No test in the streaming suite drops the `mirror` future mid-session, and the integration suites cancel only gossip/gossip; tests/common/overlap.rs:81 says dropping an unfinished `Session` models a cancelled one, but no executor drops one. Recommendation: a walk-tier pin that cancels at a drawn poll count and checks the local root is unchanged, plus a deterministic cancel-at-every-poll-prefix sweep over a serving bootstrap and over a retirement, the dual of the disruption engine's cut sweep.
15. **`Changes` and a content change with no frontier advance.** `Changes` yields on `latest()` changing while the gossip commit notifies on `peer_retiring || tree_changed || ceiling_advancing`; a content change that moved no frontier would be a lost tick. Two lenses could not construct one (every gain brings a version outside the local frontier; every shed requires the peer's frontier to dominate the leaf, which the same join brings in). Recommendation: if you see a path, `Changes` under session overlap is untested; if not, one sentence in the `Changes` field doc stating why `latest()` suffices closes the reading.
16. **`ErasedPrefix::assume` in release builds.** The length-versus-height check is debug-only (src/tree/typed/prefix.rs:45-49); in release a cross-height re-tag zero-fills the tail and misplaces a leaf rather than crashing. Programmer error only, and all tests run in debug. Recommendation: promote the O(1) check to a release `assert!`; the failure mode is silent misplacement.
17. **Two small wake and route questions.** src/peer/gossip.rs:710-711 wakes watchers when a party is taken or forked while `bookmark_update`'s closure (551-552) argues a party-only change owes no wake; both are harmless under `Changes`' frontier compare, but which rule is intended? And `race_session`'s `Ok` arm (src/tree/mirror/streaming/driver.rs:79) never consults `first_error`; the state is unreachable because `divert` parks after reporting, and nothing says so at the arm. Recommendation: adopt one wake rule at both sites and say so once; a one-line comment at the `Ok` arm, no test.

## Refuted by construction

No correctness-class finding was refuted. Of the nineteen constructed demonstrations in the record at `witness/results.md` (fourteen in the first witness pass, five in the second), eight settle entries in this document (link-28, mirror-common-8, remote-codec-14, remote-capture-atlas-13, streaming-tests-11, tests-disruption-handshake-7, deps-3, and, from the second pass, async-hazards-3), all confirming their findings; the one demonstration that refuted its finding, conformance-24, belongs to the verification-gap class and is recorded there. The remaining demonstrations (conformance-25, conformance-28, conformance-31, materialized-27, remote-capture-atlas-17, remote-proxy-tests-10, tests-bookmark-9, tests-bookmark-12, tests-observation-28) confirm verification-gap and test-quality findings and are recorded in that document, and mirror-common-3's arm swap is recorded in the simplification document; of them, only remote-capture-atlas-17 bears directly on an entry here and is cross-referenced under remote-capture-atlas-13.

## Counts

Twenty-six entries, recorded at commit 9e5784fb.

| Module | High | Medium | Low | Nit | Total |
|---|---|---|---|---|---|
| Crate root and public surface | 0 | 1 | 1 | 1 | 3 |
| Session and bookmark | 0 | 0 | 1 | 0 | 1 |
| Link | 0 | 1 | 0 | 2 | 3 |
| Tree core | 1 | 0 | 2 | 0 | 3 |
| Mirror common (with the streaming test suites) | 1 | 1 | 0 | 0 | 2 |
| Materialized | 0 | 0 | 1 | 0 | 1 |
| Remote codec | 0 | 1 | 2 | 0 | 3 |
| Remote capture and codec tests | 1 | 0 | 0 | 0 | 1 |
| Integration tests | 1 | 0 | 0 | 0 | 1 |
| Benches and examples | 0 | 2 | 5 | 1 | 8 |
| **Total** | **4** | **6** | **12** | **4** | **26** |

By provenance: eight entries are demonstrated by construction (seven in the first witness pass, async-hazards-3 in the second), five are verified (a run, a grep, or arithmetic against pinned sources), and thirteen are assessed by reading. The high count is four rather than the finalizers' three because async-hazards-3 was raised after the second witness pass. By owner gating: two entries (link-28, session-bookmark-21) need an owner decision before their resolution can be chosen; the rest do not.
