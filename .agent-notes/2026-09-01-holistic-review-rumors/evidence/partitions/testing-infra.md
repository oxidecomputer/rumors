# Partition testing-infra: The crate-internal test scaffolding (testing module, memnet, transport) and the lib-level tests

## Partition summary

The partition is the crate's own test scaffolding plus the crate-level tests that need private access. `src/testing.rs` (442 lines) is a `doc(hidden)` facade compiled under `cfg(any(test, feature = "test-internals"))`: one-line delegations that hand `pub(crate)` instruments (codec frame builders, framing chunk constants, window constants, the node census, version-bound walks) to the integration suites; `run_to_quiescence`, the closed-world deterministic poller that turns a wire stall into a `Quiescence::Stalled` error; and `window_tradeoff_table`, the renderer of the sync-budget table the rustdoc includes. `src/testing/memnet.rs` (140 lines) is a channel-backed named-listener network for the routed link. `src/testing/transport.rs` (888 lines) is the adversity layer: `IoPlan`/`IoFault`/`wrap_link` with fragmentation, self-waking delays, flush buffering, and byte- or operation-counted faults on every surface, plus `ReorderingAcceptor`, a batch-reversing acceptor. `src/tests.rs` (690 lines) holds party-linearity tests across bootstrap and retire, retire sessions severed at exact frame boundaries through a hand-rolled `Fuse`, link-poisoning tests, an uncontained-supply test at the `Rumors` tier, and the root-hash read meter pins. All 2160 lines read are test code; none ships in an application build.

The instruments that matter are in good shape. `run_to_quiescence` distinguishes a self-wake from a stall without wall-clock guessing and disables tokio's cooperative budget, with a committed test for each of those two properties. The fault injector's rule that a fault fires in place of the next successful operation keeps every threshold a function of the clean run, and the reason is stated at each site. `ReorderingAcceptor` says why a drain-only design degenerates under the deterministic scheduler and instructs consumers to assert its counter both ways. Every facade entry names the suite it serves and why the constant is read from the code rather than transcribed. `memnet` is small and exact. Every test carries a doc comment, and the severing tests explain in English why each outcome is the only correct one.

The dominant issues are duplication that the scaffolding was built to dissolve, and prose that expired when the wire changed. `src/tests.rs` carries a third byte-budgeted write fuse (`Fuse`/`FusedConnector`/`fused_link`) that `testing::wrap_link` provides; `party_of` and `with_messages` have byte-identical twins in `src/peer/gossip/tests.rs`; `window_tradeoff_table` re-implements `Window::widest`, hard-codes the private `KEY_DEPTH`, and defines a second `DESIGN_SESSION_MESSAGES`. The CBOR respelling of the wire (commit 4dd2053c) moved the preamble to 30 bytes, the greeting to one item, and the epilogue marker to two bytes, and updated the constants and bodies in `src/tests.rs` but not the comments beside them, so the `PREAMBLE_LEN` doc sums to 25, `greeting_frame_len` describes two frames, and the epilogue-severance test says "minus one byte" where the budget is minus two. Three verification gaps remain: the absorber's party is never read after any failed hand-off, `ReorderingAcceptor`'s inversion has no committed demonstration that it ever fires, and `Quiescence::PollBudget` is never observed.

## Findings

### testing-infra-1: Rustdoc cites source files by path, and calls two divergent acceptors "duplicated"
- Where: src/testing.rs:69-69 (related: src/testing.rs:81, 105, 116, 136, 145, 161, 178, 188, 203; src/testing/transport.rs:667-669; src/conformance/link/tests.rs:59-91)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (grep for `tests/`, `examples/`, `src/` over the partition's rustdoc; both acceptor bodies read side by side)
- Seen by: refutation pass (new), structure-prose [4], api-economics [35]; refutation: confirmed (path citation and "duplicated" survive; the feature-seam rationale itself holds); history: the transport.rs citation has already rotted once (repointed in becde7774 after a3da46c42 moved the suite)
- Owner-gated: no

Eleven rustdoc sentences name a consumer or sibling by file path, which a rename orphans; the one at transport.rs:667-669 also calls `ReorderingAcceptor` a duplicate of `ReversingAcceptor`, though the two mechanisms differ (a patience-bounded wait with yields at transport.rs:692-715 versus a single noop-waker poll per slot at conformance/link/tests.rs:71-84). Principle 5: prose describes what is; a citation by role survives a move and an inaccurate "duplicated" stops the next reader from seeing the divergence.

Evidence:

    69	/// (`tests/dispute_wire.rs`) can hold it against deterministic byte counts.

    667	/// A sibling of the conformance suite's `ReversingAcceptor`
    668	/// (`src/conformance/link/tests.rs`), duplicated so this crate-internal seam
    669	/// does not depend on the public `conformance` feature.

Resolution: Name consumers by role ("the dispute-wire calibration suite", "the allocator meter", "the encoder allocation meter", "the trade-off example") without the path. At transport.rs:667-669, keep the feature-seam rationale (it is correct: `testing` is `cfg(any(test, feature = "test-internals"))` and `conformance` is `cfg(any(test, feature = "conformance"))`, lib.rs:306-321) and restate the relationship as a deliberate divergence: "The conformance suite's `ReversingAcceptor` drains only arrivals already `Ready`; this decorator waits." Acceptance: `grep -n 'tests/\|examples/\|src/' src/testing.rs src/testing/transport.rs` returns nothing inside rustdoc; the word "duplicated" is gone from transport.rs:667-669.

### testing-infra-2: `window_tradeoff_table` re-derives window internals in the facade: a second `DESIGN_SESSION_MESSAGES`, a hard-coded `KEY_DEPTH`, and a reimplemented `Window::widest`
- Where: src/testing.rs:207-235 (related: src/testing.rs:323-333; src/tree/mirror/streaming/window.rs:137, 243, 485-491; src/tree/mirror/streaming/window/tests.rs:15-24, 226, 257-264; tests/tradeoff_probe.rs:55-58; examples/window_tradeoff.rs:11)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (`grep -rn '62_500\|62,500' src tests examples benches` lists three definitions and two prose quotes; window.rs:137 `const KEY_DEPTH: usize = 32;` is private; window.rs:485-491 `widest` read)
- Seen by: structure-prose [5], [6]; blind-spots [22]; api-economics [36]; refutation: confirmed; history: deliberate-but-expired (`solve_window` landed hours before `Window::widest` in 487e17ea3 and was never retargeted; no placement rationale recorded for the renderer)
- Owner-gated: no

The design-session scale `62_500` is defined independently at testing.rs:218, window/tests.rs:24, and tests/tradeoff_probe.rs:58 and quoted in prose at window.rs:243, and nothing compares the copies, so the table this constant drives and the `SCOPE_ENVELOPE_BYTES` pin can describe different sessions without a test noticing. `solve_window` recomputes `Window::widest()` as `(0..=32).map(capacity).max()`, and `0..=32` hard-codes the private `KEY_DEPTH` here and again at line 332. The renderer's constants, pin, and committed output all live under the window module; the facade's responsibility is visibility. Principle 5: a number that matters lives in one mechanically enforced place that prose cites by name.

Evidence:

    215	    /// The design session's corpus scale per side: a round 62,500,
    216	    /// sized near the spec BDP in design-size records and stated
    217	    /// round, the scale `SCOPE_ENVELOPE_BYTES` is pinned at.
    218	    const DESIGN_SESSION_MESSAGES: u64 = 62_500;
    ...
    231	        (0..=32)
    232	            .map(|height| window.capacity(height) as u64)
    233	            .max()
    234	            .expect("thirty-three heights")

    24	const DESIGN_SESSION_MESSAGES: u64 = 62_500;          (window/tests.rs)
    58	const DIVERGENT: usize = 62_500;                       (tests/tradeoff_probe.rs)

Resolution: In window.rs, declare `pub(crate) const DESIGN_SESSION_MESSAGES: u64 = 62_500;` once beside `SCOPE_ENVELOPE_BYTES` under the same `cfg(any(test, feature = "test-internals"))`, make `KEY_DEPTH` `pub(crate)` (or add a `Window::capacities()` accessor), and move the renderer body to the window module as `pub(crate) fn tradeoff_table() -> String`, with `solve_window` becoming `Window::from_budget(...).widest()`. Leave `testing::window_tradeoff_table` as a one-line delegation for the example and the pin; expose the constant through `testing` for `tradeoff_probe`'s `DIVERGENT`; let window.rs:243 cite the constant by name. Acceptance: `grep -rn '62_500' src tests` returns one definition; `grep -n '0\.\.=32' src/testing.rs` is empty; `tradeoff_table_matches_the_derivation` still passes against today's `tradeoff.md` byte for byte.

### testing-infra-3: Import placement and grouping
- Where: src/testing.rs:335-343 (related: src/testing.rs:20, 26, 32, 54, 60, 72-73, 85, 95, 111, 120, 130, 140, 170-172, 181, 191, 229, 324, 330; src/tests.rs:8-23; src/testing/transport.rs:501-507, 540-545, 563, 599, 751-759)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`grep -c 'crate::tree::mirror::streaming::' src/testing.rs` is 18; `grep -c 'crate::link::' src/testing/transport.rs` is 19; tests.rs:8-23 read)
- Seen by: structure-prose [10]; blind-spots [30]; api-economics [40]; refutation: confirmed; history: the mid-file block is the residue of 83edcd944's mechanical move of the poller; the owner applied the imports-over-paths preference to the sibling routed modules in da23a964a
- Owner-gated: no

