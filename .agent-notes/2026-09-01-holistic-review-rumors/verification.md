# Verification gaps and test quality

This document collects every finding of the holistic review of `rumors` at commit 9e5784fb4dce977cfbdfd1619886d1482b5ce764 whose primary class is *verification-gap* (something that could be wrong that the committed tests and instruments would not detect) or *test-quality* (a test weaker than its doc comment claims, or a harness that cannot see the failure it exists to catch). It answers two questions: what input family or property does no committed check reach, and which test would close it; and where does a test's doc comment promise more than its body asserts. Findings of other classes (correctness, documentation, simplification, and so on) that bear on a gap are cross-referenced by id and never reproduced here. Ids have the form `<partition or sweep key>-<n>`; the full record of each, including the lens reports it was distilled from, lives in `evidence/partitions/<key>.md` or `evidence/sweeps/<key>.md` beside this file in the committed note. Severity is the finalizers' scale: *high* (a claim the crate or its verification rests on is unprotected, or a harness masks a class of failure), *medium* (a stated property or an instrument's discriminating power is unpinned), *low* (a bounded coverage or accuracy gap with a cheap closure), *nit* (a local inaccuracy or idiom). Provenance is stated per finding: *verified* means the finalizer ran a command, grep, script, or read-only git query, or checked arithmetic against constants read in the tree; *assessed* means the claim rests on reading the cited code; *demonstrated* means one of the review's witness agents (the role is named for the `witness/` directory that holds its output) constructed the test the finding describes and ran it in a scratch worktree; the constructions and their output are in `witness/results.md`. For this document I re-checked every numbered evidence line of all 200 entries against the file and line it cites at HEAD (1522 lines; the fifteen my script could not resolve I checked by hand; the anchor pass that followed found two runs off by one, verification-infra-3 and remote-proxy-19, corrected in place), so the anchors below are verified even where a finding's claim is assessed. The conformance partition's correctness lens hung during the main run and was rerun after finalization; the five entries it added to this document (conformance-38 through conformance-42) went through the same refutation and history passes, none was constructed, and their anchors and evidence were checked by the anchor validator when they were merged. The scale rule applies: the classes together hold 203 entries (the finalizers filed 205; two cross-lens duplicates of the `future_size` defect stand as cross-references under tests-wire-format-26 and are not counted), so high, medium, and low entries appear in the finalizers' full template and the 30 nits are listed per module in compact tables that point at the evidence file for the full record.

## Highest-value items

1. The causality simulation's recycle oracle cannot see the recycle the bookmark exists to prevent: `EmissionLog::promote` flags only `later <= earlier`, while a reclaimed region's first emission compares `Greater` or incomparable, and redactions are untracked, so a mutant that reclaims every stored region on reboot destroys a durable message fleet-wide and `assert_healed` passes (demonstrated). The proptest's headline claim is judged by a blind oracle; the crate itself is covered only by the transmit-window pin (tests-bookmark-9).
2. The streaming prune's leaf-height deletion verdict has no committed test that fails when it is inverted: every fixture avoids the one shape (two leaves sharing 31 path bytes, the counterparty lacking the parent) that reaches the height-0 arm, and under the inversion the redacted leaf is re-supplied and absorbed. The cross-peer construction discriminates on HEAD and under the mutant (demonstrated) (materialized-27).
3. No frame ever crosses a link on a logical stream with index two or greater anywhere in the crate: every proxy-tier fixture is content-addressed, the committed wire snapshots stop at stream 1, and the legibility corpora cannot reach a three-byte shared prefix, so the leaf-tier decode pump, the terminal stream grammars, and fifteen of seventeen stream labels are validated against live traffic by nothing (remote-proxy-tests-10, demonstrated in the second witness pass by a frame census over the link, whose run also showed that the deep fixtures cannot close the gap as proposed because the wire correctly refuses their off-model leaf paths; with tests-wire-format-7 and tests-wire-format-18).
4. `tests/future_size.rs` is compiled out of every wired test run: the binary is gated `#![cfg(not(debug_assertions))]`, and the gate, CI, both coverage legs, and the mutants campaign run the dev profile, so its three budget tests have never executed under any committed check and whether they pass today is unknown (tests-wire-format-26, the entry of record; verification-infra-1 and suite-economics-1 refer there).
5. Two 64 KiB "tight" budgets sit below the window solve's flat decode-fan pre-charge (about 210 KB under `Local` pricing), so `Budget(64 KiB)` resolves to the same all-ones window as `Budget(0)`: the backend conformance suite's census ceiling compares two identical runs and cannot fail (conformance-28, demonstrated in the second witness pass: an inserted floor `budget_peak > floor_peak` fails at once with both peaks 14496), and `window_census`'s headline admittance assertion is `X <= X + admitted` (verified by running; tests-resource-link-window-20), with `window_corners.rs:174` in the same regime (tests-resource-link-window-28).
6. The causality harness swallows session errors that are unconditionally crate bugs under fault-free regimes: the serving task's result is dropped, boot-side errors fold to `None`, and a failed rejoin re-seeds a fresh universe, so a panic or decode failure in a serve path is invisible until a heal that catches only persistent failures (tests-bookmark-12, demonstrated in the second witness pass, where 452 injected `PartyOverlap` failures left the leakage property passing; the inter-process disruption parent has the same class, tests-disruption-handshake-7, correctness).
7. The overlap generator's shadow has no validity meta-test and its executor degrades a shadow mismatch to a skipped event on both sides, so corrupting the shadow's `Close` delivery leaves all 48 `session_overlap` cases passing (demonstrated); the shadow also snapshots at `Open` while the live session forks after the preamble (tests-observation-28, tests-common-12).
8. The backend conformance suite convicts less than the `Backend` docs say: `children`, the one population the window prices per depth, is charged but never priced pointwise (an interior-only over-hold passes the whole check; demonstrated, conformance-25); `assemble` is driven only at the root over one run, so two of its violation classes have never fired (demonstrated, conformance-31); and the `parent` Some/None clause records nothing, though the constructed run found the fault convicted by the assemble-length check rather than escaping (conformance-24).
9. `tools/testdoc` has three holes: it does not recognize `#[pollster::test]` (29 tests, all documented today, outside the gate's doc requirement; verified by running the tool: remote-proxy-tests-27, tests-disruption-handshake-33, tests-observation-37), it cannot see a `proptest!` block fn without an explicit `#[test]` (verification-infra-10), and its `.` walk reads another agent's worktree under `.claude/` (verification-infra-9).
10. The integer-envelope dominance behind the window's 2^-40 per-session bound is certified only by `examples/envelope_sim.rs`, which no recipe runs, which imports nothing from the crate, and which by its own header certifies the one-corpus family rather than the shipped pair-product functions; lowering a shipped quantile passes the gate (benches-envelope-32, streaming-backend-window-32).
11. Three instruments the doctrine asks for do not reach rumors: the mutation campaign is a hand run with no recipe, no confirming re-run, and a tree under test that predates the V1 retirement and the codec rewrite (verification-infra-3); the coverage pin's scope is `before`'s skyline kernel only, so rumors is instrumented on every CI run and never judged (verification-infra-4); and no fuzz target exists for any rumors decoder, the spec of record being unimplemented since 2026-07-27 (verification-infra-5).
12. The wire capture renderer's injectivity claim, which licenses the snapshot discipline, is sampled by no test: a generative leaf-mutation proptest refuted it after five cases through the container-key elision at `capture.rs:470` (demonstrated), and the scalar-key variant passed 2000 cases, bounding the hole (remote-capture-atlas-17).

## Crate-wide patterns

Each pattern is stated once here with its full site list; the per-module entries below carry the details and are not repeated.

- **Ceilings without liveness floors, and budgets that resolve to the floor.** A ceiling over a counter passes when the counter stops counting. The census ceiling in `conformance::backend::check` (conformance-28) and `window_census`'s admittance (tests-resource-link-window-20) both difference two runs at the identical all-ones window because a 64 KiB budget sits under the solve's flat pre-charge; `window_corners.rs:174` uses the same budget (tests-resource-link-window-28); the proxy ordering trace passes on an empty trace (remote-proxy-19); the catch-up hop ceilings in `window_corners` have no floor (tests-resource-link-window-25); the redaction dimension of the generated populations has no liveness pin though the membership alphabet has one (tests-lifecycle-15); the inter-process cut range has no pin of its own (tests-disruption-handshake-3); the `0..400` severed-connection cut range is unpinned (tests-disruption-handshake-17); `Quiescence::PollBudget` has no demonstration that it fires (testing-infra-4).
- **The same defect filed from several partitions.** The pollster hole in `tools/testdoc` (remote-proxy-tests-27, tests-disruption-handshake-33, tests-observation-37); `future_size.rs` never running (tests-wire-format-26, with verification-infra-1 and suite-economics-1 reduced to cross-references); the envelope certificate (benches-envelope-32, streaming-backend-window-32); the whole-subtree `shed` count never pinned above one (materialized-28, streaming-tests-26, tests-observation-25); the unpinned `HOP_BUDGET` floor regime (tests-disruption-handshake-10, verification-infra-16); `tradeoff_probe` unscheduled (verification-infra-14, tests-resource-link-window-18); no frames on streams two and above (remote-proxy-tests-10, tests-wire-format-7, tests-wire-format-18). Every entry is kept and cross-referenced to its siblings, except the two `future_size` duplicates, whose distinct evidence tests-wire-format-26 now carries.
- **Testdocs that claim more than their bodies check.** The largest single class in the test-quality set: api-core-20, link-12, materialized-22, remote-adapter-streams-29, remote-adapter-tests-4, remote-capture-atlas-19, remote-codec-17, remote-proxy-tests-2, remote-proxy-tests-9, remote-proxy-tests-25, session-bookmark-18, session-bookmark-42, streaming-backend-window-15, streaming-backend-window-36, streaming-backend-window-37, streaming-tests-18, swarm-example-30, testing-infra-19, testing-infra-21, tests-bookmark-4, tests-bookmark-8, tests-bookmark-21, tests-common-12, tests-common-20, tests-disruption-handshake-21, tests-disruption-handshake-22, tests-disruption-handshake-23, tests-disruption-handshake-25, tests-lifecycle-4, tests-lifecycle-21, tests-observation-17, tests-observation-20, tests-resource-link-window-7, tests-wire-format-5, tree-core-34, tree-typed-9. AGENTS.md holds every testdoc to accuracy; the gate's `testdoc` checks only that the comment exists, so review is the only enforcement, and these are the residue of code moving under prose written for an earlier shape (the V1 retirement, the CBOR respelling, the streaming swap, the two-byte epilogue marker).
- **Sessions driven by `pollster::block_on` instead of the closed-world poller.** A stall parks the thread until nextest's 180-second termination and loses the diagnosis (and, for proptests, the seed); `run_to_quiescence` reports `Err(Quiescence::Stalled)` in milliseconds. Sites: src/tests.rs (testing-infra-18), proxy/tests.rs (remote-proxy-tests-8), tests/changes.rs (tests-observation-8), the spawn-based bookmark suites with no stall bound (tests-bookmark-11), the paused-clock conformance runs with no timeout (tests-resource-link-window-10), and five negatives resting on 100 ms wall-clock windows in tests/gossip_when.rs (tests-disruption-handshake-14).
- **Convergence oracles weaker than whole-root equality.** Root-hash equality excludes the ceiling, the deletion mechanism: proxy greeting and declaration tests (remote-proxy-tests-15), the backend conformance run's two-sided agreement (conformance-30); `len()` equality alone in `window_corners` (tests-resource-link-window-27); frame-count comparisons with no convergence check in `target_message_size` (tests-resource-link-window-16); snapshot tests whose docs name a live set they never assert (tests-wire-format-5); the backend-fault property discarding the unfaulted endpoint's tree (remote-proxy-tests-7); a fleet-scale retirement asserting identity only (tests-observation-36).
- **Redaction absent from generated populations.** The conservation property never sheds (tests-observation-25); the chaos interleaving has no redaction arm (tests-disruption-handshake-18); the causal interleaving never redacts a remote or learned message (tests-observation-5); the when-model has no no-op redact (tests-bookmark-26); depth and redaction are never combined in a generated family (streaming-tests-7); the shed count is pinned only at 0 and 1 (materialized-28, streaming-tests-26); nothing pins that redactions stay in `arb_local_actions` or `arb_schedule` (tests-lifecycle-15).
- **Family claims stated as points, and finite rosters sampled rather than enumerated.** `Changes` (tests-observation-7, verification-infra-17), the routed link and handshake suites (verification-infra-17), the local-equivalence test hand-driving `TestRunner` (streaming-tests-23), two fault rosters drawn 256 times (streaming-tests-17), a measured stall-boundary table living in a comment (streaming-tests-16), driver cancellation sampled at one poll count (tests-disruption-handshake-16), the greeting round-trip as three fixtures (remote-codec-26), the preamble totality proptest that never passes the magic check (mirror-common-14), uniform `u64` corpus strategies that never sample small corpora (streaming-backend-window-38), `arb_query` unable to reach a 256-child fan (remote-capture-atlas-24), and the swarm controller judged on one seed (swarm-example-31).
- **Hand-transcribed constants beside a computed original.** `LABEL_LEN = 2` (tests-observation-24) and the "exactly two bytes" label testdoc (remote-adapter-streams-29); `FAN + 1` at five sites (remote-adapter-tests-9); `MAX_RECORD_LEN = 14` derived from retired framing (remote-adapter-tests-22); signal state codes copied into the proxy harness (remote-proxy-tests-21); `leaf_path` and the bookmark frame walk re-derived in tests/common (tests-common-6); the greeting `KEYS` order asserted by hand (remote-codec-26); `PRE_WRAP_SESSIONS = 253` (tests-lifecycle-28); the Lean wedge literal (streaming-tests-28); the branch-hash preimage restated in a bench (benches-envelope-1); the proptest version pinned in prose (tests-common-31).
- **Instruments no recipe runs.** `examples/envelope_sim.rs` (benches-envelope-32, streaming-backend-window-32), `tests/future_size.rs` (tests-wire-format-26), `tests/tradeoff_probe.rs` (verification-infra-14, tests-resource-link-window-18), the mutation campaign (verification-infra-3), the coverage legs outside every composite recipe (verification-infra-2), and the swarm example's rendezvous and shutdown claims (swarm-example-3). The justfile's opening claim that every artifact has a recipe is false for these.
- **Conformance-bug detectors with no committed demonstration.** Arms that reject a nonconforming peer or a misbehaving backend and that no test reaches: `label_item`'s four arms (remote-adapter-streams-28), nine proxy error arms (remote-proxy-31), the `Resolver`'s skip-past supply arm (materialized-37), `Work::execute`'s accept arm (remote-proxy-tests-26), `read_early`'s rejection arms (remote-adapter-tests-15), the backend-contract panics (remote-adapter-streams-14), the typed layer's `debug_assert!` guards (tree-typed-7), the three older link conformance checks (conformance-20), the legibility walker (tests-wire-format-18), six of nine `snapshot_liveness` conviction branches (tests-wire-format-20), the `ReorderingAcceptor` (testing-infra-12), the greeting rewriter (remote-proxy-tests-22), and the two driver terminal arms for a partially staged preamble (tests-disruption-handshake-15).

## Verification infrastructure: the gate, tools, CI, seeds, snapshots, coverage

The gate (`just gate`, justfile:400) is `gate-lints` (fmt-check, doclint, testdoc, workflowlint, manifestlint, digestshare, mutants-list, readme-check; justfile:403) plus eight concurrent `gate-streams`, of which the `workspace` stream runs `test-all` (`cargo nextest run --workspace --all-features`, dev profile); `ci` (justfile:1000) is the no-rot sweep GitHub runs, and `all` (justfile:1003) adds the fuzz smoke, the formal tier, and the bench judge. Against that roster the verification-infra sweep and the partition finalizers established the following, each verified by reading the recipes, running the tool, or querying CI read-only, never by running the gate itself. No root-workspace test recipe has ever run under the release profile, so the `cfg(not(debug_assertions))` future-size guard has never executed (verification-infra-1). `mutants-list` is list-only; the roster in `.cargo/mutants.toml` names no rumors file; the one campaign of record (scope A, 3141 mutants, 126 survivors, at 02560b1f) predates the V1 retirement and the codec rewrite, and its confirming re-run has no recorded outcome (verification-infra-3). `fuzz-build` and `fuzz` build only `crates/before/fuzz`; `design/rumors-frame-fuzz.md` is a spec of record unimplemented since 2026-07-27 (verification-infra-5). The coverage legs run only in CI's `coverage` job (failing at HEAD on a `before` meter), sit in no composite recipe, and judge only `crates/before/src/version/skyline/` (verification-infra-2, verification-infra-4). The 22 `.snap` files under `tests/snapshots` are swept by `tests/snapshot_liveness.rs` and the 33 seed files under `proptest-regressions/` by `tests/seed_liveness.rs`, both with vacuity guards, though the snapshot sweep judges only `tests/snapshots` (tests-wire-format-19, under the wire-format suites) and the seed sweep pins its proptest version in prose (tests-common-31, under tests/common). `tools/testdoc` recognizes five attribute forms and not `#[pollster::test]`, cannot see a `proptest!` block fn without an explicit `#[test]`, and walks untracked trees (the three pollster entries below, verification-infra-10, verification-infra-9). `ci` omits `manifestlint` while installing `cargo-mutants` unpinned against a checker that fails on any version but 27.1.0 (verification-infra-7, verification-infra-8). Two constraints a reader of every later section should hold: every in-crate test build enables both `test-internals` and `conformance` (Cargo.toml:145), so the `test-internals`-alone surface is never compiled (testing-infra-13); and nextest terminates a test after three 60-second periods (.config/nextest.toml:25-26), which is the only stall bound for any session not driven through `run_to_quiescence`. One related finding of another class: the `--cfg docsrs` rustdoc path is compiled by no leg, and it fails on the pinned nightly (deps-3, correctness, demonstrated). The three `future_size` entries and the three `pollster` entries are anchored in a test file and in `tools/testdoc` respectively; they are filed here because the defect in each is gate composition, not the content of a suite.

### verification-infra-1: The public-future size guardrail is compiled out of every wired test run

See tests-wire-format-26, the entry of record for `tests/future_size.rs` compiling to zero tests under every committed check; this sweep's anchors (Cargo.toml:180-182, justfile:1031-1042, .github/workflows/ci.yml:226-230), its release-leg resolution, and its `--no-tests=fail` liveness guard are carried there. The sweep's full entry is in `evidence/sweeps/verification-infra.md`.

### suite-economics-1: future_size guardrails compile to zero tests in every committed test run

See tests-wire-format-26, the entry of record; this sweep's run of the binary alone under the default profile (`Starting 0 tests across 1 binary`), the only executed evidence for the defect, and its history note on the by-hand release run are carried there. The sweep's full entry is in `evidence/sweeps/suite-economics.md`.

### tests-wire-format-26: future_size.rs never runs: its crate-level cfg excludes it from every committed check
- Where: tests/future_size.rs:17-20 (related: tests/future_size.rs:26-32; justfile:107-114, 597, 644, 1000; Cargo.toml:174-192; .config/nextest.toml:25-26; .cargo/mutants.toml:26-37; .github/workflows/ci.yml:107-108; src/peer/gossip.rs:1126-1128, 942-946)
- Class / severity / confidence: verification-gap / high / high
- Provenance: verified (read justfile:108-114: `test` and `test-all` are `cargo nextest run --workspace [--all-features]` with no profile; the only `--cargo-profile release` nextest legs at justfile:597 and 644 sit inside the `crates/before` fuzzfit and wasm32-pins harnesses; Cargo.toml:174-192 defines only `[profile.dev]` and `[profile.bench]` and its comment at 180 says `debug-assertions` stays on; ci.yml runs `just ci`, whose test leg is `test-all`; .cargo/mutants.toml:26-37 states the campaign runs under the dev/test profile; `grep -n -E 'future_size' justfile .github tools .cargo .config` returns nothing; `git show bf91938a` shows the 2026-08-12 budget bump 1024 to 2048, author finch, touching only this file; carried from suite-economics-1, verified by running: the binary alone under the default profile reports `Starting 0 tests across 1 binary`)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed, adding the mutants observer; history: no rationale found for the missing leg, and the cfg's original premise has expired (bdf10d4c's doc said debug layouts differ because the traverse trait dispatch "is itself boxed under `cfg(debug_assertions)` for stack safety"; that boxing left with the streaming swap, the sentence was generalized to "debug layouts carry additional state", and today the only `cfg(debug_assertions)` in src is a monotonicity `debug_assert` at window.rs:360, unrelated to layout). Carried from suite-economics-1's history: the release run has happened by hand (bf91938a, 2026-08-12, raised the budget 1024 to 2048; `.agent-notes/2026-08-19-height-erasure/README.md:105` records "verified in a release run after step 4"), but no committed recipe runs it, and `.cargo/mutants.toml:26-37` fixes the campaign of record to the dev/test profile
- Owner-gated: no
- Carried from verification-infra-1: its anchors beyond those above, Cargo.toml:180-182, justfile:1031-1042, and .github/workflows/ci.yml:226-230.

The whole binary is gated `#![cfg(not(debug_assertions))]`, and every rumors test run the repository configures (the gate, CI, coverage, the mutants campaign) is a dev-profile run with debug assertions on. The three budget tests therefore compile to an empty binary and have never executed under any committed check since the file was created; the guard against the downstream `recursion_limit` regression its module doc promises to catch "before downstream crates discover" it is decoration, and its 2048-byte budget has no committed evidence behind it (the doc at 28 still says "measured sizes (a few hundred bytes)" after a bump past 1024). A meter nothing runs is a ceiling with no liveness: every status board an acceptance concept exists for must be wired into the required checks. The by-hand release run the budget bump implies is exactly the convention held in memory the doctrine forbids.

Evidence:

    17	//! The budget is enforced only in release builds: debug layouts carry
    18	//! additional state, and they are not what users ship.
    19	
    20	#![cfg(not(debug_assertions))]

    108	test *args:
    109	    {{ justfile_directory() }}/tools/memwatch cargo nextest run --workspace {{ args }}
    113	test-all *args:
    114	    {{ justfile_directory() }}/tools/memwatch cargo nextest run --workspace --all-features {{ args }}

    180	# `debug-assertions` stays on, as the dev profile grants: the envelope

Run output carried from suite-economics-1 (the binary alone under the default profile):

    Starting 0 tests across 1 binary (59 binaries skipped)
    Summary [   0.000s] 0 tests run: 0 passed, 0 skipped
    error: no tests to run

Resolution: Lift the cfg and pin a budget that holds in both profiles: the claim is that the public futures hold only a `Pin<Box<dyn Future>>` plus locals, which is a layout fact in either profile; measure the three sizes once in dev and release (a handed number is a hypothesis), set `PUBLIC_FUTURE_BUDGET` with headroom, and state the measured band in its doc, replacing "a few hundred bytes". This keeps the gate's build set unchanged and keeps the guard inside the mutants observer, which a release-only leg would not. Add `gossip_when`'s stream (boxed at gossip.rs:946, unmeasured) to the measured surface. Add an adequacy demonstration: the commit landing the fix records that removing the `Box::pin` in `Reconciliation::reconcile` trips the budget. If the owner prefers a release leg instead, add `cargo nextest run -p rumors --test future_size --cargo-profile release` to `gate-streams` and `just ci` with a recipe comment stating why this one binary runs at release. Whichever option, add a liveness guard that the tests were collected (nextest's `--no-tests=fail`, or a count check in the recipe), since the failure mode is an empty binary reading as a pass (carried from verification-infra-1). Acceptance: `cargo nextest list -p rumors --test future_size` under the gate's invocation lists the three (or four) tests; `just gate` runs and passes them; a deliberate removal of the `Box::pin` at gossip.rs:1128 fails them.
Construction: `cargo nextest list -p rumors --test future_size` today lists no tests; `cargo nextest list -p rumors --test future_size --cargo-profile release` lists three. Remove the `Box::pin` at src/peer/gossip.rs:1128 and confirm only the release invocation notices.

Cross-reference: verification-infra-1 and suite-economics-1 report the same defect from the two sweeps and refer here; their distinct anchors and the run output are carried above. This entry is filed here rather than under the wire-format suites because the defect is gate composition, not the suite's content.

### verification-infra-2: The coverage legs sit in no composite recipe, so the full local ladder passes while CI's coverage job fails at this commit
- Where: justfile:1002-1003 (related: justfile:12-14, justfile:1005, justfile:1017-1018, .github/workflows/ci.yml:186-230, justfile:1031-1042)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read the `ci`/`all` recipe lines and the coverage section; `gh run list --workflow ci --branch main` and `gh run view 33567211421 --json jobs` / `--log-failed`, read-only)
- Verification: reframed: the sweep said "no local recipe runs the CI coverage legs"; the recipes `coverage-kernel` and `coverage-kernel-branch` exist and a developer can run them by name. What is true is narrower: neither `gate`, `ci`, nor `all` depends on them, so no composite invocation reaches them, and the header's "Everything" for `all` overclaims. Severity lowered from high to medium accordingly; the failing job itself is a `before` failure outside this partition. History: deliberate-and-holds for the gate exclusion (justfile:1005, 1017-1018 state it is too slow for the gate); no-rationale-found for their absence from `all`.
- Owner-gated: no

GitHub CI runs three jobs (ci, instruments, coverage). At HEAD the run for
this push (33567211421) failed: `coverage` failed at `just coverage-kernel`
while `ci` and `instruments` passed. The failure is
`before::meter masked_cmp_hole_envelope` (a peak-heap envelope, 1156 B
measured against a 480 B pin, under llvm-cov instrumentation; the same
test passes uninstrumented in the `ci` job at the same commit). Across the
last six main runs the coverage job failed twice (9e5784fb, 00e83cf7) and
passed three times, with only `Cargo.lock` differing under `crates/before`
between the last passing run (3327a92b) and this failing one. Whatever the mechanism, a
developer running `just all` had no way to see it.

Evidence:

    justfile
         2	# the workspace has a recipe here, tiered by feedback speed, and `just --list`
         3	# is the tour.
        12	# `ci` builds the artifacts the gate doesn't reach (the feature matrix, wasm,
        13	# bench builds, the viz bundle), exactly as GitHub CI builds them; `all` adds
        14	# what CI cannot run (the fuzz smoke and the formal tier). Neither sweep
      1002	# Everything: the no-rot sweep, plus the fuzz smoke, the formal tier, and the bench judge.
      1003	all: ci (fuzz fuzz_smoke_secs) lean eventdag muxprobe bench-judge bench-judge-tripwire
      1005	# ── the coverage legs (CI cadence; the gate never runs them) ─────────────────

    gh run view 33567211421 --json jobs
        {"conclusion":"failure","name":"coverage"}
        {"conclusion":"success","name":"instruments"}
        {"conclusion":"success","name":"ci"}

    gh run view 33567211421 --log-failed (ANSI stripped by hand)
        FAIL [   0.009s] ( 741/1812) before::meter masked_cmp_hole_envelope
        masked_cmp_hole: peak heap 1156 B exceeds the pinned envelope 480 B (input 755 B)
        error: recipe `coverage-kernel` failed on line 1034 with exit code 100

Resolution: add `coverage-kernel coverage-kernel-branch` to `all` (they
are deterministic-verdict legs and `all` is already the slow sweep), or
add a `ci-full` recipe that is `ci` plus the instruments and coverage job
legs; then re-state justfile:12-14 so `ci` names only the ci job and `all`
names what it actually covers. The `before` meter failure is a separate
question for its owner (see open questions). Acceptance: at a commit where
CI's coverage job fails, `just all` fails on the same leg.

### verification-infra-3: The rumors mutation campaign is a hand run with no recipe, no confirming re-run, and no kill-state record since 02560b1f
- Where: .cargo/mutants.toml:26-29 (related: justfile:314-326, tools/mutantcheck:21-23, tools/mutantcheck-expected.json:1-61, .agent-notes/2026-08-21-cbor-wire-mutants-scopeA/README.md:40-41 and 202, justfile:1-2)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read the roster header, the checker header, every `cargo mutants` line in the justfile, the scope-A campaign note; mapped every roster entry's file to its crate: `accumulator.rs` is suanpan, `grow.rs`/`integral.rs`/`prescan.rs`/`watermark.rs` are before; zero entries name a rumors file)
- Verification: reframed: the sweep said rumors has "no committed campaign outcome". A campaign did run (scope A, 3141 mutants, 126 survivors, cargo-mutants 27.1.0, ox-east-1, ~6 hours) and its survivors were disposed on the w2/mutant-coverage branch and recorded (4c70e958, 1a7afe1e). What remains true: no recipe names the campaign, the note's own first next step (the confirming re-run at the branch tip) has no recorded outcome, scope B never ran, and the tree under test was 02560b1f — before the V1 retirement and the codec rewrite. History: already-known (the note lists the re-run as outstanding).
- Owner-gated: yes: a campaign is hours of compute; its cadence and where its record lives are owner decisions

The roster header names `cargo mutants --workspace` as the campaign
configuration of record, but the justfile runs only `mutants-list` (which
its own comment labels list-only, never a campaign), and `tools/mutantcheck`
states in its header that no gate leg runs cargo-mutants at all. The
justfile's opening totality claim ("Every artifact in the workspace has a
recipe here") does not cover the campaign. The suite's kill power over the
streaming codec, rewritten since the only campaign, is unmeasured.

Evidence:

    .cargo/mutants.toml
        26	# Campaign configuration of record — the keys below make the plain
        27	# invocation (`cargo mutants --workspace`) run it: nextest, all
        28	# features, the whole workspace's suites, under cargo's default
        29	# dev/test profile. That is what the gate's test leg (`just test-all`)

    tools/mutantcheck
        21	silently swallows every NEW mutant the function later grows — both rot
        22	invisibly because no gate leg runs cargo-mutants at all. This checker
        23	closes that seam from two captured `--list` runs (never a campaign):

    justfile
       314	# Hold the mutants exclusion roster to its pinned counts (list-only, never a campaign).

    .agent-notes/2026-08-21-cbor-wire-mutants-scopeA/README.md
        40	verified present by name at 62447263. What is *not* verified here: that the
        41	landed tests actually kill their mutants under mutation — that needs the
       202	1. **Confirming re-run at the branch tip.** Every "killed by landed test"

Resolution: add a `mutants` recipe that runs the configuration of record
for one package (`cargo mutants -p rumors` with the roster applied) as a
named hand-run outside `gate`/`ci`/`all`, with its output location and the
tool-version pin stated in the recipe comment; run it once at HEAD to
discharge the outstanding confirming re-run; triage every survivor into a
test, a refactor, or a roster entry with rationale (never a committed
MISSED list); amend the justfile's totality claim to name the campaign as
the hand-run it is. Acceptance: a recipe exists, and a dated campaign
record at a named commit shows zero untriaged survivors in rumors.

### verification-infra-4: The coverage pin's scope is before's skyline kernel only; rumors is instrumented on every CI run and never judged
- Where: tools/covcheck-expected.json:2 (related: tools/covcheck:209-214, justfile:1006-1009, justfile:1034)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read the expectation file's `scope`, the checker's single-scope `run`, and the coverage recipes)
- Verification: confirmed; history: no-rationale-found (no `.agent-notes/` entry or justfile comment rules rumors out of scope)
- Owner-gated: yes: curating a rumors scope is owner-directed work

`tools/covcheck` reads one `scope` string and rejects any expected entry
outside it; the committed scope is `crates/before/src/version/skyline/`.
The coverage legs run `cargo llvm-cov nextest --workspace --all-features`,
so every rumors kernel — the streaming codec's decode arms, the proxy state
machine, the materialized work queues, the bookmark format — is
instrumented on every CI run and the report is discarded. None of rumors'
`unreachable!`/`expect` arms is held to a coverage residue, so an arm
reachable from wire input but unexercised reads the same as a dead one.

Evidence:

    tools/covcheck-expected.json
         2	 "scope": "crates/before/src/version/skyline/",

    tools/covcheck
       209	    scope = expected["scope"]
       210	    files = parse_lcov(lcov_text, scope)
       211	    entries = expected["branch" if branch else "line"]
       212	    for relpath in entries:
       213	        if not relpath.startswith(scope):
       214	            return [f"{relpath}: expected entry outside the pinned scope {scope!r}"]

    justfile
      1006	# GOAL: no skyline-kernel arm goes silently unexercised — every uncovered
      1034	    {{ justfile_directory() }}/tools/memwatch cargo llvm-cov nextest --workspace --all-features --lcov --output-path target/llvm-cov/workspace.lcov

Resolution: let `covcheck` accept a list of scopes (or a second expectation
file) and curate a rumors scope, starting with
`src/tree/mirror/streaming/remote/` and `src/bookmark/format.rs`; the CI
wall-time cost is zero because the instrumented run already covers the
workspace. If the owner rules rumors out of scope for now, say so in the
justfile's coverage section so the omission is recorded as a decision.
Acceptance: `covcheck-expected.json` carries a rumors scope whose entries
resolve, and a new uncovered line in that scope fails the coverage job by
name.

### verification-infra-5: No fuzz target exists for any rumors decoder; the spec of record is still unimplemented
- Where: design/rumors-frame-fuzz.md:3 (related: design/rumors-frame-fuzz.md:273-288, justfile:365-368, justfile:549-559, .agent-notes/2026-07-22-before-adversarial-resource-amplification/before-adversarial-resource-amplification.md:1562-1564)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read the design doc's status line and integration section; `ls fuzz` fails; the justfile's `fuzz-build` and `fuzz` recipes name only `crates/before/fuzz` and its five targets; the doc was last touched today, 3327a92b, so it is maintained as a spec)
- Verification: confirmed; history: already-known (the 2026-07-22 adversarial-resource note records the deferral: "Frame-level fuzzing of `rumors` stays deferred to a future campaign; its spec ... is the record that campaign resumes from")
- Owner-gated: yes: the doc's five open questions await rulings, and the work is a campaign

There is no `fuzz/` workspace beside `src/`, and the justfile's fuzz
recipes build and run only before's targets. The crate's malformed-wire
coverage is hand-crafted point suites one frame at a time; coverage-guided
search over composed bytes at the public session entry is the missing
complement, and the model framing (conformance bug detector, not a
security boundary) is already written down in the doc. Two decoders are
not reachable from the session entry the doc names and would need their
own targets: the bookmark record format (`src/bookmark/format.rs`, reached
through a `BookmarkIo` load) and the routed link header
(`src/link/routed/header.rs`, reached through the routed acceptor).

Evidence:

    design/rumors-frame-fuzz.md
         3	Status: spec of record, not yet implemented (2026-07-27).
       273	- **Location:** a detached fuzz workspace at `fuzz/` beside `src/`,
       274	  mirroring `crates/before/fuzz`: empty `[workspace]` table so the
       275	  stable-toolchain gate never builds it, `cargo-fuzz` package metadata,

    justfile
       365	[working-directory("crates/before/fuzz")]
       366	fuzz-build:
       367	    cargo fmt --check
       368	    {{ justfile_directory() }}/tools/memwatch cargo +{{ nightly_toolchain }} fuzz build --target {{ host_triple }}

    $ ls fuzz
    ls: fuzz: No such file or directory

Resolution: rule on the doc's section 7 questions, then implement the
session-level target per section 6 (detached `fuzz/` workspace,
`fuzz_session`, seeds derived from `LinkCapture` with a byte-identity gate
test, fuel and heap caps measured then pinned), wire it into `fuzz-build`
(gate) and `fuzz` (`all`), and add a target pair for the bookmark record
decoder and the routed header decoder. Acceptance: `just fuzz-build`
compiles a rumors target and `just fuzz` runs it for `fuzz_smoke_secs`.

### verification-infra-7: `ci` omits manifestlint, which the gate runs
- Where: justfile:1000 (related: justfile:403, justfile:206-209, .github/workflows/ci.yml:107-108)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (compared the two recipe lines; `git log -L` on both; manifestlint landed 2026-08-31 in 84044759, the `ci` line was last edited 2026-08-20 in de9e0bdf, and 84044759's message reports only "just gate clean")
- Verification: confirmed; history: no-rationale-found (an omission at landing, not a decision)
- Owner-gated: no

A pull request that adds a member-local version, path, or git source to a
manifest passes CI; the check exists only on the committer's machine. The
justfile header describes `ci` as adding what the gate does not reach, not
as dropping a gate lint.

Evidence:

    justfile
       403	gate-lints: fmt-check doclint testdoc workflowlint manifestlint digestshare mutants-list readme-check
      1000	ci: fmt-check doclint testdoc workflowlint digestshare readme-check fuelscape-claims mutants-list clippy clippy-default features wasm-check docs docs-internal test-all citecheck doctest bench-build fuzz-build fuelscape-verify viz

Resolution: add `manifestlint` to the `ci` recipe line; it is build-free
and python3 is already a CI prerequisite. Acceptance: `just ci` fails on a
scratch manifest edit that restates a workspace version.

### verification-infra-8: CI installs cargo-mutants unpinned while the count pin fails on any tool version but 27.1.0
- Where: .github/workflows/ci.yml:76-86 (related: tools/mutantcheck-expected.json:2, tools/mutantcheck:75-79, justfile:308-311)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read ci.yml, the expected file, and the checker's provenance paragraph; git history: the install line landed in e4d92ae4 and the tool-version pin in bec1ceaf afterwards, and the install comment was not revisited)
- Verification: confirmed; history: deliberate-but-expired (the comment's rationale, that only cargo-rdme produces a committed artifact, was true when written and is false since bec1ceaf)
- Owner-gated: no

The install step's comment says cargo-rdme is the only tool whose output is
a committed artifact and therefore the only one pinned. `tools/mutantcheck`
also pins a tool version string and fails on mismatch by design. Because
`cargo-mutants` rides the pinned install-action's manifest, a Dependabot
bump of that action can move cargo-mutants and fail `ci` on an untouched
tree, the exact failure the comment says the cargo-rdme pin avoids.

Evidence:

    .github/workflows/ci.yml
        76	      # cargo-rdme carries a version because it is the only tool here whose
        77	      # output is a committed artifact compared byte for byte: readme-check
        78	      # holds the READMEs against what it emits. Tracking the newest release
        86	          tool: just,cargo-nextest,cargo-rdme@2.1.0,cargo-fuzz,cargo-mutants,wasm-pack

    tools/mutantcheck-expected.json
         2	  "tool": "cargo-mutants 27.1.0",

    tools/mutantcheck
        75	Provenance: the counts derive from the installed cargo-mutants release
        76	(its operator set and name spellings move between releases), so the
        77	expected file pins the `--tool-version` string and a mismatch is a
        78	finding, never a silent re-baseline. Bumping the tool re-pins the counts

Resolution: pin `cargo-mutants@27.1.0` in the install step and rewrite the
comment: two tools carry versions because two committed artifacts depend
on them, and bumping either is a reviewed diff that re-pins
`tools/mutantcheck-expected.json` or the READMEs in the same commit.
Acceptance: `tools/workflowlint` still passes and the install line names
both pins.

### verification-infra-9: testdoc's `.` walk reads untracked trees, including another agent's worktree
- Where: tools/testdoc:19 (related: tools/testdoc:72-78, justfile:184-186, justfile:181, .git/info/exclude:7, tests/seed_liveness.rs:35)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read the tool; `find .claude -name '*.rs' | wc -l` reports 543; `.git/info/exclude` line 7 is the only exclusion of `.claude/worktrees/`; `.gitignore` has none; `tests/seed_liveness.rs` skips `.claude` and `doclint` takes explicit roots). The sweep reports running the tool on that worktree with exit 0; I did not reproduce that.
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The gate's testdoc leg runs `./tools/testdoc .` and the tool skips only
`.git`, `node_modules`, and `target`. The repository root holds
`.claude/worktrees/agent-a0e01f4cff55d1ec9/`, a full worktree with 543
`.rs` files, so the gate's verdict is a function of another tree's
uncommitted state. The sibling sweep `tests/seed_liveness.rs` already
skips `.claude`; `doclint` avoids the problem by taking explicit roots.

Evidence:

    tools/testdoc
        19	IGNORED_DIRECTORIES = {".git", "node_modules", "target"}

    justfile
       181	    ./tools/doclint benches crates examples src tests
       184	testdoc:
       185	    ./tools/testdoc --self-test
       186	    ./tools/testdoc .

    .git/info/exclude
         7	**/.claude/worktrees/

    tests/seed_liveness.rs
        35	const SKIP_DIRS: &[&str] = &["target", ".git", "node_modules", ".claude"];

Resolution: give testdoc the same explicit roots doclint uses
(`./tools/testdoc benches crates examples src tests`), or have it walk
`git ls-files '*.rs'`; add `.claude` to the ignore set as a second guard
and a self-test case for the ignore list. Acceptance: an undocumented test
placed under `.claude/worktrees/` does not change `just testdoc`'s verdict.

### verification-infra-10: testdoc cannot see a `proptest!` block test that lacks an explicit `#[test]`
- Where: tools/testdoc:20-23 (related: tools/testdoc:59-63, tools/testdoc:81-103, src/tree/tests.rs:206-217)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (constructed: a scratch file under my scratchpad with an undocumented `proptest!` fn and an undocumented `#[test]` fn yields exactly one finding, the unit test; a Python scan over src, tests, examples, benches counted 118 depth-1 `proptest!` block fns, 0 without `#[test]`, 0 without `///`)
- Verification: confirmed; history: no-rationale-found (the tool's header states the lexical design; the block form is not among its self-test cases)
- Owner-gated: no

The checker keys on a test attribute line. A `proptest! { fn name(x in
strategy) {...} }` item is a test (the macro emits `#[test]`), but its
source carries no attribute unless the author writes one, so the gate
never checks it. Today the class is latent: every block fn in the tree
carries an explicit `#[test]` and a `///` doc, but nothing enforces that
convention, and the property tests are the ones whose invariant statements
matter most.

Evidence:

    tools/testdoc
        20	TEST_ATTRIBUTE = re.compile(
        21	    r"^\s*#\[\s*(?:test|tokio::test|async_std::test|rstest|test_case)"
        22	    r"\s*(?:\([^]]*\))?\s*\]\s*$"
        23	)

    scratch run (file: <scratchpad>/final-sweep-verification-infra/testdoc-demo/demo.rs)
        .../demo.rs:9: test `undocumented_unit` is missing a `///` doc comment
        exit=1
    (the `proptest!` fn at line 4 of the scratch file was not reported)

Resolution: track `proptest! {` blocks lexically and treat each depth-1
`fn` inside as a test entry point; add both the block form and the
omitted-attribute case to `--self-test`. Acceptance: the scratch file
above yields two findings.

### remote-proxy-tests-27: `tools/testdoc` does not recognize `#[pollster::test]`, so the gate's doc requirement is unenforced for 12 tests here and 29 crate-wide
- Where: tools/testdoc:20-23 (related: tools/testdoc:81-107; justfile:183-186; AGENTS.md "Writing tests"; start/tests.rs:73; tests.rs:317, 329)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (I ran `python3 tools/testdoc` on a scratch probe holding an undocumented `#[pollster::test]`, an undocumented `#[test]`, and an undocumented `#[tokio::test]`: only the plain and tokio cases were reported, exit 1; `--self-test` passes and has no pollster case; `grep -rc '#\[pollster::test\]' src tests`: proxy/tests.rs 2, start/tests.rs 10, codec/tests.rs 1, tests/handshake.rs 7, tests/gossip_when.rs 2, tests/changes.rs 7)
- Seen by: structure-prose; refutation: confirmed by execution; history: deliberate-but-expired (the roster was complete when written at 358c6b1a on 2026-07-15; the first `#[pollster::test]` entered at 83edcd94 the next day and the roster was never revisited)
- Owner-gated: no

The lexical checker's attribute regex names `test`, `tokio::test`, `async_std::test`, `rstest`, and `test_case`. All 29 pollster tests are documented today, but an undocumented one passes `just testdoc`, and AGENTS.md's statement that "The gate's `testdoc` checks that the comment exists" is false for a whole attribute family in use. Principle 6: the cheapest passing artifact must be the intended one.

Evidence:

        20	TEST_ATTRIBUTE = re.compile(
        21	    r"^\s*#\[\s*(?:test|tokio::test|async_std::test|rstest|test_case)"
        22	    r"\s*(?:\([^]]*\))?\s*\]\s*$"
        23	)

Resolution: Match any path ending in `test` (for example `(?:[A-Za-z_][A-Za-z0-9_]*::)*test|rstest|test_case`) and add a `#[pollster::test]\nasync fn ...` case to `self_test()` so the form is pinned. Acceptance: a probe file containing an undocumented `#[pollster::test]` is reported by `./tools/testdoc <probe>`; `./tools/testdoc --self-test` passes with the new case; `just testdoc` remains clean on the tree.

Cross-reference: tests-disruption-handshake-33 and tests-observation-37 report the same hole from their partitions (each confirmed it by running the tool); verification-infra-10 (the `proptest!` block form) and verification-infra-9 (untracked trees) are the tool's other two holes. tests-observation-8 removes seven of the 29 sites by moving tests/changes.rs off pollster.

### tests-disruption-handshake-33: The gate's testdoc never sees `#[pollster::test]`: nine tests in this partition and 29 crate-wide are unchecked
- Where: tools/testdoc:20-23 (related: tools/testdoc:85-100; justfile:184-186; tests/handshake.rs:65, 83, 114, 152, 182, 217, 250; tests/gossip_when.rs:476, 680; tests/changes.rs)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (ran `python3 tools/testdoc` on a scratch probe under the final/ scratch directory containing undocumented `#[pollster::test]`, `#[test]`, and `#[tokio::test(flavor = "current_thread")]` functions: exit 1 naming only `undocumented_plain` and `undocumented_tokio`; the pollster case passed silently. `grep -rn 'pollster::test' tests src | wc -l` is 29: handshake.rs 7, gossip_when.rs 2, changes.rs 7, src 13)
- Seen by: api-economics [37]; refutation: confirmed (independently reproduced); history: deliberate but expired (the alternation was complete at 358c6b1a; `pollster::test` arrived two days later and the regex was never widened; the self-test pins only the tokio form)
- Owner-gated: no

TEST_ATTRIBUTE enumerates `test|tokio::test|async_std::test|rstest|test_case` anchored at `#[`, so `#[pollster::test]` never matches and those tests' doc comments are not held by the gate (justfile:184-186 runs `./tools/testdoc .`). All 29 currently carry docs, but nothing enforces it, and the self-test has no pollster case. AGENTS.md: "The gate's testdoc checks that the comment exists"; a check that exempts one attribute form is a board nothing enforces for that form.

Evidence:

        20	TEST_ATTRIBUTE = re.compile(
        21	    r"^\s*#\[\s*(?:test|tokio::test|async_std::test|rstest|test_case)"
        22	    r"\s*(?:\([^]]*\))?\s*\]\s*$"
        23	)

Resolution: Widen the alternation to accept any path ending in `test`, e.g. `(?:[A-Za-z_][A-Za-z0-9_]*::)*test` alongside `rstest|test_case`, and add a self-test case with an undocumented `#[pollster::test] async fn x() {}` expected to be reported. Acceptance: `./tools/testdoc --self-test` passes with the new case, and the tool exits 1 naming an undocumented `#[pollster::test]` function.

Cross-reference: remote-proxy-tests-27 and tests-observation-37.

### tests-observation-37: The gate's `testdoc` regex does not recognize `#[pollster::test]`, leaving seven changes.rs tests (and two other suites) outside doc-comment enforcement
- Where: tools/testdoc:20-23 (related: tests/changes.rs:18, 29, 58, 73, 103, 142, 158, tests/handshake.rs, tests/gossip_when.rs, justfile:184-186)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (ran the regex in python3: `#[pollster::test]` does not match while `#[test]` and `#[tokio::test(flavor = "current_thread")]` do; ran `./tools/testdoc` on scratch copies: tests/changes.rs with the doc above line 18 removed exits 0, tests/listen.rs with the doc above line 76 removed exits 1 naming `genesis_replay_observes_the_live_set_once`; `grep -c pollster::test` gives changes.rs 7, handshake.rs 7, gossip_when.rs 2)
- Seen by: api-economics [40]; refutation: confirmed by the same construction; history: no rationale found (the tool predates pollster's appearance in tests/ by one day; the hole also covers the src/ pollster suites since the recipe scans the whole tree)
- Owner-gated: no

`tools/testdoc` recognizes `test`, `tokio::test`, `async_std::test`, `rstest`, and `test_case`, but not `pollster::test`. AGENTS.md says the gate's `testdoc` checks that every test's doc comment exists; a check that skips one attribute in use is a gate with a hole, and the tests happen to be documented today, which is exactly the condition under which the hole is invisible.

Evidence:

    20	TEST_ATTRIBUTE = re.compile(
    21	    r"^\s*#\[\s*(?:test|tokio::test|async_std::test|rstest|test_case)"
    22	    r"\s*(?:\([^]]*\))?\s*\]\s*$"
    23	)

Resolution: Add `pollster::test` to the alternation (or match any `<path>::test` form) and add a `#[pollster::test]` case to `--self-test`. Independently, tests-observation-8 removes pollster from changes.rs. Acceptance: `./tools/testdoc --self-test` covers `#[pollster::test]`; stripping the doc above tests/changes.rs:18 fails `just testdoc`.
Construction: Copy tests/changes.rs to a scratch path, delete lines 16-17, run `./tools/testdoc <copy>`: exit 0 today.

Cross-reference: remote-proxy-tests-27 and tests-disruption-handshake-33.

## Crate root and public surface (lib, peer, rumors, batch, snapshot, tutorial)

The public surface's own tests are thin by design (the behavioral suites live under `tests/`); the gaps here are an unexercised concurrency guard in `try_into_peer`, one `Snapshot` accessor with no public-tier call, a bootstrap plumbing suite whose quantifiers outgrew its assertions, and headline figures in the crate doc with no committed derivation.

### api-core-25: `try_into_peer`'s exactly-once claim among concurrent reuniters is untested
- Where: src/rumors.rs:116-130 (related: src/rumors.rs:45-48, src/rumors.rs:54-60, src/rumors.rs:110-115, tests/disruption.rs:830-837, tests/listen.rs:309-314, tests/api_send_bounds.rs:73-79, tests/lifecycle.rs (an existing `noop_waker` harness))
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (grep of every `try_into_peer` call across tests/ and src/: each is a sole-handle reunite or a sequential per-set reunite awaiting clones dropped by spawned tasks; `grep -rn 'join!(.*try_into_peer'` is empty; no harness polls two reuniters on clones of one set; the constructed test was not run)
- Seen by: correctness; refutation: confirmed; history: no rationale found (the `claimed` CAS and the subscribe-before-shed ordering arrived in cb69fc951 with no concurrent-reuniter test; neither prior review packet mentions `Extant`)
- Owner-gated: no

The `claimed` CAS exists only for two or more reuniters observing quiescence concurrently, and the subscribe-before-shed ordering only for a reuniter that parks and is woken by a later drop. No committed test builds either shape, so deleting `claimed` or moving `drops.subscribe()` below `drop(extant)` would fail nothing deterministically. Each `Rumors` clone carries its own `Peer` value (rumors.rs:62-77), so without the CAS two reuniters would each receive a `Peer` for one identity, the linearity violation the `Peer`/`Rumors` XOR (lib.rs:113-126) exists to exclude. The guard doctrine asks that a guard name a concrete failure the committed tests catch; here they do not.

Evidence:

   116	        loop {
   117	            // Monotone once zero: creating a token takes a live `Rumors` to
   118	            // clone, and every reuniter has already shed its own.
   119	            if token.strong_count() == 0 {
   120	                // Exactly one reuniter wins the claim; the Peer/Rumors
   121	                // XOR is restored the instant this swap succeeds.
   122	                return claimed
   123	                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
   124	                    .is_ok()
   125	                    .then_some(peer);
   126	            }

Resolution: Add a point suite (tests/reunite.rs) driven with a noop waker, as tests/lifecycle.rs already does: (1) three clones, two reuniters polled to `Pending`, drop the third clone, poll both to completion, assert exactly one `Some`; (2) one reuniter parked, then the last clone dropped, assert it resolves `Some`; (3) two parked, drop one reuniter future, drop the last clone, assert the survivor gets `Some`. Then a proptest over N clones and K >= 1 reuniters with a shuffled drop/poll order asserting exactly one `Some` and every other reuniter `None`. Acceptance: the new tests are committed and fail when `claimed` is removed (two `Some`) and when `drops.subscribe()` is moved below `drop(extant)` (a parked reuniter stays `Pending` after the last drop).
Construction: `let a = Peer::<u64>::seed().into_rumors(); let b = a.clone(); let c = a.clone(); let mut f1 = pin!(a.try_into_peer()); let mut f2 = pin!(b.try_into_peer());` poll both with `futures::task::noop_waker_ref()` and assert `Pending`; `drop(c)`; poll both to `Ready`; assert `f1_result.is_some() ^ f2_result.is_some()`.

### verification-infra-6: The crate doc's operating-envelope figures have no committed derivation or enforced measurement
- Where: src/lib.rs:31-51 (related: README.md:35-57, tests/dispute_wire.rs:1-31, results/fit_gossip_grid.py:5, results/mirror-complexity.tex:3-4 and 54)
- Class / severity / confidence: verification-gap / medium / medium
- Provenance: verified (read src/lib.rs:20-60; grepped `Gb/s`, `Mb/s`, `messages/s` across src, tests, benches, examples, design, results, .agent-notes: hits only in src/lib.rs and its derived README; read tests/dispute_wire.rs's module doc; read the results/ headers and grepped them for BLAKE3 and the retired file names)
- Verification: confirmed for the in-scope claim; the results/ observations are outside this review's stated scope and are moved to the open questions. History: no-rationale-found in scope (commit 4bbd6c5b3's ruling that a dated design record keeps its original vocabulary was made for `design/streaming-latency-serialization.md`, not for `results/`)
- Owner-gated: yes: the doc's wording is the owner's

The crate doc states numeric bounds (10,000 messages/s, 1 Gb/s, 100
messages/s at 10 Mb/s, 80 Mb/s per KB of body, staleness roughly the
square of the bandwidth shortfall) and labels one of them "derived". No
committed test or constant computes any of them. The closest instrument is
`tests/dispute_wire.rs`, which pins the exact per-message wire law at
three cells and is not tied to the doc's figures. The only derivation
artifact is `results/`, which is dated, BLAKE3-denominated, and cites two
removed files (`results/ANALYSIS.md`, removed in b2ea50bb9 as an
accidentally committed LLM output; `results/mirror-complexity.md`, retired
in b29c4e0e1).

Evidence:

    src/lib.rs
        31	//! - peers produce in total **less than 10,000 messages/second**, and
        32	//! - each peer-to-peer link offers **1 Gb/s or better**.
        48	//! in proportion to roughly the square of the bandwidth shortfall (derived: a
        49	//! session's metadata amortizes, falling roughly as the inverse square root
        50	//! of the backlog at realistic scales, so equilibrium repays a bandwidth
        51	//! deficit with its square in staleness).

    tests/dispute_wire.rs
         3	//! The closed form in `Peer::sync_memory_budget`'s docs takes a
         4	//! session's mean encoded record size through the per-message wire law
         5	//! pinned here, and the design-record anchor (`DISPUTE_WIRE_BYTES`) is

    results/fit_gossip_grid.py
         5	the two-regime model documented in `results/ANALYSIS.md`. Prints the fitted

Resolution: derive the doc's figures in a committed test from the pinned
per-message wire law (the `tests/dispute_wire.rs` constants) plus stated
assumptions (fan-out, rounds per second), and have the doc cite the
constant and its validity band; the owner decides the wording. The
disposition of `results/` (re-denominate against SHA3-256 with its
references restored, move its derivation into the test, or excise) is an
owner call recorded under open questions. Acceptance: a test names each
figure in src/lib.rs and fails when the wire law moves it out of band.

Cross-reference: the api-core partition raised the same figures as an open question (src/lib.rs:45-54's "derived" scaling claims with no committed instrument).

### api-core-20: Bootstrap plumbing testdocs claim every knob; the bodies check two of four
- Where: src/peer/bootstrap/tests.rs:6-8 (related: src/peer/bootstrap/tests.rs:44-52, src/peer/bootstrap/tests.rs:54-66, src/peer/bootstrap/tests.rs:77-91, tests/payload_depth.rs:146-148, tests/observe.rs:278-281, src/message.rs:248, src/observe.rs:213-216)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read the whole file: every assertion is on `window` or `run_budget`; `payload_depth_limit` and `observe` appear in no assertion; `PayloadCodec::limit()` exists at message.rs:248; `Attachment` derives only `Clone, Default`, no `PartialEq`)
- Seen by: prose, correctness; refutation: confirmed; history: deliberate-but-expired (accurate at f11949147 for three knobs, all asserted; 40b1e96a and 4356e197 added knobs without touching this file; 368da2a5 removed the protocol assertions and kept the quantifier)
- Owner-gated: no

The module doc says "every builder knob reaches the builder's state, and every stored choice reaches the joined peer unchanged"; `knobs_store_the_selected_values` says "Each knob stores exactly the selected value"; `joined_peer_retains_the_configuration` says "The joined peer retains every builder choice". Each body asserts only `window` and `run_budget`, so a regression in `payload_depth_limit` or `observe` plumbing passes a suite whose doc says it is covered. `defaults_match_the_seed_configuration` claims the builder matches "the same budget and run target [`Peer::seed`] starts with" but compares against constants, not against a `Peer::seed()`, so a seed that drifted from the constants would not fail it. AGENTS.md: an inaccurate testdoc is a bug in the test.

Evidence:

    6	//! the session bytes in `tests/bootstrap_snapshot.rs`. This suite pins the plumbing those tests
    7	//! rest on: every builder knob reaches the builder's state, and every
    8	//! stored choice reaches the joined peer unchanged.

    54	/// Each knob stores exactly the selected value, through the same
    55	/// constructors as the matching [`Peer`] methods.

    65	    assert_eq!(budget_bytes(config.window), CUSTOM_BUDGET);
    66	    assert_eq!(config.run_budget, RunBudget::from_bytes(CUSTOM_TARGET));

    77	/// The joined peer retains every builder choice for its later sessions.

Resolution: Extend the bodies: assert `config.payload_depth_limit` and `peer.codec.limit()` against a non-default `PayloadDepthLimit`, and assert the attachment reaches the joined peer (a `pub(crate)` `Attachment::is_attached()` or the `Debug` form, since `Attachment` has no `PartialEq`); in `defaults_match_the_seed_configuration`, read the expected values off `Peer::<u64>::seed()`'s `window`, `run_budget`, and `codec`. Alternatively narrow every quantifier to the two sizing knobs and point at tests/payload_depth.rs and tests/observe.rs for the rest. Acceptance: each testdoc's quantifier matches the fields its body asserts; the defaults test reads its expected values off a seeded `Peer`.

### Nits

The full record of each (provenance, history, evidence) is in the evidence file named in the last column.

| Id | Where | Claim | Resolution | Record |
|---|---|---|---|---|
| api-core-36 | `src/snapshot.rs:54-56` | `Snapshot::earliest` has no public-tier test; its two-clause contract is pinned only at the tree layer | assert `earliest().is_none() == is_empty()` and `earliest <= v` over `iter()` in single_peer's batch proptest | `evidence/partitions/api-core.md` |

## Session and bookmark (peer/gossip, bookmark, observe, message)

The partition's suites are strong (the bookmark format suite is the standard the finalizer named for the rest of the crate); the residue is three testdocs left behind by the two-byte epilogue marker and two narrow strategies.

### session-bookmark-18: Epilogue marker tests document a one-byte marker and never vary the first byte
- Where: src/peer/gossip/tests.rs:79-89 (related: src/peer/gossip/tests.rs:6-8, src/peer/gossip.rs:45-54, src/peer/gossip.rs:1297-1307, src/tree/mirror/cbor.rs:73)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (`git show 4dd2053c -- src/peer/gossip/tests.rs` changed only the comparison lines, `[byte]` to `[EPILOGUE_MARKER[0], byte]` and `== EPILOGUE_MARKER` to `== EPILOGUE_MARKER[1]`; `git show 4dd2053c^:src/peer/gossip.rs` has `const EPILOGUE_MARKER: u8 = b'.';`; `SELF_DESCRIBED_HEAD` is `[0xd9, 0xd9, 0xf7]` at cbor.rs:73)
- Seen by: prose, correctness; refutation: confirmed, severity lowered from medium (the comparison at gossip.rs:1299 is a whole-array `!=`, so the untested first byte cannot currently diverge); history: deliberate but expired (docs written for the `u8` marker; the CBOR wire change widened the constant and edited only the two comparison lines)
- Owner-gated: no

The module doc and the exhaustiveness test's doc describe the marker as a single byte and claim the desynchronized-preamble case is covered, but `EPILOGUE_MARKER` is `[u8; 2]` and the body of `marker_byte_space_is_exhaustive` holds the first byte fixed at `EPILOGUE_MARKER[0]` and sweeps only the second. The one input the marker's own doc (gossip.rs:50-53) is designed around, a preamble's self-described tag `0xd9` arriving where the marker belongs, is constructed by no test in the crate. An inaccurate testdoc is a bug in the test (AGENTS.md), and a regression that compared only `marker[1]` would pass this suite.

Evidence:

    6	//! - The V2 session epilogue marker: the last wire ingress of every V2
    7	//!   session, one byte read from the control stream after all session
    8	//!   work. The suite exhausts that byte space and its truncation directly

    79	/// Marker decoding is exhaustive: exactly the one marker byte is accepted
    80	/// and every other byte is a typed protocol violation.

    88	    for byte in u8::MIN..=u8::MAX {
    89	        let bytes = [EPILOGUE_MARKER[0], byte];

Resolution: Re-state the module doc (6-8) and the testdoc (79-85) for the two-byte CBOR text item `"."`. Sweep both positions: either the full 65536-pair space (cheap) or the first byte over `u8` with the second fixed plus the existing sweep, asserting `InvalidData` for every non-marker pair. Add a named case feeding `[0xd9, 0xd9]` (the opening of `SELF_DESCRIBED_HEAD`) and asserting `Error::Epilogue` with `InvalidData`, so the preamble-desync claim is pinned rather than asserted. Acceptance: the docs say "two-byte item"; a committed case rejects a preamble opening; a mutant that compares only `marker[1]` fails that case.

### Nits

The full record of each (provenance, history, evidence) is in the evidence file named in the last column.

| Id | Where | Claim | Resolution | Record |
|---|---|---|---|---|
| session-bookmark-31 | `src/bookmark/format/tests.rs:67-77` | the single-byte corruption proptest applies one mask (`^= 0xff`) and never the empty payload | draw `mask in 1u8..=255` and payload `0..64` | `evidence/partitions/session-bookmark.md` |
| session-bookmark-42 | `src/observe/tests.rs:45-56` | testdoc says "whatever the session kind"; the body calls `begin(SessionKind::Gossip)` once | iterate the three kinds or drop the quantifier | `evidence/partitions/session-bookmark.md` |

## Link

The routed adapter's boundary roster misses the inclusive maximum name length; the in-memory link's unit tests restate conformance clauses at lower strength and claim properties their bodies do not observe.

### link-31: The routed-adapter roster never constructs the accepted maximum advertised name
- Where: src/link/routed/tests.rs:694-708 (related: src/link/routed/endpoint.rs:205, src/link/routed/header.rs:304-311)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (grep: the only `MAX_ADDR_LEN` use in tests is `"x".repeat(header::MAX_ADDR_LEN + 1)` at routed/tests.rs:696; no test constructs or parses a 255-byte name)
- Seen by: correctness (30a); refutation: confirmed (30b, `link()` over a recycled connection, judged low value because the router path after `READY` is identical for both header kinds; 30c is link-28's construction)
- Owner-gated: no

`out_of_bound_advertised_name_fails_construction` tries 0 and `MAX_ADDR_LEN + 1`, so the inclusive upper bound is never exercised: a 255-byte name is never accepted at endpoint.rs:205 nor parsed through `vec![0; len]` at header.rs:310 and dialed back. Boundary roster: empty, one, capacity, capacity plus one.

Evidence:

    696	    for name in [String::new(), "x".repeat(header::MAX_ADDR_LEN + 1)] {

Resolution: add a test that constructs an endpoint with a 255-byte `MemoryName`, establishes a link toward it from a peer, and asserts the peer's reverse `transfer` dials the full name (the `LINK` header round-trips the maximum). Acceptance: a new test in src/link/routed/tests.rs with a doc comment stating the boundary; the gate stays clean.
Construction: `MemoryName::new("x".repeat(header::MAX_ADDR_LEN))` as `a`'s advertised name; `establish(&b, <that name>, &mut a_incoming)`; assert `info.peer` equals the full name and `transfer(&at_a, &mut at_b, ..)` succeeds (the reverse dial uses the name carried in the header).

### link-12: Test docs claim properties the bodies do not observe (blocking, independence, allocation)
- Where: src/link/tests.rs:94-95 (related: src/link/tests.rs:113, src/link/tests.rs:21-22, src/link/tests.rs:141-142, src/link/routed/tests.rs:118-119, src/link.rs:349-351)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (read each doc against its body)
- Seen by: prose (19), correctness (32); refutation: confirmed both, merged; history: no rationale for 19; for "never allocates" the intended sense is recoverable (the epoch "allocates no identity", the crate's identity vocabulary; link.rs:351 says "a label tripwire rather than an identity")
- Owner-gated: no

An inaccurate testdoc is a bug in the test. `stream_writes_block_on_their_own_reader` says a writer past capacity "blocks until its own reader drains", but the body only joins a six-byte write against a read at capacity two, which a non-blocking pipe passes identically (the `.expect` at 113 states what is checked). `control_carries_bytes_both_ways` claims "two independent ordered byte pipes" over a strictly sequential ping/pong; independence is `check_control_duplex`'s job. `epochs_count_and_wrap` says the counter "never allocates", which read as a memory claim about a `u8` is unasserted and odd. routed/tests.rs:118-119 says bytes cross "independently" over a sequential exchange. If link-11 lands, only the epochs doc and the routed comment remain.

Evidence:

    94	/// Backpressure is per-stream and receiver-paced: a writer past the buffer
    95	/// capacity blocks until its own reader drains, then completes.

    113	    .expect("a two-byte window still carries six bytes");

    141	/// Epochs advance one per begun session and wrap: the label tripwire never
    142	/// allocates, only counts. `finish` marks each session's clean end.

    (routed/tests.rs)
    118	            // The control stream is the establishment connection:
    119	            // bytes cross in both directions independently.

Resolution: restate each to what the body checks: "a two-byte window still carries six bytes to completion"; "the control halves carry bytes in both directions, intact and in order"; "Epochs advance one per begun session and wrap at `u8::MAX`; the epoch is a mismatch check, never an identity. `finish` marks each session's clean end."; "bytes cross in both directions". If blocking is meant to be tested, add the observation (`now_or_never` on the over-capacity write before draining). Acceptance: each listed doc names only behavior its body asserts.

### Nits

The full record of each (provenance, history, evidence) is in the evidence file named in the last column.

| Id | Where | Claim | Resolution | Record |
|---|---|---|---|---|
| link-11 | `src/link/tests.rs:21-114` | four in-memory link unit tests restate conformance clauses at lower strength | delete the four, or state the localization reason in the module doc | `evidence/partitions/link.md` |

## Conformance

The link suite is the model: every liveness clause has a committed negative control asserted `Stalled`, and every legal adversity asserts it fired. The backend suite is where the substantive gaps sit: a census ceiling that compares two identical runs, the `children` population charged but never priced, an `assemble` check driven once at the root, a `parent` clause the `Backend` docs say is convicted and is not, and a convergence oracle of two-sided agreement. Three of the five backend findings were constructed and run (`witness/results.md`); the notes below record what each run showed. The partition's correctness lens hung during the main run and was rerun after finalization; the entries it added here (conformance-38, conformance-40, conformance-41, conformance-42, and the nit conformance-39) passed the same refutation and history passes but were not constructed, so none is demonstrated.

### conformance-28: The census ceiling has no liveness floor, and for `Local` the budget resolves to the floor window, so the end-to-end check passes vacuously while its testdoc says the budget binds
- Where: src/conformance/backend.rs:663-668 (related: src/conformance/backend/tests.rs:122-132, src/conformance/backend/tests.rs:189-191, src/tree/mirror/streaming/window.rs:175-176, src/tree/mirror/streaming/window.rs:190-192, src/tree/mirror/streaming/window.rs:397-399, src/tree/mirror/streaming/window.rs:436-473, src/tree/mirror/streaming/window.rs:535-542, src/tree/mirror/streaming/backend/local.rs:103-110, src/tree/typed/prefix.rs:17-21, src/link.rs:169)
- Class / severity / confidence: verification-gap / high / high
- Provenance: demonstrated (second witness pass: with `assert!(budget_peak > floor_peak)` inserted in `check`, `local_backend_conforms` failed at once, floor_peak and budget_peak both 14496 and the derived capacities all ones under `Budget(0)` and `Budget(65536)` alike; `materializing_backend_conforms` at 4 MiB admitted 1033 bytes above the floor, 193707 to 194740, so the floor holds there; before the pass, verified by arithmetic from constants read in the tree: `STREAM_COUNT = 17` (src/link.rs:169); `FAN = 256` (window.rs:132); `Local::node_bytes` is `size_of::<typed::Node<Z>>()`, const-asserted pointer-sized at local.rs:110, so 8; `Prefix<Z>` is `repr(transparent)` over tinyvec 1.11.0's `ArrayVec { len: u16, data: [u8; 32] }`, 34 bytes at alignment 2, so `(Prefix<Z>, Node<Z>)` is 48 and `FAN_SLOT_BYTES` is 40; `supply_fans = 17 × 257 × (8 + 40) = 209,712`, matching the value the window tests pin for `SUPPLY_DECODE_ENVELOPE_BYTES` at window/tests.rs:233-244 and the sync-budget note records; `64 * 1024 = 65,536`. The search at window.rs:448-456 starts at `lo = 1` and widens only when `charge(mid) <= budget`, and `charge` starts from `supply_fans`, so for `Budget(65,536)` as for `Budget(0)` every `charge(mid) > budget`, `lo` stays 1, and every capacity at 458-472 floors at one.)
- Seen by: prose, correctness, perfapi (open question); refutation: confirmed, folding the general statement into the constructed instance; history: deliberate but expired (the 64 KiB budget and the "genuinely binds" testdoc landed in 922db57a on 2026-07-22 23:09, before `from_budget` had a flat term; the flat pre-charge landed in b0304e711 on 2026-07-23 13:19 and the budget was never re-sized against it; `check` has had no floor since 922db57a)
- Owner-gated: no

`check` asserts only `admitted <= budget_bytes` over `budget_peak.saturating_sub(floor_peak)`. Under `Local` pricing the flat decode-fan pre-charge alone is 209,712 bytes, three times the 65,536-byte budget `local_backend_conforms` passes, so `WindowConfig::Budget(64 KiB)` resolves to the same all-ones window as `Budget(0)`, the two `run`s are identical, `admitted` is 0, and the ceiling cannot fail. The testdoc's "a tight budget that genuinely binds at this scale" is false, and nothing in `check` would have exposed it. Principle 2: a ceiling over a counter passes vacuously when the counter stops counting, so every ceiling needs a positive floor; and an inaccurate testdoc is a bug in the test. The `Materializing` run at 4 MiB is above its own flat term (demonstrated in the second witness pass: its budgeted peak exceeds the floor run's by 1033 bytes and the capacities differ from height 22 upward), so the floor holds there.

Evidence:

       663	    let admitted = budget_peak.saturating_sub(floor_peak);
       664	    assert!(
       665	        admitted <= budget_bytes,
       666	        "widening the window from the floor admitted {admitted} measured \
       667	         bytes at peak; the stated budget is {budget_bytes}",
       668	    );

    src/conformance/backend/tests.rs:
       124	/// `Local` is the trivial case — handles into a resident tree — so the
       125	/// suite's pointwise check reduces to the pointer-size constant, and the
       126	/// end-to-end census confirms the window's byte admittance under a tight
       127	/// budget that genuinely binds at this scale.
       128	#[test]
       129	fn local_backend_conforms() {
       130	    let _serial = serialized();
       131	    pollster::block_on(check(Local, 64 * 1024));

    src/tree/mirror/streaming/window.rs:
       397	        let supply_fans = (STREAM_COUNT as u128)
       398	            * (FAN as u128 + 1)
       399	            * (node_bytes(0, version_bound) as u128 + FAN_SLOT_BYTES as u128);

Resolution: In `check`, pair the ceiling with a floor: `assert!(budget_peak > floor_peak, "the budgeted window admitted nothing above the floor: the budget does not bind at this corpus scale")`, or resolve both `WindowConfig`s against the corpora and assert the budget window's widest capacity exceeds one. Name the `Local` budget (`LOCAL_BUDGET`, beside `MATERIALIZING_BUDGET`) and size it above the flat term, for example `SUPPLY_DECODE_ENVELOPE_BYTES + 64 * 1024` (both in scope under `cfg(test)`), with the constant's doc stating the term it must clear; restate the testdoc from the measured behavior. Re-check `MATERIALIZING_BUDGET` against its own flat term with the same floor. Acceptance: with the floor in place, the current `Local` budget fails the floor assertion; with the re-sized budget both conformance tests pass and the floor holds; the testdocs describe exactly what the body asserts.
Construction: add `assert!(budget_peak > floor_peak)` to `check` and run `local_backend_conforms`: it fails, demonstrating the two runs were identical. Alternatively instrument `run` to print the derived window's capacities for both configs and observe all ones in both.

Witness: the second witness pass ran the first form of this construction (`witness/results.md`, `## conformance-28`): `assert!(budget_peak > floor_peak, ..)` was inserted in `check` immediately before `let admitted = budget_peak.saturating_sub(floor_peak);`, together with `eprintln!`s of both peaks and of the capacities `Window::from_budget` derives at the corpus sizes under `Budget(0)` and under the test's budget; then `local_backend_conforms` and `materializing_backend_conforms` were run with `cargo nextest run -p rumors --all-features --no-capture`. Decisive output, `local_backend_conforms` (budget 64 KiB):

    WITNESS conformance-28: floor_peak=14496 budget_peak=14496 budget_bytes=65536
    WITNESS conformance-28: floor capacities (height 0..=32) = [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]
    WITNESS conformance-28: capacities identical under Budget(0) and Budget(65536): true
    WITNESS conformance-28: the budgeted run's peak (14496) did not exceed the floor run's peak (14496); the census ceiling below is vacuous
    test conformance::backend::tests::local_backend_conforms ... FAILED

and `materializing_backend_conforms` (budget 4 MiB):

    WITNESS conformance-28: floor_peak=193707 budget_peak=194740 budget_bytes=4194304
    WITNESS conformance-28: budget capacities (height 0..=32) = [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 12, 12, 16, 25, 48, 49, 49, 49, 49, 1, 1]
    WITNESS conformance-28: capacities identical under Budget(0) and Budget(4194304): false
            PASS [   0.547s] (1/1) rumors conformance::backend::tests::materializing_backend_conforms

For `Local` the floor run and the budgeted run have identical peaks, so `admitted` is 0 and the ceiling cannot fail, while the added floor fails at once; the derived window is all ones under both budgets (computed with version-bytes 0, which can only overstate capacities). For `Materializing` at 4 MiB the budgeted run admits 1033 bytes above the floor and the capacities differ from height 22 upward, so the floor would pass there. The edit was restored afterwards.

Cross-reference: the same mechanism was observed by running `tests/window_census.rs` for tests-resource-link-window-20 (capacities sum 33 over 33 heights at a 64 KiB budget, identical peaks for both arms), which corroborates the arithmetic here without a run of this suite.

### conformance-24: `Charged` leaves unchecked the `parent` Some/None clause and the bulk seams' ordering clauses the `Backend` docs say this suite convicts
- Where: src/conformance/backend.rs:325-326 (related: src/conformance/backend.rs:389-427, src/conformance/backend.rs:438-489, src/conformance/backend.rs:717-724, src/tree/mirror/streaming/backend.rs:120-135, src/tree/mirror/streaming/backend.rs:158-167, src/tree/mirror/streaming/backend.rs:179-197, src/conformance/backend/tests.rs:319-338)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: assessed (read the `Backend` trait docs: `parent` at streaming/backend.rs:129-135 says "Given at least one real child, construction **must** yield a parent ... The backend conformance suite ... convicts a violating implementation in the backend's own tests"; `assemble` at 191-197 says "exactly one node per maximal run, in run order ... The backend conformance suite ... convicts a violating override"; `leaves` at 163-167 lists containment and ascending order with encoder-panic enforcement only. Read `Charged::parent`, `leaves`, and `assemble`.)
- Seen by: correctness (two findings); refutation: confirmed (parent), reframed (leaves' doc claims only encoder enforcement; assemble's run-order clause is the one claimed and unchecked); history: no rationale found (b3b72d248, a docs-only commit, wrote the conviction sentences; the suite's checks are those 5117498d landed, none of which covers these clauses)
- Owner-gated: no

Two `Backend` docs state that this suite convicts violations it does not check. `Charged::parent` computes `fan` and maps over the result: a `None` at `fan > 0` (or a `Some` at `fan == 0`) records no violation; in-session such a backend at best breaks convergence and trips `run`'s `converged` assertion with a different message, never a by-name conviction, and no knob demonstrates either direction. `Charged::assemble` looks runs up by prefix in a `BTreeMap`, which detects a wrong prefix but not a wrong order, so "in run order" is unchecked. `Charged::leaves` checks price, aggregate membership, and count, but not containment or ascending order (the trait doc claims only encoder-panic enforcement for those, so that pair is a plain gap, not a false claim). Statement faithfulness: prose that says a check exists must be true of the code; the production consequence, a decoder panic mid-session, is what a conformance check exists to catch first. This compounds with conformance-30: because the convergence oracle is agreement between two sides running the same backend, a `parent` fault that fires symmetrically evades both.

Evidence:

       325	        let parent = self.inner.parent(prefix, children).await?;
       326	        Ok(parent.map(|node| {

    src/tree/mirror/streaming/backend.rs:
       129	    /// reply to a request — and resolves to `None` the same way. Given at least
       130	    /// one real child, construction **must** yield a parent: under the default
       131	    /// [`assemble`](Self::assemble), a `None` here becomes a missing assembled
       132	    /// node, which the reply decoder treats as a backend contract violation
       133	    /// and enforces by panic. The backend conformance suite (see
       134	    /// [`crate::conformance`]) convicts a violating implementation in the
       135	    /// backend's own tests, before a live session can meet it.

    src/tree/mirror/streaming/backend.rs:
       191	    /// - exactly one node per maximal run, in run order (never merged,
       192	    ///   split, or skipped);
       193	    /// - each node at its own run's height-`H` prefix.
       194	    ///
       195	    /// The backend conformance suite (see [`crate::conformance`]) convicts
       196	    /// a violating override in the backend's own tests, before a live
       197	    /// session can meet it.

Resolution: In `Charged::parent`, after the inner call, record `ledger::violation("parent contract: fan {fan} yielded {Some/None}")` when `parent.is_some() != (fan > 0)`. In `Charged::assemble`, track the previous yielded `Prefix<H>` and record `unordered assembly` when not strictly ascending; in `Charged::leaves`, track the previous `Prefix<Z>` and the requested prefix, recording `unordered leaf walk` and `escaped leaf walk`. Add knobs to `Materializing` (`PARENT_DROPS` returning `Ok(None)` for one interior fan; a swap of the first two yielded items in `leaves` and `assemble`) with `should_panic` controls. Surface pending ledger violations in the message of `run`'s `converged` assertion, so the by-name report is not masked by the convergence panic. Acceptance: each new control fails by name; honest backends pass; both `Backend` conviction sentences are true of the code.
Construction: add a knob making `Materializing::parent` return `Ok(None)` for one interior fan and run `check(Materializing, MATERIALIZING_BUDGET)`: today the panic is "the conformance session must converge both corpora to one root" (or a decoder-side panic), not a ledger violation naming the clause. For order: add a knob that swaps the first two leaves of `Materializing::leaves`'s output; no violation is recorded (count and prices are unchanged).

Demonstration (constructed and run; `witness/results.md`): the primary injection, one interior `parent` returning `Ok(None)`, was convicted deterministically by `check_assembled`'s length invariant at src/conformance/backend.rs:511 ("bulk-assembled len: node answers 1535, its run supplied 1536 leaves"), not by the parent clause. Synthesis note: the finding's kernel holds (`Charged::parent` maps only the `Some` arm, so the `Backend` doc's by-name conviction is false), but its prediction that such a fault "at best" trips `run`'s `converged` assertion did not survive construction, and the ordering sub-claims (swapped leaves in `leaves`, wrong run order in `assemble`) were not exercised. The practical worry, an unconvicted violating backend, does not hold for this injection; the doc inaccuracy and the missing by-name checks do.

### conformance-25: The `children` stream, the walk's in-flight references, is charged but never priced pointwise
- Where: src/conformance/backend.rs:363-377 (related: src/conformance/backend.rs:14-16, src/tree/mirror/streaming/materialized/common.rs:18-36, src/tree/mirror/streaming/window.rs:321-323, src/tree/mirror/streaming/window.rs:372-388, src/conformance/backend/tests.rs:340-356)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: assessed (read `Charged::children`; window.rs:372-388 prices held references per depth at `node_bytes(children_quantile, version_bound)` and says a scope's "held references are the children of one depth-(d−1) parent"; the in-memory session reaches them through `children_of` at materialized/common.rs:28, `backend.clone().children(prefix, node)`)
- Seen by: correctness; refutation: confirmed (an honest `Local`, 8 <= 8, and an honest `Materializing`, header + bounds <= header + 24 * min(len, FAN) + bounds, pass the proposed check); history: no rationale found (the design note named assembled parents only; 8aeed2dd added the leaf check, 5117498d the bulk seams; nothing names `children`)
- Owner-gated: no

`Charged::children` measures each exploded child and puts it on the census, but never compares the measurement to `node_bytes`. The window prices exactly these values per depth, and in the walk-to-walk conformance session the only explode path is `children_of`, so the one population the budget derivation prices per reference is the one the pointwise check skips. An over-holding `children` override (a backend eagerly loading a child's full record) is caught only if the differenced end-to-end peak happens to exceed the budget, which is loose. The module doc's pointwise clause ("every node the session assembles is measured") leaves exploded nodes outside the check, and no negative control exists for them.

Evidence:

       368	        stream! {
       369	            let mut children = pin!(self.inner.children(prefix, parent.into_inner()));
       370	            while let Some(child) = children.next().await {
       371	                yield child.map(|(prefix, node)| {
       372	                    let measured = B::measure(&node);
       373	                    (prefix, ChargedNode::wrap(node, measured))
       374	                });
       375	            }

Resolution: In the `children` stream, price each yielded child at `B::node_bytes(node.len().min(FAN), bound_bytes(&node))` (the fan is invisible here; use the monotone cap `check_assembled` uses and argues) and record `underpriced child` when `measured > priced`. Add a `CHILDREN_SLACK` knob to `Materializing::children` (resize the lazily loaded row, like `WALK_SLACK`) with a `#[should_panic(expected = "underpriced child")]` control at `BULK_OVERHOLD`. Restate the module doc's pointwise bullet to include exploded children. Acceptance: the new control fails by name; `local_backend_conforms` and `materializing_backend_conforms` still pass.
Construction: set a `CHILDREN_SLACK` knob to 64 KiB in `Materializing::children` and run `check(Materializing, MATERIALIZING_BUDGET)`: today no violation is recorded at this seam, and the run passes unless the differenced peak crosses 4 MiB.

Demonstration (`witness/results.md`): the gap holds in refined form. The literal construction (64 KiB of slack on every child row) fails, but at the walk's leaf check with 6144 `underpriced walked leaf` violations, because `Materializing::leaves` is `H::explode` and so re-enters `Charged::leaves`; a backend with a bulk leaf scan would not be caught that way. Slack restricted to interior children (`H::HEIGHT > 0`) passes `check(Materializing, MATERIALIZING_BUDGET)` with no violation at any seam and the differenced peak under 4 MiB.

### conformance-31: The `assemble` seam is exercised only at the root over one run; two of its violation classes have never fired
- Where: src/conformance/backend.rs:770-774 (related: src/conformance/backend.rs:466-469, src/conformance/backend.rs:478-488, src/conformance/backend.rs:775-783, src/tree/mirror/streaming/erased.rs:319-334, src/tree/mirror/streaming/remote/adapter/decode.rs:88, src/tree/mirror/streaming/remote/adapter/decode.rs:408, src/tree/mirror/streaming/materialized/work/assembly.rs:88, src/tree/mirror/streaming/backend/local.rs:201-220, src/conformance/backend/tests.rs:159-164)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: assessed (grep for `.assemble::<|ops::assemble` outside conformance: `erased::ops::assemble` at erased.rs:319-334 is the only route to `Backend::assemble`, and its callers are the wire decoder at decode.rs:88 and 408; the in-memory session's assembly folds through `ops::parent` at work/assembly.rs:88; so in the conformance run the only `Charged::assemble` call is `corpus`'s)
- Seen by: correctness; refutation: confirmed; history: no rationale found (5117498d's six negative controls include neither `unsupplied assembly` nor `unassembled run`; nothing records the sub-root, multi-run regime as out of scope)
- Owner-gated: no

The only call into `Charged::assemble` is `corpus`'s `assemble::<height::Root>` over one sorted run, so the run ledger ever holds one entry. `unsupplied assembly` (a node yielded at a prefix with no run) cannot fire because at `Root` there is one possible prefix; `unassembled run` cannot surface by name because a wholly dropped root trips the `expect` at 778 first. Neither has a knob or a control. Meanwhile the wire decoder runs assembly at scope heights over many runs, the regime the module doc says this seam covers ("the paths the wire codec runs"), and `Local::assemble`'s run boundary (`current != Some(target)` at local.rs:208) is never driven by this suite. A criterion the bad implementation also passes is decoration.

Evidence:

       770	    let mut assembled = pin!(
       771	        charged
       772	            .clone()
       773	            .assemble::<height::Root>(Box::pin(futures_stream::iter(leaves.into_iter().map(Ok))))
       774	    );

Resolution: In `run`, before `reset_peak`, also drive `charged.clone().assemble::<H>(...)` over each corpus's sorted leaves at a sub-root `Convert` height (a one-byte prefix yields up to 256 runs at this corpus scale) and drain it. Add two knobs to `Materializing::assemble`: drop the k-th assembled node (`unassembled run`) and re-tag one node's prefix to a neighbor (`unsupplied assembly`), each with a `should_panic` control. The `bulk-assembled len` and over-hold controls then also run in the multi-run regime. Acceptance: both new controls fail by name; the honest suites pass; the multi-run `Local::assemble` boundary is covered.
Construction: add a knob to `Materializing::assemble` that `skip(1)`s the assembled node stream (not the leaves) and run `check`: today the run panics at "a non-empty corpus assembles a root", not with `unassembled run`.

Demonstration (`witness/results.md`): a `Knob` on `Materializing::assemble` that skips one assembled node makes `check(Materializing, MATERIALIZING_BUDGET)` panic at src/conformance/backend.rs:778 with "a non-empty corpus assembles a root"; `unassembled run` never surfaces by name.

### conformance-20: `check_control`, `check_streams`, and `check_sessions` have no negative control, against the module's own rule
- Where: src/conformance/link/tests.rs:872-877 (related: src/conformance/link.rs:72-80, src/conformance/link.rs:186-190, src/conformance/link.rs:325-331, src/conformance/link.rs:1009-1018)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read the negative-controls section 872-1085 in full: mux and pooled budget for independence, two lossy tests for cancellation, coupled halves for control duplex, capped connector for concurrency; grep for `check_control\b|check_streams\b|check_sessions\b` outside link.rs returns nothing)
- Seen by: correctness, perfapi (open question); refutation: confirmed and extended to `check_sessions`; history: a standing owner ruling from the 2026-07-23 triage says "Every conformance check gets a negative control"; the older checks were never retrofitted
- Owner-gated: no

The section header claims every check proves its teeth; four of seven do. In particular the distinct-payload design of `check_control` exists so that a looped-back control half "fails the byte assertion loudly instead of only hanging" (link.rs:74-76, 188-190), a claim no fixture demonstrates, and `check_streams`'s abort-surfaces-as-EOF and exact-bytes clauses have no known-bad counterpart. Every criterion needs a committed demonstration that a known-bad mechanism fails it.

Evidence:

       872	// ─── Negative controls: every check proves its teeth ────────────────────────
       873	//
       874	// Each violating fixture must FAIL its check, and each legal-adversity
       875	// fixture must pass with its adversity proven fired. A check whose negative
       876	// control stops failing has lost its teeth — these assertions are what make
       877	// a green suite mean anything.

Resolution: Add a looped-back control fixture (rebuild one end with its own `control_write` joined to its `control_read` through `tokio::io::duplex`) and a `#[should_panic(expected = "not this side's own")]` test on `check_control`; a truncating `Tx` wrapper that drops the final byte on shutdown or drop with a `should_panic(expected = "exact bytes")` test on `check_streams`, optionally an `Rx` wrapper that never surfaces EOF asserted `Stalled`. For `check_sessions`, either a fixture that passes every focused probe and fails sessions, or reword the header to name the checks it covers. Acceptance: each new test fails its check as asserted; the header's claim is true of every check in `check`.
Construction: wire `a.control_write` to `a.control_read` via `tokio::io::duplex` and run `check_control`: the a-side `read_exact` returns `CONTROL_PROBE_AB` and the `assert_eq!` against `CONTROL_PROBE_BA` panics.

### conformance-30: `run`'s convergence oracle is agreement between the two sides, not the expected union
- Where: src/conformance/backend.rs:717-724 (related: src/conformance/backend.rs:617-621, src/conformance/backend.rs:636-645)
- Class / severity / confidence: test-quality / low / medium
- Provenance: assessed (read; `COMMON` and `DIVERGENT` at 617-621 make the expected size `COMMON + 2 * DIVERGENT`)
- Seen by: correctness; refutation: confirmed; history: no rationale found (the sync-budget note records root-hash equality as the mechanism, not a reason against a stronger oracle)
- Owner-gated: no

Convergence is witnessed by root-hash equality alone. Two sides running the same backend through the same code that both lose the same subtree still agree; the expected result is known and comparing against it costs one line. The `check` doc lists "the session failed to converge the corpora" as a panic cause, which the current check only partially delivers, and conformance-24's symmetric `parent` fault would pass it.

Evidence:

       717	    let converged = match (&ours.root, &theirs.root) {
       718	        (Some(left_root), Some(right_root)) => left_root.hash() == right_root.hash(),
       719	        _ => false,
       720	    };

Resolution: Also assert `left_root.len() == COMMON + 2 * DIVERGENT`, or build `corpus(common ⊕ left_tail ⊕ right_tail)` once and compare its root hash with both sides'. Acceptance: the assertion holds on the honest runs and fails on a session that drops any leaf on both sides.
Construction: wrap both `Handshaking::start` roots so each side's corpus omits the same shared leaf: today the check passes (equal hashes) while the reconciled set is short one message.

### conformance-35: The ledger test's doc claims drops settle the balance, but the body observes settlement only through an unexplained side effect, and not at all for the second pair
- Where: src/conformance/backend/tests.rs:543-547 (related: src/conformance/backend/tests.rs:561-568, src/conformance/backend/tests.rs:576-587, src/conformance/backend.rs:98-121, src/conformance/backend.rs:191-197)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (the `ledger` module at backend.rs:88-130 exposes no `LIVE` accessor; `PEAK` is monotone between resets and `reset_peak` copies `LIVE` into it; every assertion in the test reads `ledger::peak()`)
- Seen by: structure, prose; refutation: reframed (a no-op `Drop` would fail this test at 576-580 with `before + 216` and the message "two live handles charge twice", so the first pair's settlement is checked indirectly and mis-attributed; the second pair's is never checked); history: no rationale found (8aeed2dd reworded the testdoc to claim settlement without adding a live read)
- Owner-gated: no

A `Drop` that fails to discharge is the census's most consequential bug (every later peak inflates), and this is the test named for catching it; it does so for the first pair only because a failure would inflate the second pair's peak, which nothing in the test explains, and the final assertion at 583-587 pins peak persistence, a different property. The testdoc states an invariant the body should check directly; `reset_peak` seeds `PEAK` from `LIVE`, so a reset-then-read is the live read the test needs.

Evidence:

       543	/// The decorator's ledger accounting is exact over wrap, clone, and drop.
       544	///
       545	/// A wrapped leaf charges its measured post-custody bytes, a cloned
       546	/// handle charges its bytes again, and drops settle to the starting
       547	/// balance — the arithmetic the end-to-end census rests on.

       583	    assert_eq!(
       584	        ledger::peak(),
       585	        before + 200,
       586	        "the peak persists after handles settle",
       587	    );

Resolution: After each drop pair, `ledger::reset_peak(); assert_eq!(ledger::peak(), before, "drops settle the ledger to the starting balance");` (or add a `ledger::live()` accessor and assert it). Keep the persistence assertion only if peak persistence is meant to be pinned; then say so in the doc. Acceptance: deleting the `ledger::discharge` call in `impl Drop for ChargedNode` fails this test at a direct settle assertion; the doc's three clauses each map to an assertion.
Construction: delete `ledger::discharge(self.bytes)` in `ChargedNode::drop` (backend.rs:191-197): today the test fails only at 576-580, with a message about charging twice, not settling.

### conformance-38: The lossy-acceptor control's verdict is window-dependent: below the probe's length the sender panics with `contract: stream write`, blaming the connector
- Where: src/conformance/link.rs:877-882 (related: src/conformance/link.rs:805-815, src/conformance/link.rs:916-923, src/conformance/link/tests.rs:168-174, src/conformance/link/tests.rs:922-938, src/conformance/link/tests.rs:33-39, src/link.rs:532, src/link.rs:598-605, src/testing.rs:375-394)
- Class / severity / confidence: test-quality / low / medium
- Provenance: assessed (the poll sequence traced under `run_to_quiescence` against tokio 1.52.3's `src/io/util/mem.rs` in the registry copy Cargo.lock pins: `impl Drop for DuplexStream` calls `close_read` at 175-181, `close_read` sets `is_closed` and wakes the parked writer at 240-246, and `poll_write_internal` returns `BrokenPipe` when `is_closed` at 278-279; `PROBE` is 24 bytes; not run)
- Seen by: correctness (the lens ran after the other three; see the partition summary); refutation: confirmed (traced the same tokio path independently); history: no rationale found (d263a91d pinned the lossy control as a stall at the default window and "fails here as a hang on a collecting accept" arrived with a3da46c4; one-byte windows were considered on the passing path only, in `one_byte_windows_conform`)
- Owner-gated: no

`check_accept_cancellation`'s doc says a lossy acceptor "fails here as a hang on a collecting accept", and `lossy_accept_cancellation_is_caught` pins `Err(Quiescence::Stalled)` at `memory()`'s 8 KiB window, where both 24-byte `PROBE` writes and both `tx` drops complete inside the sender's first poll, so nothing is mid-write when a poll-drop cycle drops the dequeued `rx`. At any capacity smaller than `PROBE.len()` the second writer is still parked on its pipe when the cycle drops the `LossyAcceptor` future holding that stream's `rx`: `DuplexStream`'s `Drop` calls `close_read`, which sets `is_closed` and wakes the parked writer, whose next `poll_write` returns `BrokenPipe`, and the probe panics at `expect("contract: stream write")`, a message that indicts the connector for the acceptor's loss. The committed control runs at one capacity, so nothing pins which verdict a small-window transport gets, and the attribution the suite promises (`check`'s `# Panics`: "with a description of the clause") is exactly what this path gets wrong.

Evidence:

       877	        join_all(streams.into_iter().map(|mut tx| async move {
       878	            tx.write_all(PROBE).await.expect("contract: stream write");
       879	            tx.flush().await.expect("contract: stream flush");
       880	            drop(tx);
       881	        }))
       882	        .await;

    src/conformance/link.rs:
       810	/// in flight. An acceptor that internally dequeues a delivery and then
       811	/// awaits before returning it drops the dequeued stream with the
       812	/// cancelled future, and fails here as a hang on a collecting accept. A

    src/conformance/link/tests.rs:
       930	#[test]
       931	fn lossy_accept_cancellation_is_caught() {
       932	    let (a, b) = memory();
       933	    assert_eq!(
       934	        run_to_quiescence(super::check_accept_cancellation(a, lossy(b))),
       935	        Err(Quiescence::Stalled),
       936	        "the lost delivery must surface as a stall at the collecting accept",
       937	    );
       938	}

Resolution: Run the lossy controls at both tested capacities (a second test over `memory_with_capacity(1)` beside `lossy_accept_cancellation_is_caught`, or one test iterating both), and decide the verdict deliberately: either accept the sender-side failure and have the write's `expect` name both causes ("contract: stream write failed while the link is healthy: the transport errored, or the acceptor dropped a delivery under it"), or make the cancellation probe's writers tolerate `BrokenPipe` so the loss always surfaces at the collecting accept as documented. Update `check_accept_cancellation`'s doc to describe the failure at every tested capacity. Acceptance: a committed control runs the lossy fixture at a window smaller than `PROBE.len()` and asserts the chosen verdict; `check_accept_cancellation`'s rustdoc describes the failure a lossy acceptor produces at both tested capacities.
Construction: in src/conformance/link/tests.rs, `let (a, b) = memory_with_capacity(1); run_to_quiescence(super::check_accept_cancellation(a, lossy(b)))`. Expected today: a panic carrying "contract: stream write" rather than `Err(Quiescence::Stalled)`. If it stalls instead, the reading of the pipe's close semantics is wrong and this entry should be dropped.

### conformance-40: The decode-slot padding obligation `window.rs` places on the backend's price is checked by nothing in the suite
- Where: src/conformance/backend.rs:258-263 (related: src/conformance/backend.rs:398-407, src/tree/mirror/streaming/window.rs:139-156, src/tree/mirror/streaming/window.rs:165-176, src/tree/mirror/streaming/window.rs:397-399, src/tree/mirror/streaming/backend.rs:90-118, src/tree/mirror/streaming/backend/local.rs:108-110, src/tree/typed/prefix.rs:17-21, src/tree/mirror/streaming/window.rs:132, src/link.rs:169)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (arithmetic from types read in the tree: `Prefix<Z>` is `repr(transparent)` over tinyvec 1.11.0's `ArrayVec<[u8; 32]>` (34 bytes at alignment 2), `typed::Node<Z>` is pointer-sized by the const assertion at local.rs:110, so `(Prefix<Z>, typed::Node<Z>)` is 48 bytes and `FAN_SLOT_BYTES` is 40; a 16-byte node at 16-byte alignment makes the pair 64 bytes and the real slot excess 48; `STREAM_COUNT = 17` and `FAN = 256`, so the unaccounted term is 17 × 257 × 8 = 34,952 bytes; `grep -rn 'size_of::<(Prefix' src` finds only window.rs:176, and `grep -n size_of src/conformance/backend.rs` finds nothing)
- Seen by: correctness (the lens ran after the other three; see the partition summary); refutation: confirmed and widened (`REFERENCE_SLOT_BYTES` at window.rs:139-156 carries the same clause and is equally unchecked; a `const` assertion would forbid what the window docs ask `node_bytes` to price, so the fold-into-the-comparison form matches the stated obligation); history: no rationale found (the clause was added by 4add7f8a as a provenance note closing the size_of-derivation ruling of the 2026-07-23 review; the suite's stated scope at backend.rs:1-47 lists shape, pointwise, bulk, and end-to-end and has no "what the suite cannot see" section)
- Owner-gated: no

The window charges every decode-fan slot at `node_bytes(0, bound) + FAN_SLOT_BYTES`, where `FAN_SLOT_BYTES` is the size of the `(Prefix<Z>, typed::Node<Z>)` pair less the node under the in-memory backend, and its doc states that a backend whose `Node<Z>` demands wider alignment "owes that padding to its own `node_bytes` price"; `REFERENCE_SLOT_BYTES` carries the same clause for the per-level slots. The suite's leaf checks compare `measure(node)` (the node value alone) to `node_bytes(0, bound)`; neither side includes the slot pair's padding, the census counts node values only, and `Backend::node_bytes`'s own docs never mention slot padding. A backend whose leaf handle is 16 bytes at 16-byte alignment (an inline `u128`, say) makes the real pair 64 bytes against the 48 the constant assumes: 8 unaccounted bytes per occupant, 17 × 257 × 8 = 34,952 bytes per session, and every check here passes. An obligation stated in one module and checked in none is a criterion with no demonstration (Principle 2), and it is a compile-time fact per backend, so the check costs one comparison.

Evidence:

       258	        let priced = <N::Backend as Backend>::node_bytes(0, bound_bytes(&node));
       259	        if measured > priced {
       260	            ledger::violation(format!(
       261	                "underpriced leaf: measured {measured} B, node_bytes priced {priced} B",
       262	            ));
       263	        }

    src/tree/mirror/streaming/window.rs:
       171	/// The derivation is exact for pointer-class node handles; a backend
       172	/// whose `Node<Z>` demands wider alignment pads the real slot beyond
       173	/// `node_bytes + FAN_SLOT_BYTES` and owes that padding to its own
       174	/// `node_bytes` price.
       175	const FAN_SLOT_BYTES: usize =
       176	    std::mem::size_of::<(Prefix<Z>, typed::Node<Z>)>() - std::mem::size_of::<typed::Node<Z>>();

    src/tree/mirror/streaming/window.rs:
       150	/// parameters, so the leaf instantiation prices every level. Exact for
       151	/// pointer-class node handles; a backend whose `Node` demands a wider
       152	/// layout pads the real slots beyond this constant and owes that padding
       153	/// to its own `node_bytes` price.

Resolution: Fold the slot excess into the leaf comparisons at 258-263 and 401-407: `let slot_excess = size_of::<(Prefix<Z>, B::Node<Z>)>() - size_of::<B::Node<Z>>() - FAN_SLOT_BYTES` (with `FAN_SLOT_BYTES` made `pub(crate)`), recording `underpriced leaf` when `measured + slot_excess > priced`; do the same for `REFERENCE_SLOT_BYTES` against `(u8, B::Node<Z>)` where the per-level references are priced (`Charged::children`, once conformance-25 prices them). Prefer this form over a `const` assertion, because the window docs ask `node_bytes` to price the padding rather than forbid it. Add a negative control in backend/tests.rs: a `#[repr(align(16))]` wrapper node whose `node_bytes` omits the padding, asserted to fail by name. If measurement is not added, the backend module doc needs the "What the suite cannot see" section it lacks, naming slot padding. Acceptance: a backend whose `Node<Z>` alignment exceeds the pointer's fails `check` by name unless its `node_bytes` covers the extra slot padding; `Local` and `Materializing` pass unchanged.
Construction: define a test backend whose node wraps `typed::Node<Z>` beside a `u128` (alignment 16, so the pair excess is 48 rather than 40) with `node_bytes(c, b) = size_of::<ThatNode>() + b`, and run `check` under it: every pointwise check passes and the census counts node values only, so the 8-byte-per-slot shortfall appears in no assertion.

### conformance-41: `Charged::erase` and `assume` carry the pre-conversion measurement across the re-tag, transcribing the `Backend` re-tag clause without checking it
- Where: src/conformance/backend.rs:285-297 (related: src/conformance/backend.rs:82-85, src/conformance/backend.rs:132-139, src/conformance/backend.rs:363-369, src/tree/mirror/streaming/backend.rs:54-60, src/tree/mirror/streaming/backend.rs:77-88, src/tree/mirror/streaming/materialized.rs:481-491, src/tree/mirror/streaming/erased.rs:254-266, src/conformance/backend/tests.rs:286-303)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read; the in-memory conformance session reaches the re-tag through `greeting_fan`, `B::erase(node)` at materialized.rs:487, and `children_of`, `B::assume::<H>(node)` at erased.rs:259 and `B::erase(child)` at :264; `Charged::children` consumes the assumed parent through `into_inner` at backend.rs:369 without measuring it; `Measure` has no method over `Erased`)
- Seen by: correctness (the lens ran after the other three; see the partition summary); refutation: reframed (the carried measurement transcribes a stated trait clause, streaming/backend.rs:57-60 and :79-80, so the gap is that the clause has no demonstration; the check the clause calls for is equality of a post-`assume` measurement with the carried bytes, not a re-pricing at a capped fan; `erase` has no oracle without a `Measure` method over `Erased`); history: deliberate and holds for the shape (e5392c7c: "Charged settles and reopens its ledger entry, so the census peak is untouched"; the height-erasure record states that erasure re-tags rather than re-represents), with the premise unchecked
- Owner-gated: no

Both conversions re-wrap the converted handle with the *old* `bytes`. The comment argues the census peak is untouched, which is true only when the backend's erased and typed representations have identical residency, and that is exactly what the `Backend` trait's re-tag clause promises (a backend "can forget the tag ... and restore it ... without changing the value"; "`assume::<H>(erase::<H>(node)) == node` is the whole contract"). The suite transcribes the clause instead of checking it: a backend whose `assume` materializes (loads a child table into the typed handle) is charged at the erased size and compared to nothing, and the `Charged` doc's "keeping each node value's measured bytes on the ledger" does not hold across this path. The path is live in the conformance session: `greeting_fan` erases the root, `children_of` assumes it and erases every child, and `Charged::children` then consumes the assumed parent through `into_inner` without measuring it, so no knob can make the suite see a re-tag that changes residency. Principle 8 applied to the suite's own ledger: a carried number is a hypothesis and a measured one is a measurement, and the suite exists so a backend proves its account through `Measure`.

Evidence:

       285	    // Both conversions settle the wrapper's ledger entry and open an
       286	    // identical one around the re-tagged handle: the running total dips by
       287	    // one node's bytes between the two calls and never rises, so the
       288	    // census peak is untouched.
       289	    fn erase<H: Height>(node: Self::Node<H>) -> Self::Erased {
       290	        let bytes = node.bytes;
       291	        ChargedNode::wrap(B::erase(node.into_inner()), bytes)
       292	    }
       293	
       294	    fn assume<H: Height>(erased: Self::Erased) -> Self::Node<H> {
       295	        let bytes = erased.bytes;
       296	        ChargedNode::wrap(B::assume(erased.into_inner()), bytes)
       297	    }

    src/tree/mirror/streaming/backend.rs:
        57	    /// The height parameter on [`Node`](Self::Node) is a compile-time tag
        58	    /// over runtime data that already knows its place in the tree, so a
        59	    /// backend can forget the tag ([`erase`](Self::erase)) and restore it
        60	    /// ([`assume`](Self::assume)) without changing the value. The

    src/tree/mirror/streaming/backend.rs:
        79	    /// `H` must be the height the node was erased at:
        80	    /// `assume::<H>(erase::<H>(node)) == node` is the whole contract, and

Resolution: In `assume`, measure the result (`B::measure::<H>(&node)`), record `ledger::violation("re-tagged node changed residency: erased {bytes} B, assumed {measured} B")` when it differs from the carried bytes, and wrap at the measured value. `erase` has no oracle unless `Measure` gains a method over `Erased`, so either add `fn measure_erased(node: &Self::Erased) -> usize` and check symmetrically, or state at 285-288 that erasure is trusted on the trait's clause. Add an `ASSUME_SLACK` knob to `Materializing::assume` that grows the row, with a `#[should_panic(expected = "re-tagged node changed residency")]` control. Restate the comment: the peak is untouched when the re-tag leaves residency unchanged, which is the trait's clause and which the measurement after `assume` checks. If measurement is not added, the trust belongs in the backend module doc's accounting premises. Acceptance: the new control fails by name; `local_backend_conforms` and `materializing_backend_conforms` pass; the comment at 285-288 cites the trait clause it relies on.
Construction: add `static ASSUME_SLACK: Knob = Knob::new(0);` and in `Materializing::assume` do `row.resize(row.len() + ASSUME_SLACK.get(), 0)`; set it to `BULK_OVERHOLD` and run `check(Materializing, MATERIALIZING_BUDGET)`: today no violation is recorded (nothing measures or prices the assumed node), the ledger under-reports the slack, and the run passes.

### conformance-42: The monotonicity sweep compares adjacent grid points only, so a dip strictly inside a gap passes while the window evaluates the cost at the greeting's arbitrary bound
- Where: src/conformance/backend.rs:602-614 (related: src/conformance/backend.rs:548-569, src/conformance/backend.rs:571-583, src/conformance/backend.rs:636-641, src/tree/mirror/streaming/window.rs:355-370, src/tree/mirror/streaming/window.rs:383-384, src/tree/mirror/streaming/window.rs:397-399, src/tree/mirror/streaming/backend.rs:110-115, src/conformance/backend/tests.rs:175-181, src/conformance/backend/tests.rs:305-317, src/conformance/backend/tests.rs:502-514)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (read `sweep_bounds`: dense `0..=64`, then `power - 1, power, power + 1` for each power of two to `1 << 20`, so the gaps `65..=126` and `2^k + 2 ..= 2^(k+1) - 2` contain no grid point; the bound loop at 602-614 compares `bounds.windows(2)` only; `from_budget` evaluates `node_bytes` at `version_bound = 2 * (local + remote version bytes)` (window.rs:355-358), an arbitrary even integer, and its own `debug_assert` samples four fans and `version_bound / 2` against `version_bound` (360-370); `grep -rn 'proptest' src/tree/mirror/streaming/window` finds no property over `node_bytes`; `PRICED_DIP` is the sole monotonicity control and dips in fan at `DIP_FAN = 7`)
- Seen by: correctness (the lens ran after the other three; see the partition summary); refutation: confirmed, correcting the resolution (a proptest over a `2^20` range almost never samples a point dip at one bound; a step-shaped dip is what a property test catches reliably, and a denser grid catches the point dip at no cost since `node_bytes` is pure arithmetic); history: no rationale found (the grid's shape is deliberate, 8d959048, and its doc names round-size special-casing as the hazard the neighbors catch; nothing argues that a dip inside a gap is acceptable, and the sweep's own rustdoc names exactly that hazard)
- Owner-gated: no

`sweep_bounds` is dense to `BOUND_DENSE_CEILING` (64) and then each power of two with both neighbors, and the bound loop compares `node_bytes(fan, pair[0]) <= node_bytes(fan, pair[1])` for adjacent grid entries only, so a cost function that dips and recovers strictly inside a gap (`65..=126`, or `2^k + 2 ..= 2^(k+1) - 2`) passes; the fan dimension is exhaustive, so the gap is bound-only. `from_budget` evaluates `node_bytes(held, version_bound)` and `node_bytes(0, version_bound)` at the greeting's `version_bound`, twice the sum of the two sides' version bytes, an arbitrary even integer the grid need not contain, and its own `debug_assert` samples four fans and `version_bound / 2`. The realistic dip is a threshold: a backend that stores small bounds inline under a wider header and boxes large ones (`if bound <= 100 { 32 + bound } else { 16 + bound }`) prices 101 below 100 and recovers by 117, and the grid's 64 and 127 bracket it. The claim is a family (monotone over every pair `b1 <= b2`), the only committed control (`PRICED_DIP`) is a fan dip at a grid point, and the `Backend` doc and `check`'s `# Panics` scope the guarantee to "the swept grid", so the gap is stated without being admitted as one.

Evidence:

       602	    for pair in bounds.windows(2) {
       603	        for fan in 0..=FAN {
       604	            let here = B::node_bytes(fan, pair[0]);
       605	            let there = B::node_bytes(fan, pair[1]);
       606	            assert!(
       607	                here <= there,
       608	                "node_bytes must be monotone in version bound: bound {} prices {here} B, \
       609	                 bound {} prices {there} B, at fan {fan}",
       610	                pair[0],
       611	                pair[1],
       612	            );
       613	        }
       614	    }

    src/conformance/backend.rs:
       561	fn sweep_bounds() -> Vec<usize> {
       562	    let mut bounds: Vec<usize> = (0..=BOUND_DENSE_CEILING).collect();
       563	    let mut power = BOUND_DENSE_CEILING << 1;
       564	    while power <= BOUND_SWEEP_CEILING {
       565	        bounds.extend([power - 1, power, power + 1]);
       566	        power <<= 1;
       567	    }
       568	    bounds
       569	}

    src/tree/mirror/streaming/window.rs:
       355	        let version_bound = usize::try_from(
       356	            2 * (u128::from(local_version_bytes) + u128::from(remote_version_bytes)),
       357	        )
       358	        .unwrap_or(usize::MAX);

Resolution: Two changes, per the refutation's correction. Raise the dense ceiling (a few thousand bounds costs nothing, since `node_bytes` is pure arithmetic), which catches point dips. Add a `proptest!` in backend/tests.rs over `(fan in 0..=FAN, b1 in 0..=BOUND_SWEEP_CEILING, delta in 0..=BOUND_SWEEP_CEILING)` asserting `node_bytes(fan, b1) <= node_bytes(fan, b1.saturating_add(delta))` for `Local` and `Materializing`, with a step-shaped `PRICED_BOUND_STEP` knob (a header that shrinks above a threshold bound) and a control showing the grid sweep alone passes it while the property test fails it; commit the seed file the failing case writes. State in `node_bytes_monotone`'s doc that the grid is a sample and the property test covers the family. Acceptance: a step-shaped bound dip fails a committed test by name; a point dip at a non-grid bound below the raised dense ceiling fails the sweep; the sweep's doc states what it samples.
Construction: add `static PRICED_BOUND_DIP: Knob = Knob::new(0);` and in `Materializing::node_bytes` subtract it when `version_bound == 100`; set it to 1 and run `check(Materializing, MATERIALIZING_BUDGET)`: `node_bytes_monotone` compares (64, 127) and never evaluates at 100, so no assertion fires.

### Nits

The full record of each (provenance, history, evidence) is in the evidence file named in the last column.

| Id | Where | Claim | Resolution | Record |
|---|---|---|---|---|
| conformance-39 | `src/conformance/link/tests.rs:691-703` | `WindowedTx::poll_write` reads the grant under one lock and debits `available` under a second, so two writers polled concurrently double-spend and the second debit underflows; unreachable under `run_to_quiescence`, and the fixture states no single-poll premise | hold the one guard across the inner `poll_write`, or `checked_sub(..).expect(..)`, or state the premise at the fixture | `evidence/partitions/conformance.md` |

## Tree core

No traversal-level property states join's survivor set, so the three algebraic laws pass under an inverted leaf verdict; the associativity testdoc appeals to a differential that the V1 retirement deleted.

### tree-core-33: `join/tests.rs` says it covers deletion honoring, but every test in the file is direction-blind; no property in the tree states join's survivor formula
- Where: src/tree/traverse/join/tests.rs:1-7 (related: src/tree/traverse/unknown.rs:84-86; src/tree/traverse/join.rs:119-129 and 135; src/tree/tests.rs:294-299, 1182-1207, 1241-1252, 1310-1335, 1349-1383, 1746-1760; src/tree/mirror/streaming/materialized/unknown/tests.rs:70-83; tests/multi_peer.rs; tests/common/oracle.rs)
- Class / severity / confidence: verification-gap / low / high
- Provenance: assessed (traced, not run: `join_idempotent` exits at `ours == theirs`, join.rs:135, before any filter; `join_commutative` holds because each one-sided subtree is filtered against the other side's ceiling whichever side is `ours`; `join_associative` over disjoint parties is vacuous under an inverted leaf verdict, both orders empty; both changed-flag biconditionals compute `changed` from actual leaf-count movement, join.rs:122, so a wrong drop or gain moves flag and hash together)
- Seen by: correctness (35); refutation: reframed and lowered from medium (the direction is pinned elsewhere: `deep_divergent_join_changed_flag_is_exact`'s redaction leg, `join_destructor_unwind_leaves_tree_byte_identical`, the two poisoned-store pins, Route C of `tree_shape_is_canonical_in_the_leaf_set`, the differential `agrees_with_materialized_oracle` and `streaming_matches_join_oracle`, and end-to-end `tests/multi_peer.rs` against `tests/common/oracle.rs` all fail under inversion; what remains is the module-doc overclaim and the missing local property); history: already-known in part (the independent family-level check was `join_matches_mirror`, deleted by 368da2a50; the v1-retirement record acknowledges the loss and deferred the scoped-mutants check that would have surfaced this)
- Owner-gated: no

The module doc says the file covers "its deletion honoring by version dominance" and that these laws ground the join oracle the streaming suite relies on, but inverting the leaf arm at `unknown.rs:84` (keep leaves `<= known`, drop the rest) leaves all three laws passing, and no property anywhere in the tree states what join's survivor set is. Family coverage of the direction exists only end-to-end through gossip against the spec oracle in `tests/`, plus point fixtures here. An oracle that would agree with a wrong implementation is a blind spot; a property over `arb_divergent_pair` asserting the set formula directly would make the module doc true and ground the streaming oracle non-circularly.

Evidence:

    1    //! `Tree::join`'s algebraic laws and its deletion honoring by version
    2    //! dominance.
    3    //!
    4    //! Join is the in-memory oracle the wire reconciliation is differentially
    5    //! tested against (the streaming suites' join-oracle properties), so these
    6    //! laws — with the route-equivalence property in the tree's own suite —
    7    //! are what ground that oracle.

    src/tree/traverse/unknown.rs
    84            if causally::before(known).contains(node.ceiling()) {
    85                return None;
    86            }

Resolution: Add a proptest over `arb_divergent_pair()` and `arb_deep_divergent_pair()` comparing `join_tree(a, b)`'s leaf view to the set formula: a leaf with version `v` survives iff it is in both, or it is in `a` and `!causally::before(&Vb).contains(v)`, or it is in `b` and `!causally::before(&Va).contains(v)`; and the merged ceiling equals `Va | Vb`. Update the module doc to name it. Acceptance: the new proptest is committed, and under the reversible string swap at unknown.rs:84 (`contains` to `!contains`) it fails while the three laws still pass, demonstrating that it discriminates where they do not. Construction: build the expected view as a `BTreeMap<Vec<u8>, ()>` keyed by `version.as_bytes()` from `Tree::from_root(a).iter()` and `Tree::from_root(b).iter()` filtered by the formula; compare to the joined tree's leaf view; compare `joined.root.ceiling` to `a.ceiling | b.ceiling`. To confirm the blind spot first: apply the swap, run `just test tree::`, record which tests fail (the point fixtures and differentials listed above, none of the three laws), restore, verify `git diff` empty.

### tree-core-34: `join_associative`'s doc claims redaction-associativity coverage that no suite provides, by an argument that is now circular
- Where: src/tree/traverse/join/tests.rs:35-41 (related: src/tree/traverse/join/tests.rs:42-51; src/tree/arb.rs:92-119 and 132-172; src/tree/mirror/streaming/tests.rs:152-174)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (`grep -rn associativ src/ tests/` returns tree.rs:392 and join/tests.rs:35,43 only; the streaming property `streaming_matches_join_oracle` uses `Tree::join` as its oracle, streaming/tests.rs:152-161, so "join matches the mirror" cannot establish a law of join; the test's generators `arb_tree_root(0|1|2, 0..6)` put the three trees on disjoint parties, so no ceiling contains any other side's leaf and deletion honoring never fires)
- Seen by: prose (15), correctness (34); refutation: confirmed (the proposed three-way redacting generator passes by trace: a shared leaf redacted by one side drops in both association orders; side-only leaves survive both); history: deliberate-but-expired (the parenthetical was written in 329d891bf beside `join_matches_mirror`, a differential against the independent V1 mirror over a redacting generator, so the transitive argument was then defensible; 368da2a50 deleted that test and reversed the oracle direction, and the retirement plan named "associativity" among join's anchors without noticing this test covers only disjoint parties)
- Owner-gated: no

Every test's doc comment states its invariant and must be accurate; an inaccurate testdoc is a bug in the test (AGENTS.md). This one asserts coverage that does not exist and vouches for the oracle by what it is the oracle of. The property is a family claim the CRDT semantics rest on, and the case it skips is exactly where ceiling-based deletion inference could go wrong.

Evidence:

    35        /// The merge is associative over three mutually-disjoint trees.
    36        ///
    37        /// (Uses `arb_tree_root` on three distinct party indices so the three are
    38        /// pairwise disjoint; `arb_divergent_pair` bakes in parties 0/1/2 and so
    39        /// cannot be composed three-way. Associativity in the presence of redactions
    40        /// is covered transitively: `join` matches the mirror, which proves it under
    41        /// its own redacting generators.)
    42        #[test]
    43        fn join_associative(
    44            a in arb_tree_root(0, 0..6),
    45            b in arb_tree_root(1, 0..6),
    46            c in arb_tree_root(2, 0..6),

Resolution: Add a three-way divergent generator to `arb.rs` (a common base of shared inserts on party 0; three forks on parties 1, 2, 3, each with its own inserts and an arbitrary redaction subset of the shared keys, ceilings built by `Tree::act`) and a `join_associative_with_redactions` proptest asserting `join_tree(join_tree(a, b), c) == join_tree(a, join_tree(b, c))` (`Root` equality covers ceiling and content). Rewrite the testdoc of `join_associative` to state what it tests and drop the transitive-coverage sentence. Acceptance: `grep -rn associativ src tests` shows the new property; the testdoc for `join_associative` names no coverage it does not provide and makes no appeal to the mirror-versus-join differential. Construction: generator `base.act(p0, n_shared inserts)`; for each side i in 1..=3, `t = base.clone(); t.act(p_i, n_i inserts); t.act(p_i, forgets of drawn shared keys)`. Assert both association orders equal, and optionally all six permutations (commutativity composed).

## Tree typed

The layer's structural guards are debug assertions with no committed demonstration that they fire, and two testdocs still describe the 16-byte-digest era.

### tree-typed-7: No committed demonstration that the layer's `debug_assert!` guards fire
- Where: src/tree/typed/hash.rs:196-210 (related: src/tree/typed/untyped/fan.rs:155-158, src/tree/typed/prefix.rs:45-49, src/tree/typed/untyped.rs:280-291)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (`grep -rn 'should_panic\|catch_unwind' src/tree/typed/` returns nothing; .cargo/mutants.toml:33-35 says the campaign runs the dev/test profile so debug assertions are part of the observer)
- Seen by: correctness; refutation: confirmed; history: no-rationale-found, scope disputed in part (an inverted guard panics on good inputs and fails the committed suites; only a weakened comparison is uncovered)
- Owner-gated: no

The layer's structural guards are debug assertions: ascending radix order and the one-child ban in `Hash::branch`, `Fan::push`'s order check, `ErasedPrefix::assume`'s length-vs-height witness, and `from_sorted_leaves`'s ascending-run and bare-leaf checks. Nothing under `src/tree/typed/**` uses `should_panic` or `catch_unwind`. A guard whose comparison weakens (`<=` for `<`, a dropped conjunct) passes every committed test while protecting nothing, and cargo-mutants does not mutate inside macro invocations, so the campaign does not see these conditions either (assessed). Adequacy defense: a criterion needs a committed demonstration that a known-bad input fails it.

Evidence:

       196	            debug_assert!(
       197	                previous.is_none_or(|previous| previous < radix),
       198	                "branch children must arrive in strictly ascending radix order",
       199	            );
    ...
       207	        debug_assert!(
       208	            count != 1,
       209	            "a one-child branch is unrepresentable under the canonical-shape invariant",
       210	        );

Resolution: one `#[cfg(debug_assertions)] #[should_panic(expected = "...")]` test per guard in the guarded function's sibling tests.rs (prefix.rs gets a new `prefix/tests.rs` and `mod tests;`). Acceptance: each new test fails when its guard is deleted or weakened to `<=`, and passes with it present; the gate's `testdoc` sees a doc comment on each.
Construction: hash/tests.rs: `Hash::branch(&[], [(2u8, Hash::default()), (1u8, Hash::default())])` expecting "strictly ascending radix order"; `Hash::branch(&[], [(0u8, Hash::default())])` expecting "one-child branch is unrepresentable". fan/tests.rs: `let mut f = Fan::new(); f.push(3, child()); f.push(3, child());` expecting "not greater than the current last". untyped/tests.rs: two entries sharing one `[u8; 32]` path through `Node::from_sorted_leaves(0, &mut entries)` expecting "strictly ascending by path"; a one-entry run whose leaf was pre-wrapped with `.beneath(0)` expecting "supplies bare leaf nodes". prefix/tests.rs: `Prefix::<Root>::new().erase().assume::<Z>()` expecting "re-tags at the height it was erased at".

### tree-typed-9: Testdocs out of step with the code: 17-byte child records, every-node openers on root-only tests, "version" for "ceiling"
- Where: src/tree/typed/hash/tests.rs:10-12 (related: src/tree/typed/untyped/tests.rs:460-462, 216-223, 242-249, 118-127, 172-177; src/tree/typed/hash.rs:79)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (`CHILD_RECORD_LEN = 1 + MERKLE_HASH_LEN` with `MERKLE_HASH_LEN = 24` at hash.rs:12 and 79; `git log -S'17-byte'` shows the phrase written at 5a6dd8a21 under the 16-byte width; `git show 2d1e6ea51 | grep 17-byte` shows the widening commit removed two other "17-byte" mentions and left these two; the root-only bodies read at untyped/tests.rs:225-240 and 251-270)
- Seen by: prose ([30]), prose/history ([32] second half); refutation: reframed ("17-byte" is wrong, not merely unnamed; severity raised); history: [30] deliberate-but-expired (`version()` became `ceiling()` at f7257af49 and the helper docs were not reworded)
- Owner-gated: no

Two testdocs describe the child record as 17 bytes; it is 25 (`CHILD_RECORD_LEN`), the 17 surviving from the 16-byte-digest era. `version_is_join_of_leaf_versions` and `floor_is_meet_of_leaf_versions` open with "Every node's ..." but check only `tree.ceiling()` and `tree.floor()`; the per-node claim is what `bounds_are_the_leaf_fold_at_every_node` checks. The helper docs at 118-127 and 172-177 call a node's bound its "version" and say "branch versions are recomputed by `Node::branch`", though bounds are memoized lazily under the names `ceiling`/`floor`. AGENTS.md: an inaccurate testdoc is a bug in the test.

Evidence:

        10	/// The prefix is length-tagged in one byte, the child count is a big-endian
        11	/// `u16`, then 17-byte records follow in the iteration order given, with no
        12	/// other framing or padding.

    src/tree/typed/untyped/tests.rs:
       462	    /// ascending 17-byte `radix ‖ hash` records.

       216	    /// Every node's ceiling is the join of its descendant leaves' versions.

       125	    /// leaves pass their original version back into `Node::leaf`, and branch
       126	    /// versions are recomputed by `Node::branch` from the same per-child
       127	    /// versions we started with.

Resolution: replace "17-byte" with "`CHILD_RECORD_LEN`-byte" (or "`radix ‖ hash`") at both sites; open the two root tests with the root claim ("The root's ceiling is the join of every leaf's version") and point at `bounds_are_the_leaf_fold_at_every_node` for the per-node statement; replace "version", "branch versions", and "root version" in the helper docs with "ceiling" or "bounds", and replace "recomputed by `Node::branch`" with "memoized lazily from the rebuilt children". Acceptance: `grep -rn '17-byte' src/tree/typed` is empty; each testdoc's first sentence is checked by its own body; no helper doc in the file calls a bound a "version".

## Mirror common (cbor, framing, handshake, party, protocol, driver, erasure)

One totality strategy that never leaves the rejection arm, and one runtime-per-case idiom beside an unstated range exclusion.

### mirror-common-14: The preamble totality proptest almost never passes the magic check, so it demonstrates one arm of the parser it claims is total
- Where: src/tree/mirror/handshake/tests.rs:257-263 (related: src/tree/mirror/handshake.rs:111-119, src/tree/mirror/handshake/tests.rs:210-255, src/tree/mirror/handshake/tests.rs:296-334)
- Class / severity / confidence: verification-gap / low / high
- Provenance: assessed (read: the strategy draws all 30 bytes from `any::<[u8; V2_PREAMBLE_LEN]>()`, so the 11-byte prefix matches `V2_PREFIX` with probability 2^-88)
- Seen by: correctness; refutation: confirmed; history: no rationale found (strategy unchanged since 4dd2053c9; the CBOR-wire review's clean bill covers diagnostic order, not this strategy's reach)
- Owner-gated: no

`arbitrary_bytes_never_panic` returns `MagicMismatch` at :112-117 on effectively every case, so the version, network, and intent arms are never reached under this strategy. Those arms are exercised by `arbitrary_preamble_decodes_by_the_oracle`, but only with one-byte version and intent heads (`0..=0x17`) and the fixed network head `0x50`; canonical wide version heads (`0x19 0x01 0x00`), wide or widened network heads, and multi-byte intent heads are outside every strategy in the file (the two wide-version point tests at :296-334 are the only exceptions). The testdoc claims "the parser is total over its fixed-width input"; a generator that never leaves the rejection arm would also pass a parser that panicked on any valid prefix. Property tests sample the family the claim is about.

Evidence:

    257	    /// Arbitrary bytes in the preamble's place decode to a typed error or
    258	    /// a valid preamble, never a panic: the parser is total over its
    259	    /// fixed-width input.
    260	    #[test]
    261	    fn arbitrary_bytes_never_panic(bytes in any::<[u8; V2_PREAMBLE_LEN]>()) {
    262	        let _ = Preamble::decode(&bytes);
    263	    }

Resolution: Supplement the strategy with a prefix-valid arm, weighted heavily: `prop_oneof![1 => any::<[u8; V2_PREAMBLE_LEN]>(), 8 => any::<[u8; 19]>().prop_map(|tail| { let mut b = [0u8; V2_PREAMBLE_LEN]; b[..11].copy_from_slice(&V2_PREFIX); b[11..].copy_from_slice(&tail); b })]`, and optionally a third arm that also fixes the version byte at `Protocol::V2 as u8` so the network and intent arms are reached on most cases. Acceptance: a temporary `unreachable!()` inserted after the magic check (handshake.rs:119) is hit by the suite; the committed strategy reaches every `Err` arm of `decode` (checked once by counting arm hits).
Construction: insert `unreachable!("reached past the magic check")` at handshake.rs:119 and run `arbitrary_bytes_never_panic` alone: it passes all 256 default cases at this commit.

### Nits

The full record of each (provenance, history, evidence) is in the evidence file named in the last column.

| Id | Where | Claim | Resolution | Record |
|---|---|---|---|---|
| mirror-common-5 | `src/tree/mirror/cbor/tests.rs:10-10` | proptests draw `major in 0u8..7` with no stated reason for excluding 7; a tokio runtime is built per case where `pollster` serves | name `WRITER_MAJORS` with its reason; use `pollster::block_on` | `evidence/partitions/mirror-common.md` |

## Streaming backend and window

Every figure the window's prose quotes is pinned by recomputation, but the dominance certificate the 2^-40 bound rests on is external and unrun, the leaf-request term's identical zero lives in a 22-line comment with no pin, the coverage rosters are hand-maintained, and four testdocs claim more than their bodies check.

### streaming-backend-window-32: Integer-envelope dominance is certified only by an example no recipe runs, and that example checks a different family than the shipped code
- Where: src/tree/mirror/streaming/window.rs:570-576 (related: examples/envelope_sim.rs:17-31; justfile:774, 889-973; .github/workflows; window/tests.rs:354-381)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (grep of the justfile and .github/workflows: the only examples run are `window_tradeoff` and `before`'s `amp_board`; the justfile's "envelope suite" at :655 is `before`'s amplification envelope, not this one; envelope_sim.rs:28-31 states it certifies "the one-corpus `N` forms" while window.rs implements "the pair-based `A·B` adaptation"; window/tests.rs, read in full, never compares an integer quantile to an exact tail: `envelopes_are_consistent` checks internal ordering only)
- Seen by: correctness; refutation: confirmed; history: no-rationale-found (ded24eb3 named the example the certifying tool of record after deleting the Python simulator, but no commit or note decides whether it runs under the gate; the scope limitation is stated in the example's own header and was never closed)
- Owner-gated: yes: the pin itself is not gated, but the disposition of `envelope_sim` (gate leg, or superseded and retired) touches a recorded decision and gate policy

The module's 2^-40 per-session bound rests on each integer quantile dominating its exact-Chernoff counterpart. The comment cites the example as the certificate; nothing runs it, and its own header says it certifies a different family than `jointly_occupied`/`stage_population`. A change to `UNION_TAIL_BITS`, `BERNSTEIN_TAIL`, `TAIL_DEPTH_CAP`, or the `b >= 5` / `t/(b-3)+2` constants passes the gate unchallenged today. Principle 2: any quantity computable two ways gets a committed comparison, and a status board nothing enforces is decoration.

Evidence:

       570	// The functions below bound the resulting occupancy statistics from
       571	// above with pure integer arithmetic. Each integer quantile is
       572	// constructed to dominate its exact-Chernoff counterpart, and the
       573	// dominance is verified by `examples/envelope_sim.rs` over a dense
       574	// sampled sweep of (N, depth) — sampled, not exhaustive — so on that
       575	// certificate the integer envelopes inherit the exact tails' joint
       576	// ≥ 1 − 2⁻⁴⁰ per-session bound (UNION_TAIL_BITS counts the union). All

Resolution: Add a committed proptest in window/tests.rs over log-uniform `(a, b)` corpora and `depth in 1..=KEY_DEPTH` asserting each integer quantile is at least the exact-distribution quantile at its stated tail level: `leaves_quantile(n, j)` against the least `q` with `P(Binomial(n, 256^-j) >= q) <= 2^-(48 + 8 min(j, 40))` via a log-space survival function; `jointly_occupied(n, pair, j)` against the Poisson upper tail at mean `pair / 256^j` and level 2^-48; `child_slots_quantile` against the occupied-child-slot count. Then decide the example's fate: a `just` leg (`cargo run --release --example envelope_sim`) or retirement once the in-tree test demonstrably catches what the example caught. Acceptance: a committed test fails when any integer quantile is lowered below its exact counterpart (demonstrate by changing `+ 2` to `+ 1` in `small_mean_quantile`, or `BERNSTEIN_TAIL` to 20, and observing the failure), and the module comment cites that test by name.
Construction: For fixed grids first (`n` in {10^3, 62_500, 10^6, 10^10, 2^40, 2^63}, `j` in 1..=32): compute `log_sf(q) = log P(X >= q)` for `X ~ Binomial(n, 256^-j)` by summing `exp(lgamma terms)` from `q` to `n` (or the Poisson with `mu = n * p`, which upper-bounds the binomial tail for small `p`), find the least `q` with `log_sf(q) <= -t ln 2`, and assert `leaves_quantile(n, j) >= q`; then generalize to the proptest.

Cross-reference: benches-envelope-32 (the example's side: its integer copies, the `pair = n²` restriction, and the gating gap).

### streaming-backend-window-30: The leaf-request charge term is identically zero for every u64 corpus; the proof lives in a 22-line comment with no committed pin
- Where: src/tree/mirror/streaming/window.rs:409-443 (related: window.rs:162-163, 649-666, 750-760; window/tests.rs:57-58, 177-186, 210)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (Python replica of window.rs:580-760 at `scratchpad/final/envelope.py`: `stage_population(n, pair, 32)` is 0 at (u64::MAX, u64::MAX), (2^63, 2^63), (10^10, 10^10), (62_500, 62_500), (1, 1), and (u64::MAX, 1); at the maximal pair every depth from 25 down is zero, depth 24 is 12; raising `UNION_TAIL_BITS` to 55 makes depth 25 nonzero (12) and to 111 makes depth 32 nonzero (12), while lowering it to 8 widens the zero region)
- Seen by: structure, correctness; refutation: confirmed, with the demonstration's sign corrected (the guard at 658 is `num_bits + t + 2 < den_bits`, so a smaller `t` makes more populations zero); history: deliberate-and-holds for keeping the term (655d2ae9 chose to charge "the identical expression the grant uses" so both sides of the function state one bound; e82b862e added the exact thresholds); the absence of a pin is the un-ruled half
- Owner-gated: no: adding the pin reopens nothing; deleting the term would reopen 655d2ae9 and is the owner's call

`population[KEY_DEPTH]` multiplies `jointly_occupied(n, pair, 30)`, whose quantile is zero whenever `bitlen(pair) + 50 < 241`, and `pair` is a product of two `u64`s. So `LEAF_REQUEST_BYTES` never contributes, heights 0 through 7 always receive the floor, and the comment exists to prove it. Nothing tests the claim; `deep_levels_are_sparse` asserts `<= 16` at heights 0..=22 for one corpus. A change to `UNION_TAIL_BITS`, the `+ 2` headroom, or `TAIL_DEPTH_CAP` would falsify the comment's proof with nothing failing. Doctrine: every invariant held in prose becomes a committed check, and the solve should read as evidently right without a page of argument about a line that does nothing.

Evidence:

       419	        // the granted statistic is `jointly_occupied(n, pair, 30)`
       420	        // times the per-parent fan, whose quantile is zero for every
       421	        // representable corpus (`small_mean_quantile` at j = 30 has 241
       422	        // denominator bits against at most 128 for a product of two u64
       423	        // corpora; nonzero needs pair ≥ 2¹⁹⁰). The entries that could
    ...
       442	            total.saturating_add(population[KEY_DEPTH].min(k) * LEAF_REQUEST_BYTES as u128)

Resolution: Add a proptest `deep_stage_populations_are_zero(a in any::<u64>(), b in any::<u64>())` asserting `stage_population(n, pair, d) == 0` for `d in 25..=KEY_DEPTH` with `n = max(a, b)` and `pair = a * b`, cite it from the comment, and shrink the comment to the invariant (two sentences: the edge is charged at the width it is granted; that width is zero for every u64 pair, pinned by the test, so the edge floors at one slot). Keep the term for formula totality per 655d2ae9. The test-side `charge()` replica (tests.rs:57-58) and `scope_envelope_matches_the_derivation` (tests.rs:210) keep their matching term. Acceptance: the committed test fails when `UNION_TAIL_BITS` is raised to 111 (depth 32 becomes nonzero at the maximal pair) and passes at HEAD; the comment block is reduced to the invariant; `SCOPE_ENVELOPE_BYTES` and `tradeoff.md` are unchanged.
Construction: Temporarily set `UNION_TAIL_BITS = 111` and run the proposed test with `a = b = u64::MAX`: `stage_population(n, pair, 32)` returns 12. At `UNION_TAIL_BITS = 55`, the depth-25 assertion fails first (population 12).

### streaming-backend-window-37: Testdoc quotes 60 B / 65,404 / ~4.3× while its assertions and `Peer::sync_memory_budget` say 52 B / 91,941 / ~2.6×
- Where: src/tree/mirror/streaming/window/tests.rs:266-310 (related: src/peer.rs:421-429, 441)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (`git show 4dd2053c^:...window/tests.rs` has `Some(60)` and `65_404` under these doc lines; `git show 4dd2053c:...` has `Some(52)` and `91_941` with the doc untouched; `git blame` puts lines 269 and 273 in ba8045c5 and lines 299, 307, 308 in 4dd2053c; peer.rs:422, 425, 441 quote 52 B and ~2.6×)
- Seen by: structure, prose, correctness, perfapi; refutation: confirmed; history: no-rationale-found (4dd2053c's message says it corrected the assert message's stale "~4.6x" note and re-pinned the figures, and its diffstat includes peer.rs; the doc comment above was not touched)
- Owner-gated: no

The test exists to stop quoted figures drifting, and its own header is the drifted figure. AGENTS.md: every test's doc comment states its invariant and review holds it to the standard; an inaccurate testdoc is a bug in the test.

Evidence:

       269	/// `m* = 60 B` (quoted at `Peer::sync_memory_budget`) is the
       270	/// smallest record size whose self-consistent corpus — the spec BDP in
       271	/// `m`-size records, per side — fits entirely inside the window the
       272	/// default budget derives at that corpus; the u64 column's BDP-scale
       273	/// corpus derives a 65,404-scope window, the quoted ~4.3× figure.
    ...
       299	        Some(52),
    ...
       307	        91_941,
       308	        "the u64 BDP-scale window moved: update the ~2.6x figure quoted at \

Resolution: Rewrite the doc without the literals ("the crossover record size and the u64 BDP-scale window are the solve's own numbers, pinned here and quoted at `Peer::sync_memory_budget`") so the next re-pin cannot reopen the gap; the assertions and their messages remain the record. If figures must stay in the doc, name them once as constants used by both the assertion and the message. Acceptance: no number in the doc comment disagrees with the assertions below it or with peer.rs:422-441.

### streaming-backend-window-15: Testdoc claims the wrappers self-wake; the body's noop waker cannot observe a wake
- Where: src/tree/mirror/streaming/backend/local/adversarial.rs:137-156 (related: adversarial.rs:107)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (read: the context is built from `Waker::noop()` at line 140, so `cx.waker().wake_by_ref()` at 107 is unobservable; the assertions check only the Pending/Ready sequence)
- Seen by: prose; refutation: confirmed; history: no-rationale-found (the doc line was added in the bulk testdoc sweep 358c6b1a over an unchanged body)
- Owner-gated: no

The doc states the property that keeps a delayed poll from stranding its task; the body cannot detect its loss. Every test doc states the invariant it protects and must be accurate.

Evidence:

       137	    /// Scheduled wrappers self-wake for every delay, then preserve the backend result or item.
       138	    #[test]
       139	    fn future_and_stream_delays_self_wake_then_complete() {
       140	        let waker = Waker::noop();
       141	        let mut cx = Context::from_waker(waker);

Resolution: Poll with a counting waker (a `std::task::Wake` impl over an `AtomicUsize`) and assert the count equals the scheduled delays (3 for `vec![2, 1]`), or narrow the doc to the Pending/Ready sequence it checks. Acceptance: the body asserts a wake count, or the doc no longer claims self-waking.
Construction: Delete line 107 (`cx.waker().wake_by_ref();`) and run this test: it still passes, while any scheduled streaming test hangs. A counting waker turns that into a failing assertion here.

### streaming-backend-window-16: `QueueKind::ALL`/`PROXY` are hand-maintained rosters the coverage tests trust; the type's summary names only the materialized graph
- Where: src/tree/mirror/streaming/channel.rs:8-57 (related: src/tree/mirror/streaming/tests/capacity.rs:125-135; src/tree/mirror/streaming/remote/proxy/tests.rs:601-606, 629)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (counted seventeen variants at lines 11-27 against the fourteen-plus-three roster entries; grep: the only consumers are capacity.rs:125 and proxy/tests.rs:601, 629; no exhaustive `match` over `QueueKind` exists; `strum` is absent from Cargo.toml)
- Seen by: structure, correctness, perfapi, prose (the doc half); refutation: confirmed, severity down to low (the instrument catches every listed edge; the escape is one variant added without its roster entry); history: no-rationale-found
- Owner-gated: no

`capacity_stress_covers_every_queue_role` and the proxy coverage test iterate these arrays to assert every edge was constructed and exercised. A variant added to `QueueKind` but not to a roster compiles and passes both tests vacuously: the coverage instrument cannot detect its own roster's incompleteness. The type's first sentence says "the materialized protocol's channel graph" while three variants are proxy edges. Doctrine: no hand-maintained enumerations of facts the code can change without touching them.

Evidence:

         8	/// One semantic edge in the materialized protocol's channel graph.
    ...
        31	    /// Every materialized semantic edge, for its coverage assertions.
        32	    #[cfg(test)]
        33	    pub const ALL: [Self; 14] = [
    ...
        50	    /// Every remote-proxy semantic edge, for its coverage assertions.
        51	    #[cfg(test)]
        52	    pub const PROXY: [Self; 3] = [

Resolution: Lightest: a `#[cfg(test)]` test with an exhaustive `match` over every variant asserting `ALL.contains(&kind) || PROXY.contains(&kind)` and that the rosters are disjoint; the compiler then forces the roster update. Or derive the roster (`strum::EnumIter` as a dev-dependency) partitioned by an exhaustive `fn side(self) -> Side`. Reword line 8 to name both graphs. Acceptance: adding a variant to `QueueKind` without updating a roster fails to compile or fails a committed test.

### streaming-backend-window-36: Testdoc claims every mid-depth level pipelines; the body checks one height
- Where: src/tree/mirror/streaming/window/tests.rs:150-167 (related: window.rs:458-472)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (read)
- Seen by: prose; refutation: confirmed (and notes the universal is doubtful on its own terms: a level whose population is 2 to 16 gets capacity equal to its population, not "well past" the floor); history: no-rationale-found
- Owner-gated: no

The doc states a universal over every level whose population exceeds one; the body asserts `capacity(KEY_DEPTH - 4) > FAN` at one height. The comment inside the body is accurate about that point.

Evidence:

       150	/// The default pairing pipelines where population lives: every mid-depth
       151	/// level whose population exceeds one gets capacity well past the
       152	/// serialization floor.
    ...
       166	    assert!(window.capacity(KEY_DEPTH - 4) > FAN);

Resolution: Either iterate the depths and assert `capacity(KEY_DEPTH - depth) > 1` wherever `stage_population(n, n * n, depth) > 1` (the helpers are imported), or reword the doc to the point it checks: depth 4, the first stage whose population outgrows the structural caps, receives capacity above a full fan under the default budget. Acceptance: doc and assertions describe the same set of heights.

### streaming-backend-window-38: Uniform `u64` proptest strategies for corpus size effectively never sample small corpora
- Where: src/tree/mirror/streaming/window/tests.rs:359-362 (related: window/tests.rs:376, 403-404)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (proptest's `RangeFrom<u64>` and `Range<u64>` strategies sample uniformly, so values below 2^32 arrive with probability 2^-32; the fixed-point tests at 62,500, 10^6, 10^10, and 2^24 are the only small-corpus coverage)
- Seen by: correctness; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

`window_stays_inside_the_budget` and `envelopes_are_consistent` draw `messages in 1u64..` and `0u64..`, landing in [2^62, 2^64) three quarters of the time. The family where near-root populations are sub-fan and the Poisson branch of `small_mean_quantile` fires at shallow depths, which is where real sessions live and where an off-by-one in the bit-length comparison would show, is a point test with extra steps. Every window proptest also passes symmetric pairs.

Evidence:

       359	    fn window_stays_inside_the_budget(
       360	        messages in 1u64..,
       361	        budget in 0usize..=1 << 44,
       362	    ) {

Resolution: Use a log-uniform strategy (for example `(0u32..64).prop_flat_map(|bits| { let lo = 1u64 << bits; lo..=lo.saturating_mul(2).saturating_sub(1) })`) for both tests, and draw asymmetric `(a, b)` pairs in `window_stays_inside_the_budget`. Acceptance: both strategies produce corpus sizes spanning 1..2^63 with roughly equal mass per octave; asymmetric pairs are drawn.

### Nits

The full record of each (provenance, history, evidence) is in the evidence file named in the last column.

| Id | Where | Claim | Resolution | Record |
|---|---|---|---|---|
| streaming-backend-window-18 | `src/tree/mirror/streaming/channel/instrumented.rs:77-86` | the instrumented occupancy counter can wrap then overflow under a multi-threaded executor (latent: every in-crate driver is single-threaded) | `wrapping_add` on the returned value, or state the single-thread premise at `Stats` | `evidence/partitions/streaming-backend-window.md` |

## Materialized (the in-process walk)

One high: the leaf-height deletion verdict has no discriminating test. Two low: the whole-subtree shed count is never pinned above one, and the resolver's skip-past supply arm has no injection. Two test-quality: a design-space checker fed only a literal, and an exhaustive violation suite entering at internal walk entries without stating the decision.

### materialized-27: The leaf-height deletion verdict in `unknown` has no committed test that fails when it is inverted
- Where: src/tree/mirror/streaming/materialized/unknown.rs:88-97 (related: src/tree/mirror/streaming/materialized/unknown.rs:99-108, 156-167; src/tree/mirror/streaming/materialized/unknown/tests.rs:29-48, 66-83; work/answer.rs:136-144, 182; src/tree/arb.rs:509-513, 625-670; src/tree/mirror/streaming/tests/fixtures.rs:29-32, 494-502; .agent-notes/2026-08-21-unknown-pruning-survivor/README.md)
- Class / severity / confidence: verification-gap / high / medium
- Provenance: assessed (read; the survival itself is agent-reported in the handoff note and cannot be re-run without modifying the tree; the reachability argument and the fixture-role analysis are by reading: `git log -- .agent-notes/2026-08-21-unknown-pruning-survivor` shows one commit, 1a7afe1e; `grep unknown .cargo/mutants.toml` finds no exclusion; `leaf_sibling_path` is used only by the two `leaf_parent_*_pair` fixtures, arb.rs:575-654)
- Seen by: correctness; refutation: confirmed and reframed (the module's own differential oracle builds its tree from `Path::for_leaf` hashed paths, so it never reaches the arm either); history: already known (the note asks for exactly this construction; no follow-up has landed; the v1-retirement note's decision 3 deferred the scoped mutants re-check)
- Owner-gated: no

The height-0 arm is the only place the streaming prune judges an individual leaf, and a known-bad mechanism (the inverted verdict) passes the whole suite. Reading the reachability explains why: a single leaf reached from any height above is a path-compressed spine whose `span()` is a point, so `knowledge()` at 99-108 answers `Before` or `After` and the recursion never descends to height 0. The arm runs only when a real height-1 branch (two or more leaves sharing 31 path bytes) classifies `Between` and the holder's counterparty lacks the parent (the `answer::internal` Left arm, `unknown_providing`, or the initiator's early supplies). Every committed fixture avoids that shape: hashed-path generators cannot reach height 1 (`unknown/tests.rs:39` uses `Path::for_leaf`), the divergence fixtures keep extras concurrent ("nothing is deletion-pruned when provided across", fixtures.rs:29-32), and `leaf_parent_redaction_pair` has both sides hold the parent, so its verdict lands in `answer::leaf_parent`'s `known` filter (answer.rs:138), not here. Under the inversion the receiver's `Resolver` absorbs the re-supplied redacted leaf (its containment check passes because the sender holds it), so this is a behavioral hole in the redaction contract, not a quantitative one. Doctrine: every criterion needs a committed demonstration that a known-bad mechanism fails it, and "redaction leaves no tombstones" rests on exactly this verdict.

Evidence:

    88	        if prefix.height() == 0 {
    89	            // A leaf is known iff its ceiling is causally at or before
    90	            // `known`; a concurrent ceiling is beyond the known-at range,
    91	            // so those survive.
    92	            let verdict = Some(node).filter(|node| !self::known(node, known));
    93	            if verdict.is_none() {
    94	                stats.shed(1);
    95	            }
    96	            return Ok(verdict);
    97	        }

Resolution: Two committed tests. (1) In `unknown/tests.rs`, a `leaf_sibling_path` variant of `tree_and_known` (k leaves under one shared 31-byte prefix with per-leaf known flags) driven through the existing differential oracle, the cheapest kill. (2) A cross-peer fixture beside `leaf_parent_redaction_pair` in `src/tree/arb.rs`: side `a` holds leaves at `leaf_sibling_path(0x00)` (party 0) and `leaf_sibling_path(0x01)` (party 1); side `b` has forgotten 0x00 and never held 0x01, so it holds nothing under that parent, with ceiling `v00 | tick(party 2)`; drive it through `streaming_mirror_sides` in both orientations, assert both endpoints equal `join_oracle`, and assert the holder sheds exactly one and the other gains exactly one. Then generalize (2) into a proptest with `Tree::join` as oracle. Verify the kill by applying the inversion as a reversible string swap, confirming the new tests fail, restoring, and checking `git diff` is empty. Record the disposition in the handoff note (which may then be retired). Acceptance: a committed test fails with `!self::known` replaced by `self::known` and passes on HEAD.

Construction: `a = act(None, [(leaf_sibling_path(0x00), v00 on party 0, Insert), (leaf_sibling_path(0x01), v01 on party 1, Insert)])`; `b` = empty root with ceiling `v00 | forget_tick(party 2)`. Whichever side initiates, a's root child at radix 0x00 is exclusive: as initiator it flows through the early-supply `unknown` call (levels.rs:111); as responder through `answer::internal`'s Left arm (answer.rs:80). The compressed spine's span is `[meet(v00, v01), v00 | v01]`, `Between` against b's ceiling, so the recursion descends to the height-1 branch and judges each leaf: 0x00 known (shed), 0x01 unknown (travels). Expected: both sides hold {0x01}. Mutated (`!` removed): b receives 0x00, the redacted leaf, and not 0x01; equality with `join_oracle` fails.

Demonstration (`witness/results.md`): The cross-peer construction converges to `join_oracle` in both orientations on HEAD; with the `!` at unknown.rs:92 removed, the initiating side ends holding the redacted leaf at the all-zero path and lacking the concurrent sibling. The construction discriminates the arm; whether the mutant survives the whole committed suite (the handoff note's claim) was not re-run.

### materialized-28: The whole-subtree `shed` arms are never pinned at a count above one
- Where: src/tree/mirror/streaming/materialized/unknown.rs:103-106 (related: src/tree/mirror/streaming/materialized/unknown.rs:149-152; src/tree/mirror/streaming/tests/stats.rs:185-186, 208-210, 258-259; tests/session_stats.rs:64-66, 109-111, 302-335; src/tree/mirror/streaming/stats.rs:94-97)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified for the gap (grep `messages_shed` across tests/ and streaming/tests/: every assertion is 0, 1, or the conservation identity; the conservation proptest issues only `send_all`, so nothing is redacted); assessed for the kill (by reading)
- Seen by: correctness; refutation: confirmed; history: no rationale found (7626543e pinned "Redaction honored reads as one shed"; nothing above one was recorded or deferred)
- Owner-gated: no

`SessionStats::messages_shed` promises "a pruned subtree adds its exact live-leaf count, so the number always reads in messages", but replacing `stats.shed(node.len() as u64)` with `stats.shed(1)` at either site survives the suite. Any quantity computable two ways gets a committed test comparing them; `messages_gained`'s multi-leaf arm is covered by the conservation proptest, so the gap is on the shed side only.

Evidence:

    103	            Dominance::After => {
    104	                stats.shed(node.len() as u64);
    105	                return Ok(None);
    106	            }

Resolution: Add a redaction schedule to `sessions_conserve_the_live_count` (a `Forget` over the shared base, as `arb_divergent_pair` already does in-crate) so `after = before + gained - shed` exercises shed > 0; and add a walk-tier pin where one side holds k >= 2 leaves under one root radix the other has seen and forgotten, asserting `messages_shed == k`. Acceptance: a committed test fails when either `stats.shed(node.len() as u64)` is replaced by `stats.shed(1)`.

Construction: a holds leaves p0, p1 under root radix R (both party 0, v1 < v2); b has forgotten both (ceiling >= v2, nothing under R, ballast elsewhere). Gossip: a's root child R is exclusive, `unknown` at height 31 classifies `After` and sheds `node.len() == 2`; mutated it sheds 1 and `live(after) == before + gained - shed` fails on a's side.

Cross-reference: streaming-tests-26 (the walk-tier pin with k >= 2) and tests-observation-25 (the public conservation property that never sheds) state the same gap at two other tiers; one redaction arm in `sessions_conserve_the_live_count` plus one walk-tier fixture closes all three.

### materialized-37: The `Resolver`'s skip-past supply arm has no injection
- Where: src/tree/mirror/streaming/materialized/work/resolver.rs:81-83 (related: work/resolver.rs:74-79; work/tests/violations.rs:160-183; src/tree/mirror/streaming/testing/faulting.rs:222-226)
- Class / severity / confidence: verification-gap / low / high
- Provenance: assessed (read; the `InvalidSupply` script at violations.rs:169-183 sends two supplies at one radix against an empty fan, which trips `radix <= *last` at 74; the `Faulting` harness pushes `Supply(0)` twice, faulting.rs:224-225, also line 74; nothing reaches 81-83)
- Seen by: correctness; refutation: confirmed, with one correction (`>` to `>=` at 81 is an equivalent mutant because the `==` case is consumed at 78, so arm deletion is the catchable mutation); history: no rationale found
- Owner-gated: no

A supply whose radix skips past a held-but-unmatched child is a third structural arm no test reaches; deleting it survives the suite (the supply is absorbed and `finish()` reports `UnfinishedReply` instead). Adequacy: keep known-bad artifacts committed and failing.

Evidence:

    81	                    Some((next, _)) if radix > *next => {
    82	                        return violation(Violation::InvalidSupply);
    83	                    }

Resolution: Add an `Injection::InvalidSupplySkipsHeld` script: `ours = {r}` (any held radix with `r < 255`), reply `[Supply(r + 1, supplied)]`, expected `InvalidSupply`; run it through the same 32-height dispatch. Acceptance: a committed injection fails when the arm at 81-83 is removed.

Construction: `ours = [(5, node)]`, reply `[Supply(7, node)]`: `resolved.last()` is `None`, `fan.peek()` is `Some(5)`, `7 != 5`, `7 > 5` -> `InvalidSupply` on HEAD; with the arm deleted the supply is absorbed and `finish()` reports `UnfinishedReply`.

### materialized-22: `assert_parent_early` is fed only a hand-written literal, so the pin its testdoc promises cannot fire
- Where: src/tree/mirror/streaming/materialized/progress/tests.rs:96-117 (related: src/tree/mirror/streaming/materialized/progress.rs:195-263; src/tree/mirror/streaming/tests.rs:83-95; formal/lean/StreamingMirror/Statement.lean:21)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (grep `assert_parent_early` across src/ and tests/: the definition at progress.rs:214 and one call at progress/tests.rs:116, fed the four-event literal at 110-115)
- Seen by: structure; refutation: confirmed; history: deliberate and holds for the retention (4407590b: "The parent-early (d5) probe stays as the design-space record ... still deliberately unwired", restated at progress.rs:195-212, and the Lean artifact keeps `Sched.deadlock_free_d5`); what no source supports is the testdoc's claim that the test moves when the walk's order changes
- Owner-gated: yes: disposition of a deliberately retained design-space record

The testdoc says the trace "is the encoder's own order" and "if this test starts failing because the panic disappears, the encoder's order changed corners". The input is a literal, not a captured trace, so a change to the walk cannot move this test; only editing the literal can. An inaccurate testdoc is a bug in the test. The checker (about 50 lines plus a 20-line doc) has no other caller and its stated purpose is to document a rejected alternative, which the formal model already records.

Evidence:

    98	/// This trace is the encoder's own order (the same trace
    99	/// `accepts_wire_resolution_work_parent_order` accepts): the sole disputed
    100	/// child's dependent work departs after the final resolution and before the
    101	/// parent summary, exactly what the weave's d5 placement forbids. Pinned as
    102	/// the design-space record (finding #7, adjudicated: the encoder keeps the
    103	/// epilogue placement and the `d6`/`assert_parent_last` check instead): if
    104	/// this test starts failing because the panic disappears, the encoder's
    105	/// order changed corners — re-audit the parent-placement trade before
    106	/// accepting.

Resolution: Either make the pin real (capture a session with `with_trace` as `streaming/tests.rs:83-95` does and run it through `assert_parent_early` under `#[should_panic]`), or dissolve `assert_parent_early` and this test, leaving the d5/d6 record to the model, and reword `assert_parent_last`'s doc (progress.rs:127-128) to stop pointing at the removed checker. Keeping a checker purely as a record is the owner's call; at minimum the testdoc must stop claiming a sensitivity it lacks. Acceptance: either a real-session trace reaches `assert_parent_early`, or the function and test are gone and no prose references them.

### materialized-39: The exhaustive violation suite enters at internal walk entries without stating the decision, while the public-wiring injector covers two variants and cannot reach the terminal phase
- Where: src/tree/mirror/streaming/materialized/work/tests/violations.rs:246-297 (related: work/tests/violations.rs:1; src/tree/mirror/streaming/tests/faults.rs:41-46, 64-101; src/tree/mirror/streaming/testing/faulting.rs:193-241)
- Class / severity / confidence: test-quality / low / medium
- Provenance: verified for the entries and strategies (read: the three `InjectHeight` impls call `Work::leaf_level`, `leaf_parent_level`, `internal_level`; the module doc at line 1 records no decision; `arb_connected_violation` samples `UnexpectedQuery` and `UncontainedSupply` while `Faulting` scripts all eight reply-shaped violations; `server_steps`/`client_steps` draw from `0..=15`); assessed for the phase-count arithmetic (a depth-32 comb session has 17 reply phases per side, so step 15 never reaches the terminal reply `absorb` consumes)
- Seen by: correctness; refutation: confirmed, with the structural corroboration; history: contradicts doctrine, not an AGENTS.md hard rule (83db6b26 built the suite at `Work::*_level` because a per-height, per-variant matrix needs a scripted counterparty, a defensible reason that is not stated)
- Owner-gated: no

Doctrine: exhaustive suites exercise the public surface; internal-entry checks only as deliberate, documented decisions at the check site, because a suite locked to an internal entry lets coverage drift when the public wiring (`Descending::reply`, `complete_responder`, `complete_initiator`) changes. The reason here is good and unstated. Separately, the connected suite that does cross the public wiring carries two variants and a step range that structurally misses the opening and terminal legs, which is the mechanical reason materialized-14 is unpinned.

Evidence:

    1	//! Semantic-violation injection across every materialized walk height.

    286	        let mut work = Work::new(Local, Window::FLOOR, Recorder::default());
    287	        let (responses, _asked, _upper, _lower) = work.internal_level::<H>(

    41	fn arb_connected_violation() -> impl Strategy<Value = Violation> {
    42	    prop_oneof![
    43	        Just(Violation::UnexpectedQuery),
    44	        Just(Violation::UncontainedSupply),
    45	    ]
    46	}

Resolution: State the decision in the violations.rs module doc (the per-height, per-variant matrix needs a scripted counterparty the connected driver cannot express cheaply); widen `arb_connected_violation` to every variant `Faulting` can script, and widen the step range (or add a terminal-phase case) so the public wiring carries the taxonomy at least once per variant and per leg. Acceptance: the module doc names why it enters at `Work::*_level`; `connected_violation_aborts_without_mutating_root` covers every `Violation` variant the harness can construct, including at the opening and terminal phases.

## Streaming protocol tests (the in-process suite under src/tree/mirror/streaming/tests/)

The suite's oracles are independent and derived (join for content, prefix closure for disputes, the Lean literal for the wedge). Its gaps are strategy reach (the deep-spine generator omitted from the differential, depth never combined with redaction, the terminal fault phase excluded on both sides), a stall probe whose boolean collapses violation into completion (streaming-tests-11, correctness, demonstrated), and a harness that grew by accretion (a family-shaped test hand-driving `TestRunner`, rosters sampled rather than enumerated, tautological atomicity assertions, and seed comments naming values no strategy generates).

### streaming-tests-19: The fault-injection step range stops one phase short of the terminal reply on both sides
- Where: src/tree/mirror/streaming/tests/faults.rs:66-67 (related: src/tree/mirror/streaming/driver.rs:161-171; src/tree/mirror/streaming/testing/faulting.rs:164-168, 248-270, 288-299, 403-436; src/tree/mirror/streaming/protocol.rs:148-151; proptest-regressions/tree/mirror/streaming/tests/faults.txt:8)
- Class / severity / confidence: verification-gap / medium / medium
- Provenance: verified (read the driver schedule and every `Faulting` phase impl; `git log -S<hash>` places seed line 8 (`server_steps = 16`) in ddc9a2d8, and `git show ddc9a2d8:...faults.rs` shows `server_steps in 0usize..=15` in the same commit)
- Seen by: blind-spots ([25]); refutation: confirmed; history: no rationale found, and the evidence strengthens the finding: `complete_responder` already injected at that commit, and a seed pinned at exactly the excluded step means proptest shrank toward 16 and could go no lower, so steps <= 15 passed and 16 failed. Inference (the history pass's, labeled as such): the range was narrowed around the failure rather than the failure diagnosed
- Owner-gated: no

The connected schedule gives each side seventeen outgoing phases: the opening (step 0), fifteen loop replies (1..=15), then the initiator's sixteenth reply at height `Z` (protocol.rs:148-151) and the responder's `complete_responder` (16). `fault_phase` decrements `remaining` on every non-injecting phase, `connect` does not, and `complete_responder` injects at `remaining == 0` (faulting.rs:416), so step 16 is exactly the leaf-height terminal phase on either side, where leaf supplies and leaf requests pair. The strategy draws `0usize..=15`, so that phase is never corrupted through the connected driver, and the one committed seed that names step 16 records a failure there.

Evidence:

        66	        server_steps in 0usize..=15,
        67	        client_steps in 0usize..=15,

    faults.txt:
         8	cc f7515199a92a2d04d20b953c417021f4771ca93cc0ec4603cf51d6da120fdb8b # shrinks to server_steps = 16, client_steps = 0

Resolution: Widen both ranges to `0usize..=16` (or enumerate them per streaming-tests-17) and run. If the terminal phase turns out unreachable for the comb fixture, the `Ok(_)` arm already fails the test and that is the finding; if the fault is not surfaced as the expected violation, that is a `Faulting` or driver finding. Either way the disposition is recorded, not the range narrowed. Acceptance: step 16 is drawn or enumerated for both sides and the test passes, or the failure is filed.

Construction: Set `server_steps = 16` and `violation = Violation::UnexpectedQuery` against `full_depth_comb_pair(2, LeafOrder::Interleaved)`; the fault lands in `complete_responder`'s reply stream and must surface as `MirrorError::Client(MaterializedError::Violation(Violation::UnexpectedQuery))`.

Cross-reference: materialized-39 records the same structural miss from the walk side (a depth-32 comb session has 17 reply phases per side, so step 15 never reaches the terminal reply `absorb` consumes); streaming-tests-17 proposes enumerating the range around a named `REPLY_PHASES` constant.

### streaming-tests-26: The whole-subtree shed count is never pinned above one
- Where: src/tree/mirror/streaming/tests/stats.rs:195-218 (related: src/tree/mirror/streaming/materialized/unknown.rs:104, 150; src/tree/mirror/streaming/stats.rs:92-98; tests/session_stats.rs:109, 302-335)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (grep of every `messages_shed` assertion in src/ and tests/: `== 0`, `== 1`, or the conservation identity, at stats.rs:185, 186, 208, 210, 258, 259 and session_stats.rs:64, 66, 109, 111, 324, 328; grep of every `shed(` call: `unknown.rs:94` sheds 1 for a leaf, `:104` and `:150` shed `node.len() as u64` for a wholly-known internal node; read `sessions_conserve_the_live_count` (session_stats.rs:302-335): its strategy draws `send_all` only, never a redaction)
- Seen by: blind-spots ([22]); refutation: confirmed (adds that the gate's `mutants-list` leg is list-only, so a surviving mutant here is not gated either); history: no rationale found (487e17ea and 7626543e describe where sheds are counted and pin "one shed"; the open investigation brief `.agent-notes/2026-08-21-unknown-pruning-survivor/README.md:92-94` proposes exactly this instrument)
- Owner-gated: no

`SessionStats::messages_shed` documents that a subtree pruned without descending "adds its exact live-leaf count", and `unknown.rs` implements that with `stats.shed(node.len() as u64)` at two sites. Every committed pin asserts exactly 1 or 0, and the only conservation proptest never redacts, so the `len`-crediting arm is exercised only with `len == 1` and its multi-leaf semantics is unobserved. A mutant `node.len() as u64` -> `1` at either site survives both the walk-tier and the public suites (Principle 6: a criterion the wrong implementation also passes is decoration).

Evidence:

       208	    assert_eq!(a_stats.messages_shed, 1);
       209	    assert_eq!(a_stats.messages_gained, 1);
       210	    assert_eq!(b_stats.messages_shed, 0);
       211	    assert_eq!(b_stats.messages_gained, 0);

    unknown.rs:
       103	            Dominance::After => {
       104	                stats.shed(node.len() as u64);
       105	                return Ok(None);
       106	            }

Resolution: Add a walk-tier pin with `grown` and `act`: a shared base of k >= 2 leaves under one controlled child (paths `[0x30, i, 0..]`) plus one shared leaf elsewhere so the root stays disputed; side b is the base plus one extra on party 1; side a is the base with the k leaves forgotten on party 2 (a lacks child 0x30 and its ceiling dominates the k versions). Assert `b_stats.messages_shed == k` and `live(&theirs) == b_before - k + b_stats.messages_gained`. Add a redaction step to `sessions_conserve_the_live_count`'s strategy (a drawn subset of the shared sends) so the law is checked with shed > 1 over the wire. Acceptance: a committed test asserts `messages_shed >= 2` from one whole-subtree prune; the public conservation proptest draws redactions; the `node.len()` -> `1` mutant at unknown.rs:104 and :150 is caught.

Construction: Build the pair as described with `fixtures::grown` and `traverse::act(node, [(path, version, Action::Forget)], ..)` and run `mirror_with_stats(a, b)`: today's code reports `messages_shed == k` on b; replacing `node.len() as u64` with `1` at unknown.rs:104 makes the new assertion fail while every existing test still passes.

Cross-reference: materialized-28 and tests-observation-25.

### streaming-tests-6: `arb_oracle_pair` omits the deep-spine generator built to reach divergence below the root
- Where: src/tree/mirror/streaming/tests.rs:168-174 (related: src/tree/arb.rs:232-291; src/tree/tests.rs:1201; capacity.rs:356-385)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (grep: `arb_deep_divergent_pair` is used only at its definition, arb.rs:244, and at src/tree/tests.rs:1201)
- Seen by: blind-spots ([24]); refutation: reframed (the original claim that depth reaches the streaming walk only through hand fixtures is false: `scheduled_structured_disputes_match_oracle` draws pyramids to depth 6 and boundary fans to depth 31 against `join_oracle`; what the deep-spine family adds is its zero-width, subset, identical, and ceiling-only merges at drawn depth); history: no rationale found (the strategy dates from 83edcd94; the deep generator arrived later in 48d5253d for the join suite; 69c7ac80 retargeted the differential to `Tree::join` without touching the strategy)
- Owner-gated: no

The central differential samples content-addressed pairs whose keys scatter at the root fan (arb.rs:235-237: "Content-addressed generators cannot produce this shape"). `arb_deep_divergent_pair` draws a shared spine of depth 0..32 with novelty widths that include zero, so subset, identical, and ceiling-only merges at depth are sampled; the join suite uses it and the streaming differential does not. Adding the arm costs nothing and widens the walk's oracle coverage to the family the generator exists for.

Evidence:

       168	fn arb_oracle_pair() -> impl Strategy<Value = (Root, Root)> {
       169	    prop_oneof![
       170	        4 => arb_divergent_pair(),
       171	        2 => (arb_tree_root(0, 0..=8), arb_tree_root(1, 0..=8)),
       172	        1 => arb_tree_root(0, 0..=8).prop_map(|root| (root.clone(), root)),
       173	    ]
       174	}

Resolution: Add `2 => arb_deep_divergent_pair()` (and optionally `arb_wide_divergent_pair()`) as arms and name the deep-spine family in the doc at 163-167; commit any seed that appears. Acceptance: the arm is present and `streaming_matches_join_oracle` passes.

Construction: none needed beyond the strategy edit; a failure, if any, writes to `proptest-regressions/tree/mirror/streaming/tests.txt`.

### streaming-tests-7: Redaction at depth is pinned by one two-leaf fixture; no generated family combines depth with redaction
- Where: src/tree/mirror/streaming/tests.rs:194-196 (related: src/tree/arb.rs:132-172, 244-291; stats.rs:198-218; src/tree/mirror/streaming/backend.rs:127-129)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read `arb_divergent_pair` at arb.rs:132-172: forgets are drawn over content-addressed shared keys; `arb_deep_divergent_pair` at 244-291 draws no forgets)
- Seen by: blind-spots ([23]); refutation: reframed (redaction under a depth-1 shared prefix is reached by hash collision, roughly 1/256 per shared pair, so "never" is overstated; the pruned-to-nothing reply the backend doc names is exercised at the root fan; the deletion filter has its own differential in `unknown/tests.rs`); history: no rationale found
- Owner-gated: no

The only deterministic redaction below the root in the streaming suite is `leaf_parent_redaction_pair` (one leaf forgotten, one concurrent sibling), and the only generated redaction is `arb_divergent_pair`'s, whose forgets land at content-addressed keys. No strategy draws a shared spine of chosen depth together with a subset of forgets, so a redaction that empties a whole disputed child below the root, or two sides redacting different siblings under one leaf parent, is unsampled. The join oracle is available and cheap, so the family is stateable as a property.

Evidence:

       194	#[test]
       195	fn honors_redaction_under_leaf_parent_dispute() {
       196	    let (a, b, expected) = leaf_parent_redaction_pair();

Resolution: Add `arb_deep_redaction_pair` beside `arb_deep_divergent_pair` in `arb.rs` (a shared spine of drawn depth carrying k drawn shared leaves; each side forgets a drawn subset on its own party and adds drawn concurrent extras), include it as an arm of `arb_oracle_pair`, and add two fixtures: (a) both sides hold prefix P, side a holds child Q under P with k >= 2 leaves that side b forgot (b's request for Q prunes to nothing on a); (b) each side forgets the other's sibling under one 31-byte prefix. Acceptance: `arb_oracle_pair` draws forgets under a shared prefix of depth >= 1; the two fixtures are committed with `join_oracle` as the expectation in both orientations.

Construction: For (a): `grown(None, 0, 1, &(), &[spine, q1, q2])` with `spine = path_at(&[0; 32])`, `q1 = path_at(&[0, 0, 1, 0x10])`, `q2 = path_at(&[0, 0, 1, 0x11])`; side a is that node; side b is the same node with `q1` and `q2` removed via `act(node, [(q1, v1, Action::Forget), (q2, v2, Action::Forget)], ..)` ticked on party 1, plus one concurrent extra under `[0, 0, 2]` on party 1. Expected is `join_oracle(a, b)` in both orientations.

### streaming-tests-28: The Lean wedge literal is transcribed by hand with no mechanical comparison
- Where: src/tree/mirror/streaming/tests/wedge.rs:38-40 (related: wedge.rs:41-65; formal/lean/StreamingMirror/Mux/Instances.lean:56-67; justfile:287-292)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read `Instances.lean:56-67` against `wedge.rs:48-64` scope for scope: they match today, including `fan := 7` and `capLevel := 1` against `LEAN_WEDGE_FAN` and `LEAN_WEDGE_CAP_LEVEL`; read justfile:289-292: `citecheck` resolves citation names against the `crates/before` test inventory only, and no gate leg reads Instances.lean)
- Seen by: structure-prose (open question), blind-spots ([33]); refutation: confirmed; history: deliberate-and-holds (the manual discipline is stated in code and dates from 97dfbcdf; nothing records why hand-checking suffices)
- Owner-gated: yes: adding a gate leg is gate policy; declaring the manual discipline acceptable is a documented design decision

`lean_wedge_literal()` mirrors `Mux.wedge` and the doc asks the human to keep them in sync. The Rust pin (`wedge(6) == lean_wedge_literal()`) checks the generator against the transcription, not the transcription against the Lean, so a change to `Instances.lean`'s literal (the shape `wc_impossibility` quantifies over) would leave the bridge asserting the wrong shape with the gate green. Doctrine: any quantity computable two ways gets a committed test comparing them.

Evidence:

        38	/// The Lean wedge literal, transcribed scope-for-scope from the Lean
        39	/// definition `Mux.wedge` — the Lean definition is the source of truth;
        40	/// if the literal changes there, change this.

Resolution: Owner choice: (a) a small `tools/` check that extracts `def wedge` from `Instances.lean` and compares it to a serialized `lean_wedge_literal()` (the `muxprobe-expected.tsv` precedent suggests a Lean-emitted expectation file a Rust test could read), wired into the gate; or (b) record at the site that the transcription is human-checked and why that is acceptable. Acceptance: a gate leg fails when the two diverge, or the doc states the accepted manual discipline.

### streaming-tests-18: The 'input roots remain untouched' assertions compare a value with its own aliased clone
- Where: src/tree/mirror/streaming/tests/faults.rs:61-100 (related: faults.rs:1, 103-122, 188; src/tree.rs:108-135; src/tree/typed/untyped.rs:21-23, 684-695; src/tree/mirror/streaming/backend/local.rs:232-237; src/tests.rs:550)
- Class / severity / confidence: test-quality / medium / high
- Provenance: assessed (read: `tree::Root` has private `ceiling` and `root` fields and `PartialEq` compares them (tree.rs:108-135); `Node` is `Arc<NodeInner>` and `Node::eq` is `ptr_eq || hash` (untyped.rs:21-23, 694); the sessions receive `StreamingRoot::from(client_root.clone())`, and `From<tree::Root> for Root<Local>` moves the clone (backend/local.rs:232-237); nothing between the snapshot and the assertion holds a reference to the originals)
- Seen by: structure-prose ([4]), blind-spots ([27] item 1), api-economics ([36]); refutation: confirmed and raised faults.rs:1 ("lifecycle atomicity") as the same claim at module level; history: no rationale found (tautological from birth in ddc9a2d8: the sessions already received clones)
- Owner-gated: no

Both tests snapshot `before = (client_root.clone(), server_root.clone())`, hand further clones to the sessions, and close with `prop_assert_eq!((client_root, server_root), before)`. `Root` is an immutable `Arc` tree with no `&mut` path from a session to the caller's value, so the two tuples alias the same heap nodes and the assertion is decided by the types. The testdocs ("remain untouched", "with both input roots untouched") and the module doc ("lifecycle atomicity") advertise an atomicity property the tests do not test, and the walk tier cannot test it, because a session's only output is its `Ok` value. The commit-tier atomicity claim is pinned elsewhere (`uncontained_supply_fails_gossip_and_poisons_the_link`, src/tests.rs:550). A guard is justified by naming a constructible failure it catches (Principle 3); an inaccurate testdoc is a bug in the test.

Evidence:

         1	//! Connected-session abort routing and lifecycle atomicity.

        61	    /// A genuine malformed reply crosses the fully connected driver as its
        62	    /// detected violation while both materialized input roots remain untouched.

        71	        let before = (client_root.clone(), server_root.clone());

       100	        prop_assert_eq!((client_root, server_root), before);

Resolution: Delete the `before` snapshots and the two closing `prop_assert_eq!`s (71, 100, 122, 188); drop "while both materialized input roots remain untouched" (61-62), "with both input roots untouched" (108-109), and "and lifecycle atomicity" (1); rename the first test to `connected_violation_aborts_with_its_typed_error`. If a walk-tier atomicity statement is wanted, its observable is the session's `Output` on the failing path, not the caller's clone. Acceptance: no assertion in faults.rs compares a pre-session clone of an input root to itself; the docs claim only error routing, classification, and unchanged reconciliation under tolerated lies.

### streaming-tests-23: A family-shaped test hand-drives `TestRunner` (no shrinking, no seed) around a counter that cannot fail
- Where: src/tree/mirror/streaming/tests/local_eq.rs:144-254 (related: local_eq.rs:24-25, 127-143)
- Class / severity / confidence: test-quality / medium / high
- Provenance: assessed (read the loop body: every iteration either panics on one of its five asserts or executes `nondegenerate += 1`; the role probe `remote_plus(&[0])` is computable inside a `proptest!` closure); the history pass verified via `git log -S'nondegenerate += 1'` that the counter has been unconditional since 97dfbcdf
- Seen by: structure-prose ([5]), blind-spots ([28]), api-economics ([43]); refutation: confirmed and added that the `ValueTree` and `TestRunner` imports at 24-25 exist only for this loop; history: no rationale found
- Owner-gated: no

`free_insertions_are_invisible_to_the_local_view` samples 32 `arb_divergence()` cases through `TestRunner::deterministic()` inside a plain `#[test]`, so a failing case neither shrinks nor persists to `local_eq.txt`, and the same 32 specs run forever. `assert_eq!(nondegenerate, CASES)` can never be the failing assertion, and the doc's "counted" and "at 100% frequency" framing describes that vestigial counter rather than a property. AGENTS.md: when the claim is a family, state it as a proptest invariant so the shrunk counterexample rides along as a committed seed.

Evidence:

       145	fn free_insertions_are_invisible_to_the_local_view() {
       146	    const CASES: u32 = 32;
       147	    let mut runner = TestRunner::deterministic();
       148	    let strategy = arb_divergence();
       149	    let mut nondegenerate = 0u32;

       247	        nondegenerate += 1;
       248	    }
       249	
       250	    assert_eq!(
       251	        nondegenerate, CASES,
       252	        "every constructed case realizes a nondegenerate LocalEq pair"
       253	    );

Resolution: Move the body into the file's `proptest!` block as `fn free_insertions_are_invisible_to_the_local_view(spec in arb_divergence())` with `prop_assert!`s (`#![proptest_config(ProptestConfig::with_cases(32))]` if the case count matters), delete `CASES`, `runner`, `nondegenerate`, and the two imports at 24-25, and retitle the doc from "NONDEGENERACY, counted" to the invariant. Acceptance: no `TestRunner` in local_eq.rs; the test is a `proptest!` member; a forced failure writes to `proptest-regressions/tree/mirror/streaming/tests/local_eq.txt`.

### streaming-tests-2: A driver unit test sits in the streaming tests root while driver.rs keeps an inline test module
- Where: src/tree/mirror/streaming/tests.rs:36-60 (related: src/tree/mirror/streaming/driver.rs:203-238 (outside the partition); src/tree/mirror/streaming.rs:197-204)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (read tests.rs:36-60, driver.rs:85-101 and 203-238, streaming.rs:197-204)
- Seen by: structure-prose, api-economics; refutation: confirmed the placement and reframed the api-economics "dissolve it" option (the test pins crate-owned `Client`/`Server` routing, not only `futures`' fail-fast); history: no rationale found (the test and driver.rs's inline module were created together in cbfe1aff)
- Owner-gated: no

`terminal_errors_preempt_parked_peers` exercises `driver::try_join_mapped` alone, with no tree or session; the driver's other two tests live in an inline `#[cfg(test)] mod tests` at driver.rs:203. The three belong together in a `driver/tests.rs` sibling (AGENTS.md's convention); today a reader of driver.rs's tests does not see this one, and the streaming suite opens with a non-session test. The crate-owned claim is that a left error surfaces as `Client` and a right error as `Server`, the routing `descend`'s equal-version short circuit relies on (streaming.rs:198-203); the testdoc names only the fail-fast half, which is `futures::future::try_join`'s contract.

Evidence:

        36	/// Either terminal error preempts a peer which can no longer make progress.
        37	#[test]
        38	fn terminal_errors_preempt_parked_peers() {
        39	    let left = try_join_mapped(
        40	        future::ready(Err::<(), _>("left")),
        41	        MirrorError::<&str, Infallible>::Client,

    driver.rs:
       203	#[cfg(test)]
       204	mod tests {

Resolution: Create `src/tree/mirror/streaming/driver/tests.rs` (`mod tests;` in driver.rs), move this test and the two inline ones there, drop the `try_join_mapped` import from tests.rs, and restate the doc as "A left-side error surfaces as `Client` and a right-side error as `Server`, preempting a parked counterparty." Acceptance: driver.rs has `mod tests;` and no inline module; tests.rs has no `try_join_mapped` reference; the testdoc names the routing.

### streaming-tests-16: A measured stall-boundary table lives in a comment while the test pins one cell
- Where: src/tree/mirror/streaming/tests/capacity.rs:299-312 (related: capacity.rs:244-278)
- Class / severity / confidence: test-quality / low / medium
- Provenance: assessed (read the comment and the two assertions below it)
- Seen by: structure-prose ([11]), api-economics ([42]); refutation: confirmed; history: no rationale found (0ad9077a added the tally and the two P=4 assertions together; the parent-placement note cites the 253/254 and 9/27/81 pins but never the P=6/8/12 cells, so those numbers exist only in this comment)
- Owner-gated: no

The control comment reports four measured boundaries (P=4 at C=1, P=6 at C<=3, P=8 at C<=4, P=12 at C<=6) as confirmation of the per-scope law, but the assertions pin only P=4 at C=1 and C=2; `parent_delay_single_parent_boundary` likewise states "fan <= cap + 2" and pins cap 1 and one point at cap 2. A number in prose is a hypothesis until a committed check moves on it (Principle 8), and a hand-maintained tally rots silently (Principle 5). Each cell costs at most five sessions of a tree under 40 cells.

Evidence:

       299	    // Control: growing the count of fan-3 parents under ONE root scope grows
       300	    // the ROOT's own fan, so the per-scope law (fan ≤ cap + 2) predicts the
       301	    // stall boundary — confirmed empirically (P=4 stalls at C=1, P=6 at C≤3,
       302	    // P=8 at C≤4, P=12 at C≤6: exactly P > C + 2 throughout).
       303	    assert!(
       304	        stalls_under_any_schedule(&parents_of_three(4), 1),

Resolution: Replace the tally with a loop over `[(4, 1), (6, 3), (8, 4), (12, 6)]` asserting `stalls(parents_of_three(p), c)` and `completes(parents_of_three(p), c + 1)` (using the three-way probe from streaming-tests-11), and likewise `for cap in 1..=6` in `parent_delay_single_parent_boundary`; reduce the comment to the law. Acceptance: every (P, C) pair named in the two tests' prose is asserted by the body, or absent.

Cross-reference: the three-way probe this resolution relies on is the fix for streaming-tests-11 (correctness, high), whose collapse of a protocol violation into "did not stall" was demonstrated by construction: under all five probe schedules a server faulting `UnexpectedQuery` in its first reply read as completion (`witness/results.md`).

### streaming-tests-17: Two finite fault rosters are sampled 256 times instead of enumerated, around a bare phase-count literal
- Where: src/tree/mirror/streaming/tests/faults.rs:41-58 (related: faults.rs:63-68, 115-119; src/tree/mirror/streaming/driver.rs:161-171)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (read the strategies and the driver schedule; the refutation pass verified that no `PROPTEST_CASES` override exists in `.config/nextest.toml` or the justfile)
- Seen by: blind-spots ([26]), api-economics ([41]); refutation: reframed (the per-run miss probability under sampling is small, about 2% for one of the 64 (violation, side, step) cells and negligible for the ten lie cells, so the gain is determinism, legibility of the step bound, and session count, not coverage risk); history: no rationale found. Interplay: enumerating orphans seed lines 7, 8, 12, and 15 of faults.txt (see streaming-tests-20)
- Owner-gated: no

`arb_connected_violation` and `arb_greeting_lie` are `prop_oneof![Just(..)]` over 2 and 5 values; `greeting_lies_classify_exactly` draws a 10-point space 256 times (each case one 32-height comb session plus a baseline session on the tolerated branch), and `connected_violation_aborts_without_mutating_root` draws 32 points per direction 256 times, two sessions per case. Nothing is shrinkable or unbounded; exhaustive loops cover every point deterministically in roughly 74 sessions instead of about 1000. The step bound `0usize..=15` is the driver's reply-phase count (driver.rs:164-168) left as a literal, and it undercounts by one (streaming-tests-19).

Evidence:

        41	fn arb_connected_violation() -> impl Strategy<Value = Violation> {
        42	    prop_oneof![
        43	        Just(Violation::UnexpectedQuery),
        44	        Just(Violation::UncontainedSupply),
        45	    ]
        46	}

        66	        server_steps in 0usize..=15,
        67	        client_steps in 0usize..=15,

Resolution: Give `Violation` (or a test-local const) and `GreetingLie` an `ALL` array; rewrite both tests as plain `#[test]`s with nested loops over `ALL x [true, false]` and `ALL x 0..=REPLY_PHASES x side`, where `REPLY_PHASES` is a named constant tied to the driver's schedule; keep `materialized_backend_failures_are_fail_fast` as a proptest. Acceptance: both tests are deterministic enumerations; faults.rs has one `proptest!` block; the orphaned seed lines have an owner ruling.

### streaming-tests-20: Two committed seed comments name values the current strategies cannot generate
- Where: proptest-regressions/tree/mirror/streaming/tests/faults.txt:7-8 (related: faults.txt:11-15; proptest-regressions/tree/mirror/streaming/tests/stats.txt:7; faults.rs:41-46, 66-67; src/tree/mirror/streaming/testing/faulting.rs:240)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (cat of both seed files; grep of `Just(Violation` in faults.rs shows only `UnexpectedQuery` and `UncontainedSupply`; `git log -S` for both hashes returns ddc9a2d8; `git show ddc9a2d8:...faults.rs` has `let violation = Violation::UnexpectedQuery;` with `0usize..=15` and no `violation` parameter, so both lines were orphaned at birth)
- Seen by: blind-spots ([30]); refutation: confirmed; history: no rationale found for lines 7-8; the "minted during mutation review" annotations (lines 11-15, stats.txt:7) are deliberate and recorded as a practice across three commits (7626543e, f099dcb1, 50c8b0a3)
- Owner-gated: yes: removing or rewording seed lines is governed by the AGENTS.md rule to commit every seed and never strip one

Line 7 names `violation = UnaskedReply`, which `arb_connected_violation` cannot draw and `Faulting` cannot synthesize (`Violation::UnaskedReply | Violation::UnansweredQuery => unreachable!()`, faulting.rs:240); line 8 names `server_steps = 16`, outside `0..=15`, in a two-parameter shape no current test has. Proptest replays every seed in the file for every test in the file, so the lines stay live, but each now reproduces a case other than the one its comment describes, and the regression each was minted for is unreachable by the strategy. A seed is an instrument of record whose comment tells a reader what it reproduces; a stale comment turns "verified" into "told" (Principle 8). The history pass could not locate the test that produced line 8.

Evidence:

         7	cc ea434720ac87c1b9710680e38f956817be9512b9dca9d1a30aaf3d98fb5667fe # shrinks to violation = UnaskedReply, server_steps = 0, client_steps = 0
         8	cc f7515199a92a2d04d20b953c417021f4771ca93cc0ec4603cf51d6da120fdb8b # shrinks to server_steps = 16, client_steps = 0

Resolution: Owner call: (a) keep the lines and correct the comments to what the seeds now generate, or (b) remove the two lines in a commit whose message names the orphaning. The "minted ... during mutation review" notes state what the seed pins and are recorded precedent; leave them unless the owner wants the campaign reference dropped. Acceptance: every `# shrinks to` comment in the file names a value the file's current strategies can generate.

## Remote codec (budgets, decode, encode, frames, greeting, signals)

The codec's invariants are each enforced in one place and pinned, but the async reader's transport-failure classification has no per-part witness against the sync oracle (the region two correctness defects live in), one boundary is tested from the rejecting side only, the greeting round-trip is three fixtures, and three testdocs overclaim.

### remote-codec-15: The async reader's transport-failure classification has no per-part witness against the oracle
- Where: src/tree/mirror/streaming/remote/codec/decode/tests.rs:774-780 (related: src/tree/mirror/streaming/remote/codec/tests/error_atlas.rs:229-263 and :719-747; src/tree/mirror/streaming/remote/codec/tests.rs:543-548; src/tree/mirror/streaming/remote/codec/decode/async_io.rs:201-213)
- Class / severity / confidence: verification-gap / low / high
- Provenance: assessed (read: `FailingReader` and `FailAfterReader` implement `std::io::Read`; the codec's only `AsyncRead` fixture, `CountingRead`, never fails)
- Seen by: correctness (35); refutation: reframed (session-level fault suites `AdversarialRead` and `Cut` do drive read errors through live sessions and match `CodecDecodeErrorKind::Read`; what is missing is the codec-level per-part comparison); history: the atlas's `Read` witnesses were built on the sync oracle when both readers shared a per-head read shape, and 18527932 changed the async shape without revisiting them
- Owner-gated: no

Every `DecodeErrorKind::Read` witness in the codec suites and in the error atlas drives the sync oracle through a `std::io::Read` fixture; no `AsyncRead` fixture ever fails, so `FrameRead`'s `Read` arms (`Arrived::short` with a failure, `classify` on a non-EOF error, the opener path) are exercised at the codec level only by success and EOF. The session-level suites confirm that a failing transport surfaces as some `Read`, not that it surfaces at the same part the oracle names; remote-codec-11 lives entirely in this unexamined region. I keep it at low rather than nit because it is the blind spot that let two correctness defects through.

Evidence:

    774	struct FailingReader;
    775	
    776	impl std::io::Read for FailingReader {
    777	    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
    778	        Err(std::io::ErrorKind::Other.into())
    779	    }
    780	}

    error_atlas.rs:735	impl std::io::Read for FailAfterReader {

Resolution: Add a `FailAfterAsyncReader` mirroring `FailAfterReader` (serve `remaining` bytes, then `Err(Other)`, then optionally EOF) and a `decode_both`-style helper taking a reader factory, so the sync `FailAfterReader` and its async twin are compared at the offsets `error_atlas::decode_errors` already uses (0, 1, 3, 4 for a query; 3 and 6 for a supply); record the async witnesses in the atlas beside the sync ones. Acceptance: each frame part has an async `Read(part=...)` witness in the atlas, and the classification equals the sync oracle's at every offset for a sticky failing reader; the non-sticky shape from remote-codec-11 is included.

Construction: `struct FailAfterAsyncReader { bytes: Vec<u8>, remaining: usize, then_eof: bool }` implementing `AsyncRead`: serve `min(remaining, buf.remaining())` bytes while `remaining > 0`, then `Poll::Ready(Err(Other))` once, then EOF if `then_eof`. Drive `FrameRead::frame` and compare `kind_signature` against `decode(speaker, budget, &mut FailAfterReader::new(bytes, remaining))` for each offset.

Cross-reference: remote-codec-11 (correctness) and remote-codec-14 (correctness, demonstrated: the over-budget lone-record read consumed the following frame's three bytes) are defects in exactly the region this blind spot leaves unexamined.

### remote-codec-16: The lone-record acceptance boundary at exactly `RECORD_TAG_LEN + 1` is tested only from the rejecting side
- Where: src/tree/mirror/streaming/remote/codec/decode/tests.rs:1006-1007 (related: src/tree/mirror/streaming/remote/codec/decode/async_io.rs:449; src/tree/mirror/streaming/remote/codec/decode.rs:155; decode/tests.rs:373-401)
- Class / severity / confidence: verification-gap / low / high
- Provenance: assessed (read; `RECORD_TAG_LEN` is `head_len(63) == 2`, so the sweep is `0..3` and the gate is `len < 3`; the only zero-content-record test runs at the default budget on the `read_payload` path)
- Seen by: correctness (39); refutation: confirmed (`.cargo/mutants.toml` excludes nothing in rumors, so a `<=` mutant would be a reported survivor by reading, not run); history: ed0f1775 introduced the guard and the rejecting-side sweep together
- Owner-gated: no

The over-budget gate rejects declared bodies shorter than `RECORD_TAG_LEN + 1` on the length alone; the corner test sweeps `0..3` and asserts rejection, but no test presents a 3-byte body (the zero-content record `[d8 3f 40]`) over budget and asserts acceptance. A mutant loosening `<` to `<=` at either decoder survives. Off-by-one at a boundary is judged from both sides; a check tested only on its rejecting side cannot tell `<` from `<=`.

Evidence:

    1006	        // Declared bodies too short for a record's heads, none delivered.
    1007	        for declared in 0..RECORD_TAG_LEN + 1 {

    async_io.rs:449	            if len < super::super::frame::RECORD_TAG_LEN + 1 {

Resolution: Extend `overbatched_corners_classify_exactly` with `supply(stream, Flow::End, &raw_record(&[]))` under the zero budget, asserting `decode_both` yields `Ok` with a one-record run (the same run `a_zero_length_record_is_structurally_valid` accepts within budget). Acceptance: the new case passes at HEAD and fails when the comparison is loosened to `<=` in either decoder.

Construction: Bytes: opener `[0x83, 0x09, 0x07]`, then `[0xd8, 0x3f, 0x43]`, then body `[0xd8, 0x3f, 0x40]`. Under `RunBudget::from_bytes(0)`: `len` 3 is not below 3; `record_prefix` yields a 3-byte prefix with content 0; `lone_record_spans(3, 0)` is `2 + 1 + 0 == 3`; `resume_payload` returns the prefix untouched (`3 < 3` is false); `from_encoded` accepts one zero-content record. With `<=`, `overbatched()` fires instead.

### remote-codec-17: Three testdocs state invariants their bodies do not check
- Where: src/tree/mirror/streaming/remote/codec/decode/tests.rs:1061-1070 (related: decode/tests.rs:1092-1095; src/tree/mirror/streaming/remote/codec/greeting/tests.rs:52-80; src/tree/mirror/streaming/remote/codec/frame/tests.rs:27-37 and :71-74; src/tree/mirror/streaming/remote/codec.rs:47-50)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (read: each doc compared against its body)
- Seen by: prose (21, 22, 23); refutation: confirmed all three; history: the "at ingress" wording transcribes the depth-limit spec's loose use of the word (REVIEW.md step 7, landed by 4356e197); the greeting overclaim survived 574cc760's removal of the magic-key case; the regime overclaim is original to f2b74a97
- Owner-gated: no

AGENTS.md holds every testdoc to accuracy; an inaccurate one is a bug in the test. Three here overclaim. `an_over_deep_supplied_payload_dies_typed_at_ingress` says the failure is "at wire ingress", but the body accepts the run at `LeafRun::from_encoded` and takes the error from `run.records(codec).next()`, the deferred record decode; the module's own vocabulary (codec.rs:47-50) separates exactly these two stages and the test's inline comment says "at the record iterator". `greeting_key_roster_is_exact` promises rejection of "a missing or out-of-order key, or trailing bytes" but constructs a renamed key and trailing bytes only; a dropped key (the count branch at greeting.rs:123-127) and a swapped pair are untested. `record_len_matches_an_actual_push` opens with "at every CBOR byte-string head width a version can occupy", then concedes the chain lengths land in the first two regimes and asserts only `>= 2`.

Evidence:

    1061	/// A hand-crafted record whose payload nests one scope past the peer's
    1062	/// depth limit dies typed at wire ingress, while the same shape at
    1063	/// exactly the limit decodes clean, pinning the boundary.
    ...
    1092	    // One scope past the limit: typed rejection at the record iterator.
    1093	    let over = record_with_payload(&deep_payload(limit.get() as usize + 1));
    1094	    let run = LeafRun::from_encoded(raw_record(&over)).unwrap();
    1095	    let error = run.records(codec).next().unwrap().unwrap_err();

    greeting/tests.rs:52	/// The greeting's map admits exactly one spelling: a missing or
    greeting/tests.rs:53	/// out-of-order key, or trailing bytes, are each rejected — one
    greeting/tests.rs:67	    wrong_key[at] = b'x';
    frame/tests.rs:27	/// `record_len` prices exactly what `push` writes, at every CBOR
    frame/tests.rs:28	/// byte-string head width a version can occupy.
    frame/tests.rs:72	        checked_regimes.len() >= 2,

Resolution: Rename the depth test to `..._fails_typed_at_the_record_iterator` and restate its first sentence ("fails typed at the record iterator, run structure having already passed ingress"). In the greeting test either construct the two named cases (splice the `set_len` entry out and decrement the map head from `0xa6` to `0xa5`, expecting `Shape("greeting is not a map of one entry per roster key")`; swap the `listing` and `set_len` entries, expecting the roster `Shape`) or restate the doc to "a renamed key or trailing bytes". In the record-length test either write "at the one- and two-byte head widths" or extend the sweep until `head_len` reaches 3 and assert `== 3`. Acceptance: each doc's stated rejections and widths correspond to constructed inputs asserted in the body; no testdoc in the partition places a `DecodeLeafError` at ingress.

Construction (greeting case): build the canonical map via `greeting_map(&sample(Vec::new()))`, locate the `set_len` text head and the following uint, remove those bytes, decrement the map head byte, and assert `parse_greeting` returns `GreetingError::Shape(_)`; the current test passes unchanged whether or not the count branch exists.

### Nits

The full record of each (provenance, history, evidence) is in the evidence file named in the last column.

| Id | Where | Claim | Resolution | Record |
|---|---|---|---|---|
| remote-codec-26 | `src/tree/mirror/streaming/remote/codec/greeting.rs:35-44` | `KEYS`' deterministic order is asserted by hand and the greeting round-trip is three fixtures | a test that `KEYS` ascends under `(head_len, bytes)`; a round-trip proptest over versions, listings, and `u64` fields | `evidence/partitions/remote-codec.md` |
| remote-codec-6 | `src/tree/mirror/streaming/remote/codec/budget/tests.rs:7-14` | `RunBudget::default() == from_bytes(DEFAULT)` cannot fail, since `default` is defined as that call | assert `.bytes() == DEFAULT_TARGET_MESSAGE_SIZE` | `evidence/partitions/remote-codec.md` |

## Remote capture renderer, the codec test suite, and the error atlas

The renderer's injectivity claim, which licenses the snapshot discipline, is sampled by no test (demonstrated to be false through the container-key elision); the atlas leaves one enum to a convention and one exemption untied to its variant; three testdocs claim more than their bodies check.

### remote-capture-atlas-17: The renderer's injectivity claim has no committed test; every fallback is pinned by a point example
- Where: src/tree/mirror/streaming/remote/codec/capture/tests.rs:3-10 (related: src/tree/mirror/streaming/remote/codec/capture.rs:12-30, tests/wire_legibility.rs:1-17)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: assessed (capture/tests.rs read in full: no `proptest!` block; tests/wire_legibility.rs is a proptest over `ciborium::Value` parseability, not rendering distinctness)
- Seen by: correctness; refutation: confirmed; history: already known and reopened (REVIEW.md item 12 recorded "No action beyond B1", a decision not to add a test; the scope-A mutants note independently records that "the renderer is total over CBOR, while everything that exercises it speaks only the protocol's dialect" and proposes generative families, still open; finding 13 is the new evidence that reopens item 12)
- Owner-gated: yes (reopens a recorded ruling; the choice of form, inverse parser or leaf mutation, decides how much of the rendering grammar the test depends on)

The suite's three commitments are localization, explicit fallback for named bad inputs, and totality. The property the module doc rests the snapshot discipline on, that distinct wire bytes render distinctly, is sampled nowhere: each fallback test hand-builds one bad input and checks for the `!!` line. A generative test over canonical CBOR items would have found the `…` elision (finding 13) and will find the next hole (a scalar spelling that collides, a tag arm that drops content). Principle: every criterion needs a committed demonstration that a known-bad mechanism fails it; a property stated in prose and never sampled is decoration, and "injective on all canonical CBOR" is a family, so it belongs in a proptest.

Evidence:

    3	//! Three commitments: the rendered value tree localizes the semantic
    4	//! field a snapshot re-accept moved to exactly one rendered line
    5	//! carrying the exact value (its surrounding vocabulary is the wire
    6	//! snapshots' to pin), bytes the walk cannot vouch for render as
    7	//! an explicit failure above their exact hex (never as a silently pretty
    8	//! tree, never as silent omission), and the totality witness

Resolution: Add a proptest with a `Node`-shaped strategy (bounded depth and size; uints, nints, bytes, text, arrays, maps with arbitrary keys, tags including the protocol's named tags and 24/63 over byte strings), encoded through `cbor::write_head`. Either (a) write a small inverse parser from the rendering's grammar back to bytes and assert round-trip, which pins injectivity outright, or (b) the cheaper mutation form: generate an item, mutate exactly one scalar leaf anywhere (map keys included), and `prop_assert_ne!` the two renderings through `render_item`; cover `Naming::Listing` and the `"listing"` key context so `render_listing` is exercised. Acceptance: the proptest is committed with a doc comment stating the injectivity invariant; it fails on the current tree and passes once finding 13 is fixed; any shrunk seed rides along in `proptest-regressions`.

Construction: `fn arb_node(depth: u32) -> impl Strategy<Value = Node>` via `prop_recursive`, with a test-local `fn encode(node: &Node, out: &mut Vec<u8>)` mirroring the renderer's grammar (one `write_head` per major; tag 24/63 content as a byte string of a nested encoding). For form (b): pick a random leaf path, replace the leaf with a different scalar, render both, and assert inequality. Form (a) also catches the float-width and trailing-byte survivors the mutants note lists as injectivity-voiding.

Demonstration (`witness/results.md`): The leaf-mutation proptest (form b) refuted the injectivity claim after five successful cases: `a1a1fb…` and `a1a13b…`, differing inside a map used as a map key, both render `{ … => float32'ae8b2e12' }` through the elision at capture.rs:470 (remote-capture-atlas-13, correctness, is that hole). The scalar-key variant passed 2000 cases, so within that generator's coverage the container-key elision is the only hole. The run wrote a seed under proptest-regressions/tree/mirror/streaming/remote/codec/capture/tests.txt, preserved in the scratchpad and removed from the worktree.

### remote-capture-atlas-19: Three testdocs claim more than their bodies check
- Where: src/tree/mirror/streaming/remote/codec/capture/tests.rs:41-42 (related: src/tree/mirror/streaming/remote/codec/capture/tests.rs:238-258, src/tree/mirror/streaming/remote/codec/capture/tests.rs:260-272, src/tree/mirror/streaming/remote/codec/capture.rs:572-587, src/tree/mirror/streaming/remote/codec/capture.rs:238)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (each body read against its doc; capture.rs:572-587 read for the rendered form)
- Seen by: prose; refutation: confirmed; history: no rationale found (the "version-addressed hash" phrase was inaccurate from birth: written ten minutes after the commit that renamed the annotation to "causal version, event tree")
- Owner-gated: no

(1) `supply_reflection_localizes_the_field_that_moved` says "the record's version-addressed hash moves on the same line"; the moved line is `VERSION_TAG(h'<version bytes>') / causal version, event tree: ... /` (capture.rs:578-586): no hash is rendered, the hex is the version's own bytes. (2) `control_items_are_named_by_shape` claims "an unknown shape is a broken capture, not a renderable one" but never exercises the panic at capture.rs:238. (3) `nesting_past_the_depth_bound_falls_back` claims the nesting "falls back explicitly" while the body asserts only that `parse_node` returns an error containing "deeper than"; the fallback rendering is asserted only by the two `deep_*` tests. Every test's doc comment states its invariant in English and must be accurate; an inaccurate testdoc is a bug in the test.

Evidence:

    41	/// Containment, not equality: the record's version-addressed hash
    42	/// moves on the same line. The annotation's surrounding vocabulary is

    238	/// Control items are named by their shape; an unknown shape is a broken
    239	/// capture, not a renderable one.

    260	/// Nesting past the walk's depth bound falls back explicitly instead of
    261	/// recursing without bound on input-controlled depth.

Resolution: (1) "the version's canonical bytes (as hex) and its event-tree rendering move on the same line". (2) Add a `#[should_panic(expected = "no known shape")]` companion feeding a bare uint item, or drop the clause. (3) Retitle to what it checks (`parse_node` rejects nesting past `MAX_DEPTH` with the depth reason), or drive it through `render_item` and `assert_depth_fallback`. Acceptance: each testdoc names only what its body asserts.

### remote-capture-atlas-31: The admitted `FramePart` marker hole is closable with a wildcard-free `describe_part`
- Where: src/tree/mirror/streaming/remote/codec/tests/error_atlas.rs:76-86 (related: src/tree/mirror/streaming/remote/codec/error.rs:40-53, src/tree/mirror/streaming/remote/codec/tests/error_atlas.rs:14-18)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (error.rs:41-53: `FramePart` has five variants and derives `Debug`; the markers are `part={variant:?}` spellings)
- Seen by: correctness; refutation: confirmed; history: no rationale found (3939d36d gave every other error enum a wildcard-free describe function and left `FramePart` to a comment; 151135c5 reworded the comment for accuracy but kept the by-hand mechanism)
- Owner-gated: no

The rest of the file's design is that every enum gets a wildcard-free `describe_*` so the compiler is the tripwire; `FramePart` is the one enum left to a convention held in a comment. Every hole found becomes a committed check, never a convention held in memory.

Evidence:

    79	    // never fails at the signal separately from the frame head). FramePart
    80	    // has no exhaustive match here, so a new component's marker must be
    81	    // added by hand alongside its witnesses.
    82	    "part=FrameHead",
    83	    "part=Signal",

Resolution: Add `fn describe_part(part: FramePart) -> &'static str` with a wildcard-free match returning each variant's `Debug` spelling; derive the five `part=` markers from it (or assert in `atlas_covers_every_error_variant` that `part=<describe_part(v)>` appears for every variant). Then the comment can say the compiler enforces the roster. Acceptance: adding a `FramePart` variant without a witness fails compilation or the coverage test; the by-hand sentence is removed.

### remote-capture-atlas-32: The one variant-backed exemption is not tied to its variant, so the confessed exemption-rot hole stays open
- Where: src/tree/mirror/streaming/remote/codec/tests/error_atlas.rs:92-98 (related: src/tree/mirror/streaming/remote/codec/tests/error_atlas.rs:20-23, src/tree/mirror/streaming/remote/codec/tests/error_atlas.rs:89-91, src/tree/mirror/streaming/remote/codec/tests/error_atlas.rs:146-152, src/tree/mirror/framing.rs:57-65, src/tree/mirror/streaming/remote/codec/error.rs:64-76)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (framing.rs:59-65: `LengthOverflow { pub len: usize, pub source: std::num::TryFromIntError }`, both fields public; codec/error.rs:65-76: `SupplyTooLarge(#[from] LengthOverflow)`; the coverage test asserts only `!atlas.contains(marker)` at 146-152)
- Seen by: structure, prose; refutation: reframed (the `Greeting`/`Listing` entries name no variant of any inventoried enum, but 151135c5 added them deliberately as promotion tripwires with the reason stated inline and at module doc lines 6-12, so deleting them reopens a recorded decision without new evidence; the constructive half survives); history: deliberate and holds for the tripwires; the exemption-rot hole was a recorded choice (33bd7d56 punch list) to document rather than close, and nothing rules against closing it
- Owner-gated: no

The module doc admits (lines 20-23) that an exemption outlives its variant silently because the check asserts absence, which a deleted variant satisfies trivially, so pruning must happen by hand. For the one exemption that names a real variant, `SupplyTooLarge`, the hole is closable: both `LengthOverflow` fields are public, so the test can construct the variant, describe it, and assert the description carries the exempt marker; deleting the variant then fails compilation and renaming it fails the marker match. The `EXEMPT_MARKERS` summary at line 89 ("Variants deliberately absent") also reads loosely against the two tripwire entries, which name handshake-layer types rather than variants of the inventoried enums.

Evidence:

    92	const EXEMPT_MARKERS: &[(&str, &str)] = &[
    93	    (
    94	        "kind: SupplyTooLarge(",
    95	        "requires a run body past the wire's run byte cap: a >4 GiB in-memory \
    96	         run is resource exhaustion by construction; the cap itself is pinned \
    97	         at its exact boundary in frame/tests.rs",
    98	    ),

    20	//! or the witnesses; the comments at each match carry that obligation. An
    21	//! exemption also outlives its variant silently — the check asserts absence,
    22	//! which a deleted variant satisfies trivially — so pruning a variant must
    23	//! prune its `EXEMPT_MARKERS` entry by hand.

Resolution: In `atlas_covers_every_error_variant`, build `EncodeErrorKind::SupplyTooLarge(LengthOverflow { len: usize::MAX, source: u32::try_from(u64::MAX).unwrap_err() })`, run `describe_encode_kind` on it, and assert the output starts with the marker's suffix (`SupplyTooLarge(`). Rescope lines 20-23 to the two type-level tripwires (which have no variant to prune) or delete the sentence. Reword line 89 to "Markers deliberately absent from the atlas". Acceptance: every variant-backed `EXEMPT_MARKERS` entry is tied to a constructed instance; the module doc no longer describes a by-hand pruning obligation for variants.

### Nits

The full record of each (provenance, history, evidence) is in the evidence file named in the last column.

| Id | Where | Claim | Resolution | Record |
|---|---|---|---|---|
| remote-capture-atlas-24 | `src/tree/mirror/streaming/remote/codec/tests.rs:89-92` | `arb_query`'s `btree_map(any::<u8>(), .., 0..=256)` cannot realize a 256-child fan (about 162 distinct keys) | generate the fan as a subsequence of the radix space | `evidence/partitions/remote-capture-atlas.md` |
| remote-capture-atlas-20 | `src/tree/mirror/streaming/remote/codec/capture/tests.rs:263-268` | depth-bound tests write `0..=MAX_DEPTH` and `10 * MAX_DEPTH` arrays; neither side of the exact edge is pinned | add `MAX_DEPTH - 1` parses and `MAX_DEPTH` fails | `evidence/partitions/remote-capture-atlas.md` |
| remote-capture-atlas-33 | `src/tree/mirror/streaming/remote/codec/tests/error_atlas.rs:305-308` | the atlas pins one of `FrameShape`'s two `detail` strings | add `frame/arity-one` and `frame/arity-four` witnesses | `evidence/partitions/remote-capture-atlas.md` |

## Remote adapter, streams, and the adapter test suites

`read_early` is a second hand-written copy of the frame grammar with three of its rejection arms pinned; `label_item`'s four wire-reachable arms have no test; the backend-contract panics are promised in `Backend`'s docs and never demonstrated; `early_supplies`' distinguishing contracts (incremental yield, post-error withholding) are indistinguishable from `try_collect` in every committed consumer; and `MAX_RECORD_LEN` derives from retired framing while the largest committed fixture exceeds it.

### remote-adapter-tests-15: `read_early` duplicates the frame grammar but only three of its rejection arms are pinned on the early path
- Where: src/tree/mirror/streaming/remote/adapter/tests/opening.rs:245-290 (related: src/tree/mirror/streaming/remote/adapter/decode.rs:136-200, src/tree/mirror/streaming/remote/adapter/decode.rs:306-392, src/tree/mirror/streaming/remote/adapter/decode.rs:352-355, .agent-notes/2026-08-21-cbor-wire-mutants-scopeA/03-proxy-adapter.md:63-90)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`grep -n 'BareEndAfterReaction\|UnexpectedStreamEnd\|TruncatedReply\|UnpositionedQuery'` over opening.rs and fan_occupancy.rs is empty; read both loops side by side: `read_reply` carries the empty-run `debug_assert!` at 352-355 and `read_early`'s record loop at 155-178 does not)
- Seen by: blind-spots, api-economics; refutation: confirmed (medium stands: the recorded disposition is the missing check); history: already known as the `!any` mutant survivor with a grammar-recognizer differential family as its disposition, marked a design and not landed; the `debug_assert!` asymmetry is not covered by that note (8e1ed47a added the assert to `read_reply`; `read_early` was written five days later without it)
- Owner-gated: no

`early_supplies` reads frames through `read_early`, a second hand-written copy of the loop in `read_reply`. On the early path the suites pin `OverdrawnSupply`, `ExtraOpeningReply`, and `UnpositionedMatch` only. Each of `TruncatedReply` (decode.rs:151-152), `UnpositionedQuery` (185-187), `BareEndAfterReaction` (188-189, via the `any` flag), `UnexpectedStreamEnd` (190), `OversizedVersion`, `LeafOrder`, `SupplyOrder`, and `Record` has a distinct arm there and is tested only through `decode_reply`/`decode_leaf_reply`. When a grammar is implemented twice, a pin on one copy says nothing about the other; a fix applied to one loop (reordering the `any` check, dropping a `?`) passes the suite. The doctrine is that every hole found becomes a committed check, never a note.

Evidence:

    245	/// The opening-supply stream carries exactly one reply: frames after its
    246	/// end are rejected, not absorbed into a phantom second reply.
    247	#[test]
    248	fn second_opening_supply_reply_is_rejected() {
    249	    let frames: Vec<Frame> = vec![Frame::End(End::Reply), Frame::End(End::Reply)];

    188	            Frame::End(End::Reply) if !any => Flow::End,
    189	            Frame::End(End::Reply) => return Err(DecodeError::BareEndAfterReaction),
    190	            Frame::End(End::Stream) => return Err(DecodeError::UnexpectedStreamEnd),

Resolution: land the recognizer-differential family the note designs: generate short frame words over {Supply(Continue), Supply(End), End(Reply), End(Stream), Match, Query}, define the accepted language once (`Supply(Continue)* Supply(End)` or one bare `End(Reply)`; nothing after), and assert `early_supplies` accepts exactly it and rejects each other word with the variant the recognizer predicts; the existing point tests become witnesses of the family or are dropped. Alternatively make the malformed cases entry-point-parametric (one table run through both `decode_reply` over `Scope::opening(&[])` and `early_supplies` over the root prefix), or factor the shared record handling into one function both loops call. Add the empty-run `debug_assert!` to `read_early` or state at the site why the asymmetry is intended. Acceptance: a test in opening.rs fails when decode.rs:188's `if !any` is replaced by `true`; each of `BareEndAfterReaction`, `UnexpectedStreamEnd`, `TruncatedReply`, `UnpositionedQuery` is asserted at least once against `early_supplies`.
Construction: `early_supplies` over `[Frame::Reaction(WireReaction::Supply(one record), Flow::Continue), Frame::End(End::Reply)]` must return `Err(DecodeError::BareEndAfterReaction)`; with the `!any` guard replaced by `true` it returns `Ok` with one node, which no committed test observes.

### remote-adapter-tests-22: `MAX_RECORD_LEN`'s derivation describes the retired framing, its stated envelope is false for the committed fixture, and nothing enforces it
- Where: src/tree/mirror/streaming/remote/adapter/tests/runs.rs:42-49 (related: src/tree/mirror/streaming/remote/adapter/tests/runs.rs:53-67, 162-165; src/tree/mirror/streaming/remote/codec/frame.rs:33, 36, 155-169; src/tree/mirror/cbor.rs:80-88; crates/before/src/codec/gamma.rs:25-45; crates/before/src/version/skyline/literal.rs:18-23)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (re-ran the refutation pass's offline model of the framing from the read code: `RECORD_TAG_LEN = head_len(63) = 2`, body head 1, `VERSION_TAG_LEN = head_len(0xD256) = 3`, version head 1, skyline leaf literal as topology bit plus Elias-gamma plus the `1 0*` marker, ciborium shortest-form uint payload, untagged SHA3-256 path; the model reproduces `before`'s committed `0xE0` vector for the empty version. `colliding_leaves(10)` selects values 201, 328, 415, 422, 429, 511, 839, 1076, 1077, 1106 with record lengths [14, 15, 15, 15, 15, 15, 15, 15, 15, 15], sum 149; `colliding_leaves(4)` peaks at 14. `git log -L42,49` shows the doc lines from f94f2056 (2026-07-18) untouched since; borsh was retired at f2b74a97 (2026-08-18) and the record re-spelled again at 4dd2053c, neither touching runs.rs)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed and aligned to medium; history: deliberate but expired (exact for the framing f94f2056 introduced)
- Owner-gated: no

The comment derives 14 as "a 4-byte header, a version of at most two bytes, and the fixed 8-byte payload"; under the current codec the per-record framing is 7 bytes, a leaf version of the fixture's scalars (around 2^15..2^18) is 5 bytes, and the payload is 1-3 CBOR bytes. Nine of the ten records in the largest committed case are 15 bytes, so the constant's own contract ("Upper envelope on one encoded `u64` leaf record here") is breached by the fixture. `MAX_CASE_BUDGET = 11 × 14 + SUPPLY_FRAME_OVERHEAD` exceeds the count-10 run (149 + overhead) by five budget values, so the doc's promised coverage of "run-swallowing budgets" survives at the maximum case on a five-byte accident that nothing committed protects. The laws hold at every budget, so this is silent coverage narrowing, not a false pass; the ghost derivation is the same defect the repo's hard rule on prose names.

Evidence:

    42	/// Upper envelope on one encoded `u64` leaf record here: a 4-byte header,
    43	/// a version of at most two bytes, and the fixed 8-byte payload.
    44	const MAX_RECORD_LEN: usize = 14;
    45	
    46	/// Exclusive bound on generated byte budgets: past every record and past a
    47	/// whole case's run with its frame envelope, so the sweep covers zero,
    48	/// sub-record, mid-run, and run-swallowing budgets.
    49	const MAX_CASE_BUDGET: usize = (MAX_CASE_LEAVES + 1) * MAX_RECORD_LEN + SUPPLY_FRAME_OVERHEAD;

    155	    pub fn record_len(version: &Version, message: &Message) -> usize {
    156	        let body = Self::record_body_len(version, message);
    157	        RECORD_TAG_LEN
    158	            .saturating_add(cbor::head_len(body as u64))
    159	            .saturating_add(body)

Resolution: derive the sweep's upper bound from the fixture rather than prose: compute it as `SUPPLY_FRAME_OVERHEAD + colliding_leaves(MAX_CASE_LEAVES).iter().map(|l| LeafRun::record_len(&l.version, &l.message)).sum::<usize>() + 1` (via `prop_flat_map` on `count`, or a `LazyLock`), or keep the constant and `prop_assert!(LeafRun::record_len(&leaf.version, &leaf.message) <= MAX_RECORD_LEN)` for every generated leaf as the envelope's liveness floor. Rewrite the doc in present-tense codec terms (record tag and body head, version tag and head, canonical version bytes, CBOR payload) or cite `LeafRun::record_len`. Add one witness that the maximum generated budget yields a single frame for the largest case. Acceptance: a committed assertion ties the sweep's upper bound to the actual record lengths of the generated leaves (lowering `MAX_RECORD_LEN` to 13 fails the property); the constant's doc names no borsh-era widths; a test shows the ten-leaf case encodes as one frame at the top budget.

### remote-adapter-streams-2: `early_supplies`' post-error withholding is stated in a comment but no committed test can distinguish it from `try_collect`
- Where: src/tree/mirror/streaming/remote/adapter/decode.rs:109-113 (related: src/tree/mirror/streaming/remote/adapter/decode.rs:83-131, src/tree/mirror/streaming/remote/adapter/tests/opening.rs:153-165, src/tree/mirror/streaming/remote/adapter/tests/opening.rs:203-211, src/tree/mirror/streaming/remote/adapter/tests/fan_occupancy.rs:136-146, src/tree/mirror/streaming/remote/proxy/work/pump.rs:417-431)
- Class / severity / confidence: verification-gap / low / high
- Provenance: assessed (read; every `early_supplies` consumer in `opening.rs` and `fan_occupancy.rs` is `try_collect`, confirmed by grep)
- Seen by: correctness ([28]); refutation: reframed (the protected hazard is the assembler's end-of-input flush of the truncated final group, not the withheld complete group); history: no-rationale-found (the mechanism has been inline since 55d76d5c; the test gap is unrecorded)
- Owner-gated: no

The joint reader/assembler poll returns `Ready(None)` the moment the reader has failed, so nothing the assembler still holds is yielded. What that protects: when `read_early` returns `Err`, its `leaves` sender drops, the channel looks cleanly ended, and `ops::assemble` flushes the group still in assembly as if complete (backend.rs:191: "one node per maximal run"), so the truncated final group would otherwise surface as a node. Every committed consumer uses `try_collect`, which stops at the first `Err` regardless, so a refactor that polled `assembled` before consulting `read_result` (yielding the flushed group ahead of the error) passes the whole suite while handing `Early::advance_to` a node the reader never finished vouching for. Every criterion needs a committed demonstration that a known-bad mechanism fails it.

Evidence:

       109	                // A reader error poisons everything after it: the group in
       110	                // assembly may be a truncation, so nothing more is yielded.
       111	                if matches!(read_result, Some(Err(_))) {
       112	                    return Poll::Ready(None);
       113	                }

Resolution: Add a test to `adapter/tests/opening.rs` built like `opening_supplies_decode_by_radix_group` (three first-byte groups a < b < c, one supply frame each), a paced source (`.then(|f| async { yield_now().await; f })` as `fan_occupancy.rs:172-175` does), and a failure inside c (a `SupplyLedger::new(|a| + |b| + 1)` allowance, or a repeated record for `LeafOrder`); drive with `next()` and assert the exact sequence `[Some(Ok((a, _))), Some(Err(_)), None]`. Acceptance: the new test fails when the two statements in the `poll_fn` closure are reordered so `assembled` is polled before `read_result` is checked, and passes at HEAD.
Construction: Under a current-thread runtime with the paced source the schedule is deterministic: poll 1 reads a's frame and pends; poll 2 reads b's frame, the assembler consumes a's leaves and b's first leaf and yields a; poll 3 reads c until the failing record, `read_result` becomes `Some(Err)`, and the closure returns `Ready(None)` before the assembler is polled again, so b (complete in the assembler) and c (flushed by the dropped sender) never surface. With the statements reordered, poll 3 polls the assembler first, which pulls the remaining channel contents and yields b before the error is consulted.

### remote-adapter-streams-14: Backend-contract panics are promised in `Backend`'s docs but never demonstrated to fire, and the conformance suite does not check run order or containment
- Where: src/tree/mirror/streaming/remote/adapter/encode.rs:249-262 (related: src/tree/mirror/streaming/remote/adapter/encode.rs:223, src/tree/mirror/streaming/remote/adapter/decode.rs:292-300, src/tree/mirror/streaming/remote/adapter/decode.rs:417-441, src/tree/mirror/streaming/backend.rs:158-203, src/conformance/backend.rs:379-490, src/lib.rs:322, src/conformance.rs:18-19)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`grep -rn should_panic src/tree/mirror/streaming src/conformance` hits only `materialized/progress/tests.rs`; read the conformance `leaves` wrapper (379-428: pricing, aggregate coverage, walked count) and `assemble` wrapper (430-490: runs keyed by `Prefix::<H>::containing` in a `BTreeMap`, leftovers and unsupplied nodes checked), neither of which examines order or containment)
- Seen by: correctness ([29]); refutation: reframed (only `Backend::assemble`'s doc cites the conformance suite; `Backend` is crate-private, so panic is the right shape and the finding is about demonstration, not error typing); history: already-known for `validate_leaf` (scope-A campaign dispositioned `validate_leaf → ()` as a misbehaving-backend adequacy family, not yet landed); the `reify` asserts and the decode.rs:299 `unreachable!` have no recorded disposition
- Owner-gated: no

`validate_leaf` asserts containment and strict path order for every leaf a backend enumerates, `render` asserts at least one leaf per node (223), `reify` asserts one assembled node per run at the run's prefix (425-439, which also enforces run order since the skeleton is walked in order), and `decode` has an `unreachable!` premised on the assembler consuming its whole input (292-300). `Backend::leaves`'s doc (backend.rs:162-163) says "the wire encoder enforces each by panic" and `Backend::assemble`'s (188-197) says "the reply decoder enforces each by panic" and that the conformance suite "convicts a violating override in the backend's own tests". The conformance `assemble` wrapper convicts merged, split, skipped, and wrong-prefix nodes but is order-insensitive, and its `leaves` wrapper checks counts and pricing, never order or containment. No `#[should_panic]` demonstration exists for any of the three encoder/decoder messages. A guard is justified by naming a constructible failure it catches; every criterion needs a committed demonstration that a known-bad mechanism fails it. `Backend` is crate-private (`mod tree;` at lib.rs:322; `conformance::backend` is `#[cfg(test)] pub(crate)`), so these are programmer-error panics by the crate's own standard and panic is the right shape; what is missing is the demonstration and an accurate doc.

Evidence:

       249	fn validate_leaf(expected: ErasedPrefix, previous: Option<Prefix<Z>>, current: Prefix<Z>) {
       250	    let path = Path::from(current);
       251	    assert_eq!(
       252	        &<[u8; 32]>::from(path)[..expected.as_bytes().len()],
       253	        expected.as_bytes(),
       254	        "a backend enumerates leaves beneath the requested node prefix",
       255	    );
       256	    if let Some(previous) = previous {
       257	        assert!(
       258	            previous < current,
       259	            "a backend enumerates leaves in strict path order",
       260	        );
       261	    }
       262	}

    backend.rs:
       195	    /// The backend conformance suite (see [`crate::conformance`]) convicts
       196	    /// a violating override in the backend's own tests, before a live
       197	    /// session can meet it.

Resolution: Land the scope-A disposition: a test backend wrapping `Local` whose `leaves` override swaps two adjacent yields (or displaces one prefix) and whose `assemble` override drops the last node or ends early while leaves remain, with `#[should_panic(expected = ..)]` tests for each of the three encoder/decoder messages and the decode.rs:299 `unreachable!`. Either add order and containment checks to the conformance `leaves`/`assemble` wrappers so "convicts a violating override" is true for every listed clause, or narrow backend.rs:195-197 to the clauses the suite checks. Acceptance: a committed test panics with each message under a deliberately misbehaving backend; `Backend::leaves`/`assemble` rustdoc lists exactly the clauses that are enforced somewhere, and names where.
Construction: Wrap `Local` in a test backend whose `leaves` collects the inner stream, reverses two adjacent items, and re-yields; call `encode_reply` on a two-leaf node under `#[should_panic(expected = "strict path order")]`. For `reify`, an `assemble` override that drops its last yielded node reaches the `expect` at decode.rs:427; one that ends its stream after the first node while leaves remain reaches decode.rs:299.

### remote-adapter-streams-28: `label_item`'s two rejection arms and two EOF-deferral arms have no committed test
- Where: src/tree/mirror/streaming/remote/streams.rs:756-778 (related: src/tree/mirror/streaming/remote/streams.rs:159-168, src/tree/mirror/streaming/remote/streams.rs:694-704, src/tree/mirror/streaming/remote/streams/tests.rs:227-236, src/tree/mirror/streaming/remote/streams/tests.rs:415-448, src/tree/mirror/streaming/remote/streams/tests.rs:461-485, src/tree/mirror/streaming/remote/proxy/tests/failures.rs:28-53, src/tree/mirror/streaming/remote/proxy/tests/failures.rs:167-243)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`git grep -n 'AcceptError::Label\|label item is not an unsigned int\|label head is not canonical\|label_item' -- src/ tests/` hits only streams.rs itself and the capture harness's unrelated `label_item`; the committed tests reach `SupplyFailed` only through `acceptor.accept()` failing (tests.rs:47-51, 467), never through `label_item`'s `Ok(None)` or `Err(Io)` arms; the transport-fault proptest injects errors carrying `InjectedIo` (failures.rs:29-53) and never a clean close)
- Seen by: correctness ([27]); refutation: confirmed, severity lowered to low (the arms decide error classification and termination timing under a nonconforming peer or a dying transport, a conformance-bug detector, not a data outcome); history: already-known (the scope-A mutation campaign recorded `label_item`'s `MAJOR_UINT` guard as a survivor and dispositioned it with an exhaustive initial-byte matrix against `cbor::read_head` plus an EOF witness; not landed at HEAD)
- Owner-gated: no

Four wire-reachable arms are untested: a label item that is not an unsigned int, a non-canonical head, a clean close before an item (`Ok(None)` classified as `SupplyFailed(UnexpectedEof)`), and an I/O failure inside a head. Every sibling `AcceptError` arm (`Epoch`, `UnknownStream`, `Duplicate`, `Unexpected`) is pinned in `streams/tests.rs`. The EOF-deferral arms are the mechanism behind the peer-side consequence `StreamSender::frame`'s `# Cancel safety` section promises (a short label read classified as a failed supply that drops every undelivered claim slot), so the file's one operational hazard claim rests on unpinned code. A wire-reachable rejection with no test is a criterion the wrong implementation also passes. The campaign's recorded family covers both `Label` arms and the clean-close arm; the `Err(Io)` arm and the cancel-safety cross-reference are additions.

Evidence:

       761	    match cbor::read_head_async(rx).await {
       762	        Ok(Some(head)) if head.major == cbor::MAJOR_UINT => Ok(head.value),
       763	        Ok(Some(_)) => Err(AcceptError::Label {
       764	            origin: Origin::direction(speaker),
       765	            detail: "label item is not an unsigned int",
       766	        }
       767	        .into()),
       768	        Ok(None) => Err(AcceptFate::SupplyFailed(
       769	            std::io::ErrorKind::UnexpectedEof.into(),
       770	        )),
       771	        Err(cbor::HeadReadError::Io(io)) => Err(AcceptFate::SupplyFailed(io)),
       772	        Err(cbor::HeadReadError::Malformed(_)) => Err(AcceptError::Label {
       773	            origin: Origin::direction(speaker),
       774	            detail: "label head is not canonical",
       775	        }
       776	        .into()),
       777	    }

Resolution: Land the recorded disposition beside `accept_driver_rejects_unknown_stream_index`, using `raw_labeled`-style raw connects on a `memory()` link: (1) write `[0x40, 0x03]` (a byte-string head where the epoch belongs) and assert `AcceptError::Label { detail: "label item is not an unsigned int", .. }`; (2) write `[0x18, 0x00, 0x03]` (epoch 0 spelled with a one-byte argument) and assert `AcceptError::Label { detail: "label head is not canonical", .. }`; (3) write `[EPOCH]` alone and drop the writer, drive a receiver awaiting its claim through `first_reported_error`, and assert `StreamError::SupplyClosed { source: None, .. }` with `take_supply_failure()` yielding `UnexpectedEof`; (4) write `[0x18]` (the first byte of a two-byte head) and drop, asserting the same deferral. Cross-reference (3) from `StreamSender::frame`'s cancel-safety section so the doc claim names its pin. Acceptance: the four tests exist, each fails when its arm in `label_item` is replaced by a different `AcceptFate`, and the `frame()` cancel-safety text names the test that pins the peer-side classification.
Construction: As in the resolution; all four are buildable from the existing `raw_labeled`, `claims`, `error_route`, and `first_reported_error` helpers.

### remote-adapter-tests-14: `early_supplies`' documented incremental yield is never observed
- Where: src/tree/mirror/streaming/remote/adapter/tests/opening.rs:153-165 (related: src/tree/mirror/streaming/remote/adapter/tests/opening.rs:229-241, src/tree/mirror/streaming/remote/adapter/tests/fan_occupancy.rs:136-146, src/tree/mirror/streaming/remote/adapter/decode.rs:57-61, src/tree/mirror/streaming/remote/proxy/work/pump.rs:493-506)
- Class / severity / confidence: verification-gap / low / high
- Provenance: assessed (read; every happy-path test of `early_supplies` in the partition collects the whole stream before asserting)
- Seen by: blind-spots; refutation: confirmed (no other test in the crate observes the schedule; the consequence of regression is pipelining latency, not deadlock or memory); history: the contract is deliberate and stated in code (decode.rs:57-61, adapter.rs:33-35, 55d76d5c's message), the tests were written collecting from the start
- Owner-gated: no

decode.rs states the distinguishing contract of `early_supplies`: "this stream yields each assembled node as soon as its group completes". The three happy-path tests `try_collect` before asserting, so an implementation that buffered every group until the reply end passes all of them. The property is what the responder's root merge-join pipelines on, and it is the function's stated reason to exist apart from `decode_reply`.

Evidence:

    153	    let decoded: Vec<(u8, _)> = runtime()
    154	        .block_on(
    155	            early_supplies::<Local, _>(
    156	                Local,
    157	                u64::MAX,
    158	                unbounded(),
    159	                Prefix::new().erase(),
    160	                stream::iter(frames),
    161	                PayloadCodec::new::<u64>(PayloadDepthLimit::default()),
    162	            )
    163	            .try_collect(),
    164	        )
    165	        .expect("a canonical opening-supply reply decodes");

Resolution: feed frames through a `tokio::sync::mpsc` channel on the current-thread runtime: send the frames completing group A plus the first record of group B, poll the stream once, and assert A's `(radix, node)` has been yielded before any further frame is sent; then send the rest and assert B and the end. The fan_probe's determinism argument (single thread, FIFO channel) applies unchanged. Acceptance: a committed test fails when `early_supplies` is replaced by a variant that collects all groups before yielding.
Construction: in opening.rs, replace `stream::iter(frames)` with the receiving half of an `mpsc` channel; after sending group A's frames and one record of group B, `poll_next` the stream once (a `futures::poll!` on the pinned stream) and assert `Some(Ok((radix_a, _)))`; a buffered-until-end implementation returns `Pending`.

### remote-adapter-tests-4: backend_errors.rs claims every reachable backend operation, but `Leaf::leaf` failure is not injectable
- Where: src/tree/mirror/streaming/remote/adapter/tests/backend_errors.rs:1-1 (related: src/tree/mirror/streaming/remote/adapter/tests/backend_errors.rs:203-205, src/tree/mirror/streaming/testing/failing.rs:26-33, src/tree/mirror/streaming/testing/failing.rs:185-195, src/tree/mirror/streaming/remote/adapter/decode.rs:168-170, src/tree/mirror/streaming/remote/adapter/decode.rs:375-377)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read failing.rs: `Operation` has only `Children` and `Parent`; `FailingNode::leaf` maps `Failure::Inner` and never injects; decode.rs maps `Leaf::leaf` errors to `DecodeError::Backend` at two sites)
- Seen by: structure-prose, blind-spots; refutation: confirmed; history: deliberate but expired (true at 07504b2e; 8aeed2dd made `Leaf::leaf` fallible and kept `Failing` out of construction without narrowing this doc)
- Owner-gated: no

The adapter reaches a third fallible backend operation on the decode side, leaf construction, where payload custody passes to the backend; `Failing` cannot fail there, so the atomicity claims (no partial reply, no later call, sentinel untouched) at that moment are unexercised. The test's own doc at 203-205 correctly says "explode/assemble"; the module doc claims more than the instrument can inject.

Evidence:

    1	//! Source-error propagation across every backend operation reachable by the adapter.

    185	    // Custody passes straight through: fault injection targets the
    186	    // traversal operations, not construction.

Resolution: either add an `Operation::Leaf` injection point to `Failing` (in-crate test infrastructure) and a decode row that fails at the k-th record of a multi-record run, asserting the typed error, the history, the untouched sentinel, and via the census that no node from the failing record took custody; or narrow line 1 to "every backend traversal operation" and, at failing.rs:185, state why construction is exempt (the comment currently gives no reason beyond itself). Acceptance: a committed test drives `decode_reply` to `DecodeError::Backend(Failure::Injected(Operation::Leaf))`, or the module doc no longer claims every reachable operation.

### remote-adapter-tests-6: the injected-failure matrix's second ordering is dead for the committed fixture, and its atomicity assertion is fixture-specific
- Where: src/tree/mirror/streaming/remote/adapter/tests/backend_errors.rs:61-107 (related: src/tree/mirror/streaming/remote/adapter/tests/backend_errors.rs:208, src/tree/mirror/streaming/remote/adapter/encode.rs:158-245)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (re-ran the refutation pass's offline model: the skyline leaf literal for `(0xfeed_face << 8) | 7` hashed with untagged SHA3-256, as `PathHash::of` does at hash.rs:257-258, gives path e1891ba5…932e with no 0xff byte at indices 0..=30; the model reproduces `before`'s committed `0xE0` vector for the empty version at span/tests.rs:471)
- Seen by: blind-spots (the over-strong assertion); refutation: raised the dead arm as new and confirmed the assertion reading; history: no rationale found (fixture and arms from 07504b2e, no message body)
- Owner-gated: no

The test selects the reaction order by whether the supply radix is 255, but for `LeafCase::new(0xfeed_face, 7)` no byte of the path at any of the 31 exercised heights is 0xff, so the `[Query, Supply]` arm never executes and the atomicity assertion is only ever exercised with the failing supply first. Separately, `yielded.is_empty()` is stronger than the contract for replies of three or more reactions (the encoder holds only the last frame pending, encode.rs:158, so a frame before the failing reaction is legitimately yielded); the two-reaction fixture cannot distinguish the doc's claim ("no frame after the failure") from "no frame at all". The branch's presence suggests coverage it does not deliver.

Evidence:

    63	        let positional_radix = if supply_radix < u8::MAX {
    64	            supply_radix + 1
    65	        } else {
    66	            supply_radix - 1
    67	        };
    ...
    73	            let replies = if supply_radix < u8::MAX {
    74	                vec![
    75	                    Reaction::Supply(supply_radix, supply),
    76	                    Reaction::Query(Vec::new()),
    77	                ]
    78	            } else {
    79	                vec![
    80	                    Reaction::Query(Vec::new()),
    81	                    Reaction::Supply(supply_radix, supply),
    82	                ]
    83	            };
    ...
    102	            assert!(
    103	                yielded.is_empty(),
    104	                "height {} failure {fail_after} published a frame or question",
    105	                Self::HEIGHT,
    106	            );

Resolution: drive both orderings explicitly (a row per ordering, or a second fixture value whose path carries a 0xff byte at some height) rather than selecting by the fixture's radix; add a three-reaction row (for example `[Match, Match, Supply(failing)]` under a two-child listing) asserting that exactly the frames before the pending one were yielded with `Flow::Continue`, the pending frame was withheld, and the stream ended; or reword the doc to "withholds the pending frame". Acceptance: both orderings execute for every height (a counter or a per-ordering row makes that visible); the new row fails if `render` yields its pending frame after a source error.

### remote-adapter-tests-12: the set-length test claims failure at the first over-record but pins only residency below 128
- Where: src/tree/mirror/streaming/remote/adapter/tests/malformed.rs:597-683 (related: src/tree/mirror/streaming/remote/adapter/tests/opening.rs:178-199, src/tree/mirror/streaming/remote/adapter/decode.rs:367-382, src/testing.rs:51-52)
- Class / severity / confidence: test-quality / low / medium
- Provenance: assessed (read; the refutation pass confirmed by reading that every `OverdrawnSupply` site in src/ and tests/ matches the variant only)
- Seen by: structure-prose; refutation: confirmed; history: no rationale found (08f2899b added the `small < SMALL` clause as a loose bound; the slope-then-equality design is deliberate and recorded, the exactness clause is not)
- Owner-gated: no

The testdoc says the reply "fails typed at its first over-declaration record" and "custody provably stops at the charge", but the body pins the error variant, residency equality across a doubled overrun, and `small < SMALL` (128). A decoder that admitted a hundred records before charging would pass. With an allowance of one, residency at rejection is O(1), so a tight bound is derivable. Relatedly, opening.rs:178-180 claims the rejection lands "while the one opening reply is still open", but its fixture is one `Flow::End` frame with no trailing sentinel, so openness is unobserved (backend_errors.rs:124-159 shows the sentinel technique). The test also reads the process-global `census` directly (line 612) rather than through `testing::node_census`, whose doc states the premise the equality rests on ("tests that assert on them must own the process"); that premise is not restated here.

Evidence:

    597	/// A reply streaming past the declared `set_len` fails typed at its first
    598	/// over-declaration record, under node residency independent of the
    599	/// overrun; a declaration exactly covering the stream admits it whole.
    ...
    678	    assert!(
    679	        small < SMALL as usize,
    680	        "custody stops at the charge: {small} resident handles against a \
    681	         {SMALL}-leaf stream",
    682	    );

Resolution: measure the residency at rejection once and pin it (exact, or `<= 1 + <assembly's open-parent slot>` with the derivation in a comment) so a charge delayed by k records fails; for the opening test, append a sentinel frame and assert it is unconsumed, or drop the "still open" clause; restate the process-per-test premise at the census read. Acceptance: a hand-mutation that moves `ledger.charge(1)` after `Leaf::leaf` in decode.rs, or delays it by several records, fails the test; every clause of the testdoc has an assertion behind it.
Construction: temporarily reorder decode.rs:372-377 so the charge follows the `Leaf::leaf` await and the send; run `a_reply_past_the_declared_set_len_fails_at_its_first_over_record` and observe it still passes (residency grows by at most one). Then delay the charge by four records via a counter and observe it still passes because 4 < 128.

### remote-adapter-streams-29: The label testdoc states a point ("exactly two bytes") as the law; the label is three bytes from epoch 24
- Where: src/tree/mirror/streaming/remote/streams/tests.rs:22-27 (related: src/tree/mirror/streaming/remote/streams.rs:57-68, src/tree/mirror/cbor.rs:80-83, src/tree/mirror/cbor.rs:105-112, src/link.rs:352, src/link.rs:391, tests/reuse.rs:184-226, src/tree/mirror/streaming/remote/streams/tests.rs:424)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (cbor.rs:81-83 gives head widths `0..=23 => 1, 24..=0xff => 2`; `render_head` at 112 emits `[major | 24, value as u8]` for 24..=255; the epoch is `u8` (link.rs:352) advanced by `wrapping_add(1)` (391); `label` writes two heads (streams.rs:65-66) and sizes its buffer `head_len(epoch) + 1` (64); the construction below is derived from `render_head`, not executed)
- Seen by: prose ([17]), correctness ([32]); refutation: confirmed (both); history: already-known (true when written in b3b877d9, when the label was `[epoch, stream.index()]`; expired at 4dd2053c when the label became two CBOR heads; the CBOR review recorded the rot as R4 with a `two-byte` sweep that does not match this testdoc's `two bytes`, and S1 records the owner-endorsed count-free wording for such fixes)
- Owner-gated: no

The testdoc claims the label is exactly two bytes. From the 25th session on a reused link the epoch head is two bytes and the label three; the code's own capacity computation already accounts for it, and `tests/reuse.rs::epoch_wrap_keeps_the_pair_in_lockstep` exercises epochs 253 through 2 end to end, so the label itself is right and the defect is the testdoc and the missing family statement. AGENTS.md: an inaccurate testdoc is a bug in the test; a claim over `epoch: u8` is a family and belongs to a proptest. A related fragility: tests.rs:424 hand-spells a label as `&[EPOCH, Stream::COUNT]`, which is a valid one-byte head only because both values are below 24.

Evidence:

        22	/// The label is exactly two bytes: the session epoch then the stream index.
        23	#[test]
        24	fn label_is_epoch_then_stream() {
        25	    let stream = Stream::new(3).expect("stream 3 exists");
        26	    assert_eq!(label(7, stream), [7, 3]);
        27	}

Resolution: Reword the testdoc count-free per S1 ("The label is the epoch head then the stream-index head, each a shortest-form CBOR unsigned int; both below 24 encode as one byte each"), keep the literal case as the readable example, and add a proptest over `epoch in any::<u8>()` and every `Stream` asserting `label(epoch, stream).len() == cbor::head_len(epoch) + 1` and that two `cbor::read_head` calls recover `(epoch, index)` and exhaust the input; spell tests.rs:424 through `cbor::write_head`. Acceptance: the testdoc is true for every `u8` epoch; a committed proptest exercises epochs at and above 24; any seed file that appears is committed.
Construction: `label(24, Stream::new(3).unwrap())` is `[0x18, 0x18, 0x03]` by `render_head` (major 0, info 24, argument byte 24; then `0x03`), three bytes, refuting "exactly two bytes".

### Nits

The full record of each (provenance, history, evidence) is in the evidence file named in the last column.

| Id | Where | Claim | Resolution | Record |
|---|---|---|---|---|
| remote-adapter-streams-30 | `src/tree/mirror/streaming/remote/streams/tests.rs:36-43` | six identical `StreamSender::new` and four `StreamReceiver::new` spellings hide each test's distinguishing inputs | `sender(..)` and `receiver(..)` helpers | `evidence/partitions/remote-adapter-streams.md` |
| remote-adapter-tests-7 | `src/tree/mirror/streaming/remote/adapter/tests/backend_errors.rs:164-176` | bare `assert!(matches!(..))` rejections print the pattern, not the received error | let-else with a diagnostic, or `{error:?}` in the message | `evidence/partitions/remote-adapter-tests.md` |
| remote-adapter-tests-9 | `src/tree/mirror/streaming/remote/adapter/tests/fan_occupancy.rs:113-115` | `FAN + 1` is spelled independently at five sites and only one pair is mechanically bound | one `pub(crate)` records-per-stream constant in window.rs | `evidence/partitions/remote-adapter-tests.md` |
| remote-adapter-tests-13 | `src/tree/mirror/streaming/remote/adapter/tests/malformed.rs:718-718` | the `SupplyOrder` pin matches the variant and ignores both fields | destructure and assert `previous == radix == leading byte` | `evidence/partitions/remote-adapter-tests.md` |

## Remote proxy and the proxy test suites

The production topology (the proxy as protocol Server on both ends) runs on six tests while every adversity suite puts one proxy in the Client position; no frame crosses a link on a stream with index two or greater; the ordering trace passes on an empty trace; the backend-fault property discards the unfaulted tree; nine error arms and the accept arm of `Work::execute` are unreached; and the greeting suites' convergence oracle excludes the ceiling.

### remote-proxy-tests-10: No frame ever crosses a link on a logical stream with index two or greater, anywhere in the crate
- Where: src/tree/mirror/streaming/remote/proxy/tests.rs:388-394 (related: tests.rs:26; src/tree/arb.rs:235-237, 567, 625; src/tree/mirror/streaming/tests/fixtures.rs:475, 494; src/tree/mirror/streaming/remote/codec/signal.rs:55-77; src/tree/mirror/streaming/driver.rs:161-171; src/tree/mirror/streaming/remote/proxy/state.rs:89-91; tests/snapshots/*.snap)
- Class / severity / confidence: verification-gap / high / high
- Provenance: demonstrated (second witness pass: an observer counting frames per sent stream index over the in-memory link showed a 24-leaf content-addressed divergence opening only stream indices 0 and 1 on either side, matching the committed snapshots, while both deep fixtures opened streams 0..=16 and failed over the wire with `Decode(LeafOutsideScope ..)`, so the gap holds and the proposed closing test cannot close it as written; before the pass, verified by grep: `grep -rhoE '(Initiator|Responder) stream [0-9]+ \(height [0-9]+\)' tests/snapshots | sort | uniq -c` returns only stream 0 at height 31, `Initiator stream 1 (height 30)`, and `Responder stream 1 (height 29)`; `RemoteHandshaking` is constructed outside this partition only in streaming.rs, peer/gossip.rs, and peer/gossip/tests.rs, none of which uses a deep fixture; every fixture in this partition is content-addressed or `early_first_child_dispute_pair`)
- Seen by: blind-spots; refutation: reframed (the leaf-tier functions do run on every session; what has no wire witness is frames on streams >= 2); history: no rationale (the deadlock note records this tier's generator distribution being found too thin for the deadlock's width geometry and closed at b3b877d9 with `early_first_child_dispute_pair`; nothing extends that to depth; `git log -S'leaf_parent_dispute_pair' -- src/tree/mirror/streaming/remote` is empty)
- Owner-gated: no

Every fixture reaching the proxy tier is content-addressed: SHA3 scatters leaf paths at the root fan, so disputes resolve within the first two heights, and successive streams descend two heights each, so logical streams 2..16 in either direction never carry a frame. The proxy's per-height stages do execute on every session (`mirror_connected` steps every reply round, and `stream_at::<H>` runs for every height), but with empty request sets; the stream opens at indices >= 2, the leaf-tier decode pump on live traffic, and the `LeafParentReplies`/`TerminalLeafReplies` grammar arms validating real frames are exercised over a link by nothing, and the committed public snapshots are the mechanical statement of the gap. The deep fixtures that exist (`leaf_parent_dispute_pair`, `leaf_parent_redaction_pair`, both `pub` in `tree::arb` and returning their expected union; `full_depth_comb_pair`, `pyramid_pair`, `pub(super)` in the streaming tests) run only through two in-process participants. The doc's "arbitrary valid divergence" overstates what the generator reaches. Correct at all scales: a wrong speaker/height parity for a deep stream, a mis-wired leaf reply, or a placement-grammar bug on the terminal streams would pass every test in the crate that runs over a link.

Evidence:

       388	    /// For arbitrary valid divergence, crossing the codec and per-stream
       389	    /// transport is observationally identical to the in-process protocol.
       390	    #[test]
       391	    fn wire_reconciliation_matches_local(
       392	        (a, b) in arb_divergent_pair(),
       393	        schedule in vec(0_u8..=2, 0..128),
       394	    ) {

    src/tree/arb.rs:
       235	/// Content-addressed generators cannot produce this shape — SHA3-256 scatters
       236	/// their keys at the root fan, so a merge's divergent descent below the
       237	/// root is reachable only through chosen paths like these. Both sides

Resolution: Drive the deep fixtures through the production topology: add tests calling `reconcile_symmetric_accepts::<()>` on `leaf_parent_dispute_pair()` and `leaf_parent_redaction_pair()` (their third element is the oracle; also compare to `reconcile_locally`), widen `full_depth_comb_pair`/`pyramid_pair` to `pub(crate)` or move them beside the leaf-parent fixtures and drive them too, and add a proptest over `arb_deep_divergent_pair()` at this tier. Re-denominate the `wire_reconciliation_matches_local` doc to the shapes it samples, or add the deep generator to it. Acceptance: a test in this partition, driving `RemoteHandshaking` over a link, observes frames on a logical stream with index >= 2 in each direction (via `IoReport.connects >= 3` per side, or a hook capture) with reconciled roots equal to the oracle; the leaf-parent dispute and redaction fixtures both converge over the wire; a wire snapshot renders a stream header deeper than stream 1.
Construction: `let (a, b, union) = leaf_parent_dispute_pair(); let (l, r) = run_to_quiescence(reconcile_symmetric_accepts::<()>(a, b, TRANSPORT_CAPACITY)).expect(..); assert_eq!((l, r), (union.clone(), union));` If it passes, commit it and the gap is closed; if it fails, a wire-only defect has been found. Outcome (second witness pass): it fails, and neither branch applies; the Witness paragraph below explains.

Witness: the second witness pass ran this construction and a frame census beside it (`witness/results.md`, `## remote-proxy-tests-10`). Two tests appended to proxy/tests.rs called the unmodified `reconcile_symmetric_accepts::<()>` on `leaf_parent_dispute_pair()` and `leaf_parent_redaction_pair()` after first checking each fixture against `reconcile_locally`; a third, diagnostic test attached a crate-private observer that counts frames per sent data-stream index and ran the content-addressed baseline (24 leaves a side) and both deep fixtures at `TRANSPORT_CAPACITY` and at 64 KiB. The committed snapshots, grepped mechanically, name only `stream 0 (height 31)` (22 times), `stream 1 (height 29)` (twice), and `stream 1 (height 30)` (seven times). Decisive output:

    WITNESS remote-proxy-tests-10 [content-addressed baseline, capacity 37]: frames per sent stream index: side A {0: 19, 1: 27}; side B {0: 43, 1: 5}; highest stream index opened = 1
    WITNESS remote-proxy-tests-10 [leaf_parent_dispute_pair, capacity 37]: frames per sent stream index: side A {1: 2, 2: 2, 3: 2, 4: 2, 5: 2, 6: 2, 7: 2, 8: 2, 9: 2, 10: 2, 11: 2, 12: 2, 13: 2, 14: 2, 15: 2, 16: 4}; side B {0: 2, 1: 2, 2: 2, 3: 2, 4: 2, 5: 2, 6: 2, 7: 2, 8: 2, 9: 2, 10: 2, 11: 2, 12: 2, 13: 2, 14: 2, 15: 2}; highest stream index opened = 16
    WITNESS remote-proxy-tests-10 [leaf_parent_dispute_pair, capacity 37]: side A Err(Server(Stream(SupplyClosed { origin: Stream { speaker: Responder, stream: Stream(16) }, source: Some(Custom { kind: UnexpectedEof, error: "peer link is gone" }) }))); side B Err(Server(Decode(LeafOutsideScope { expected: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], actual: [84, 185, 194, 209, 144, 126, 178, 114, 158, 214, 57, 99, 201, 76, 40, 254, 51, 214, 176, 221, 37, 187, 136, 235, 47, 123, 221, 17, 135, 50, 207, 39] })))
            FAIL [   0.016s] (2/3) rumors tree::mirror::streaming::remote::proxy::tests::zz_witness_leaf_parent_dispute_pair_over_wire
            FAIL [   0.016s] (3/3) rumors tree::mirror::streaming::remote::proxy::tests::zz_witness_leaf_parent_redaction_pair_over_wire
         Summary [   0.069s] 3 tests run: 1 passed, 2 failed, 838 skipped

Verified by running: the content-addressed baseline opens only stream indices 0 and 1 on either side, matching the committed snapshots; both deep fixtures pass the in-process oracle and open streams 0..=16 over the link, so the deep tiers are reachable in principle; over the wire both fail, at either capacity, with the receiving side's primary error `Decode(LeafOutsideScope ..)` (the other side's `SupplyClosed` follows the failed side dropping its link). Assessed by reading in the same pass: `LeafOutsideScope` is raised at src/tree/mirror/streaming/remote/adapter/decode.rs:503-512, where the receiver derives the leaf's path with `Path::for_leaf(version)` and checks it against the reply scope, and the fixtures place leaves at `leaf_sibling_path` (src/tree/arb.rs:503-513), paths no version-addressed leaf can have. So the construction's failure is neither of the branches it names: the fixtures violate the version-addressing invariant the wire relies on, the wire correctly refuses them, and closing the gap needs a different vehicle, either a content-addressed deep divergence found by hash-prefix search (reachable only to modest depth) or a codec and stream-tier harness that feeds deep-indexed frames directly. The edits were restored afterwards.

Cross-reference: the same gap at the snapshot tier is tests-wire-format-7 (no committed `.snap` carries a stream header deeper than `Responder stream 1 (height 29)`) and at the legibility tier tests-wire-format-18 (the corpora cannot produce a three-byte shared prefix on both sides).

### remote-proxy-tests-7: The backend-fault property discards the unfaulted endpoint's tree, so a divergent completion passes
- Where: src/tree/mirror/streaming/remote/proxy/tests.rs:296-300 (related: tests.rs:445-499, 219-224; tests/failures.rs:244-253)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read the helper and the property: the only uses of `result.0`/`result.1` are error matching and `is_ok()`; the transport dual at failures.rs:244-253 compares any `Ok` counterparty to the oracle)
- Seen by: blind-spots; refutation: confirmed; history: no rationale (`left.map(|_| ())` is from cbfe1aff; 6410212a added the transport dual's stronger check and did not revisit this property)
- Owner-gated: no

`reconcile_with_stacked_failures` maps both results to `()`, so `proxy_backend_failures_are_fail_fast` checks only the faulted side's error identity plus quiescence. An unfaulted counterparty that completes `Ok` with a tree different from the oracle is indistinguishable from a correct one, and a backend failure injected after the counterparty absorbed a partial supply run is exactly the moment a wrong-but-complete tree could emerge. The doc's "terminates both endpoints" is pinned only by liveness; the mirror's contract that an `Ok` endpoint returns the reconciled root is not pinned here at all, while its transport sibling pins it.

Evidence:

       296	    let (left, right) = join!(Box::pin(mirror(a, remote_b)), Box::pin(mirror(remote_a, b)));
       297	    (
       298	        (left.map(|_| ()), right.map(|_| ())),
       299	        if fail_left { a_io } else { b_io },
       300	    )

       445	    /// Every reached proxy backend failure terminates both endpoints and
       446	    /// survives transport cancellation with its exact operation identity.

Resolution: Return the roots from the helper (`left.map(|(root, _)| ..)`, `right.map(|(_, root)| ..)`), adding the inverse of `failing_root` (tests.rs:219-224) to convert `Root<Failing<Local>>` back to `TreeRoot`; compute `reconcile_locally(a, b)` in the property and assert any `Ok` result equals the oracle's side, as failures.rs:247-253 does. `stacked_backend_and_transport_failures_remain_distinct` can keep discarding at its own call sites. Acceptance: `proxy_backend_failures_are_fail_fast` contains a `prop_assert_eq!` against the local oracle for whichever endpoint returns `Ok` when the injection fired, and the helper's return type carries `TreeRoot`.
Construction: Wrap the unfaulted proxy's backend to substitute one leaf's payload after N operations while the faulted side fails; the current property passes because the tree is never compared.

### remote-proxy-tests-24: The harness arrangement puts the right proxy in the Client position, which production never does
- Where: src/tree/mirror/streaming/remote/proxy/tests/harness.rs:610-613 (related: tests.rs:75, 296; tests/containment.rs:49, 68-69, 86-87, 100; tests/harness.rs:44-48; src/tree/mirror/streaming/remote/proxy/start.rs:112-114, 130-155, 157-197, 199-219; src/tree/mirror/streaming.rs:143-153, 156-172; src/peer/gossip.rs:1148-1152, 1207-1210; src/conformance/backend.rs:710-712)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`mirror(client, server)` at streaming.rs:143-146; `handshake` calls `client.connect()` then `server.accept()` at 165-168; production calls `streaming::handshake(local, proxy)` at gossip.rs:1152 and 1210, so the proxy is always the Server; `drive` runs `mirror(remote_left, right)`, as do tests.rs:75, 296 and containment.rs:49; the remote `Connect`/`CompleteConnect` impls at start.rs:130-197 have no non-test caller; the remote `Accept` runs `try_join(send, receive)` at 217-219)
- Seen by: blind-spots; refutation: confirmed; history: deliberate-and-holds for the impls' existence (cbfe1aff: "Expose symmetric materialized and remote Handshaking entry points"; streaming.rs:29-33 documents that the drivers run any two implementors; containment.rs uses the asymmetric arrangement on purpose to pin position-independence of the materialized participant's violation report, and says so at the check site) but not for the coverage distribution
- Owner-gated: yes: dissolving `Connect`/`CompleteConnect`/`Connecting` on the remote `Handshaking` reverses cbfe1aff's design; adding the production arrangement to the harness is not gated

Every transport-adversity, malformed-frame, declaration, and greeting session runs its right endpoint as `mirror(remote_left, right)`: the proxy is the protocol Client, running `Connect` (receive the greeting first) and `CompleteConnect` (then send), a sequence production never executes, while the production-shaped concurrent `try_join(send, receive)` exchange under chunked, delayed, or rewritten control I/O runs on only one endpoint per session. The hub already names the production shape (`reconcile_symmetric_accepts`, "Drive the production topology") but only its own six tests use it. The `RightError`/`MirrorError::Client(proxy error)` arms in `receiving_error`, `endpoint_error`, and the declaration tests are test-only shapes. The asymmetric shape is a legal configuration of the generic protocol, so the tests are not wrong; the gap is that no adversity coverage runs production's wiring on both ends, and the remote `Connect` impl's only defender is containment.rs's server-position check.

Evidence:

       610	    let (left, right) = join!(
       611	        Box::pin(mirror(left, remote_right)),
       612	        Box::pin(mirror(remote_left, right)),
       613	    );

    src/tree/mirror/streaming.rs:
       165	    let (our_handshake, client) = client.connect().await.map_err(Error::Client)?;
        ...
       168	    let (peer, server) = server.accept(our_handshake).await.map_err(Error::Server)?;

    src/peer/gossip.rs:
      1152	            let handshaken = streaming::handshake(local, proxy)

Resolution: Ungated: give `drive` a topology axis (coupling with remote-proxy-tests-5) and make the production arrangement (`mirror(right, remote_left)`, mapping `right.map(|(root, _control)| root.into())`) the default for the transport, malformed, declaration, and greeting suites; keep the asymmetric arrangement for containment.rs, whose reason is documented at the check site; collapse the Server/Client projection accordingly (remote-proxy-tests-16). Gated: decide whether `impl Connect for Handshaking`, `impl CompleteConnect`, and `Connecting` in start.rs stay for containment's server-position wire check (an impl with no production caller, kept for one test) or go, with the in-process twin `uncontained_supply_is_rejected_by_streaming` carrying position-independence. Acceptance: no adversity, script, or rewrite test constructs `mirror(<RemoteHandshaking>, <materialized>)`; `grep -rn 'MirrorError::Client(' src/tree/mirror/streaming/remote/proxy` matches only materialized-side errors and containment.rs; the owner has recorded the ruling on the remote `Connect` impl.

### remote-proxy-tests-26: The `Accept` arm of `Work::execute` is never resolved through `execute`
- Where: src/tree/mirror/streaming/remote/proxy/work/tests.rs:28-38 (related: work/tests.rs:41-75; src/tree/mirror/streaming/remote/proxy/work.rs:215-233; src/tree/mirror/streaming/remote/streams.rs:796-825; src/tree/mirror/streaming/remote/streams/tests.rs)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`grep -rln 'AcceptError\|Error::Accept('` over test files hits only streams/tests.rs, at the driver; work.rs:221-225 skips the post-select accept poll for `Ok(_) | Err(Error::Accept(_))`; the five tests in work/tests.rs resolve only the protocol and stream-error arms)
- Seen by: blind-spots; refutation: confirmed; history: no rationale (the arm and its comment date from 54420d7f, whose witnesses concern the deposit-versus-consequence race; no follow-up added an accept-arm witness)
- Owner-gated: no

`execute` carries a `match` arm whose stated purpose is to avoid re-polling a completed accept driver, and no test can provoke that path: a regression that polled the completed driver (an `async fn` resumed after completion panics) or let the accept arm's violation be outranked by a deposited supply failure would be caught by nothing at the session terminal. `AcceptError`'s variants (`Epoch`, `UnknownStream`, `Label`, `Duplicate`, `Unexpected`) are pinned only at the driver. A guard is justified by a concrete, constructible failure the committed tests cannot catch; the `ParkedSession` fixture already holds the live `peer: MemoryLink` needed to construct one.

Evidence:

        28	/// the way the session wires it, with everything the tests must keep alive.
        29	struct ParkedSession {
        30	    work: Work<Failing<Local>, DuplexStream, DuplexStream, MemoryAcceptor>,
        31	    /// The claim table the protocol owns in production. Keeping it alive
        32	    /// ensures no stream-layer closure can accidentally win the error race.
        33	    claims: Claims<DuplexStream>,
        34	    /// A publishing half of the error route, for tests that report to it.
        35	    route: ErrorRoute,
        36	    /// The peer link; dropping it would close the stream supply.
        37	    peer: MemoryLink,
        38	}

    work.rs:
       221	            match &outcome {
       222	                // A violation resolved the accept arm: the driver is
       223	                // complete and must not be polled again, and a violating
       224	                // driver never deposited (it returns instead of parking).
       225	                Ok(_) | Err(Error::Accept(_)) => {}

Resolution: Add a test to work/tests.rs: from `peer.into_parts()`, `connector.connect()` a stream and write a label pair whose epoch item disagrees with `session.epoch()` (or whose stream item is `>= Stream::COUNT`), then `run_to_quiescence(work.execute(future::pending()))` and assert `Err(Error::Accept(AcceptError::Epoch { .. }))` (or `UnknownStream`) with the exact values and no panic. Optionally a full-stack dual via a label mutation in `ScriptedWrite` (it already parses past the two label items) reaching `MirrorError::Server(RemoteError::Accept(_))` in malformed.rs. Acceptance: a test in work/tests.rs whose asserted outcome is `Error::Accept(_)` from `execute`.
Construction: In `parked_session()`'s peer link: connect, write two canonical unsigned-int heads with the wrong epoch, flush, then execute; expect `Error::Accept(AcceptError::Epoch { .. })`.

### remote-proxy-19: The proxy ordering trace has no liveness floor: an empty trace satisfies both assertions
- Where: src/tree/mirror/streaming/remote/proxy/work/progress/trace.rs:24-55 (related: src/tree/mirror/streaming/remote/proxy/work/progress/trace.rs:79-109, src/tree/mirror/streaming/remote/proxy/work/progress/trace.rs:188-194, src/tree/mirror/streaming/remote/proxy/work/progress.rs:10-17, src/tree/mirror/streaming/remote/proxy/tests.rs:402, src/tree/mirror/streaming/remote/proxy/tests.rs:426, src/tree/mirror/streaming/remote/proxy/tests.rs:597-606, src/tree/mirror/streaming/remote/proxy/tests.rs:610-624, src/testing.rs:375-394)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read: `Trace`'s field is private and has no length accessor; both assertions iterate `self.0` and their trailing asserts quantify over empty maps, so both pass on an empty vector; `record` pushes only when the thread-local is `Some`, i.e. on the `with_trace` caller's thread; grep of proxy/tests.rs shows the only consumers are `assert_valid` at 402 and 599 and `assert_registration_causality` at 426, with no floor; the channel instrument beside them does have one at 602-606. The refutation pass ran the three consuming tests at PROPTEST_CASES=1024: all pass, which shows the trace is populated today and says nothing about the floor.)
- Seen by: correctness; refutation: confirmed; history: no-rationale-found (the asymmetry with the channel floor is original to cbfe1aff3)
- Owner-gated: no

The three tests that consume the trace certify wire-before-question and registration causality only as long as `run_to_quiescence` happens to poll on the calling thread and `Progress::new`/`record` stay wired; if either drifts, the pins pass vacuously. Doctrine: meters need liveness floors; an ordering check over a counter passes when the counter stops counting. The cheapest artifact that passes these assertions today is a `Progress` that records nothing.

Evidence:

    24	/// A completed positive session's proxy-ordering trace.
    25	#[derive(Debug)]
    26	pub struct Trace(Vec<Event>);
    27	
    28	impl Trace {
    29	    /// Assert wire-before-question and reply-before-scope ordering.
    30	    pub fn assert_valid(&self) {
    31	        let mut questions = BTreeMap::<(usize, usize), usize>::new();
    32	        let mut scopes = BTreeMap::<(usize, usize), usize>::new();
    33	        for (index, event) in self.0.iter().enumerate() {

    188	pub fn record(work: usize, kind: Kind, height: usize) {
    189	    EVENTS.with(|events| {
    190	        if let Some(events) = events.borrow_mut().as_mut() {
    191	            events.push(Event { work, height, kind });

    tests.rs:601	    for kind in QueueKind::PROXY {
    tests.rs:602	        assert!(
    tests.rs:603	            report.kind(kind).channels > 0,

Resolution: give `Trace` a floor every divergent two-proxy session must meet and call it beside the channel floor in `instrumented_channels_cover_every_proxy_edge` and in the two proptests: for example `assert_covers_divergent_session()` requiring exactly one `DecodedReply` at `UnderRoot::HEIGHT` (the initiator-side proxy's greeting-seeded opening, `Work::initiator`, pump.rs:82-86) and exactly one `LocalQuestion` at `UnderRoot::HEIGHT` (the responder-side proxy's opening publication, encode.rs:142-143) across the trace, plus at least one `WireReply`. Commit the known-bad demonstration: `with_trace(|| ())` yields a trace the floor rejects. Acceptance: a `should_panic` test builds `Trace` from `with_trace(|| ())` and fails the floor with a named message; the three consuming tests call the floor and still pass; deleting the `progress.decoded_reply` call in `Work::initiator` fails at least one committed test.

Construction: `let (_, trace) = with_trace(|| ()); trace.assert_valid(); trace.assert_registration_causality();` passes today, which is the demonstration that the checks are vacuous. Add the floor and assert this construction panics.

### remote-proxy-31: Nine proxy error arms have no committed test that fires them
- Where: src/tree/mirror/streaming/remote/proxy/work/pump.rs:479-521 (related: src/tree/mirror/streaming/remote/proxy/work/encode.rs:60, src/tree/mirror/streaming/remote/proxy/work/encode.rs:67, src/tree/mirror/streaming/remote/proxy/work/encode.rs:96, src/tree/mirror/streaming/remote/proxy/work/encode.rs:139-140, src/tree/mirror/streaming/remote/proxy/work/encode.rs:170, src/tree/mirror/streaming/remote/proxy/work/encode.rs:210, src/tree/mirror/streaming/remote/proxy/tests/malformed.rs:164-184, src/tree/mirror/streaming/remote/proxy/tests/harness.rs, .agent-notes/2026-08-21-cbor-wire-mutants-scopeA/03-proxy-adapter.md)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (grep over src, tests, benches, examples: `UnansweredRemoteQuery`, `MissingOpening`, `OpeningEncode`, `ExtraOpening`, `UnaskedLocalReply` appear only at their encode.rs construction sites; `TerminalQuery` only at encode.rs:67 and pump.rs:394; the one committed proxy `UnaskedReply` assertion, `duplicated_reply_is_rejected_as_unasked`, lands on `reject_extra` at pump.rs:536, not the three `Early` arms)
- Seen by: correctness; refutation: confirmed; history: already-known in part (the scope-A note records the `Early::finish` arms and the `terminal` guard as surviving mutants with designed dispositions: an `Early` pairing property after a module split plus one full-proxy wire witness, and a scripted-channel contract family over `encode::terminal`; nothing has landed since 2026-08-21)
- Owner-gated: no

The early-supply cursor rejects a nonconforming initiator on three arms (a supply group behind the request cursor, a leftover lookahead at `finish`, an unread group at `finish`), and the encoders reject a nonconforming local participant on six. No test reaches any of them. These are conformance-bug detectors, not a security boundary, but the three cursor arms are the only detectors for an initiator whose early set disagrees with the responder's requests, and a detector no test fires is one whose drift nothing would notice. Doctrine: every criterion needs a committed demonstration that a known-bad mechanism fails it.

Evidence:

    479	                // Behind the request cursor: this group was never asked
    480	                // about at the root, so nothing will ever absorb it.
    481	                return Err(Error::UnaskedReply);
    511	    async fn finish(&mut self) -> Result<(), Error<B::Error>> {
    512	        if self.lookahead.is_some() {
    513	            return Err(Error::UnaskedReply);
    514	        }
    515	        if let Some(supplies) = &mut self.supplies
    516	            && !self.exhausted
    517	            && supplies.next().await.transpose()?.is_some()
    518	        {
    519	            return Err(Error::UnaskedReply);
    520	        }

    encode.rs:210	        return Err(Error::UnaskedLocalReply);

Resolution: remote-reachable group (pump.rs:481, 513, 519): land the scope-A note's `Early` pairing property (for ascending radix sets R requested and S supplied, `advance_to` over R against a scripted supply stream of S then `finish` yields each r answered iff r in S and ends in `UnaskedReply` iff S is not a subset of R), plus one full-proxy wire witness: extend the harness's greeting rewrite to drop one radix from the responder's listing as the initiator hears it, so the initiator early-ships a radix the responder never requests; place the orphan below a requested radix for the behind-cursor arm, as the highest early radix with a request following for the leftover-lookahead arm, and with no request after it for the unread-group arm. Local-only group (encode.rs sites): a `mirror(scripted_participant, proxy)` where a scripted in-process participant yields one reply too many, one too few, an opening that is not a query, or a terminal reply that asks a question; or, if these are judged unreachable by construction of the crate's own walk, say so at each site. Acceptance: each listed arm is reached by a committed test asserting the exact variant, or carries a comment stating why it is a local-programmer-error detector with no constructible peer input.

Construction: in `proxy/tests/harness.rs`, add a greeting rewrite that drops a chosen root radix `r` from the responder's greeting as received by the initiator, with `r` held by both trees; run the rewritten reconciliation; the initiator's `encode::opening` computes `early = true` for `r` and ships it, the responder never requests `r`, and the responder-side proxy's `Early` returns `UnaskedReply` from whichever arm the radix ordering selects. Assert the receiving endpoint's error is `RemoteError::UnaskedReply` and select the arm by choosing `r` relative to the requested radices.

### remote-proxy-tests-25: `bytes_past_the_stream_end_are_never_read` cannot distinguish "never read" from "read and tolerated"
- Where: src/tree/mirror/streaming/remote/proxy/tests/malformed.rs:186-196 (related: malformed.rs:212-216; tests/harness.rs:278-281; src/tree/mirror/streaming/remote/streams.rs:478-492; src/link.rs:171-199)
- Class / severity / confidence: test-quality / medium / medium
- Provenance: assessed (read: after `script.fired()` the only assertions are both sides `Ok`; the receiver breaks at `End(Stream)` and hands the half back at streams.rs:478-492; `ScriptedConnector::connect` discards the inner `Done`; the refutation pass found no test at any tier asserting the returned half's remaining bytes)
- Seen by: blind-spots; refutation: confirmed; history: no rationale (4be4b830, a work-in-progress snapshot, rewrote a test asserting the typed `AfterEnd` rejection into this liveness-only form without saying why)
- Owner-gated: no

The doc's first clause, "a duplicated stream-end frame is never read", has no assertion behind it: a receiver that read past the end control and ignored a second `End(Stream)` would also complete `Ok`. The invariant matters, as the doc says: on a reusing link those bytes are the next stream's header, so "read and tolerated" is a bug the test would bless, and the link contract's completion clause (`Done` is invoked "exactly at the protocol's end of the data, handing the half back") has no test at any tier that inspects what remains in the handed-back half. The implementation is right today by reading; the test cannot tell.

Evidence:

       186	/// Bytes past a stream's end control belong to the transport, not the
       187	/// session: a duplicated stream-end frame is never read, and the session
       188	/// completes as if it were absent.
        ...
       195	fn bytes_past_the_stream_end_are_never_read() {
       196	    const STREAM_END_STATE: u8 = 9;

       212	        assert!(left_result.is_ok(), "left session failed: {left_result:?}");
       213	        assert!(
       214	            right_result.is_ok(),
       215	            "right session failed: {right_result:?}"
       216	        );

    harness.rs:
       278	    async fn connect(&self) -> io::Result<(Self::Tx, Done<Self::Tx>)> {
       279	        let (tx, _) = self.inner.connect().await?;
       280	        Ok((ScriptedWrite::new(tx, self.script.clone()), Done::discard()))
       281	    }

Resolution: Give the receiving side's acceptor a `Done` that captures the returned receive half (the contract hands it back resting at the frame boundary); after the session, read from the captured half and assert exactly the duplicated frame's bytes remain. Alternatively compose `wrap_link` under the scripted connector and assert the receiving side's `read_bytes` equals the sender's `write_bytes` minus the duplicate's length. Acceptance: the test asserts, by byte count or by reading the returned half, that the duplicated frame's bytes were not consumed by the session.
Construction: Temporarily make the receiver in streams.rs continue past `End(Stream)` and break on the second one; the current test still passes.

### remote-proxy-tests-2: The random-content greeting fuzz cannot reach the map decoder its doc claims to exercise
- Where: src/tree/mirror/streaming/remote/proxy/start/tests.rs:186-194 (related: src/tree/mirror/streaming/remote/codec/greeting.rs:120-149)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (read of `parse_greeting`: the first head must be `MAJOR_MAP` with value exactly `KEYS.len()`, then each key's text head must match the fixed roster byte for byte; the distribution consequence is arithmetic on those checks)
- Seen by: blind-spots; refutation: confirmed and strengthened; history: deliberate-but-expired at 4dd2053c
- Owner-gated: no

The generator feeds `content in vec(any::<u8>(), 0..96)` into an honestly sized item. `parse_greeting` accepts only a map head whose value is exactly 6 (one byte value in 256), then requires the text head and the literal bytes of `"listing"` before any version, roster, or listing decoding runs. Across 256 cases about one reaches the first key check and none reach the version atom or the listing, so the property is a heads-only no-panic check while its doc promises coverage of "key roster, version atom, listing shape and order". The doc was carried over from the pre-CBOR fuzz (`4dd2053c`), where two raw bodies fed the bit codec and borsh listing decoder directly and random bytes did reach deep. A testdoc that overstates its reach is a bug in the test (AGENTS.md, Writing tests).

Evidence:

       186	    /// The item is honestly sized around arbitrary content, so the fuzz
       187	    /// lands on the map decoder (heads, key roster, version atom, listing
       188	    /// shape and order) rather than on the allocator via a lied length —
       189	    /// the head lies are pinned deterministically above. Every outcome
       190	    /// must be `Ok` or one of the three typed greeting errors.
       191	    #[test]
       192	    fn arbitrary_greeting_bodies_never_panic(
       193	        content in vec(any::<u8>(), 0..96),
       194	    ) {

    codec/greeting.rs:
       122	    let head = cbor::read_head(&mut input).map_err(GreetingError::Head)?;
       123	    if head.major != cbor::MAJOR_MAP || head.value != KEYS.len() as u64 {

Resolution: Generate from a valid `encode_greeting` output and apply drawn mutations (byte flips at drawn offsets, truncation, insertion, drawn listing entries), or draw `Greeting` fields structurally plus a key permutation; if the pure-random arm stays, narrow its doc to the heads it reaches. Acceptance: a sampled run of the property produces at least one `Error::HandshakeListing` and at least one `GreetingError` past the first head, and the doc names only what the generator reaches.
Construction: Temporarily tally the error variant per case; under the current generator the tally is `HandshakeDecode(Shape)` for every case that is not `Ok`, with no `HandshakeListing` and no version-atom error.

### remote-proxy-tests-8: Two session-driving tests run on pollster, so a stall would hang to the nextest kill instead of failing as `Stalled`
- Where: src/tree/mirror/streaming/remote/proxy/tests.rs:316-330 (related: tests.rs:345-355; src/testing.rs `run_to_quiescence`; .config/nextest.toml:25-26)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read; every other session in the partition goes through `run_to_quiescence`; nextest terminates after three 60-second periods)
- Seen by: api-economics; refutation: confirmed; history: no rationale (both were `#[tokio::test]` at cbfe1aff; 83edcd94, a work-in-progress commit, switched them to pollster while `run_to_quiescence` was already in use by siblings in the same file)
- Owner-gated: no

`equal_versions_return_both_roots` and `divergent_leaves_converge` are `#[pollster::test]` over `reconcile(..)`, a full two-proxy session on memory links. Under pollster a reintroduced stall parks the thread until nextest's 180-second termination and reports no cause; under `run_to_quiescence` it is `Err(Quiescence::Stalled)` with the stalled future named. The determinism argument in `.config/nextest.toml` is applied inconsistently to the two smallest sessions, the ones a stall regression reaches first. (`start/tests.rs`'s pollster uses read from `&[u8]` and cannot stall; no change is needed there.)

Evidence:

       316	/// Equal versions close every unused logical stream without opening descent.
       317	#[pollster::test]
       318	async fn equal_versions_return_both_roots() {

       328	/// Concurrent version-addressed leaves cross every proxy layer and converge.
       329	#[pollster::test]
       330	async fn divergent_leaves_converge() {

Resolution: Rewrite both as `#[test] fn ... { let (a, b) = run_to_quiescence(reconcile(..)).expect("..."); ... }`, matching `symmetric_accept_handshakes_are_live`. Acceptance: `grep -n pollster src/tree/mirror/streaming/remote/proxy/tests.rs` is empty.

### remote-proxy-tests-9: The preamble-then-session byte-isolation claim is tested only under a name and doc about payload types
- Where: src/tree/mirror/streaming/remote/proxy/tests.rs:357-371 (related: tests.rs:156-158)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read; `reconcile_after_preamble`'s sole caller is line 368)
- Seen by: api-economics; refutation: confirmed; history: no rationale (the shape is from 83edcd94; `nth_party` existed since 1af0ab9a)
- Owner-gated: no

`reconcile_after_preamble` documents itself as "proving that neither phase consumes the other's bytes", and its only caller is `symmetric_accepts_with_distinct_payloads_are_live`, whose name and doc speak only of payloads. A reader auditing preamble/session byte isolation will not find the test; a reader of the test will not know the preamble is under test. The test also builds its parties with `before::Party::seed()`/`fork()` where every sibling uses `nth_party`. A testdoc states the invariant the test protects and must be accurate.

Evidence:

       357	/// Distinct payloads exercise supplied-leaf paths different from the unit
       358	/// payload used by the broad protocol properties.
       359	#[test]
       360	fn symmetric_accepts_with_distinct_payloads_are_live() {
       361	    let mut a_party = before::Party::seed();
       362	    let b_party = a_party.fork();
        ...
       368	    let (a, b) = run_to_quiescence(reconcile_after_preamble::<u64>(a.root, b.root))

       156	/// Drive the production proxy topology after the shared preamble on the same
       157	/// transport halves, proving that neither phase consumes the other's bytes.

Resolution: Either split into a `preamble_and_session_share_the_control_halves` test (documenting the byte-isolation claim, unit payload) and run the payload test on `reconcile_symmetric_accepts::<u64>`, or fold the preamble claim into this test's name and doc. Use `nth_party(0)`/`nth_party(1)`. Acceptance: the doc of whichever test calls `reconcile_after_preamble` states the byte-isolation claim; no `Party::seed()` in the file.

### remote-proxy-tests-15: Hash-only convergence assertions never check the reconciled ceiling
- Where: src/tree/mirror/streaming/remote/proxy/tests/declarations.rs:33-43 (related: declarations.rs:164-165, 365-366, 393-394; tests/greeting.rs:20-23, 45-51, 63-64, 77-78, 137-143, 171-172, 203-204; tests.rs:335-340; src/tree.rs:113-117, 131-135, 274-278; src/tree/typed/node.rs:410-417; src/tree/mirror/streaming/tests.rs:157)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read `Tree::hash`, which hashes `Node::root_hash(&Option<Root>)` over the nodes alone after `From<Root> for Option<node::Root>` drops the ceiling; `Root: PartialEq` compares `self.ceiling == other.ceiling && self.root == other.root`; the hub's properties compare whole `TreeRoot` values against `reconcile_locally`)
- Seen by: blind-spots, api-economics, structure-prose (the duplicated helpers); refutation: confirmed, downgraded (the hub already pins the ceiling for arbitrary divergence at this tier); history: no rationale (`hash_of` was introduced at 0aa29ed9 when `Root: PartialEq` already compared the ceiling and `divergent_leaves_converge` beside it asserted whole-root equality; 739e4d1f copied the helpers into declarations.rs)
- Owner-gated: no

`hash_of` reduces a `tree::Root` to `Tree::hash()`, which covers the nodes only; the root ceiling rides outside the nodes and `Root: PartialEq` compares it separately. Seven fixture tests (greeting.rs: both `carried_listing_*`, `empty_carried_listing_asks_for_everything`, `converged_session_carries_listings_unused`, `mixed_empty_and_populated_converges`; declarations.rs: the three `overstated_*_still_converge*`) therefore pass if the session merges content correctly but returns a wrong ceiling. In `empty_carried_listing_asks_for_everything` the initiator's tree is empty and its version is not, so the one thing it brings to the session is the one thing not asserted. The ceiling is the deletion mechanism (redaction leaves no tombstones: a wrong ceiling mis-honors later redactions). `hash_of`/`union_hash` are also verbatim duplicates across the two files, and `union_hash` reimplements the private `streaming/tests.rs::join_oracle`. The hub's whole-root oracle already pins the ceiling for generic divergence, which is why this is low rather than medium.

Evidence:

        33	/// The observable root hash of a reconciled `tree::Root`.
        34	fn hash_of(root: &crate::tree::Root) -> [u8; MERKLE_HASH_LEN] {
        35	    Tree::<()>::from_root(root.clone()).hash()
        36	}
        37	
        38	/// The expected reconciled union, computed by the in-memory join oracle.
        39	fn union_hash(a: &crate::tree::Root, b: &crate::tree::Root) -> [u8; MERKLE_HASH_LEN] {
        40	    let mut union = Tree::<()>::from_root(a.clone());
        41	    union.join(Tree::from_root(b.clone()));
        42	    union.hash()
        43	}

    src/tree.rs:
       131	impl PartialEq for Root {
       132	    fn eq(&self, other: &Self) -> bool {
       133	        self.ceiling == other.ceiling && self.root == other.root
       134	    }
       135	}

Resolution: Add one `join_oracle(a: &TreeRoot, b: &TreeRoot) -> TreeRoot` to harness.rs (built on `Tree::join`, as tests.rs:335-340 does) and assert `assert_eq!(left, expected)` on whole roots in the seven tests; delete both `hash_of`/`union_hash` pairs. Acceptance: no convergence assertion in greeting.rs or declarations.rs goes through `Tree::hash()` alone; a deliberately wrong ceiling on one reconciled side fails the test.
Construction: Patch the proxy's equal or diverged completion to return the local pre-session ceiling instead of the merged one; `empty_carried_listing_asks_for_everything` still passes because both hashes equal `populated.hash()`.

### remote-proxy-tests-20: A `Tree::join` invariant is filed in the proxy greeting suite under a name that states the failure
- Where: src/tree/mirror/streaming/remote/proxy/tests/greeting.rs:206-234 (related: greeting.rs:1-8, 250, 254-256, 264, 271; src/tree/traverse/join/tests.rs:24-51; src/tree/traverse/join.rs:140; tests/session_overlap.rs:78)
- Class / severity / confidence: test-quality / low / medium
- Provenance: assessed (read; `join/tests.rs` holds only idempotence, commutativity, and associativity; `tests/session_overlap.rs:78` is the public end-to-end twin `overlapped_install_never_loses_innocent_messages`; the inertness of the wire calls follows from the doc's own statement at 254-256 and from equal-version sessions returning the local root, but I did not run the wire-free form)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed, downgraded (end-to-end coverage exists in tests/session_overlap.rs); history: no rationale for the tier (f5426c9d landed the test with the join fix; 4bbd6c5b deliberately preserved the body and assertions while reframing the doc, and kept the name and the `WITNESS` label, which appears nowhere else in the codebase)
- Owner-gated: no

The test asserts that joining a clone-derived fan (`ours.clone()` + `remove(r_h)`) against its original is an identity on the tree, and its own doc concludes "`Tree::join` merge-walks the two radix fans in lockstep ... this witness holds the join to that". The two `wire_reconcile` calls reconcile equal versions (returning the fork-time root handle, which `t0.root.clone()` also is) and `t0` against a redacting twin (equal to `join` by the design of record); the doc itself says "the sharing that matters is created by our install below, not here". So the proxy contributes nothing the assertion sees, the file's module doc scopes it to "the greeting-carried opening listing", a maintainer changing `join` would not look here, the name states the bug rather than the invariant, the `WITNESS (...)` header is a roster-style prefix ahead of the claim, line 234's `use crate::tree::Action;` shadows the import at line 13, and there is no blank line between 205 and 206.

Evidence:

       205	}
       206	/// WITNESS (the gossip install must merge-walk clone-derived fans):
       207	///
       208	/// Two honest, overlapping sessions at one peer must not silently delete a
       209	/// message nobody redacted.
        ...
       232	#[test]
       233	fn overlapping_sessions_lose_innocent_leaf_after_honored_redaction() {
       234	    use crate::tree::Action;

       254	        // S1's counterparty: converged at T0, then redacted the leaf at
       255	        // `k` (a local act rebuilds its own fans afresh; the sharing that
       256	        // matters is created by our install below, not here).

Resolution: Move it to `src/tree/traverse/join/tests.rs` as a unit test over `Tree::join` with the wire removed: `s2_reconciled` becomes `t0.root.clone()` and `s1_reconciled` becomes `Tree::from_root(t0.root.clone()).join(twin)`; keep the sweep over the redacted leaf and the mechanism paragraph (the desync-onto-neighboring-radixes explanation is the valuable part); rename to the invariant (`join_of_own_causal_past_is_identity_on_clone_derived_fans`); drop the `WITNESS` header so the first sentence is the claim; delete the inner `use`; consider stating it as a proptest over fan width and redacted leaf. Acceptance: greeting.rs contains only greeting-listing tests; join/tests.rs holds the clone-derived-fan identity test with no proxy imports; reintroducing a shortcut walk over shared runs (the shape f5426c9d removed) still fails it.
Construction: Copy the body without `wire_reconcile` as above; the assertion at 282-286 must still hold, and the reverted join must still fail it.

### remote-proxy-tests-21: Signal state codes are hand-copied into the harness and the malformed suite because `Signal` is not re-exported
- Where: src/tree/mirror/streaming/remote/proxy/tests/harness.rs:38-42 (related: harness.rs:185-190; tests/malformed.rs:95, 196; src/tree/mirror/streaming/remote/codec/signal.rs:235-245, 249, 261; src/tree/mirror/streaming/remote/codec.rs:73, 101-103; src/tree/mirror/streaming/remote/codec/signal/tests.rs:138)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read `Signal::STATES`: Match Continue/End = 0/1, QueryEmpty = 2/3, Query = 4/5, Supply = 6/7, End(Reply) = 8, End(Stream) = 9; `Signal::state` and `from_state` are `pub fn`; `mod signal;` is private at codec.rs:73 and the re-export list at 101-103 omits `Signal`; `state_roster_snapshot` pins the roster)
- Seen by: structure-prose, blind-spots, api-economics; refutation: reframed (the codes are wire-format constants pinned by the roster snapshot, so a renumbering fails loudly crate-wide; the `State(0)`/`State(9)` tests would also fail loudly, since their outcomes depend on the mutation's meaning; only `FrameSelector::Query`/`EndingReaction` could select different frames without their own failure); history: no rationale (3327a92b rewrote this layer to the two-item opener, kept the literals, and did not add `Signal` to the re-export list)
- Owner-gated: no

`QUERY_STATES = 4..=5`, `REACTION_STATE_COUNT = 8`, the `state % 2 == 1` ending-reaction rule, `MATCH_CONTINUE_STATE = 0`, and `STREAM_END_STATE = 9` restate `Signal::STATES` by hand. They are correct today and a renumbering is an owner-ruled format change the snapshot would catch, so the cost is legibility and single-source: a frame selector that names `Signal::Query(Flow::Continue)` says what it is, and the `Query`/`EndingReaction` selectors would otherwise retarget silently under a roster change (a `script.fired()` witness would still hold). Doctrine: a number that matters lives in one mechanically-enforced place that prose may cite by name. The resolution as the first two lenses wrote it does not compile: `Signal` is unreachable from this partition until codec.rs re-exports it.

Evidence:

        38	/// Dense states occupied by the two nonempty-query flow variants.
        39	const QUERY_STATES: RangeInclusive<u8> = 4..=5;
        40	
        41	/// Dense states below this boundary carry reactions rather than bare ends.
        42	const REACTION_STATE_COUNT: u8 = 8;

       185	        let selected = match script.selector {
       186	            FrameSelector::First => true,
       187	            FrameSelector::State(expected) => state == expected,
       188	            FrameSelector::Query => QUERY_STATES.contains(&state),
       189	            FrameSelector::EndingReaction => state < REACTION_STATE_COUNT && state % 2 == 1,
       190	        };

    codec.rs:
       101	pub use signal::{
       102	    DecodeSignalError, End, Flow, InvalidSignalPlacement, Speaker, Stream, StreamClass,
       103	};

Resolution: Add `#[cfg(test)] pub use signal::Signal;` to codec.rs; in the harness select by `Signal::from_state(state)` (`Query` matches `Ok(Signal::Query(_))`, `EndingReaction` matches `Ok(Signal::Match(Flow::End) | Signal::QueryEmpty(Flow::End) | Signal::Query(Flow::End) | Signal::Supply(Flow::End))`); in malformed.rs write `Signal::Match(Flow::Continue).state()` and `Signal::End(End::Stream).state()`; delete the four local constants. Acceptance: no numeric state literal remains in `proxy/tests/**`; the malformed suite passes with every `script.fired()` assertion holding.

### remote-proxy-tests-22: The greeting rewriter carries no fired witness, so the three still-converge tests pass if the rewrite never applies
- Where: src/tree/mirror/streaming/remote/proxy/tests/harness.rs:284-297 (related: harness.rs:394-446; tests/declarations.rs:147-167, 350-368, 375-396)
- Class / severity / confidence: test-quality / low / medium
- Provenance: verified (read: `GreetingRewrite`/`RewriteRead` expose nothing like `Script::fired()`; the three overstated tests assert only convergence, which an unrewritten session also yields)
- Seen by: api-economics; refutation: confirmed (narrow gap: the understated tests share the rewriter and fail loudly without it); history: no rationale (739e4d1f added the rewriter into a harness whose `Script` already had `fired()`)
- Owner-gated: no

`overstated_target_message_size_still_converges`, `overstated_version_bytes_still_converge`, and `overstated_set_len_from_the_bulk_side_still_converges` assert convergence; the cheapest passing artifact for them is "rewrite nothing". The understated tests exercise the same rewriter and would fail loudly if it did nothing, so the residual gap is a `u64::MAX`-specific failure to apply; still, Principle 6 asks every check to name what fails it, and the harness's other decorator already models the answer.

Evidence:

       284	/// One greeting size declaration replaced in the traffic a side receives.
       285	///
       286	/// Rewriting the *received* greeting simulates a buggy counterparty whose
        ...
       292	#[derive(Clone, Copy)]
       293	pub struct GreetingRewrite {
       294	    field: GreetingField,
       295	    /// The declaration the receiving side decodes instead of the honest one.
       296	    value: u64,
       297	}

Resolution: Give `RewriteRead` a shared `Arc<AtomicBool>` set when it enters `RewriteState::Serving` with a rewritten item, return it from `reconcile_rewritten_greetings`, and assert it in the three convergence tests (and, for symmetry, the failing ones). Acceptance: making `GreetingRewrite::apply` return its input unchanged fails all seven declaration tests.

### remote-proxy-20: The trace ledgers are keyed by (endpoint, height), which two live recorders share at height zero
- Where: src/tree/mirror/streaming/remote/proxy/work/progress/trace.rs:35-45 (related: src/tree/mirror/streaming/remote/proxy.rs:31-32, src/tree/mirror/streaming/remote/proxy/work/pump.rs:161-162, src/tree/mirror/streaming/remote/proxy/work/pump.rs:259-268, src/tree/mirror/streaming/remote/proxy/work/pump.rs:318-326, src/tree/mirror/streaming/remote/proxy/work/pump.rs:396, src/tree/mirror/streaming/remote/proxy/work/encode.rs:63, src/tree/mirror/streaming/remote/proxy/work/encode.rs:99, src/tree/mirror/streaming/remote/proxy/work/progress/trace/tests.rs:57-66)
- Class / severity / confidence: test-quality / low / medium
- Provenance: assessed (read: on the responder-side proxy the `Descending<2>` decode pump records `decoded_reply(H::HEIGHT = 0, n)` and `terminal_decode_pump` records `decoded_reply(Z::HEIGHT, 0)` under the same `work`; on the initiator-side proxy `leaf_replies`' encoder records `wire_reply(Z::HEIGHT, ..)` beside `complete_initiator`'s `encode::terminal`; `yield_reply_scopes!` records before the `yield`, which suspends through the relay's `send_or_cancel`. The refutation pass ran the three trace-consuming tests at PROPTEST_CASES=1024: all pass.)
- Seen by: correctness; refutation: reframed (the `scopes` ledger's window opens at a single derived height-0 scope, not two; reaching it needs a dispute nested through every schedule stage, which the trace-asserting generators cannot produce); history: no-rationale-found; the collision-schedule test mode note (2026-08-21) would make the interleaving reachable
- Owner-gated: no

`assert_drained` requires the previous entry at `(work, height)` to be fully published before a new `WireReply`/`DecodedReply` at the same key. At height zero two recorders per endpoint are live concurrently, so a legitimate interleaving (the leaf-parent stage records a decoded reply with one derived leaf scope, yields, and before the scope publishes the terminal records its own decoded reply) trips the ledger as a false positive. It cannot occur under the committed generators because a leaf-height question needs two leaves sharing a 31-byte hash prefix, so the check is sound only by hash uniformity, and the premise is unstated. The committed negative test `rejects_next_decoded_reply_before_scopes` records exactly the event sequence that interleaving would produce. When the planned collision-schedule test mode lands and the terminal stage carries real work, this instrument starts failing on correct sessions.

Evidence:

    35	                Kind::WireReply { questions: count } => {
    36	                    assert_drained(&questions, event, index, "questions");
    37	                    questions.insert((event.work, event.height), count);
    38	                }
    39	                Kind::LocalQuestion => consume(&mut questions, event, index, "wire reply"),
    40	                Kind::DecodedReply { scopes: count } => {
    41	                    assert_drained(&scopes, event, index, "scopes");
    42	                    scopes.insert((event.work, event.height), count);
    43	                }

    proxy.rs:31	        $progress.decoded_reply($height, $count);
    proxy.rs:32	        $yielded;
    pump.rs:396	                progress.decoded_reply(Z::HEIGHT, 0);
    pump.rs:162	        let responses = self.decode_pump(questions, incoming, next_scopes, early, H::HEIGHT);

Resolution: key the ledgers by stage: add a stage tag to `Kind::WireReply`/`DecodedReply`, or record the terminal encoder and decoder under a distinct label; alternatively state the one-recorder-per-key premise and the prefix-collision argument at the ledger. Keying by stage is preferable because the premise is scheduled to expire. Acceptance: two concurrent recorders never share a ledger key, or the premise is stated at the ledger with the argument.

Construction: in trace/tests.rs, the sequence `record(0, DecodedReply { scopes: 1 }, 0); record(0, DecodedReply { scopes: 0 }, 0); record(0, NextScope, 0);` is what a correct responder-side proxy produces when its height-2 decode derives one leaf scope and the terminal decode records between the yield and the scope publication; `assert_valid` rejects it. Reaching it end to end needs a pair of trees disputing down to height 1, i.e. two leaves under one 31-byte prefix, which the collision-schedule mode is designed to construct.

## Test scaffolding (src/testing, src/tests)

The stall detector is exemplary and under-used by its own file; two decorators (`ReorderingAcceptor`'s inversion, `Quiescence::PollBudget`) have no demonstration that they fire; the absorber's party is never read after a failed hand-off; and the CBOR respelling of the wire moved the constants but not the comments beside them.

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

Resolution: In `overlapping_retiree_party_is_rejected`, borrow the survivor into the future (`let survivor = &survivor; async move { survivor.gossip(&mut b_link).await }`) and assert `party_of(&survivor) == Party::seed()` afterwards (never forked, and the overlapping join must leave it untouched). In `severed_epilogue_marker_is_uncertain`, replace `drop(survivor)` with `assert_eq!(party_of(&survivor), Party::seed(), "the absorber committed the donated region before its epilogue failed")`. In `severed_party_frame_is_uncertain`, capture `let before = party_of(&survivor);` ahead of the session and assert equality afterwards (the frame never arrived). State each absorber-side post-condition in the test doc. Acceptance: all three tests read the absorber's party after the failed session; each doc names the absorber-side invariant; the tests still pass.
Construction: Make the absorber's `existing.join(party)` at gossip.rs:820 a no-op on the success path and run `severed_epilogue_marker_is_uncertain`: it passes today, because nothing reads the survivor's party; with the proposed assertion it fails with the survivor's half-region instead of `Party::seed()`.

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

### testing-infra-4: `Quiescence::PollBudget` has no committed demonstration that it fires
- Where: src/testing.rs:376-393 (related: src/testing.rs:345-352, 400-417; .config/nextest.toml:1-5)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`grep -rn PollBudget src tests benches examples` returns only the variant at 351 and the return at 393)
- Seen by: blind-spots [23]; refutation: confirmed; history: no rationale found (the poller carried only `observes_wake_contract` from birth)
- Owner-gated: no

The runaway guard is the poller's second failure signal, and nothing in the repository observes it: `observes_wake_contract` covers `Ok` and `Stalled` only. A regression that made the loop unbounded, or returned `Stalled` from the guard, passes every test. Principle 6 (adequacy): a guard is justified by a committed demonstration that the failure it names is caught; this is the liveness instrument every protocol suite rests on (.config/nextest.toml:1-5).

Evidence:

    376	    const MAX_POLLS: usize = 1_000_000;
    ...
    383	    for _ in 0..MAX_POLLS {
    ...
    393	    Err(Quiescence::PollBudget)

Resolution: Add to the poller's tests: `assert_eq!(run_to_quiescence(std::future::poll_fn(|cx: &mut Context<'_>| { cx.waker().wake_by_ref(); Poll::<()>::Pending })), Err(Quiescence::PollBudget));` (a million trivial polls completes in milliseconds). While there, give `MAX_POLLS` a one-line sizing statement (the largest closed-world session the suites run, with headroom), since a legitimate long session exceeding it would be misreported. Acceptance: a test asserts `Err(Quiescence::PollBudget)` for a perpetually self-waking future.
Construction: Change line 393 to `Err(Quiescence::Stalled)` (or the loop bound to unbounded with a counter) and run the workspace tests: nothing fails today.

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

### Nits

The full record of each (provenance, history, evidence) is in the evidence file named in the last column.

| Id | Where | Claim | Resolution | Record |
|---|---|---|---|---|
| testing-infra-19 | `src/tests.rs:82-88` | two testdocs omit an assertion the body makes (`Retire::Uncertain`; `peer_out.is_err()`) | one clause each | `evidence/partitions/testing-infra.md` |
| testing-infra-23 | `src/tests.rs:446-471` | a cancellation-poisons-the-link pin uses the `pub(crate)` `Peer::gossip` though public `Rumors::gossip` reaches the claim | move to tests/lifecycle.rs via `into_rumors()` | `evidence/partitions/testing-infra.md` |

## Integration tests: tests/common

The shared harness never faults the donor side of a bootstrap or the absorber side of a retirement, samples cut offsets where a total sweep is a few thousand runs, re-derives `UnorderedMessages` instead of consuming it, transcribes two crate derivations `rumors::testing` could export, pins the proptest version it transcribes in prose, and models the overlap fork at `Open` where the live session forks at its first poll.

### tests-common-25: fault plans never fault the donor side of a bootstrap or the absorber side of a retirement
- Where: tests/common/sim.rs:490-519 (related: tests/common/sim.rs:118-122, tests/common/sim.rs:160-167, tests/common/sim.rs:330-333, tests/common/sim.rs:787-800, src/peer/gossip.rs:782-793, src/link.rs:532, tests/disruption.rs:909-923)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`grep -n 'FaultPlan::NONE' tests/common/sim.rs`: the serving side at 503 and the absorber at 798; `MEMORY_STREAM_CAPACITY = 8 * 1024` at src/link.rs:532; the donor's `party::send` error branch at src/peer/gossip.rs:782-793; a corpus grep of every `wrap_link`, `faulty_link`, and `fault::faulty` call under tests/ finds no donor- or absorber-side plan anywhere)
- Seen by: blind-spots; refutation: confirmed; history: deliberate-and-holds for the stated asymmetry (the joiner's plan "covers both observable directions"), which makes no claim about the donor's or absorber's own budget expiry
- Owner-gated: no

`run_boot` wraps the serving endpoint with `FaultPlan::NONE` and `run_plan` wraps the absorber the same way, so in every plan the donor of a party and the receiver of a whole party observe only their counterparty's death. The doc argues the joiner's read cut "models the server's frames dying in flight", but the joiner's read cut leaves the server's writes succeeding into the 8 KiB memory buffer; the donor's `party::send` error branch ("A lost fork merely leaks its region") is reached only when scheduling happens to drop the joiner before the server writes, never at a chosen offset and never mid-frame. `Session` carries `fault_a` and `fault_b` for gossip; the asymmetric sessions are the one place the endpoint-by-direction matrix is half empty. The donor and the receiver of a party run different code with different recovery semantics (guard snap-back before `party.take()`, leak after); a shape that wedges one side can look benign on its dual.

Evidence:

       491	/// The serving side stays clean; the joiner's fault plan covers both
       492	/// observable directions of a duplex (its read cut models the server's
       493	/// frames dying in flight). A joiner that fails may or may not have cost
       494	/// the server its donated fork, so it conservatively counts as a possible
       495	/// loss either way.
    ...
       503	        let mut link = fault::faulty(serve_side, FaultPlan::NONE);

       798	                let mut link = fault::faulty(absorber_side, FaultPlan::NONE);

    (src/peer/gossip.rs:782-787)
                match party::send(donated, write, &observe).await {
                    Err(e) => {
                        // A retiring donation in limbo must be assumed received:
                        // report `Intent::Retire` alongside the error so that the
                        // `Peer` is not handed back. A lost fork merely leaks its
                        // region; we remain.

Resolution: add a serving-side plan per boot entry and an `absorber: FaultPlan` to `RetireOp`, mirroring `Session`'s two plans. Draw the new fields after `windows`: sim.rs:330-333 states that draw order is the seed-compatibility surface, and appending keeps every committed disruption seed regenerating its existing prefix. Accounting: a donor whose session errors after `party::send` began is a possible loss (the newcomer may or may not hold the fork); one that errors before the donation is not (the guard rejoins). The absorber-side arms already exist (`absorbed.is_err()`), and finding 25 makes the `Retired`-with-absorber-`Err` arm reachable. Re-run `max_cut_spans_the_envelope_session`. Acceptance: a committed case in tests/disruption.rs runs a bootstrap whose server-side write budget expires inside the party frame and a retirement whose absorber-side read budget expires inside the party frame, both passing `assert_party_invariants` and the classifier; `grep -n 'FaultPlan::NONE' tests/common/sim.rs` no longer matches the serve and absorber wrap sites.

Construction: in tests/disruption.rs, build the seed-plus-fork fixture; meter one clean bootstrap with `fault::metered` to learn the server's write extent; run `run_boot` with a server plan `write_cut: Some(k)` for each `k` up to that extent (or the chosen `k` inside the party frame) and assert: the classifier accepts the server's error; afterward either the newcomer holds a party disjoint from the server's or the server's party equals its pre-donation party (fold-join), with the leak permitted only for errors after `party.take()`. Do the same for a retirement with an absorber `read_cut` swept over the absorber's read extent.

### tests-common-16: `Peer::drain` re-derives `UnorderedMessages`, so generated multi-peer schedules never drive the public observer
- Where: tests/common/peer.rs:63-75 (related: tests/common/peer.rs:6-14, src/rumors/unordered.rs:102-108, src/rumors/unordered.rs:217-219, src/rumors.rs:372)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (mechanism match verified by reading: unordered.rs:105 opens a pass with `range_owned(causally::since(checkpoint))` and 219 absorbs `checkpoint |= &ceiling`, the two steps `drain` performs; the swap's feasibility is assessed: `unordered_messages_since` exists and the stream holds a watch receiver, so `try_into_peer` is not blocked)
- Seen by: blind-spots; refutation: reframed (`Snapshot::range` is public API, so no internal-entry rule is breached; the gap is coverage); history: deliberate-and-holds for pull-based draining, silent on why the observer is not consumed
- Owner-gated: no

The module doc says the drain matches "the `UnorderedMessages` delivery contract", and it does, by re-implementing the observer's two steps rather than consuming the observer. The consequence is coverage: schedules of two to eight peers with bootstraps and retirements, the richest generated inputs in the suite, never run the public `UnorderedMessages` stream; it is exercised only by tests/listen.rs, tests/causal.rs, tests/api_send_bounds.rs, and sim.rs's concurrent `run_observers`. An observer-backed drain would pin the stream over every generated schedule and let the shadow meta-test cover it for free.

Evidence:

        65	    pub fn drain(&mut self) -> usize {
        66	        let snapshot = self.local.snapshot();
        67	        let mut new = 0;
        68	        for (version, message) in snapshot.range(causally::since(&self.checkpoint)) {
        69	            self.observations
        70	                .push((version.clone(), (*message).clone()));
        71	            new += 1;
        72	        }
        73	        self.checkpoint |= snapshot.latest();
        74	        new
        75	    }

    (src/rumors/unordered.rs:105, 219)
                    walk: inner.tree.range_owned(causally::since(checkpoint.clone())),
                        this.checkpoint |= &ceiling;

Resolution: hold an `UnorderedMessages<T>` in the test `Peer`, created in `new` via `local.unordered_messages_since(latest)`, and implement `drain` by polling it with `now_or_never` until quiet (the pattern tests/listen.rs and tests/causal.rs already use); delete the `checkpoint` field and the range re-derivation. Acceptance: peer.rs no longer calls `snapshot.range(causally::since(..))`; shadow_validity.rs, multi_peer.rs, membership.rs, partition.rs, and sanity.rs pass unchanged; deliberately skipping the observer's ceiling absorption fails them.

Construction: none is needed to show a defect (this is a coverage gap); the demonstration is the acceptance above.

### tests-common-22: cut offsets are only ever sampled; no suite sweeps every offset of a hand-off session
- Where: tests/common/sim.rs:282-293 (related: tests/common/sim.rs:104, tests/common/fault.rs:99-125, tests/disruption.rs:432, tests/session_overlap.rs:104-128)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (`grep -rn 'for cut in\|for offset in\|write_cut: Some(\|read_cut: Some(' tests/*.rs`: no session-level offset loop; the only fixed offsets are point regressions at tests/bookmark_causality.rs:1305,1310 and tests/gossip_when.rs:714,718,950)
- Seen by: blind-spots; refutation: severity down to low (codec-level truncation is swept totally in src; what is missing is the session-level algebra); history: no rationale (MAX_CUT's inline argument is about sampling density, not totality)
- Owner-gated: no

The only source of session-level cut offsets is `arb_fault`, uniform over `0..MAX_CUT`. The party hand-off frame and the epilogue marker are narrow windows in a session of that length, so whether a run lands a cut inside them at a given role and direction is sampling. tests/session_overlap.rs already shows the total-sweep pattern for poll prefixes; `fault::metered` gives the exact extent to sweep to; the sweep is a few thousand in-memory runs. Construct, do not argue.

Evidence:

       286	    let cut = prop_oneof![2 => Just(None), 3 => (0..MAX_CUT).prop_map(Some)];

Resolution: add a deterministic sweep (tests/disruption.rs or a sibling): build the seed-plus-fork fixture, meter one clean bootstrap and one clean retirement, then for each role, each direction, and each offset `0..=measured` run the session with that single cut and assert the classifier plus party conservation (the newcomer holds the fork xor the server rejoined it, allowing the documented post-take leak) and, for retirement, the `Retired`/`Recovered`/`Uncertain` algebra against the absorber's outcome. Pair with finding 25 so the donor and absorber sides are in the sweep. Acceptance: a committed test iterates every offset up to the metered extent for both directions of both hand-offs, its doc states the invariant, and it stays inside the nextest slow-test budget (60 s period, terminate after 3).

Construction: as the resolution describes; the metered extent from `fault::metered` on a clean run of each hand-off bounds the loop.

### tests-common-31: the proptest version the seed sweep transcribes is pinned in prose only
- Where: tests/seed_liveness.rs:26-29 (related: Cargo.lock:1802-1803, Cargo.toml:1)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (Cargo.lock:1802-1803 pins proptest 1.11.0; the root Cargo.toml is both the workspace and the package, so `CARGO_MANIFEST_DIR/Cargo.lock` is the lock file)
- Seen by: structure-prose; refutation: confirmed; history: deliberate-and-holds as a prose convention (390160a7 added the provenance paragraph "re-verify on upgrades"); the finding strengthens it into a check
- Owner-gated: no

The sweep's correctness rests on a transcription of proptest 1.11.0's `FileFailurePersistence::resolve`, and the doc asks the reader to re-verify when the dependency moves. Nothing fires when Cargo.lock changes; a bump that changed persistence resolution would leave this sweep passing against stale rules. Every hole found becomes a committed check, never a convention held in memory.

Evidence:

        26	//! Provenance of the transcribed rules: proptest 1.11.0,
        27	//! `FileFailurePersistence::resolve` (`failure_persistence/file.rs`).
        28	//! Re-verify the transcription when the workspace's proptest dependency
        29	//! moves to a release that touches persistence resolution.

Resolution: add `const TRANSCRIBED_PROPTEST_VERSION: &str = "1.11.0";` and a test that reads `Cargo.lock` from `CARGO_MANIFEST_DIR`, finds the `[[package]] name = "proptest"` block, and asserts its `version` equals the constant, with a message telling the bumper to re-check `FileFailurePersistence::resolve` and update the constant. Acceptance: bumping proptest in Cargo.lock fails the seed-liveness binary until the constant is updated.

### tests-common-6: the harness transcribes two crate derivations that `rumors::testing` could export, and a fixture self-check built on one compares it with itself
- Where: tests/common/flaky.rs:40-45 (related: tests/common/flaky.rs:57-64, tests/common/shape.rs:15-19, src/tree/typed/hash.rs:257-259, src/bookmark/format.rs:423, src/testing.rs:6-31, tests/gossip_snapshot.rs:107-108)
- Class / severity / confidence: test-quality / low / medium
- Provenance: verified (`PathHash::of` at src/tree/typed/hash.rs:257-259 is `Sha3_256::digest(bytes).into()`, the same derivation as shape.rs:18; `bookmark::format::decode` at format.rs:423 is `pub(crate)`; the `testing.rs` export list has no path or bookmark helper; the comment at tests/gossip_snapshot.rs:107-108 read)
- Seen by: structure-prose (named constants), blind-spots (self-check independence), api-economics (transcribed internals); refutation: confirmed all three; history: deliberate-but-expired (flaky.rs's "cannot reach the crate's codec" was written a month before `rumors::testing` existed)
- Owner-gated: yes: the preferred fix adds exports to the feature-gated `testing` surface

`shape::leaf_path` recomputes the crate's leaf address by hand, and `persisted_record_bytes` re-walks the bookmark frame (tag `55799`, a three-item array, payload at index 2, tag `24`) against a rationale that has expired: `rumors::testing` is the sanctioned test-internals entry point and already exports comparable helpers. The cost is not only drift risk: the batched-run fixture's self-check in tests/gossip_snapshot.rs recomputes prefixes with the same `leaf_path` that `shaped_pair` selected by, so it can catch a `keep_only` mistake but not the derivation drift its comment says it catches. If the transcription stays, its numbers should at least be named.

Evidence:

        40	/// Integration tests cannot reach the crate's codec, and don't need to: the
        41	/// stored file is one self-described CBOR item, so a generic walk — unwrap
        42	/// tag 55799, take the frame array's payload item, unwrap tag 24, then strip
        43	/// each stored clock's tag — recovers the untagged record serde understands.
    ...
        57	    let Value::Tag(55799, frame) = file else {
    ...
        63	    let payload = items.into_iter().nth(2).expect("a three-item frame array");
        64	    let Value::Tag(24, payload) = payload else {

    (tests/common/shape.rs:15-19)
        15	/// A leaf's tree path: the full-width SHA3-256 hash of its version's
        16	/// canonical bytes.
        17	pub fn leaf_path(version: &Version) -> [u8; 32] {
        18	    sha3::Sha3_256::digest(version.as_bytes()).into()
        19	}

    (tests/gossip_snapshot.rs:107-108)
        // Self-check the landed shape: if hashing or version assignment
        // drifts, fail here with a clear message rather than in the hex.

Resolution: preferred: export `testing::leaf_path(&Version) -> [u8; 32]` delegating to the crate's leaf-address derivation and `testing::decode_bookmark_record(&[u8]) -> Result<BTreeMap<Network, Vec<Clock>>, FormatError>` delegating to `bookmark::format::decode`; delete both transcriptions; the self-check then compares the crate's derivation with the landed wire. Fallback: name the constants (`SELF_DESCRIBED_CBOR_TAG`, `EMBEDDED_CBOR_TAG`, `FRAME_PAYLOAD_INDEX`) at the top of flaky.rs, reword flaky.rs:40 to what is true today, and reword tests/gossip_snapshot.rs:107-108 to claim only what a same-function comparison checks. Either way, give the batched-run fixture a transcript-level assertion (a run frame carrying exactly two records) before its snapshot, as the radix fixtures already have. Acceptance: no `Sha3_256::digest`, `55799`, or `Tag(24` in tests/common, or each is a named constant; no comment claims a same-function comparison detects hash drift.

### tests-common-12: the overlap shadow models the fork at `Open`; the live session forks at its first poll; the doc states the model as fact and the guard's skips are unmeasured
- Where: tests/common/overlap.rs:17-25 (related: tests/common/overlap.rs:92-117, tests/common/overlap.rs:178-179, tests/common/overlap.rs:233-241, tests/common/overlap.rs:562-564, tests/common/overlap.rs:636, src/rumors.rs:489, tests/session_overlap.rs:154-178)
- Class / severity / confidence: test-quality / low / medium
- Provenance: assessed (read: `open` at 93-117 builds two futures without polling them; `Rumors::gossip` at src/rumors.rs:489 is `pub async fn`, so nothing runs until the first poll; the shadow snapshots at `Open` at line 636; the divergence case below is constructed by reasoning, not run)
- Seen by: blind-spots (guard hides shadow drift), api-economics (shadow unverified); refutation: reframed (the guard is essential to a by-design imprecision; the gap is the unmeasured skip count); history: deliberate-and-holds for the guard's stated purpose
- Owner-gated: no

The module doc says `Open` "captures both endpoints' fork-time state" and the event doc says `Open` forks "both sides' working state here". The live session forks at the first poll that reaches its fork, after any events between `Open` and the first `Step`. Consider an endpoint that redacts, in that gap, a message its counterparty never held: the shadow's `Open`-time snapshot has the message live, so `merge_session` credits the counterparty with it at `Close`, and a later `RedactObservation` there emits a `Redact` the live peer never observed. The guard at 237-241 is therefore necessary, and the "shadow imprecision" its comment names is by design. What is missing is a measurement: nothing counts how many emitted `Redact`s the guard drops, so redaction coverage under overlap has no liveness floor, and the docs describe the model as if it were the protocol. The earlier proposal to replace the guard with an assertion would fail valid schedules.

Evidence:

        17	//! The alphabet extends [`schedule::events::Event`] with three session
        18	//! events over a small set of *slots*: [`OverlapEvent::Open`] captures
        19	//! both endpoints' fork-time state and parks, [`OverlapEvent::Step`]
    ...
       233	                // The generator's shadow makes this always-observed; the
       234	                // guard mirrors the serial executor's, so a shadow
       235	                // imprecision degrades to a skipped event on both sides
       236	                // of the comparison rather than an invalid `redact`.
       237	                let observed = peers[*peer].observations.iter().any(|(v, _)| v == version);
       238	                if observed {
    ...
       636	                open.insert(slot, (a, b, sim.clone()));

Resolution: (1) restate at 17-25, 178-179, and 562-564 that the shadow models the fork at `Open` while the live session forks at its first poll, and that the guard exists because the two differ; replace the guard comment's borrowed justification (there is no gossip filter here) with that one. (2) Return the skipped-`Redact` count from `execute_overlap_and_quiesce` and pin a ceiling in tests/session_overlap.rs (a fraction of emitted `Redact`s over the run), with a committed demonstration that a broken `merge_session` (skipping the `Close` merge) exceeds it. Acceptance: the docs describe the model as a model; the suite fails when the skip ceiling is exceeded and still passes at HEAD.

Construction: a hand-built `OverlapSchedule` with three peers after a converged preamble: `Insert { peer: a, value }` (event k), `Open { slot: 0, a, b }`, `Redact { peer: a, target_event_idx: k }`, `Close { slot: 0 }`, `Redact { peer: b, target_event_idx: k }`. The shadow emits the final `Redact` (its `Close` merge credited b with k); the live b never observes k (a's fork at first poll has k dead), so the executor's guard skips it. A skip counter reports 1; today nothing reports anything.

Cross-reference: tests-observation-28 carries the missing meta-test and the demonstration that a broken `merge_session` leaves the suite passing.

### tests-common-20: the shadow docs claim an exact observation sequence; only the set is pinned, and the live order is tree order
- Where: tests/common/schedule/arb.rs:248-250 (related: tests/common/schedule/arb.rs:58-60, tests/common/schedule/arb.rs:355-360, tests/common/peer.rs:33-35, tests/common/peer.rs:68, src/snapshot.rs:129-130, tests/shadow_validity.rs:25-26)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (src/snapshot.rs:129 documents `range` as "order is unspecified"; `Peer::drain` records in that order; the shadow's `absorb` pushes novel indices in sorted `EventIdx` order; tests/shadow_validity.rs:25-26 compares sets for exactly this reason)
- Seen by: blind-spots; refutation: confirmed; history: no rationale (the only recorded reason the shadow's order matters is seed stability of its own redact-target selection)
- Owner-gated: no

`SimState`'s doc says `observed_log` is "the exact sequence of `EventIdx`s that the live `Peer<T>` would have appended", and the gossip comment says per-peer observation order is what the live peer produces. Neither is guaranteed: the live log is in `Snapshot::range` order, which the crate documents as unspecified, and the meta-test compares sets because of that. The order is the shadow's own, used to draw redact targets deterministically. A model doc must be accurate; a reader who relied on the sequence claim (to add a sequence-wise comparison, say) would misdiagnose the failure.

Evidence:

       248	/// * `observed_log[p]` is the exact sequence of `EventIdx`s that the
       249	///   live `Peer<T>` would have appended to its observation vector by
       250	///   this point — driven by both local inserts and gossip events.

    (tests/shadow_validity.rs:25-26)
    //! Comparison is set-wise: callback order within a batch is
    //! unspecified, so a sequence-wise comparison would over-constrain.

Resolution: restate 248-250, 58-60, and 355-360: `observed_log[p]` is the set of `EventIdx`s the live peer has observed, kept in the shadow's own sorted-per-pass order so redact targets are drawn deterministically; the live peer's order is the tree's and is not compared. Acceptance: no passage in arb.rs claims sequence agreement with the live peer.

### Nits

The full record of each (provenance, history, evidence) is in the evidence file named in the last column.

| Id | Where | Claim | Resolution | Record |
|---|---|---|---|---|
| tests-common-29 | `tests/common/wire.rs:44-48` | four suites alias `tokio_block_on as block_on`, erasing the poller distinction wire.rs draws | call `tokio_block_on` by name | `evidence/partitions/tests-common.md` |

## Integration tests: lifecycle suites (bootstrap, retire, lifecycle, membership, multi_peer, pairwise, partition, reuse, redaction)

Nothing pins that redactions stay in the generated populations; the network-mismatch test is vacuous on empty peers and has no retire dual; the epoch-wrap test never reads the epoch; two redaction no-op tests can fail only by panic; and two binaries pin one union law.

### tests-lifecycle-15: Nothing pins that redactions stay in the `arb_local_actions` or `arb_schedule` populations
- Where: tests/pairwise.rs:47-53 (related: tests/multi_peer.rs:24-26, tests/partition.rs:48, tests/sanity.rs:25, tests/retire.rs:363-365, tests/bootstrap.rs:61, tests/membership.rs:99-135, tests/common/action.rs:26-32, tests/common/action.rs:79-83, tests/common/schedule/arb.rs:210-218, tests/common/schedule/arb.rs:365-372, tests/common/schedule/arb.rs:395-407)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`grep -n 'Event::Redact\|LocalAction::Redact' tests/*.rs` returns nothing outside tests/common; `grep -rln 'TestRunner::deterministic' tests/` lists only membership.rs and window_sweep.rs; action.rs:79-83 drops `Redact` when no version exists, arb.rs:395-407 drops `RedactObservation` when the observed log is empty)
- Seen by: blind-spots; refutation: confirmed (committed seeds carry `Redact` actions, but a seed replays RNG state and a zero weight would regenerate insert-only cases); history: no-rationale-found (population pins were added for the membership alphabet and the budget arm as each landed; the older redaction dimension never received one)
- Owner-gated: no

`LocalAction::Redact(idx)` is dropped at build time when nothing has been sent, and `Choice::RedactObservation` is dropped when the peer's observed log is empty. Both drops are legitimate, but no test counts effectual redactions in a sampled population, so a `prop_oneof!` weight typo, a `%` bug in `lookup_observation`, or a change to `created_version` would leave every redaction-bearing property in pairwise, bootstrap, retire, async_wire, multi_peer, partition, and sanity green while exercising insert-only merges. `membership_population_contains_churn` pins exactly this liveness for the membership alphabet and says why ("Either could silently vanish ... leaving the suite green while testing no membership at all"). Meters need liveness floors: a property over a generated dimension passes vacuously when the generator stops producing it.

Evidence:

        47	proptest! {
        48	    /// After one bidirectional gossip session, the two peers' live
        49	    /// content (as exposed through `readout`) is equal.
        50	    #[test]
        51	    fn gossip_converges(
        52	        a_actions in arb_local_actions(),
        53	        b_actions in arb_local_actions(),

Resolution: Add deterministic-runner population tests mirroring `membership_population_contains_churn`: sample `arb_local_actions()` N times and assert some sample contains a `Redact` positioned after at least one `Insert`; sample `arb_schedule(any::<u64>(), N_PEERS, MAX_EVENTS)` and assert some emitted schedule contains an `Event::Redact`. Place them beside the strategies' consumers (pairwise.rs and multi_peer.rs) or in one `tests/population_liveness.rs`. Acceptance: setting the `Redact` weight to 0 in `arb_actions` or in `arb_choice` fails the new pin while the property suites otherwise still pass (the negative control demonstrates the pin fires).
Construction: in a scratch copy, change `1 => any::<usize>().prop_map(LocalAction::Redact)` at action.rs:29 to weight 0 and run pairwise, bootstrap, retire, async_wire: all pass today; the proposed pin fails.

### tests-lifecycle-18: pairwise.rs and async_wire.rs pin the same union law under the same harness in two binaries
- Where: tests/pairwise.rs:220-237 (related: tests/pairwise.rs:51-61, tests/async_wire.rs:1-11, tests/async_wire.rs:43-59, tests/async_wire.rs:65-81, tests/bootstrap.rs:6-8)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (`diff` of pairwise.rs:224-236 against async_wire.rs:47-58 with `dup`/`bootstrap_fork` and the `Peer` path normalized differs only in how `expected` is bound and in async_wire's extra fingerprint assert, which `gossip_converges` at 51-61 provides; both draw `arb_local_actions()` twice over an empty floored seed)
- Seen by: api-economics; refutation: confirmed; history: deliberate-but-expired (async_wire.rs existed to twin `sync_wire.rs`, and pairwise.rs dropped its wire tests then; the shared-state port re-pointed pairwise at the wire and 83edcd944 deleted the sync twin, leaving async_wire.rs without its reason)
- Owner-gated: no

`gossip_unions_content` together with `gossip_converges` is exactly `async_wire.rs::async_gossip_converges_on_the_union`: the same two `arb_local_actions()` inputs, the same construction, the same `extend` oracle, the same readout and fingerprint assertions, at default case counts in two link units. Only the `String` leg of async_wire.rs is unique. The framing "the *asynchronous* gossip path" (async_wire.rs:1, common/wire.rs:1) names a distinction the crate no longer has, and bootstrap.rs:6 says "Mirrors `async_wire.rs`'s setup".

Evidence:

       220	    fn gossip_unions_content(
       221	        a_actions in arb_local_actions(),
       222	        b_actions in arb_local_actions(),
       223	    ) {
       224	        let seed = rumors::Peer::<u64>::seed().sync_window_floor().into_rumors();
       225	        let a = build_local(dup(&seed), &a_actions);
       226	        let b = build_local(dup(&seed), &b_actions);
       227	
       228	        let a_before = readout(&a.snapshot());
       229	        let b_before = readout(&b.snapshot());
       230	        let mut expected = a_before;
       231	        expected.extend(b_before);
       232	
       233	        wire_gossip(&a, &b);
       234	
       235	        prop_assert_eq!(readout(&a.snapshot()), expected.clone());
       236	        prop_assert_eq!(readout(&b.snapshot()), expected);

Resolution: Keep the union law in pairwise.rs (the algebraic-laws file), move the `String` leg there as one generic body called for both payload types, delete tests/async_wire.rs, and rewrite bootstrap.rs:6-8 to point at pairwise.rs or drop the sentence; drop "asynchronous" from common/wire.rs:1. Acceptance: one binary owns the union-of-readouts property for u64 and String; `grep -rl async_gossip_converges tests/` is empty; bootstrap.rs's module doc references no removed file.

Synthesis note: deleting tests/async_wire.rs orphans `proptest-regressions/async_wire.txt`, which holds four `cc` seeds (verified), and `tests/seed_liveness.rs` would then convict the file; AGENTS.md forbids stripping a seed, so the seeds must be re-homed into `pairwise.txt` in the same commit (the tests-common finalizer recorded the same caveat).

### tests-lifecycle-21: The redaction no-op tests assert quantities a redaction cannot change and leave the documented no-op contract unpinned; one testdoc says the docs are silent where they speak
- Where: tests/redaction.rs:94-133 (related: tests/common/peer.rs:63-75, tests/common/peer.rs:89-94, src/rumors.rs:176-177, src/batch.rs:79, src/tree.rs:406-408, src/batch.rs:132-143, tests/single_peer.rs:370-384)
- Class / severity / confidence: test-quality / medium / medium
- Provenance: verified for the vacuity (peer.rs:65-75 `drain` records only live leaves above the checkpoint and `redact_one` drains, so `observations.len()` cannot change across a redaction; in `redact_twice_is_idempotent` the only message is already redacted before `readout_before` is taken, and in `redact_unknown_version_is_noop` alice is a fork of an empty seed, so both readout comparisons are over empty maps) and for the doc (rumors.rs:176 "Redacting a version not currently held is a no-op", blame ce27df86e 2026-08-20; batch.rs:79; redaction.rs:117-118 blame 80a3155f41 2026-05-18); assessed for whether the tree's own tests hold the changed-flag contract (not checked)
- Seen by: blind-spots, structure-prose, api-economics; refutation: confirmed, severity lowered to low with the reasoning that the tests are weak rather than wrong; history: deliberate-but-expired for the doc (silent when written, documented 2026-08-20), and the observation-count assert was vacuous from birth
- Owner-gated: no

Both tests can fail only on a panic. The contract these tests exist to pin (an unheld or already-redacted version is a no-op: the causal ceiling does not move, `Tree::act` returns `false`, `Batch::commit`'s `send_if_modified` wakes no observer) is not examined: neither `latest()` nor a `changes()` observer is read. An implementation that ticked the party on an ineffectual forget would pass both tests unchanged. The doc at 116-118 says "the public docs are silent on this corner", which `Rumors::redact` and `Batch::redact` contradict; a test that guards a documented clause should say so, since that is what makes it non-negotiable at review. I hold this at medium against the refutation's low because the doctrine is explicit: a criterion the bad implementation also passes is decoration, and no public-tier test pins the no-tick promise.

Evidence:

       103	        let readout_before = readout_multiset(&peer.local.snapshot());
       104	        let obs_before = peer.observations.len();
       105	
       106	        peer.redact_one(&version);
       107	
       108	        prop_assert_eq!(readout_multiset(&peer.local.snapshot()), readout_before);
       109	        prop_assert_eq!(peer.observations.len(), obs_before);

       116	    /// Pins down the currently implemented behavior so
       117	    /// future regressions surface; the public docs are silent on
       118	    /// this corner.

Resolution: In both tests, give the peer live content that survives the redaction so the readout comparison is not over an empty map; capture `peer.local.snapshot().latest().clone()` before the redaction and assert it is unchanged after; subscribe a `changes()` observer (as single_peer.rs:370-384 does, draining the immediate first tick) and assert `now_or_never()` yields `None` after the redaction. Rewrite the doc at 116-118 to cite the no-op clause `Rumors::redact` states; consider folding the two tests into one property over "a version not currently held (never held, or already redacted)". Acceptance: both tests fail if `Tree::act` (or `Batch::commit`) is mutated to advance the ceiling or report `true` on an ineffectual forget; the testdoc no longer claims the docs are silent.
Construction: mutate `Batch::commit` at batch.rs:143 to `inner.tree.act(party, actions); true` (unconditional wake) or make the ceiling join run for every action: redaction.rs still passes today; the proposed `latest()`/`changes()` assertions fail.

### tests-lifecycle-5: No point pin that a newcomer's inherited floor suppresses a stale peer's copy of a redacted message
- Where: tests/bootstrap.rs:79-87 (related: tests/stale_floor.rs:27-40, tests/common/schedule/arb.rs:288-295, tests/common/schedule/arb.rs:315-336, src/rumors.rs:182-194)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read; `grep 'resurrect' tests/` finds only the two-peer gossip_snapshot cases and an unrelated bookmark_causality mechanism; the membership shadow's `record_bootstrap` copies `ever_known` and `absorb` honors `dst_known && !dst_live`, so the shape is reached generatively in membership.rs only)
- Seen by: blind-spots; refutation: confirmed; history: no-rationale-found (`stale_floor.rs` was born as a reproducer for one direction of the floor's job)
- Owner-gated: no

The comment states the newcomer is floored at the served tree's version, and `stale_floor.rs` pins one consequence: a newcomer's own sends survive. The other half of the floor's job has no point test: a newcomer served after the provider redacted X must, on first gossip with a third peer that still holds X, treat X as deleted rather than re-learn it. A bootstrap that served the tree with a floor below the redaction's tick would pass every test in this file and resurrect redacted content at the first three-peer gossip; only the membership engine reaches the shape, generatively.

Evidence:

        79	        // The newcomer's party is disjoint from the provider's retained half
        80	        // and floored at the served tree's version, so a fresh origination
        81	        // survives reconciliation on both sides.

Resolution: Add a point test beside `bootstrap_reproduces_a_fork`: seed sends X; `stale = bootstrap_fork(&seed)`; seed redacts X; `newcomer = bootstrap_fork(&seed)`; `wire_gossip(&newcomer, &stale)`; assert X absent from both snapshots and the newcomer's `latest()` dominates X's version. Acceptance: the test fails if the bootstrap serves a floor below the redaction tick instead of the served tree's ceiling.
Construction: `seed.send(X); let stale = bootstrap_fork(&seed); seed.redact(&v_x); let newcomer = bootstrap_fork(&seed); wire_gossip(&newcomer, &stale); assert!(newcomer.snapshot().get(&v_x).is_none() && stale.snapshot().get(&v_x).is_none())`. With `Rumors::redact`'s documented deletion honoring this passes today; a floor regression makes it fail.

### tests-lifecycle-7: The bookmarked join driver and the mutual bookmarked bail skip the drain check every other successful session runs
- Where: tests/bootstrap.rs:209-224 (related: tests/bootstrap.rs:285-295, tests/bootstrap.rs:46, tests/bootstrap.rs:136, tests/bootstrap.rs:178, tests/common/wire.rs:65-79)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`grep -n 'assert_control_drained\|tokio::join!' tests/bootstrap.rs`: joins at 38, 132, 167, 215, 287; drains at 46, 136, 178 only)
- Seen by: api-economics; refutation: confirmed; history: no-rationale-found (the helper landed six days after the drain gate was applied "one call per site"; the introducing commit does not mention drain)
- Owner-gated: no

`wire_bookmarked_join` returns the `Joined` outcome without calling `assert_control_drained`, and `mutual_bookmarked_bail_returns_the_bookmark` runs its mutual bootstrap at 285-295 without one either, whereas `both_bootstrapping_bail_with_none` (136) drains the identical unbookmarked session. `Joined::Joined` and `Joined::Bailed` are successful sessions under the assert's contract (wire.rs:65-79), so the bookmarked join path's boundary cleanliness is unchecked by the four `Joined`-arm tests that use the helper. The drain invariant was gated because a leftover control byte surfaces later as a confusing violation; a driver that omits the gate leaves that class uncaught on its path. Totality over spot checks.

Evidence:

       209	fn wire_bookmarked_join(
       210	    provider: &Rumors<u64>,
       211	    bookmark: FlakyInMemoryBookmark,
       212	) -> Joined<u64, FlakyInMemoryBookmark> {
       213	    block_on(async move {
       214	        let (mut provider_link, mut newcomer_link) = rumors::link::memory_with_capacity(LINK_BUF);
       215	        let (served, joined) = tokio::join!(
       216	            provider.gossip(&mut provider_link),
       217	            Peer::<u64>::bootstrap()
       218	                .bookmark(bookmark)
       219	                .join(&mut newcomer_link),
       220	        );
       221	        served.expect("the provider serves the bookmarked bootstrap");
       222	        joined
       223	    })
       224	}

Resolution: Inside the helper, call `assert_control_drained(provider_link, newcomer_link)` for `Joined::Joined`, `Joined::Bailed`, and `Joined::Unbookmarked` (the persist failure happens after the session, so its boundary is clean too), skipping only `Joined::Failed` with a one-line reason; add the drain to the mutual bail at 285-295. Both disappear under the shared driver of tests-lifecycle-3. Acceptance: every successful bookmarked join and bail in bootstrap.rs passes through `assert_control_drained`; the planted-byte negative control in reuse.rs still fails the assert.

### tests-lifecycle-11: The network-mismatch test uses empty peers, so its "before any content crosses" claim is vacuous, and the retire dual is untested
- Where: tests/network.rs:56-77 (related: src/peer/gossip.rs:1155-1164, src/peer/gossip.rs:696-702, src/peer/gossip.rs:1384-1404, tests/bookmark_causality.rs)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`grep -rln NetworkMismatch tests/ src/`: in tests only network.rs and bookmark_causality.rs, both on the gossip path; gossip.rs:1157-1163 places the check after the handshake and before `reconcile()` at 1164; gossip.rs:696-702 takes the whole party when retiring, before that check, and `PartyGuard::drop` at 1398-1400 restores it)
- Seen by: blind-spots; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

`gossip_rejects_foreign_network` seeds two universes and gossips them with nothing sent on either side. The doc says rejection happens "before any content crosses the wire", but with empty replicas there is no content to cross, and the session promise that `Err` leaves the replica unchanged goes unchecked. Production checks `remote_network != network` before `reconcile()`, so the claim is true today, but nothing holds it. The dual is also missing: a retiree from universe X retiring into an absorber from universe Y takes its whole party speculatively before the check and relies on `PartyGuard` to restore it; no test in tests/ exercises `NetworkMismatch` on the retire path. Two independently seeded universes must never interact; this guard is the only thing between them.

Evidence:

        56	/// Two peers from different seeds that try to [`gossip`](rumors::Rumors::gossip)
        57	/// are both rejected with [`Error::NetworkMismatch`] at the handshake, before
        58	/// any content crosses the wire.
        59	#[test]
        60	fn gossip_rejects_foreign_network() {
        61	    let alice = seeded::<u64>(1).into_rumors();
        62	    let bob = seeded::<u64>(2).into_rumors();

Resolution: Populate both peers with distinct content and take hashes before the session; wrap both links with `rumors::testing::wrap_link` and assert `connects + accepts == 0` on both reports and both hashes unchanged after the `NetworkMismatch`. Add `retire_rejects_foreign_network`: convert a populated universe-X handle to a `Peer`, retire it against a universe-Y gossiper, assert `Retire::Recovered { error: Error::NetworkMismatch { .. }, peer }`, the absorber's `Err(NetworkMismatch)`, the recovered peer's hash unchanged, and (via `dangerously_alias_party`) its party equal to the pre-session alias. Acceptance: moving the network check after `reconcile()` fails the sharpened gossip test on the stream counters; dropping the `PartyGuard` restore fails the retire test on the party comparison.
Construction: for the retire dual, `let x = seeded::<u64>(1).into_rumors(); x.send(1)?; let y = seeded::<u64>(2).into_rumors(); let pre = x.dangerously_alias_party(); let x = block_on(x.try_into_peer())?; let (out, served) = block_on(async { tokio::join!(x.retire(&mut lx), y.gossip(&mut ly)) });` then match `out` against `Retire::Recovered { error: Error::NetworkMismatch { .. }, peer }` and compare `peer.into_rumors().dangerously_alias_party()` to `pre`.

### tests-lifecycle-23: The header misplaces where party accounting lives, and the retire-into-bootstrapper point test omits the party check integration tests can make
- Where: tests/retire.rs:16-20 (related: tests/retire.rs:262-294, tests/party_conservation.rs:12-14, tests/party_conservation.rs:60-64, tests/common/sim.rs:1022-1057, tests/membership.rs:72-74, src/rumors.rs:418-424, Cargo.toml:145)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`dangerously_alias_party` is `pub` under `cfg(any(test, feature = "test-internals"))` at rumors.rs:420-422; Cargo.toml:145 enables `test-internals` for the crate's own tests; party_conservation.rs:60-64 and sim.rs:1022-1029 call it; membership.rs:74 calls `assert_party_invariants`; retire.rs:262-294 asserts network, content, and one origination, with no party assertion)
- Seen by: blind-spots, api-economics; refutation: confirmed; history: deliberate-but-expired (the header was true for about an hour on 2026-06-10: 1d3a3df4d1 landed at 22:42 and 9eadfc680 exposed the alias at 23:39; party_conservation.rs arrived a month later)
- Owner-gated: no

The header says the party-accounting side "lives in the crate-level tests, which can read the party." Integration tests read the party too, and the gate-permanent accounting suite is `tests/party_conservation.rs`; membership.rs in this very partition asserts seed reconstitution through `assert_party_invariants`. Meanwhile `retire_into_bootstrapper_hands_off_the_identity` checks network, content, and one origination but not that seed ⊔ successor reconstitutes `Party::seed()` and that the two are disjoint, even though the identity hand-off to a newborn is a distinct wire path (the bootstrap side receives the whole party as the trailing frame) and the sharp check is one call away. No generative engine reaches this cross: the membership executor absorbs through plain gossip and party_conservation's `Op::Retire` targets a live fleet member.

Evidence:

        16	//! (No test covers a retire refused by outstanding snapshots, because
        17	//! none can exist: the `Peer`/`Rumors` XOR makes "retire while
        18	//! observers share the party" unrepresentable at compile time. The
        19	//! party-accounting side — every retire reconstituting the seed's whole
        20	//! id-space — lives in the crate-level tests, which can read the party.)

Resolution: Rewrite the sentence to name `tests/party_conservation.rs` and the membership suite's `assert_party_invariants`. In `retire_into_bootstrapper_hands_off_the_identity`, after the hand-off call `crate::common::sim::assert_party_invariants(&[seed.clone(), successor.clone()], 0)`. Optionally add an `into_newcomer` arm to party_conservation's `Op::Retire`. Acceptance: the point test fails if the successor receives a fork rather than the retiree's whole region; the header names the actual accounting suites.
Construction: the assertion `assert_party_invariants(&[seed, successor], 0)` passes today by the retire contract; mutating gossip.rs:702 `inner.party.take()` to `inner.party.as_mut().map(Party::fork)` (donating a fork instead of the whole party on retire) leaves content and network intact and fails only this assertion.

### tests-lifecycle-28: The epoch-wrap test infers the wrap from hand-computed constants and never reads the public `SessionState::epoch()`
- Where: tests/reuse.rs:184-194 (related: tests/reuse.rs:36-47, tests/reuse.rs:198-226, tests/reuse.rs:153-168, tests/reuse.rs:252-258, src/link.rs:346-352, src/link.rs:363-367, src/link.rs:385-393, src/link.rs:470-478, src/link.rs:499)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (reuse.rs:194-226 read: nothing observes the epoch; link.rs:352 `epoch: u8`, :391 `wrapping_add(1)`, :365-367 `pub fn epoch(&self) -> u8`, :499 `pub session: SessionState` on `LinkParts`; reuse.rs:252-258 already uses `into_parts`/`into_link`)
- Seen by: structure-prose, blind-spots; refutation: confirmed; history: deliberate-but-expired (when the test was written, `SessionState`'s fields were not publicly readable; ee67e4cbb exposed `epoch()` six days later)
- Owner-gated: no

The test runs 253 no-op sessions then 6 divergent rounds and asserts convergence; the doc states the rounds run "at epochs 253 through 255 and the wrapped 0 through 2", but nothing observes an epoch. `PRE_WRAP_SESSIONS = 253` is a hand-derived literal tied to the `u8` width. If the epoch became `u16`, or `begin` stopped counting stream-less sessions, the wrap would never occur and the test would keep passing as an ordinary reuse test. `empty_sessions_advance_epochs_in_lockstep` (153-168) asserts its own premise through the wrapped link's counters; this test does not. Every bound needs proof its signal is alive; state the structure ("two before the wrap") and let the compiler compute the tally.

Evidence:

       189	/// Strategy: converged no-op sessions burn epochs cheaply (they
       190	/// open no data streams but still count — the lockstep the test above
       191	/// pins), then divergent rounds bracket the wrap itself, running at
       192	/// epochs 253 through 255 and the wrapped 0 through 2.

        42	const PRE_WRAP_SESSIONS: usize = 253;

Resolution: `const PRE_WRAP_SESSIONS: usize = u8::MAX as usize - 2;` with the doc saying "two epochs before the wrap". After the pre-wrap loop, on each end: `let parts = link.into_parts(); assert_eq!(parts.session.epoch(), PRE_WRAP_SESSIONS as u8); let mut link = parts.into_link();`. After the wrap rounds, assert both ends read `((PRE_WRAP_SESSIONS + WRAP_ROUNDS as usize) % 256) as u8`, which witnesses the lockstep directly. Acceptance: changing `PRE_WRAP_SESSIONS` so no wrap occurs, or widening the epoch type, fails the test at the epoch assertion; no literal 253 in reuse.rs.

### tests-lifecycle-4: The bootstrap testdoc claims a failure mode the body cannot detect
- Where: tests/bootstrap.rs:52-59 (related: tests/bootstrap.rs:79-87, tests/bootstrap.rs:90-118, tests/party_conservation.rs:237-262, tests/stale_floor.rs:1-20)
- Class / severity / confidence: test-quality / low / medium
- Provenance: assessed (read; reasoning over the body at 79-87: the provider sends nothing after the fork, so a newcomer ticking a copied party still lands strictly above the provider's ceiling and its message is learned, not dropped)
- Seen by: blind-spots, structure-prose (the "silently destroy" tell); refutation: confirmed; history: no-rationale-found (the wording is original to the shared-state port)
- Owner-gated: no

The doc says the newcomer's origination surviving gossip back into the provider is something "a non-disjoint or stale-floored party would silently destroy." The stale-floor half holds (and names its mechanism in `tests/stale_floor.rs`). The non-disjoint half does not: a copied party manifests only when both sides originate concurrently (same version, two payloads), and the provider originates nothing after the fork here. Disjointness is pinned directly in `tests/party_conservation.rs:237-262`, so the invariant is covered; this doc is inaccurate about what this test proves, and "silently destroy" appears without the mechanism (a dominated version is treated as already forgotten). An inaccurate testdoc is a bug in the test.

Evidence:

        52	    /// Bootstrapping from a provider yields exactly the provider's live
        53	    /// content, message identities included (versions are stable across
        54	    /// peers), leaves the provider's own content untouched, and creates a
        55	    /// *disjoint* party.
        56	    ///
        57	    /// Disjointness is proven behaviorally: a message the newcomer originates
        58	    /// survives a gossip round back into the provider, which a non-disjoint or
        59	    /// stale-floored party would silently destroy.

Resolution: Either drop "non-disjoint" and "creates a *disjoint* party" from the doc, cite `party_conservation.rs` for disjointness, and state the floor mechanism ("a stale floor would leave the newcomer's version dominated, and deletion honoring would drop it as already seen"); or sharpen the body so the claim is true: have the provider also `send` after the fork, gossip, and assert both payloads live on both sides. Apply the same to the `String` variant. Acceptance: the doc's stated failure mode is one the body demonstrably detects, and the adverb has its mechanism beside it.

### tests-lifecycle-26: retire_redaction.rs is a one-test binary restating a retire.rs claim with a hand-rolled session and an application-scenario framing
- Where: tests/retire_redaction.rs:1-8 (related: tests/retire_redaction.rs:39-46, tests/retire.rs:52-65, tests/retire.rs:184-217)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (file read in full, 54 lines, one test; its session at 39-46 respells `retire_into_gossip` at retire.rs:52-65 including the drain assert; retire.rs:184-217 already pins a retiree's redaction propagating through the retire session)
- Seen by: structure-prose, api-economics; refutation: confirmed, severity lowered to low; history: deliberate-but-expired (born in 374adc960 as a regression pin beside the rumormill soak harness and framed in rumormill's chatroom vocabulary; rumormill left the workspace in 397fcb081 on 2026-07-31; the retire.rs sibling already existed the day the file was created)
- Owner-gated: no

The single test proves that a redaction the retiree performed after its last gossip reaches the absorber through the retire session. retire.rs already tests this claim with a different construction (message inserted before the fork versus originated by the retiree and learned via gossip), and neither doc names that difference as the reason for two tests. The module doc frames the test as an application scenario ("chatroom goodbye path", "presence entry") rather than by the mechanism it names in passing ("deletion honoring rides version bounds"). Each `tests/*.rs` file is a separate binary linking `common`, so this is a full link unit for one test, and a reader of retire.rs will not know the second construction exists.

Evidence:

         1	//! A retiree's last-moment redactions must survive into the absorber.
         2	//!
         3	//! This is the chatroom goodbye path: a departing peer redacts its own
         4	//! presence entry, then retires its party into a live peer. The retire
         5	//! session's built-in reconciliation must carry the *absence* (deletion
         6	//! honoring rides version bounds), not just the retiree's unsent content —
         7	//! otherwise every clean departure leaves a ghost entry behind that only
         8	//! application-level staleness sweeps can clear.

Resolution: Move the test into retire.rs, drive it through `retire_into_gossip` (or the shared driver of tests-lifecycle-3), and either merge it with `retiree_redaction_propagates_through_retire` as two arrangements of one test or state in its doc what distinguishes the gossip-learned construction (the redacted version lives above the absorber's frontier, so the absence must be inferred from the retiree's ceiling rather than from shared pre-fork state). Delete tests/retire_redaction.rs and rewrite the framing as mechanism. Acceptance: tests/retire_redaction.rs is gone; retire.rs carries the gossip-learned case with a doc naming what distinguishes it; `just test retire` passes.

### Nits

The full record of each (provenance, history, evidence) is in the evidence file named in the last column.

| Id | Where | Claim | Resolution | Record |
|---|---|---|---|---|
| tests-lifecycle-20 | `tests/partition.rs:50-56` | peer indices drawn as modulo'd `any::<usize>()` seeds over a fixed fleet, against multi_peer.rs's stated idiom | `prop_flat_map` on the schedule; state the rule once | `evidence/partitions/tests-lifecycle.md` |

## Integration tests: bookmark suites (attach, causality, transmit window, when)

The causality simulation's oracle is blind to the recycle that destroys content and its harness swallows errors that are unconditionally bugs under fault-free regimes (two highs); its determinism claim is false because `Network` ids come from `OsRng`; three of the four suites carry no stall bound; and the attach contract's read-failure and corrupt-frame halves are untested.

### tests-bookmark-9: The recycle oracle cannot see the recycle that destroys content
- Where: tests/bookmark_causality.rs:140-146 (related: tests/bookmark_causality.rs:26-28, 96-98, 411-413, 868-893, 1215-1239; src/bookmark.rs:450; src/reconciliation.rs:96-100; crates/before/src/version.rs:51; tests/bookmark_transmit_window.rs:274-305)
- Class / severity / confidence: verification-gap / high / high
- Provenance: assessed (read; traced `reclaim`'s admission test, the sieve's `<=` rule, and `Version`'s order; no test run)
- Seen by: blind-spots; refutation: confirmed, with two refinements to the construction adopted below; history: no rationale found (the version-only argument is birth-state; the reclaim-recycle of 077b64db was pinned in a new suite and nothing records why this suite did not catch it)
- Owner-gated: no

`EmissionLog::promote` flags a recycle only when a later durable emission's full version compares `Less` or `Equal` to an earlier one's. The recycle the bookmark exists to prevent (a rebooted peer reclaims its region below a frontier some replica durably holds, then ticks) yields a version that compares `Greater` or incomparable whenever the reclaimed peer's frontier carries any other region's progress, so the predicate passes it. Because redactions are deliberately untracked (L411-413) and the post-heal check demands a witness only for leaves still live (L868-893), the consequence, the causal sieve deleting the durable message fleet-wide, is invisible too. The module doc's step at L26-28 is the incorrect inference: the version order forbids `later <= earlier`, but a recycle is an own-region collision under a strictly greater or concurrent full version. The suite's negative control (L1221-1239) exercises only the equal-version shape. Mitigation: `record_dominates_the_transmitted_frontier` pins the crate invariant at the transmit boundary, so the crate is covered; the finding is that the proptest's headline claim ("however adverse the run") is judged by an oracle blind to its primary failure class (Principle 6: every criterion needs a committed demonstration that a known-bad mechanism fails it).

Evidence:

       140	            // A recycle is `later <= earlier` in the causal (partial) order;
       141	            // genuinely concurrent versions compare `None` and are fine.
       142	            assert!(
       143	                !matches!(
       144	                    later.version.partial_cmp(&earlier.version),
       145	                    Some(Ordering::Less | Ordering::Equal)
       146	                ),
        ...
        27	//! incomparable, never `<=`). The id-region need not enter the comparison at all
        28	//! — it would only rule out collisions the version order already forbids.

    src/reconciliation.rs:
        96	//! - A leaf whose version is **contained in** (`<=`) the lacking side's
        97	//!   version records a send that side's history already covers: the lacking
        98	//!   side has necessarily *seen* the message, but no longer holds it. Seen
        99	//!   but absent means it must have been deleted — so the holder drops the
       100	//!   leaf locally too, honoring a deletion it was never told about directly.

    src/bookmark.rs:
       450	        for clock in clocks.extract_if(.., |clock| clock.own_version() <= *version) {

Resolution: (1) Track redactions: `World::redact` knows the version it redacts; record it in a per-network ledger (the shape tests/disruption.rs already uses). After `heal`, assert that every message live at some live peer of the winning network at heal start, and never redacted, is live at every peer. Scope the ledger to the winning network: `heal` collapses other universes by re-bootstrapping their members, discarding their content. Under a correct implementation a frontier dominates an emission only by having merged it or a redacter's frontier, so the check is sound; crash-lost content is excluded because it is not live at heal start. (2) Optionally strengthen `promote`: record the emitter's party alias (`dangerously_alias_party()`) on each `Emission` and flag `later.version / later.party <= earlier.version / earlier.party` whenever the two parties overlap; then drop the version-only argument at L26-28 and L96-98. (3) Commit the demonstration: the mutant below passes today's suite and must fail the strengthened one. Acceptance: with the mutant applied to `Bookmarked::reclaim`, `bookmarking_never_recycles_a_version` fails on a plan of the constructed shape; on HEAD both variants still pass across the full case count.
Construction: mutant: in `src/bookmark.rs::reclaim`, push the alias at `Version::new()` instead of `version.clone()` (L467-470), so every stored region is admitted at once on reboot. Plan: n = 3, one network (A = 2, B = 1, C = 0 after bootstraps). `Send(A)` produces m at v_m (ticks only A's region). `Gossip(A, B)` over a clean wire: m propagates, `secure(A)` promotes it (durable). `Send(C)` three times. `Crash(A)`; the next step targeting A calls `revive`, which picks the lowest-index live peer in the network, C (never saw m), and `bootstrap_into` attaches the record (`record` never reclaims). A session `Gossip(A, C)` runs `bookmark_update`, whose `reclaim` admits A's old region under the mutant. `Send(A)` produces v' whose A-region height is at or below m's and whose C-region carries C's three sends: `v'.partial_cmp(&v_m)` is `Greater` or `None`, never `Less | Equal`, so `promote` is silent. `heal`: B meets A'; once A''s frontier dominates v_m and A' lacks m, the sieve deletes m at B; the fleet converges without m; `assert_live_content_is_durable` passes because m is no longer live anywhere. With the ledger from step (1), m is live at B at heal start and unredacted, and the survival assertion fails.

Demonstration (`witness/results.md`): Under the mutant (the reclaim alias pushed at `Version::new()`, src/bookmark.rs:467-470), A' reclaims its old region on the gossip with C, its first post-reclaim emission compares `Greater` to v_m, `promote` stays silent, the heal's sieve deletes m at all three nodes, and `world.assert_healed()` passes; only the added survival check fails. On HEAD the same plan is benign and m survives everywhere.

### tests-bookmark-12: The harness swallows session errors that are unconditionally bugs under fault-free regimes
- Where: tests/bookmark_causality.rs:580-602 (related: tests/bookmark_causality.rs:495, 511-535, 563, 621-643, 696-716, 751-754, 802-803, 1054, 1064-1073; tests/common/fault.rs:56-59; tests/common/sim.rs:282-285)
- Class / severity / confidence: test-quality / high / high
- Provenance: demonstrated (second witness pass: under `run_reliable_plan`, every third `World::gossip` was replaced by `Err(Error::PartyOverlap)` on both sides, 452 sessions failed that way across the property's cases, and `bookmarking_prevents_party_leakage` passed; before the pass, assessed by reading: `FaultPlan::is_clean` exists at fault.rs:57-59 and `arb_fault(false)` is `Just(FaultPlan::NONE)` at sim.rs:282-285, both read)
- Seen by: structure-prose, api-economics; refutation: confirmed, with the bound stated below; history: no rationale found (`let _ = serve_out;` is birth-state b7fb409f, whose own message applied the "stays loud" principle to `retire` and `sim` only; the L586 "wire fault" wording is 624917eb drift while L563 "clean" has stood since birth)
- Owner-gated: no

`bootstrap_into` drops the serving task's result, `Result<Result<Gossiped, Error>, JoinError>`, with `let _ = serve_out;` (a panic inside the server's gossip path vanishes) and folds every boot-side error to `None` with `.ok().flatten()?` and `.bookmark(bookmark).await.ok()`. `World::gossip` inspects results only for `NetworkMismatch` and treats every other `Err` as a disruption (L495). `revive` then seeds a fresh universe on a failed rejoin (L633-642), abandoning `single_network`'s one-network premise mid-plan with no assertion. Under `run_reliable_plan` (clean wires, empty feeds, one network) and every `arb_plan` case with `faults = false`, no session can legitimately fail except by `NetworkMismatch`, so any other `Err` is a protocol, codec, or bookmark-format bug; the harness hides it until `heal`, which asserts success and so catches only failures that reproduce there. The masked class is the state-dependent, intermittent one, precisely what the `retire` classifier at L696-716 was written for. `Error::Bookmark(BookmarkIo::Format(_))`, a crate-owned defect since the store returns only bytes the crate wrote, is classified nowhere. The L585-587 comment also names a wire fault as a cause on a wire the helper's own doc calls clean; on this wire the boot side fails only downstream of the server's persist fault (surfacing as a truncated hand-off) or its own attach persist fault. Principle 6: a harness that maps every server-side failure, including panics, to a benign plan outcome lets a crate bug pass as adversity. Bound: persistent failures do surface at heal, and the transmit-window and attach suites cover clean-wire sessions with asserted outcomes.

Evidence:

       585	                // `Ok(None)` cannot happen (the server is gossiping, not
       586	                // bootstrapping); a wire fault drops us to `None`, as does an
       587	                // injected persistence failure in the eager bookmark attach.
       588	                let peer = Peer::<Msg>::bootstrap()
       589	                    .join(&mut link)
       590	                    .await
       591	                    .ok()
       592	                    .flatten()?;
       593	                peer.sync_window_floor().bookmark(bookmark).await.ok()
        ...
       599	            let (boot_out, serve_out) = tokio::join!(boot, serve);
       600	            let _ = serve_out;
       601	            boot_out.expect("bootstrap task")
        ...
       530	        let mismatched = matches!(out_a, Err(Error::NetworkMismatch { .. }))
       531	            || matches!(out_b, Err(Error::NetworkMismatch { .. }));
        ...
       696	        // Never swallow the absorber's result: a retirement's whole point is the
       697	        // hand-off, and silently dropping a failed absorption is exactly what hid
       698	        // the codec leak this test was written to catch. The retire session runs

Resolution: give `World` knowledge of the fault regime (a `reliable: bool` set by `single_network` and by `arb_plan`'s `faults` flag, or `FaultPlan::is_clean` plus an emptiness accessor on `FaultFeed`) and assert `Ok` for gossip, bootstrap-serve, bootstrap-join, and attach whenever the step cannot legitimately fail. Where faults are possible, extract the classifier from `retire` (L706-716) into one `fn assert_not_codec_bug(&Error<FlakyInMemoryBookmark>)` applied to every session result on both sides, extended to `Error::Bookmark(BookmarkIo::Format(_))`, `Error::PartyOverlap`, and `Error::Io` with `InvalidData`; apply it to `Retire::Recovered { error }` and `Retire::Uncertain { error }` too. Bind `serve_out.expect("bootstrap serve task")` so a server panic fails the test. Rewrite L585-587 to name the actual sources on a clean wire. Acceptance: a planted decode failure in the bootstrap serve path fails `bookmarking_prevents_party_leakage` at the offending step with the classified error, not at heal and not never; a deliberately injected `panic!` in a serve-side path fails `bookmarking_never_recycles_a_version` instead of leaving the node dormant; the two proptests and the reconstructed tests still pass unchanged.
Construction: temporarily make `World::gossip` return `Err(Error::PartyOverlap)` from one side on every third call under `run_reliable_plan`; today the leakage property passes because heal's `clean_gossip` (which asserts) runs only after the plan; with the fix, the first such step fails.

Witness: the second witness pass ran this construction (`witness/results.md`, `## tests-bookmark-12`). Two counters were added to tests/bookmark_causality.rs and the session block in `World::gossip` was replaced, on every third call, by `(Err(Error::PartyOverlap), Err(Error::PartyOverlap))` with no session run, a stand-in for any state-dependent non-`NetworkMismatch` failure; then `cargo nextest run -p rumors --all-features --no-capture -E 'test(bookmarking_prevents_party_leakage)'`. The run printed 452 lines of the form `WITNESS tests-bookmark-12: injected Err(PartyOverlap) on both sides, injection #N` (counted with `grep -c`) and ended:

    test bookmarking_prevents_party_leakage ... ok
    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 3.27s
            PASS [   3.279s] (1/1) rumors::bookmark_causality bookmarking_prevents_party_leakage

The property passed with 452 injected failures because `gossip` inspects results only for `NetworkMismatch` (tests/bookmark_causality.rs:530-531) and `heal`'s `clean_gossip`, which does assert, runs only after the plan; a harness asserting that non-mismatch errors are impossible under the fault-free regime would have failed on the first injection. Bound: only the leakage property was run; the injection hook also sits under `run_plan`, which was not run. The edit was restored afterwards.

Cross-reference: tests-disruption-handshake-7 (correctness, high, demonstrated) is the same harness class in the inter-process engine: a panic in a serving task on a connection the child expects to fail is visible only in captured stderr, and the parent's loss accounting changes with no failing assertion.

### tests-bookmark-6: The attach contract's read-failure and corrupt-frame halves are untested through `Peer::bookmark`
- Where: tests/bookmark_attach.rs:86-96 (related: src/peer.rs:265-271; src/peer/gossip.rs:150-157, 372-398; tests/bookmark_causality.rs:591-593; tests/bootstrap.rs:198-204; src/bookmark/format/tests.rs)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`grep -rn BookmarkIo tests/` returns nothing; attach L55, L90, L180 and bootstrap.rs:200-202 inject write faults only; the `# Errors` contract at src/peer.rs:265-271 read)
- Seen by: blind-spots; refutation: reframed (the causality proptest's `read_faults` do reach the attach at L593 in aggregate, but `.ok()` discards the outcome, so nothing checks the contract's shape); history: no rationale found (`BookmarkIo::Format` arrived in 80898f19 with decoder-level coverage only)
- Owner-gated: no

`Peer::bookmark`'s contract covers "cannot be read or written" and promises a specific shape: nothing reaches storage, the peer comes back untouched, and `Unbookmarked.error` is a `BookmarkIo` with an `Io` and a `Format` arm. Every point test injects a write fault. No test drives a `load` failure or a store holding foreign or corrupt bytes through `Peer::bookmark` and checks the peer's party unchanged, the store byte-identical, and the error arm correct. Environmental failures (unreadable or corrupt storage) must be observable as errors of the documented shape (Principle 1); the format decoder's unit tests prove the decode rejects corruption but not that the attach path routes it to `Unbookmarked` without side effects.

Evidence:

        88	        let failing = FlakyInMemoryBookmark::new(
        89	            store.clone(),
        90	            Arc::new(Mutex::new(FaultFeed::new(vec![], vec![true]))),
        91	            0,
        92	        );
        93	        let Unbookmarked { peer, error } = peer
        94	            .bookmark(failing)
        95	            .await
        96	            .expect_err("the injected write failure must surface");

    src/peer.rs:
       267	    /// If the bookmark cannot be read or written, nothing reaches storage and
       268	    /// the peer is handed back **untouched**, still unbookmarked, inside
       269	    /// [`Unbookmarked`], to drop or retry. Because the attach never reclaims, the
       270	    /// live party is exactly as it was: a failed attach cannot leave reclaimed
       271	    /// identity live in this peer yet stranded on disk.

Resolution: add two point tests beside `failed_persist_returns_peer_for_retry`. Acceptance: both tests exist, each names the `BookmarkIo` arm it expects, and a mutant that makes `bookmark_inner` return `Ok(peer)` on a read error fails the first.
Construction: (a) in tests/bookmark_attach.rs, a non-pristine peer (one message sent) attaches a `FlakyInMemoryBookmark` over `FaultFeed::new(vec![true], vec![])`; assert `Err(Unbookmarked { error: BookmarkIo::Io(_), peer })`, `peer.dangerously_alias_party()` equal to the party before, the store still `None`, and a retry over a healthy feed succeeding. (b) Pre-seed the store with `Some(vec![0xff; 8])` (or a frame with foreign magic) and attach the same non-pristine peer; assert `BookmarkIo::Format(_)`, the store bytes byte-identical afterwards, and the party unchanged.

### tests-bookmark-8: The "fully deterministic" schedule claim is false: `Network` ids come from `OsRng` and decide the tie-break
- Where: tests/bookmark_causality.rs:46-56 (related: tests/bookmark_causality.rs:287, 545, 551-554, 639, 745-747, 1225, 1296-1327; src/peer.rs:206-215; src/network.rs:24-25; src/tree/mirror/streaming/tasks.rs:26; src/tree/mirror/streaming/materialized.rs:841; tests/common/wire.rs:49-59)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (`Peer::seed()` is `Self::seed_rng(&mut OsRng)` at src/peer.rs:206-208; `Network` derives `Ord` at src/network.rs:24; `grep -rl seed_rng tests/` lists nine suites, none of them bookmark suites; `git log -S'fully deterministic'` resolves to b7fb409f)
- Seen by: blind-spots, api-economics (the `select!` source, merged here as secondary); refutation: confirmed; history: no rationale found (`Peer::seed_rng` predates the suite, cb69fc95, so the suite could have seeded from birth)
- Owner-gated: no

The module doc promises a fully deterministic schedule whose counterexamples replay byte-for-byte and whose shrinking is sound, but every universe is created by `Peer::<Msg>::seed()` (L287, L639, L1225), which draws its `Network` from `OsRng`, and both the mismatch resolution (L545, keyed by `tuple` at L551-554) and the heal winner (L745-747) compare `(min_ticks, network)`. Fresh peers tie at zero ticks, so the winner is a coin flip per run: the same proptest seed can take different paths, a shrunk case can pass on retry, and `reconstructed_cut_gossip_then_retire_under_bookmark_faults` (n = 3, nodes 1 and 2 both pristine at `Gossip(1, 2)`) exercises one of two mismatch resolutions at random. A secondary, plausible-but-undemonstrated source: the streaming mirror's two unbiased `tokio::select!` sites draw tokio's thread-local RNG, and the runtime builder sets no seed. An inaccurate testdoc is a bug in the test, and proptest's shrinking and committed seeds presume replayability.

Evidence:

        48	//! Unlike `disruption.rs`, this simulation runs on a *current-thread* runtime
        49	//! with a fully deterministic, plan-driven schedule (each session is its own
        50	//! `block_on`). The bug class is about the *ordering* of
        51	//! emit/gossip/crash/retire/persist-fail events and the persistence-fault
        52	//! sequence, not watch-channel thread races; determinism makes counterexamples
        53	//! replay byte-for-byte, makes shrinking sound, and makes capturing each
        ...
       287	                let peer = block_on(Peer::<Msg>::seed().sync_window_floor().bookmark(bookmark))
        ...
       545	        let (winner, loser) = if ta >= tb { (a, b) } else { (b, a) };

    src/peer.rs:
       206	    pub fn seed() -> Self {
       207	        Self::seed_rng(&mut OsRng)
       208	    }

Resolution: thread a deterministic RNG through `World` (a `SmallRng` seeded per world, or a counter mapped through `seed_from_u64`) and replace the three `Peer::<Msg>::seed()` calls with `Peer::seed_rng(&mut rng)`; assert generated networks are pairwise distinct. Then the two reconstructed plans pin one path each and can assert which node re-bootstrapped. Calibrate the doc sentence for the `select!` bound ("deterministic up to tokio's `select!` branch order in the session internals") or add a cheap replay-identity check on a fixed plan using `common::fault::metered`'s `ByteMeter`. Acceptance: two runs of `reconstructed_cut_gossip_then_retire_under_bookmark_faults` resolve the mismatch with the same, asserted winner, and the doc claim at L46-56 is true as written.

### tests-bookmark-11: Spawn-based sessions carry no stall bound, so a stall becomes a 180 s nextest kill that loses the proptest reproducer
- Where: tests/bookmark_causality.rs:506-523 (related: tests/bookmark_attach.rs:17, 22-43; tests/bookmark_transmit_window.rs:42, 240-261; tests/bookmark_when.rs:66; tests/common/wire.rs:34-59; .config/nextest.toml:14-26)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (`grep -c tokio::spawn`: attach 2, causality 12, transmit 12, when 0; `grep -n timeout tests/bookmark_*.rs` returns nothing; attach, causality, and transmit import `tokio_block_on as block_on`, when imports `block_on`; `.config/nextest.toml:25-26` sets `slow-timeout = { period = "60s", terminate-after = 3 }`)
- Seen by: api-economics; refutation: confirmed, and raised the `join!`-over-`async move` observation adopted in the resolution; history: no rationale found (nextest.toml's collision note is a global ruling silent on per-suite bounds)
- Owner-gated: no

`bookmark_when.rs` drives sessions with `tokio::join!` under `common::wire::block_on`, the closed-world poller, so a wire stall fails immediately at its source. The other three suites use `tokio_block_on` with `tokio::spawn` and never bound a session, so a stall hangs until nextest terminates the process; for the two causality proptests that is exactly the collision nextest.toml documents: a case killed mid-shrink persists no seed. The causality spawn rationale (a faulted side must drop its link to surface EOF) is stated in terms of a `join!` over futures that borrow links declared outside them; a `join!` over two `async move` blocks that each own their link drops a completed block's link when `join!`'s `MaybeDone` transitions, exactly as a spawned task does, so the spawn may have outlived the constraint that justified it (Principle 3). attach's rationale ("the wires are reliable, so the bootstrap succeeds") describes the case the shared `join!`-based drivers already handle.

Evidence:

       506	        // Each side owns its faulted link inside its own task, so when a wire
       507	        // fault kills one side it returns and *drops* its link, surfacing EOF
       508	        // to the counterparty. A bare `join!` would instead hold both sides'
       509	        // links until both finished, deadlocking the survivor on a read that
       510	        // never completes.
       511	        let (out_a, out_b) = block_on(async {

    .config/nextest.toml:
        14	# One collision this budget knowingly accepts: proptest persists a failing
        15	# case's regression seed only after shrinking completes, so a genuinely
        16	# failing property whose shrink phase outruns the 180-second terminate
        17	# budget is killed mid-shrink and the reproducer is lost at exactly the

Resolution: first, one experiment: rewrite `World::gossip` as `block_on(join!(async move { let mut link = fault::faulty(side_a, fault_a); ra.gossip(&mut link).await }, async move { ... }))` under `common::wire::block_on` and run the two reconstructed tests plus a faulted proptest case. If the faulted side's EOF still reaches the survivor, drop `tokio::spawn` and `tokio_block_on` from causality (L506-523, L580-602, L672-694, L786-801) and transmit (the gated session becomes `join!(ga, gb, async { bm_a.entered().await; a.send(M1).unwrap(); bm_a.release() })`; `Notify` needs no runtime), which restores the stall detector without adding a timeout. If it does not, wrap each spawned pair in `tokio::time::timeout(SESSION_BUDGET, join!(...))` (the runtime is built with `enable_all()`) and panic naming the step. Move attach's `bootstrap_unbookmarked` onto the shared driver either way (finding 3). Acceptance: a planted never-completing gossip future in `bookmarking_never_recycles_a_version` fails the case inside the budget and writes a seed to proptest-regressions/bookmark_causality.txt; tests/bookmark_attach.rs imports `common::wire::block_on`, not `tokio_block_on`.

### tests-bookmark-21: The destruction check in `restart_after_transmit_never_destroys_durable_messages` is an unreachable arm, and no assertion witnesses a durable message surviving
- Where: tests/bookmark_transmit_window.rs:412-427 (related: tests/bookmark_transmit_window.rs:228, 233-234, 394-399, 468-483)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified for the history (`git log -S'if let Some(version) = m1_at_b'` resolves to 077b64db, "Both red at this commit"; `git log -S'm1_at_b.is_none()'` resolves to ccd88401, "so a schedule drift un-arming its conditional arm fails loudly"); the reading of the body is assessed
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed, structure-prose's framing kept because it names the missing M0 witness; history: the assert is deliberate and its purpose (drift is loud) still holds and is served by the assert alone; the arm is knowingly vacuous residue from the red commit with no recorded reason to remain, and ccd88401 passed its fix-round review without a ruling on it
- Owner-gated: no

The assert at L423 fails first on any drift, so the arm at L475-483 is dead code by construction, and the comment's "goes live only if a change lets a mid-persist commit ride the wire again" is false while the assert stands: on that change the test fails at L423 and the arm still never runs. The doc's headline (L394-395: "A crash after the gated session cannot make the network destroy a live, unredacted message") therefore has no witness in the body. What the body checks is that M1 was not transmitted (a restatement of `record_dominates_the_transmitted_frontier`) and that the fleet reconverges after A restarts from C. M0, the one message durable before the crash (held by B and C from the two bootstraps at L228 and L233-234), is never asserted present after the heal; three replicas converging on content that lost M0 would pass. An inaccurate testdoc is a bug in the test; a branch that cannot execute is scaffolding with no named catch.

Evidence:

       415	        // Under the transmit-window invariant the gated session snapshots
       416	        // its tree before the persist, so M1 — committed inside the persist's
       417	        // in-flight window — stays out of the session and dies, never
       418	        // durable, with A's crash: the destruction arm below is vacuous on
       419	        // this schedule and goes live only if a change lets a mid-persist
       420	        // commit ride the wire again. Asserting the expected outcome makes
       421	        // such drift loud here rather than silently un-arming the check.
       422	        let m1_at_b = leaf_version(&b, M1);
       423	        assert!(
       424	            m1_at_b.is_none(),
       425	            "the gated session must not transmit an own event committed inside the \
       426	             persist's in-flight window",
       427	        );
        ...
       475	        if let Some(version) = m1_at_b {
       476	            for (label, peer) in [("A'", &a2), ("B", &b), ("C", &c)] {
       477	                assert!(
       478	                    leaf_version(peer, M1).is_some(),

Resolution: pick one shape. (a) Recommended: keep the L423 assert as the drift alarm, delete the arm and its comment, add `assert!(leaf_version(peer, M0).is_some(), ...)` for A', B, and C after the heal so the "destroy" claim has a witness, and restate the doc as what is checked (an own event committed inside the persist window is not transmitted; after A crashes and reboots from C the fleet reconverges holding every pre-crash durable message). (b) Positive-path: build a schedule where M1 becomes durable before the crash (send M1, clean gossip A with B, crash A, restart from C which never saw M1, heal) and assert M1 survives everywhere; modest value since the invariant is pinned at the transmit boundary. Acceptance: no branch of the test is unreachable under its own assertions; the doc's first sentence names exactly the assertions in the body; a pre-crash durable message is asserted present on every replica after the heal.

### tests-bookmark-15: Bookmark fault schedules hold at most seven decisions per node, so late-life faults are unreachable
- Where: tests/bookmark_causality.rs:983-988 (related: tests/common/flaky.rs:116-122, 148-154; src/bookmark.rs:436-464)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read: `0..8` is exclusive; an exhausted queue defaults to success at flaky.rs:149 and 153)
- Seen by: blind-spots; refutation: confirmed, count corrected from eight to seven; history: no rationale found (the length is not derived from a plan's operation count)
- Owner-gated: no

`arb_fault_bits` yields at most seven booleans per node, while a 40-step plan can drive far more bookmark operations per node (each revive is a read and a write; each session up to two writes). Once the queue is exhausted every operation succeeds, so a fault can never land after a node's seventh read or seventh write; a failed write while the record already holds several stranded incarnations (the state the `overlapping`/`covers` filtering in `reclaim` exists for) is outside the generator's reach. A strategy must reach the interesting region; this one front-loads adversity into the first few operations.

Evidence:

       983	fn arb_fault_bits(faults: bool) -> BoxedStrategy<Vec<bool>> {
       984	    if !faults {
       985	        return Just(Vec::new()).boxed();
       986	    }
       987	    prop::collection::vec(prop_oneof![3 => Just(false), 1 => Just(true)], 0..8).boxed()

Resolution: generate fault positions sparsely (a small `BTreeSet<usize>` of failing operation indices over a range that covers a plan's plausible operation count, for example `0..128`), or lengthen the vector to cover it; keep the success bias and the shrink-toward-empty property `FaultFeed`'s exhausted-queue default already provides. Acceptance: a plan can fail a node's bookmark write after that node has completed more than seven prior writes; shrunk minimal counterexamples still shrink toward empty schedules.
Construction: a plan of `Send(0)`, `Crash(0)` repeated nine times (each revive is one read and one write on node 0), then `Gossip(0, 1, NONE, NONE)`: today no generated schedule can fail node 0's tenth write; with a sparse index set containing 9 it can.

### tests-bookmark-26: The when-model's alphabet has no no-op redact and no helper redaction
- Where: tests/bookmark_when.rs:629-655 (related: tests/bookmark_when.rs:578-605; src/rumors.rs:252-259)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read the op alphabet and `apply`; `redact_all`'s doc at src/rumors.rs:258 states "versions not currently held are skipped")
- Seen by: blind-spots; refutation: confirmed for the coverage gap (the reader's mechanism claim about `act.rs` was not adopted); history: no rationale found (bb6ce27e models only "a redact that removed a held key")
- Owner-gated: no

`Op::Redact` only ever redacts a version the subject currently holds and is skipped on an empty set, so the contract's unheld-version no-op is never checked against the bookmark schedule: a change that ticked on an ineffectual redaction would cause a write the model does not predict, and nothing here would see it. No helper ever redacts, so the subject never incorporates a ceiling-only remote advance (a frontier move with no content change), the one remote-content shape that differs from `HelperSend`. These are the two edges of the "write on local work, never on hearsay" line, and neither is in the alphabet.

Evidence:

       629	            Op::Redact(i) => {
       630	                // Redacting a message the application currently holds always
       631	                // records a deletion in the subject's own region, ticking it;
       632	                // redacting nothing (an empty set) is a true no-op. Liveness
       633	                // is read from the snapshot — the application's own view —
       634	                // never from the version arithmetic the suppression uses.

Resolution: add `Op::RedactAgain` (re-redact the most recently redacted version; model: no local change, so the next session drives no I/O) and `Op::HelperRedact(usize)` (a helper redacts one of its own held messages; model: pure hearsay). Both are a few lines in `apply` and the strategy. Acceptance: the proptest alphabet includes both ops and the model's predictions still match at every step.
Construction: with `Op::RedactAgain` in the alphabet, a mutant `Rumors::redact` that ticks the own region on an unheld version produces `Delta { writes: 1 }` at the next `Gossip` where the model predicts `writes: 0`, and the proptest fails.

### Nits

The full record of each (provenance, history, evidence) is in the evidence file named in the last column.

| Id | Where | Claim | Resolution | Record |
|---|---|---|---|---|
| tests-bookmark-4 | `tests/bookmark_attach.rs:45-55` | several assertions admit outcomes their docs exclude (a write-only fault feed under a "touches no storage" claim; `writes >= 1` where the model says one; `Ok(None)` accepted beside `Err`) | tighten each to the intended outcome (sites listed in the entry) | `evidence/partitions/tests-bookmark.md` |
| tests-bookmark-23 | `tests/bookmark_when.rs:1` | `bookmark_when.rs` and `bookmark_transmit_window.rs` collide with `gossip_when` and the pipeline "window" vocabulary (owner-gated) | rename to `bookmark_io_schedule.rs` and `bookmark_persist_coverage.rs` | `evidence/partitions/tests-bookmark.md` |

## Integration tests: observation suites (listen, causal, changes, observe, session_stats, session_overlap, shadow_validity, party_conservation, stale_floor)

The overlap generator's shadow has no validity meta-test (demonstrated blind); the conservation property never sheds; `Changes` has only point tests; the causal tests pin an order the public doc declines to promise; and several docs claim what the session learned, a handler count, or a live count that the bodies never check.

### tests-observation-28: The overlap generator's shadow has no validity meta-test; the executor degrades a shadow mismatch to a silent skip; the shadow snapshots at `Open` while the real session forks after the preamble
- Where: tests/shadow_validity.rs:44-53 (related: tests/common/overlap.rs:228-242, tests/common/overlap.rs:178-180, tests/common/overlap.rs:92-117, tests/common/overlap.rs:636, tests/session_overlap.rs:155-158, src/peer/gossip.rs:625-629, src/peer/gossip.rs:695)
- Class / severity / confidence: verification-gap / medium / medium
- Provenance: assessed (read: `with_shadow` strategies exist only in tests/common/schedule/arb.rs; the overlap executor guards on `observed` with a comment saying a shadow imprecision degrades to a skipped event; `open()` builds futures without polling; the real session awaits `handshake::preamble` before `prior_tree = Some(inner.tree.clone())`; the shadow snapshots `sim.clone()` at `Open`)
- Seen by: api-economics [39]; refutation: confirmed with one correction (the shadow can also over-approximate: a `Redact` at `a` between `Open` and the real fork is live in the shadow's snapshot but absent from the wire fork, so "assert instead of guard" is safe only after the fork point is fixed); history: no rationale found for the missing twin or the inaccurate `Open` doc; the skip is a stated design choice in code
- Owner-gated: no

The serial schedule generator's shadow has a committed meta-test in this file for both alphabets. The overlap generator's `Knowledge` shadow, which governs which `Redact` events are emitted, has no `_with_shadow` strategy and no twin here, and the overlap executor turns a mismatch into a skipped event on both sides of the comparison, so shadow drift thins the tested redaction space with no failing signal. The shadow already disagrees with the code it models: it snapshots at `Open`, but a real session forks its working state only after the preamble exchange, so events between `Open` and the first `Step` ride the real session and not the modeled one, and the `Open` variant's doc ("forking both sides' working state here") is inaccurate against that path. Doctrine: a test blind spot is a finding; every hole becomes a committed check.

Evidence:

    44	proptest! {
    45	    /// For every peer, the shadow simulator's `observed_log` and
    46	    /// `live` sets (as `BTreeSet<EventIdx>`) match the live
    47	    /// executor's observations and current readout, translated
    48	    /// through `resolved_versions` back to event indices.
    49	    #[test]
    50	    fn shadow_predicts_live_state(
    51	        (schedule, shadow) in arb_schedule_with_shadow(any::<u64>(), N_PEERS, MAX_EVENTS),

    (tests/common/overlap.rs)
    233	                // The generator's shadow makes this always-observed; the
    234	                // guard mirrors the serial executor's, so a shadow
    235	                // imprecision degrades to a skipped event on both sides
    236	                // of the comparison rather than an invalid `redact`.
    237	                let observed = peers[*peer].observations.iter().any(|(v, _)| v == version);
    238	                if observed {
    239	                    peers[*peer].redact_one(version);
    240	                    oracle.redact(*target_event_idx);
    241	                }

Resolution: (1) Expose `arb_overlap_schedule_with_shadow` returning the final `Knowledge` and add the overlap twin of `shadow_predicts_live_state` to this file. (2) Decide, with that test in hand, whether the shadow should snapshot at the first `Step` (the real fork point) rather than at `Open`, and fix the `Open` doc to say when the fork happens. (3) Only after the fork point matches the code, change the executor's guard to an assertion so a shadow mismatch fails instead of skipping. Acceptance: a committed test compares the overlap shadow's per-peer `observed_log`/`live` to the executor's; the executor asserts rather than skips; the `Open` doc states the fork point accurately.
Construction: Change `Knowledge::merge_session` in tests/common/overlap.rs to deliver nothing for a `Close`; today every property in session_overlap.rs still passes because the affected `Redact`s are never generated or are skipped. With the meta-test in place it fails.

Demonstration (`witness/results.md`): With `Knowledge::merge_session` replaced by a no-op in the `Close` arm, `overlapping_schedules_converge_to_the_oracle` passes all 48 cases; tests/shadow_validity.rs never references the overlap generator. Cross-reference: tests-common-12 describes the guard's design rationale (the shadow forks at `Open`, the live session at its first poll) and asks for a skip-count ceiling rather than an assertion.

### tests-observation-25: The conservation property never sheds: `messages_shed` is identically zero across the whole property
- Where: tests/session_stats.rs:297-335 (related: tests/session_stats.rs:91-116, tests/session_stats.rs:1-10, src/tree/mirror/streaming/stats.rs:82-98, src/tree/mirror/streaming/materialized/unknown.rs:94, src/tree/mirror/streaming/materialized/work/answer.rs:142, tests/common/action.rs:37)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read the strategy: two `vec(any::<u64>())` draws, no redaction; read `honored_redaction_counts_as_shed`, the suite's only nonzero-shed witness; `grep '\.shed('` lists the five count sites in unknown.rs and answer.rs)
- Seen by: blind-spots [25]; refutation: confirmed, severity down to low (each shed count site has a committed deterministic point witness, at the public tier or the walk tier); history: no rationale found (sends-only since 7626543e4)
- Owner-gated: no

`sessions_conserve_the_live_count` draws only `a_sends` and `b_sends`; no case redacts, so `messages_shed` is 0 in every generated session and the law the module doc advertises as this suite's contribution ("`len_after = len_before + gained - shed` over real sessions") is exercised with its shed term structurally absent. The only public-surface shed witness is one point test with one side shedding one message. A counter that miscounts under a family-specific shape (both sides shedding, sheds inside disputed scopes, pruned-subtree sheds adding `node.len()`) passes this property. Doctrine: a strategy must reach the interesting region of the law it states. I keep this at medium against the refutation's low because the property is the suite's only family-level statement of the law and the change is small; the point witnesses do evidence the counter's liveness.

Evidence:

    302	    fn sessions_conserve_the_live_count(
    303	        a_sends in proptest::collection::vec(any::<u64>(), 0..24),
    304	        b_sends in proptest::collection::vec(any::<u64>(), 0..24),
    305	    ) {
    ...
    322	            prop_assert_eq!(
    323	                a.snapshot().len() as u64,
    324	                a_before + a_g.stats.messages_gained - a_g.stats.messages_shed,
    325	            );

Resolution: Build both sides on a shared converged base and let each side act: a preamble of shared sends before the fork, then `a = build_local(bootstrap_fork(&seed), &a_actions)` and `b = build_local(bootstrap_fork(&seed), &b_actions)` with `arb_local_actions()` (which draws `Redact`), so cross-redactions of base messages held on the other side occur. Keep the conservation and byte-mirror assertions; add a point case with both sides shedding. Acceptance: over a default run some cases report `messages_shed > 0` on each side (pin with a dedicated point case); a mutant deleting the `stats.shed(1)` call at answer.rs:142 fails this suite.

Cross-reference: materialized-28 and streaming-tests-26.

### tests-observation-3: Causal tests pin a deterministic, replica-independent delivery order that the public `CausalMessages` doc declines to promise
- Where: tests/causal.rs:156-182 (related: tests/causal.rs:122-124, tests/causal.rs:143-153, src/rumors/causal.rs:18-21, src/rumors/causal.rs:54-60, src/rumors.rs:379-381)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (read both texts; `Version::rank` is public in `before`)
- Seen by: blind-spots [24]; refutation: confirmed; history: deliberate-and-holds (8dc0596ed on 2026-06-11 replaced "deterministic linear extension of the causal order" with the "may differ between replicas" sentence while moving engine internals out of user-facing prose; the tests, one day older, pin the internal invariant the private field doc still states)
- Owner-gated: yes: the resolution either amends the public contract or re-labels the tests

`delivery_order_is_replica_independent` asserts two converged replicas replay concurrent messages in the identical sequence, and `converged_backlog_has_no_inversions` asserts strictly increasing `(rank, canonical bytes)` order within a backlog. The public contract (src/rumors/causal.rs:18-21) says concurrent messages come "in arbitrary order, which may differ between gossiping replicas". The private field doc (54-60) says delivery is "causal and deterministic". A reader cannot tell which text is the specification of record: an implementation honoring the public promise with a different tiebreak fails these tests, and a user who could rely on replica-independent replay is not told. The goal stands beside the mechanism; here the test's doc ("the rank order is a property of the set") states a contract the public doc withholds.

Evidence:

    156	/// Two converged replicas deliver the same backlog in the *same* order to
    157	/// fresh observers: the rank order is a property of the set, not of the
    158	/// replica, the insertion order, or the gossip schedule.
    ...
    178	    assert_eq!(
    179	        from_a, from_b,
    180	        "identical sets replay identically, replica notwithstanding"
    181	    );

    (src/rumors/causal.rs)
    18	/// For any two yielded messages with versions `v` and `w`, if `v < w` then the
    19	/// `v` message is yielded first. Concurrent messages are delivered in arbitrary
    20	/// order, which may differ between [`gossip`](crate::Rumors::gossip)ing
    21	/// replicas of the same [`Rumors`](crate::Rumors).

Resolution: Owner ruling, then align both texts. (a) Promote: state in the `CausalMessages` rustdoc that concurrent messages are delivered in a deterministic order that is a function of the set alone (causal rank, then canonical version bytes), identical on every replica holding the same set; keep both tests as pins of that contract. (b) Demote: keep the public doc, and re-label the two tests' docs as pins of the engine's current staging order (an internal-entry check, documented at the site as a deliberate decision), or rewrite `delivery_order_is_replica_independent` to assert `assert_causal` on each side plus set-equality and delete the strict `(rank, bytes)` assertion at 146-153. Acceptance: the rustdoc on `CausalMessages` and the two test docs state the same order contract, or the tests declare themselves internal-order pins at the site.

### tests-observation-17: `retire_ends_the_observer`'s doc claims the final drain includes what the session learned; the session learns nothing
- Where: tests/listen.rs:241-243 (related: tests/listen.rs:246-248, tests/listen.rs:268-273, src/peer/gossip.rs:416-419)
- Class / severity / confidence: test-quality / medium / high
- Provenance: assessed (read 241-274: `survivor` is seeded and never sends; `retiree.send(7)` precedes the observer's creation at 250; the sole content assertion is that 7 appears)
- Seen by: structure-prose [9], blind-spots [26]; refutation: confirmed; history: no rationale found (the doc's second sentence restates the production promise at gossip.rs:416-419; the body has never exercised it since 040a0d042)
- Owner-gated: no

The doc's second sentence states the write-back-before-end ordering `retire` promises ("observers of a retiring set ... drain the *reconciled* final state — everything the session learned included — before they end", src/peer/gossip.rs:416-419). In the body the survivor holds nothing, so the session teaches the retiree nothing, and a from-genesis observer delivers message 7 whether or not the write-back precedes termination. An implementation that ended the observer on the pre-session state passes. AGENTS.md: the doc comment states the behavior the test protects; an inaccurate testdoc is a bug in the test, and this one states a production promise with no failing implementation in the suite.

Evidence:

    241	/// §6.9 (retire variant): retiring the rumor set ends its observers. The
    242	/// retire session's write-back lands the reconciled state first, so the
    243	/// observer's final drain includes everything the session learned.
    ...
    246	    let survivor = Peer::<u64>::seed().sync_window_floor().into_rumors();
    247	    let retiree = bootstrap_fork(&survivor);
    248	    retiree.send(7).unwrap();
    ...
    270	    assert!(
    271	        items.iter().any(|(_, m)| *m == 7),
    272	        "the final drain delivered the retiree's own message"
    273	    );

Resolution: After the observer subscribes (line 250) and before the retirement, `survivor.send(8).unwrap();`; after the drain assert that the items contain both 7 and 8, with 8 named as the message the session learned. Acceptance: ending the retiree's observer ahead of the write-back (or dropping learned content from it) fails the test on message 8.
Construction: Apply the two-line change above and temporarily swap the order of the write-back and the observer-ending step in `retire_inner`; the test must fail on 8. Without the change, the same swap passes.

### tests-observation-5: The causal interleaving strategy never redacts a remote or gossip-learned message; the unordered interleaving has no gossip
- Where: tests/causal.rs:359-386 (related: tests/causal.rs:405-423, tests/causal.rs:258-301, tests/listen.rs:528-552, tests/common/sim.rs:571)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read `Op`, `arb_ops`, and the executor loop: `sent` is pushed only in `SendA`; listen.rs's `Op` is Send, Redact, Drain)
- Seen by: blind-spots [28]; refutation: confirmed; history: no rationale found for causal.rs's redact scope; listen.rs's alphabet matches the deleted plan's specification, and the multi-peer regime is covered by `run_observers` in tests/common/sim.rs
- Owner-gated: no

`Op::Redact` indexes only `sent`, the versions A itself created; B never redacts, and A never redacts a message it learned from B. The region where deletion honoring interacts with the staged backlog (B redacts a message A has ingested and staged but not delivered, then gossip sheds it from A's tree before the pop) is unreachable, as is A redacting a learned message mid-backlog. The point test `staged_then_redacted_is_still_delivered` covers only the local-redaction variant. In listen.rs, `exactly_once_under_interleaving` has no gossip op, so the unordered observer's exactly-once claim under gossip-learned content plus redaction is sampled only by disruption.rs's concurrent observers. White-box worst-case construction: the shape that stresses the staging assumption belongs in the roster.

Evidence:

    366	    /// Redact the `idx % sent`-th message sent at `a` so far (dropped
    367	    /// if none).
    368	    Redact(usize),
    ...
    415	                Op::Redact(idx) => {
    416	                    if !sent.is_empty() {
    417	                        a.redact(&sent[idx % sent.len()]);
    418	                    }
    419	                }

Resolution: Extend causal.rs's alphabet with `RedactB(usize)` (B redacts one of its own sends, tracked in `sent_b`) and `RedactLearned(usize)` (A redacts the idx-th version of `a.snapshot()`), keeping the existing assertions. For listen.rs, either add `Gossip` and `SendB` ops mirroring causal.rs or state in the testdoc that the multi-peer regime is covered by `disruption.rs`. Acceptance: a default run's shrunk Debug output shows redactions of B-originated and A-learned versions; a point test pins the staged-then-shed shape (A stages both, B redacts one, gossip, drain).
Construction: In causal.rs add a point test: `a` seeds, `b = bootstrap_fork(&a)`, `b.send(1); b.send(2)`, `wire_gossip(&a,&b)`, `let mut obs = a.causal_messages(); step(&mut obs)` (ingests both, delivers one), `b.redact(&staged_version); wire_gossip(&a,&b); drain(&mut obs)`. The current strategy cannot generate this sequence; the point test states what the expected delivery is (the staged message is delivered once, per the ingest-liveness contract) and fails if the pop path re-reads the tree.

### tests-observation-7: `Changes` has only point tests, and never issues a no-op commit with an observer attached
- Where: tests/changes.rs:1-6 (related: src/rumors.rs:176-177, src/rumors.rs:224, src/batch.rs:129-144, src/tree/tests.rs:1123-1129)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read changes.rs in full; read `Batch::commit` and the tree-tier changed-flag property doc)
- Seen by: blind-spots [30]; refutation: reframed (the changed flag is already pinned `false` on empty and unheld-forget batches at the tree tier, `act_changed_flag_tracks_the_root_hash`; what is missing is the end-to-end pin that the flag suppresses the wakeup, and a family-level property); history: no rationale found
- Owner-gated: no

The module doc states a family claim ("exactly one coalesced tick per observed frontier advance (however many commits that was)") and the suite pins it with eight fixed sequences and no property. Two documented no-op commits are never issued with an observer attached: a redact of an unheld version (src/rumors.rs:176-177) and `send_all` of an empty iterator (src/rumors.rs:224, "wakes no observer"). `Batch::commit` passes `act`'s flag straight to `send_if_modified` (src/batch.rs:140-143), so a `commit` that returned `true` unconditionally would escape this suite. AGENTS.md: when the claim is a family, state it as a proptest invariant.

Evidence:

    3	//! Pins the contract stated on the type: an immediate first yield, exactly
    4	//! one coalesced tick per observed frontier advance (however many commits
    5	//! that was), ticks for every kind of commit — send, redact, and a join
    6	//! learned by gossip — and a clean end once the set closes.

    (src/rumors.rs)
    224	    /// it. An empty iterator commits nothing and wakes no observer.

Resolution: Add a proptest over `Vec<{Send(u64), SendAll(0..=3 values), RedactHeld(idx), RedactUnheld, Poll}>` that records `latest()` at each Poll and asserts `try_next() == Tick` iff `latest` differs from the last reported one, `Quiet` otherwise. Add two point tests: `redact` of a version the set never held and `send_all(std::iter::empty::<u64>())` both leave a reported signal `Quiet`. Acceptance: a mutant making `Batch::commit`'s closure return `true` unconditionally fails the new tests.
Construction: In tests/changes.rs, `let rumors = Peer::<u64>::seed().sync_window_floor().into_rumors(); let mut changes = rumors.changes(); assert_eq!(changes.next().now_or_never(), Some(Some(()))); rumors.send_all(std::iter::empty::<u64>()).unwrap(); assert_eq!(changes.next().now_or_never(), None);` and the same with `rumors.redact(&Version::new())`. Today nothing in the suite makes either call with an observer attached.

### tests-observation-23: The overlap sweep and the pincer motif both run one orientation; the dual is reached only by the random soup
- Where: tests/session_overlap.rs:61-134 (related: tests/session_overlap.rs:113-121, tests/common/overlap.rs:480-500)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (read the sweep body and `Pincer::choices`: both open the converged pair, run the mutating session inside the park, then close)
- Seen by: blind-spots [31]; refutation: confirmed; history: no rationale found (both instruments were built in the discovering incident's orientation)
- Owner-gated: no

The sweep opens the innocent session A<->C, parks it after `n` polls, runs the redacting session A<->B to completion, then finishes A<->C. `Pincer::choices` emits the same orientation. The mirror image, the redacting session forked first and installing last over the innocent one, is a distinct install-time interleaving (its merged frontier carries the redaction tick while A's current tree has been re-joined with C's fork-time copy) and is sampled only when the soup happens to produce it. Doctrine asks for the dual family per operation pair; the module doc names the symptom as orientation-independent.

Evidence:

    113	            let s2 = {
    114	                let mut s2 = overlap::open(&a, &c);
    115	                s2.step(n);
    116	                s2
    117	            };
    118	            // ...S1 (A <-> B) installs the honored redaction at A...
    119	            wire_gossip(&a, &b);
    120	            // ...and S2 resumes and installs after it.
    121	            s2.finish();

Resolution: Add a second arm to the sweep with A<->B opened first and parked at `n`, `wire_gossip(&a, &c)` run whole, then the parked session finished, against the same `expected`. In `Pincer::choices`, draw an orientation bit and emit the mirrored sequence (Open x<->w mutating, Step, Gossip x<->y, Close) when set. Acceptance: both orientations are swept over every (target, n) pair; the pincer strategy's Debug output shows both orientations in a default run.
Construction: Duplicate lines 110-121 with the roles of `b` and `c` swapped in the open/park/finish structure (open A<->B after `b.redact`, park, `wire_gossip(&a, &c)`, finish) and run the sweep; today no committed test executes that sequence deterministically.

### tests-observation-36: The fleet-scale retirement test constructs content movement and asserts only identity
- Where: tests/party_conservation.rs:402-441 (related: tests/party_conservation.rs:409-413, tests/common/oracle.rs:91-100, tests/retire.rs:67-70)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read 414-441: the only `prop_assert!` is on `survivor == Party::seed()`; each newcomer sends `i as u64` once)
- Seen by: blind-spots [27]; refutation: confirmed; history: deliberate-and-holds for the file's charter (identity invariants, line 1; 574559635 asserts exactly the survivor's party), with the comment at 409-413 reading as a claim the test does not check
- Owner-gated: no

The test grows a fleet of up to a hundred peers, has each newcomer originate one unique value "so retirements move content, not just identity", retires everyone into one survivor, and asserts only that the survivor's party is `Party::seed()`. Nothing checks that the survivor holds the originated messages, so a retirement path that moves identity but drops content passes here; content-moving retirement is asserted elsewhere only at small scale (retire.rs's `retire_into_counting_gossip`). The comment states an observable the test does not assert; either assert it (cheap) or reword the comment to say the sends exist to exercise the retire path under content.

Evidence:

    409	        // Grow the fleet one bootstrap at a time through `apply`, which
    410	        // resolves provider indices modulo the live fleet: any random
    411	        // topology (chain, fan, or mixture) is reachable. Each newcomer
    412	        // originates once (`Bootstrap` pushes it at the fleet's end), so
    413	        // retirements move content, not just identity.
    ...
    435	        let survivor = alias(&fleet[0]);
    436	        prop_assert!(
    437	            survivor == Party::seed(),

Resolution: After the retirement loop, `prop_assert_eq!(readout_multiset(&fleet[0].snapshot()), (0..providers.len() as u64).map(|i| (i, 1)).collect::<BTreeMap<_, _>>())` (importing `readout_multiset` from `crate::common::oracle`), or at least `prop_assert_eq!(fleet[0].snapshot().len(), providers.len())`. Acceptance: a mutant that skips content reconciliation in the retire path fails this test.
Construction: Temporarily make the retiree's write-back in `retire_inner` skip the gossip round's content while still donating the party; today this test passes; with the readout assertion it fails.

### tests-observation-8: changes.rs runs under `#[pollster::test]`, bypassing the closed-world poller, and four of its async tests never await
- Where: tests/changes.rs:14-19 (related: tests/changes.rs:29-54, tests/changes.rs:58-69, tests/changes.rs:73-85, tests/changes.rs:103-137, tests/changes.rs:142-154, tests/changes.rs:158-163, tests/listen.rs:309-314, tests/common/wire.rs:40-42, .config/nextest.toml:1-5)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (`grep -n '\.await' tests/changes.rs` gives only lines 76, 83, 106, 123, 162; read `block_on` in wire.rs and the nextest.toml design statement)
- Seen by: structure-prose [13], api-economics [43]; refutation: confirmed; history: no rationale found (the swap from `tokio::test` to pollster landed in 83edcd944, a WIP commit with no message body, the same commit that introduced `run_to_quiescence`)
- Owner-gated: no

Every other suite in the partition drives sessions through `common::wire::block_on`, which is `run_to_quiescence(..).expect(..)`: a wire stall fails at the stalled poll with a verdict. changes.rs runs seven tests under pollster's executor instead, so a stall in `bootstrap_fork_async`/`wire_gossip_async` would park the thread until nextest's 180 s kill with no diagnosis. Four of those tests (`first_poll_yields_immediately`, `one_tick_per_observed_commit`, `unpolled_commits_coalesce_to_one_tick`, `set_closure_ends_the_stream`) have no `.await` at all, and `observer_does_not_block_peer_reclaim` awaits `try_into_peer()` so that a regression (an observer counted against quiescence) fails by hang, where listen.rs:309-314 makes the identical claim with `.now_or_never().expect(..)` and fails crisply. The sync `bootstrap_fork`/`wire_gossip` already serve listen.rs and causal.rs.

Evidence:

    14	use crate::common::wire::{bootstrap_fork_async, wire_gossip_async};
    ...
    18	#[pollster::test]
    19	async fn first_poll_yields_immediately() {

    158	#[pollster::test]
    159	async fn observer_does_not_block_peer_reclaim() {
    160	    let rumors: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
    161	    let _changes = rumors.changes();
    162	    assert!(rumors.try_into_peer().await.is_some());
    163	}

Resolution: Make all seven tests plain `#[test] fn`, call `bootstrap_fork`/`wire_gossip`, and in the reclaim test use `rumors.try_into_peer().now_or_never().expect("an observer does not count against quiescence")`. Drop the pollster import from this file (handshake.rs and gossip_when.rs keep the dev-dependency unless they follow). This also removes changes.rs from the `testdoc` hole in tests-observation-37. Acceptance: `grep -c pollster tests/changes.rs` is 0; the seven tests pass unchanged in behavior.

Cross-reference: this change also removes seven of the 29 sites the testdoc hole covers (tests-observation-37).

### tests-observation-9: The received-side mirror check lacks the handler-count equality its sent-side twin has
- Where: tests/observe.rs:205-227 (related: tests/observe.rs:173-200, tests/observe.rs:123-143)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read both helpers; only `assert_mirrors` has the length equality)
- Seen by: blind-spots [29]; refutation: confirmed; history: no rationale found (both helpers date from the hook's first commit)
- Owner-gated: no

`assert_mirrors` asserts `sent.len() == capture.streams.len()` before matching per index, but `assert_received_mirrors_remote` only iterates the remote's captured streams and finds a matching received handler for each; a phantom received data-stream handler at an index the peer never spoke passes. `SessionRecord::data` rejects duplicate indices per direction, so only the extra-index case escapes. The module doc promises "one handler per directed data stream" in both directions; the received direction is the dual and should carry the same totality check.

Evidence:

    180	    assert_eq!(
    181	        sent.len(),
    182	        capture.streams.len(),
    183	        "{side}: one sent data handler per opened transport stream"
    184	    );
    ...
    215	    let received = local.data(Direction::Received);
    216	    for blob in &remote_capture.streams {
    217	        let ((_, index), label_len) = stream_label(blob);
    218	        let Some((_, _, bytes)) = received.iter().find(|(i, ..)| *i == index) else {
    219	            panic!("{side}: no received handler for the peer's data stream {index}");

Resolution: Add `assert_eq!(received.len(), remote_capture.streams.len(), "{side}: one received data handler per stream the peer opened");` before the loop at 216. Acceptance: a mutant that invokes a received data-stream handler for an index the peer never opened fails `assert_received_mirrors_remote`.

### tests-observation-14: `bootstrap_sessions_are_observed` omits the `assert_election` check its retire sibling makes
- Where: tests/observe.rs:436-444 (related: tests/observe.rs:482, tests/observe.rs:406-411, src/tree/mirror/streaming/remote/proxy/start.rs:356-376, src/peer/gossip.rs:1152-1165)
- Class / severity / confidence: test-quality / low / medium
- Provenance: assessed (read start.rs: the election fires whenever greeting versions differ, before any data stream opens; read the provider path's handshake at gossip.rs:1152; in the fixture the provider holds one message and the newcomer is at genesis)
- Seen by: structure-prose [10]; refutation: confirmed; history: no rationale found (the asymmetry dates from the hook's first commit, whose message lists the election check only for the gossip proptest)
- Owner-gated: no

The retire pairing ends with `assert_election(&absorber_session, &retiree_session)`; the bootstrap pairing does not. A bootstrap session runs the same handshake and descent, and with a seeded provider facing an empty newcomer the greeting versions differ, so both sides elect and data streams open; the election and speaker agreement are checkable here too. The two fixed pairings exist to cover the session kinds end to end (module doc 16-18); the election is part of that view.

Evidence:

    438	    assert_eq!(provider_session.info.kind, SessionKind::Gossip);
    439	    assert_eq!(newcomer_session.info.kind, SessionKind::Bootstrap);
    440	    assert_mirrors("provider", &provider_session, &provider_capture);
    441	    assert_mirrors("newcomer", &newcomer_session, &newcomer_capture);
    442	    assert_received_mirrors_remote("provider", &provider_session, &newcomer_capture);
    443	    assert_received_mirrors_remote("newcomer", &newcomer_session, &provider_capture);
    444	}

Resolution: Add `assert_election(&provider_session, &newcomer_session);` and mention the election in the doc at 406-411; if the bootstrap pairing deliberately skips it, say why at the site. Acceptance: both fixed-pairing tests call `assert_election`, or the bootstrap test's doc states why the election is out of scope.

### tests-observation-20: `checkpoint_resume_loses_nothing`'s doc states a mid-pass bound the body admits is tautological, and neither run is checked for internal duplicates
- Where: tests/listen.rs:616-620 (related: tests/listen.rs:679-692, tests/causal.rs:454-459, tests/causal.rs:510-520)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read 612-693: `first_versions`/`second_versions` are `BTreeSet`s, so a within-run duplicate collapses unchecked; the comment at 691-692 says the mid-pass bound holds by construction)
- Seen by: api-economics [50]; refutation: confirmed; history: deliberate-and-holds for the omitted assertion (the author wrote the 691-692 comment at birth); the doc sentence was carried from the plan's wording
- Owner-gated: no

The doc promises that mid-pass "re-deliveries are permitted but only for messages the interrupted pass already delivered"; the body computes `redelivered = first ∩ second`, and its own comment notes `redelivered ⊆ first_versions` "holds by construction", so the stated property is not tested. Neither run is checked for duplicates within itself, so a `_since(checkpoint)` observer re-yielding one message twice passes here. The doc should name only checked claims; the substantive mid-pass claim worth pinning is that each run is itself exactly-once.

Evidence:

    616	    /// If the
    617	    /// stop fell *mid-pass*, re-deliveries are permitted but only for
    618	    /// messages the interrupted pass already delivered (at-least-once); if
    619	    /// the observer had *completed* its pass, nothing from it is
    620	    /// re-delivered (exactly-once across completed passes).
    ...
    691	        // (Mid-pass, `redelivered ⊆ first_versions` holds by construction;
    692	        // the loss-freedom assertion above is the substantive check.)

Resolution: Reword the doc to the two checked claims (the union covers the final live set; nothing re-fires after a completed pass) and add `prop_assert_eq!(first_versions.len(), first_run.len())` and `prop_assert_eq!(second_versions.len(), second_run.len())` so each run is duplicate-free; delete the 691-692 comment. Mirror the duplicate check in causal.rs:510-520. Acceptance: the doc names only checked properties; a within-run duplicate fails the test.

### tests-observation-24: `LABEL_LEN = 2` transcribes a label length the code computes, and is true only for session epochs below 24
- Where: tests/session_stats.rs:246-249 (related: tests/session_stats.rs:277-288, src/tree/mirror/streaming/remote/streams.rs:62-67, src/link.rs:390-391, src/link.rs:428, src/tree/mirror/streaming/remote/codec/capture.rs:117-121, tests/observe.rs:186, tests/observe.rs:217)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read `label()`: two CBOR uint heads sized by `cbor::head_len`; a fresh link starts at epoch 0 and advances one per session; `stream_label` returns the parsed label length and observe.rs uses it at two sites)
- Seen by: structure-prose [21], api-economics [47]; refutation: confirmed; history: already-known (R4 of the CBOR-wire review ruled the two-byte assumption wrong for epoch >= 24 and fixed tests/observe.rs; its site list omitted session_stats.rs)
- Owner-gated: no

The doc presents the label as a fixed two bytes, but the label is two CBOR unsigned-int heads, so an epoch of 24 or more encodes in two bytes and the label becomes three. The test runs one session on a fresh link (epoch 0), so it passes; the constant encodes a fixture fact as a protocol fact, and `rumors::testing::stream_label` already returns the parsed length. A quantity computable two ways gets computed one way or compared; here one suite computes and the other transcribes.

Evidence:

    246	/// Bytes of the label a sender writes before its first frame; the codec
    247	/// seam's counters exclude it, so the transport tally exceeds
    248	/// `bytes_sent` by exactly this much per opened stream.
    249	const LABEL_LEN: usize = 2;

    (src/tree/mirror/streaming/remote/streams.rs)
    64	    let mut label = Vec::with_capacity(cbor::head_len(u64::from(epoch)) + 1);
    65	    cbor::write_head(&mut label, MAJOR_UINT, u64::from(epoch));
    66	    cbor::write_head(&mut label, MAJOR_UINT, u64::from(stream.index()));

Resolution: Have `CountingWrite` retain each stream's first write and subtract `stream_label(first_bytes).1` per opened stream, or drive the test through `capture_sides` (which yields per-stream blobs) and sum `stream_label(blob).1`; delete `LABEL_LEN` and reword the doc to say the label length is parsed. Acceptance: no literal label length remains in session_stats.rs; the byte-tally assertion derives label bytes from `stream_label`.

### verification-infra-17: The Changes observer, the routed link, and the handshake state family claims as point tests
- Where: tests/changes.rs:3-6 (related: tests/routed_link.rs:92-100, tests/handshake.rs:1-12, src/link/routed/tests.rs, src/link/routed/header/tests.rs:117-125, src/conformance/link.rs)
- Class / severity / confidence: test-quality / low / medium
- Provenance: verified (a `grep -c proptest` census over every file in tests/: 33 of 57 suites use none; `src/link/routed/tests.rs` has 15 `#[test]`s and no proptest; the conformance suite under `src/conformance/` uses no proptest, so the routed-link tests that call `rumors::conformance::link::check` run a fixed schedule; only the header codec has a socket-address round-trip property)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

Most point-only suites are deliberately point pins (snapshots, meters,
static bounds). Three hold family-shaped claims with points: the Changes
observer pins "exactly one coalesced tick per observed frontier advance
(however many commits that was)" with one test; the routed link layer runs
the conformance suite's fixed schedule and one mesh test, with no property
over connect/accept/label interleavings; the handshake suite pins hand
transcribed preambles. When the claim is a family, stating it as a
proptest invariant is what lets the shrunk counterexample ride along as a
committed seed.

Evidence:

    tests/changes.rs
         3	//! Pins the contract stated on the type: an immediate first yield, exactly
         4	//! one coalesced tick per observed frontier advance (however many commits
         5	//! that was), ticks for every kind of commit — send, redact, and a join
         6	//! learned by gossip — and a clean end once the set closes.

    tests/routed_link.rs
        92	/// Run the whole conformance suite against fresh TCP pairs at the
        94	async fn tcp_conformance(buffers: Option<u32>, dialer_first: bool) {
        97	        rumors::conformance::link::check(async || tcp_pair(buffers, dialer_first).await),

    census: grep -c proptest tests/changes.rs tests/routed_link.rs tests/handshake.rs src/conformance/link.rs -> 0 0 0 0

Resolution: add a Changes property over generated commit sequences and poll
points asserting the coalescing law, and a routed-link property over
generated connect/accept/label schedules against the in-memory network
(the conformance suite as oracle); keep the point tests as named corners.
Acceptance: each file carries a `proptest!` block whose doc states the
family invariant.

### Nits

The full record of each (provenance, history, evidence) is in the evidence file named in the last column.

| Id | Where | Claim | Resolution | Record |
|---|---|---|---|---|
| tests-observation-22 | `tests/session_overlap.rs:29-37` | `converged_trio` seeds `a` at the default window while every other fixture pins the floor, with no stated reason | pin the floor or state the intent | `evidence/partitions/tests-observation.md` |

## Integration tests: disruption, gossip_when, pipelining, hop trace, handshake, and handshake liveness

Two driver terminal arms for a partially staged preamble have no test anywhere; five negatives rest on 100 ms wall-clock windows in a file whose module doc says "no timers anywhere"; the handshake suite discards the preamble it calls itself an oracle for; the chaos interleaving has no redaction arm; the inter-process cut range has no pin of its own; and two hop instruments measure one fixture and are never compared.

### tests-disruption-handshake-15: Two driver terminal arms (when-exhaustion with staged preamble bytes; drop with staged bytes) have no test anywhere
- Where: tests/gossip_when.rs:438-442 (related: src/peer/gossip.rs:990-997, 1355-1373; src/tree/mirror/handshake.rs:265-280; src/peer/gossip/tests.rs:196-200)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (grep for `staged`, `Tick(None)`, `poison`, `drop(` in src/peer/gossip/tests.rs: the only `Staged` use is the bootstrap helper at 196-200; grep -rln 'gossip_when(' tests src: tests/gossip_when.rs, tests/session_stats.rs, src/rumors.rs, src/tutorial.rs, all over 8 KiB or 64 KiB links where a 30-byte write lands whole; read the two arms)
- Seen by: blind-spots [23]; refutation: confirmed (Staged::fill reads into the 30-byte buffer with one `read`, so no existing link capacity can leave a partial fill); history: no rationale found (bc9341c1 mentions Drive::drop "keeps covering the dropped-driver case" without citing a test)
- Owner-gated: no

when_exhaustion_then_hangup_both_end_cleanly covers `Trigger::Tick(None)` only with an empty staging buffer. The sibling arm at gossip.rs:994-997 (the `when` stream ends while a remote preamble is partially staged: serve that session, then end) and Drive's Drop at 1367-1373 (poison the link when dropped with staged bytes) exist for the partially-consumed-preamble state, which no test constructs; every gossip_when test in the tree runs over a link wide enough for the preamble to arrive in one read. A regression in either arm (returning None and stranding the peer mid-preamble; leaving the link unpoisoned so the next session misreads the remainder) passes the suite. The driver's own doc calls it "the case only this drop can see"; a branch whose justifying state no test reaches is unpinned, and the construction is cheap and deterministic.

Evidence:

       438	/// When the `when` stream ends with nothing in flight, the driver ends
       439	/// cleanly — and its still-running counterparty sees the dropped connection
       440	/// as a clean goodbye (end-of-stream at a session boundary), not an error.
       441	#[tokio::test(flavor = "current_thread")]
       442	async fn when_exhaustion_then_hangup_both_end_cleanly() {

    src/peer/gossip.rs
       993	                        Trigger::Tick(None) if drive.staged.is_empty() => return None,
       994	                        Trigger::Tick(None) => {
       995	                            drive.done = true;
       996	                            Led::Remote
       997	                        }
       ...
      1367	impl<T, B: BookmarkError, S> Drop for Drive<'_, T, B, S> {
      1368	    fn drop(&mut self) {
      1369	        if !self.staged.is_empty() {
      1370	            self.state.poison();
      1371	        }
      1372	    }
      1373	}

Resolution: Over `rumors::link::memory_with_capacity(1)`, let B run one-shot `gossip` and poll it a few times so its preamble crosses byte by byte; poll A's driver (`a_sessions.next().now_or_never()`) so `staged` holds between 1 and 29 bytes (assert this from the fixture, e.g. via a metered link). Then (a) drop A's cue sender and drive both under `run_to_quiescence`: A yields `Ok(Gossiped { led: Led::Remote, .. })` then None, B gets Ok; (b) in a second test drop A's driver instead and assert `run_to_quiescence(a.gossip(&mut a_link))` is `Ok(Err(Error::LinkPoisoned))`, while the existing empty-boundary drop leaves the link reusable. Acceptance: two new tests whose fixture asserts the staged byte count is strictly between 0 and 30; mutating gossip.rs:994-997 to `return None` and 1369-1371 to a no-op each fails exactly one of them.
Construction: As in the resolution; the capacity-one link is the only shape that can leave a partial fill, since `Staged::fill` reads with one `read` per poll.

### tests-disruption-handshake-14: Five negative assertions rest on 100 ms wall-clock windows that pass on expiry; the module doc says "no timers anywhere"
- Where: tests/gossip_when.rs:180-184 (related: tests/gossip_when.rs:4-6, 45, 220-224, 283-287, 293-297, 398-402, 519, 552, 560, 596; src/testing.rs:366-394)
- Class / severity / confidence: test-quality / medium / high
- Provenance: assessed (read; mechanism from tokio's Timeout, which polls the inner future before the sleep; run_to_quiescence's coop handling read at src/testing.rs:372-381)
- Seen by: structure-prose [0], blind-spots [24], api-economics [38]; refutation: confirmed; history: deliberate but expired (written at e43edcd7 before any stall detector existed; run_to_quiescence was imported into this file at 459cff1a for one test without converting the negatives)
- Owner-gated: no

The claims "no echo session", "no heartbeat session", "no observer woke", "no when-changed session", and "a converged chain is quiet" are each established by awaiting a 100 ms timeout and asserting it expired. Timeout polls the inner future first, so an erroneous session still in flight at the deadline yields Pending, the sleep fires, and the negative passes: the failure direction is a false pass under load, and each site spends 100 ms of real time. The file already proves negatives deterministically with `run_to_quiescence` at 519, 552, 560, and 596 (Err(Quiescence::Stalled) when the future parks with no wake arranged); the detector disables tokio's cooperative budget around the subject, so it is safe inside a tokio test. The module doc's "no timers anywhere" was scoped to cue timing at birth, but with `use tokio::time::timeout` at 45 and five verdicts resting on a timer a reader takes it suite-wide. Doctrine: test verdicts should read the same under any machine load; a threshold over wall time relocates the flakiness to the threshold.

Evidence:

         4	//! Every test drives the policy stream by hand — a `futures` mpsc channel
         5	//! whose receiver is the `when` stream — so initiation timing is fully
         6	//! deterministic with no timers anywhere. The suite pins the driver's whole

       180	    let echo = futures::future::join(a_sessions.next(), b_sessions.next());
       181	    assert!(
       182	        timeout(Duration::from_millis(100), echo).await.is_err(),
       183	        "an echo session ran on a converged connection"
       184	    );

Resolution: Replace each negative window with `assert!(matches!(run_to_quiescence(futures::future::join(a_sessions.next(), b_sessions.next())), Err(Quiescence::Stalled)), "...")` (rumors::testing::Quiescence is public), moving the affected tests onto `common::wire::block_on` if calling the poller inside a runtime reads awkwardly; keep the 10 s DEADLINE only on positive waits, which fail loudly on expiry. Rewrite lines 4-6 to say what is true: cues are hand-fed and negative checks are stall-detected, with wall-clock deadlines remaining only as watchdogs on positive waits. Acceptance: no `from_millis(100)` remains in the file; each former negative names `Quiescence::Stalled`; the module doc no longer claims "no timers anywhere".
Construction: In a scratch copy make `Trigger::Tick(Some(Gossip::WhenChanged))` in src/peer/gossip.rs always initiate and insert `std::thread::sleep(Duration::from_millis(150))` in the driver's first poll; suppression_swallows_echoes_not_news passes at line 182 today, while the quiescence form fails on the first run.

### tests-disruption-handshake-22: The handshake suite discards alice's preamble at every fake-peer site, repeats a twelve-line scaffold six times, and never checks the local set
- Where: tests/handshake.rs:94-97 (related: tests/handshake.rs:5-6, 10-12, 125-126, 161-162, 190-191, 225-226, 258-259; src/tree/mirror/handshake.rs:98-107; src/network.rs:80; src/tree/mirror/handshake/tests.rs:244-246, 268-281)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (grep for `got` in tests/handshake.rs: filled by read_exact at six sites and never read afterwards; grep BootstrapRetireConflict tests/: no hits; src/network.rs:80 shows `to_bytes` is pub(crate))
- Seen by: structure-prose [5], blind-spots [29], api-economics [39]; refutation: confirmed (the rejection variants pin layout positions indirectly, so it is half an oracle, not none); history: no rationale found (`got` never asserted since 8b1ade01; the "independent oracle" sentence was added at 4dd2053c over an inbound-only suite)
- Owner-gated: no

The module doc calls the suite "an independent oracle of the documented wire spelling", but every fake peer reads alice's 30 bytes into `got` and drops them: the encoder's spelling is never compared to the hand-transcribed layout, and the decoder is exercised only through rejections. The doc also promises errors surface "rather than corrupting the local rumor set", and no test inspects alice's set after an error. The remaining preamble diagnosis, `Error::BootstrapRetireConflict`, is unit-tested but has no end-to-end mapping here. Six copies of the same scaffold (84-110, 115-147, 153-177, 183-210, 218-245, 251-276) differ only in the reply bytes and the expected variant. The written preamble is pinned elsewhere (the unit test prefix_matches_the_writers and the insta snapshots), so the wire is not unprotected; this suite does not do what it says it does.

Evidence:

        10	//! The layout is transcribed here by hand, deliberately: this suite is an
        11	//! independent oracle of the documented wire spelling, so it must not
        12	//! derive the bytes from the code under test.

        94	        let mut got = [0u8; PREAMBLE_LEN];
        95	        b_r.read_exact(&mut got).await.expect("fake peer read");
        96	        let reply = preamble(bad_opening, Protocol::V2 as u8, INTENT_REMAIN);
        97	        b_w.write_all(&reply).await.expect("fake peer write");

Resolution: Add one helper `async fn alice_against(reply: &[u8]) -> (Result<Gossiped, Error>, Rumors<String>)` that builds alice, runs the fake peer, asserts `got[..13] == preamble(V2_OPENING, Protocol::V2 as u8, INTENT_REMAIN)[..13]` and `got[29] == INTENT_REMAIN` (bytes 13..29 are the universe's network id, which has no public accessor; skip them), and drops the write half after writing (the truncation case passes `&partial[..6]`). Each rejection test becomes: build the reply, call the helper, match the variant, assert `alice.snapshot().is_empty()`. Add a `bootstrap_retire_conflict_surfaces_error` case (network `[0; 16]`, intent 1). Acceptance: one helper and six tests of a few lines each; changing `Protocol::V2 as u64` to `3` in `Preamble::encode` (src/tree/mirror/handshake.rs:102) fails every rejection test, not only the snapshot suite; every error test asserts the local set is unchanged; `Error::BootstrapRetireConflict` appears in tests/handshake.rs.
Construction: In a scratch copy set the encoded version to 3 at handshake.rs:102; today only handshake_roundtrip_succeeds fails (both sides real) and no rejection test observes the outgoing byte.

### tests-disruption-handshake-3: The inter-process family draws its cuts from the intra-process envelope's range and has no pin of its own
- Where: tests/disruption.rs:526-539 (related: tests/common/sim.rs:104, 286; tests/disruption.rs:422-448, 568-585)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read)
- Seen by: blind-spots [32]; refutation: reframed (whether most cuts miss is unmeasured; the reconstructed plans' cut offsets near 2000 are evidence that sessions reached such offsets when found); history: no rationale found
- Owner-gated: no

arb_child_plan draws every cut through arb_fault, whose range `0..MAX_CUT` (3072) is pinned two-sided against the intra-process envelope session (five peers, tens of unique values) by max_cut_spans_the_envelope_session. The inter-process population is different: at most three seed messages in the parent and five sends per child. No measurement says where MAX_CUT sits relative to a child's bootstrap, sessions, and retirement, so the family has no liveness floor of its own; the intra-process leg's discipline ("re-measure there before touching this number", sim.rs:103) does not reach it.

Evidence:

       526	fn arb_child_plan(faults: bool) -> impl Strategy<Value = ChildPlan> {
       527	    (
       528	        0usize..6,
       529	        arb_fault(faults),
       530	        prop::collection::vec(arb_fault(faults), 1..4),
       531	        arb_fault(faults),
       532	    )

    tests/common/sim.rs
       286	    let cut = prop_oneof![2 => Just(None), 3 => (0..MAX_CUT).prop_map(Some)];

Resolution: Meter one clean child cycle (bootstrap, session, final gossip, retire) against the parent through `fault::metered` over TCP or the in-memory link, and pin a named inter-process bound from both sides as max_cut_spans_the_envelope_session does; draw arb_child_plan's cuts from that bound. Acceptance: a named constant and a two-sided pin exist for the inter-process family, and arb_child_plan draws from it.
Construction: Add a test that runs `child_main`'s clean path against a metered link and prints the per-endpoint byte count; compare to MAX_CUT. Whichever way the comparison falls, the number is currently unknown, which is the gap.

### tests-disruption-handshake-10: Two hop instruments measure the same fixture and are never compared; HOP_BUDGET's floor figures are derived, not measured
- Where: tests/gossip_pipelining.rs:33-41 (related: tests/gossip_pipelining.rs:55-61; tests/hop_trace.rs:495-505; tests/window_corners.rs:118-146; benches/support/latency.rs:515-522)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read both fixtures and both assertions; read window_corners.rs:126-138 and window.rs:300-319 for the budget-0 floor equivalence)
- Seen by: structure-prose [4], blind-spots [28], api-economics [40], [42]; refutation: 4 and 40 confirmed, 42 reframed (the floor regime is measured in window_corners at budget 0, which floors every capacity at one, but on a different fixture); history: the loose bound's coexistence with the exact pin is deliberate and holds (HOP_BUDGET's own doc: the headroom "admits a deeper engaged ladder, never wave costs"; 814f07ad tightened 64 to 24 knowing the exact figure), so dissolving the test is an owner question, not cleanup
- Owner-gated: no (the equality and the measured floor are additive; whether the loose bound stays is the open question below)

gossip_pipelining asserts `session_hops(...) < 24` through the bench instrument and hop_trace pins `trace.hops() == 7` through its tracer, on a byte-identical fixture (finding 31); the two readings of one quantity are never equated, so a divergence between `latency::session_hops` and the tracer would go unnoticed. HOP_BUDGET's rationale quotes a floor cost ("≥ 500 hops") and two ratios ("3.4×", "20×") that no committed test measures on this fixture: the floor was measured once in a commit message (818a8707, about 370 hops) and restated as arithmetic. Doctrine: any quantity computable two ways gets a committed comparison, and a criterion's known-bad demonstration belongs on the criterion's own fixture with a measured number.

Evidence:

        35	/// A pipelined descent measures 7 exact hops (the phase ladder's few
        36	/// active levels); the bound's headroom admits a deeper engaged ladder,
        37	/// never wave costs. A floor-window descent pays one round trip per
        38	/// disputed scope — here ≥ ~250 scopes, hence ≥ 500 hops — so the bound
        39	/// sits 3.4× above the pipelined measurement and the serialized regime
        40	/// sits 20× above the bound.
        41	const HOP_BUDGET: u32 = 24;

        55	    let measured = latency::session_hops(LINK_CAPACITY, DELAY, diverged_pair());

    tests/hop_trace.rs
       504	    assert_eq!(trace.hops(), 7, "insertion-shaped session hop count");

Resolution: Once the fixture is shared (finding 31), add one committed equality `latency::session_hops(CAPACITY, DELAY, pair) == trace.hops()` tying the two instruments, and add a floor leg on the same fixture (`sync_window_floor()` both sides) asserting `floor_measured > HOP_BUDGET`; let HOP_BUDGET's doc quote the measured floor figure and drop the hand-derived ratios. Acceptance: one committed equality between the two hop instruments exists; a test fails if the floor drops below the budget; the constant's doc contains no arithmetic over unmeasured numbers.
Construction: Build `diverged_insertions()` in hop_trace, run it through both `traced_session` and `latency::session_hops(CAPACITY, DELAY, ...)`, and assert equality; for the floor, build the same pair with `.sync_window_floor()` and print `session_hops`.

Cross-reference: verification-infra-16 is the sweep's statement of the same unpinned floor regime.

### verification-infra-16: The pipelining hop budget's known-bad regime is stated in prose, not driven by a committed red cell
- Where: tests/gossip_pipelining.rs:33-41 (related: tests/gossip_pipelining.rs:50-51, tests/window_knee.rs:1-16, tests/future_size.rs:26-32)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (read the file in full: one `#[test]`, measuring only the production window; read window_knee's module doc and test roster, which does carry the above-knee direction for its own shapes)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The budget's adequacy rests on a figure at the constant (a floor-window
descent pays ≥ ~500 hops, 20× above the budget), but no committed cell
drives the floor window and asserts it reads above the budget. The window
suite is not blind (`window_knee` pins wave growth above its knee), but
this pin lacks its own demonstration that a serialized descent reads red.
`tests/future_size.rs` has the same shape (a budget calibrated against
"a few hundred bytes" in prose), though its known-bad artifact is not
constructible from outside the crate; there the fix is to pin the measured
sizes as constants beside the budget.

Evidence:

    tests/gossip_pipelining.rs
        37	/// never wave costs. A floor-window descent pays one round trip per
        38	/// disputed scope — here ≥ ~250 scopes, hence ≥ 500 hops — so the bound
        39	/// sits 3.4× above the pipelined measurement and the serialized regime
        40	/// sits 20× above the bound.
        41	const HOP_BUDGET: u32 = 24;
        50	#[test]
        51	fn window_pipelines_disputed_scopes() {

    tests/future_size.rs
        28	/// The budget is set generously above the measured sizes (a few hundred bytes)
        29	/// so legitimate growth — an extra captured local, a slightly fatter error type

Resolution: add a floor-window cell to gossip_pipelining (the same
`diverged_pair` built with `sync_window_floor()`) asserting `measured >=
SERIALIZED_FLOOR` with the floor derived from the disputed-scope count, so
the budget's discrimination is measured rather than stated; in future_size,
pin the measured sizes as constants and assert the budget is within a
stated factor of them. Acceptance: the pipelining file carries two cells,
one passing under the production window and one asserting the floor window
exceeds `HOP_BUDGET`.

Cross-reference: tests-disruption-handshake-10 carries the two-instrument equality and depends on the fixture sharing of tests-disruption-handshake-31.

### tests-disruption-handshake-16: Driver cancellation is sampled at one poll count; the family over the drop point is well-formed and absent
- Where: tests/gossip_when.rs:490-494 (related: tests/gossip_when.rs:468-534; tests/lifecycle.rs:40, 60-92)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read; lifecycle.rs uses a fixed MID_FLIGHT_POLLS = 4; grep for drop-point proptests in tests/ and src/**/tests.rs finds none)
- Seen by: blind-spots [31]; refutation: confirmed; history: no rationale found
- Owner-gated: no

The drop lands after exactly two `now_or_never` polls per side, early in the descent; the one-shot sibling in lifecycle.rs is likewise one point. The drop points that separate the documented post-commit exception (local commit done, epilogue not yet exchanged) from the general case are never reached. Every invariant the test asserts (each side keeps its own send, nothing beyond the union, both links poisoned once `begin` ran, a fresh connection converges) holds at every poll count, so the claim is a family and doctrine asks for a property test.

Evidence:

       490	        // Freeze the session mid-flight: a couple of single polls per side
       491	        // get the preambles (and the first protocol frames) onto the wire,
       492	        // well short of completion.
       493	        a_tx.unbounded_send(()).expect("driver alive");
       494	        for _ in 0..2 {

Resolution: Add a proptest `drop_after in 0usize..N` (N from a metered clean run's poll count) that polls both drivers `drop_after` times, drops them, and asserts the union and atomicity invariants plus poison-then-recover; record whether `a.snapshot().len() == 2` at the drop and, when it is, assert the drop landed after the last data frame. Acceptance: a proptest over the drop point exists in tests/gossip_when.rs (or tests/lifecycle.rs) and passes; any shrunk failure persists to proptest-regressions/.
Construction: Reuse the body of dropping_a_driver_mid_session_commits_nothing with the loop bound drawn from the strategy.

### tests-disruption-handshake-17: The severed-connection cut range `0..400` is unpinned, and with write-only cuts the certification clause's asymmetric case is unreachable
- Where: tests/gossip_when.rs:703-706 (related: tests/gossip_when.rs:695-701, 713-720, 757-770; src/peer/gossip.rs:1285-1319; tests/lifecycle.rs:155-175; tests/disruption.rs:422-448)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read; the reachability argument is reasoned from the epilogue's `try_join` and the test's own "failing side's drop surfaces as EOF" comment, not constructed)
- Seen by: blind-spots [25], api-economics [50]; refutation: reframed (folded in a new observation: the epilogue's try_join makes one-Ok-one-Err unreachable from write-only cuts); history: no rationale found (literal original to e43edcd7 under a different preamble; never re-derived)
- Owner-gated: no

The proptest draws both write cuts uniformly in `0..400` and nothing measures the fixture session's extent, so no check notices the family drifting to mostly-clean or mostly-past-the-end as the wire changes; disruption.rs shows the idiom for the same kind of constant (a metered envelope pinned two-sided). Separately, the doc promises the epilogue's certification "under arbitrary cut geometry", but with write-only cuts the non-trivial case (Ok on one driver, Err on the other) cannot arise: a cut on either side's marker write fails that side, its link drops, and the other side's marker read sees EOF, so both fail; the branch at 763-770 therefore runs only when neither cut landed inside the session. The asymmetric case has a deterministic witness in tests/lifecycle.rs (a read cut one byte short of the marker), so the property is protected, but this proptest's claim is stronger than what it can exercise.

Evidence:

       699	    /// already converged before any recovery (the epilogue's certification,
       700	    /// held under arbitrary cut geometry rather than only at pinned byte
       701	    /// boundaries), and a fresh clean connection converges the pair fully.

       703	    fn severed_connections_fail_loudly_and_recover(
       704	        a_write_cut in 0usize..400,
       705	        b_write_cut in 0usize..400,
       706	    ) {

Resolution: Name the range as a constant and pin it against a metered clean run of the same `pair()` fixture from both sides (as max_cut_spans_the_envelope_session does), or draw the cut as a fraction of the metered extent; add read cuts to the family (`read_cut: Some(..)` on either side) so the certification branch's asymmetric case is reachable, and let the doc claim only what the family can produce. Acceptance: a named constant with a committed two-sided pin replaces the literal 400; a generated or deterministic case yields Ok on one driver and Err on the other and passes the certification assertions.
Construction: Run `pair()` with both sends through `fault::metered` on a clean link and print each endpoint's written bytes; then add `b_read_cut in 0usize..RANGE` and observe the asymmetric branch firing.

### tests-disruption-handshake-18: The chaos interleaving proptest has no redaction arm
- Where: tests/gossip_when.rs:788-797 (related: tests/gossip_when.rs:809-888, 373-436)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read the Op enum, op_strategy, and the feeder loop 843-859: no redact call exists in the proptest)
- Seen by: blind-spots [30]; refutation: confirmed; history: no rationale found
- Owner-gated: no

`Op` is sends, ticks, and pumps, so the property "under any interleaving of commits, ticks, and scheduler progress ... no session errors and full convergence" is established for inserts alone, while the doc's "commits" suggests redactions are included. Redaction through the driver (a frontier-only advance, suppression keyed on `latest()`, a ceiling advance racing an in-flight session) is pinned only by the deterministic chain test at 373-436, one point in the space this generator should sweep.

Evidence:

       788	#[derive(Debug, Clone, Copy)]
       789	enum Op {
       790	    SendA,
       791	    SendB,
       792	    TickA,
       793	    TickB,

       810	    /// Chaos: under *any* interleaving of commits, ticks, and scheduler
       811	    /// progress on both sides of one connection, no session errors and
       812	    /// full convergence.

Resolution: Add `RedactA` and `RedactB` arms that redact the peer's own latest live message when one exists and count executed redactions; assert at the end `len == sends - executed_redactions` beside the existing hash and latest equalities. Acceptance: `op_strategy()` produces redaction ops; the final assertion accounts for them; the committed seed in proptest-regressions/gossip_when.txt still replays.
Construction: Extend the feeder's match with the two arms and re-run the proptest.

### tests-disruption-handshake-26: No default-window peer runs a descent over the one-byte link
- Where: tests/handshake_liveness.rs:143-148 (related: tests/handshake_liveness.rs:7-12, 154-165, 255-258, 335-338; tests/common/wire.rs:209-218; src/tree/mirror/streaming/window.rs:4-12; tests/disruption.rs:706, 916, 936)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (grep for `memory_with_capacity(1\b|MIN_CAPACITY` across tests and src: only this file, window_corners.rs:158 with budget 0, which `Window::from_budget` floors to capacity one everywhere, and the link conformance test; every descent cell here uses `seasoned()` or `bootstrap_fork_async`, both floor-pinned)
- Seen by: blind-spots [22]; refutation: reframed (the design docs argue liveness at any width: window.rs:11-12 "capacity only relaxes the wait graph, so every schedule live at the floor stays live at any width", so this is a coverage gap on a documented argument, not an uncertified configuration; severity lowered to low); history: partial rationale (db2718d4 pinned the floor blanket-wide; 919132dc swept the window dimension through the intra-process engines only, with no note deferring this matrix or the TCP leg)
- Owner-gated: no

Every descent cell in the one-byte matrix is floor-versus-floor: `seasoned()` pins `sync_window_floor()` and `bootstrap_fork_async` pins `WindowChoice::Floor`. The only default-window peers over MIN_CAPACITY are bootstrap newcomers with empty trees (255-258, 335-338), whose descent is one-sided supply. The documented claim that wider windows inherit the floor's liveness is therefore exercised only over 8 KiB links (the sim's window sweep), never against the transport that hides nothing, and the inter-process TCP engine likewise pins the floor at 706, 916, 936 while its intra-process sibling sweeps WindowAssignment. Making the argument's generalization observable is cheap: the harness already has `WindowChoice::apply`.

Evidence:

       143	/// A seasoned replica: a fresh universe with a wide version.
       144	async fn seasoned() -> Rumors<u64> {
       145	    let seed: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
       146	    season(&seed, 0).await;
       147	    seed
       148	}

Resolution: Parameterize `seasoned()` and `seasoned_pair()` over WindowChoice and instantiate the descent cells (divergent, bulk_initiator, retire, empty_meets_populated) at least at Default-vs-Default and one asymmetric Floor-vs-Default; in disruption.rs let the inter-process parent peers and the child take a window choice from the plan. Acceptance: new cells such as `divergent_default_window` and `divergent_asymmetric_window` exist under `block_on` and pass; `ProcPlan` carries a window field drawn from `arb_window_choice`.
Construction: Copy divergent_session with `WindowChoice::Default.apply(...)` on both seeds and run under `block_on`; a wedge surfaces as a deterministic Stalled failure.

### tests-disruption-handshake-8: The inter-process content check gates on EXIT_CLEAN children only and credits the wrong mechanism
- Where: tests/disruption.rs:849-862 (related: tests/disruption.rs:808-815, 963-971, 984-1005)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read child_main 940-1008: the clean final gossip with `.expect("clean final gossip")` precedes the retire loop; EXIT_BOOT_LOSS and EXIT_UNCERTAIN are returned only after it)
- Seen by: blind-spots [26]; refutation: confirmed; history: no rationale found (gate, final gossip, and comment born together at 9eadfc68)
- Owner-gated: no

Every child runs a clean final gossip before its retirement begins, and a failure there panics (an abnormal exit the parent rejects), so every child that returned any recognized exit code has its sends in a parent cast. The check admits only EXIT_CLEAN children, excluding the boot-loss and uncertain-retire exits although the guarantee holds for them, and those faulted-retire arms are the interesting ones. The comment attributes the guarantee to the retirement's reconciliation; the mechanism that establishes it is the final gossip, as the child's own comment at 963-964 says.

Evidence:

       849	    // Every cleanly-retired child's sends must have survived into the
       850	    // parent's converged content: its final retirement reconciled before
       851	    // the party hand-off, so nothing it published may be lost.
       852	    let live: BTreeSet<u64> = readouts[0].values().copied().collect();
       853	    for (index, child) in plan.children.iter().enumerate() {
       854	        if clean_children[index] {

       963	    // One clean session so everything this child published is home even
       964	    // before the retirement reconciles.
       ...
       970	        cast.gossip(&mut link).await.expect("clean final gossip");

Resolution: Drop the `clean_children` gate (every child that reached a recognized exit code passed the final gossip) and restate the comment: the clean final gossip commits the child's sends before retirement begins, so every child's sends must be live. Acceptance: the loop asserts `live.contains(&child_value(index, s))` for every child regardless of exit code; the comment names the final gossip; the test still passes.

### tests-disruption-handshake-21: protocol_constants_match_spec compares a test constant to its own definition
- Where: tests/handshake.rs:57-61 (related: tests/handshake.rs:29-32, 54-56)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (git show 368da2a5 -- tests/handshake.rs removed the `PROTOCOL_MAGIC` and V1 half; lines 30-32 define V2_OPENING with the same three leading bytes)
- Seen by: structure-prose [6], api-economics [39]; refutation: confirmed (line 59 is a real pin of the public discriminant and stays); history: deliberate but expired (at birth the test compared crate constants to literals; 368da2a5 deleted the live half and left the tautology)
- Owner-gated: no

The second assertion checks `V2_OPENING[..3]` against the bytes that V2_OPENING's own definition spells two lines above; it can only fail if the file disagrees with itself. The doc says the opening "starts every preamble", but no preamble is observed. A check whose only input is itself catches nothing.

Evidence:

        54	/// The fixed markers match the hand-encoded layout: the self-described
        55	/// CBOR opening starts every preamble, and the wire version is the
        56	/// dialect's discriminant.
        57	#[test]
        58	fn protocol_constants_match_spec() {
        59	    assert_eq!(Protocol::V2 as u16, 2);
        60	    assert_eq!(&V2_OPENING[..3], &[0xd9, 0xd9, 0xf7]);
        61	}

Resolution: Delete line 60; keep the discriminant pin, or fold `Protocol::V2 as u8` into the outgoing-preamble check proposed in finding 22 (byte 11 of the bytes alice writes), which pins the discriminant against the wire rather than against the enum. Acceptance: no assertion in tests/handshake.rs compares a local constant to a literal restating it.

### tests-disruption-handshake-23: handshake_precedes_protocol_traffic re-runs the magic-mismatch case under a doc it does not check
- Where: tests/handshake.rs:247-262 (related: tests/handshake.rs:81-110; src/tree/mirror/handshake.rs:111-118)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read both bodies; both match `Error::MagicMismatch` from `Preamble::decode`'s prefix check)
- Seen by: structure-prose [7]; refutation: confirmed; history: deliberate but expired (at 70e28462 the preamble rode a length-prefixed frame and the reply declared a 64-byte payload, so the doc's length clause described the body; 4dd2053c made the preamble a self-described 30-byte item and the clause lost its referent)
- Owner-gated: no

The body writes 30 'X' bytes and expects MagicMismatch: mechanically the same as magic_mismatch_surfaces_error with different bytes. The doc claims rejection happens "before any peer-declared protocol frame length can be read or trusted", which nothing in the body observes; only the variant is matched. Two tests for one behavior with different docs is drift waiting to happen, and a testdoc states what the body checks.

Evidence:

       247	/// The preamble must be the connection's first bytes: a peer that skips it and
       248	/// goes straight to protocol traffic is rejected as a magic mismatch before
       249	/// any peer-declared protocol frame length can be read or trusted.
       ...
       262	        let reply = [b'X'; PREAMBLE_LEN];

Resolution: Fold into magic_mismatch_surfaces_error as a second reply pattern (a small table of openings), or rewrite the doc to claim only what is asserted: any 30 bytes without the rumors opening are diagnosed as MagicMismatch quoting the first six. Acceptance: each handshake.rs test's doc names a distinct observed behavior.

### Nits

The full record of each (provenance, history, evidence) is in the evidence file named in the last column.

| Id | Where | Claim | Resolution | Record |
|---|---|---|---|---|
| tests-disruption-handshake-25 | `tests/handshake_liveness.rs:132-134` | the fixture self-check comment states a purpose the `GREETING_FLOOR` doc contradicts | reword to the const doc's claim | `evidence/partitions/tests-disruption-handshake.md` |

## Integration tests: allocation meters, message size, latency, opening supply, routed and TCP links, window suites

`window_census`'s headline admittance is vacuous at its 64 KiB budget (verified by running); the one nonzero binding run target is never checked for convergence; `tradeoff_probe` is a hand-run instrument the public rustdoc quotes; the paused-clock conformance runs lack the timeout the suite's docs require; and two convergence checks compare `len()` alone.

### tests-resource-link-window-20: window_census's admittance test is vacuous: TIGHT_BUDGET resolves to the all-ones floor, so both differenced arms run the identical window
- Where: tests/window_census.rs:27-28 (related: tests/window_census.rs:87-102, tests/window_census.rs:116-136, tests/window_corners.rs:174, src/tree/mirror/streaming/window.rs:397-399, src/tree/mirror/streaming/window.rs:436-456, src/tree/mirror/streaming/remote/adapter/tests/malformed.rs:647-656, src/tree/mirror/streaming/stats.rs:151)
- Class / severity / confidence: verification-gap / high / high
- Provenance: verified (the refutation pass ran `cargo nextest run -p rumors --all-features -E 'binary(window_census)' --no-capture`; I read its log at scratchpad/refute-tests-resource-link-window/window_census.log, lines 35-37: `budget 0, divergence 20000: peak 182214, generations 48002+99576, overhead 34636`, `budget 65536, divergence 20000: peak 182214, generations 48002+99576, overhead 34636`, `admittance 25154 (capacities sum 33)`; I read `from_budget` for the mechanism)
- Seen by: blind-spots, api-economics (as a missing liveness floor); refutation: reframed and raised to high on the run; history: no-rationale-found
- Owner-gated: no

A capacities sum of 33 over 33 heights means every height resolves to capacity 1 at `TIGHT_BUDGET`: `from_budget`'s `charge(k)` starts at the flat decode-fan term `supply_fans` (window.rs:397-399, 437), so when that term alone exceeds the budget, `charge(mid) <= budget` never holds, `lo` stays 1, and every non-root height is clamped to 1. The 64 KiB budget is below that pre-charge (e48bf5ab's decomposition puts the decode fans at 0.21 MB), and the pre-charge is independent of set size. The two arms of the differencing therefore run the identical configuration, the readings are identical, and `windowed <= floor + admitted` is `X <= X + 25154`. The constant's doc, "A budget that binds at test scale: a few scopes per level", is false at this session size. No floor in the suite would have caught this: the census counter's liveness is pinned at unit level (malformed.rs:647-656 asserts `residency >= SMALL`), but nothing here shows the differenced quantity alive. Meters need liveness floors (Principle 2), and the cheapest artifact that passes must be the intended one (Principle 6). The same 64 KiB budget at window_corners.rs:174 (`growth_during_a_session_only_serializes`) resolves to the floor by the same mechanism (assessed, not run), so that test's "The window derives from the sizes exchanged at the greeting" describes no size-dependent derivation in the shape exercised.

Evidence:

    27	/// A budget that binds at test scale: a few scopes per level.
    28	const TIGHT_BUDGET: usize = 64 * 1024;

    124	    let floor = overhead(0, DIVERGENT_WIDE);
    125	    let windowed = overhead(TIGHT_BUDGET, DIVERGENT_WIDE);
    126	    eprintln!(
    127	        "admittance {admitted} (capacities sum {})",
    128	        capacities.iter().sum::<usize>(),
    129	    );
    130	    assert!(
    131	        windowed <= floor + admitted,

    window.rs:
    436	        let charge = |k: u128| -> u128 {
    437	            let mut total = supply_fans;

Resolution: Measure first, then: pick a `TIGHT_BUDGET` whose derived capacities exceed one at 21,024 messages a side (window_knee's 2 MiB lands 4..=256 at ~2,300 messages; the population is larger here, so the budget likely needs to be larger); assert the fixture's own liveness, `capacities.iter().sum::<usize>() > capacities.len()`, beside the admittance computation; have `reconcile` return the two `Gossiped` values and assert the windowed arm's `stats.window_granted > 1` so the real session, not only the test-internals solve, is shown to widen; add `assert!(windowed > floor, ...)` with the measured value in the message; replace `saturating_sub` at lines 96 and 280 with `checked_sub(...).expect("peak covers both resting generations")` so a negative reading fails instead of reading 0; rewrite line 27's doc to state the property the budget must have. Re-measure window_corners.rs:174 alongside and either raise its budget past the pre-charge or restate that test's doc to what it exercises. Acceptance: the census log shows different capacities sums and different peaks for the two arms; the new floors are committed with measured values; a run at `TIGHT_BUDGET = 64 * 1024` fails the fixture's liveness assertion.

### tests-resource-link-window-16: the one nonzero binding run target is never checked for convergence
- Where: tests/target_message_size.rs:224-267 (related: tests/target_message_size.rs:61-74, tests/target_message_size.rs:1-8, tests/common/gossip_snapshot.rs:476-481, tests/common/gossip_snapshot.rs:492-516)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`capture_gossip` returns only the rendered string and the drivers `expect` Ok; the `frames` closure never inspects the handles; convergence is asserted only via `diverged_pair(0, 0)` and `diverged_pair(0, DEFAULT_TARGET_MESSAGE_SIZE)`, both at exchanged minimum 0; `grep -rn 'target_message_size('` outside the file finds only target 0 and builder unit tests)
- Seen by: blind-spots; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

The module doc claims "any minimum (including the degenerate zero) leaves reconciliation convergent"; the body asserts convergence at exchanged minimum 0 only. `nonzero_minimum_binds_both_encoders` runs four cells at `SMALL_TARGET`, the one target in the suite that splits runs, and compares frame counts without looking at the resulting sets. A run-splitting bug that dropped or duplicated a record at a small nonzero target would complete `Ok`, keep every tuple equality, and pass. `capture_gossip_returning` exists for exactly this.

Evidence:

    225	    let frames = |left, right| {
    226	        let (l, r) = seeded_diverged_pair(
    227	            left,
    228	            right,
    229	            (BINDING_MESSAGES_PER_SIDE, BINDING_MESSAGES_PER_SIDE),
    230	        );
    231	        directional_supply_frames(&capture_gossip(l, r))
    232	    };

Resolution: Use `capture_gossip_returning` in the `frames` closure and assert `l.snapshot() == r.snapshot()` per cell, so each of the four cells also pins convergence. Acceptance: each cell asserts snapshot equality, and the assertion is reachable (an encoder that skips the last record of a split run fails it).
Construction: In a scratch build, make the supply encoder drop the final record of any run it splits under a nonzero target; today every assertion in this test still passes (the frame counts stay equal across the mixed and uniform-small cells); with snapshot equality per cell it fails.

### tests-resource-link-window-18: tradeoff_probe's ignore gate is a recorded ruling whose rationale lives only in history, and nothing schedules the run that public rustdoc quotes
- Where: tests/tradeoff_probe.rs:186-188 (related: tests/tradeoff_probe.rs:1-10, src/peer.rs:414-417, justfile:772-775, .github/workflows/ci.yml:134, .github/workflows/ci.yml:180-181, .agent-notes/2026-07-22-sync-budget/sync-budget.md:267-276)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (`grep -n 'tradeoff\|run-ignored\|--ignored' justfile` matches only the `window-tradeoff` table recipe; ci.yml has no `--run-ignored`; Cargo.toml has no `[[test]]` entry; src/peer.rs:414-417 quotes "ran 1.35–1.96× the form's figure (`tests/tradeoff_probe.rs`)"; `git show e48bf5ab` records "Kept ignore-gated: a full cell run is ~11 s in release and several times that in the debug gate, so promotion into the regular suite is declined")
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed (strengthened by the peer.rs citation); history: deliberate-and-holds (the ignore gate is Finch's ruling; a recipe is not precluded by it)
- Owner-gated: yes: whether to add a gate-adjacent recipe or retire the instrument is a verification-policy decision

The ruling that keeps this instrument out of the regular suite (cost) still holds, but the module doc says only "runs only by explicit request" without saying why, so the next reader re-derives or reopens it. Separately, nothing in the justfile or CI invokes the run; it compiles under `test-all` but its assertion of record never executes, so it can sit red indefinitely while `Peer::sync_memory_budget`'s public rustdoc quotes its measurement. Its distinct claim, the wave form at the design corpus (62,500 a side) and the design record (m = 172), is covered by nothing else (`window_operator` runs 8,192 u64 records). An instrument no recipe invokes is decoration (Principle 2), and the justfile is the source of truth for verification (AGENTS.md).

Evidence:

    186	#[test]
    187	#[ignore = "one-shot validation instrument: run explicitly with --run-ignored"]
    188	fn tradeoff_closed_form_validation_run() {

    src/peer.rs:
    414	    /// Measured: sessions whose serialized one-way trips are counted
    415	    /// exactly on a virtual clock, at 8–26 MB budgets on the minimal
    416	    /// and design corpora, ran 1.35–1.96× the form's figure
    417	    /// (`tests/tradeoff_probe.rs`).

Resolution: In either outcome, state the cost rationale in the module doc at the `#[ignore]`. Then, owner's call: (a) add a `just tradeoff-probe` recipe (`cargo nextest run --release --test tradeoff_probe --run-ignored all`) to the CI `instruments` job beside `worst-cases-pin`, with the module doc pointing at the recipe instead of spelling the cargo command; or (b) retire the file, moving the design-corpus and design-record cell into an enforced suite if that claim is wanted, and re-denominating or excising the peer.rs:414-417 citation. Acceptance: the module doc names the cost; and either `grep -n tradeoff_probe justfile .github/workflows/*.yml` finds the recipe and step, or the file is gone and peer.rs cites no unrun instrument.

Cross-reference: verification-infra-14.

### verification-infra-14: tradeoff_probe is a hand-run instrument with no recipe, against the justfile's totality claim
- Where: tests/tradeoff_probe.rs:186-188 (related: tests/tradeoff_probe.rs:1-10, justfile:1-3, justfile:1002-1003)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read the module doc and attribute; `grep -n 'tradeoff_probe\|run-ignored\|ignored' justfile` is empty; the only other `#[ignore]` in the tree is disruption.rs's child-process entry point, which is not an instrument)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The instrument is `#[ignore]`-gated and its invocation lives only in its
module doc; no recipe names it although the justfile opens by claiming
every artifact has one. Its counts are deterministic (virtual time), so it
qualifies for `all` cadence, and its predictions (the solve-derived wave
form the committed trade-off table tabulates) can drift from the measured
sessions with nothing noticing.

Evidence:

    tests/tradeoff_probe.rs
         1	//! One-shot validation instrument, ignore-gated: the solve-derived
         2	//! trade-off predictions held against measured wire-time slowdowns.
         9	//!     cargo nextest run --release --test tradeoff_probe \
        10	//!         --run-ignored all --no-capture
       186	#[test]
       187	#[ignore = "one-shot validation instrument: run explicitly with --run-ignored"]
       188	fn tradeoff_closed_form_validation_run() {

    justfile
         1	# rumors workspace: the source of truth for verification. Every artifact in
         2	# the workspace has a recipe here, tiered by feedback speed, and `just --list`

Resolution: add a `tradeoff-probe` recipe wrapping the documented command
and include it in `all`, or amend the justfile's totality claim to name the
hand-run exceptions. Acceptance: `grep tradeoff justfile` names the recipe
and the module doc points at it instead of a bare command.

Cross-reference: tests-resource-link-window-18 (test-quality, medium) carries the recorded cost ruling (e48bf5ab) and the `Peer::sync_memory_budget` citation.

### tests-resource-link-window-28: growth_during_a_session_only_serializes never witnesses that growth landed mid-session, and its budget resolves to the floor
- Where: tests/window_corners.rs:173-200 (related: tests/window_corners.rs:166-171, tests/window_corners.rs:174, src/peer/gossip.rs:167-171, src/peer/gossip.rs:810-814)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read; the refutation pass traced the poll structure: the racer needs 64 outer polls while a 4,000-dispute session takes far more, so most batches land after the greeting in practice; nothing asserts it; the floor-budget point follows from finding 20's mechanism at the same 64 KiB and is not run)
- Seen by: blind-spots, api-economics; refutation: confirmed at low (unwitnessed, not vacuous); history: no-rationale-found
- Owner-gated: no

The racer commits 64 batches interleaved by `yield_now`, and the test asserts both sessions complete and the follow-up converges. Nothing shows any commit landed after the session's greeting snapshot; if the interleaving front-loaded every commit, the first session already converged everything and the doc's "the next session converges whatever it missed" is exercised on nothing. The witness is cheap and deterministic under this poller. Separately, `pair(64 * 1024, ...)` sits below the derivation's flat pre-charge (finding 20), so the doc's "The window derives from the sizes exchanged at the greeting; commits racing the session make those sizes stale" describes a derivation the exercised shape never performs: the window is the floor regardless of sizes.

Evidence:

   174	    let (left, right) = pair(64 * 1024, 2_048, 2_000, 2_000);

   179	        let race = async {
   180	            for _ in 0..64 {
   181	                racer.send_all((0..32).map(|_| rng.next_u64())).unwrap();
   182	                tokio::task::yield_now().await;
   183	            }
   184	        };
   185	        let (left_result, right_result, ()) =
   186	            tokio::join!(left.gossip(&mut a), right.gossip(&mut b), race);

Resolution: Between the two sessions assert `left.snapshot().len() > right.snapshot().len()` with a message naming it as the mid-session-growth witness (or `left.snapshot().latest() != left_result.converged`, since `converged` is the merged frontier before concurrent commits are joined, gossip.rs:810-814); finish with snapshot equality per finding 27; raise the budget past the pre-charge so the size-dependent derivation the doc describes actually runs, or restate the doc. Acceptance: the witness is committed and passes; awaiting `race` before the `join!` makes it fail.
Construction: Move `race` ahead of the session (await it to completion before the `tokio::join!` of the two gossips). Every current assertion still passes, demonstrating the vacuity of the present body; with the between-sessions inequality added, the moved version fails.

### tests-resource-link-window-10: paused-clock conformance runs lack the timeout the suite's own docs require
- Where: tests/latency_link.rs:28-43 (related: src/conformance/link.rs:24-26, tests/tcp_link.rs:54-61, tests/routed_link.rs:94-101, .config/nextest.toml:26)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (src/conformance/link.rs:24-26 reads "**Run under a timeout**: the contract's liveness clauses fail as hangs"; `grep -n 'Duration::\|tokio::time' src/conformance/link.rs` is empty, so the suite arms no timer of its own; tcp_link and routed_link wrap their calls, these two do not)
- Seen by: blind-spots; refutation: confirmed; history: no-rationale-found (a3da46c42c wrote the requirement and re-pointed these tests without adding one)
- Owner-gated: no

A wedged clause here is reported only by nextest's 180 s terminate. Under `start_paused`, `tokio::time::timeout` is the ideal detector: with no runnable task and no shorter wire timer pending, auto-advance jumps to the deadline and the clause fails in zero wall time with the harness's own message.

Evidence:

    28	#[tokio::test(start_paused = true)]
    29	async fn conforms_at_zero_delay() {
    30	    rumors::conformance::link::check(async || latency::delayed_pair(CAPACITY, Duration::ZERO))
    31	        .await;
    32	}

Resolution: Wrap both `check` calls in `tokio::time::timeout(SUITE_TIMEOUT, ...).await.expect("conformance suite ran past its liveness bound")` as the socket runners do, with a `SUITE_TIMEOUT` constant. Acceptance: both tests carry the timeout; a deliberately wedged pipe fails with that message rather than after 180 s.
Construction: In a scratch copy of `latency::delayed_pair`, make one stream's `poll_read` return `Pending` without arranging a wake; today the test hangs until nextest kills it, with the timeout it fails immediately.

### tests-resource-link-window-7: decode_alloc testdoc claims an admitted-overhang complement no meter in the file exercises
- Where: tests/decode_alloc.rs:263-271 (related: src/testing.rs:152-156, src/tree/mirror/streaming/remote/codec/budget.rs:102, src/tree/mirror/streaming/remote/codec/budget.rs:128-132, src/tree/mirror/streaming/remote/codec/decode/tests.rs:965-989)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (src/testing.rs:155 `decode_supply_frame_budgeted(read, usize::MAX)`; budget.rs:130 saturates at `MAX_RUN_BUDGET_BYTES`, `u32::MAX as usize - SUPPLY_FRAME_OVERHEAD`, so an 8 MiB lone record is within budget; the overhang admission is pinned only by `oversized_lone_record_still_decodes`, unmetered)
- Seen by: blind-spots; refutation: confirmed; history: no-rationale-found (the sentence was inaccurate when written, ed0f1775e)
- Owner-gated: no

The doc says the lone-record tests above pin "that the admitted overhang still reaches the body read", but those tests run through `decode_supply_frame`, which decodes at the saturated ceiling budget, so their record is never an overhang. The ingress gate decides one boundary (one record over budget admitted, two rejected), and the meter prices only the rejection side. An inaccurate testdoc is a bug in the test.

Evidence:

    269	/// identically but request the body's bytes, and reds here. The honest
    270	/// lone-record tests above are the complement, pinning that the admitted
    271	/// overhang still reaches the body read.

Resolution: Run `supply_full_delivery_costs_at_most_payload_plus_chunk` (or a twin) through `decode_supply_frame_budgeted(&bytes[..], 0)`, making the lone record a true overhang at no extra cost, and reword lines 269-271 to name that test as the complement. Acceptance: a metered test decodes a lone record under a budget smaller than the record, asserts `Ok`, and holds the same `[N, N + chunk]` band.

### tests-resource-link-window-8: a public-vocabulary matchability pin with an unfailable assert lives in the allocator meter
- Where: tests/decode_alloc.rs:317-334 (related: tests/decode_alloc.rs:1-12, tests/decode_alloc.rs:17)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (`git log -L 317,334:tests/decode_alloc.rs` shows the block introduced by 39290c4a "export HeadError"; the body constructs and matches the same variant with no `metered` call)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: no-rationale-found (placement unargued; the file already imported `rumors::error`)
- Owner-gated: no

`leaf_run_head_defect_is_publicly_matchable` meters nothing and its `matches!` cannot fail at runtime; its value is that the file compiles against `rumors::error::HeadError`. That is a public-API surface pin, which the module doc ("Allocator metering for the wire decoders'...") does not admit. A reader auditing the error taxonomy will not look here, and modules have one responsibility.

Evidence:

    317	/// The head-grammar defect carried by [`LeafRunError::Head`] is public
    318	/// vocabulary: a caller outside the crate can write the type
    319	/// `rumors::error::HeadError` and match the variant a non-shortest-form
    320	/// head classifies as.
    321	#[test]
    322	fn leaf_run_head_defect_is_publicly_matchable() {

Resolution: Move the test to the binary that pins public error vocabulary (or a small `tests/error_surface.rs`), state in its doc that compiling is the check (a `let _: HeadError = ...;` suffices), and drop the now-unused `HeadError`/`LeafRunError` imports here. Acceptance: decode_alloc.rs contains only metered tests; the pin lives beside its peers.

### tests-resource-link-window-19: window_census rests correctness on the runner where its sibling meter closes the same hazard with a lock; the peak differencing is spelled twice
- Where: tests/window_census.rs:15-16 (related: tests/window_census.rs:87-102, tests/window_census.rs:273-286, src/testing.rs:51-52, src/tree/typed/untyped.rs:53-56, tests/decode_alloc.rs:23-25, src/conformance/backend/tests.rs:37)
- Class / severity / confidence: test-quality / low / medium
- Provenance: verified (the counters are `static AtomicUsize` at untyped.rs:54-56; testing.rs:51-52 states the premise; `grep -c 'cargo test' justfile` is 0 and `grep -c 'cargo nextest' justfile` is 8; decode_alloc.rs:23-25 and conformance/backend/tests.rs:37 serialize the same hazard with a static mutex)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: deliberate-and-holds (premise stated at both the suite and the accessor; the gate is nextest-only; R12's fix d8afb610 chose a lock for the sibling genre)
- Owner-gated: no

The premise is stated and the gate enforces it, so this is a robustness improvement rather than a defect: under `cargo test` (threads in one process) the two census tests would reset and read each other's peaks, and the two version-bound tests allocate nodes concurrently. decode_alloc.rs faces the identical process-global-counter problem and closes it runner-independently with `METER_LOCK` ("the suite is correct under any test runner's threading"). "Sound" is also used loosely here. Separately, `overhead` (89-102) and `floor_overhead_is_bounded_by_content` (274-281) spell the same before/reset/reconcile/peak/after arithmetic.

Evidence:

    15	//! The census is process-global, which is sound here because nextest runs
    16	//! each test in its own process.

Resolution: Add `static CENSUS_LOCK: Mutex<()>` taken in every test body (the version-bound tests construct nodes too), mirroring decode_alloc's `metered`; reword the module doc to say the lock makes the suite runner-independent; hoist the differencing into `overhead` returning `(overhead, after)` and call it from the floor test. If the owner rules `cargo test` unsupported, keep the premise but drop "sound" for "correct under nextest's process-per-test model". Acceptance: every window_census test serializes on one lock (or the doc names the runner requirement as such); one site of the peak-differencing arithmetic.

### tests-resource-link-window-25: the catch-up hop ceilings have no floor
- Where: tests/window_corners.rs:98-102 (related: tests/window_corners.rs:112-115, tests/window_corners.rs:228-231, tests/latency_link.rs:104-108)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (both tests assert only `measured <= 12`; the real-clock leg at 228-231 asserts `catch_up >= 2 * delay` and latency_link.rs:104-108 asserts `hops >= 2` for a diverged pair, so the irreducible-exchange floor exists in the file on the other clock and in the sibling binary)
- Seen by: blind-spots; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

A degenerate session (one that never opened the descent, or a harness change reporting 0) passes both asymmetric catch-up tests. Pair every ceiling with a positive floor derived from the mechanism's irreducible work (Principle 2): completion depends on a reply to a delivered message, so at least two causally chained hops.

Evidence:

    98	    assert!(
    99	        measured <= 12,
   100	        "a one-common-message catch-up must cost ladder hops, not waves: \
   101	         {measured} hops",
   102	    );

Resolution: Add `assert!(measured >= 2, ...)` at both sites, stating the floor as the one request/response the transfer cannot avoid. Acceptance: both tests carry a floor; a `hops` returning 0 fails them.

### tests-resource-link-window-27: convergence witnessed by len() equality alone at two sites
- Where: tests/window_corners.rs:141-145 (related: tests/window_corners.rs:195-199, src/snapshot.rs:16-20, tests/routed_link.rs:250, tests/routed_link.rs:343-344, tests/tradeoff_probe.rs:119-123, src/conformance/link.rs:1061-1067)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (both sites compare `snapshot().len()`; `Snapshot` derives `PartialEq, Eq` at snapshot.rs:16; routed_link.rs:250 uses `assert_eq!(seed.snapshot(), newcomer.snapshot())` and 343-344 and tradeoff_probe.rs:119-123 compare `hash()`)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: no-rationale-found (R13 ruled the same oracle shape a finding in the conformance suite and it was fixed there; window_corners was not swept)
- Owner-gated: no

Both docs say the session "converges"; the bodies prove equal cardinality, which a session that dropped one message from each side and picked up another would pass. Set equality is as cheap and strictly stronger, and it is the oracle the crate uses elsewhere. The cheapest passing artifact must be the intended one (Principle 6).

Evidence:

    141	    assert_eq!(
    142	        left.snapshot().len(),
    143	        right.snapshot().len(),
    144	        "the serialized session still converges",
    145	    );

Resolution: Compare `left.snapshot()` with `right.snapshot()` (or their `hash()`) at 141-145 and 195-199. Acceptance: no `snapshot().len()` equality stands alone as a convergence witness in the window suites.

### Nits

The full record of each (provenance, history, evidence) is in the evidence file named in the last column.

| Id | Where | Claim | Resolution | Record |
|---|---|---|---|---|
| tests-resource-link-window-12 | `tests/opening_supply.rs:143-156` | two absence assertions have no in-file positive control; a renamed label passes | assert a same-format header line exists, or share the strings as constants | `evidence/partitions/tests-resource-link-window.md` |

## Integration tests: wire snapshots, CBOR evolution, dispute wire, legibility, snapshot liveness, payload depth, send bounds

No snapshot pins a stream index of two or higher and the deep fixture has no depth floor; `api_send_bounds` says "every" and omits five async entry points; the legibility walker has no negative case; the snapshot sweep judges only `tests/snapshots`; and several snapshot testdocs name a converged live set the bodies never assert. `tests/future_size.rs` (tests-wire-format-26) is filed under the infrastructure section.

### tests-wire-format-27: api_send_bounds says "every async public method" but omits gossip_when, Peer::bookmark, BookmarkedBootstrap::join, and two of the three observer streams
- Where: tests/api_send_bounds.rs:1-3 (related: tests/api_send_bounds.rs:19-22, 27-38; src/rumors.rs:437, 489, 617-632; src/peer.rs:272, 290; src/peer/bootstrap.rs:247, 337; src/rumors/causal.rs:151; src/rumors/changes.rs:127; src/rumors/unordered.rs:191; Cargo.toml:72, 128)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`grep -n -E 'pub (async )?fn (try_into_peer|gossip|gossip_when|bookmark|retire|join)\b'` over src/rumors.rs, src/peer.rs, src/peer/bootstrap.rs, plus `grep -rn -E 'impl.*Stream for' src/rumors/*.rs`, enumerate the async surface; `gossip_when` returns `impl Stream<Item = ...> + Unpin + 'a` with `S: Stream + 'a` and no `Send` in its signature; the suite checks `gossip`, `Bootstrap::join`, `retire`, `try_into_peer`, and `UnorderedMessages::next` only; `static_assertions` is a regular dependency)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: deliberate but expired ("every" was exact on 2026-05-27 for `Local::{message,process,gossip}`; `gossip_when` landed 2026-06-12 and `Peer::bookmark` 2026-06-17 with no change to this file; later touches generalized the wording to "handle types" without re-auditing)
- Owner-gated: no

The public async surface is `Rumors::{try_into_peer, gossip, gossip_when}`, `Peer::{bookmark, retire}`, `Bootstrap::join`, `BookmarkedBootstrap::join`, and the `Stream` faces of `UnorderedMessages`, `CausalMessages`, and `Changes`. Five of those are unchecked, so "every" over-claims and a `!Send` regression in any of them (an `Rc` captured in the bookmark path, a non-`Send` cue stream bound) compiles. `gossip_when` is the method most likely to be `tokio::spawn`ed, exactly the motivating use the doc names. A guard whose doc says "every" invites a maintainer to assume a new async method is covered when it is not. The hand-rolled `require_send_sync`/`require_send_type` also duplicate `static_assertions::assert_impl_all!`, already a dependency.

Evidence:

    1	//! Static assertions that every async public method on the `rumors`
    2	//! handle types returns a `Send` future, and that the handle types
    3	//! themselves are `Send + Sync`.

Resolution: Add `require_send` checks for `Peer::bookmark` (with `NoBookmark` or a trivial `Bookmark`), `BookmarkedBootstrap::join`, `gossip_when`'s stream (with `futures::stream::pending()` as the cue), and `CausalMessages::next` / `Changes::next`; replace lines 19-22 and 27-38 with item-level `static_assertions::assert_impl_all!(Peer<String>, Rumors<String>, Snapshot<String>: Send, Sync); assert_impl_all!(UnorderedMessages<String>: Send);`. Or narrow the module doc to the enumerated set. Acceptance: every `pub async fn` and every `impl Stream`-returning method on `Peer`, `Rumors`, `Bootstrap`, `BookmarkedBootstrap`, and every observer's `Stream` impl has a compile-time `Send` check, and the doc's "every" is true.
Construction: Add `fn gossip_when_stream_is_send() { let s = rumors.gossip_when(futures::stream::pending::<Gossip>(), &mut link); require_send(&s); }`; today this is the only way to learn whether it compiles.

### tests-wire-format-14: missing_fields_error_absent_a_default tests ciborium in isolation, against the module's own stated method
- Where: tests/cbor_evolution.rs:216-251 (related: tests/cbor_evolution.rs:5-8, 150-170; src/message.rs:300-310)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (the only `ciborium::` calls in the file are at 237, 240, and 243; the module doc at 7-8 says the property is pinned "through the crate's own encode and decode paths, not against a serializer in isolation"; the crate's ingress decode is `ciborium::de::from_reader_with_recursion_limit` via `decode_exact`, a different entry from `from_reader`)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: no rationale found (the direct calls are present in the file's first commit alongside the doc that forbids them; the only plausible constraint, no session-shaped clean-failure route until 0ac1872c two days later, has lapsed)
- Owner-gated: no

The module doc states the file's method: every rule is pinned through the crate's own paths, never against a serializer alone. This test calls `ciborium::ser::into_writer` and `ciborium::de::from_reader` directly and constructs no `Peer`, so a change to the crate's payload codec (a wrapper deserializer, a positional encoding, a different engine) would leave it green while the contract it claims to pin breaks. Differential and contract suites exercise the public API; here the entry is not even the crate's. The test is also the file's only non-async test, a shape that stands out for the wrong reason.

Evidence:

    236	    let mut narrow = Vec::new();
    237	    ciborium::ser::into_writer(&Narrow { id: 5 }, &mut narrow).unwrap();
    238	
    239	    // Without a default, the absent field is an error, not a guess.
    240	    assert!(ciborium::de::from_reader::<Wide, _>(narrow.as_slice()).is_err());

Resolution: Route the tolerated case through `exchanged::<Narrow, WideDefaulted>` (the filled default must arrive over a session) and the rejected case through the failing-bootstrap shape `undecodable_payload_fails_bootstrap_cleanly` already uses (a `Peer<Wide>` bootstrapping from a `Narrow` donor returns an error, never a panic, and moves nothing). If the owner chooses option (b) of tests-wire-format-9 and wants to keep a serializer-level pin, say so at the test with a comment naming it as the one deliberate exception. Acceptance: the test contains no direct `ciborium::` call and both branches run through `Peer`/`Rumors`, or the exception is stated at the site.

### tests-wire-format-7: No snapshot pins a data stream with index 2 or higher, and deep_trie_divergence has no depth floor
- Where: tests/gossip_snapshot.rs:492-515 (related: tests/gossip_snapshot.rs:4-5, 599-607; tests/snapshots/gossip_snapshot__deep_trie_divergence.snap:289; src/tree/mirror/streaming/remote/codec/signal.rs:17-24,55-73; src/link.rs:161-169)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`grep -h -o -E '(Initiator|Responder) stream [0-9]+ \(height [0-9]+\)' tests/snapshots/*.snap | sort | uniq -c` yields exactly four header kinds: Initiator 0 (h31), Initiator 1 (h30), Responder 0 (h31), Responder 1 (h29); `STREAM_COUNT = 17` at link.rs:169; `deep_trie_divergence` at 500-515 asserts nothing before `insta::assert_snapshot!`)
- Seen by: blind-spots; refutation: confirmed, with a mechanism correction adopted below; history: no rationale found (`DEEP_TRIE_PER_SIDE = 16` was chosen on 2026-06-05 to collide in one leading byte and never revisited; REVIEW.md item 14 catalogued corpus gaps and named only the nonempty Query frame)
- Owner-gated: no

The module doc says the corpus "pins every wire byte", but the deepest fixture reaches `Responder stream 1 (height 29)`; the stream-label encoding and per-height frame forms for the remaining fifteen streams the schedule can open are pinned nowhere in `tests/snapshots`. By the stride at signal.rs:17-24 and 55-73 (stream 0 is height 31 for both speakers; Initiator stream k carries height 32-2k, Responder stream k height 31-2k), a stream index of 2 needs a dispute surviving at a height-29 node, which requires leaves under the same three-byte path prefix on both sides; `deep_trie_divergence`'s one-byte collisions reach exactly index 1. The same fixture carries no liveness floor for its depth claim, unlike `shared_subtree_dispute_pins_a_nonempty_query` (599-607), so a corpus change that flattened it would re-accept with no mechanical objection. White-box worst-case construction asks that the constructed shapes be diffed against the committed roster; the roster stops two levels below the root.

Evidence:

    500	#[test]
    501	fn deep_trie_divergence() {
    502	    let (a, b) = block_on(async {
    503	        let a: Rumors<u64> = seeded();
    504	        let b = bootstrap_fork_async(&a).await;
    505	        {
    506	            a.send_all(0..DEEP_TRIE_PER_SIDE).unwrap();
    507	        }
    508	        {
    509	            b.send_all(DEEP_TRIE_PER_SIDE..2 * DEEP_TRIE_PER_SIDE)
    510	                .unwrap();
    511	        }
    512	        (a, b)
    513	    });
    514	    insta::assert_snapshot!(capture_gossip(a, b));
    515	}

Resolution: Add a depth floor to `deep_trie_divergence` (`stream_frames(&capture, "Responder stream 1 (height 29)")` must be `Some`), so its stated purpose is tamper-evident. For the deeper labels, either stage one fixture with both peers holding leaves under a shared three-byte prefix (a shared pair at `shaped_pair(pool, 3, true)` staged before the fork, plus a divergent third leaf under the same prefix, as `shared_subtree_dispute_pins_a_nonempty_query` does at one byte; a birthday pool on the order of 2^12 sends), floored on a `stream 2` header; or, if the staging cost is judged too high, add a codec-level unit pin of the stream-label encoding for indexes 2..16 and say so in this module doc. Acceptance: `deep_trie_divergence` fails before the snapshot comparison if its depth degrades; either a committed snapshot contains a `stream 2` header or the module doc states where the deeper labels are pinned.
Construction: Stage `send_pool(&a, 0, 4096)`, `shaped_pair(&pool(&a, 0, 4096), 3, true)`, `keep_only`, fork `b`, then land a third leaf under the same three-byte prefix on `a` via a targeted pool search on `leaf_path(v)[..3]`; capture and check for a `stream 2` header.

Cross-reference: remote-proxy-tests-10 is the proxy-tier statement of this gap and names the deep fixtures (`leaf_parent_dispute_pair`, `pyramid_pair`) that could close both; tests-wire-format-18 is the legibility-tier statement.

### tests-wire-format-19: The snapshot sweep judges only tests/snapshots, though insta writes beside the asserting file
- Where: tests/snapshot_liveness.rs:105-109 (related: tests/snapshot_liveness.rs:132-146, 319-351)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read `scan_tests_side`, which reads exactly `root/tests/snapshots`, against `scan_src_side`, which recurses for any `snapshots` directory; `find . -type d -name snapshots` shows no `tests/common/snapshots` today)
- Seen by: api-economics; refutation: confirmed; history: no rationale found (REVIEW.md item 13's resolution said "Walk `tests/snapshots/*.snap`" and the implementation transcribed it; the gap was never considered)
- Owner-gated: no

insta places a snapshot in a `snapshots` directory beside the asserting source file, so an `assert_snapshot!` added to `tests/common/*.rs` (or any nested test module) would land at `tests/common/snapshots/`, outside both scans; an orphan there would never be convicted. The module doc claims every committed snapshot resolves to a live generator, and the src side already applies the recursive discipline; the tests side does not. No such directory exists today, so this is a blind spot rather than a live orphan.

Evidence:

    105	/// Judge every file under `<root>/tests/snapshots`: the stem's prefix
    106	/// before the first `__` names the suite binary, the rest the
    107	/// snapshot.
    108	fn scan_tests_side(root: &Path, out: &mut Vec<Snap>) {
    109	    let dir = root.join("tests").join("snapshots");

Resolution: Recurse under `tests/` the way `scan_src_side` does, convicting any `snapshots` directory other than `tests/snapshots` outright (the module doc's "extending the rule is then a reviewed decision, not a silent skip" already sets the policy), and add a fixture line `tests/common/snapshots/x.snap` to `suite_snapshots_resolve_to_their_binary` asserting the conviction. Acceptance: a fixture tree containing `tests/common/snapshots/anything.snap` is convicted by the sweep.
Construction: In `suite_snapshots_resolve_to_their_binary`, add `.file("tests/common/snapshots/stray.snap", "")` and assert the sweep reports five verdicts with that path convicted; today the sweep reports four and never sees the file.

### tests-wire-format-5: Snapshot tests whose docs promise a converged live set assert nothing about it
- Where: tests/gossip_snapshot.rs:418-421 (related: tests/gossip_snapshot.rs:62-68, 465-466, 611-616, 654-662; tests/common/gossip_snapshot.rs:476-516)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (read `capture_gossip` and `capture_gossip_returning`: the drivers `expect("gossip A")` / `expect("gossip B")` and the harness asserts the control drain; nothing inspects the live sets; only `early_supplies_honor_redactions` at 381-404 takes the handles back and checks them)
- Seen by: blind-spots; refutation: confirmed; history: no rationale found (`capture_gossip_returning` arrived on 2026-08-19 for one test; no note records a byte-pins-only division of labor)
- Owner-gated: no

`fork_insert_redact` ("must converge both peers on the live set `{3, 4}`"), `redaction_only` ("converge on `{2}`"), `both_redact_the_same_message` ("converges idempotently on `{2}`"), `string_payload` ("converge on both"), and `one_sided_transfer` state post-session outcomes that only the human who accepted the snapshot ever judged. A snapshot re-accept is exactly the moment the semantic claim is unguarded: a re-accept under a wrong implementation would pin wrong bytes with no mechanical objection. The doc claims a stronger property than the test checks.

Evidence:

    418	/// Reconciliation must converge both peers on the live set `{3, 4}`: the two
    419	/// redactions are contagious and cross the wire alongside the two novel
    420	/// inserts, so the capture pins inserts, fork divergence, bidirectional
    421	/// transfer, and redaction propagation all at once.

Resolution: Use `capture_gossip_returning` in these scenarios and assert the live sets the docs name (a sorted-payloads `assert_eq!` per side), or reword the docs to claim only the pinned wire form. Acceptance: each doc's stated live set is asserted after the capture, or the doc no longer states one.
Construction: In `fork_insert_redact`, replace `capture_gossip(a, b)` with `capture_gossip_returning`, then assert both snapshots' payloads equal `[3, 4]`; the assertion passes today and would fail under a redaction-propagation regression that the snapshot alone would only catch as a byte diff a reviewer could re-accept.

### tests-wire-format-12: Error-path tests accept any error of the outer shape, not the decode failure their docs name
- Where: tests/cbor_evolution.rs:162-163 (related: tests/cbor_evolution.rs:16-18, 204-205; tests/payload_depth.rs:325-327, 361-365; src/error.rs:52-53; src/tree/mirror/streaming/remote/proxy/error.rs:18-81; src/tree/mirror/streaming/remote/codec/error.rs:104-109; src/message.rs:300-315)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (read `MirrorError = mirror::Error<MaterializedError<Infallible>, RemoteError<Infallible>>` at error.rs:53 and the `RemoteError` enum at proxy/error.rs:18-81, which alone has some twenty variants; read `decode_exact` at message.rs:300-315 producing `PayloadDecodeError::Io`, which the codec surfaces as `DecodeLeafError::Message(io::Error)`)
- Seen by: blind-spots; refutation: confirmed, with the note that the exact leaf must be read off the ingress path rather than guessed; history: no rationale found (a division-of-labor argument is available but unstated: the codec-level tests and the error atlas pin `DecodeLeafError::Message`)
- Owner-gated: no

cbor_evolution's module doc promises "a decode error at the receiver's wire ingress", and payload_depth's doc promises "the typed decode error", but the assertions are `is_err()` (cbor_evolution 163, 205) and `matches!(b_err, rumors::Error::Mirror(_))` (payload_depth 363), which any of roughly twenty unrelated leaves (an accept error, a framing violation, `UnaskedReply`) would also satisfy. The public `rumors::error` module re-exports the whole taxonomy so callers and tests can name the leaf; a regression that failed these sessions for an unrelated reason passes.

Evidence:

    162	    let joined = Peer::<u64>::bootstrap().join(&mut near).await;
    163	    assert!(joined.is_err(), "a String payload must not decode as u64");

    362	    assert!(
    363	        matches!(b_err, rumors::Error::Mirror(_)),
    364	        "the receiver's exit is the typed decode failure: {b_err:?}"
    365	    );

Resolution: Trace the ingress path once and match the leaf that carries the payload `io::Error` (the `DecodeLeafError::Message` route through `RemoteError`, whichever variant it surfaces in) at all three sites; in cbor_evolution's bootstrap case assert the same on the newcomer's error. Acceptance: each of the three assertions names the decode leaf, and the assertion messages match what they check.
Construction: In `a_sender_exits_typed_when_its_counterparty_aborts_on_decode`, print `b_err` with `{:?}` once to learn the exact variant chain, then replace `Error::Mirror(_)` with that pattern; the same pattern applies at cbor_evolution 163 and 205.

### tests-wire-format-18: The legibility walker has no committed negative case, and the corpora never open a stream index of 2 or higher
- Where: tests/wire_legibility.rs:109-116 (related: tests/wire_legibility.rs:37-79, 118-122; src/tree/mirror/streaming/remote/codec/signal.rs:17-24, 55-73)
- Class / severity / confidence: test-quality / low / medium
- Provenance: assessed (read the file: its only tests are the three proptests at 127, 157, 181, and nothing feeds `walk_sequence`/`walk_value` a rejected input; read the stride at signal.rs, from which a stream index of 2 requires a dispute at a height-29 node, i.e. leaves under the same three-byte path prefix on both sides)
- Seen by: blind-spots; refutation: reframed (the mechanism for the rarity was corrected from "two-byte shared prefix" to "three-byte shared prefix on both sides", which makes the region rarer still); history: no rationale found (the legible-wire design note and REVIEW.md specify only the positive property)
- Owner-gated: no

The walker is the whole oracle, and nothing committed shows it rejecting a non-CBOR byte, residue behind a tag-24 item, or a tag 63 wrapping a non-bstr; a walker bug that returned `Ok` on everything would pass all 72 cases vacuously. Every criterion needs a committed demonstration that a known-bad artifact fails it. And with at most eleven payloads per bucket over uniform paths, a shared three-byte prefix present on both sides almost never arises, so the property is checked on streams 0 and 1 only; together with tests-wire-format-7, the deeper stream labels have neither a byte pin nor a property behind them.

Evidence:

    109	fn corpora() -> impl Strategy<Value = (Vec<Vec<u8>>, Vec<Vec<u8>>, Vec<Vec<u8>>)> {
    110	    let payload = vec(any::<u8>(), 0..48);
    111	    (
    112	        vec(payload.clone(), 0..12),
    113	        vec(payload.clone(), 0..12),
    114	        vec(payload, 0..12),
    115	    )
    116	}

Resolution: Add three unit checks on the walker (a `[0xff]` sequence; `24(<< 0x00 0x00 >>)` with residue; `63(0)`) asserting `Err`. Add one deterministic case staged through `common::shape` (a shared pair at three bytes on both sides plus a divergent leaf) so at least one capture per run has a stream index of 2 or more, asserting `streams.len() >= 3` on that capture. Acceptance: negative walker tests committed and red on the bad inputs; one legibility run includes a capture with three or more data streams.
Construction: `assert!(walk_sequence(&[0xff], "bad").is_err())`; build tag 24 around the two-byte string `[0x00, 0x00]` via `ciborium::ser::into_writer(&Value::Tag(24, Box::new(Value::Bytes(vec![0, 0]))), ..)` and assert `Err` mentioning "residue"; build `Value::Tag(63, Box::new(Value::Integer(0.into())))` and assert `Err` mentioning "byte string".

Cross-reference: remote-proxy-tests-10 and tests-wire-format-7.

### tests-wire-format-20: The snapshot_liveness fixture tests demonstrate three of nine conviction branches
- Where: tests/snapshot_liveness.rs:187-213 (related: tests/snapshot_liveness.rs:66-81, 86-102, 119-122, 162-169, 319-351, 357-390)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (counted the `Err(` sites: `contained` two, `snap_stem` two, `scan_tests_side`'s no-`__` branch one, `judge_snapshots_dir`'s directory branch one, `judge_src_stem` three; the two fixture tests produce only "does not exist", "contains neither", and "unaccepted snapshot")
- Seen by: structure-prose; refutation: confirmed, count corrected from four to three of nine; history: no rationale found (all nine branches existed at the sweep's creation; the fixture scope was transcribed from REVIEW.md item 13's acceptance, which named one stray file)
- Owner-gated: no

`judge_src_stem`'s three convictions (missing `rumors__` prefix, too-short module path, directory segments not spelling the location), `snap_stem`'s stray-file conviction, `scan_tests_side`'s no-`__` conviction, and the nested-directory conviction are never exercised; a typo in any of them (a wrong `split_at`, an inverted comparison) would pass. Every criterion needs a committed demonstration that a known-bad artifact fails it. The two fixture tests also differ in style: the first indexes `verdicts[0..3]` by sort position, the second builds a by-path map.

Evidence:

    188	    let Some(rest) = stem.strip_prefix("rumors__") else {
    189	        return Err("the stem does not open with this crate's `rumors__` prefix".to_owned());
    190	    };
    191	    let segments: Vec<&str> = rest.split("__").collect();
    192	    if segments.len() < components.len() + 2 {

Resolution: Extend the module fixture with `other__foo__tests__x.snap`, `rumors__foo__tests.snap`, `rumors__wrong__tests__x.snap`, a `notes.txt`, a nested directory, and a tests-side `nosep.snap`, asserting each conviction's message; use the by-path map in both fixture tests. Acceptance: each `Err(...)` literal in the sweep has a fixture line that produces it.
Construction: Add `.file("src/foo/snapshots/other__foo__tests__x.snap", "")` to `module_snapshots_resolve_through_the_module_path` and assert its verdict contains "rumors__ prefix"; repeat for the other five.

### Nits

The full record of each (provenance, history, evidence) is in the evidence file named in the last column.

| Id | Where | Claim | Resolution | Record |
|---|---|---|---|---|
| tests-wire-format-4 | `tests/gossip_snapshot.rs:139-147` | `asymmetric_message_targets` pins one direction of a symmetric `min` | add the dual fixture or narrow the doc | `evidence/partitions/tests-wire-format.md` |
| tests-wire-format-22 | `tests/payload_depth.rs:132-134` | `PayloadDepthLimit::new(0)` and `new(u64::MAX)` have no committed pin | two point tests | `evidence/partitions/tests-wire-format.md` |

## Benches and examples

A bench restates the shipped preimage by hand with nothing tying the copy to the function; the version-bounds pruning the benches advertise is enforced by no committed instrument; the envelope simulator's certificate is run by nothing; and the swarm example's rendezvous, membership, and shutdown claims are prose-only while its test re-implements the party loop's turn.

### benches-envelope-32: The dominance certificate window.rs cites compares the simulator's own copy of the integer envelopes against its own oracle, never covers the shipped pair-product forms, and is run by nothing
- Where: examples/envelope_sim.rs:690-726 (related: examples/envelope_sim.rs:28-31, examples/envelope_sim.rs:48-53, examples/envelope_sim.rs:538-682, src/tree/mirror/streaming/window.rs:570-578, src/tree/mirror/streaming/window.rs:689-701, src/tree/mirror/streaming/window/tests.rs:376-381, Cargo.toml:194-198, justfile:97-99)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (window.rs:572-575 says the dominance "is verified by `examples/envelope_sim.rs` over a dense sampled sweep"; the example's imports at 48-53 are `std` and `rand` only, with no `use rumors`; `check_integer_dominates` compares `occ_int`/`joint_int`/`q_leaves_int`/`q_slots_int`/`stage_pop_int` (562-661) against the example's own float oracle (206-407); `grep -rn envelope_sim justfile .github tools src tests Cargo.toml` returns only window.rs:573; justfile:97-99 `check` only type-checks examples; `window/tests.rs:376-381` `envelopes_are_consistent` checks internal relations only, and every bound-level call in that file (`jointly_occupied`/`stage_population` at 52, 57, 207, 210, 378, 380) passes `pair = n * n`; I compared the example's `_int` family to window.rs function by function and the formulas agree at `pair = n²` today, while the shipped `jointly_occupied(n, pair, j)` generalizes to `A·B` and takes the saturating `pow256` path where the example's `expect` at 608 sits)
- Seen by: structure, correctness, perfapi, prose (the "nothing runs it" half); refutation: confirmed, severity lowered from high to medium because window.rs:626-666 carries inline sufficiency proofs for `bernstein` and `small_mean_quantile`, making the sweep a secondary empirical check of an argued bound; history: deliberate-and-holds for the independent-replica design (ded24eb3a: "examples/envelope_sim.rs is the certifying tool of record"; 3824c7754 verified the port byte-identical against the Python; the example's header at 28-31 concedes the pair-product gap in its own words) and no-rationale-found for the gating gap (`git log -S envelope_sim -- justfile .github` is empty at every commit)
- Owner-gated: yes (reverses the recorded "tool of record" ruling)

A provenance claim in production code names a check that exercises a sibling implementation, in a place the gate never runs. The shipped `occupied`/`jointly_occupied`/`children_quantile`/`stage_population` are separate code with a divergent signature (the pair product), the pair-product adaptation is certified by nothing at any asymmetric `(A, B)`, and a refactor of `small_mean_quantile` or `bernstein` in window.rs today would leave the cited certificate untouched and green. Principle 6 (the cheapest artifact that satisfies the proxy must be the intended one) and Principle 8 (a provenance claim must name a check that exercises the code making the claim): "a board nothing enforces is decoration". The design value the ruling protected, an independent float oracle, survives the fix; what changes is what the oracle is pointed at.

Evidence:

   690	fn check_integer_dominates() {
   691	    let ns: [u64; 13] = [

   706	    for &n in &ns {
   707	        for j in 0..=DEPTH {
   708	            assert!(occ_int(n, j) >= occ_hi(n, j), "occ dominance at ({n}, {j})");
   709	            assert!(
   710	                joint_int(n, j) >= joint_hi(n, j),
   711	                "joint dominance at ({n}, {j})"
   712	            );

    28	//! The shipped window derivation (`src/tree/mirror/streaming/window.rs`)
    29	//! implements the pair-based `A·B` adaptation of the same integer
    30	//! family; this tool certifies the one-corpus `N` forms and the
    31	//! integer-over-exact dominance those adaptations rest on.

Resolution: Move the exact-Chernoff oracle (`p_occ`, `binom_tail_log`, `chernoff_quantile`, `occ_hi`, `joint_hi`, `occ_quantile`, `stage_pop`; about 120 lines) into `src/tree/mirror/streaming/window/tests.rs` as a differential test `integer_envelopes_dominate_exact_chernoff` that sweeps the same `(N, depth)` grid against the real `occupied`, `jointly_occupied`, `children_quantile`, and `stage_population`, extending the oracle to asymmetric `(A, B)` (joint per-slot probability `p_occ(A) · p_occ(B)`) so the shipped pair-product path is covered. Re-state window.rs:570-578 to name that test. Delete the example's integer copies (538-682) and `check_integer_dominates`; if the Monte Carlo tiers are worth keeping they become an `#[ignore]`-gated test beside it, and what remains of the example is design exploration that retires to `.agent-notes/`. The cheaper interim step is `test = true` on the example in Cargo.toml (the `swarm` precedent at 194-198) with the two certification calls under `#[test]`, which closes the gating gap but not the replica gap. Acceptance: `just gate` runs a test that fails when any shipped integer quantile is lowered below its exact-Chernoff counterpart at a sampled `(A, B, depth)` (demonstrate once by a deliberate `- 1` on `bernstein`'s return); window.rs no longer names `examples/envelope_sim.rs`; the example's integer copies are gone or the example itself is.
Construction: In window.rs, change `bernstein`'s return to `mean_hi + (2 * mean_hi * t).isqrt() + t - 1` and run `just gate`: every committed test passes, because nothing compares the shipped quantiles against an exact tail. Then run `cargo run --release --example envelope_sim` by hand: it also passes, because it never sees window.rs.

Cross-reference: streaming-backend-window-32 states the same gap from the window's side and proposes the exact-distribution oracle in terms of `leaves_quantile`, `jointly_occupied`, and `child_slots_quantile`; the two resolutions are one piece of work.

### benches-envelope-13: The version-bounds pruning the benches advertise is enforced by no committed instrument
- Where: benches/in_memory.rs:170-174 (related: benches/in_memory.rs:225-229, benches/in_memory.rs:289-291, src/tree/typed/untyped/iter.rs:221-228, src/tree/tests.rs:872-884, src/tree.rs:616-622, src/tree/typed/untyped.rs:39-50)
- Class / severity / confidence: verification-gap / medium / medium
- Provenance: verified (absence: `grep -n 'cfg(test)' src/tree/typed/untyped/iter.rs` is empty; `grep -rln 'visited\|nodes_touched\|descents\|census' src tests benches` hits only the node-handle residency census (untyped.rs:39-50, tests/window_census.rs:1-16), which counts live handles, not walk visits; tree/tests.rs:880 says the prune/promote shortcuts "are pure optimization" and the proptest compares yield only; the meter pattern exists at tree.rs:616)
- Seen by: correctness; refutation: confirmed (adds that the `Throughput::Elements(delta)` denominator is exact because `causally::since` excludes its argument); history: no-rationale-found (ac68e8121 placed the claim as a bench measurement; no note considers a committed visit counter)
- Owner-gated: no (a test-only meter); the resolution lands in `src/tree`, another partition's files

`range_delta`, `observer_delta`, and `causal_delta` state that a small delta against a large snapshot costs "the delta plus the pruning frontier, not the tree", the same property iter.rs:224-228 documents. The only committed check of the range walk is a differential proptest that compares yield against the naive filter; a `range` degraded to a full scan with a per-leaf filter passes every test, and only a human reading Criterion output would notice. Principle 2: a claimed performance property must move a committed number; an oracle that agrees with the wrong implementation is a blind spot.

Evidence:

   170	/// Throughput is charged against the delta, not the set size: the
   171	/// memoized version bounds let the walk prune everything the checkpoint
   172	/// dominates, so a small delta against a large snapshot should cost the
   173	/// delta plus the pruning frontier, not the tree. Comparing one column
   174	/// (fixed delta) across set sizes is exactly that claim under measurement.

Resolution: Add a `cfg(test)` visit counter to the query walk in `src/tree/typed/untyped/iter.rs` (the root-hash meter at tree.rs:616 is the pattern), and a committed test on a constructed shape (large `N`, `D` much smaller than `N`, a checkpoint dominating all but the last `D` sends) asserting nodes visited is at most `c · (D + frontier)` with a liveness floor (at least `D` leaves visited). Then the bench docs cite the test as the enforced claim. Acceptance: a committed test fails when the coverage prune in the walk is disabled and passes at HEAD; the three bench docs point at it.
Construction: In iter.rs, force the query walk's `coverage` verdict to "descend" (never prune) and run the tree and integration suites: every committed test passes; only the `range_delta` and `observer_delta` Criterion columns move.

### benches-envelope-1: branch_hash restates Hash::branch's preimage by hand, and nothing ties the copy to the shipped function
- Where: benches/branch_hash.rs:14-18 (related: benches/branch_hash.rs:57-73, src/tree/typed/hash.rs:75-79, src/tree/typed/hash.rs:162-213, src/tree/typed/hash.rs:167, Cargo.toml:145)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (field-by-field comparison of `contiguous` against `Hash::branch`: `BRANCH_TAG = 1`, one-byte prefix length, backfilled big-endian `u16` count, `1 + MERKLE_HASH_LEN` child records, `with_capacity(4 + …)`; `grep -rn branch_hash src tests` finds only the hash.rs:167 comment; `src/testing.rs` exposes no branch hash)
- Seen by: correctness, perfapi; refutation: confirmed; history: deliberate-and-holds (3e912ce8, for review ruling R30; the stated constraint "not public API" is true, but `rumors::testing` existed then and benches build with `test-internals`)
- Owner-gated: no (a `#[doc(hidden)]`, feature-gated `testing` addition is not public API)

hash.rs:167 cites this bench as the measurement behind the one-shot form, and the bench reproduces the preimage layout by hand. The two agree today, but a layout change in `Hash::branch` (count width, tag, field order) leaves the bench measuring a stale preimage while the production comment keeps citing it as evidence. Principle 8: a "measured by" claim is only as good as the instrument's fidelity, and fidelity that depends on two files being edited together is a convention, not a check. The `MERKLE_HASH_LEN` import protected the bench through the 16-to-24 widening; the other fields have no such protection.

Evidence:

    14	//! The layout is restated locally because the tree's hashing internals are
    15	//! not public API; it mirrors the preimage documented at `Hash::branch`,
    16	//! which the hash tests pin byte-for-byte. `contiguous` reproduces the
    17	//! shipped form including its per-call buffer allocation, so the measured
    18	//! difference is the end-to-end cost a caller sees, not the hash core alone.

Resolution: Expose the shipped assembly through `rumors::testing` (a `branch_hash(prefix, children) -> [u8; MERKLE_HASH_LEN]` shim over `Hash::branch`) and make `contiguous` call it, keeping `streamed` local as the alternative under test (it restates the layout by nature); or add a test in `tests/` that `#[path]`-includes the bench (the `latency_link.rs` pattern) and asserts `contiguous(prefix, kids)[..MERKLE_HASH_LEN]` equals the shipped digest for each `FANOUTS` entry. Rewrite lines 14-18 to match. Acceptance: a committed test fails when `contiguous` and `Hash::branch` disagree on any preimage byte for the swept fan-outs, or `contiguous` is the shipped code.
Construction: Change `BRANCH_TAG` in hash.rs to 2 and run the gate: the hash tests fail on the pinned layout, but nothing points at the bench, which keeps hashing tag 1 and keeps being cited at hash.rs:167.

### benches-envelope-12: batch_insert's timed body includes dropping the N-node tree and an OsRng draw
- Where: benches/in_memory.rs:88-101 (related: benches/in_memory.rs:143-150, src/peer.rs:206-208, src/peer.rs:212-213)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (Cargo.lock pins criterion 0.5.1; its `bencher.rs:80-92` `iter` runs `black_box(routine())` in the loop and calls `measurement.end` afterwards, so the returned `Rumors` drops inside the timed interval; `iter_batched` at 247-257 with batch size 1 calls `end` before `drop(black_box(output))`, so `redact` excludes its drop; `iter_with_large_drop` at 185-190 is `iter_batched(…, SmallInput)`; peer.rs:206-208 `seed()` is `Self::seed_rng(&mut OsRng)`)
- Seen by: correctness, perfapi (the OsRng half); refutation: confirmed (criterion version corrected; the perfapi magnitude estimate dropped); history: deliberate-and-holds for the `b.iter` shape (10f1e5166c chose it for peak memory), which never addressed the denominator
- Owner-gated: no

`b.iter` returns the built set, so each iteration's deallocation of the tree (one million `Arc` nodes at the top size) is inside the measured interval, while `redact` uses `iter_batched` and excludes its drop; the two groups are denominated differently while both docs present a per-element cost of one operation. The doc names the seed as the only extra cost and omits the larger one. The seed also draws sixteen bytes from `OsRng` per iteration, a syscall the doc calls negligible without a bound. Denominate precisely.

Evidence:

    88	/// `b.iter` builds and drops one set per iteration, so peak memory stays at a
    89	/// single tree even at N = 1M. The trivial `seed().into_rumors()` is inside
    90	/// the timed body, but its cost is negligible against N inserts.

    97	            b.iter(|| {
    98	                let rumors: Rumors<()> = Peer::seed().into_rumors();
    99	                send_units(&rumors, black_box(n));
   100	                rumors
   101	            })

Resolution: `b.iter_batched(|| Peer::seed().into_rumors(), |rumors| { send_units(&rumors, black_box(n)); rumors }, BatchSize::PerIteration)`: one tree alive at a time (the doc's stated constraint), the seed and its syscall untimed, and the drop after `end`. Do not use `iter_with_large_drop`, whose `SmallInput` batching keeps several trees alive. State in the doc what the timed body contains. Acceptance: the `batch_insert` timed body ends before the tree is dropped and contains no `OsRng` draw; the doc names the denominator.

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

### Nits

The full record of each (provenance, history, evidence) is in the evidence file named in the last column.

| Id | Where | Claim | Resolution | Record |
|---|---|---|---|---|
| benches-envelope-3 | `benches/gossip_fixed.rs:152-153` | the identical corner reports `Throughput::Elements(0)` ("0 elem/s") | skip throughput at `param == 0`, or split the corner as gossip_grid does | `evidence/partitions/benches-envelope.md` |
| benches-envelope-15 | `benches/window_wallclock.rs:50-56` | the tokio runtime and link pair are built inside the timed routine | hoist `DelayedWire::new_wall_clock` above `bench_function` | `evidence/partitions/benches-envelope.md` |
| swarm-example-30 | `examples/swarm/tests.rs:69-73` | testdoc says "live count"; the body judges a four-sample post-ring mean | name the averaging in the doc | `evidence/partitions/swarm-example.md` |

## Positives

What this dimension of the crate does well, drawn from the partition reports and deduplicated. Each item names the artifact so a reader can check it.

- **Every test carries a doc comment stating its invariant in English.** The finalizers read every test body against its doc across every partition; the inaccuracies they found are the test-quality entries above, and they are the exceptions. `tools/testdoc` enforces the comment's presence with a `--self-test` that pins eight lexical cases and treats a missing root as a usage error, and every one of the 118 `proptest!` block fns in src, tests, examples, and benches carries an explicit `#[test]` and a `///` (verified by scan), so the checker's block-form blind spot is closed by convention today.
- **A deterministic deadlock detector, used almost everywhere.** `testing::run_to_quiescence` (src/testing.rs:366-394) turns a wire stall into `Err(Quiescence::Stalled)` without wall-clock guessing, disables tokio's cooperative budget so an inherited budget cannot read as a stall, and has committed tests for both properties; twenty-three files call it, `.config/nextest.toml`'s liveness rationale rests on it, and every in-memory session in the lifecycle, observation, and wire-format suites runs under it.
- **Independent differential oracles at every tier.** `Tree::join` is the streaming mirror's oracle (`streaming_matches_join_oracle`, and the proxy hub's `reconcile_locally`); `traverse::unknown` is the oracle for the streaming prune; `Local`'s bulk `leaves`/`assemble` overrides are held to the level-by-level `Convert` default by observational equivalence over deep-spine and wide-fan shapes; the integration suites use a `BTreeMap` oracle keyed by event index that reads redaction as absence and never invokes the merge; `reference_hash` re-derives the canonical tree shape from the sorted path set without calling the implementation; `bookmark_when.rs`'s `Model` predicts I/O from operation semantics and shares none of the crate's suppression arithmetic; `session_stats.rs` tallies bytes at the transport layer.
- **Known-bad mechanisms constructed and shown to fail.** `value_oracle_tripwires_catch_known_bad_mechanisms` (a suppressed redaction, a dropped insert), `folding_delivered_versions_can_lose_a_message` (the tempting wrong observer), `capacity_stress_witness_requires_inter_level_fan` (stalls at 253, completes at 254), the planted-byte negative control for `assert_control_drained` in tests/reuse.rs, every liveness clause of the link conformance suite with a fixture asserted `Stalled`, the `should_panic` knobs of the backend conformance suite, and the red-first history of the transmit-window pins (077b64db and ccd88401 both committed the failing test before the fix).
- **Meters with liveness floors and calibrated harness overhead.** `decode_alloc.rs` pairs every ceiling with a floor and uses a non-power-of-two length so doubling overshoot cannot mask; `encode_alloc.rs` calibrates its own harness allocations; `dispute_wire.rs` proves its counter alive on a session that disputes nothing and bounds the fixed overhead below one byte per message before pinning exact integers; `max_cut_spans_the_envelope_session` pins `MAX_CUT` from both sides using the counters the cuts spend; the root-hash meter has a liveness leg; `fan_occupancy.rs` pairs each `FAN + 1` equality with a paced negative control; `membership_population_contains_churn` pins a generated dimension's liveness in all three ways it could rot.
- **Fixtures that assert their own shape before the byte comparison.** The shape-staged snapshot fixtures in tests/gossip_snapshot.rs land a hash-dependent tree shape deterministically (pool, search, redact) and assert it before `insta` runs, two adding in-test liveness floors on the frame sequence; `hop_trace.rs`'s `transfer_pair` asserts its staged shape before any hop arithmetic; `early_first_child_dispute_pair` cross-checks its path simulation against the built trees; `handshake_liveness.rs` self-checks that the greeting version dwarfs the window without pinning greeting bytes.
- **Seeds and snapshots are mechanically anchored.** `tests/main.rs` is an empty binary whose doc says why it exists; `tests/seed_liveness.rs` reverses proptest's persistence resolution, guards its own vacuity, and commits a fixture for each verdict class; `tests/snapshot_liveness.rs` convicts any committed `.snap` no live test generates and refuses to skip a pending `.snap.new`; the two committed `bookmark_causality` seeds match their explicit reconstructions exactly; the capture harness holds every observer-hook item to the transport capture (`assert_items_account_for`) before rendering a pin.
- **Total checks where a point check would be the cheap artifact.** `intent_byte_space_is_exhaustive` sweeps all 256 bytes; `every_truncation_boundary_is_typed` cuts at every prefix; the bookmark format suite flips every single byte and truncates at every prefix; `leaf_query_matrix_is_exhaustive` asserts `checked == 8`; `injected_fault_reports_exact_violation` drives every scriptable fault through all 32 walk heights; `transport_failures_are_exact_and_fail_fast` predicts from the clean run whether each fault can fire and asserts the prediction; the error atlas closes coverage from both ends with wildcard-free `describe_*` matches and names its one remaining hole.
- **The verification tooling states its own limits.** `tools/mutantcheck`'s header states its model of record, dialect boundary, dedup rule, tool-version provenance, and liveness floors; `tools/covcheck` is tamper-evident in both directions and refuses a report with no branch instrumentation; `gate-streams` records an ok/failed marker per stream so an OOM-killed stream cannot read as a pass; `.config/nextest.toml` names the one collision its timeout budget accepts and the recovery procedure; `Cargo.toml` says why debug assertions stay on in the dev profile.
- **Type-level facts where prose would rot.** `define_peer!` plus the type-level height descent make the exchange schedule a compile-time fact (a wrong round count cannot build); `const _: () = assert!(size_of::<Node<Z>>() == size_of::<*const ()>())` pins the per-reference price the window rests on; `H0::HEIGHT == 0 && H32::HEIGHT == 32` is asserted at compile time; `Error::widen` closes its uninhabited arm with `match never {}`.
- **The suite is fast and quiet under load.** 836 tests in 22 s wall on a machine whose load average rose from 9.6 to 19 during the run, no slow markers, retries, or flaky results (the suite-economics sweep's run of record, read not reproduced); the virtual-time discipline in `benches/support/latency.rs` is what lets the window pins pass interleaved with everything else.

## Open questions for Finch

Decisions only the owner can make, deduplicated across partitions, each with a recommendation.

1. **`tests/future_size.rs`** (tests-wire-format-26, the entry of record; verification-infra-1 and suite-economics-1 refer there): lift the `cfg(not(debug_assertions))` and pin a budget measured in both profiles, or add a release-profile leg for this one binary to `gate-streams`? Recommendation: lift the cfg. The cfg's original premise (debug-only boxing in the traverse dispatch) left the tree in June; a dev-profile budget keeps the gate's build set unchanged and keeps the guard inside the mutants observer, which runs dev/test. Either way the number needs a fresh measurement first, and the first run may itself fail.
2. **The envelope certificate** (benches-envelope-32, streaming-backend-window-32): move the exact-Chernoff oracle into `window/tests.rs` as a differential proptest over the shipped `occupied`/`jointly_occupied`/`children_quantile`/`stage_population`, extended to asymmetric `(A, B)`, and retire the example's integer copies; or add `test = true` on the example as the interim step. Recommendation: the in-tree proptest, because the example's own header says it certifies a different family; retire the example only once the proptest demonstrably catches a lowered quantile.
3. **Mutation campaign cadence** (verification-infra-3): a named hand-run recipe (`cargo mutants -p rumors`) run once at HEAD to discharge the outstanding confirming re-run, with survivors triaged into tests, refactors, or roster entries with rationale; or a scheduled remote run recorded in `.agent-notes/`. Recommendation: the recipe now, the schedule later; the suite's kill power over the rewritten codec is unmeasured.
4. **Coverage pin scope** (verification-infra-4): extend `covcheck` to a rumors scope starting with `src/tree/mirror/streaming/remote/` and `src/bookmark/format.rs`, or state in the justfile's coverage section that rumors is out of scope for now. Recommendation: extend; the instrumented run already covers the workspace, so the cost is curation.
5. **A rumors fuzz target** (verification-infra-5): rule on `design/rumors-frame-fuzz.md`'s section 7 questions, then implement the session-level target plus a pair for the bookmark record and routed header decoders. Recommendation: rule; the doc has been a maintained spec with no implementation since 2026-07-27.
6. **Composite recipes** (verification-infra-2): add the coverage legs to `all` (or a `ci-full`) and re-state the justfile header so `ci` names only the ci job. Recommendation: add them to `all`; at HEAD `just all` passes while CI's coverage job fails on a `before` meter, and a developer had no way to see it. Whether a process-global heap meter is meaningful under `-C instrument-coverage` is a `before` question outside this review.
7. **The backend conformance suite** (conformance-28, conformance-25, conformance-31, conformance-24, conformance-30): land the census floor first and let it size `LOCAL_BUDGET` above the flat pre-charge; price `children` pointwise; drive `assemble` at a sub-root height over many runs; record the `parent` Some/None clause; compare against the expected union. Recommendation: all five, floor first; the constructed runs show the suite catches some of these faults only incidentally.
8. **The causality simulation's oracle** (tests-bookmark-9): extend `bookmark_causality.rs` with a per-network redaction ledger and a survival check, or accept that the random simulation pins only the degenerate recycle and leave the destruction class to the transmit suite's constructed schedules. Recommendation: the ledger; the proptest's headline claim is the recycle property, and the demonstrated mutant then becomes a committed check.
9. **The overlap shadow** (tests-observation-28, tests-common-12): add the `arb_overlap_schedule_with_shadow` meta-test, then decide whether the shadow should snapshot at the first `Step` (the real fork point) rather than at `Open`, and only then turn the executor's guard into an assertion. Recommendation: in that order; asserting before the fork point matches would fail valid schedules.
10. **Causal delivery order** (tests-observation-3): promote the replica-independent `(rank, canonical bytes)` order into the public `CausalMessages` contract, or keep the public under-promise and re-label the two tests as internal-order pins. Recommendation: promote; the order is a function of the set alone and a user building a replicated log will want it.
11. **Seed dispositions** (streaming-tests-20; tests-lifecycle-18's consequence): the two `faults.txt` lines whose comments name values no current strategy generates, and the four `cc` seeds in `proptest-regressions/async_wire.txt` that deleting `tests/async_wire.rs` would orphan. AGENTS.md says never strip a seed. Recommendation: correct the two comments (or remove the lines in a commit naming the orphaning), and re-home the async_wire seeds into `pairwise.txt` in the same commit that deletes the binary, so `seed_liveness` keeps passing.
12. **`ReorderingAcceptor`** (testing-infra-12): a unit-level witness that the patience wait produces a batch of two, keeping the proxy-tier `== 0` assertion cbc4a0aa ruled on, or dissolve the decorator and let the conformance suite's `ReversingAcceptor` carry the inversion proof. Recommendation: the witness plus the doc correction; it costs one test and preserves the recorded decision.
13. **The remote proxy's `Connect`/`CompleteConnect` impls** (remote-proxy-tests-24): once the harness runs production's arrangement by default, keep an impl with no production caller for containment.rs's server-position check, or dissolve it and let the in-process twin carry position-independence. Recommendation: dissolve, unless the wire path is judged to show something the in-process twin cannot.
14. **The renderer's injectivity pin** (remote-capture-atlas-17): an inverse parser from the rendering back to bytes (pins injectivity outright and makes the rendering grammar part of the contract) or the leaf-mutation property (cheaper, weaker, and already demonstrated to find the container-key hole). Recommendation: the inverse parser; it also catches the float-width and trailing-byte survivors the mutants note lists.
15. **The Lean wedge literal** (streaming-tests-28): a Lean-emitted expectation file read by `wedge_generator_matches_the_lean_literal` (the `muxprobe-expected.tsv` precedent), or the documented manual discipline. Recommendation: the expectation file, so the Rust literal can be deleted rather than maintained.
16. **`tests/tradeoff_probe.rs`** (verification-infra-14, tests-resource-link-window-18): a `just tradeoff-probe` recipe in the CI `instruments` job beside `worst-cases-pin`, or retirement with the design-corpus cell moved into an enforced suite. Recommendation: the recipe; `Peer::sync_memory_budget`'s public rustdoc quotes its measurement, and a scheduled run makes that citation accurate at about eleven seconds per run.
17. **The crate doc's operating-envelope figures** (verification-infra-6): derive them in a committed test from the pinned per-message wire law plus stated assumptions, with the doc citing the constant and its validity band; and decide the fate of `results/` (dated, BLAKE3-denominated, citing two removed files). Recommendation: derive in a test; re-denominate or excise `results/`.
18. **Deterministic seeding in the bookmark suites** (tests-bookmark-8): thread a seeded RNG through `World` via `Peer::seed_rng`, which is `#[doc(hidden)]` today. This intersects the api-core question of whether `seed_rng` becomes documented API. Recommendation: use it from the test suites regardless (the crate's own tests already do in nine suites) and rule on its visibility separately.
19. **`cargo test` as a supported entry** (tests-resource-link-window-19): if yes, `window_census`'s process-global census needs the lock its sibling meters use; if the justfile is the only sanctioned entry, it is a consistency nicety. Recommendation: add the lock either way; three lines, matching `decode_alloc.rs`.
20. **A wide-window differential and a walk-tier cancellation pin** (streaming-tests open questions): every `Tree::join` comparison in src runs at `WindowConfig::FLOOR`, and no test drops the `mirror` future mid-session. Recommendation: one wide-window arm in `streaming_matches_join_oracle`, and a cancel-at-drawn-poll pin that checks the local root is unchanged, unless the latter already exists with forged peers and I missed it.

## Counts

Entries by module section and severity (verification-gap and test-quality together; the per-class split follows). Two cross-lens duplicates of the `future_size` defect (verification-infra-1, suite-economics-1) stand as cross-references under tests-wire-format-26 and are not counted, so the finalizers' 205 entries are 203 here; the Low column's total previously read 113 against a column summing to 112 and is corrected.

| Section | High | Medium | Low | Nit | Total |
|---|---:|---:|---:|---:|---:|
| Verification infrastructure | 1 | 10 | 1 | 0 | 12 |
| Crate root and public surface | 0 | 2 | 1 | 1 | 4 |
| Session and bookmark | 0 | 0 | 1 | 2 | 3 |
| Link | 0 | 0 | 2 | 1 | 3 |
| Conformance | 1 | 3 | 7 | 1 | 12 |
| Tree core | 0 | 1 | 1 | 0 | 2 |
| Tree typed | 0 | 0 | 2 | 0 | 2 |
| Mirror common | 0 | 0 | 1 | 1 | 2 |
| Streaming backend and window | 0 | 2 | 5 | 1 | 8 |
| Materialized | 1 | 0 | 4 | 0 | 5 |
| Streaming protocol tests | 0 | 4 | 7 | 0 | 11 |
| Remote codec | 0 | 0 | 3 | 2 | 5 |
| Remote capture renderer, the codec test suite, and the error atlas | 0 | 1 | 3 | 3 | 7 |
| Remote adapter, streams, and the adapter test suites | 0 | 2 | 8 | 4 | 14 |
| Remote proxy and the proxy test suites | 1 | 5 | 9 | 0 | 15 |
| Test scaffolding | 0 | 3 | 3 | 2 | 8 |
| Integration tests: tests/common | 0 | 1 | 6 | 1 | 8 |
| Integration tests: lifecycle suites | 0 | 3 | 7 | 1 | 11 |
| Integration tests: bookmark suites | 2 | 4 | 2 | 2 | 10 |
| Integration tests: observation suites | 0 | 4 | 10 | 1 | 15 |
| Integration tests: disruption, gossip_when, pipelining, hop trace, handshake, and handshake liveness | 0 | 3 | 10 | 1 | 14 |
| Integration tests: allocation meters, message size, latency, opening supply, routed and TCP links, window suites | 1 | 2 | 8 | 1 | 12 |
| Integration tests: wire snapshots, CBOR evolution, dispute wire, legibility, snapshot liveness, payload depth, send bounds | 0 | 2 | 6 | 2 | 10 |
| Benches and examples | 0 | 2 | 5 | 3 | 10 |
| **Total** | **7** | **54** | **112** | **30** | **203** |

| Class | High | Medium | Low | Nit | Total |
|---|---:|---:|---:|---:|---:|
| verification-gap | 6 | 36 | 53 | 5 | 100 |
| test-quality | 1 | 18 | 59 | 25 | 103 |

Ten of the 203 entries were constructed and run by the review's witness agents (conformance-24, conformance-25, conformance-31, materialized-27, remote-capture-atlas-17, tests-bookmark-9, and tests-observation-28 in the first pass; conformance-28, remote-proxy-tests-10, and tests-bookmark-12 in the second), and two constructions in other classes are cross-referenced above (streaming-tests-11, tests-disruption-handshake-7). Of those twelve, eleven demonstrated the gap as stated or in refined form, and one (conformance-24) refuted the finding's prediction while confirming its kernel. Every numbered evidence line in the document was re-verified against HEAD for this synthesis; the anchor pass that followed found two runs off by one (verification-infra-3, remote-proxy-19) and corrected them in place. The five conformance entries merged after the rerun correctness lens (conformance-38 through conformance-42) were validated by the same anchor script at merge time.
