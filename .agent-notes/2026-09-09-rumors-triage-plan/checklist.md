# Rumors review checklist

Coverage reconciled on 2026-09-09 against `main` at `90e4509d`: **995 original findings, 171 rulings, and 55 later reports**. Each original finding and later report has one primary home below. Decisions can affect several items. This is complete scope accounting, not a claim that the code or old branch fixes have been verified.

**Current work:** routed connection pooling (02). Destructor safety landed as `2c77220a`, after user review and a clean gate. Next candidates: deep fixtures (03), then protocol termination (04).

An unchecked box means its full outcome still needs confirmation and, where necessary, implementation. A checked decision-only item records a declined/deferred/superseded proposal, not a code fix. Existing ledger SHAs below are leads for verification; they do not close an item. After verification and user-approved merge, check only the satisfied items and append the actual commit. Split an item if only part is complete.

While active, record `branch`, `worktree`, `review base`, and `working / ready for review` immediately below its checkbox. Review and edit the real branch in Zed; pause agent edits and base changes while the user reviews. “lgtm” approves merging that batch and proceeding to the next, after required verification, with no further confirmation. Run the project checks again after edits or rebasing. See [the working and prose standards](README.md#2-how-each-change-proceeds).

Every batch includes aggressive opportunistic simplification of touched code and prose. Understand implementation and callers first. Public comments explain contracts; private comments explain the relevant why, how, and where. Remove unnecessary comments, wrappers, instruments, and tests. No tests of doc wording or new source-scanning enforcement. Preserve regression seeds and deliberate snapshot discipline.

## Sources and reading order

The source IDs locate entries in the six [review reports](../2026-09-01-holistic-review-rumors/README.md). Read their resolutions together with the [rulings](../2026-09-01-holistic-review-rumors/triage/rulings.md), including amendments; an earlier resolution is not the final implementation instruction. Current user instructions prevail. The original [ledger](../2026-09-01-holistic-review-rumors/triage/ledger.tsv) is a source index, not a second tracker.

`N01`–`N55` identify the 55 table rows, in order, in [new findings](../2026-09-01-holistic-review-rumors/triage/new-findings.md): N01 is line 12 and N55 is line 66. They are reference labels, not another report. The [inventory](inventory.md) records prepared branches and their limits.

The numbered sections are work groups; each checkbox is an outcome to review. Several related boxes may land together, or one box may require smaller commits. Prerequisites concern the affected code, not every unrelated item in an earlier-numbered group.

## Work groups

- [01. Commit ownership and publication](#01-commit-ownership-and-publication)
- [02. Routed connection pooling](#02-routed-connection-pooling)
- [03. Deep-tree fixtures and reproducible schedules](#03-deep-tree-fixtures-and-reproducible-schedules)
- [04. Departure, malformed replies, and error preservation](#04-departure-malformed-replies-and-error-preservation)
- [05. Pipelining and conformance claims under deep geometry](#05-pipelining-and-conformance-claims-under-deep-geometry)
- [06. Window arithmetic and operating costs](#06-window-arithmetic-and-operating-costs)
- [07. Peer lifecycle, observers, and snapshots](#07-peer-lifecycle-observers-and-snapshots)
- [08. Bookmark contracts and file implementation](#08-bookmark-contracts-and-file-implementation)
- [09. Public diagnostics and observability](#09-public-diagnostics-and-observability)
- [10. Tree edits, joins, and memo ownership](#10-tree-edits-joins-and-memo-ownership)
- [11. Typed-tree representation and measured changes](#11-typed-tree-representation-and-measured-changes)
- [12. Wire codec, greeting, and stream adapters](#12-wire-codec-greeting-and-stream-adapters)
- [13. Link contract and transport cleanup](#13-link-contract-and-transport-cleanup)
- [14. Walk, materialized backend, and proxy simplification](#14-walk-materialized-backend-and-proxy-simplification)
- [15. Generators and property-test runs](#15-generators-and-property-test-runs)
- [16. Shared test and benchmark support](#16-shared-test-and-benchmark-support)
- [17. Lifecycle and observation tests](#17-lifecycle-and-observation-tests)
- [18. Disruption and handshake tests](#18-disruption-and-handshake-tests)
- [19. Bookmark behavioral tests](#19-bookmark-behavioral-tests)
- [20. Resource, wire-format, and public-surface tests](#20-resource-wire-format-and-public-surface-tests)
- [21. Benchmark cost and measured performance](#21-benchmark-cost-and-measured-performance)
- [22. Verification recipes and dependencies](#22-verification-recipes-and-dependencies)
- [23. Public documentation and explanation placement](#23-public-documentation-and-explanation-placement)
- [24. Module layout and remaining local cleanup](#24-module-layout-and-remaining-local-cleanup)
- [25. Publication preparation](#25-publication-preparation)
- [26. Historical, declined, deferred, and external work](#26-historical-declined-deferred-and-external-work)
- [27. Finish and reconcile](#27-finish-and-reconcile)

## 01. Commit ownership and publication

Prerequisites: None.

First implementation. Start from the payload destructor regression on current main; assess the prepared ownership solution against it.

- [x] **Release displaced roots and retained payloads after every replica guard is gone.**
  Merged as `2c77220a` after the user's “lgtm.” Retaining the original tree and incoming payload handles fixes destructor access during local and gossip commits. Three regressions timed out on the parent and pass with the fix. `just gate` passed in 329 seconds, including all 1,857 workspace tests and four future-size checks. The larger optimistic publication change remains below.
  Findings: `async-hazards-3`.
  Decisions: T34.

- [ ] **Move the gossip join out of the replica write lock.**
  Apply T170 after the focused destructor repair. Check root identity and ceiling before swapping; preserve concurrent commits on retry. Assess the fallback from its actual locking behavior: a standard RwLock alone does not establish starvation freedom. Keep the ownership lifetimes simple and the progress argument accurate.
  Decisions: T170.

- [ ] **Keep no-op, content-change, and party-only publication behavior distinct.**
  An all-skipped action group changes neither the root nor ceiling. Content changes advance the frontier; party-only changes do not wake content observers. Finish the plain-party representation separately in 07.
  Findings: `api-core-2`, `tree-core-29`.
  Decisions: T35, T37, T107, T166.

- [ ] **Reconcile the window census with the roots the commit actually retains.**
  The old peak-minus-two-generations premise breaks under the optimistic commit. First identify a useful quantity the existing census can measure; correct or remove the invalid assertion. Compare parent and changed code before attributing an improved pin. Treat a changed premise separately from an improved count; do not add a new meter to preserve the old presentation.
  Decisions: T139, T171.
  Later reports: N54.


## 02. Routed connection pooling

Prerequisites: None; finish before the transport cleanup in 13.

An independent correctness repair. Reuse the stream-level reproducer and assess the adapter-owned design.

- [ ] **Keep idle connections with their owning link and reuse them without disrupting another link.**
  **Working.** Branch: `codex/routed-pooling`; worktree: `/Users/oxide/src/rumors/.worktrees/routed-pooling`. Start from main after `2c77220a` and the tracking-note commit; record the exact review base when the worktree is created.
  Use T167 as the current design: adapter-owned bounded pooling, enabled by default, Dial responsible for dialing. Cover two links to one peer, cancellation during the reuse handshake, link shutdown, and router fairness. Preserve the stream-level failure demonstration. Earlier recycle/token designs are references, not additional requirements. N48 supplies consumer evidence to reassess against the retained API; N49’s sush keepalive work is external, not a Rumors implementation requirement.
  Findings: `link-14`, `link-28`.
  Decisions: T31, T152, T153, T156, T160, T164, T167.
  Later reports: N06, N07, N24, N25, N48, N49.


## 03. Deep-tree fixtures and reproducible schedules

Prerequisites: None to construct fixtures; 04 and 05 before enabling the complete CI run.

Keep ordinary hash-shaped coverage alongside explicit deep and wide cases.

- [ ] **Exercise deep data streams without changing production hashing or wire snapshots.**
  Retain useful RUMORS_PATH_SCHEDULE fixtures and both 28-byte and 31-byte depths. Establish which stream indices and root widths are reached. Resolve the variable-depth/wider-root proposal by the coverage it adds; do not replace the ordinary geometry with one narrow schedule. The collision recipe stays out of the gate and joins CI only after its known failures are resolved.
  Findings: `remote-capture-atlas-28`, `remote-proxy-tests-10`, `tests-wire-format-7`.
  Decisions: T23, T123, T132, T162, T163, T164.
  Later reports: N27, N28, N29.

- [ ] **Make cancellation tests depend on a reproducible session schedule.**
  Reproduce the differing poll counts, determine their cause, and retain the counterexamples. Keep the mirror-versus-join and wide-window properties. Deterministic ordering in a testable session must not turn the router’s fair select into a starving one.
  Findings: `streaming-tests-3`, `streaming-tests-6`, `streaming-tests-7`.
  Decisions: T19, T127, T132, T162.
  Later reports: N19, N42.
  Prior ledger receipts to reconcile: `ab65bcbf`.


## 04. Departure, malformed replies, and error preservation

Prerequisites: 03 for deep reproductions; coordinate all edits to the shared driver.

Reconcile the deep-geometry and vanish branches as one error-handling design, in small diffs.

- [ ] **End waits when the peer can no longer provide an owed stream.**
  Observe control departure while waiting for data. Sweep vanish points over measured bytes and stream counts, including deep geometry. Preserve a completed local outcome when nothing further is owed. Cover cancellation and the link’s actual departure obligations.
  Decisions: T145, T154, T165, T166.
  Later reports: N01, N21.

- [ ] **Preserve semantic violations when a concurrent transport failure also arrives.**
  One precedence rule covers route, proxy pump, and materialized pump: departure or supply failure can outrank only symptoms they can cause. Test both orientations and same-wave failures; return a pump failure rather than leaving it stranded in an unread relay.
  Findings: `remote-proxy-12`, `remote-proxy-31`, `remote-proxy-tests-26`.
  Decisions: T126, T132, T165.
  Later reports: N18, N32, N33.

- [ ] **Reproduce and fix the duplicated-reply stall at deep disputes.**
  N26 and N31 are the same outstanding defect. The reported delayed extra-reply check is a hypothesis; establish the blocked dependency before choosing the fix. Both endpoints must terminate with the appropriate failure.
  Decisions: T162.
  Later reports: N26, N31.


## 05. Pipelining and conformance claims under deep geometry

Prerequisites: 03–04; 02 for the pooled transport case.

Investigate claims that the existing deep fixtures contradict before changing their thresholds.

- [ ] **Determine and repair the depth-dependent pipelining behavior.**
  Use one hop measurement, confirm convergence and the serial floor, and reproduce the 347-hop deep session. Explain the actual cost and repair the scheduling if the promised pipelining fails. A materially different public performance promise needs review of the concrete evidence, not a looser bound to pass CI.
  Findings: `tests-disruption-handshake-10`, `tests-resource-link-window-25`, `verification-infra-16`.
  Decisions: T28, T98, T132, T159, T162.
  Later reports: N41.
  Prior ledger receipts to reconcile: `7c08a81b`.

- [ ] **Make conformance memory and liveness checks test their stated premises.**
  Reconcile the already-landed repairs and alignment accounting. Establish whether a wider stage must raise this measured peak at deep geometry. Keep a valid fixture or remove the invalid premise with its reason. Distinguish a sub-contract pooled-buffer tolerance from an actual Link guarantee; a passing 28-byte case does not settle the 31-byte stall.
  Findings: `conformance-24`, `conformance-25`, `conformance-28`, `conformance-29`, `conformance-30`, `conformance-31`, `conformance-35`, `conformance-40`, `conformance-41`, `conformance-42`, `tests-resource-link-window-19`, `tests-resource-link-window-20`.
  Decisions: T6, T18, T132, T139, T142.
  Later reports: N22, N34.
  Prior ledger receipts to reconcile: `084e4ec7`, `2697e7ac`, `3a014c3c`, `558666ef`, `709eea83`.


## 06. Window arithmetic and operating costs

Prerequisites: 01 for retained-root measurements; 03–05 when claims cover deep paths.

Keep compact independent calculations and measurements that support useful library claims.

- [ ] **Check the shipped window formulas against an independent numerical calculation; remove the simulator.**
  Verify the actual pair-product forms, boundaries, and integer rounding. Delete the obsolete envelope simulator and its remaining references once the useful oracle is retained.
  Findings: `benches-envelope-28`, `benches-envelope-29`, `benches-envelope-31`, `benches-envelope-32`, `benches-envelope-33`, `benches-envelope-34`, `streaming-backend-window-32`.
  Decisions: T10, T43.

- [ ] **Derive window charges and structural limits from the types or constants that own them.**
  Codec owns the derived stream count; link cites it. Thread target size into the walk. Derive fixed charges with size_of, remove impossible leaf-terminal work and defensive clamps, and explain the window’s actual budget premises.
  Findings: `link-3`, `materialized-10`, `remote-codec-3`, `streaming-backend-window-25`, `streaming-backend-window-26`, `streaming-backend-window-28`, `streaming-backend-window-30`, `streaming-backend-window-31`, `streaming-backend-window-34`, `testing-infra-2`.
  Decisions: T86, T87, T88, T103, T104, T120, T129, T132.

- [ ] **Support public cost claims with a sparse, affordable overhead grid.**
  Measure protocol bytes independently of payload across shared-history, insertion, and redaction cases. Derive numerical claims from named cases and test only useful trends. Reword the public paragraph with the user. Do not test doc-comment text or scan the test’s own roster. Do not add a deep-copy API merely to cache fixtures; N51 explains why the attempted cache changed bytes.
  Findings: `verification-infra-6`.
  Decisions: T17, T168.
  Later reports: N35, N51.

- [ ] **Keep only useful budget and residency measurements, with precise claims.**
  Confirm nonzero-target runs converge, witness growth during a session, and distinguish resting memory from admitted work. Remove the unsupported quoted tradeoff measurement and gratuitous matchability assertion. T110 declines new walk-allocation meters, not necessary behavioral tests.
  Findings: `materialized-30`, `streaming-backend-window-9`, `streaming-backend-window-11`, `streaming-backend-window-37`, `streaming-tests-16`, `tests-resource-link-window-7`, `tests-resource-link-window-8`, `tests-resource-link-window-16`, `tests-resource-link-window-18`, `tests-resource-link-window-28`, `verification-infra-14`.
  Decisions: T16, T28, T110, T113, T128, T130, T132.
  Prior ledger receipts to reconcile: `791b5360`, `7c08a81b`.


## 07. Peer lifecycle, observers, and snapshots

Prerequisites: 01 before party/publication cleanup; other API pieces can be separate.

Make each API change with its callers, before moving the shared harness.

- [ ] **Represent retirement ownership directly and return usable builders from failed joins.**
  Move the in-flight party into the retire future and use a plain Party in shared state. Both bookmark and ordinary bootstrap return typed Joined outcomes; bail/failure returns the whole builder. Exercise cancellation and exactly-once party reunion.
  Findings: `api-core-25`, `async-hazards-2`, `fresh-eyes-9`.
  Decisions: T37, T70, T77, T125, T128.

- [ ] **Return the stamped Version from send and simplify version lookup at callers.**
  Findings: `tests-common-2`, `tests-lifecycle-31`.
  Decisions: T67.

- [ ] **Provide the intended Snapshot read surface and equality contract.**
  Name the public iterator; add versions() and contains(); remove public Snapshot::hash and MERKLE_HASH_LEN; document equality in terms of network, live set, and frontier. Read through snapshots rather than adding parallel Rumors readers.
  Findings: `api-audit-2`, `api-core-34`, `api-core-35`, `api-core-36`, `benches-envelope-19`, `fresh-eyes-4`, `inventory-1`, `tree-core-5`.
  Decisions: T47, T60, T61, T65, T97, T132.

- [ ] **Accumulate observers through one shared channel representation.**
  Preserve observer attachment, wake behavior, and multiple observers; test the actual delivered changes. The shared Channel cleanup can land before the additive observer API.
  Findings: `api-core-19`, `api-core-33`, `session-bookmark-40`.
  Decisions: T74, T125, T132, T159.

- [ ] **Keep causal delivery tests within the public ordering contract.**
  Concurrent messages may be delivered in arbitrary order across ingestions. Test causal prerequisites and the intended single-pass behavior, not a replica-independent total ordering.
  Findings: `session-bookmark-37`, `tests-observation-3`.
  Decisions: T78, T132.

- [ ] **Make public bounds, borrowing, and wrapper traits express the real requirements.**
  Put payload bounds on types, add Borrow<Version> redaction and the approved wrapper traits without unnecessary generic bounds, and document why before is re-exported. Keep usize counts with runtime validation.
  Findings: `api-audit-1`, `api-audit-3`, `api-audit-7`, `api-audit-12`, `api-core-1`, `api-core-5`, `api-core-22`, `api-core-30`, `deps-6`, `tree-core-2`.
  Decisions: T47, T72, T73, T81, T83, T84, T125.

- [ ] **Core API implementation and comments.**
  Simplify forwarding and bootstrap plumbing, remove duplicate Peer/Rumors explanations, and make settings, version order, and cancellation comments match the retained API.
  Findings: `api-core-3`, `api-core-4`, `api-core-6`, `api-core-8`, `api-core-9`, `api-core-13`, `api-core-14`, `api-core-15`, `api-core-16`, `api-core-21`, `api-core-23`, `api-core-24`, `api-core-28`, `api-core-31`, `api-core-32`, `api-core-37`.
  Decisions: T52, T72, T82, T92, T132.


## 08. Bookmark contracts and file implementation

Prerequisites: 07 for join outcomes; owned-byte trait before conformance and file storage.

Separate the trait/API change, conformance suite, and file implementation into reviewable diffs.

- [ ] **Make Bookmark own its error type and the bytes it stores.**
  Remove the redundant serialization/error wrappers, state load/store/attach guarantees, and simplify consumers against that contract.
  Findings: `api-audit-4`, `async-hazards-1`, `session-bookmark-23`, `session-bookmark-24`, `session-bookmark-25`, `session-bookmark-29`, `session-bookmark-30`.
  Decisions: T62, T132.

- [ ] **State and test when a checkpoint makes older bookmark state reclaimable.**
  The first unsuppressed pre-session checkpoint after dominance is the relevant boundary. Include read failures and corrupt frames through Peer::bookmark.
  Findings: `session-bookmark-21`.
  Decisions: T32.

- [ ] **Ship a focused Bookmark conformance suite.**
  Validate caller implementations against the storage contract, including failure behavior. Explain the runner’s timeout responsibility rather than suggesting its check imposes one.
  Findings: `conformance-1`, `tests-bookmark-6`.
  Decisions: T69, T130.

- [ ] **Add a minimal atomic file-backed Bookmark behind a feature.**
  Build on the retained T62 trait and run the T69 suite against it. Choose the feature name and explain actual durability guarantees; keep this separate from persistent message storage.
  Findings: `fresh-eyes-8`.
  Decisions: T94, T159.

- [ ] **Session and bookmark implementation and explanations.**
  Simplify driver layers, reclaim/attach helpers, repeated payload conversion and unnecessary generic bounds. Explain checkpoint, epilogue and retirement behavior from the retained contracts.
  Findings: `session-bookmark-1`, `session-bookmark-2`, `session-bookmark-3`, `session-bookmark-4`, `session-bookmark-5`, `session-bookmark-6`, `session-bookmark-7`, `session-bookmark-9`, `session-bookmark-10`, `session-bookmark-11`, `session-bookmark-12`, `session-bookmark-13`, `session-bookmark-14`, `session-bookmark-15`, `session-bookmark-16`, `session-bookmark-17`, `session-bookmark-19`, `session-bookmark-22`, `session-bookmark-26`, `session-bookmark-27`, `session-bookmark-28`, `session-bookmark-32`, `session-bookmark-33`, `session-bookmark-35`, `session-bookmark-36`, `session-bookmark-39`, `session-bookmark-41`, `session-bookmark-43`, `session-bookmark-45`, `session-bookmark-47`.
  Decisions: T46, T48, T49, T50, T52, T56, T104, T129, T132.


## 09. Public diagnostics and observability

Prerequisites: 04 before error projection cleanup; 02 before router-event counters.

Do not let API polish obscure the protocol failures the earlier batch repairs.

- [ ] **Expose nameable, useful error categories and remove impossible cases.**
  Keep constructors/scheduling helpers internal. Implement the ruled head/listing diagnostics, useful Display output and local-versus-peer meaning. Use the fixed preamble array and infallible reply constructors where they eliminate states. Keep the flat Error<B> shape and explain widening.
  Findings: `api-audit-13`, `api-audit-14`, `api-core-7`, `fresh-eyes-10`, `materialized-17`, `mirror-common-10`, `mirror-common-15`, `remote-adapter-streams-19`, `remote-adapter-streams-22`, `remote-codec-9`, `remote-codec-18`, `remote-codec-19`, `remote-codec-28`, `remote-codec-30`, `remote-proxy-2`, `remote-proxy-3`, `session-bookmark-46`.
  Decisions: T46, T55, T63, T82, T85.

- [ ] **Expose useful session outcomes, settings, and event counts.**
  Add the finished hook, frame counts, window-stall zero/nonzero readout, configuration/debug readback and effective run budget. Return LinkInfo to the dialer. Router counters are limited to actual events demonstrated by a behavioral test; justify each retained hook. For session-bookmark-20, distinguish the already-ruled testing decoder export from a new public inspection API; identify any unmet operator need before proposing another surface.
  Findings: `api-audit-11`, `link-16`, `link-21`, `materialized-2`, `remote-adapter-streams-21`, `remote-codec-5`, `session-bookmark-20`, `session-bookmark-38`.
  Decisions: T66, T68.

- [ ] **Gate test-only controls and provide small testing exports where they remove copied derivations.**
  Gate seed_rng. Eager warming in 10 replaces the warm_caches API rather than adding more hooks. Export the ruled leaf-path/bookmark decoding helpers only where consumers need them; remove assert_parent_early.
  Findings: `api-audit-15`, `api-core-11`, `api-core-12`, `benches-envelope-6`, `materialized-22`, `tests-common-6`, `tests-lifecycle-10`.
  Decisions: T64, T85, T96.


## 10. Tree edits, joins, and memo ownership

Prerequisites: 01; do root-version changes before dependent optimization measurements.

Keep correctness fixes separate from independent speed improvements when that makes review easier.

- [ ] **Store each leaf’s action version and traverse sorted actions without repeated sorting or copying.**
  Use the single-sort and slice-recursion design, preserving action order, unchanged-root identity, and deletion semantics. Assess N12’s alternative only against the retained simpler walk. Do not recreate the eliminated join sink or add its proposed meter.
  Findings: `tree-core-11`, `tree-core-13`, `tree-core-14`, `tree-core-16`, `tree-core-27`, `tree-core-30`, `tree-core-31`.
  Decisions: T38, T39, T107, T124, T132.
  Later reports: N12, N23.

- [ ] **Publish roots with their required memos already warm.**
  Warm the required hash/version metadata outside replica guards. Remove explicit warm hooks and ensure the first greeting does not walk a cold tree. Measure the remaining one-difference session cost rather than assuming this removes it.
  Findings: `api-core-29`, `async-hazards-4`, `tree-core-8`.
  Decisions: T42, T96, T132.

- [ ] **Remove message-buffer slack with a direct size invariant.**
  Use an exact boxed cache where appropriate; verify capacity directly. No new allocation instrument is required.
  Findings: `api-core-10`.
  Decisions: T110, T113.

- [ ] **Protect tree algebra, deletion, and traversal invariants with meaningful cases.**
  Cover survivor construction and redaction in associativity, use a checked geometry hint, and repair vacuous root comparisons. Preserve regression seeds when changing generators.
  Findings: `streaming-tests-18`, `tree-core-22`, `tree-core-24`, `tree-core-33`, `tree-core-34`, `tree-typed-30`.
  Decisions: T39, T112, T124, T128, T132.

- [ ] **Tree invariants, names, and helper cleanup.**
  Clarify root, act/react and join contracts; remove redundant forwarders/names and explanations that confuse action versions, observer notifications or deletion.
  Findings: `tree-core-1`, `tree-core-3`, `tree-core-4`, `tree-core-9`, `tree-core-10`, `tree-core-12`, `tree-core-15`, `tree-core-17`, `tree-core-18`, `tree-core-19`, `tree-core-20`, `tree-core-21`, `tree-core-23`, `tree-core-25`, `tree-core-26`, `tree-core-28`, `tree-core-32`, `tree-core-35`.
  Decisions: T48, T49, T52, T53, T124, T132.


## 11. Typed-tree representation and measured changes

Prerequisites: 10 when a change affects the same traversal or baseline.

Derive repeated structure once; measure the proposed storage tradeoffs before retaining them.

- [ ] **Remove the leaf-hash heap buffer; measure the remaining representation tradeoffs.**
  Measure inline prefixes, branch preimages, and per-leaf conversion against the existing benches and census. Retain only demonstrated improvements and explain regressions. Avoid timing destructor work accidentally.
  Findings: `tree-core-7`, `tree-typed-6`, `tree-typed-23`.
  Decisions: T111, T132.

- [ ] **Derive height/radix structure and simplify owned traversal states.**
  Remove redundant enumerations, copies, and optional states; keep typed boundaries and iterator guarantees clear. Check the release assertion for erased-prefix length.
  Findings: `tree-typed-11`, `tree-typed-12`, `tree-typed-16`, `tree-typed-17`, `tree-typed-20`, `tree-typed-21`, `tree-typed-24`, `tree-typed-31`, `tree-typed-34`, `tree-typed-35`.
  Decisions: T36, T125, T132.

- [ ] **Check typed-tree preconditions where a real failure construction is possible.**
  Read the debug assertions and keep targeted cases that detect a broken invariant. Remove or justify unreachable guards; do not add a generic assertion-testing framework. Correct testdoc claims about record width, openers and ceilings.
  Findings: `tree-typed-7`, `tree-typed-9`.
  Decisions: T132.

- [ ] **Typed-tree names, helpers, and documentation.**
  Simplify typed conversion/helpers and redundant trait bounds; correct prefix/path, hash, iterator and panic explanations at their owning level.
  Findings: `tree-typed-1`, `tree-typed-2`, `tree-typed-3`, `tree-typed-4`, `tree-typed-5`, `tree-typed-8`, `tree-typed-10`, `tree-typed-13`, `tree-typed-14`, `tree-typed-15`, `tree-typed-18`, `tree-typed-19`, `tree-typed-22`, `tree-typed-25`, `tree-typed-26`, `tree-typed-27`, `tree-typed-28`, `tree-typed-29`, `tree-typed-32`, `tree-typed-33`.
  Decisions: T46, T49, T52, T95, T97, T125, T132.


## 12. Wire codec, greeting, and stream adapters

Prerequisites: 04 and 09 before simplifying shared failure paths.

Preserve deliberate wire-format decisions and the sync decoding oracle.

- [ ] **Keep sync/async decoding consistent, including partial reads and boundary failures.**
  Confirm the prior codec fixes, share duplicated fragments, and exercise both sides of length boundaries and errors after a partial read. Preserve failure order. Remove the RunBudget default self-comparison and make test explanations match the properties actually exercised.
  Findings: `mirror-common-5`, `mirror-common-8`, `remote-codec-6`, `remote-codec-8`, `remote-codec-10`, `remote-codec-11`, `remote-codec-14`, `remote-codec-15`, `remote-codec-16`, `remote-codec-17`, `remote-proxy-tests-25`.
  Decisions: T24, T33, T41, T126, T132.
  Prior ledger receipts to reconcile: `1b7f3de4`, `28055b77`, `ded8318e`.

- [ ] **Parse the version head directly and simplify the fixed greeting vocabulary.**
  Keep canonicality/error behavior deliberate and explain the accepted rule. Replace fixed-roster Option assembly and duplicated head tables with direct code. Replace inert random-magic tests with actual malformed frames.
  Findings: `inventory-5`, `mirror-common-3`, `mirror-common-14`, `remote-codec-24`, `remote-codec-26`, `remote-codec-27`, `remote-proxy-tests-2`.
  Decisions: T105, T126, T132.

- [ ] **Demonstrate decode-channel progress below a fan and remove per-reply overhead only if measured.**
  Construct the sub-FAN case with the existing harness. Explain amortization accurately. Measure a pull-reader change before retaining it; do not add a standing meter by default.
  Findings: `remote-adapter-streams-6`, `remote-proxy-27`.
  Decisions: T106.

- [ ] **Simplify stream state machines and repeated encoder/decoder plumbing.**
  Remove impossible supply states, duplicate flush/finish work, and representations the type already determines. Keep bounded buffering, incremental yields, and single failure publication explicit.
  Findings: `async-hazards-6`, `mirror-common-7`, `mirror-common-17`, `mirror-common-24`, `mirror-common-25`, `mirror-common-32`, `remote-adapter-streams-2`, `remote-adapter-streams-3`, `remote-adapter-streams-4`, `remote-adapter-streams-5`, `remote-adapter-streams-9`, `remote-adapter-streams-10`, `remote-adapter-streams-12`, `remote-adapter-streams-14`, `remote-adapter-streams-16`, `remote-adapter-streams-20`, `remote-adapter-streams-24`, `remote-adapter-streams-25`, `remote-adapter-streams-26`, `remote-adapter-streams-27`, `remote-adapter-streams-28`, `remote-adapter-streams-29`, `remote-adapter-streams-30`.
  Decisions: T52, T119, T126, T132.

- [ ] **Test adapter failures at the actual conversion and ordering boundaries.**
  Inject leaf conversion failures; use legal full-fan and over-limit constructions; merge duplicate law sweeps without losing their assertions. Replace self-comparisons and wrong fixture claims with observable behavior.
  Findings: `remote-adapter-tests-4`, `remote-adapter-tests-6`, `remote-adapter-tests-7`, `remote-adapter-tests-9`, `remote-adapter-tests-10`, `remote-adapter-tests-11`, `remote-adapter-tests-12`, `remote-adapter-tests-13`, `remote-adapter-tests-14`, `remote-adapter-tests-15`, `remote-adapter-tests-17`, `remote-adapter-tests-19`, `remote-adapter-tests-20`, `remote-adapter-tests-21`, `remote-adapter-tests-22`.
  Decisions: T126, T132.

- [ ] **Avoid needless scratch initialization in listing reads.**
  Use the same initialized-length discipline as the body reader only where the read API proves it safe. Preserve short-read and error behavior; do not add unsafe code merely to obtain a smaller diff.
  Findings: `remote-codec-12`.
  Decisions: T132.

- [ ] **Shared wire types and protocol explanations.**
  Clarify frame/schedule types and rename ambiguous helpers. Remove redundant bounds, wrappers and protocol-selection language while retaining useful decoding-oracle explanations.
  Findings: `mirror-common-1`, `mirror-common-2`, `mirror-common-4`, `mirror-common-6`, `mirror-common-9`, `mirror-common-11`, `mirror-common-13`, `mirror-common-18`, `mirror-common-19`, `mirror-common-20`, `mirror-common-21`, `mirror-common-22`, `mirror-common-23`, `mirror-common-26`, `mirror-common-27`, `mirror-common-28`, `mirror-common-29`, `mirror-common-30`, `mirror-common-31`, `mirror-common-34`, `mirror-common-35`, `mirror-common-36`, `mirror-common-37`.
  Decisions: T46, T49, T50, T52, T53, T54, T132.

- [ ] **Stream adapter names and explanations.**
  Clarify what each adapter owns and when it reads, writes, flushes or reports failure; simplify repetitive names and comments.
  Findings: `remote-adapter-streams-1`, `remote-adapter-streams-7`, `remote-adapter-streams-8`, `remote-adapter-streams-11`, `remote-adapter-streams-13`, `remote-adapter-streams-15`, `remote-adapter-streams-17`, `remote-adapter-streams-18`, `remote-adapter-streams-23`.
  Decisions: T48, T52, T57, T132.

- [ ] **Adapter test helpers and comments.**
  Share repeated fixtures and setup, improve failure diagnostics, and make each test comment state the behavior its assertions establish.
  Findings: `remote-adapter-tests-1`, `remote-adapter-tests-2`, `remote-adapter-tests-3`, `remote-adapter-tests-5`, `remote-adapter-tests-8`, `remote-adapter-tests-16`, `remote-adapter-tests-18`.
  Decisions: T48, T50, T52, T119, T132.

- [ ] **Codec helpers, constants, and comments.**
  Derive repeated limits once, remove vacuous defaults and unnecessary wrapper/visibility distinctions, and document byte units and frame boundaries accurately.
  Findings: `remote-codec-1`, `remote-codec-2`, `remote-codec-4`, `remote-codec-7`, `remote-codec-13`, `remote-codec-20`, `remote-codec-21`, `remote-codec-22`, `remote-codec-23`, `remote-codec-25`, `remote-codec-29`, `remote-codec-31`, `remote-codec-32`, `remote-codec-33`.
  Decisions: T46, T50, T52, T99, T132.


## 13. Link contract and transport cleanup

Prerequisites: 02, then relevant API/error changes in 09.

Keep the contract that callers implement distinct from router implementation details.

- [ ] **Simplify link construction, validated headers, and configuration.**
  Publish Link fields and remove LinkParts; provide the approved SessionState traits/readback; validate advertised names once in a newtype; wrap the router read counter. Retain usize configuration checks and TCP_NODELAY.
  Findings: `inventory-18`, `link-5`, `link-6`, `link-20`, `link-25`, `link-29`.
  Decisions: T44, T45, T47, T80, T81, T84, T156.

- [ ] **Make focused conformance probes exercise independent streams and cancellation without a backlog assumption.**
  Keep focused signatures and loosen only unnecessary bounds. Poll accept concurrently with the cancellation probe. Resolve the lossy-fixture blame and concurrent window accounting cases. Keep negative controls only where they demonstrate a real blind spot.
  Findings: `conformance-6`, `conformance-7`, `conformance-8`, `conformance-9`, `conformance-11`, `conformance-13`, `conformance-16`, `conformance-18`, `conformance-20`, `conformance-27`, `conformance-37`, `conformance-38`, `conformance-39`, `link-11`, `link-12`, `link-31`, `testing-infra-12`.
  Decisions: T21, T69, T71, T101, T122, T128, T132.
  Prior ledger receipts to reconcile: `c734e79f`.

- [ ] **Remove measured or obvious per-connection overhead without changing the transport contract.**
  Remove redundant boxing, reference-count clones, and scratch storage where ownership already permits it. Rewrite the affected link/router comments against the actual bounded-pooling behavior.
  Findings: `link-1`, `link-27`.
  Decisions: T132.

- [ ] **Conformance helpers and contract explanations.**
  Align probe explanations with their actual observations; simplify fixture setup, visibility and bounds without changing what a caller-built implementation must satisfy.
  Findings: `conformance-2`, `conformance-3`, `conformance-4`, `conformance-5`, `conformance-10`, `conformance-12`, `conformance-14`, `conformance-15`, `conformance-17`, `conformance-19`, `conformance-21`, `conformance-22`, `conformance-23`, `conformance-26`, `conformance-32`, `conformance-33`, `conformance-34`, `conformance-36`.
  Decisions: T48, T49, T50, T132.

- [ ] **Link and router helpers and documentation.**
  Rewrite the public stream, cancellation and pooling contracts; simplify router bookkeeping, duplicated header layout, names and private explanations.
  Findings: `link-2`, `link-4`, `link-7`, `link-8`, `link-9`, `link-10`, `link-13`, `link-15`, `link-17`, `link-18`, `link-19`, `link-22`, `link-23`, `link-24`, `link-26`, `link-30`.
  Decisions: T46, T49, T55, T80, T100, T128, T132.


## 14. Walk, materialized backend, and proxy simplification

Prerequisites: 04, 06, 09 and the relevant codec edits in 12.

Refactor the error handling we retain, avoiding a second representation of the same state.

- [ ] **Share classification and completion paths without hiding backend failures.**
  Confirm the landed walk fixes; borrow Recorder, collapse repeated completion/classification, and remove redundant invariants or enum variants. Test failure after work has begun, not only empty early exits.
  Findings: `materialized-13`, `materialized-14`, `materialized-26`, `materialized-27`, `materialized-31`, `materialized-35`, `materialized-36`, `materialized-40`, `remote-proxy-23`, `remote-proxy-25`, `remote-proxy-28`, `remote-proxy-29`, `remote-proxy-30`, `streaming-backend-window-20`, `streaming-backend-window-23`, `streaming-backend-window-29`.
  Decisions: T26, T40, T118, T128, T132.
  Later reports: N04, N05.
  Prior ledger receipts to reconcile: `0bebd811`, `bd96695c`.

- [ ] **Collapse the handshake handoff and name its actual premises once.**
  Use one election enum, one greeting-derived bundle, and one Progress shape shared by proxy and walk. Preserve responsibility boundaries while deleting pass-through wrappers.
  Findings: `async-hazards-5`, `materialized-11`, `materialized-34`, `mirror-common-16`, `remote-proxy-4`, `remote-proxy-7`, `remote-proxy-24`, `streaming-tests-24`.
  Decisions: T99, T118, T119, T128, T132.

- [ ] **Run the height-indexed trait experiment and judge the result.**
  Construct one alternative to the repeated bound chains. Compare compile time and diagnostic clarity; retain a simpler result or record the evidence for abandoning it. This is a bounded experiment, not a required new abstraction.
  Findings: `mirror-common-33`.
  Decisions: T117, T132.

- [ ] **Exercise meaningful walk and proxy failures, shedding, isolation, and terminal behavior.**
  Ensure injected faults fire and deep/wide work reaches the relevant paths. Merge implied checks and tiny finite-case properties where useful; fix the manual property runner so failures and seeds are preserved. Replace duplicate rosters with one definition rather than source-reading tests.
  Findings: `materialized-28`, `materialized-37`, `materialized-39`, `remote-proxy-19`, `remote-proxy-20`, `remote-proxy-tests-5`, `remote-proxy-tests-6`, `remote-proxy-tests-7`, `remote-proxy-tests-8`, `remote-proxy-tests-9`, `remote-proxy-tests-12`, `remote-proxy-tests-15`, `remote-proxy-tests-18`, `remote-proxy-tests-20`, `remote-proxy-tests-21`, `remote-proxy-tests-22`, `remote-proxy-tests-23`, `remote-proxy-tests-24`, `streaming-backend-window-15`, `streaming-backend-window-16`, `streaming-backend-window-18`, `streaming-backend-window-36`, `streaming-backend-window-38`, `streaming-tests-11`, `streaming-tests-17`, `streaming-tests-19`, `streaming-tests-23`, `streaming-tests-26`.
  Decisions: T22, T26, T101, T126, T128, T132, T159.
  Later reports: N03.
  Prior ledger receipts to reconcile: `0153509f`, `ab65bcbf`.

- [ ] **Materialized-backend implementation and explanations.**
  Simplify local helpers, bounds, counters and result representation. Explain backend materiality and ownership for maintainers, without narrating obsolete designs.
  Findings: `materialized-1`, `materialized-3`, `materialized-4`, `materialized-5`, `materialized-6`, `materialized-7`, `materialized-8`, `materialized-9`, `materialized-12`, `materialized-15`, `materialized-16`, `materialized-18`, `materialized-19`, `materialized-20`, `materialized-21`, `materialized-23`, `materialized-24`, `materialized-25`, `materialized-29`, `materialized-32`, `materialized-33`, `materialized-38`.
  Decisions: T46, T48, T49, T52, T54, T128, T132.

- [ ] **Proxy state and explanations.**
  Simplify request ownership and repetitive driver arms; explain route, pump and window responsibilities at the code that owns them.
  Findings: `remote-proxy-1`, `remote-proxy-5`, `remote-proxy-6`, `remote-proxy-8`, `remote-proxy-9`, `remote-proxy-10`, `remote-proxy-11`, `remote-proxy-13`, `remote-proxy-14`, `remote-proxy-15`, `remote-proxy-16`, `remote-proxy-17`, `remote-proxy-18`, `remote-proxy-21`, `remote-proxy-22`, `remote-proxy-26`, `remote-proxy-32`.
  Decisions: T48, T49, T52, T57, T126, T132.

- [ ] **Proxy test helpers and comments.**
  Share duplicated setup and error projections, correct endpoint/handshake descriptions, and remove assertions that add no coverage.
  Findings: `remote-proxy-tests-1`, `remote-proxy-tests-3`, `remote-proxy-tests-4`, `remote-proxy-tests-11`, `remote-proxy-tests-13`, `remote-proxy-tests-14`, `remote-proxy-tests-16`, `remote-proxy-tests-17`, `remote-proxy-tests-19`.
  Decisions: T48, T50, T52, T132.

- [ ] **Backend/window helpers and explanations.**
  Simplify duplicated scheduling, unused role state and helper placement. Separate backend guarantees from window arithmetic and explain each at its owning abstraction.
  Findings: `streaming-backend-window-1`, `streaming-backend-window-2`, `streaming-backend-window-3`, `streaming-backend-window-4`, `streaming-backend-window-5`, `streaming-backend-window-6`, `streaming-backend-window-7`, `streaming-backend-window-8`, `streaming-backend-window-10`, `streaming-backend-window-12`, `streaming-backend-window-13`, `streaming-backend-window-14`, `streaming-backend-window-17`, `streaming-backend-window-19`, `streaming-backend-window-21`, `streaming-backend-window-22`, `streaming-backend-window-24`, `streaming-backend-window-27`, `streaming-backend-window-33`, `streaming-backend-window-35`.
  Decisions: T46, T48, T49, T53, T102, T103, T128, T132.

- [ ] **Streaming test helpers and comments.**
  Consolidate runner/fixture boilerplate and repeated constants; correct outcome, capacity, seed and trace explanations against the actual tests.
  Findings: `streaming-tests-1`, `streaming-tests-4`, `streaming-tests-5`, `streaming-tests-8`, `streaming-tests-9`, `streaming-tests-10`, `streaming-tests-12`, `streaming-tests-13`, `streaming-tests-14`, `streaming-tests-15`, `streaming-tests-21`, `streaming-tests-22`, `streaming-tests-25`, `streaming-tests-27`.
  Decisions: T48, T49, T52, T128, T132.


## 15. Generators and property-test runs

Prerequisites: Scoped generator repairs can start independently; workspace settings wait for compatible before generators.

Retain direct constructions and counterexamples, not the prepared branch’s expanded linter.

- [ ] **Generate constrained cases directly while preserving the regressions they must cover.**
  Repair rejecting strategies throughout Rumors; turn known shrunk values into explicit regressions when RNG consumption changes their replay. Keep all seed files. Repair the listen ordering search by varying versions. Do not add spellings scanners or source-inspecting case-count tests.
  Findings: `remote-capture-atlas-24`.
  Decisions: T132, T157, T161.
  Later reports: N13, N17, N20, N36, N45, N46.

- [ ] **Run meaningful high-count release properties with one case-count setting.**
  Use environment-controlled counts, fast gate runs and the ruled 4000-case CI target after measuring the retained suite. Exclude dev-profile meter pins from release for their actual premise. Global zero-rejection budgets depend on before’s generators. Assess --no-fail-fast for complete failure reporting. The optional 16000-case rerun is not a completion prerequisite.
  Findings: `suite-economics-4`.
  Decisions: T116, T129, T148, T151, T164.
  Later reports: N14, N37, N38.


## 16. Shared test and benchmark support

Prerequisites: Relevant lifecycle, Bookmark, link and observer signatures from 07–09 and 13.

Move stable helpers once, then simplify their callers by category.

- [ ] **Compile the shared test and bench harness once through a path dev-dependency.**
  Separate public testing support from dev-only helpers. Consolidate category binaries where this reduces build cost and redundant execution. Rehome seeds to paths proptest actually reads; preserve every seed and its counterexample.
  Findings: `benches-envelope-22`, `suite-economics-8`, `tests-common-8`, `tests-disruption-handshake-28`.
  Decisions: T115.

- [ ] **Use one set of session drivers, observer readouts, fingerprints, and fault wrappers.**
  Include bookmark attach and handoff drain checks, both donor/absorber fault directions, and overlap forks at the actual first poll. Prefer the public observer and deterministic poller where applicable. Distinguish byte faults and vanish faults by behavior.
  Findings: `session-bookmark-8`, `testing-infra-8`, `testing-infra-9`, `testing-infra-11`, `testing-infra-13`, `testing-infra-17`, `testing-infra-18`, `testing-infra-19`, `testing-infra-20`, `testing-infra-23`, `tests-common-3`, `tests-common-4`, `tests-common-12`, `tests-common-14`, `tests-common-15`, `tests-common-16`, `tests-common-18`, `tests-common-19`, `tests-common-20`, `tests-common-22`, `tests-common-25`, `tests-common-29`, `tests-common-30`.
  Decisions: T76, T129, T130, T132, T144, T150.
  Prior ledger receipts to reconcile: `b09aaf8b`.

- [ ] **Remove allocation-count nondeterminism and load-sensitive session deadlines.**
  Reproduce the extra allocation before changing the count; isolate the measured region from other workers and lazy initialization. Replace the shared-runtime two-second correctness deadline with an appropriate progress check. Reuse existing support rather than building a new measurement framework.
  Findings: `tests-resource-link-window-5`.
  Decisions: T132.
  Later reports: N11, N39, N50.

- [ ] **Testing support implementation and comments.**
  Simplify fault wrappers and helper placement; correct preamble, marker and diagnostic descriptions and state real poll-budget behavior.
  Findings: `testing-infra-1`, `testing-infra-3`, `testing-infra-5`, `testing-infra-6`, `testing-infra-7`, `testing-infra-10`, `testing-infra-14`, `testing-infra-15`, `testing-infra-16`.
  Decisions: T48, T49, T52, T53, T129, T132.

- [ ] **Common test helpers and explanations.**
  Share repeated oracles, fault wrappers and helper types; clarify schedule rounds versus polls and use the public observer instead of reimplementing it.
  Findings: `tests-common-1`, `tests-common-5`, `tests-common-7`, `tests-common-9`, `tests-common-10`, `tests-common-11`, `tests-common-13`, `tests-common-17`, `tests-common-21`, `tests-common-23`, `tests-common-24`, `tests-common-26`, `tests-common-27`, `tests-common-28`.
  Decisions: T48, T50, T52, T56, T130, T132.


## 17. Lifecycle and observation tests

Prerequisites: 07 and 16; 15 for generator changes.

Work one behavioral suite at a time, keeping the important invariants visible.

- [ ] **Make lifecycle properties check content, versions, and party ownership directly.**
  Include stale-redaction suppression after bootstrap, populated network mismatch in both handoff directions, actual epoch readback and final drain. Fuse implied multi-peer laws, remove redundant binaries and preserve their seeds. Reduce the costly fleet retirement fixture to the work its assertions require.
  Findings: `api-core-20`, `suite-economics-5`, `tests-lifecycle-4`, `tests-lifecycle-5`, `tests-lifecycle-7`, `tests-lifecycle-9`, `tests-lifecycle-11`, `tests-lifecycle-15`, `tests-lifecycle-16`, `tests-lifecycle-17`, `tests-lifecycle-18`, `tests-lifecycle-20`, `tests-lifecycle-21`, `tests-lifecycle-23`, `tests-lifecycle-25`, `tests-lifecycle-26`, `tests-lifecycle-28`, `tests-lifecycle-29`, `tests-lifecycle-33`.
  Decisions: T26, T59, T108, T130, T131, T132, T158.
  Later reports: N16.
  Prior ledger receipts to reconcile: `e1f1bd69`.

- [ ] **Check observer progress, duplicates, wakes, and content with varied schedules.**
  Include remote redactions and gossip, no-op commits with observers, both overlap orientations, nonzero shedding and actual retirement content movement. Remove vacuous protocol/encoded-bit assertions. Repair the overlap oracle itself; a test of the model is useful only if it detects a wrong execution.
  Findings: `tests-observation-5`, `tests-observation-7`, `tests-observation-8`, `tests-observation-9`, `tests-observation-10`, `tests-observation-11`, `tests-observation-13`, `tests-observation-14`, `tests-observation-17`, `tests-observation-20`, `tests-observation-22`, `tests-observation-23`, `tests-observation-24`, `tests-observation-25`, `tests-observation-26`, `tests-observation-28`, `tests-observation-30`, `tests-observation-35`, `tests-observation-36`.
  Decisions: T13, T78, T90, T130, T132, T144.
  Prior ledger receipts to reconcile: `428acc44`.

- [ ] **Lifecycle test fixtures and comments.**
  Consolidate peer builders, fingerprints and session helpers; remove redundant tests and runtime setup while keeping version, content and party invariants explicit.
  Findings: `tests-lifecycle-2`, `tests-lifecycle-3`, `tests-lifecycle-6`, `tests-lifecycle-8`, `tests-lifecycle-12`, `tests-lifecycle-13`, `tests-lifecycle-14`, `tests-lifecycle-19`, `tests-lifecycle-22`, `tests-lifecycle-24`, `tests-lifecycle-27`, `tests-lifecycle-30`, `tests-lifecycle-32`.
  Decisions: T52, T130, T131, T132.

- [ ] **Observation test helpers and explanations.**
  Share observer draining and version/event translation, remove vacuous or repeated assertions, and correct stale callback, delivery and checkpoint descriptions.
  Findings: `tests-observation-1`, `tests-observation-2`, `tests-observation-4`, `tests-observation-6`, `tests-observation-12`, `tests-observation-15`, `tests-observation-16`, `tests-observation-18`, `tests-observation-19`, `tests-observation-21`, `tests-observation-27`, `tests-observation-29`, `tests-observation-31`, `tests-observation-32`, `tests-observation-33`, `tests-observation-34`.
  Decisions: T48, T49, T50, T52, T130, T131, T132.


## 18. Disruption and handshake tests

Prerequisites: 04–05 and 16.

Replace ineffective or overlapping tests with cases that reach the behavior in their claim.

- [ ] **Keep party-conservation fault coverage in the in-process harness.**
  Confirm the inter-process deletion and the meaningful vanish replacement. The reported alias-overlap failure was caused by reading different instants; do not recreate that oracle. Preserve retained seeds and explicit counterexamples.
  Findings: `suite-economics-10`, `tests-disruption-handshake-2`, `tests-disruption-handshake-3`, `tests-disruption-handshake-4`, `tests-disruption-handshake-7`, `tests-disruption-handshake-8`.
  Decisions: T132, T143, T149, T154.
  Later reports: N15.
  Prior ledger receipts to reconcile: `37bcab3f`.

- [ ] **Exercise driver termination, cancellation, redaction, and handshake ordering without vacuous checks.**
  Replace negative assertions that merely wait for a timer. Use derived cut ranges, verify staged-preamble terminal arms and local content, include default-window descent over a one-byte link. Distinguish the accepted driver-cancellation family from T19’s declined exhaustive handoff sweep.
  Findings: `session-bookmark-18`, `session-bookmark-42`, `testing-infra-4`, `testing-infra-21`, `testing-infra-22`, `tests-disruption-handshake-14`, `tests-disruption-handshake-15`, `tests-disruption-handshake-16`, `tests-disruption-handshake-17`, `tests-disruption-handshake-18`, `tests-disruption-handshake-21`, `tests-disruption-handshake-22`, `tests-disruption-handshake-23`, `tests-disruption-handshake-25`, `tests-disruption-handshake-26`.
  Decisions: T19, T28, T129, T130, T132.
  Prior ledger receipts to reconcile: `480fda21`, `56df8484`.

- [ ] **Disruption and handshake fixtures and comments.**
  Share handshake/hop fixtures and session helpers, remove obsolete protocol-column structure, and rewrite cut/latency comments from actual behavior.
  Findings: `tests-disruption-handshake-1`, `tests-disruption-handshake-5`, `tests-disruption-handshake-6`, `tests-disruption-handshake-9`, `tests-disruption-handshake-11`, `tests-disruption-handshake-12`, `tests-disruption-handshake-13`, `tests-disruption-handshake-19`, `tests-disruption-handshake-20`, `tests-disruption-handshake-24`, `tests-disruption-handshake-27`, `tests-disruption-handshake-29`, `tests-disruption-handshake-30`, `tests-disruption-handshake-31`, `tests-disruption-handshake-32`.
  Decisions: T48, T50, T52, T130, T131, T132.


## 19. Bookmark behavioral tests

Prerequisites: 08 and 16; 15 for schedule generators.

Keep the durability oracle independent enough to detect destruction of live durable messages.

- [ ] **Reconcile the completed causality repairs and cover storage failure paths.**
  Confirm deterministic identity draws, fault-free session errors, bounded progress and the corrected recycle oracle. Remove any stale party-alias comparison. Use the shared attach driver.
  Findings: `tests-bookmark-8`, `tests-bookmark-9`, `tests-bookmark-11`, `tests-bookmark-12`.
  Decisions: T8, T26, T149, T150.
  Prior ledger receipts to reconcile: `0b878c20`, `32cc9ae6`, `36b90a78`, `df382d09`.

- [ ] **Exercise durability and checkpoint guarantees across the full schedule.**
  Reach late-life faults, no-op/helper redactions and both link directions. Check a durable message actually survives; require the precise load/write outcomes the contract promises. Keep corruption sweeps meaningful after the owned-byte API.
  Findings: `session-bookmark-31`, `tests-bookmark-4`, `tests-bookmark-7`, `tests-bookmark-15`, `tests-bookmark-19`, `tests-bookmark-21`, `tests-bookmark-24`, `tests-bookmark-26`.
  Decisions: T130, T132.

- [ ] **Simplify bookmark healing and clarify the tests’ names.**
  Remove the redundant confirming mesh round after proving the existing convergence check suffices. Name the durability/checkpoint suites so they do not collide with gossip_when or protocol-window concepts; preserve their seeds when moving files.
  Findings: `tests-bookmark-13`, `tests-bookmark-23`.
  Decisions: T132.

- [ ] **Bookmark test helpers, names, and comments.**
  Consolidate Scene/model/instrument helpers and repeated counting assertions; correct durability descriptions and unnecessary healing work.
  Findings: `tests-bookmark-1`, `tests-bookmark-2`, `tests-bookmark-3`, `tests-bookmark-5`, `tests-bookmark-10`, `tests-bookmark-14`, `tests-bookmark-16`, `tests-bookmark-17`, `tests-bookmark-18`, `tests-bookmark-20`, `tests-bookmark-22`, `tests-bookmark-25`.
  Decisions: T48, T49, T131, T132.


## 20. Resource, wire-format, and public-surface tests

Prerequisites: 03, 05–09, 13 and 16 as each case requires.

Keep tests of library behavior and useful public type constraints; remove tautological pins.

- [ ] **Make resource and transport tests establish progress and convergence.**
  Respect conformance timeout responsibilities, make absence claims discriminate the intended event, use full state rather than len equality, and retain reconciled handles from hop measurements. Generalize actual Changes/routed/handshake behavior where the claim is a family.
  Findings: `tests-resource-link-window-10`, `tests-resource-link-window-12`, `tests-resource-link-window-24`, `tests-resource-link-window-26`, `tests-resource-link-window-27`, `verification-infra-17`.
  Decisions: T98, T132.

- [ ] **Strengthen wire examples and errors without accidental snapshot changes.**
  Check both target-size orientations, actual convergence, the intended decode error and payload-depth boundaries. Exercise all public async/observer bounds. Use known malformed capture cases; snapshot changes still require deliberate protocol/renderer attribution.
  Findings: `remote-capture-atlas-20`, `tests-wire-format-4`, `tests-wire-format-5`, `tests-wire-format-12`, `tests-wire-format-14`, `tests-wire-format-18`, `tests-wire-format-22`, `tests-wire-format-27`.
  Decisions: T5, T85, T93, T130, T132.

- [ ] **Simplify capture rendering and keep one vocabulary definition.**
  Verify the cbor-diag delegation; do not build an inverse parser or recreate deleted renderer code. Consolidate signal/detail mappings and error formatting, keeping useful diagnostic meaning without tests of prose style or duplicated rosters.
  Findings: `remote-capture-atlas-8`, `remote-capture-atlas-12`, `remote-capture-atlas-13`, `remote-capture-atlas-14`, `remote-capture-atlas-15`, `remote-capture-atlas-16`, `remote-capture-atlas-17`, `remote-capture-atlas-19`, `remote-capture-atlas-23`, `remote-capture-atlas-27`, `remote-capture-atlas-29`, `remote-capture-atlas-31`, `remote-capture-atlas-32`, `remote-capture-atlas-33`, `remote-capture-atlas-34`, `remote-capture-atlas-35`.
  Decisions: T9, T55, T132, T138, T140.
  Prior ledger receipts to reconcile: `e1ceb38d`.

- [ ] **Keep seed and snapshot discovery accurate without deleting counterexamples.**
  Use tempfile for provenance fixtures. Make discovery cover the actual asserting files and seed parameters. T59’s old deletion instructions yield to the current never-strip-seeds rule; preserve/rehome files and recover point counterexamples when strategies change.
  Findings: `streaming-tests-20`, `tests-common-31`, `tests-common-32`, `tests-lifecycle-1`, `tests-observation-38`, `tests-wire-format-19`, `tests-wire-format-20`, `tests-wire-format-21`.
  Decisions: T59, T91, T132.

- [ ] **Confirm the public future-size check runs under its intended profile.**
  Findings: `suite-economics-1`, `tests-wire-format-25`, `tests-wire-format-26`, `verification-infra-1`.
  Decisions: T7, T26.
  Prior ledger receipts to reconcile: `713d5854`.

- [ ] **Capture and atlas helper cleanup.**
  Simplify capture helpers and field vocabulary; correct rendered-format and fixture descriptions against the retained cbor-diag renderer.
  Findings: `remote-capture-atlas-1`, `remote-capture-atlas-2`, `remote-capture-atlas-3`, `remote-capture-atlas-4`, `remote-capture-atlas-5`, `remote-capture-atlas-6`, `remote-capture-atlas-7`, `remote-capture-atlas-9`, `remote-capture-atlas-10`, `remote-capture-atlas-11`, `remote-capture-atlas-18`, `remote-capture-atlas-21`, `remote-capture-atlas-22`, `remote-capture-atlas-25`, `remote-capture-atlas-26`, `remote-capture-atlas-30`.
  Decisions: T49, T50, T52, T126, T132.

- [ ] **Resource and transport test helpers and comments.**
  Consolidate pair builders, capture parsing, budget constants and repeated bootstrap helpers. Explain target size, capacity and observed state precisely.
  Findings: `tests-resource-link-window-1`, `tests-resource-link-window-2`, `tests-resource-link-window-3`, `tests-resource-link-window-4`, `tests-resource-link-window-6`, `tests-resource-link-window-9`, `tests-resource-link-window-11`, `tests-resource-link-window-13`, `tests-resource-link-window-14`, `tests-resource-link-window-15`, `tests-resource-link-window-17`, `tests-resource-link-window-21`, `tests-resource-link-window-22`, `tests-resource-link-window-23`, `tests-resource-link-window-29`.
  Decisions: T48, T50, T52, T129, T131, T132.

- [ ] **Wire test helpers and explanations.**
  Share snapshot/CBOR fixture and observer helpers; correct preamble and error descriptions without changing protocol snapshots as incidental cleanup.
  Findings: `tests-wire-format-1`, `tests-wire-format-2`, `tests-wire-format-3`, `tests-wire-format-6`, `tests-wire-format-8`, `tests-wire-format-9`, `tests-wire-format-10`, `tests-wire-format-11`, `tests-wire-format-13`, `tests-wire-format-15`, `tests-wire-format-16`, `tests-wire-format-17`, `tests-wire-format-23`, `tests-wire-format-24`, `tests-wire-format-28`.
  Decisions: T48, T50, T52, T93, T131, T132.


## 21. Benchmark cost and measured performance

Prerequisites: 10–12 for affected implementation baselines; 16 for shared support.

Investigate costs with existing benchmarks before accepting a new optimization or instrument.

- [ ] **Measure useful operations without fixture, runtime, or destructor work in the timed body.**
  Use shipped hashing, return byte throughput from session stats, remove redundant version harvesting, and smoke every bench. Recheck whether four parents suffice for capacity coverage; use a checked geometry hint. Keep target-specific capacity explanations accurate.
  Findings: `benches-envelope-1`, `benches-envelope-2`, `benches-envelope-3`, `benches-envelope-11`, `benches-envelope-12`, `benches-envelope-13`, `benches-envelope-15`, `benches-envelope-16`, `benches-envelope-18`, `benches-envelope-20`, `benches-envelope-23`, `benches-envelope-25`, `suite-economics-2`, `suite-economics-3`, `suite-economics-7`.
  Decisions: T89, T109, T112, T114, T129, T132.

- [ ] **Locate the linear term in a session with one difference.**
  Profile at two shared-set sizes in release, identify the traversal/allocation responsible, then fix it or bring the measured cost back for a contract decision. Do not silently accept an unexamined scaling regression.
  Later reports: N52.

- [ ] **Use measured test-profile improvements, without blindly optimizing the whole crate.**
  Reconcile the SHA3/keccak dev-profile optimization and assess remaining send/encoding cost only through measurement. Keep a focused change only if it improves total iteration cost, including compilation.
  Decisions: T123, T169.
  Later reports: N53.

- [ ] **Bench fixtures, helpers, and explanations.**
  Remove duplicated fixture setup, misleading timing/cost descriptions, stale CLI guidance, and hand-maintained counts.
  Findings: `benches-envelope-4`, `benches-envelope-5`, `benches-envelope-7`, `benches-envelope-8`, `benches-envelope-9`, `benches-envelope-10`, `benches-envelope-14`, `benches-envelope-17`, `benches-envelope-21`, `benches-envelope-24`, `benches-envelope-26`, `benches-envelope-27`, `benches-envelope-30`.
  Decisions: T50, T129, T132.

- [ ] **Test-cost explanations and redundant execution.**
  Remove drifted timing and timeout claims. Explain current test-cost choices without copying machine-specific measurements into comments.
  Findings: `suite-economics-6`, `suite-economics-11`.
  Decisions: T53, T132.


## 22. Verification recipes and dependencies

Prerequisites: API lints only after their affected public surfaces are clean; broad settings after 15.

Use existing compiler/build checks. Do not expand bespoke prose or source-spelling enforcement.

- [ ] **Make gate and CI run the intended checks reproducibly.**
  Reconcile existing recipe/testdoc repairs, pass --locked to relevant cargo commands, avoid untracked build trees in source discovery, and verify default-feature as well as docsrs documentation. Keep normal build checks rather than adding a linter to inspect these recipe strings.
  Findings: `remote-proxy-tests-27`, `tests-disruption-handshake-33`, `tests-observation-37`, `verification-infra-2`, `verification-infra-7`, `verification-infra-8`, `verification-infra-9`, `verification-infra-10`, `verification-infra-12`, `verification-infra-13`.
  Decisions: T15, T20, T26, T28, T30, T155.
  Later reports: N02, N08, N40.
  Prior ledger receipts to reconcile: `459f1ac3`, `ba120979`, `d67c5a1a`, `f1d7773c`.

- [ ] **Enable the approved compiler lints when their real code fixes are complete.**
  Apply idiomatic simplifications and useful Debug/public-type fixes. Add each normal Rust/Clippy lint at zero remainder. Keep pub-in-private where appropriate; do not equate visibility spelling with public API or create a new documentation-style checker.
  Findings: `clippy-pedantic-1`, `clippy-pedantic-2`, `clippy-pedantic-3`, `clippy-pedantic-4`, `clippy-pedantic-5`, `clippy-pedantic-6`, `clippy-pedantic-7`, `clippy-pedantic-8`, `clippy-pedantic-9`, `clippy-pedantic-10`, `clippy-pedantic-11`, `clippy-pedantic-12`, `clippy-pedantic-13`, `clippy-pedantic-14`, `clippy-pedantic-15`.
  Decisions: T46, T47, T52, T54, T57, T132, T158.

- [ ] **Prune unused dependencies and confirm feature combinations still build.**
  Check runtime versus dev-only placement, including bytes/serde and static_assertions. Remove pollster only when its last needed consumer is gone; retain required feature combinations. Do not remove regression seeds with obsolete tools.
  Findings: `deps-1`, `deps-2`, `deps-3`, `deps-4`, `deps-5`, `deps-7`, `deps-8`, `deps-9`, `deps-10`, `deps-11`, `inventory-2`.
  Decisions: T20, T26, T28, T64, T90, T128, T132, T135, T147, T159.
  Prior ledger receipts to reconcile: `038ab91d`, `1af1d346`, `920d8ae5`, `9f19d098`, `d67c5a1a`.

- [ ] **Verification configuration and explanations.**
  Remove dead renderer/tool references and outdated configuration descriptions; keep recipe explanations aligned with checks that actually run.
  Findings: `verification-infra-11`, `verification-infra-15`, `verification-infra-18`.
  Decisions: T5, T132.
  Prior ledger receipts to reconcile: `fafc31b1`.


## 23. Public documentation and explanation placement

Prerequisites: Contracts and API signatures from earlier batches; prose improves in every batch.

Finish the cross-cutting public narrative after the implementation it describes has settled.

- [ ] **Explain the library model and API at the reader’s level.**
  Check membership/custody, universe safety, ceiling/floor, session, cancellation and bootstrap/retire claims against code. Public docs state contracts and user-relevant costs; private docs explain invariants and implementation reasoning. Use the current authenticated-honest-peer model; T95’s adversary rationale is superseded. Regenerate READMEs from rustdoc.
  Findings: `api-core-17`, `api-core-18`, `api-core-26`, `api-core-27`, `prose-hygiene-2`, `session-bookmark-34`, `session-bookmark-44`, `tree-core-6`.
  Decisions: T46, T51, T58, T75, T82, T95, T132, T141, T158.
  Later reports: N09.

- [ ] **Put sizing and reconciliation explanations where they belong.**
  Move the sizing guide to a docs-only explanation module, use the codec’s stream count, remove protocol-selection language for a one-protocol exchange, and keep CBOR evolution tests within the existing promise. Rewrite Link’s public contract against the retained pooling and departure behavior.
  Findings: `mirror-common-12`, `module-graph-4`.
  Decisions: T56, T82, T87, T92, T93, T100, T102, T103, T104.

- [ ] **Public API documentation and reachability.**
  Clarify which types callers can name, remove duplicate API descriptions, and document errors and costs at the public boundary.
  Findings: `api-audit-5`, `api-audit-6`, `api-audit-8`, `api-audit-9`, `api-audit-10`, `api-audit-16`, `api-audit-17`, `api-audit-18`.
  Decisions: T55, T82, T92, T132.

- [ ] **Crate orientation and public examples.**
  Repair examples, crate-level navigation, and misleading public descriptions against the retained API and trust model.
  Findings: `fresh-eyes-1`, `fresh-eyes-2`, `fresh-eyes-3`, `fresh-eyes-5`, `fresh-eyes-6`, `fresh-eyes-7`, `fresh-eyes-11`.
  Decisions: T49, T82, T92, T104, T132.

- [ ] **Cross-cutting prose accuracy and vocabulary.**
  Rewrite inaccurate or overwrought passages for their readers, remove invented vocabulary and stale names, and align method versus type/module introductions. Review this by reading, not a wording checker.
  Findings: `prose-hygiene-1`, `prose-hygiene-3`, `prose-hygiene-4`, `prose-hygiene-5`, `prose-hygiene-6`, `prose-hygiene-7`, `prose-hygiene-8`, `prose-hygiene-9`, `prose-hygiene-10`, `prose-hygiene-11`, `prose-hygiene-12`.
  Decisions: T5, T48, T49, T125, T130, T132.
  Prior ledger receipts to reconcile: `fafc31b1`.


## 24. Module layout and remaining local cleanup

Prerequisites: Affected behavioral/API work first; broad import reflow and path moves last.

The component items below collect the remaining documentation and small implementation findings. They are scope lists, not permission for giant diffs.

- [ ] **Simplify module boundaries, then flatten the extra streaming path level.**
  Move tests and the ruled inline production modules to sibling files. Use the existing module analysis once if useful; do not commit another analyzer. Flatten streaming into mirror after overlapping fixes. Update callers, docs and seed paths together.
  Findings: `module-graph-2`, `module-graph-6`, `module-graph-14`, `streaming-tests-2`.
  Decisions: T53, T86, T121, T132, T158, T159.

- [ ] **Apply import grouping after overlapping changes have settled.**
  Use the pinned nightly formatting option and review the Rumors reflow separately. Coordinate any crates/ reflow with current before work rather than rebasing all fixes across it.
  Decisions: T52, T158.

- [ ] **Repeated constants, representations, and stale comments.**
  Replace copied facts with their owning definitions where useful; remove redundant representations and stale claims. Keep private-module visibility changes out unless they improve an actual boundary.
  Findings: `inventory-3`, `inventory-4`, `inventory-6`, `inventory-7`, `inventory-8`, `inventory-9`, `inventory-10`, `inventory-11`, `inventory-12`, `inventory-13`, `inventory-14`, `inventory-15`, `inventory-16`, `inventory-17`, `inventory-19`, `inventory-20`.
  Decisions: T46, T52, T54, T57, T64, T125, T132.

- [ ] **Module boundaries and navigation.**
  Remove misleading module/re-export distinctions, relocate misplaced helpers and explanations, and keep the navigation tied to current responsibilities.
  Findings: `module-graph-1`, `module-graph-3`, `module-graph-5`, `module-graph-7`, `module-graph-8`, `module-graph-9`, `module-graph-10`, `module-graph-11`, `module-graph-12`, `module-graph-13`, `module-graph-15`, `module-graph-16`.
  Decisions: T46, T52, T121, T125, T132.


## 25. Publication preparation

Prerequisites: Accepted implementation and documentation work complete.

Prepare a small design note before changing packaging. No publishing is authorized.

- [ ] **Prepare workspace licensing and package metadata for later publication.**
  Apply the ruled MPL-2.0 licensing and source headers, keep every package publish=false, and verify packaging through the appropriate local dry runs. Bring the concrete crate-description sentences to the user for review. Do not set publishable flags or publish packages.
  Findings: `deps-12`.
  Decisions: T79.


## 26. Historical, declined, deferred, and external work

Prerequisites: No prerequisite for recording these decisions.

These entries prevent old proposals from silently returning as new implementation requirements.

- [ ] **Confirm the swarm example and memwatch are gone, including obsolete dependencies and references.**
  The ledger’s swarm SHA 2cb24d45 is not an ancestor of main; the actual deletion is 6e5eb555. Verify the current tree and remaining dependency/prose references rather than trusting the old receipt.
  Findings: `suite-economics-9`, `swarm-example-1`, `swarm-example-2`, `swarm-example-3`, `swarm-example-4`, `swarm-example-5`, `swarm-example-6`, `swarm-example-7`, `swarm-example-8`, `swarm-example-9`, `swarm-example-10`, `swarm-example-11`, `swarm-example-12`, `swarm-example-13`, `swarm-example-14`, `swarm-example-15`, `swarm-example-16`, `swarm-example-17`, `swarm-example-18`, `swarm-example-19`, `swarm-example-20`, `swarm-example-21`, `swarm-example-22`, `swarm-example-23`, `swarm-example-24`, `swarm-example-25`, `swarm-example-26`, `swarm-example-27`, `swarm-example-28`, `swarm-example-29`, `swarm-example-30`, `swarm-example-31`.
  Decisions: T27, T132, T135, T136.
  Prior ledger receipts to reconcile: `2cb24d45`.

- [x] **Keep mutation and coverage scope at the recorded decisions.**
  No new Rumors mutation campaign and no new Rumors coverage threshold. Targeted tests or a one-off mutation used to validate an actual regression remain available.
  Findings: `verification-infra-3`, `verification-infra-4`.
  Decisions: T11, T12.

- [x] **Keep Rumors decoder fuzzing explicitly deferred.**
  T14 defers this to the existing rumors-frame-fuzz design. It remains deferred at the end of this plan unless the user reopens it; it is not an unimplemented accepted batch.
  Findings: `verification-infra-5`.
  Decisions: T14.

- [ ] **Retain the human-checked formal transcription without a generated comparison tool.**
  Verify the invariant and named Lean definition/theorem when touching the case. Code does not cite formal design/progress files or file paths.
  Findings: `streaming-tests-28`.
  Decisions: T29.
  Prior ledger receipts to reconcile: `5d51974f`.

- [ ] **Verify historical integration repairs and preserve the before counterexamples.**
  Recheck the repaired imports and fault-plan call at the retained main base. N30/N44 belong to before’s fuel-band work: preserve their tracked seed evidence, check whether the required gate still reproduces them, and resolve that actual blocker without importing all before triage.
  Decisions: T146.
  Later reports: N10, N30, N44.

- [x] **Replace the old coordination machinery with this checklist and editable branch review.**
  One active implementation, current main, one reviewable diff at a time. The user reviews and may edit in Zed before merge. No separate identity, PR/packet fleet, gate wait-queue tool, or process-wide fairness framework. N43/N47 arose from the retired multi-lane gate workflow; reconsider only if a real current problem recurs. Old raw logs were lost; record fresh verification with the current change instead of claiming they were re-read.
  Decisions: T1, T2, T3, T4, T25, T133, T134, T137.
  Later reports: N43, N47, N55.


## 27. Finish and reconcile

Prerequisites: All accepted outcomes above resolved and merged, or a new explicit disposition recorded.

A completed branch list is not the completion test.

- [ ] **Review the remaining code and prose against every unchecked item.**
  Confirm source coverage again, verify each outcome against the integrated code, record actual merge commits, and list every deferred/external decision. Check stale references, unnecessary instruments and comments, API documentation accuracy, seeds, and deliberate wire snapshots. No new checklist test or permanent tracker is needed.
  Decisions: T5, T141, T171.

## Coverage check

A one-off comparison matched the assigned IDs to the six source reports and ledger, with no missing or multiply assigned original findings. All later-report rows and all rulings have homes. No checker, gate, or test was added for this document. Re-run an independent comparison when closing the effort; do not infer coverage from the number of completed branches.