`testing.rs` places its `use std::{...}` block after 330 lines of items and spells `crate::tree::mirror::streaming::remote::...` and `...::window::...` in full at every delegation; `transport.rs` spells `crate::link::Link/Connector/Acceptor/LinkParts` nineteen times and `tokio::io::AsyncRead/AsyncWrite` at 504-505 and 756-757 despite importing them at line 14; `tests.rs:8-23` interleaves crate, std, and external imports across blank-line groups rustfmt will not merge. Doctrine: imports over long qualified paths except where the qualification informs.

Evidence:

    335	use std::{
    336	    future::Future,
    337	    pin::pin,
    338	    sync::{
    339	        Arc,
    340	        atomic::{AtomicBool, Ordering},
    341	    },
    342	    task::{Context, Poll, Wake, Waker},
    343	};

Resolution: Hoist the std block to the top of testing.rs; add `use crate::tree::mirror::streaming::{Local, remote, window};` and shorten the delegations; import `crate::link::{Acceptor, Connector, Link, LinkParts}` once in transport.rs and use the already-imported `AsyncRead`/`AsyncWrite`; regroup tests.rs:8-23 as std, external, crate. Acceptance: no `use` item below the first `fn` in testing.rs; `grep -c 'crate::tree::mirror::streaming::remote::' src/testing.rs` and `grep -c 'crate::link::' src/testing/transport.rs` are 0 outside `use` lines.

### testing-infra-4: `Quiescence::PollBudget` has no committed demonstration that it fires
- Where: src/testing.rs:376-393 (related: src/testing.rs:345-352, 400-417; .config/nextest.toml:1-5)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`grep -rn PollBudget src tests benches examples` returns only the variant at 351 and the return at 393)
- Seen by: blind-spots [23]; refutation: confirmed; history: no rationale found (the poller carried only `observes_wake_contract` from birth)
- Owner-gated: no

The runaway guard is the poller's second failure signal, and nothing in the repository observes it: `observes_wake_contract` covers `Ok` and `Stalled` only. A regression that made the loop unbounded, or returned `Stalled` from the guard, passes every test. Principle 6 (adequacy): a guard earns its place by a committed demonstration that the failure it names is caught; this is the liveness instrument every protocol suite rests on (.config/nextest.toml:1-5).

Evidence:

    376	    const MAX_POLLS: usize = 1_000_000;
    ...
    383	    for _ in 0..MAX_POLLS {
    ...
    393	    Err(Quiescence::PollBudget)

Resolution: Add to the poller's tests: `assert_eq!(run_to_quiescence(std::future::poll_fn(|cx: &mut Context<'_>| { cx.waker().wake_by_ref(); Poll::<()>::Pending })), Err(Quiescence::PollBudget));` (a million trivial polls completes in milliseconds). While there, give `MAX_POLLS` a one-line sizing statement (the largest closed-world session the suites run, with headroom), since a legitimate long session exceeding it would be misreported. Acceptance: a test asserts `Err(Quiescence::PollBudget)` for a perpetually self-waking future.
Construction: Change line 393 to `Err(Quiescence::Stalled)` (or the loop bound to unbounded with a counter) and run the workspace tests: nothing fails today.

### testing-infra-5: Inline `mod tests {}` blocks against the sibling-file convention
- Where: src/testing.rs:396-397 (related: src/testing/transport.rs:801-807, 827; AGENTS.md:72)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (`grep -rln '^mod tests {' src/` returns five files, two of them in this partition)
- Seen by: structure-prose [8]; api-economics [34]; refutation: confirmed; history: both blocks predate the convention (dfd19c447, 2026-07-24), which stated no exemption
- Owner-gated: no

Both scaffolding files carry their unit tests inline; AGENTS.md's convention is `mod tests;` with a `tests.rs` sibling so the implementation reads without them. The transport tests also reach for `futures::{pin_mut, poll}` (803, 827) where the same file uses `std::pin::pin!` (694).

Evidence:

    396	#[cfg(test)]
    397	mod tests {

    801	#[cfg(test)]
    802	mod tests {
    803	    use futures::{pin_mut, poll};

Resolution: Move the blocks to `src/testing/tests.rs` and `src/testing/transport/tests.rs`; use `std::pin::pin!` in place of `pin_mut!`. Acceptance: `grep -rln '^mod tests {' src/testing.rs src/testing/transport.rs` is empty; the four moved tests still run.

### testing-infra-6: Prose tells across the partition, including an off-model "malicious" and a `Side` doc bound to one retired topology
- Where: src/testing/memnet.rs:9-10 (related: src/tests.rs:86-87, 142, 414, 497, 543-548, 558, 564, 672; src/testing/transport.rs:21-24, 638, 654, 660-665, 675, 688, 701, 744, 749; tests/routed_link.rs:10 (outside this partition, same phrase))
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -n -i 'genuine\|seam\|honest\|malicious\|silently\|load-bearing'` and `grep -n -w sound` over the four files; each hit read in context)
- Seen by: structure-prose [13]; blind-spots [27]; api-economics [41]; refutation: confirmed ("honest peer" at transport.rs:64 and 616 is the model-of-record term and stays); history: `malicious` predates and contradicts AGENTS.md's model-of-record hard rule (a59dc786d); the `Side` docs date from when the module served only the proxy harness
- Owner-gated: no

