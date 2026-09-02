# Partition swarm-example: The swarm example and its convergence tests

## Partition summary

`examples/swarm.rs` (1800 lines) is an interactive demonstrator and the crate's documented self-measurement tool. `main` seeds one `Rumors<Vec<u8>>`, forks it by real bootstrap sessions (`bootstrap_fork`) into N party-disjoint replicas, one OS thread each, and hands the directory to a coordinator thread that grows the swarm by fork and shrinks it by `Peer::retire` over an in-memory wire. Each party thread loops: serve inbound sessions from its inbox, obey membership commands, initiate a Poisson-scheduled `Rumors::gossip` over a `memory_with_capacity` link pair when it can claim itself and a peer with compare-and-swap on a per-party `engaged` flag, and otherwise churn under a steady-state controller (`p_add = T / (T + L)`) fed by an `UnorderedMessages` observer that replays the set into a per-thread redaction pool. A metrics layer decorates the link halves through `Link::into_parts`/`LinkParts::into_link` with byte- and direction-flip-counting `AsyncRead`/`AsyncWrite` wrappers; a ratatui UI or a headless sampler reads windowed rates off the atomic counters. `examples/swarm/tests.rs` (149 lines, test code) is one seeded, single-threaded test that drives three forked parties through a target drop and raise and judges each party's four-sample settled mean against a half-to-double band; `Cargo.toml` registers the example with `test = true`, so `just test` runs it.

The concurrency core is in good shape and argued at the site. The rendezvous is a one-slot protocol whose wait-for-graph argument is stated once in the module doc and whose `wind_down` doc gives the one-paragraph proof the code then follows; `InflightGuard` names the concrete failure it prevents and the ordering it relies on; the shutdown handoff is SeqCst on both sides and deadline-bounded with a diagnostic instead of a hang. Every `expect` on a session carries a pointer to the one place that argues why the example panics where an application would match on the error, and every claim I cross-checked against the crate (the `Error` variants' semantics, `SessionState` riding through decoration, `try_into_peer` resolving immediately, the CBOR encoding of `Vec<u8>`) holds. The controller's stale-discard argument is precise, and the test that pins it says which defect it catches and why three parties is the smallest swarm that exposes it.

The dominant issue is an instrument the transport change left behind: the "roundtrips/sync" row counts write-then-read flips on the control stream, which since the per-stream link landed carries only the session envelope (preamble, greeting, epilogue), so the row reports a small divergence-independent constant while the module doc promises the session's request-to-response turns. About ninety lines (`Rounds`, `RoundState`, `CountRead`, the `rounds` wiring, the row) exist to print that constant, and the `Gossiped` the example already receives carries the per-session measures the readout wants. The second substantive issue is the payload type: `Vec<u8>` encodes as a CBOR integer array (about 1.9 wire bytes per random payload byte, with per-element encode and decode on every send and receive) where `bytes::Bytes`, the idiom the crate's own tests use, encodes as a byte string. The rest is a set of low findings a careful maintainer would schedule (CLI input reaching `assert!` and a library `# Panics` precondition, a terminal-restore gap on the error arms, two headless rows that include the warmup window, a data struct carrying display strings, a comment ghost of the retired `Key` vocabulary, docs that describe state the code does not have) and a batch of nits: orphaned lines from a mechanical summary split, em-dashes in `//` comments, moralized adjectives, duplicated rationales, an unreachable drain kept "for good measure", and a `Snapshot` name collision that forces qualified paths.

Total lines read: 1949 (`examples/swarm.rs` 1800; `examples/swarm/tests.rs` 149, test code). Cross-checked in `src/link.rs`, `src/tree/mirror/streaming/remote.rs`, `remote/streams.rs`, `remote/proxy/{start,work}.rs`, `src/peer/gossip.rs`, `src/rumors/unordered.rs`, `src/message.rs`, `src/lib.rs`, `tests/dispute_wire.rs`, `Cargo.toml`, `justfile`, `tools/doclint`, the vendored `ciborium-0.2.2`, and the link-transport review ledger. No cargo, build, or test command was run.

## Findings

### swarm-example-1: Module doc omits the run command and two quit keys, and its step numbering disagrees with `run_party`
- Where: examples/swarm.rs:13-20 (related: examples/swarm.rs:114-123, examples/swarm.rs:550-611, examples/swarm.rs:1558-1561, examples/swarm.rs:1760-1761)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read)
- Seen by: prose; refutation: confirmed; history: no rationale found (all four gaps original to e8442e6a)
- Owner-gated: no

The module doc is the essay a reader uses to run and interpret the example, and three details drift from the code: `# What it does` numbers the party loop 1-3 (serve, initiate, controller) while `run_party` numbers it 1-4 with membership commands as step 2, so cross-referencing the numbers misleads; `# Controls` and the footer list only `q` for quit while `ui_loop` also quits on `Esc` and `Ctrl-C`; and the doc never says how to run the example (`cargo run --release --example swarm -- [flags]`), where whether `--release` matters for a throughput readout is the first question a reader has. (The readout bullet at 109-110, "on the initiator's I/O", is resolved by swarm-example-16.)

Evidence:

    13	//! 1. **Serve** any inbound sync requests waiting in its inbox (it is the
    17	//! 3. Otherwise, run the **steady-state controller**: compare the number of
    116	//! `↑`/`↓` select a parameter, `←`/`→` adjust it (`Shift` for a coarse step),
    117	//! `space` pauses all churn, `q` quits.
    566	        // 2. Obey membership commands from the coordinator, between sessions.
    1558	                KeyCode::Char('q') | KeyCode::Esc => break,
    1559	                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {

Resolution: Add "obey membership commands from the coordinator" to the overview list (or drop its numbers); write "`q`, `Esc`, or `Ctrl-C` quits" at 117 and in the footer at 1760-1761 (or remove the extra keys from the handler); add a two-line `# Running` section with the `cargo run --release --example swarm -- [flags]` invocation and the `--headless-secs` variant. Acceptance: every key the loop handles is listed; the overview's steps map one-to-one onto `run_party`'s numbered comments; a reader can run the example from the module doc alone.

### swarm-example-2: "version vector" collides with the distributed-systems term of art
- Where: examples/swarm.rs:27-28 (related: none)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep finds this single occurrence; `git blame -L 27,28` attributes it to 9c73d7b4, the `Key` retirement, which mechanically renamed "key vector")
- Seen by: prose; refutation: confirmed; history: no rationale found (rename residue)
- Owner-gated: no

The module doc calls each party's `Vec<Version>` redaction pool "the version vector". In a crate built on Interval Tree Clocks, "version vector" names the causality structure that ITCs generalize, not a list of message versions; the rest of the file says "pool" or "version pool". Established terms of art are used only in their established sense.

Evidence:

    27	//! the next sync. The version vector is per-thread: no shared rumor-set
    28	//! state, no lock contention on the hot path.

