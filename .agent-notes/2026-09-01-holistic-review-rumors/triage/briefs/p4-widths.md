<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from rulings T85, T87, T88, T126, T129, and T132 in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P4 lane: one definition per width, count, and transcribed constant

## Goal

Every width, count, and constant the code relies on has one
definition that the compiler or a committed test ties to its use: the
32-byte path width is one named constant with `Root::HEIGHT` asserted
against it; the stream count is derived in the codec and cited by the
link by name (T87); the slot constants derive from `size_of` (T88); the
design-session count has one definition and the tradeoff table one
renderer (T129); and every test-side transcription of a computed
original (labels, epochs, record lengths, state codes, the branch
preimage, the proptest version, `FAN + 1`) becomes the original or is
tied to it by an assertion, with `rumors::testing` exporting the
derivations the harness copied (T85). Effort: high (production
constants; one pin may move under T88).

## Ground rules

These apply to every P4 lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal `<base sha>` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `<base sha>`, fast-forward; if it has diverged, stop and report. Never call
  EnterWorktree; operate on the worktree through `git -C <path>` and
  absolute paths, one shell invocation at a time.
- **The review documents are the specification.** Each member entry below
  quotes its Resolution and Acceptance verbatim from the topic document in
  `.agent-notes/2026-09-01-holistic-review-rumors/`; the entry's full record
  (evidence, construction, demonstration) is under `### <id>:` there and in
  `evidence/`. Line anchors are at the reviewed commit `9e5784fb`; re-anchor
  from the quoted evidence, never from the line numbers.
- **Goal beside mechanism.** Where a quoted resolution and the goal above
  come apart, the goal wins, and the discrepancy is reported.
- **Stops.** Report and leave the entry open; do not work around: anything
  that moves an `insta` snapshot or a committed pin; any change to a public
  signature or public rustdoc contract the resolution does not name;
  anything that contradicts a ruling in `triage/rulings.md`; any deviation
  from the stated resolution; anything this brief marks as a stop. A stop on
  one entry does not block the others.
- **Negative controls.** Every repaired instrument lands with a committed
  demonstration that a known-bad artifact fails it. The constructions in
  `evidence/witness.md` and each entry's Construction line are those
  artifacts; convert each into a committed test (`should_panic`, an asserted
  `Err`, a `--self-test` case, or a reversible mutation whose observed
  failure the commit message records verbatim).
- **Resource discipline.** Build with `cargo nextest run --no-run` before
  running; during iteration run only the binaries the entry names; never
  iterate on a timing measurement. One full `just gate` before each commit,
  run in the background redirected to a log under `<scratchpad>/p4-widths/`,
  polled with short foreground checks (the foreground command cap is ten
  minutes). Keep every working file under that directory. `just all` and
  `just ci` are run once each at the end, not per commit.
- **Commits.** One commit per logical unit, its message describing the
  change and naming the entry ids and ruling. Commit every proptest seed
  file that appears. Prose speaks in the present tense: no reference to code
  that no longer exists, no dated rationale at a declaration site. Comments
  use spaced double-hyphens, never em-dashes; every test has a doc comment
  stating its invariant. Never delete anything outside your worktree; if the
  disk fills, stop and report.
- **Self-retirement.** After the final commit, from inside the worktree:
  `cargo metadata --no-deps --format-version 1 | jq -r .build_directory`,
  then delete that directory and the worktree's `target/`. Leave the
  worktree in place.
- **Report.** For each entry: landed, stopped, or open; the commit sha(s);
  the acceptance evidence (the command and its decisive output, verbatim).
  Then anything left open and why. Your report is data: the coordinator
  verifies each entry's Acceptance against the tree at the reported sha
  before the ledger records it. Report what you could not do rather than
  working around it.
- **Prose.** `PROSE.md` in this directory binds this lane (ruling T141): every
  paragraph you touch passes its three tests (altitude, concision,
  legibility), and the diff is net shorter in prose unless your report says
  what the added sentences buy.