One sentence breaches a hard rule: tests.rs:86-87 models the forged retiree as "a buggy or malicious peer", while AGENTS.md puts hostile-peer regimes off-model and makes the overlap check a conformance-bug detector. The rest is default dialect: "seam honest" (memnet.rs:10), "genuine"/"genuinely" seven times in transport.rs and twice in tests.rs, "load-bearing" (662), "silently" without its mechanism (664, 749), "sound" for "correct here" (transport.rs:701, tests.rs:414), "honest divergence" (tests.rs:558, 672), and "parity leg", "tripwires", "seam", "tier" stacked at tests.rs:543-548. `Side`'s variant docs call the endpoints "proxy endpoint[s] in the test harness", but `IoSide::Left/Right` label peers' links in tests/lifecycle.rs:70-71. Writing-style rule: describe code by the property that holds; metaphors only where they rewrite as mechanism.

Evidence:

    9	//! single-poll executor, and names are plain strings, which keeps the
    10	//! address seam honest: nothing here resembles an IP address.

    86	/// we forge it with [`Party::dangerously_alias`] — a copy of the absorber's
    87	/// *exact* region — to model a buggy or malicious peer. The overlap is detected

    21	    /// The first proxy endpoint in the test harness.
    22	    Left,
    23	    /// The second proxy endpoint in the test harness.
    24	    Right,

    662	/// disposition instead of assuming it. The genuine wait is load-bearing: a
    663	/// decorator that only drains arrivals already `Ready` never sees a second
    664	/// arrival under the deterministic scheduler and silently degenerates to
    665	/// pass-through — which is exactly what the asserted counter makes loud.

Resolution: tests.rs:87: "a buggy or nonconforming peer". memnet.rs:10: "names are plain strings, so nothing here can be mistaken for an IP address". transport.rs:21-24: "The endpoint the report attributes operations to; which is which is the test's choice." Define "inversion" once (a released batch of two or more) and drop the "genuine" qualifiers; "waits, yielding, for a further arrival" for "genuinely waits"; "degenerates to pass-through with no failing assertion" for "silently degenerates"; "divergence on both sides" for "honest divergence"; "correct" for "sound"; rewrite tests.rs:543-548 as "the same rejection the mirror suites pin in process and over their wires, observed here through `Rumors::gossip`; the poisoned store is built by a local `Tree::join`, which no session check guards". Acceptance: the grep above returns only "honest peer" at transport.rs:64 and 616; "proxy endpoint" is gone from transport.rs.