Resolution: Write "The pool is per-thread" (or "version pool"), matching `Donation`'s and `run_party`'s vocabulary. Acceptance: `grep -n 'version vector' examples/swarm.rs` returns nothing.

### swarm-example-3: The rendezvous, membership, shutdown, and decorated-link claims are prose-only
- Where: examples/swarm.rs:89-93 (related: examples/swarm.rs:58-67, examples/swarm.rs:481-507, examples/swarm.rs:1200-1245, Cargo.toml:194-198, src/conformance/link.rs:158-169)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`grep swarm` over justfile, .github/workflows, .config/nextest.toml, .cargo/mutants.toml returns nothing; Cargo.toml:194-198 read; the conformance `check` signature read at src/conformance/link.rs:158-169). That `cargo nextest run --workspace` executes the example's test binary is assessed from `test = true`, not run.
- Seen by: correctness; refutation: reframed (the controller test does run under `just test`; the untested-surface claim stands); history: no rationale found (Cargo.toml's comment scopes the tests to the controller; no note declares the rest deliberately untested)
- Owner-gated: yes: adding gate legs or a smoke test is a gate-cost decision

The module doc makes checkable claims (no wait-for cycle, shrink never fails, the drain is bounded, decoration preserves the link contract), and the only test in the partition covers the controller arithmetic. `main`, the coordinator, `grow`/`shrink`, `wind_down`, `try_initiate`, the shutdown drain, and `initiator_link`/`responder_link` are reached only by `cargo check`/`clippy --all-targets`. The crate ships `conformance::link::check` for exactly this kind of caller-built link, and the swarm builds two and runs neither through it. Every contract clause should have a committed check that fails if it is wrong.

Evidence:

    89	//! Because every live party is a disjoint fork of the common seed, any two can
    90	//! always reconcile, so shrink never fails. The directory itself is an
    91	//! [`ArcSwap`], so the sync hot path reads it without locking; only the
    92	//! coordinator ever swaps it, one membership change at a time. The floor is two
    93	//! parties — there is no one to gossip with below that.

    Cargo.toml
    197	# The steady-state controller's convergence tests live in the example.
    198	test = true

Resolution: (1) In `swarm/tests.rs`, under `#[cfg(feature = "conformance")]`, run `rumors::conformance::link::check` over `(initiator_link(a, ..), responder_link(b, ..))` from a `memory_with_capacity` pair, under a timeout; `check` is generic over both ends separately, so the asymmetric pair fits. (2) Factor `main`'s body into a `run_swarm(args, drive)` that a bounded smoke test can call: three parties, dial `controls.parties` 3 to 6 to 3 over a short wall-clock budget, then run the shutdown sequence and assert the drain completed inside `SHUTDOWN_DRAIN_DEADLINE` and `net.peers.load().len()` equals the desired count. (3) Optionally a `just ci` leg running `--headless-secs 2 --parties 4` as a no-rot check. Acceptance: a test fails if a decorated link violates the link contract; a test fails if grow/shrink/wind-down/shutdown wedge or leave the directory at the wrong size.
Construction: break the decorated link deliberately (drop `session: parts.session` for a fresh `SessionState` in `initiator_link`) and observe that nothing in `just gate` fails today; the conformance check in (1) would.

### swarm-example-4: Moralized adjectives: "genuine", "honest", "a real application"
- Where: examples/swarm.rs:155-155 (related: examples/swarm.rs:570, examples/swarm.rs:737-738, examples/swarm.rs:767, examples/swarm.rs:1026)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -nwiE 'genuine|honest|real application'` over both files lists exactly 155, 570, 737, 767, 1026)
- Seen by: structure, prose; refutation: confirmed (both lenses; sides with cutting "real application" too); history: deliberate but expired ("genuine" entered with f612b471 to contrast a wire-bootstrapped fork against the `Known::fork` clone removed in that same commit; the referent is gone)
- Owner-gated: no

"genuine" adds nothing to "party-disjoint" (a non-genuine disjoint peer does not exist); "panicking is honest" moralizes a choice whose mechanism the same sentence gives (in-process links, one universe, so a failed session is a bug); "a real application" contrasts the example against an unnamed unreal one where "an application over a network" says what is meant. Each deletes without loss, which is the test for a default-dialect tell.

Evidence:

    155	/// Create a genuine party-disjoint peer that inherits `parent`'s content.
    570	                    // Create a genuine disjoint child that inherits our content,
    737	    // failed session is a bug and panicking is honest. A real application

Resolution: 155 and 570: drop "genuine". 737: "so a failed session is a bug, and the example panics to report it. An application over a network matches on the error instead:". 767 and 1026: "an application matches on the session error instead of panicking". Acceptance: none of "genuine", "honest", "real application" remains in the partition.

### swarm-example-5: `Payload = Vec<u8>` encodes as a CBOR integer array, nearly doubling wire and decode cost per payload byte
- Where: examples/swarm.rs:185-188 (related: examples/swarm.rs:853-857, examples/swarm/tests.rs:13, Cargo.toml:126, tests/dispute_wire.rs:33-36, src/message.rs:25-27)
- Class / severity / confidence: performance / medium / high
- Provenance: verified (vendored ciborium-0.2.2 src/ser/mod.rs:165-168 emits `Header::Bytes` for `serialize_bytes` and :226-227 emits `Header::Array` for `serialize_seq`; serde's `Vec<T>` serializes through `serialize_seq`; Cargo.toml:126 enables `bytes` with `serde`; the 1.906 bytes-per-uniform-byte figure is arithmetic from RFC 8949 major type 0, not measured by running the example)
- Seen by: perfapi; refutation: confirmed (severity lowered to low: the bandwidth row reports true wire bytes); history: deliberate but expired (`Vec<u8>` was chosen under Borsh, which wrote it as a blob; f2b74a97 switched the codec and acb556fc rewrote the doc without changing the type)
- Owner-gated: no

serde serializes `Vec<u8>` element by element, so ciborium emits a CBOR array of unsigned integers: one byte for values 0..=23 and two for 24..=255, about 1.9 wire bytes per uniformly random payload byte, plus a per-element decode on every receive and on every send's self-decode admission check, and the cached `Message` bytes inflated likewise. The doc's "small per-element constant" is literally true (about 0.9 bytes per element) and hides a near-doubling relative to the payload it prices. `bytes::Bytes`, already a regular dependency with the `serde` feature and the type the crate's own wire tests use for exactly this reason, encodes as a byte string. I hold this at medium rather than the refutation's low because the example is the crate's showcase and its documented measurement tool, and the crate docs' payload-type section says nothing about byte payloads, so this is the one place a user learns the idiom.

Evidence:

    185	/// Message payload type: opaque, randomized bytes. CBOR encodes `Vec<u8>`
    186	/// as an integer array, so the wire cost tracks the message size (within
    187	/// a small per-element constant).
    188	type Payload = Vec<u8>;

    ciborium-0.2.2 src/ser/mod.rs
    165	    fn serialize_bytes(self, v: &[u8]) -> Result<(), Self::Error> {
    166	        self.0.push(Header::Bytes(v.len().into()))?;
    226	    fn serialize_seq(self, length: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
    227	        self.0.push(Header::Array(length))?;

    tests/dispute_wire.rs
    33	//! Payload corpora are [`bytes::Bytes`], which serde carries as a CBOR
    34	//! byte string: a fixed-length payload has one deterministic encoded

Resolution: `type Payload = bytes::Bytes;` with `random_message` returning `Bytes::from(buf)`; restate the doc: a byte string costs its length plus a one-to-three-byte head. `TEST_MESSAGE_SIZE` and the controller test are unaffected. Acceptance: the `Payload` doc no longer says "integer array" or "per-element"; with `--headless-secs 5 --message-size 256`, wire bytes per learned message fall from roughly 490 plus framing to roughly 259 plus framing (observable once swarm-example-18 surfaces `messages_gained`, or from a one-off `SessionStats::bytes_sent` probe).

### swarm-example-6: Private docs and comments describe state the code does not have
- Where: examples/swarm.rs:258-260 (related: examples/swarm.rs:264, examples/swarm.rs:291-294, examples/swarm.rs:171, examples/swarm.rs:1023, examples/swarm.rs:1399-1410, examples/swarm.rs:355-358, examples/swarm.rs:363, examples/swarm.rs:372, examples/swarm.rs:377-378, examples/swarm.rs:575, examples/swarm.rs:661-663, examples/swarm.rs:678, examples/swarm.rs:709, examples/swarm.rs:717, examples/swarm.rs:907-908)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (every `compare_exchange` in the file is at 646, 675, 695 by grep; `inflight` is absent from `Snapshot::take` at 1399-1410 and moves both ways at 342/349; `bootstrap_fork` and `shrink` build raw `memory_with_capacity` pairs at 171 and 1023; the remaining items by reading)
- Seen by: prose (four candidates merged here); refutation: all four confirmed, two lowered to nit; history: no rationale found for any (the `Metrics` claim was false when d987a2be wrote it; the `SwarmPeer` list omitted `id` and `control` from e8442e6a; the `try_initiate` arms were added by d987a2be without a doc update; the "claim race" comment never had a referent)
- Owner-gated: no

Five sites state something the code does not do, the Principle 5 failure in its plainest form. `Metrics`' doc says the counters are "sampled by the UI" and "All monotonic since start except `sync_nanos_best`", but `inflight` goes up and down and is read by shutdown, not the UI; `wire_bytes` says "every wire" while the bootstrap and retire links are undecorated, so only sync-session wires are tallied (the readout stays self-consistent because `wire_direction_nanos` covers the same sessions; the doc's denominator is what is wrong). `SwarmPeer`'s doc enumerates "only" three of its five fields. `Command`'s doc says the reply is "for the party to hand its [`Rumors`] back" while `Fork` hands back a child's. `try_initiate`'s doc names two `false` arms of five (already engaged, shutdown begun, and peer thread gone are omitted). The coordinator's balanced-branch comment names "the loser of a claim race" as a trigger, but the coordinator performs no claims and in that branch only the `parties` knob can change either count.

Evidence:

    258	/// Process-wide counters, sampled by the UI. All monotonic since start except
    259	/// `sync_nanos_best`; the UI differences successive snapshots to get windowed
    260	/// rates.
    264	    /// Total bytes written to every wire — control stream and every data
    355	/// Holds no rumor-set
    356	/// data — only the inbox to deliver session endpoints, the engaged flag that
    357	/// serializes each party into one session at a time, and a gauge of its
    358	/// current live-message count for the UI.
    377	/// A membership command sent by the coordinator to a single party. Each carries
    378	/// a one-shot reply channel for the party to hand its [`Rumors`] back.
    661	/// Attempt to initiate a sync with a random other party. Returns `true` if a
    662	/// session actually ran (both claims succeeded), `false` if the peer was busy
    663	/// or there was no one to pick.
    907	            // Balanced: nothing to do until the knob or the loser of a claim
    908	            // race changes things. Poll at a human-noticeable cadence.

Resolution: `Metrics`: "Process-wide counters. The rate counters are monotonic since start and the UI differences successive snapshots; `sync_nanos_best` is a windowed minimum and `inflight` a gauge that shutdown drains." `wire_bytes`: "Total bytes written on every sync session's wire (the bootstrap and retire links are not decorated) ...". `SwarmPeer`: state the role without the list. `Command`: "a [`Rumors`] back: its own on wind-down, a fresh child's on fork." `try_initiate`: "Returns `true` iff a session ran; `false` when this party is already engaged, no other party is live, the peer is engaged, shutdown has begun, or the peer's thread has exited." Coordinator: "Balanced: only the parties knob can change this. Poll at a human-noticeable cadence." Acceptance: each doc names only properties its fields have; each `return false` in `try_initiate` has a clause; no reference to claims remains in `run_coordinator`.

### swarm-example-7: Doc paragraphs left ragged by a mechanical summary split
- Where: examples/swarm.rs:282-283 (related: examples/swarm.rs:330-331, examples/swarm.rs:355-356, examples/swarm.rs:515-516, examples/swarm.rs:626-627, examples/swarm.rs:1155-1156; milder: examples/swarm.rs:399-400, examples/swarm.rs:877-878, examples/swarm.rs:1197-1198)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (awk over the file: lines 282 (29 chars), 330 (19), 355 (22), 515 (13), 626 (28), 1155 (34) are each followed by a full-width continuation; 1183-1184 is a two-line paragraph (58/50) and does not fit; `git blame` per the history pass attributes the short lines to dfd19c44 and the continuations to earlier commits)
- Seen by: structure, prose; refutation: confirmed (site list corrected); history: the split is deliberate and holds (`tools/doclint` rule 1, summary under `MAX_SUMMARY_CHARS`); the un-reflowed wrap is residue
- Owner-gated: no

A blank `///` was inserted after each first sentence to satisfy doclint's summary rule without re-wrapping the paragraph that follows, leaving a two-to-five-word line hanging above a full-width one at six sites (three more mildly). Rendered rustdoc is unaffected; the source is what a maintainer reads, and these sit on the concurrency invariants.

Evidence:

    282	    /// The sampler swaps the
    283	    /// sentinel back in each time it reads, so the value is windowed where
    330	/// Shutdown drains
    331	/// the in-flight counter to zero before letting threads exit, so the slot
    515	/// In-memory
    516	/// sessions complete in milliseconds even on a heavily loaded machine, so

Resolution: Re-wrap the second paragraph at each site to fill the column, keeping the blank `///` doclint requires. Acceptance: no `///` paragraph in the file opens with a line under roughly half the wrap width followed by a full-width continuation.

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

### swarm-example-9: Current-thread runtime construction is spelled out four times
- Where: examples/swarm.rs:424-426 (related: examples/swarm.rs:541-543, examples/swarm.rs:885-887, examples/swarm/tests.rs:82-84)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (grep `new_current_thread` lists exactly swarm.rs 424, 541, 885 and tests.rs 82)
- Seen by: structure; refutation: confirmed; history: no rationale found
- Owner-gated: no

The three-line `Builder::new_current_thread().build().expect(..)` appears in `main`, `run_party`, `run_coordinator`, and the test, differing only in the expect string. The flavor choice (both ends of a link share one thread) is a design decision stated in the module doc at 54-56 and nowhere near the constructions.

Evidence:

    424	    let seed_runtime = tokio::runtime::Builder::new_current_thread()
    425	        .build()
    426	        .expect("build seed runtime");

Resolution: `fn current_thread_runtime() -> Runtime` with the flavor rationale in its doc; call it at 424, 541, 885 and tests.rs:82. Acceptance: `new_current_thread()` appears once in the partition.

### swarm-example-10: `expect` messages are labels rather than proofs, and one overclaims "any depth limit"
- Where: examples/swarm.rs:431-431 (related: examples/swarm.rs:178-181, examples/swarm.rs:839, examples/swarm/tests.rs:65-66, examples/swarm/tests.rs:92, src/message.rs:59, src/message.rs:77-79)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`PayloadDepthLimit::new(steps: u64)` at src/message.rs:77-79 has no floor; `DEFAULT_PAYLOAD_DEPTH_LIMIT` is 256 at :59; the refutation pass read ciborium's recursion accounting to confirm a flat array is rejected under a zero limit)
- Seen by: prose; refutation: confirmed; history: no rationale found (ce27df86 wrote the string into both files; 212c6914 carried it through the `send_all` port)
- Owner-gated: no

Every `expect` message is a one-line proof of why it cannot fire, and the proof should be true as stated. `main`'s and the test's seed insert say "flat test payloads are within any depth limit": the swarm's are not test payloads, and `PayloadDepthLimit::new(0)` is constructible, under which a flat array is rejected; the true reason is that the swarm's `Peer::seed()` uses the default 256-step limit. `bootstrap_fork`'s three expects ("serve bootstrap", "bootstrap newcomer", "provider served bootstrap") and the test's ("gossip a", "gossip b") are bare labels with no pointer to the one place (736-742) that argues why the example panics.

Evidence:

    431	            .expect("flat test payloads are within any depth limit");
    178	    served.expect("serve bootstrap");

    src/message.rs
    77	    pub const fn new(steps: u64) -> Self {
    78	        PayloadDepthLimit(steps)

Resolution: In `main` and the test: "a flat byte array is one step deep, far inside the default depth limit". In `bootstrap_fork` and `reconcile`: name the argument ("in-process bootstrap in one universe: a failure is a bug") or move it into `bootstrap_fork`'s doc and have the expects cite it. Acceptance: no expect message in the partition claims "any" limit; each states its own proof or names the function whose comment does.

### swarm-example-11: Em-dashes in `//` comments
- Where: examples/swarm.rs:487-487 (related: examples/swarm.rs:558, examples/swarm.rs:600, examples/swarm.rs:720-721, examples/swarm.rs:739, examples/swarm.rs:835, examples/swarm/tests.rs:125-126)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -nE '^\s*//[^/!].*—'` over both files lists exactly these lines)
- Seen by: prose; refutation: confirmed; history: no rationale found; the nearest ruling (R76, colons in assert messages) does not cover `//` comments, and the pattern is workspace-wide (160 sites under src/, tests/, benches/ per the history pass)
- Owner-gated: no

The owner's doctrine keeps typographic em-dashes for rendered prose (`///`, `//!`) and uses a colon, semicolon, or spaced double-hyphen in code comments. Seven `//` comments in the partition use em-dashes; they are the local face of a workspace pattern, so the durable fix is mechanical rather than seven edits.

Evidence:

    487	    // session is wedged, and joining its thread would hang forever — report
    558	        // last turn — the sessions just served included — and republish our

Resolution: Recast each as a colon, semicolon, or parenthetical, or ` -- ` where a dash is wanted. Since the pattern spans the workspace, consider a `tools/doclint`-style rule for em-dashes in non-doc comments so the sweep is done once (see the open question). Acceptance: the grep returns nothing for the partition.

### swarm-example-12: Constants restated by hand: "five seconds" beside `from_secs(5)`; the test copies the clap default literal
- Where: examples/swarm.rs:512-518 (related: examples/swarm.rs:224-228, examples/swarm/tests.rs:15-16)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`grep '16 \* 1024'` over both files finds exactly swarm.rs:227 and tests.rs:16)
- Seen by: structure, prose; refutation: confirmed; history: no rationale found (all d987a2be)
- Owner-gated: no

`SHUTDOWN_DRAIN_DEADLINE`'s doc spells out "five seconds", a copy of the value beside it that rots on the next edit. `TEST_DUPLEX_CAPACITY` is `16 * 1024` "matching the swarm default", a second copy of `Args::duplex_capacity`'s `default_value_t` with nothing tying them: a default change in the example leaves the test measuring a different link. Named constants over magic numbers; a number that matters lives in one mechanically shared place.

Evidence:

    515	/// In-memory
    516	/// sessions complete in milliseconds even on a heavily loaded machine, so
    517	/// five seconds is generous.
    518	const SHUTDOWN_DRAIN_DEADLINE: Duration = Duration::from_secs(5);
    227	    #[arg(long, default_value_t = 16 * 1024)]

    examples/swarm/tests.rs
    15	/// In-memory stream capacity for the test links, matching the swarm default.
    16	const TEST_DUPLEX_CAPACITY: usize = 16 * 1024;

Resolution: Doc: "... so a deadline three orders of magnitude above that is generous." Introduce `const DEFAULT_DUPLEX_CAPACITY: usize = 16 * 1024;` in swarm.rs with the `Args` rationale, use it in `default_value_t`, and have the test use it via `super::*`; `TEST_DUPLEX_CAPACITY` disappears. Acceptance: the deadline doc contains no numeral; `16 * 1024` appears once in the partition, at a named constant.

### swarm-example-13: `drain_versions` hand-rolls `UnorderedMessages::try_next` and clones an owned `Version`
- Where: examples/swarm.rs:618-622 (related: examples/swarm.rs:137, src/rumors/unordered.rs:167-182, src/rumors/unordered.rs:191-192, src/lib.rs:346)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (`try_next` read at src/rumors/unordered.rs:175-182; `TryNext` re-exported at src/lib.rs:346; `Stream::Item = (Version, Arc<T>)` at :192; grep shows line 619 is the only use of `FutureExt`/`StreamExt` in the file)
- Seen by: correctness, perfapi (clone); refutation: confirmed; history: deliberate but expired (the `now_or_never` spelling predates `try_next`, cbfc1571; the clone was written against the lending `borrow_next`, which cd7c09db replaced with the owned `next()`)
- Owner-gated: no

The crate exposes `try_next` as its non-blocking step ("One `Stream` poll with a no-op waker, rendered as the trichotomy"), which is exactly what `observer.next().now_or_never()` spells here, and the example is the place users learn the API. The `.clone()` bumps and immediately drops a refcount on a value the pattern already owns, and makes a reader look for a borrow that is not there.

Evidence:

    618	fn drain_versions(observer: &mut UnorderedMessages<Payload>, pool: &mut Vec<Version>) {
    619	    while let Some(Some((version, _))) = observer.next().now_or_never() {
    620	        pool.push(version.clone());
    621	    }
    622	}

    src/rumors/unordered.rs
    175	    pub fn try_next(&mut self) -> TryNext<T> {
    176	        use futures::{FutureExt, StreamExt};
    177	        match self.next().now_or_never() {
    192	    type Item = (Version, Arc<T>);

Resolution: `while let TryNext::Message((version, _)) = observer.try_next() { pool.push(version); }`, importing `rumors::TryNext`; the `futures` import at 137 then goes. Acceptance: no `now_or_never` or `.clone()` in `drain_versions`; the loop reads the trichotomy by name; `just clippy` stays clean.

### swarm-example-14: `wind_down`'s post-claim straggler drain is unreachable by the function's own argument
- Where: examples/swarm.rs:654-657 (related: examples/swarm.rs:624-631, examples/swarm.rs:640-650, examples/swarm.rs:693-718, examples/swarm.rs:554, examples/swarm.rs:642, examples/swarm.rs:749)
- Class / severity / confidence: vestigial / low / high
- Provenance: assessed (read; the protocol trace is in the claim)
- Seen by: structure, prose, correctness; refutation: confirmed (three candidates, one finding); history: no rationale found (the loop and "for good measure" are original to e8442e6a, under the same claim-before-send protocol)
- Owner-gated: no

An initiator pushes to a responder's inbox only after winning the compare-and-swap on that responder's `engaged` flag, and never clears the peer's flag after a successful send (749 clears only its own); the flag stays set until the responder serves and clears it (554, 642). A successful CAS in `wind_down` therefore reads a flag no initiator holds, so the inbox is empty and, the flag now permanently set, stays empty. The loop cannot execute, its comment concedes it ("nothing new can arrive"), and "for good measure" names no failure it catches: the circular-justification tell. Dead handling code also weakens a reader's trust in the proof above it.

Evidence:

    654	    // Locked: nothing new can arrive. Drain any straggler for good measure.
    655	    while let Ok(end) = inbox.try_recv() {
    656	        serve_sync(runtime, net, &rumors, end);
    657	    }
    628	/// new session with us. Because an initiator sets a peer's flag *before*
    629	/// delivering the session, a successful claim here proves nothing is owed; if

Resolution: Delete the loop and its comment (the doc at 626-631 carries the argument); or, if a check is wanted for a future path that pushes to an inbox without claiming, `debug_assert!(inbox.try_recv().is_err(), "an initiator delivered a session without claiming this party")`, which names the one mistake it would catch. Acceptance: no serve path exists after the successful CAS in `wind_down`, or the check that replaces it names what it catches.

### swarm-example-15: The engaged-flag release is hand-written on every exit of `try_initiate`
- Where: examples/swarm.rs:684-718 (related: examples/swarm.rs:328-351, examples/swarm.rs:552-555, examples/swarm.rs:640-643, examples/swarm.rs:749)
- Class / severity / confidence: simplification / nit / medium
- Provenance: assessed (read)
- Seen by: structure; refutation: reframed (the panic-safety half does not carry: a panicked initiator's thread is dead and later claimants back off by design; what remains is a legibility proposal); history: no rationale found (`InflightGuard` was added for R34's in-flight slot; no note considers a guard for the flags)
- Owner-gated: no

`try_initiate` releases `me.engaged` at five sites and `peer.engaged` at three, and the correctness of the rendezvous rests on each exit releasing exactly the flags it claimed, checked today by reading all five. A small `Claim<'a>(&'a AtomicBool)` guard (try-acquire by CAS, store `false` on drop, an explicit `transfer()` that forgets it) would make the release structural the way `InflightGuard` does one line below, and would put the one non-obvious step, handing the peer's release to the responder after a successful `send`, where the reader looks. A design proposal with no robustness payoff; the cost is reading five sites.

Evidence:

    706	    if !net.running.load(Ordering::SeqCst) {
    707	        peer.engaged.store(false, Ordering::Release);
    708	        me.engaged.store(false, Ordering::Release);
    709	        return false;
    710	    }
    714	    if peer.inbox.send(theirs).is_err() {
    715	        peer.engaged.store(false, Ordering::Release);
    716	        me.engaged.store(false, Ordering::Release);
    717	        return false;
    718	    }

Resolution: Add `Claim` with `try_acquire(&AtomicBool) -> Option<Claim>`, `Drop` storing `false`, and `transfer(self)` forgetting it; `try_initiate` becomes a chain of `?`-style early returns with one `theirs.transfer()` after the successful `inbox.send`. The responder's serve-then-store pairs (552-555, 640-643) can hold the same type across the serve. Acceptance: `try_initiate` contains no `engaged.store(false, ..)`; the only explicit ownership transfer is the peer claim after a successful send.

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

### swarm-example-17: The same rationale stated twice at three points
- Where: examples/swarm.rs:730-734 (related: examples/swarm.rs:754-755, examples/swarm.rs:268-269, examples/swarm.rs:1085-1087, examples/swarm.rs:387-389, examples/swarm.rs:529-533, examples/swarm.rs:571-572, examples/swarm.rs:1014-1015)
- Class / severity / confidence: documentation / nit / medium
- Provenance: assessed (read)
- Seen by: prose; refutation: confirmed with a caveat (not every restatement should go; the clear cuts are the parenthetical at 733-734 and the vaguer `Arc` rationale at 268-269); history: no rationale found (each pair written in one pass by one commit)
- Owner-gated: no

Two explanations appear twice so that an edit to one drifts from the other: the parenthetical "(Learned messages surface through the party's observer on its next drain.)" at 733-734 and 754-755, where in `try_initiate` it interrupts a comment about latency and `serve_sync`'s doc already carries it; and the `Arc`-not-borrow rationale for `wire_bytes` at 268-269 (vague) and 1085-1087 (precise, naming the `'static` bound on `Connector::Tx`). The observer-replay pool-rebuild mechanism is stated at four sites (387-389, 529-533, 571-572, 1014-1015); the module doc and `Donation`'s doc are the two that should stay.

Evidence:

    733	    // (Learned messages surface through the party's observer on its next
    734	    // drain.)
    754	/// (Learned messages surface through the party's observer on its next
    755	/// drain.)
    268	    /// Shared into the link-decorating writers, which outlive
    269	    /// the borrow of `Metrics`, so it is an [`Arc`] rather than a bare field.
    1085	/// The counter is owned as an [`Arc`] rather than borrowed so this wrapper
    1086	/// can back a [`Connector::Tx`], whose `'static` bound outlives any borrow of
    1087	/// the metrics.

Resolution: Delete the parenthetical at 733-734; replace 268-269 with a pointer to `CountWrite`'s doc ("an [`Arc`]: see [`CountWrite`]"); trim the pool-rebuild sentence at 571-572 and 1014-1015 to a pointer at `Donation`. Acceptance: each listed rationale has one full statement in the file; the other sites link to it or are gone.

### swarm-example-18: The example discards `Gossiped` and never surfaces `SessionStats`, the crate's own per-session measurement
- Where: examples/swarm.rs:743-745 (related: examples/swarm.rs:769-771, examples/swarm.rs:95-112, examples/swarm.rs:121-123, src/peer/gossip.rs:167-180, src/tree/mirror/streaming/stats.rs:39-152)
- Class / severity / confidence: feature-gap / low / high
- Provenance: verified (grep finds no `Gossiped` or `SessionStats` in either partition file; `Gossiped.stats: SessionStats` read at src/peer/gossip.rs:174-179)
- Seen by: perfapi; refutation: confirmed; history: no rationale found (`SessionStats` landed in 487e17ea one day after the readout work and touched no example; not yet ported rather than declined)
- Owner-gated: no

Both `gossip` calls drop the returned `Gossiped`, so the crate's `SessionStats` (`disputed_scopes`, `messages_gained`, `messages_shed`, `bytes_sent`/`bytes_received` at the codec seam, `window_granted`) never reaches the readout. The example instead re-derives a transport-level byte count with about 120 lines of decorators and reports nothing that explains the bandwidth (how many messages moved, how many scopes were disputed, how wide the window ran). As the crate's showcase and its documented self-measurement tool, it teaches a reader nothing about `Gossiped`; and reporting only what it measured itself while ignoring what the library measured is the verified-versus-told asymmetry in miniature.

Evidence:

    743	    runtime
    744	        .block_on(rumors.gossip(&mut link))
    745	        .expect("initiator gossip");

    src/peer/gossip.rs
    174	    /// What the session measured about itself; every count is local, so
    179	    pub stats: SessionStats,

Resolution: Bind the `Gossiped` at both calls and fold `stats.messages_gained`, `messages_shed`, `disputed_scopes`, `bytes_sent`, `bytes_received` into `Metrics` as monotonic `AtomicU64`s; add rows (UI and headless) for messages gained/shed per sync and disputed scopes per sync; label the existing row "wire bytes (transport)" beside a "reconciliation bytes (codec)" row so the envelope overhead is visible; update the module doc's readout list. Pairs naturally with swarm-example-16 option (a). Acceptance: `--headless-secs` prints the new rows; `# The readout` lists exactly the rows printed; `grep -n Gossiped examples/swarm.rs` finds a binding.

### swarm-example-19: `steady_state_op` clamps a probability the types already bound, and its docs omit the zero-target case
- Where: examples/swarm.rs:812-820 (related: examples/swarm.rs:40, examples/swarm.rs:789-792, examples/swarm.rs:835-839, examples/swarm.rs:216-217, examples/swarm.rs:1366)
- Class / severity / confidence: simplification / nit / high
- Provenance: assessed (read)
- Seen by: structure (clamp); refutation: confirmed, with an overflow caveat on the proposed integer sum, and the zero-target doc gap raised as new; history: no rationale found (clamp original to e8442e6a)
- Owner-gated: no

`target` arrives as `u64`, so after `as f64` it is non-negative and `target <= 0.0` is exactly `target == 0`; for `target > 0` and `live >= 0` the ratio lies in `(0, 1]` and cannot be NaN, so `clamp(0.0, 1.0)` catches nothing constructible. A guard must name a constructible failure. Separately, the module doc's "At `L = 0` it always adds" and the function doc's "1.0 when empty" hold only for a positive target: at `target == 0` (reachable, since the UI floors `target` at 0 and `--target 0` is accepted) the guard yields 0, the op redacts until the pool has no live entry, then the fallback inserts one, so live oscillates between 0 and 1. The behavior is fine and the fallback is documented at 794; the two "always adds" sentences need "for a positive target".

Evidence:

    812	    let target = target as f64;
    813	    let live = snap.len() as f64;
    814	    let p_add = if target <= 0.0 {
    815	        0.0
    816	    } else {
    817	        target / (target + live)
    818	    };
    819	
    820	    if !rng.gen_bool(p_add.clamp(0.0, 1.0)) {
    40	//! At `L = 0` it always adds; at `L = T` the odds are even; as `L` grows past
    790	/// `target / (target + live)` (1.0 when empty, 0.5 at target, → 0 far over),

Resolution: Test the zero case on the integer and compute the ratio in `f64` (an integer sum can overflow for a CLI `--target` near `u64::MAX`): `let p_add = if target == 0 { 0.0 } else { target as f64 / (target as f64 + snap.len() as f64) };` and `rng.gen_bool(p_add)`. Add "for a positive target" at 40 and 790 (or state the fixed point as `max(target, 1)`). Acceptance: no `clamp` in `steady_state_op`; the zero branch tests the integer; the docs' "always adds" claims are qualified.

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

### swarm-example-21: "key pool" is a ghost of the retired `Key` vocabulary
- Where: examples/swarm.rs:1014-1015 (related: examples/swarm.rs:387-389, examples/swarm.rs:529, examples/swarm.rs:572)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`git blame -L 1014,1015` attributes the comment to 45835294 (2026-06-10); 9c73d7b4 (2026-08-18, "api: retire Key; a message's public identity is its Version") swept "key pool" at four other sites in this file and missed this one; grep finds no other pool-sense "key" in the file)
- Seen by: prose; refutation: confirmed; history: contradicts the hard rule
- Owner-gated: no

The shrink comment calls the redaction pool the "key pool". The crate retired `Key`; every other site says "version pool", "redaction pool", or "pool". AGENTS.md's first hard rule: nothing in the codebase refers to code that no longer exists.

Evidence:

    1014	    // Merge: retire b into a over an in-memory wire. The survivor's key pool
    1015	    // is rebuilt by observer replay in its new thread, so nothing but the

Resolution: "The survivor's version pool is rebuilt by observer replay ...". Acceptance: `grep -n 'key pool' examples/` returns nothing.

### swarm-example-22: `CountConnector::connect` discards the inner `Done` instead of forwarding completion
- Where: examples/swarm.rs:1166-1176 (related: src/link.rs:55-63, src/link.rs:171-199, src/link.rs:598-605, src/link/routed/stream.rs:55, src/link/routed/router.rs:229)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (`MemoryConnector::connect` returns `Done::discard()` at src/link.rs:604, so the swarm is behavior-preserving; `Done` is `Box<dyn FnOnce(Half) + Send>` at :179 and `Done::new` takes `impl FnOnce + Send + 'static` at :191, so the forwarding form fits `Connector::connect`'s `Send` future; the routed link's recycling `Done`s per the perfapi lens's grep)
- Seen by: correctness, perfapi; refutation: confirmed (severity raised to low to match); history: deliberate and holds for the memory link (4be4b830: "plain transports pair with Done::discard()"), silent on wrappers
- Owner-gated: no

The wrapper drops the connector's `Done` and hands out a fresh `Done::discard()`. That is behavior-preserving for `MemoryConnector`, whose own `Done` is `discard`, but the link contract's completion clause makes `Done` where "A transport that reuses connections recovers them", and this is the crate's only worked decoration of a `Link`. A reader copying it onto the routed link (which recycles through `Done`) would sever the transport's stream recovery at every clean end. Examples teach patterns; forwarding is the general form and discarding a coincidence for the in-memory link.

Evidence:

    1166	    async fn connect(&self) -> io::Result<(Self::Tx, Done<Self::Tx>)> {
    1167	        let (tx, _) = self.inner.connect().await?;
    1174	            Done::discard(),

    src/link.rs
    176	/// anywhere else (its handle unused) is an abort. A transport that
    177	/// reuses connections recovers them here; the rest pair every stream
    178	/// with [`discard`](Self::discard).
    604	        Ok((tx, Done::discard()))

Resolution: `let (tx, done) = self.inner.connect().await?;` and return `Done::new(move |w: CountWrite<DuplexStream>| done.complete(w.inner))`, with one comment line: a wrapper forwards the half's clean end to the transport it wraps. Or state inline why discarding is right here (the memory link's `Done` is itself `discard`) if the general form is not wanted in the example. Acceptance: either no `Done::discard()` remains in the example, or its one use carries the stated reason.

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

### swarm-example-24: Bool-typed parameters in `Field::adjust` and `bump` make call sites unreadable without the signature
- Where: examples/swarm.rs:1353-1353 (related: examples/swarm.rs:1377-1385, examples/swarm.rs:1564-1565)
- Class / severity / confidence: idiom / nit / medium
- Provenance: assessed (read)
- Seen by: structure; refutation: confirmed (a two-variant enum reads better than a signed step here, since `bump` works on `AtomicU64` with bounds to 5,000,000 and an `i64` round-trip adds casts); history: no rationale found
- Owner-gated: no

`adjust(&net.controls, false, coarse)` requires the reader to know that the second positional bool is "increase". Types-first: a bool literal at a call site carries no meaning.

Evidence:

    1353	    fn adjust(self, controls: &Controls, increase: bool, coarse: bool) {
    1377	fn bump(value: &AtomicU64, increase: bool, step: u64, min: u64, max: u64) {
    1564	                KeyCode::Left => Field::ALL[selected].adjust(&net.controls, false, coarse),
    1565	                KeyCode::Right => Field::ALL[selected].adjust(&net.controls, true, coarse),

Resolution: A two-variant `Direction { Down, Up }` on `adjust` and `bump`, with `Left => adjust(Direction::Down, coarse)`. Acceptance: the two `adjust` call sites read their direction without the signature.

### swarm-example-25: The local `Snapshot` collides with `rumors::Snapshot`, and several items are spelled by long path at every use
- Where: examples/swarm.rs:1388-1388 (related: examples/swarm.rs:780, examples/swarm.rs:807, examples/swarm.rs:152, examples/swarm.rs:563, examples/swarm.rs:1546; qualified paths: examples/swarm.rs:167, 424, 541, 633, 665, 757, 885, 983 (`tokio::runtime::`), 1594, 1620, 1644, 1672, 1716, 1752 (`ratatui::Frame`), 1053 (`std::sync::Mutex`), 1514-1515 (`std::panic::`), 171, 713, 1023 (`rumors::link::memory_with_capacity`); examples/swarm/tests.rs:62-63, 82)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep for each qualified path; the site list is the grep output)
- Seen by: structure, perfapi; refutation: confirmed; history: no rationale found (the local struct is original; the `rumors::Snapshot` parameter arrived with d987a2be; the collision is accidental)
- Owner-gated: no

The counters struct is named `Snapshot`, so the crate's type must be written `rumors::Snapshot<Payload>` at 780 and 807 while every other `rumors` item is imported, and `rumors.snapshot()` at 563 and `Snapshot::take(net)` at 1546 return different types under one name. The same pattern (a qualified path where an import would do and the qualification informs nothing) recurs for `tokio::runtime::{Runtime, Builder}`, `ratatui::Frame` beside imported `ratatui::Terminal`, `std::sync::Mutex`, `std::panic::{take_hook, set_hook}`, and `rumors::link::memory_with_capacity` whose siblings are imported at 151.

Evidence:

    1388	struct Snapshot {
    780	    snap: &rumors::Snapshot<Payload>,
    1594	    frame: &mut ratatui::Frame,
    167	    runtime: &tokio::runtime::Runtime,
    1514	    let default_hook = std::panic::take_hook();

Resolution: Rename the counters struct (`Counters` or `Sample`, matching its doc "a point-in-time read of the counters") and import `rumors::Snapshot`; add `Frame` to the ratatui import, `Runtime`/`Builder` from `tokio::runtime`, `memory_with_capacity` to the `rumors::link` import, and `panic::{set_hook, take_hook}` (the `panic::` prefix is the informing form). Acceptance: no `rumors::Snapshot`, `ratatui::Frame`, or `tokio::runtime::` path appears at a use site in the partition.

### swarm-example-26: `compute` hides a consuming side effect inside an otherwise pure function
- Where: examples/swarm.rs:1464-1469 (related: examples/swarm.rs:1398-1411, examples/swarm.rs:1546-1547, examples/swarm.rs:463-479)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (read)
- Seen by: structure; refutation: confirmed, with a correction (parties start before the UI, so the zero-width startup call at 1547 can consume a real best; it is harmless because its result is only the display until the first sample, not because nothing has run); history: no rationale found
- Owner-gated: no

`compute(net, prev, now)` reads as a pure function of two snapshots and the live gauges, but it also swaps the `sync_nanos_best` sentinel, so its call count matters: a second call in one window loses the window's best. `Snapshot::take` is already the "read the counters now" operation; a read that re-arms a windowed minimum belongs there, leaving `compute` pure and the startup call obviously safe. Finished code should be evidently correct, and a function named `compute` whose call count matters is not.

Evidence:

    1464	    // Consume the window's fastest session and re-arm the sentinel for the
    1465	    // next window.
    1466	    let latency_best_nanos = net
    1467	        .metrics
    1468	        .sync_nanos_best
    1469	        .swap(u64::MAX, Ordering::Relaxed);
    1547	    let mut stats = compute(net, &prev, &Snapshot::take(net));

Resolution: Move the swap into `Snapshot::take` as a `sync_nanos_best` field with a doc line stating that taking a snapshot consumes the window's best; `compute` reads `now.sync_nanos_best`. Acceptance: `compute` performs no atomic stores or swaps; `Snapshot` documents the consuming read.

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

### swarm-example-28: `mod tests` is declared between two formatting helpers, and the type aliases follow their first use
- Where: examples/swarm.rs:1781-1786 (related: examples/swarm.rs:1766-1779, examples/swarm.rs:1788-1800, examples/swarm.rs:166-170, examples/swarm.rs:185-193)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (the block sits between `format_rate` (1766-1779) and `format_duration` (1788-1800) by reading; `Payload` is used at 168 and 170 and defined at 188)
- Seen by: structure, prose; refutation: confirmed; history: no rationale found (d987a2be inserted the block between the two adjacent helpers; f612b471 inserted `bootstrap_fork` above the pre-existing aliases)
- Owner-gated: no

The `#[path]`-annotated `mod tests;` (whose explanatory comment is good and deserves a findable home) splits two sibling adaptive-unit formatters, and `Payload` and `SessionEnd` are defined after `bootstrap_fork` uses `Payload`, so a top-down reader meets the alias before its doc.

Evidence:

    1781	// The example file is its own crate root, so the module path is stated
    1786	mod tests;
    1788	/// Format a short duration with an adaptive unit (ns / µs / ms / s).
    166	fn bootstrap_fork(
    168	    parent: &Rumors<Payload>,
    188	type Payload = Vec<u8>;

Resolution: Move the six-line `mod tests` block to the end of the file (or beside the `use` block); move the two type aliases above `bootstrap_fork`. Acceptance: `format_rate` and `format_duration` are adjacent; `Payload` is defined before its first use.

### swarm-example-29: The test re-implements the party loop's turn instead of sharing it
- Where: examples/swarm/tests.rs:38-53 (related: examples/swarm.rs:557-564, examples/swarm.rs:611)
- Class / severity / confidence: verification-gap / low / high
- Provenance: assessed (read; the history pass's `git show d987a2be` shows that commit reordering the drain/snapshot in `run_party`, the class of edit the copy cannot follow)
- Seen by: correctness; refutation: confirmed; history: no rationale found
- Owner-gated: no

`Party::churn` hand-copies the sequence `run_party` performs (drain observer, snapshot, controller op), and its doc asserts the copy matches "exactly as the party loop does". Nothing enforces it: if `run_party` later snapshots before draining, the test keeps passing against its own copy while the swarm's controller reads stale liveness. A hand-maintained mirror of a sequence is the same failure as a hand-maintained count.

Evidence:

    38	    /// Run `ops` controller operations at `target`, draining the observer
    39	    /// and snapshotting before each op exactly as the party loop does.
    40	    fn churn(&mut self, target: u64, ops: usize) {
    41	        for _ in 0..ops {
    42	            drain_versions(&mut self.observer, &mut self.pool);
    43	            let snap = self.rumors.snapshot();

    examples/swarm.rs
    562	        drain_versions(&mut observer, &mut pool);
    563	        let snap = rumors.snapshot();

Resolution: Extract the observation step into one function both sites call, e.g. `fn observe(observer, pool, rumors) -> rumors::Snapshot<Payload>` performing drain-then-snapshot; `run_party` calls it at 562-563 and `churn` before `steady_state_op`. The testdoc's "exactly as the party loop does" then holds by construction. Acceptance: `run_party` and the test share the drain/snapshot path; reordering it in one place reorders it in both.
Construction: swap lines 562 and 563 in `run_party`; `controller_converges_through_retargeting` still passes, and nothing else fails.

### swarm-example-30: Testdoc says "live count"; the body judges a four-sample post-ring mean
- Where: examples/swarm/tests.rs:69-73 (related: examples/swarm/tests.rs:121-128, examples/swarm/tests.rs:141-142)
- Class / severity / confidence: test-quality / nit / high
- Provenance: assessed (read)
- Seen by: prose; refutation: confirmed; history: no rationale found (d800957e changed the judgment and added the body rationale without touching the doc)
- Owner-gated: no

The test's doc comment, its statement of record, says each party's "live count" must land in band; the body asserts on `sum / SAMPLES`, the mean over four churn rounds each settled by a full ring pass, and the body comment explains why a single sample would put the band's edge inside the oscillation. An inaccurate testdoc is a bug in the test.

Evidence:

    71	/// Driving three gossiping parties through a target drop and a target raise
    72	/// must land each party's live count within half-to-double of every phase's
    141	        for sum in settled {
    142	            let live = sum / SAMPLES as u64;

Resolution: "... must land each party's mean live count, over four churn rounds each settled by a full ring pass, within half-to-double of every phase's target ...". Acceptance: the doc names the averaging the assertion performs.

### swarm-example-31: The controller test's trajectory is coupled to `before`'s version encoding through pool order
- Where: examples/swarm/tests.rs:141-147 (related: examples/swarm/tests.rs:1-7, examples/swarm.rs:618-622, examples/swarm.rs:826-833, src/rumors/unordered.rs:95-108)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (`git log -1 d800957e` records the flip: "the single sample sat inside the churn oscillation and flipped with the key relayout, while the descent itself is unchanged"; the pool is filled in observer yield order at 618-622, the observer's pass walks the tree range per the refutation's read of unordered.rs:105, and each redact draw indexes that order at 827-828)
- Seen by: correctness; refutation: confirmed; history: deliberate and holds for the four-sample mean (d800957e's stated response), with the coupling recorded only in that commit message
- Owner-gated: no (adding seeds multiplies one test's runtime; not a gate-policy change)

The test is one seeded trajectory, and the identity each redact draw selects depends on the pool's order, which is the observer's yield order, which is tree path order, which is SHA3 of the version's encoded bytes. A change to `before`'s encoding therefore reshuffles the trajectory with no change to the controller; d800957e records exactly that and responded by averaging four samples, which narrows the coupling but does not remove it. The testdoc states determinism (true) but not this sensitivity, so the next relayout that pushes a trajectory to the band's edge will be read as a controller regression. The claim is convergence for any seed, and one seed is asked to stand for it.

Evidence:

    141	        for sum in settled {
    142	            let live = sum / SAMPLES as u64;
    143	            assert!(
    144	                live >= target / 2 && live <= target * 2,
    6	//! local pool. Everything here is single-threaded and seeded, so a failure
    7	//! reproduces exactly.

    d800957e (commit message)
    - The swarm controller test judges the post-ring equilibrium as a mean
      over four settled samples instead of one endpoint draw: the single
      sample sat inside the churn oscillation and flipped with the key
      relayout, while the descent itself is unchanged

Resolution: State the coupling in the testdoc (pool order is the observer's tree order, so a version-encoding change re-draws the trajectory), and judge the family: run the phase schedule under a small fixed set of party seeds (three suffices) and require each run's means in band; the stale-draw defect the test exists to catch (72 against 20 per d987a2be) fails every seed, while a benign relayout that grazes the band on one trajectory does not fail all of them. Cut `rounds` per phase rather than seeds if runtime matters. Acceptance: the testdoc names the coupling; the judgment spans more than one seed; an encoding perturbation equivalent to d800957e passes without re-tuning.
Construction: change the three party seed constants (0xa11ce, 0xb0b, 0xca201) to any other values and observe whether every phase still lands in band; a family judgment is one whose verdict does not depend on which three.

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