- **Machine.** The illumos box (`ox-east-1`, per the `building-on-illumos` skill) is
  where a lane builds, tests, and gates. The wrapper syncs the Mac
  worktree to `~/src/<worktree basename>` on the box and runs one command
  there with its own target directory, so lanes do not collide; cargo
  runs `--locked` there; nothing is edited or committed on the box. A
  clean gate on the box is the gate of record for a commit; the Mac runs
  no gate (Finch's ruling). The box gate is `on-illumos.sh <worktree> 'just gate'`. One leg is
  expected red there and counts as clean when it is the only failure:
  `fuzz`, because libFuzzer has no illumos port (`FuzzerPlatform.h`
  refuses the target); a lane quotes that line and runs no fuzz build
  elsewhere (Finch's ruling: fuzzing is CI's). clippy's
  `missing_const_for_thread_local` misfires on illumos, where
  `thread_local!` expands through the OS-keyed path; `before` carries an
  illumos-scoped crate-level allow, so a lane based before that landed
  either rebases or passes `RUSTFLAGS="-A clippy::missing_const_for_thread_local"`
  in the remote command for that one run. Two legs that
  pin toolchain-derived numbers may fire on the box if its toolchains
  differ from the pinned ones; a lane reports such a leg with both numbers
  rather than re-pinning anything. `tools/memwatch` is deleted by the
  `p1-memwatch` lane, which runs first and unstacked; `p1-gate` rebases
  onto it. Benchmarks whose committed baselines are the Mac's run on the
  Mac, once, on a quiet machine. Clock guard, checked before every box
  run: rsync preserves mtimes and cargo's rebuild detection is
  mtime-based, so a box clock ahead of the Mac by more than a couple of
  seconds means a green build of stale code; on skew, a lane either runs
  with a fresh target directory on the box (a cold build, no stale
  artifact to trust) or waits, and says which. Stepping the box's clock is
  admin work on a shared machine and is Finch's, never a lane's. The
  builder cap counts Mac builders only; on the box (96 cores, 1 TiB) there
  is no lane cap, only the load: hold a launch while the one-minute load
  average sits above about 150 on 192 threads, and keep wall-time
  measurements under `pset-run` to one at a time, announced in the merge
  queue first.
- **Out of scope.** The formal tier (`lean`, `eventdag`, `muxprobe`, everything under
  `formal/`) and `before`'s bench judge (`bench-judge`,
  `bench-judge-tripwire`) are never run or edited by a rumors lane; a
  recipe that composes them (`all`) is exercised by its other legs
  individually. A rustdoc on a Rust-side literal derived from the Lean
  artifact is Rust prose and may be edited where a ruling names it.

## Mechanism, in order

1. **`PATH_LEN`**, one commit: tree-typed-12's option (1) and
   inventory-16 agree; `KEY_DEPTH` (streaming-backend-window-25) and
   `Network::LEN` (mirror-common-11) alias it;
   `const _: () = assert!(Root::HEIGHT == PATH_LEN)`. Negative control:
   a scratch build with the literal at 31 fails to compile; quote it.
   Oracle: `grep -n '\b32\b' src/tree/typed/*.rs src/tree/typed/untyped/*.rs`
   outside doc comments returns only the definition.
2. **The stream count** (T87: link-3, remote-codec-3's count half,
   conformance-13, remote-proxy-tests-17), one commit: the codec derives
   `COUNT` from `STREAMED_HEIGHT_COUNT` and `STREAM_HEIGHT_STRIDE`,
   `link::STREAM_COUNT` cites it, the pin becomes a check of the
   derivation, `const _` ties `STREAM_COUNT <= 256`. Oracle:
   `git grep -n -E '= 17\b' -- src/link.rs src/tree/mirror/streaming/remote/codec/signal.rs`
   is at most one line.
3. **`size_of` slots** (T88) and the design count (T129: testing-infra-2);
   `tradeoff_table_matches_the_derivation` byte-identical, or the pin
   re-accepted in the same commit with the attribution stated.
4. **The fan and the rest of the literals**: streaming-tests-15,
   tests-resource-link-window-21, remote-adapter-tests-9,
   materialized-40, remote-capture-atlas-23, remote-capture-atlas-11,
   remote-proxy-21, tests-disruption-handshake-19,
   streaming-backend-window-34 (a `const fn`).