### testing-infra-7: `memnet` diagnostics misname their cause; `LISTEN_BACKLOG` has no witness
- Where: src/testing/memnet.rs:116-121 (related: src/testing/memnet.rs:25-28, 72-77, 112-113, 134-139; .agent-notes/2026-07-29-routed-link/routed-link.md:246 (sizing argument, per the history pass))
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (memnet.rs read in full; `grep -rn backlog src/link tests` finds only the routed endpoint's `incoming_backlog` tests, never 65 unaccepted dials against `LISTEN_BACKLOG`)
- Seen by: blind-spots [28]; api-economics [42]; refutation: confirmed; history: unchanged since b16a800b5; the constant's sizing argument lives only in the design note
- Owner-gated: no

`try_send` fails for `Full` and for `Closed`, and the comment at 112-113 knows both causes ("A full or abandoned backlog"), but the message names only the first; `listen` never removes a dropped listener's sender (72-77), so dialing a name whose `MemoryListen` was dropped reports a full backlog. A listener displaced by a later `listen` sees "the memory network is gone" (138) while the network is live. `LISTEN_BACKLOG = 64` exists for realism and nothing exercises it. A harness message is the first thing a failing routed test shows; one that names the wrong cause costs a detour.

Evidence:

    112	        // A full or abandoned backlog refuses the dial outright; a
    ...
    116	        listener.try_send(accepted).map_err(|_| {
    117	            io::Error::new(
    118	                io::ErrorKind::ConnectionRefused,
    119	                "the listener's backlog is full",
    120	            )
    121	        })?;

Resolution: Match `TrySendError::Full` and `TrySendError::Closed` to two messages ("the listener's backlog is full" / "the listener at this name was dropped"), and reword 138 to "the listener was displaced or the network dropped". For `LISTEN_BACKLOG`, either add a unit test that the 65th unaccepted dial is refused and state the sizing inline (the routed link's worst-case stream complement), or switch to `mpsc::unbounded_channel` and delete the constant. Acceptance: dialing a name whose listener was dropped yields an error naming the dropped listener; `LISTEN_BACKLOG` is either tested with its rationale beside it or gone.

### testing-infra-8: `IoFault`/`IoPlan` admit configurations the mechanism reinterprets without saying so
- Where: src/testing/transport.rs:66-74 (related: src/testing/transport.rs:60-65, 93-107, 180, 194-204, 208-213, 257, 298, 303, 370; src/tree/mirror/streaming/remote/proxy/tests/failures.rs:184-189)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (transport.rs read; failures.rs:184-189 read; `grep -rn 'read_delays\|write_delays\|flush_delays' src tests` finds only generators drawing `0_u8..=2`)
- Seen by: structure-prose [12]; blind-spots [25]; api-economics [38], [39]; refutation: confirmed; history: no rationale found; the operations-only rule for supply faults has been prose since 1a07a0901
- Owner-gated: no

`IoFault` pairs any `Operation` with any `FaultUnit`, but `failure` counts flushes, connects, and accepts regardless of unit (194-204), so a `Flush`/`Bytes` fault fires on an operation count while `InjectedIo` reports `unit: Bytes` (208-213); the one strategy that generates faults hand-avoids the combination (failures.rs:184-189). Two more normalizations are invisible from the fields: delays saturate at two polls (`.min(2)`, line 180) though the fields are `Vec<u8>`, and a chunk of 0 becomes 1 (298, 303, 370). Types-first: make the meaningless combination unrepresentable, or state every normalization where the field is declared; named constants over magic numbers for the clamp.

Evidence:

    66	#[derive(Clone, Copy, Debug, Eq, PartialEq)]
    67	pub struct IoFault {
    68	    /// Surface which fails.
    69	    pub operation: Operation,
    70	    /// Successful prefix admitted before failure.
    71	    pub after: usize,
    72	    /// Whether `after` counts operations or bytes.
    73	    pub unit: FaultUnit,
    74	}

    180	        let delay = delays.get(*step).copied().unwrap_or(0).min(2);

    199	            (Operation::Flush, _) => self.report.flushes,
    200	            // Supply operations transfer no bytes; only operation counting
    201	            // is meaningful for them.
    202	            (Operation::Connect, _) => self.report.connects,
    203	            (Operation::Accept, _) => self.report.accepts,

Resolution: Either reshape `IoFault` as an enum carrying the unit only on the byte-moving surfaces (`Read { after, unit }`, `Write { after, unit }`, `Flush { after }`, `Connect { after }`, `Accept { after }`) so failures.rs:184-189 disappears, or document on `IoFault::unit` that it is ignored for `Flush`/`Connect`/`Accept`. Introduce `const MAX_DELAY_POLLS: u8 = 2;` with its reason (a schedule must not starve the peer) and cite it from the three `*_delays` field docs, or drop the clamp and let the generators own the bound; state "a chunk of 0 reads as 1" on the chunk fields or reject 0. Acceptance: no wildcard over `FaultUnit` remains, or the field docs name every ignored case; `grep -n 'min(2)' src/testing/transport.rs` is empty; `every_transport_fault_surface_is_reachable` still passes.

### testing-infra-9: `State` spells the fault-arming rule three ways, with match arms no call site reaches
- Where: src/testing/transport.rs:186-216 (related: src/testing/transport.rs:220-231, 234-244, 247-263, 284-292, 304, 371)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (call sites checked: `failure(Operation::Read)` only at 288 under `injected.is_some()`, which `failure` returns on at 191-193; `remaining_bytes` called only with `Read` at 304 and `Write` at 371)
- Seen by: structure-prose [7]; blind-spots [25] (Flush arm); refutation: confirmed; history: deliberate-but-expired (the `(Read, _)` arms were live until b3b877d9b gated the read path; the `Flush` arm in `remaining_bytes` has been unreachable since birth)
- Owner-gated: no

`failure(Operation::Read)` is only ever called when `injected` is already set, and `failure` returns early in exactly that case, so the `(Operation::Read, _)` arms at 195-196 never execute; `read_fault_armed` and `inject_read` restate the arming test and the `InjectedIo` construction that `failure` already holds, and `inject_read` writes `get_or_insert` then `expect("just recorded")` on the same field (242-243). `remaining_bytes`'s `Operation::Flush` arm (257) is reached by no caller. Legibility: finished code should be obviously correct, and three spellings of "fire once the prefix is spent, then keep firing" hide the one deliberate special case (reads fire post-poll on payload).

Evidence:

    194	        let completed = match (operation, fault.unit) {
    195	            (Operation::Read, FaultUnit::Operations) => self.report.reads,
    196	            (Operation::Read, FaultUnit::Bytes) => self.report.read_bytes,

    242	        self.report.injected.get_or_insert(injected);
    243	        io::Error::other(self.report.injected.expect("just recorded"))

    257	            Operation::Flush => self.report.flushes,

Resolution: Split into `fn armed(&self, op) -> bool` and `fn inject(&mut self, op) -> io::Error` (idempotent via `let injected = *self.report.injected.get_or_insert(...)`). Write/flush/connect/accept: `if state.armed(op) { return Err(state.inject(op)) }`; read: `armed` before the unclamped read, `inject` on a payload-bearing result. `remaining_bytes` matches `(Read | Write, Bytes)` and returns `usize::MAX` otherwise. Acceptance: one arming predicate and one injector; every match arm in `State` is reachable from a call site; the two inline transport tests and the proxy `failures.rs` suite pass unchanged.

### testing-infra-10: A `debug_assert` that restates tokio's `ReadBuf` contract, and an assertion of unit against unit
- Where: src/testing/transport.rs:328-328 (related: src/testing/transport.rs:309, 880)
- Class / severity / confidence: vestigial / nit / high
- Provenance: assessed (read; `before` at 309 has no other use)
- Seen by: structure-prose [11]; blind-spots [29]; refutation: confirmed (the `drop(survivor)` half of [11] is superseded by testing-infra-22); history: both arrived in 77674c9c0 with no comment
- Owner-gated: no

After `buf.advance(read)`, the assert checks that `filled()` grew by `read`, which is `ReadBuf::advance`'s documented postcondition; it cannot fail without a bug in tokio and samples nothing about this wrapper. `assert_eq!(sent, ());` at 880 asserts unit against unit. Doctrine on asserts: a guard names a concrete, constructible failure the tests cannot catch; a recompute of a dependency's contract names none.

Evidence:

    328	                debug_assert_eq!(buf.filled().len() - before, read);

    880	        assert_eq!(sent, ());

Resolution: Delete line 328 and the `before` binding at 309; delete line 880 and bind `sent` as `_`. Acceptance: neither line remains; tests otherwise unchanged.

### testing-infra-11: `wrap_io` is exported with no consumer, `wrap_link` rebuilds its state instead of using it, and `AdversarialLink` cannot be named by callers
- Where: src/testing/transport.rs:451-475 (related: src/testing.rs:7-12; src/testing/transport.rs:489-537, 540-545, 628-632; tests/lifecycle.rs:70-71; tests/reuse.rs:140-141)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`grep -rn wrap_io src tests benches examples`: the re-export at testing.rs:10 and this file's definition and two inline tests only; `grep -rn AdversarialLink src tests benches examples`: transport.rs:502 and 540 only)
- Seen by: structure-prose [15]; blind-spots [26]; api-economics [40]; refutation: confirmed (severity lowered: the module is `doc(hidden)`, so no rendered docs expose the unreachable name); history: deliberate-but-expired (`wrap_io`'s external callers were migrated to `wrap_link` by b3b877d9b; the export and the "extended over a whole link" framing survived)
- Owner-gated: no

`wrap_io` is re-exported but its only callers are the two unit tests at the bottom of this file; `wrap_link`'s doc frames itself as `wrap_io` "extended over a whole link" (494-497) but duplicates the `State` literal (510-517 against 458-465) instead of building on it, and the `AdversarialRead { inner, state, delay: None }` literal appears three times (467-471, 519-523, 628-632) while only `wrap_write` exists. `AdversarialLink` is `pub type` in the private module and absent from the facade's export list, so `wrap_link`'s return type cannot be written by tests/lifecycle.rs or tests/reuse.rs. Principle 3: a function whose only callers are its own tests is a candidate for dissolution or for becoming the primitive it is documented as.

Evidence:

    452	pub fn wrap_io<R, W>(
    453	    side: Side,
    454	    plan: IoPlan,
    455	    read: R,
    456	    write: W,
    457	) -> (AdversarialRead<R>, AdversarialWrite<W>, IoReportHandle) {
    458	    let state = Arc::new(Mutex::new(State {
    459	        side,
    460	        plan,
    461	        report: IoReport::default(),
    462	        read_step: 0,
    463	        write_step: 0,
    464	        flush_step: 0,
    465	    }));

Resolution: Add `State::new(side, plan)` and a `wrap_read` twin of `wrap_write`; have `wrap_link` and the acceptor use them. Either route `wrap_link`'s control halves through `wrap_io` (sharing the state `Arc` with the connector and acceptor wrappers) or narrow `wrap_io` to `pub(super)` and drop it from the facade, re-pointing its two tests at `wrap_link` over `link::memory()`. Add `AdversarialLink` to the `pub use transport::{...}` list. Acceptance: one `State` constructor; `wrap_io` has a non-test caller or is not exported; `rumors::testing::AdversarialLink` is importable.

### testing-infra-12: `ReorderingAcceptor`'s inversion has no committed demonstration that it ever fires
- Where: src/testing/transport.rs:660-669 (related: src/testing/transport.rs:646, 679-721, 730-741, 743-775; src/tree/mirror/streaming/remote/proxy/tests.rs:113-136, 502-565; src/conformance/link/tests.rs:41-91, 146-149, 916-919)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`grep -rn 'reorder_accepts\|reordered.load'` over src and tests: the only load on this decorator's counter is proxy/tests.rs:559-564 asserting `0`; the `> 0` loads at conformance/link/tests.rs:147 and 917 are on `ReversingAcceptor`; both implementations read side by side)
- Seen by: blind-spots [18]; refutation: confirmed; history: deliberate-and-holds for the proxy-tier `== 0` tripwire (cbc4a0aa2 records the topology argument), but the same commit made the two acceptors different mechanisms, so the doc's "proven instead by the conformance suite's `ReversingAcceptor` tests" is a claim about another type, and the patience-wait design has no witness
- Owner-gated: yes: option (b) below reopens cbc4a0aa2's recorded decision; option (a) does not

`reorder_accepts` has one consumer, `wide_symmetric_accepts_reordered_match_local`, whose final assertion is `reordered == 0`, and the doc it defers the proof to (proxy/tests.rs:513-515) names `ReversingAcceptor`, a single-noop-waker-poll drain (conformance/link/tests.rs:71-84), not this patience-bounded wait (transport.rs:692-715). So `REORDER_PATIENCE`, `yield_once`, and the wait this doc calls "load-bearing" have no committed demonstration that they ever produce a batch of two, and the property they exist to exercise at the proxy tier (streams paired by label under arrival inversion) is untested there. Principle 6 (adequacy): a decorator that degenerated to pass-through passes the only test that uses it.

Evidence:

    660	/// Every batch of two or more is a genuine inversion, recorded in the
    661	/// shared `reordered` counter so a test can assert the adversity's actual
    662	/// disposition instead of assuming it. The genuine wait is load-bearing: a
    663	/// decorator that only drains arrivals already `Ready` never sees a second
    664	/// arrival under the deterministic scheduler and silently degenerates to
    665	/// pass-through — which is exactly what the asserted counter makes loud.

    559	    assert_eq!(
    560	        reordered.load(Ordering::Relaxed),
    561	        0,                                                (proxy/tests.rs)

Resolution: (a) Add a unit test in transport.rs's tests: build a `memory()` pair, wrap one acceptor with `reorder_accepts(link, 2, counter)`, and under `run_to_quiescence` `join!` a task that connects two streams with a task that accepts twice; assert the release order is reversed and the counter reads 1 (the second connect lands during the acceptor's `yield_once`, so the patience loop is exercised, not merely present). Rewrite proxy/tests.rs:513-515 and transport.rs:667-669 to cite that witness and to describe the two acceptors as deliberately different. (b) Alternatively dissolve `ReorderingAcceptor`, `reorder_accepts`, `yield_once`, `REORDER_PATIENCE`, and `reconcile_symmetric_accepts_reordered`, leaving the conformance suite's link-level inversion proof. Recommendation: (a). Acceptance: a committed test asserts `reordered > 0` for `ReorderingAcceptor` with a batch of two released newest-first, or the decorator and its consumer are gone; no doc claims the conformance tests prove this implementation.
Construction: Replace the body of `ReorderingAcceptor::accept` with `self.inner.accept().await` and run the workspace tests: nothing fails.

### testing-infra-13: The `test-internals`-alone surface that justifies duplicating `yield_once` is never built
- Where: src/testing/transport.rs:726-729 (related: src/testing/transport.rs:730-741; src/conformance/link.rs:680-696; justfile:507-522; Cargo.toml:108-116, 145; src/lib.rs:306-307, 319-321)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (both `yield_once` bodies read, identical; `grep -n test-internals justfile` returns nothing; justfile:520-522 read; Cargo.toml:145 read)
- Seen by: api-economics [35]; structure-prose [4]; refutation: reframed (the seam rationale is correct for the direction it states; conformance-alone is built at justfile:522, test-internals-alone never); history: deliberate-and-holds for the duplication (59827c7a6 names the seam), with the recipe gap unrebutted
- Owner-gated: no (the `features` recipe is outside the gate, justfile:502)

`yield_once` is duplicated byte for byte across `testing` and `conformance` so that each feature builds without the other, and that independence is the stated reason for the copy. The `features` recipe's own goal is "every cfg-gated surface on its own, so nothing rots behind `--all-features`", yet it checks `conformance` alone and never `test-internals` alone, and every in-crate build turns both on (Cargo.toml:145), so the surface the duplication pays for is never compiled. Principle 6: a cfg surface no leg builds is one `--all-features` hides rot behind.

Evidence:

    726	/// Runtime-agnostic (the deterministic driver is no runtime at all), unlike
    727	/// `tokio::task::yield_now`; a copy of `conformance`'s helper, on the same
    728	/// feature seam that keeps [`ReorderingAcceptor`] separate from its
    729	/// `ReversingAcceptor` sibling.

    520	    cargo check -p rumors --no-default-features               (justfile)
    521	    cargo check -p rumors --features meter
    522	    cargo check -p rumors --no-default-features --features conformance

Resolution: Add `cargo check -p rumors --no-default-features --features test-internals` to the `features` recipe. Optionally put one `yield_once` in a private module gated `#[cfg(any(test, feature = "conformance", feature = "test-internals"))]` and import it from both sides, which satisfies the seam without the copy. Acceptance: `just features` builds `test-internals` alone and passes; one `fn yield_once` in `src/`, or the copy's doc still states the seam accurately.

### testing-infra-14: Em-dashes in `//` comments
- Where: src/testing/transport.rs:689-690 (related: src/tests.rs:271-272, 399, 457, 564, 612)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`grep -n '^\s*//[^/!].*—'` over the four files returns exactly these eight lines)
- Seen by: structure-prose [9]; refutation: confirmed; history: already-known house rule (R76 in the 2026-07-23 review packet); nothing enforces it mechanically, and two of the sites postdate the ruling
- Owner-gated: no

Eight `//` comment lines use em-dashes. Doctrine: colons or semicolons over em-dashes in log messages and code comments (terminal compatibility); rendered `///` prose is not at issue.

Evidence:

    689	        // a fresh inner accept once — dropping it while pending is exactly
    690	        // the cancellation tolerance the link contract demands — then

Resolution: Rewrite each with a colon, semicolon, or parentheses. Acceptance: the grep above is empty.

### testing-infra-15: The `src/tests.rs` module doc describes only the party tests
- Where: src/tests.rs:1-6 (related: src/tests.rs:446-471, 536-618, 620-624)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (file read in full; the tests at 449, 476, 549, 625, 644, 667 are not about parties)
- Seen by: structure-prose [14]; api-economics [37] (module-doc half); refutation: confirmed; history: deliberate-but-expired (true at bb3c4f2a9; expired across 61bd5de70, 7c9175a0b, 9d35b663c, none of which touched it)
- Owner-gated: no

The charter scopes the file to "party mechanics" reachable only through a forged `Peer` or a party read. The file also holds link-poisoning tests, the uncontained-supply test (which needs `tree::arb::poisoned_root`), and three root-hash meter pins (which need `tree::meter`), and line 623 positions the pins by count ("the two commit-path pins below"). A module doc's first sentence must be true of the module; hand-maintained positional counts rot.

Evidence:

    1	//! Crate-level unit tests for party mechanics that the public integration tests
    2	//! can't reach.
    3	//!
    4	//! They need either a *forged* `Peer` (private fields) or to read a `Peer`'s
    5	//! [`Party`] and compare it to [`Party::seed`]. Both require in-crate access,
    6	//! so they live here rather than in `tests/`.

    623	/// The liveness leg for the two commit-path pins below — a ceiling asserted

Resolution: Restate: "Crate-level tests that need in-crate access: party linearity across bootstrap and retire, retire sessions severed at exact frame boundaries, link poisoning, the containment violation at the API tier, and the root-hash read meter." At 623, name the pins instead of counting them. Acceptance: the module doc names every family of test in the file; no "the N ... below" phrasing.

### testing-infra-16: `PREAMBLE_LEN` and `greeting_frame_len` docs describe wire shapes that no longer exist
- Where: src/tests.rs:25-28 (related: src/tests.rs:262-264, 271-272; src/tree/mirror/handshake.rs:12-17, 45-53; src/tree/mirror/streaming/remote/codec/greeting.rs:1-8)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (handshake.rs:45 `const V2_PREFIX: [u8; 11]`, :53 `V2_PREAMBLE_LEN = V2_PREFIX.len() + 1 + (1 + NETWORK_LEN) + 1` with `NETWORK_LEN = 16`, i.e. 30; handshake.rs:16 "the item is 30 bytes, fixed"; `git blame -L 25,28` dates lines 25-26 to f6cf2579 (2026-07-16) and line 28 to 4dd2053c (the CBOR wire); codec/greeting.rs:3 "One control-stream item")
- Seen by: structure-prose [0]; blind-spots [21]; api-economics [33]; refutation: confirmed; history: deliberate-but-expired (the breakdown was maintained through two earlier layout changes; 4dd2053c changed only the constant line in that hunk and left both docs as context)
- Owner-gated: no

The doc sums the preamble as magic(6) + proto_version(2) + network(16) + intent(1) = 25, but the constant it aliases is the 30-byte self-described CBOR item, and none of the four named fields exists in that shape; a reader hand-checking the three fuse budgets gets a wrong number. `greeting_frame_len`'s doc describes "the causal-version frame plus the root-fan listing frame" while the greeting is one tag-24 item and the helper's own comment at 272 says "measure its one wire item". AGENTS.md hard rule and Principle 5: nothing refers to code or layouts that no longer exist; no hand-maintained arithmetic restates what a constant computes.

Evidence:

    25	/// The preamble's wire length: magic(6) + proto_version(2) + network(16) +
    26	/// intent(1). The fault-injection budgets
    27	/// below land cuts on exact protocol boundaries relative to this.
    28	const PREAMBLE_LEN: usize = crate::tree::mirror::handshake::V2_PREAMBLE_LEN;

    262	/// The wire length of `retiree`'s complete greeting — the causal-version
    263	/// frame plus the root-fan listing frame — so a [`Fuse`] budget can land on
    264	/// an exact protocol boundary.

Resolution: Rewrite 25-27 as "The preamble's fixed wire length, the handshake's own constant, so the fuse budgets below land on exact protocol boundaries." (or drop the alias and import `V2_PREAMBLE_LEN`, whose doc at handshake.rs:50-52 carries the correct decomposition). Rewrite 262-264 as "The wire length of `retiree`'s greeting item, the one control-stream item after the preamble, so a fuse budget can land on an exact protocol boundary." Acceptance: neither doc contains a byte breakdown or the word "frame" for the greeting; handshake.rs:16 remains the single place the width is stated.

### testing-infra-17: `party_of` and `with_messages` are duplicated between `src/tests.rs` and `src/peer/gossip/tests.rs`
- Where: src/tests.rs:30-45 (related: src/peer/gossip/tests.rs:225-245; src/peer.rs:145-169; src/lib.rs:319-321)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`grep -rn 'fn party_of\|fn with_messages\|fn provider_with' src tests`; gossip/tests.rs:237-245 body read, identical to tests.rs:38-45 save the parameter name; `provider_with` at 226-234 is `send_all` over a seed)
- Seen by: structure-prose [3]; refutation: confirmed; history: no rationale found (the copy arrived with db99759db)
- Owner-gated: no

`party_of` has a byte-identical body in `src/peer/gossip/tests.rs`, and `provider_with` there is `with_messages(Peer::seed(), values)`. `Peer`'s fields are `pub(crate)` and `testing` is compiled for every in-crate test build, so one home is possible; the `dangerously_alias` escape hatch in particular should have one home with one caveat.

Evidence:

    37	/// Read a `Peer`'s party for assertions.
    38	fn party_of(k: &Peer<u64>) -> Party {
    39	    k.inner
    40	        .borrow()
    41	        .party
    42	        .as_ref()
    43	        .expect("a live Peer holds its party")
    44	        .dangerously_alias()
    45	}

Resolution: Host `pub(crate) fn party_of<T>(&Peer<T>) -> Party` and `with_messages` in a `cfg(test)` submodule of `crate::testing` (so the aliasing helper does not ship under `test-internals`); both suites import them and `provider_with` goes. Acceptance: `grep -rn 'fn party_of' src` returns one definition under `src/testing` (plus the unrelated label-based helper in `src/tree/tests.rs`); `provider_with` is gone.

### testing-infra-18: Closed-world sessions in `src/tests.rs` run under `pollster`, forgoing the crate's stall detector
- Where: src/tests.rs:49-53 (related: src/tests.rs:66, 112, 192, 350, 664-665, 678; tests/common/wire.rs:39-42; src/testing.rs:375-394; .config/nextest.toml:1-5, 26)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (`grep -n pollster src/tests.rs`: 50, 66, 112, 192, 350, 678 drive memory-link sessions, 269 is the pure `greeting_fan` computation; `grep -n run_to_quiescence src/tests.rs`: 460, 466, 484, 488, 585, 613)
- Seen by: blind-spots [24]; api-economics [32]; refutation: confirmed (severity lowered: no present defect; the gain is a fast, attributed failure on a future liveness regression); history: the pollster helpers predate the poller; every later test in the file adopted the poller except 9d35b663c's, whose single-thread argument the poller satisfies equally
- Owner-gated: no

Six closed-world sessions are driven by `pollster::block_on`, which parks the thread until a wake a stalled future never delivers, so a reintroduced wire stall in retire or bootstrap fails here as nextest's 180 s kill with no diagnosis. The same file drives its stall-sensitive tests through `run_to_quiescence`, which turns the same stall into a deterministic `Quiescence::Stalled` in milliseconds, and .config/nextest.toml:1-5 names that poller as the intended liveness mechanism. `tests/common/wire.rs:39-42` already spells the one-liner as `block_on`.

Evidence:

    49	fn retire_child_into(survivor: Peer<u64>, child: Peer<u64>) -> Peer<u64> {
    50	    pollster::block_on(async {
    51	        let (mut a_link, mut b_link) = memory();
    52	        let (child_out, survivor_out) =
    53	            tokio::join!(child.retire(&mut a_link), survivor.gossip(&mut b_link),);

Resolution: Replace each session-driving `pollster::block_on(async { ... })` with `run_to_quiescence(async { ... }).expect("the closed in-memory session stays live")`; consider exporting that one-liner from `testing` as `block_on` with `#[track_caller]` and its poll budget stated in the doc, so `src/tests.rs`, `tests/common/wire.rs`, and the integration suites stop re-deriving it. Reword the doc at 664-665 to name the poller. Acceptance: `grep -n pollster src/tests.rs` returns only line 269 or nothing; the tests pass; a deliberately stalled variant fails with `Quiescence::Stalled` in under a second.

### testing-infra-19: Two testdocs omit an assertion the body makes
- Where: src/tests.rs:82-88 (related: src/tests.rs:120-129, 362-368, 384-387)
- Class / severity / confidence: test-quality / nit / high
- Provenance: verified (docs and bodies read side by side)
- Seen by: structure-prose [16]; refutation: confirmed; history: the `Retire::Uncertain` assertion was added by 7979639ff without touching the doc; the `is_err()` assertion and its doc were written together in c3230533f
- Owner-gated: no

`overlapping_retiree_party_is_rejected`'s doc states only the absorber's `PartyOverlap` while the body also asserts `Retire::Uncertain { .. }` for the forged side (126-129); `severed_descent_recovers_the_retiree`'s doc speaks only of the retiree while the body asserts `peer_out.is_err()` (384-387). AGENTS.md: a test's doc comment states its invariant and must be accurate.

Evidence:

    82	/// A peer that absorbs a retiree whose party **overlaps** its own rejects it
    83	/// with [`Error::PartyOverlap`] rather than corrupting its clock.

    126	    assert!(
    127	        matches!(retire_out, Retire::Uncertain { .. }),

Resolution: Add one clause to each doc: "the forged retiree is consumed as `Retire::Uncertain`, never falsely `Retired`" and "the absorbing side fails too". Fold in with testing-infra-22's doc updates. Acceptance: each assertion in the two bodies corresponds to a sentence in its doc.

### testing-infra-20: `Fuse`/`FusedConnector`/`fused_link` reimplement `wrap_link`'s byte-counted write fault
- Where: src/tests.rs:209-337 (related: src/tests.rs:345-360; src/testing/transport.rs:51-74, 186-216, 247-263, 360-375, 498-537, 587; src/peer/gossip.rs:450-455; src/link.rs:598-604; tests/lifecycle.rs:193-201; tests/common/fault.rs:1-27, 243-251)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (both mechanisms traced by reading: `Fuse::poll_write` admits `min(buf.len(), remaining)` then fails at zero (233-250); `AdversarialWrite::poll_write` clamps to `remaining_bytes(Write)` and `failure(Write)` fires once `write_bytes >= after` (360-375, 194-207); both share one budget across the control write and every connected stream (316-337 against 509-535); with a Write fault, `failure(Read | Accept | Connect)` returns `None` at 188, so reads and supply are unaffected in both. Retire classifies by `(intent, result)` alone (gossip.rs:450-455); `grep -rn 'ErrorKind::\|\.kind()' src` outside tests finds kind matches only on `UnexpectedEof` (party.rs:118-119, 154; codec decode; routed endpoint), so `BrokenPipe` versus `io::Error::other(InjectedIo)` is assertion-neutral)
- Seen by: structure-prose [1]; blind-spots [20]; api-economics [31]; refutation: confirmed ([31]'s latent `Done` claim tempered: `fused_link` takes `MemoryLink`, whose connector returns `Done::discard()`, link.rs:604); history: no rationale found (b3b877d9b introduced `wrap_link` and rewrote `Fuse` into the shared-budget shape in the same commit without saying why it stayed; 4be4b830 then re-plumbed `Fuse` for `Done`, the cascade this predicts)
- Owner-gated: no

About 130 lines implement a write-side byte fuse over a whole link that `testing::wrap_link(side, IoPlan { fault: Some(IoFault { operation: Write, after: budget, unit: Bytes }), ..IoPlan::default() }, link)` already provides, with an `IoReportHandle` besides; `tests/common/fault.rs` is a third copy. The public tier already severs sessions through `wrap_link` (tests/lifecycle.rs:193-201) while the in-crate tier hand-rolls it. Principle 3: the harness exists so suites do not each own a fault injector; three fuses is three places a change to `Link` or `Done` must land.

Evidence:

    211	/// An [`AsyncWrite`] wrapper that forwards writes until a byte budget is
    212	/// exhausted, then fails every write with [`BrokenPipe`]: a deterministic
    213	/// stand-in for a connection severed at a chosen point in the session.
    214	///
    215	/// The budget is shared across every fused writer of one link — the control
    216	/// half and each data stream — so the cut lands at a chosen point in the
    217	/// session's total outgoing byte count, wherever that byte travels.
    ...
    316	/// Fuse one link's whole outgoing side to a shared byte budget.
    317	fn fused_link(

Resolution: Delete `Fuse`, `FusedConnector`, and `fused_link`; in `severed_retire` build `let plan = IoPlan { fault: Some(IoFault { operation: IoOperation::Write, after: budget, unit: IoFaultUnit::Bytes }), ..IoPlan::default() }; let (mut a_link, report) = wrap_link(IoSide::Left, plan, a_link);` and optionally assert `report.snapshot().injected.is_some()` to prove the cut fired. Drop the `AsyncWrite`, `Pin`, `Context`, `Poll`, `Connector`, `MemoryConnector`, `MemoryAcceptor`, `Link` imports that only the fuse needed. `tests/common/fault.rs` belongs to another partition; its header states the one capability `IoPlan` lacks (a severed direction also refuses connect/accept), which is the gap to close before it can follow. Acceptance: `src/tests.rs` contains no `impl AsyncWrite` and no `Connector` impl; the four `severed_*` tests pass with their assertions untouched; `grep -rn 'struct Fuse' src tests` returns at most the `tests/common/fault.rs` definition.

### testing-infra-21: The epilogue-marker severance test's doc is off by one byte, and the accounting pin has one byte of high-side slack
- Where: src/tests.rs:423-427 (related: src/tests.rs:408-409, 233-250, 514-523; src/peer/gossip.rs:54, 1291; tests/lifecycle.rs:188-190, 196 (same expired premise, outside this partition))
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (gossip.rs:54 `const EPILOGUE_MARKER: [u8; 2] = [0x61, b'.'];`, written as one `write_all` at 1291; the fuse arithmetic in `Fuse::poll_write` (233-250) walked for budgets sum-1, sum, sum+1, sum+2: at sum the marker write fails whole; at sum+1 one marker byte is admitted and the retry fails, so the child is still `Uncertain(Epilogue)` and the absorber still fails its two-byte `read_exact` with `Epilogue`; at sum-1 the party frame is truncated and the `Err(Error::Epilogue(_))` assertion at 439-442 fails)
- Seen by: blind-spots [19]; refutation: confirmed; history: deliberate-but-expired (7979639ff introduced "one epilogue marker byte" and this test; 4dd2053c made the marker two bytes and did not touch the comment)
- Owner-gated: no

The doc says the cut lands on "the last byte of a clean retire session" and the budget is "that full clean session minus one byte", but the marker is two bytes, so the budget is the clean session minus two. An over-count of exactly one byte in `greeting_frame_len + party_frame_len` therefore passes both this test and `severed_party_frame_is_uncertain`, and no dual with the full budget proves the sum from below. AGENTS.md: an inaccurate testdoc is a bug in the test. Principle 6 (shape over point): a boundary pin needs both sides of the boundary.

Evidence:

    423	    // Both empty and converged, so the retiree's outgoing bytes are exactly
    424	    // preamble + greeting + party frame + epilogue marker. The budget is
    425	    // that full clean session minus one byte: everything through the party
    426	    // frame is delivered, and the marker write is the write that fails.
    427	    let budget = PREAMBLE_LEN + greeting_frame_len(&child) + party_frame_len(&child);

    54	const EPILOGUE_MARKER: [u8; 2] = [0x61, b'.'];      (gossip.rs)

Resolution: Make `EPILOGUE_MARKER` `pub(crate)`; set `budget = PREAMBLE_LEN + greeting_frame_len(&child) + party_frame_len(&child) + EPILOGUE_MARKER.len() - 1`, which makes "minus one byte" true and pins over-counting by any amount; reword 408-409 to "the last byte of the two-byte epilogue marker". Add the dual `severed_after_marker_is_retired` with `budget = ... + EPILOGUE_MARKER.len()`, asserting `Retire::Retired` and the peer `Ok`, which pins under-counting. Acceptance: both tests exist; injecting `+ 1` into `party_frame_len` fails the minus-one test (`Retired`), and injecting `- 1` fails the full-budget test (`Uncertain`).
Construction: Add `+ 1` to `party_frame_len`'s return: `severed_party_frame_is_uncertain` and `severed_epilogue_marker_is_uncertain` both still pass today.

### testing-infra-22: The absorber's party is never read after a failed hand-off
- Where: src/tests.rs:437-443 (related: src/tests.rs:1-6, 82-83, 112-118, 529-533; src/peer/gossip.rs:816-829, 872-873, 884-886, 897-899; crates/before/src/party.rs:298-300; crates/before/src/clock.rs:199-202; src/link.rs:300; tests/retire.rs:16-20)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (the three tests read end to end; gossip.rs read: the absorber's `existing.join(party)` (816-829) and bookmark persist (884-886) precede `epilogue` (897), so an absorber reporting `Err(Epilogue)` holds the donated region, and on overlap the function returns at 872-873 before the epilogue with `send_if_modified` having returned `false`; `Party::join` computes `self.view().sum(other.view())` before mutating (party.rs:299-300) and `Clock::join`'s Errors clause states `self` is unmodified on overlap (clock.rs:201-202); tests/retire.rs:19-20 delegates party accounting to this file)
- Seen by: blind-spots [17]; refutation: confirmed; history: no rationale found (7979639ff's own comment states the post-condition the test then discards)
- Owner-gated: no

Three tests fail the identity hand-off and discard the absorbing peer without reading its party: `overlapping_retiree_party_is_rejected` moves `survivor` into its future (116) and asserts only the two error shapes, though its doc claims the rejection happens "rather than corrupting its clock"; `severed_epilogue_marker_is_uncertain` says in a comment that "The absorber committed the party" and then `drop(survivor)`; `severed_party_frame_is_uncertain` likewise ends in `drop(survivor)`. The absorber-side outcome is the other half of every identity-accounting claim these tests make, the session promise's post-commit exception (link.rs:300) is asserted nowhere as a party value, and this file is the one place that can read a party. Principle 6: a test asserting a weaker property than its doc claims.

Evidence:

    437	    // The absorber committed the party before its own epilogue read hit the
    438	    // severed wire: it reports the same post-commit residue.
    439	    assert!(
    440	        matches!(peer_out, Err(Error::Epilogue(_))),
    441	        "the absorber's confirmation of the retiree's completion fails, got {peer_out:?}"
    442	    );
    443	    drop(survivor);

Resolution: In `overlapping_retiree_party_is_rejected`, borrow the survivor into the future (`let survivor = &survivor; async move { survivor.gossip(&mut b_link).await }`) and assert `party_of(&survivor) == Party::seed()` afterwards (never forked, and the overlapping join must leave it untouched). In `severed_epilogue_marker_is_uncertain`, replace `drop(survivor)` with `assert_eq!(party_of(&survivor), Party::seed(), "the absorber committed the donated region before its epilogue failed")`. In `severed_party_frame_is_uncertain`, capture `let before = party_of(&survivor);` ahead of the session and assert equality afterwards (the frame never arrived). State each absorber-side post-condition in the test doc. Acceptance: all three tests read the absorber's party after the failed session; each doc names the absorber-side invariant; the tests stay green.
Construction: Make the absorber's `existing.join(party)` at gossip.rs:820 a no-op on the success path and run `severed_epilogue_marker_is_uncertain`: it passes today, because nothing reads the survivor's party; with the proposed assertion it fails with the survivor's half-region instead of `Party::seed()`.

### testing-infra-23: `a_cancelled_session_poisons_the_link_for_gossip` reaches its claim through the `pub(crate)` `Peer::gossip`, though the public tier can express it
- Where: src/tests.rs:446-471 (related: src/peer/gossip.rs:459; src/rumors.rs:489; src/peer.rs:623; src/link.rs:378-386; tests/lifecycle.rs:60-124)
- Class / severity / confidence: test-quality / nit / medium
- Provenance: verified (`Peer::gossip` is `pub(crate)` at gossip.rs:459; `Rumors::gossip` is `pub` at rumors.rs:489; `into_rumors` is `pub` at peer.rs:623; tests/lifecycle.rs:67-124 read)
- Seen by: api-economics [37]; refutation: reframed (not subsumed: this test cancels during the handshake with no counterparty, exercising the poison latch's "even before its first byte" clause (link.rs:381-384), while lifecycle.rs cancels mid-descent; move rather than delete); history: no rationale found for the placement
- Owner-gated: no

The test pins a distinct cancellation point (stalled awaiting the peer's preamble, no counterparty at all) but needs in-crate access only because it calls the `pub(crate)` `Peer::gossip`; through `child.into_rumors()` and the public `Rumors::gossip` it can live beside `a_session_cancelled_mid_descent_poisons_the_link` in tests/lifecycle.rs, which is where a reader looks for link-poisoning behaviour. Doctrine: exercise the public API where it reaches the claim.

Evidence:

    446	/// A gossip session cancelled mid-flight poisons the link: the next session
    447	/// on it fails fast with [`Error::LinkPoisoned`], before any wire traffic,
    448	/// instead of misreading the interrupted session's leftover control bytes.
    449	#[test]
    450	fn a_cancelled_session_poisons_the_link_for_gossip() {

Resolution: Move the test to tests/lifecycle.rs, converting the child with `into_rumors()` and keeping the doc's "before the peer's preamble arrives" distinction explicit. Acceptance: the test lives in tests/lifecycle.rs beside its mid-descent sibling and passes; `src/tests.rs` no longer contains it.

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