5. **Test-side transcriptions**: tests-common-6 (the two `testing`
   exports), benches-envelope-1 (a committed test against
   `Hash::branch`), tests-common-31 (the lockfile pin test),
   tests-lifecycle-28, tests-observation-24, remote-adapter-streams-29
   (a proptest over every epoch), remote-adapter-tests-22 (a
   fixture-derived bound), remote-proxy-tests-21 (`Signal` under
   `cfg(test)`). Each lands with its Construction as the negative
   control, quoted.
6. **Closing commit**: `62_500` one definition; `16_384|2_048`
   definitions only; `LABEL_LEN`, `253`, `MAX_RECORD_LEN = 14`, and
   numeric state literals under `proxy/tests/**` gone;
   `Sha3_256::digest|55799|Tag\(24` absent from `tests/common` or named.

## Members

Heading entries quote Resolution and Acceptance verbatim from the topic document; nit rows quote their table row (site, issue, resolution). Each line opens with the entry's primary site at `9e5784fb`; the full record, with its related sites and evidence, is under `### <id>:` in the document the ledger's `doc` column names. Rulings are cited by number; T132 is the roster approval for every low and nit.

- **tests-common-6** (low; T85; T85: `rumors::testing` exports `leaf_path` and `decode_bookmark_record`, delegating to the crate's derivations (the collision-schedule mode of T23 rides along by construction)). `tests/common/flaky.rs:40-45`. Resolution: preferred: export `testing::leaf_path(&Version) -> [u8; 32]` delegating to the crate's leaf-address derivation and `testing::decode_bookmark_record(&[u8]) -> Result<BTreeMap<Network, Vec<Clock>>, FormatError>` delegating to `bookmark::format::decode`; delete both transcriptions; the self-check then compares the crate's derivation with the landed wire. Fallback: name the constants (`SELF_DESCRIBED_CBOR_TAG`, `EMBEDDED_CBOR_TAG`, `FRAME_PAYLOAD_INDEX`) at the top of flaky.rs, reword flaky.rs:40 to what is true today, and reword tests/gossip_snapshot.rs:107-108 to claim only what a same-function comparison checks. Either way, give the batched-run fixture a transcript-level assertion (a run frame carrying exactly two records) before its snapshot, as the radix fixtures already have. Acceptance: no `Sha3_256::digest`, `55799`, or `Tag(24` in tests/common, or each is a named constant; no comment claims a same-function comparison detects hash drift.
- **link-3** (low; T87; T87 rules the opposite ownership from this Resolution: the codec derives the count and `link::STREAM_COUNT` cites it by name; the pin becomes a check of the derivation). `src/link.rs:161-169`. Resolution: keep ownership in `link` (the direction the existing imports already run) and have the codec cite it: `pub const COUNT: u8 = crate::link::STREAM_COUNT as u8;` with `const _: () = assert!(STREAMED_HEIGHT_COUNT.div_ceil(STREAM_HEIGHT_STRIDE) + 1 == crate::link::STREAM_COUNT);` beside the schedule constants in signal.rs, so the compiler checks the arithmetic the doc states. Delete `stream_count_matches_the_codec` and the `#[cfg(test)] codec_stream_count()` accessor; consider retiring the private `STREAM_COUNT` alias at streams.rs:71. Flag to the codec partition's reviewer. Acceptance: `grep -rn '= 17' src/link.rs src/tree/mirror/streaming/remote/codec/signal.rs` returns exactly one line; a compile-time assertion ties the value to the stride and height constants; `codec_stream_count` is gone; `just gate` clean.
- **remote-codec-3** (low; T87; the count half is T87 (codec owns, link cites); the fan half (`MAX_QUERY_CHILDREN` from `FAN`) is T120 in P8 and is not landed here). `src/tree/mirror/streaming/remote/codec/budget.rs:86-89`. Resolution: Define the opener length once (beside `WireSignal::ENCODED_LEN` in signal.rs, or in frame.rs) and the supply head length once (beside `RECORD_TAG_LEN`), then `SUPPLY_FRAME_OVERHEAD = OPENER_LEN + SUPPLY_HEAD_LEN`, `FULL_FAN_QUERY_FRAME_LEN` starts from `OPENER_LEN`, and `Heads<OPENER_LEN>` / `Heads<SUPPLY_HEAD_LEN>` use the shared names. Add `const fn record_item_len(content: usize) -> usize` (saturating) and have `record_len` and `push` call it; `lone_record_spans` compares through a u64 twin or a `usize::try_from`. The pin `record_len_matches_an_actual_push` stays meaningful because it compares the closed form against bytes actually written, not against a second arithmetic. Write `pub const COUNT: u8 = (STREAMED_HEIGHT_COUNT / STREAM_HEIGHT_STRIDE + 1) as u8;` with a `const _: () = assert!(...)` that it fits, keeping `link::STREAM_COUNT` as the transport's own literal and the existing cross-layer pin. Ask whether `MAX_QUERY_CHILDREN` should be defined as `FAN` (or both from one radix constant); if they are meant as distinct quantities, say so at one of the two declarations. Acceptance: exactly one definition each of the opener length, the supply head length, and the record item sum in the codec; no bare `17` in signal.rs; `default_budget_matches_its_derivation` (pinned 1_830_400), `full_fan_frame_len_matches_an_actual_encode`, `record_len_matches_an_actual_push`, and `stream_count_matches_the_codec` pass unchanged.
- **streaming-backend-window-26** (low; T88; T88: if `SCOPE_ENVELOPE_BYTES` moves, re-pin it in the same commit with the layout attribution stated (ruled, not a stop)). `src/tree/mirror/streaming/window.rs:158-163`. Resolution: Name the containers and derive: `SCOPE_FIXED_BYTES = size_of::<Query<E>>() + size_of::<Resolution<E>>()` (or whichever pair is meant, at the `Local` erased instantiation) and `LEAF_REQUEST_BYTES = size_of::<Prefix<Z>>()` or its padded form with the padding intent stated; keep the pointer-class caveat the sibling docs carry. If the derived value differs from 128, re-pin `SCOPE_ENVELOPE_BYTES`, `tradeoff.md`, and the peer.rs figures in one deliberate commit naming the layout attribution. Cite `DESIGN_SESSION_MESSAGES` by name for the 62,500 restatements, or move that constant out of the test module. Acceptance: all four slot constants are `size_of` expressions over named types; `scope_envelope_matches_the_derivation` passes; if the pin moved, the commit states the attribution.
- **remote-adapter-tests-22** (medium; T126). `src/tree/mirror/streaming/remote/adapter/tests/runs.rs:42-49`. Resolution: derive the sweep's upper bound from the fixture rather than prose: compute it as `SUPPLY_FRAME_OVERHEAD + colliding_leaves(MAX_CASE_LEAVES).iter().map(|l| LeafRun::record_len(&l.version, &l.message)).sum::<usize>() + 1` (via `prop_flat_map` on `count`, or a `LazyLock`), or keep the constant and `prop_assert!(LeafRun::record_len(&leaf.version, &leaf.message) <= MAX_RECORD_LEN)` for every generated leaf as the envelope's liveness floor. Rewrite the doc in present-tense codec terms (record tag and body head, version tag and head, canonical version bytes, CBOR payload) or cite `LeafRun::record_len`. Add one witness that the maximum generated budget yields a single frame for the largest case. Acceptance: a committed assertion ties the sweep's upper bound to the actual record lengths of the generated leaves (lowering `MAX_RECORD_LEN` to 13 fails the property); the constant's doc names no borsh-era widths; a test shows the ten-leaf case encodes as one frame at the top budget.
- **testing-infra-2** (medium; T129). `src/testing.rs:207-235`. Resolution: In window.rs, declare `pub(crate) const DESIGN_SESSION_MESSAGES: u64 = 62_500;` once beside `SCOPE_ENVELOPE_BYTES` under the same `cfg(any(test, feature = "test-internals"))`, make `KEY_DEPTH` `pub(crate)` (or add a `Window::capacities()` accessor), and move the renderer body to the window module as `pub(crate) fn tradeoff_table() -> String`, with `solve_window` becoming `Window::from_budget(...).widest()`. Leave `testing::window_tradeoff_table` as a one-line delegation for the example and the pin; expose the constant through `testing` for `tradeoff_probe`'s `DIVERGENT`; let window.rs:243 cite the constant by name. Acceptance: `grep -rn '62_500' src tests` returns one definition; `grep -n '0\.\.=32' src/testing.rs` is empty; `tradeoff_table_matches_the_derivation` still passes against today's `tradeoff.md` byte for byte.
- **benches-envelope-1** (low; T132). `benches/branch_hash.rs:14-18`. Resolution: Expose the shipped assembly through `rumors::testing` (a `branch_hash(prefix, children) -> [u8; MERKLE_HASH_LEN]` shim over `Hash::branch`) and make `contiguous` call it, keeping `streamed` local as the alternative under test (it restates the layout by nature); or add a test in `tests/` that `#[path]`-includes the bench (the `latency_link.rs` pattern) and asserts `contiguous(prefix, kids)[..MERKLE_HASH_LEN]` equals the shipped digest for each `FANOUTS` entry. Rewrite lines 14-18 to match. Acceptance: a committed test fails when `contiguous` and `Hash::branch` disagree on any preimage byte for the swept fan-outs, or `contiguous` is the shipped code. Construction: Change `BRANCH_TAG` in hash.rs to 2 and run the gate: the hash tests fail on the pinned layout, but nothing points at the bench, which keeps hashing tag 1 and keeps being cited at hash.rs:167.
- **conformance-13** (nit; T132). ``src/conformance/link.rs:745-745``: `index as u8` ties the in-band index to `STREAM_COUNT <= 256` with nothing at compile time Resolution: `u8::try_from(index).expect("STREAM_COUNT fits an index byte")`, or a `const _: () = assert!(STREAM_COUNT <= u8::MAX as usize + 1);` beside the probe
- **inventory-16** (nit; T132). ``src/tree/typed/prefix.rs:44-49``: The 32-byte path width is a repeated literal with no shared name Resolution: Define one `pub(crate) const PATH_LEN: usize = 32;` under typed/ (or use `Root::HEIGHT`); replace the arithmetic literals; alias window.rs's `KEY_DEPTH` to it
- **materialized-40** (nit; T132). ``src/tree/mirror/streaming/materialized/work/tests/violations.rs:345``: `violations.rs` hard-codes the height count and mirrors `Violation` with an identity enum Resolution: `for height in 0..Root::HEIGHT`; generate over `Violation` directly or derive the `Injection` mapping with a stated reason
- **mirror-common-11** (nit; T132). ``src/tree/mirror/handshake.rs:176-178``: Two widths restated as literals beside the constants that name them Resolution: Write `[u8; MISMATCH_PREVIEW_LEN]` at both variants; add `Network::LEN` and replace `NETWORK_LEN` and network.rs's literal 16s with it
- **remote-adapter-streams-29** (low; T132). `src/tree/mirror/streaming/remote/streams/tests.rs:22-27`. Resolution: Reword the testdoc count-free per S1 ("The label is the epoch head then the stream-index head, each a shortest-form CBOR unsigned int; both below 24 encode as one byte each"), keep the literal case as the readable example, and add a proptest over `epoch in any::<u8>()` and every `Stream` asserting `label(epoch, stream).len() == cbor::head_len(epoch) + 1` and that two `cbor::read_head` calls recover `(epoch, index)` and exhaust the input; spell tests.rs:424 through `cbor::write_head`. Acceptance: the testdoc is true for every `u8` epoch; a committed proptest exercises epochs at and above 24; any seed file that appears is committed. Construction: `label(24, Stream::new(3).unwrap())` is `[0x18, 0x18, 0x03]` by `render_head` (major 0, info 24, argument byte 24; then `0x03`), three bytes, refuting "exactly two bytes".
- **remote-adapter-tests-9** (nit; T132). ``src/tree/mirror/streaming/remote/adapter/tests/fan_occupancy.rs:113-115``: `FAN + 1` is spelled independently at five sites and only one pair is mechanically bound Resolution: one `pub(crate)` records-per-stream constant in window.rs
- **remote-capture-atlas-11** (nit; T132; T138/T140 replaced the hand renderer with cbor-diag: verify at base and quote the absence if the site is gone). ``src/tree/mirror/streaming/remote/codec/capture.rs:356-362``: Bare major numbers `7` and `1` beside named `MAJOR_*` constants Resolution: Define local `MAJOR_NINT` and `MAJOR_SIMPLE` constants in capture.rs and use them at 356 and 362
- **remote-capture-atlas-23** (low; T132). `src/tree/mirror/streaming/remote/codec/tests.rs:42-63`. Resolution: `const SIGNAL_COUNT: usize = Signal::STATE_COUNT as usize;`; widen `Signal::STATES` to `pub(super)` and iterate it in place of `SIGNALS`; replace the `position` lookup with `usize::from(signal.state())`; write `[0; SPEAKER_COUNT]`; reword line 246 to "Every (speaker, stream, signal) placement pins either its canonical frame bytes or its typed rejection." Acceptance: codec/tests.rs contains no literal signal roster and no literal `10` or `340`; both snapshot files are byte-identical; a new `Signal` variant fails compilation in codec/tests.rs without touching a literal.
- **remote-proxy-21** (nit; T132). ``src/tree/mirror/streaming/remote/proxy/work/progress/trace.rs:92-98``: Trace causality check mixes a named height with literal heights Resolution: import `Z` and `S` beside `UnderRoot` and write `Z::HEIGHT` / `<S<Z>>::HEIGHT`
- **remote-proxy-tests-17** (nit; T132). ``src/tree/mirror/streaming/remote/proxy/tests/failures.rs:194-197``: Unnamed capacities and counts: `17` four times, `64 * 1024`, `31` Resolution: Use `TRANSPORT_CAPACITY` in failures.rs (17 carries no intent), or name the value with a one-line doc
- **remote-proxy-tests-21** (low; T132). `src/tree/mirror/streaming/remote/proxy/tests/harness.rs:38-42`. Resolution: Add `#[cfg(test)] pub use signal::Signal;` to codec.rs; in the harness select by `Signal::from_state(state)` (`Query` matches `Ok(Signal::Query(_))`, `EndingReaction` matches `Ok(Signal::Match(Flow::End) | Signal::QueryEmpty(Flow::End) | Signal::Query(Flow::End) | Signal::Supply(Flow::End))`); in malformed.rs write `Signal::Match(Flow::Continue).state()` and `Signal::End(End::Stream).state()`; delete the four local constants. Acceptance: no numeric state literal remains in `proxy/tests/**`; the malformed suite passes with every `script.fired()` assertion holding.
- **streaming-backend-window-25** (nit; T132). ``src/tree/mirror/streaming/window.rs:134-137``: `KEY_DEPTH` restates `height::Root::HEIGHT` as a fresh literal Resolution: `const KEY_DEPTH: usize = <height::Root as Height>::HEIGHT;`, or at minimum a `const _` assertion tying the two
- **streaming-backend-window-34** (low; T132). `src/tree/mirror/streaming/window.rs:601-606`. Resolution: Make `tail_exponent` a `const fn` (replace `j.min(TAIL_DEPTH_CAP)` with an `if`; `u128::div_ceil` is const) and define `const BERNSTEIN_TAIL: u128 = tail_exponent(0);` with the doc reading "the flat union level: `tail_exponent` at zero sharpening". Drop the maintenance sentence. Acceptance: `BERNSTEIN_TAIL` has no literal initializer; `envelopes_are_consistent` and the window tests pass unchanged.
- **streaming-tests-15** (nit; T132). ``src/tree/mirror/streaming/tests/capacity.rs:136-143``: Magic numbers where a named constant exists or is owed Resolution: Import `window::FAN` and write `FAN`, `FAN - 2`, `FAN - 3` with the derivation stated once at 183-192
- **tests-common-31** (low; T132). `tests/seed_liveness.rs:26-29`. Resolution: add `const TRANSCRIBED_PROPTEST_VERSION: &str = "1.11.0";` and a test that reads `Cargo.lock` from `CARGO_MANIFEST_DIR`, finds the `[[package]] name = "proptest"` block, and asserts its `version` equals the constant, with a message telling the bumper to re-check `FileFailurePersistence::resolve` and update the constant. Acceptance: bumping proptest in Cargo.lock fails the seed-liveness binary until the constant is updated.
- **tests-disruption-handshake-19** (nit; T132). ``tests/gossip_when.rs:918-921``: The preamble width is a bare `30` beside a named constant in a sibling suite Resolution: Move `PREAMBLE_LEN` and its derivation comment to tests/common and import it in both suites
- **tests-lifecycle-28** (low; T132). `tests/reuse.rs:184-194`. Resolution: `const PRE_WRAP_SESSIONS: usize = u8::MAX as usize - 2;` with the doc saying "two epochs before the wrap". After the pre-wrap loop, on each end: `let parts = link.into_parts(); assert_eq!(parts.session.epoch(), PRE_WRAP_SESSIONS as u8); let mut link = parts.into_link();`. After the wrap rounds, assert both ends read `((PRE_WRAP_SESSIONS + WRAP_ROUNDS as usize) % 256) as u8`, which witnesses the lockstep directly. Acceptance: changing `PRE_WRAP_SESSIONS` so no wrap occurs, or widening the epoch type, fails the test at the epoch assertion; no literal 253 in reuse.rs.
- **tests-observation-24** (low; T132). `tests/session_stats.rs:246-249`. Resolution: Have `CountingWrite` retain each stream's first write and subtract `stream_label(first_bytes).1` per opened stream, or drive the test through `capture_sides` (which yields per-stream blobs) and sum `stream_label(blob).1`; delete `LABEL_LEN` and reword the doc to say the label length is parsed. Acceptance: no literal label length remains in session_stats.rs; the byte-tally assertion derives label bytes from `stream_label`.
- **tests-resource-link-window-21** (low; T132). `tests/window_census.rs:33-40`. Resolution: In window_census: `const COMMON: usize = 1_024;` used at both sites; derive the height count from `capacities.len()` at the use site; derive the fan from `usize::from(u8::MAX) + 1` as encode_alloc does, or expose `FAN` through `rumors::testing`. In window_corners: name `LADDER_HOPS_BOUND` (12), `WAVE_FLOOR_HOPS` (64), and the linear-cost factor, carrying the rationale the inline comments at 93-97 already give. Acceptance: no bare `1_024`, `256`, `33`, or repeated `12`/`64` outside a named constant in the two files.
- **tree-typed-12** (low; T132). `src/tree/typed/height.rs:125-133`. Resolution: (1) add `pub const PATH_LEN: usize = 32;` beside `MERKLE_HASH_LEN` in hash.rs with a one-line doc (the SHA3-256 output width and the root height), and use `[u8; PATH_LEN]` and `PATH_LEN - H::HEIGHT` at the listed sites; optionally give `Height` a derived `const DEPTH: usize = PATH_LEN - Self::HEIGHT;` so `Path::pop` and `Prefix` read `H::DEPTH`. (2) Merge the two macros into one `heights!(H0 H1 … H32)` that emits both the `Height` impl and the numbered alias per name, using the `$n + 1` accumulator `impl_heights!` already has, then `pub type Root = H32;` and `const _: () = assert!(Root::HEIGHT == PATH_LEN);`. If the two-row layout is wanted as pedagogy, keep it and add only the `Root::HEIGHT` assert. Acceptance: height.rs has one enumeration of the heights (or the literal plus an assert on `Root::HEIGHT`); `grep -n '\b32\b' src/tree/typed/*.rs src/tree/typed/untyped/*.rs` outside doc comments returns only the constant's definition; erased.rs's `at_parent_height!` compiles unchanged.

## Hazards and stops

- T88 may move `SCOPE_ENVELOPE_BYTES`: ruled, not a stop, but the commit
  states the layout attribution and re-pins `tradeoff.md` and the
  `peer.rs` figures together.
- `p2-commit-path` (unmerged) edits `typed/node.rs`, `prefix.rs`,
  `untyped.rs`: step 1 after its merge.
- T151: the proptests added here set no `cases`.
- `tests/common` is the P5 harness lane's territory; P4 precedes P5,
  and the exports of T85 are what let that lane delete the copies.
