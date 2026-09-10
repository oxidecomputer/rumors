# Rumors review checklist

**Active:** 01/07, retirement ownership — `codex/retirement-ownership`, base `main`.
**Next:** 07, usable bootstrap retry outcomes; then 10, action and join cleanup.

Check code outcomes only after verification and merge; retain the landing
commit. Checked dispositions are labelled explicitly.
[Workflow and tracking rules](README.md#3-tracking-without-another-system).

Sources: [report IDs](../2026-09-01-holistic-review-rumors/README.md),
[T rulings and amendments](../2026-09-01-holistic-review-rumors/triage/rulings.md),
[N later-report rows](../2026-09-01-holistic-review-rumors/triage/new-findings.md)
(N IDs number rows from the first data row). Ranges are inclusive. Read the current
ruling before implementing; [old ledger receipts](../2026-09-01-holistic-review-rumors/triage/ledger.tsv)
and [prepared branches](inventory.md) are leads, not completion evidence.
Dependencies apply only to the affected work within a group.

## 01. Commit ownership and publication

Coupled work: optimistic gossip publication must preserve notification behavior
and recheck retained-root accounting.

- [x] Release displaced roots and retained payloads after every replica guard is gone — `2c77220a`.
  Sources: `async-hazards-3`, T34.

- [x] Move the gossip join out of the replica write lock — `e238db87`.
  Sources: T170.

- [x] Notify observers for tree changes, without party-only wakeups — `06f96386`.
  Sources: T35.

- [x] Preserve no-op trees and memos — `915571b7`.
  Sources: `tree-core-29`, T107, T166. Coupled with 10's tree edits.

- [ ] Eliminate the absent-party batch path. **Working.**
  Sources: `api-core-2`, T37. Coupled with 07's retirement ownership.

- [x] Reconcile the window census with the roots the commit actually retains — `e238db87`.
  Sources: T139, T171, N54.

## 02. Routed connection pooling

- [x] Keep idle connections with their owning link; preserve fairness and progress across routing and reuse — `45aaec54`.
  Sources: `link-14`, `link-28`, T31, T152, T153, T156, T160, T164, T167, N06, N07, N24, N25, N48, N49; [Sush compatibility](sush-pooling.md).

- [x] Identify and fix Sush's concurrent-connect conformance timeout — Sush `5ea47e0`.
  Source: Owner follow-up; [reproduction context](sush-pooling.md).

## 03. Deep-tree fixtures and reproducible schedules

Dependencies: None to construct fixtures; 04 and 05 before enabling the complete CI run.

- [x] Exercise deep wire reconciliation across branching depths, nested and asymmetric trees, and transport and window variations — `a16bcfb8`.
  Sources: `remote-proxy-tests-6`, T23, T132, T162–164, N27–29.

- [ ] Run the broader behavioral suites under deep geometry without changing production hashing or wire snapshots.
  Sources: `remote-capture-atlas-28`, `remote-proxy-tests-10`, `tests-wire-format-7`, T23, T123, T132, T162–164, N27–29.

- [x] Make cancellation tests depend on a reproducible session schedule — `f643bd60`.
  Sources: `streaming-tests-3`, `streaming-tests-6`, `streaming-tests-7`, T19, T127, T132, T162, N19, N42.

## 04. Departure, malformed replies, and error preservation

Dependencies: 03 for deep reproductions; coordinate all edits to the shared driver.

- [x] End waits when the peer can no longer provide an owed stream — `64964725`.
  Sources: T145, T154, T165, T166, N01, N21.

- [x] Preserve reported violations when a concurrent stream-supply failure also arrives — `f92fcbc3`.
  Sources: `remote-proxy-12`, `remote-proxy-tests-26`, T126, T132, T165, N32.

- [ ] Surface violations stranded in proxy and materialized response relays.
  Sources: T132, T165, N18, N33.

- [ ] Reproduce and fix the duplicated-reply stall at deep disputes.
  Sources: T162, N26, N31.

## 05. Pipelining and conformance claims under deep geometry

Dependencies: 03–04 for deep geometry; 02 for the pooled transport case.

- [ ] Determine and repair the depth-dependent pipelining behavior.
  Sources: `tests-disruption-handshake-10`, `tests-resource-link-window-25`, `verification-infra-16`, T28, T98, T132, T159, T162, N41.

- [ ] Make conformance memory and liveness checks test their stated premises.
  Sources: `conformance-24`, `conformance-25`, `conformance-28–31`, `conformance-35`, `conformance-40–42`, `tests-resource-link-window-19`, `tests-resource-link-window-20`, T6, T18, T132, T139, T142, N22, N34.

- [x] Extend link conformance coverage for combined contention, overlapping opens, completion, and cancellation; state the remaining test limits — `45aaec54`.
  Source: [Link contract](../../src/link.rs) and [public suite](../../src/conformance/link.rs).

## 06. Window arithmetic and operating costs

Dependencies: 01 for retained-root measurements; 03–05 when claims cover deep paths.

- [ ] Check the shipped window formulas against an independent numerical calculation; remove the simulator.
  Sources: `benches-envelope-28`, `benches-envelope-29`, `benches-envelope-31–34`, `streaming-backend-window-32`, T10, T43.

- [ ] Derive window charges and structural limits from the types or constants that own them.
  Sources: `link-3`, `materialized-10`, `remote-codec-3`, `streaming-backend-window-25`, `streaming-backend-window-26`, `streaming-backend-window-28`, `streaming-backend-window-30`, `streaming-backend-window-31`, `streaming-backend-window-34`, `testing-infra-2`, T86–88, T103, T104, T120, T129, T132.

- [ ] Support public cost claims with a sparse, affordable overhead grid.
  Sources: `verification-infra-6`, T17, T168, N35, N51.

- [ ] Keep only useful budget and residency measurements, with precise claims.
  Sources: `materialized-30`, `streaming-backend-window-9`, `streaming-backend-window-11`, `streaming-backend-window-37`, `streaming-tests-16`, `tests-resource-link-window-7`, `tests-resource-link-window-8`, `tests-resource-link-window-16`, `tests-resource-link-window-18`, `tests-resource-link-window-28`, `verification-infra-14`, T16, T28, T110, T113, T128, T130, T132.

## 07. Peer lifecycle, observers, and snapshots

Coupled work: party ownership with 01; other API pieces can be separate.

- [ ] Represent retirement ownership directly and return usable builders from failed joins.
  Sources: `api-core-25`, `async-hazards-2`, `fresh-eyes-9`, T37, T70, T77, T125, T128.

- [ ] Return the stamped Version from send and simplify version lookup at callers.
  Sources: `tests-common-2`, `tests-lifecycle-31`, T67.

- [ ] Provide the intended Snapshot read surface and equality contract.
  Sources: `api-audit-2`, `api-core-34–36`, `benches-envelope-19`, `fresh-eyes-4`, `inventory-1`, `tree-core-5`, T47, T60, T61, T65, T97, T132.

- [ ] Accumulate observers through one shared channel representation.
  Sources: `api-core-19`, `api-core-33`, `session-bookmark-40`, T74, T125, T132, T159.

- [ ] Keep causal delivery tests within the public ordering contract.
  Sources: `session-bookmark-37`, `tests-observation-3`, T78, T132.

- [ ] Make public bounds, borrowing, and wrapper traits express the real requirements.
  Sources: `api-audit-1`, `api-audit-3`, `api-audit-7`, `api-audit-12`, `api-core-1`, `api-core-5`, `api-core-22`, `api-core-30`, `deps-6`, `tree-core-2`, T47, T72, T73, T81, T83, T84, T125.

- [ ] Simplify core API plumbing and correct its comments.
  Sources: `api-core-3`, `api-core-4`, `api-core-6`, `api-core-8`, `api-core-9`, `api-core-13–16`, `api-core-21`, `api-core-23`, `api-core-24`, `api-core-28`, `api-core-31`, `api-core-32`, `api-core-37`, T52, T72, T82, T92, T132.

## 08. Bookmark contracts and file implementation

Dependencies: 07 for join outcomes; owned-byte trait before conformance and file storage.

- [ ] Make Bookmark own its error type and the bytes it stores.
  Sources: `api-audit-4`, `async-hazards-1`, `session-bookmark-23–25`, `session-bookmark-29`, `session-bookmark-30`, T62, T132.

- [ ] State and test when a checkpoint makes older bookmark state reclaimable.
  Sources: `session-bookmark-21`, T32.

- [ ] Ship a focused Bookmark conformance suite.
  Sources: `conformance-1`, `tests-bookmark-6`, T69, T130.

- [ ] Add a minimal atomic file-backed Bookmark behind a feature.
  Sources: `fresh-eyes-8`, T94, T159.

- [ ] Simplify session and bookmark plumbing and explanations.
  Sources: `session-bookmark-1–7`, `session-bookmark-9–17`, `session-bookmark-19`, `session-bookmark-22`, `session-bookmark-26–28`, `session-bookmark-32`, `session-bookmark-33`, `session-bookmark-35`, `session-bookmark-36`, `session-bookmark-39`, `session-bookmark-41`, `session-bookmark-43`, `session-bookmark-45`, `session-bookmark-47`, T46, T48–50, T52, T56, T104, T129, T132.

## 09. Public diagnostics and observability

Dependencies: 04's reported-error attribution fix before the public error redesign; 02 before router-event counters.

- [x] Collapse public session failures to actionable causes and one protocol-violation diagnostic; keep operation outcomes separate — `faa518a5`.
  Sources: `api-audit-13`, `api-audit-14`, `api-core-7`, `fresh-eyes-10`, `mirror-common-15`, `remote-proxy-2`, `remote-proxy-3`, T46, T63, T82, T85; owner direction, 2026-09-10.

- [ ] Finish internal diagnostic cleanup: remove impossible cases, refine decoder context, and apply the error-enum conventions.
  Sources: `materialized-17`, `mirror-common-10`, `remote-adapter-streams-19`, `remote-adapter-streams-22`, `remote-codec-9`, `remote-codec-18`, `remote-codec-19`, `remote-codec-28`, `remote-codec-30`, `session-bookmark-46`, T55, T63, T85.

- [ ] Expose useful session outcomes, settings, and event counts.
  Sources: `api-audit-11`, `link-16`, `link-21`, `materialized-2`, `remote-adapter-streams-21`, `remote-codec-5`, `session-bookmark-20`, `session-bookmark-38`, T66, T68.

- [ ] Gate test-only controls and provide small testing exports where they remove copied derivations.
  Sources: `api-audit-15`, `api-core-11`, `api-core-12`, `benches-envelope-6`, `materialized-22`, `tests-common-6`, `tests-lifecycle-10`, T64, T85, T96.

## 10. Tree edits, joins, and memo ownership

Coupled work: no-op handling with 01. Do root-version changes before dependent optimization measurements.

- [x] Store each leaf’s action version and traverse sorted actions without repeated sorting or copying — `915571b7`.
  Sources: `tree-core-11`, `tree-core-13`, `tree-core-16`, `tree-core-27`, `tree-core-30`, T38, T39, T107, T124, T132, N12.

- [ ] Simplify action representations and join reassembly; cover their remaining regressions.
  Sources: `tree-core-14`, `tree-core-31`, N23.

- [ ] Publish roots with their required memos already warm.
  Sources: `api-core-29`, `async-hazards-4`, `tree-core-8`, T42, T96, T132.

- [ ] Remove message-buffer slack with a direct size invariant.
  Sources: `api-core-10`, T110, T113.

- [ ] Protect tree algebra, deletion, and traversal invariants with meaningful cases.
  Sources: `streaming-tests-18`, `tree-core-22`, `tree-core-24`, `tree-core-33`, `tree-core-34`, `tree-typed-30`, T39, T112, T124, T128, T132.

- [ ] Clarify tree invariants and simplify names and helpers.
  Sources: `tree-core-1`, `tree-core-3`, `tree-core-4`, `tree-core-9`, `tree-core-10`, `tree-core-12`, `tree-core-15`, `tree-core-17–21`, `tree-core-23`, `tree-core-25`, `tree-core-26`, `tree-core-28`, `tree-core-32`, `tree-core-35`, T48, T49, T52, T53, T124, T132.

## 11. Typed-tree representation and measured changes

Dependencies: 10 when a change affects the same traversal or baseline.

- [ ] Remove the leaf-hash heap buffer; measure the remaining representation tradeoffs.
  Sources: `tree-core-7`, `tree-typed-6`, `tree-typed-23`, T111, T132.

- [ ] Derive height/radix structure and simplify owned traversal states.
  Sources: `tree-typed-11`, `tree-typed-12`, `tree-typed-16`, `tree-typed-17`, `tree-typed-20`, `tree-typed-21`, `tree-typed-24`, `tree-typed-31`, `tree-typed-34`, `tree-typed-35`, T36, T125, T132.

- [ ] Check typed-tree preconditions where a real failure construction is possible.
  Sources: `tree-typed-7`, `tree-typed-9`, T132.

- [ ] Simplify typed-tree helpers and clarify their contracts.
  Sources: `tree-typed-1–5`, `tree-typed-8`, `tree-typed-10`, `tree-typed-13–15`, `tree-typed-18`, `tree-typed-19`, `tree-typed-22`, `tree-typed-25–29`, `tree-typed-32`, `tree-typed-33`, T46, T49, T52, T95, T97, T125, T132.

## 12. Wire codec, greeting, and stream adapters

Dependencies: 04 and 09 before simplifying shared failure paths.

- [ ] Keep sync/async decoding consistent, including partial reads and boundary failures.
  Sources: `mirror-common-5`, `mirror-common-8`, `remote-codec-6`, `remote-codec-8`, `remote-codec-10`, `remote-codec-11`, `remote-codec-14–17`, `remote-proxy-tests-25`, T24, T33, T41, T126, T132.

- [ ] Parse the version head directly and simplify the fixed greeting vocabulary.
  Sources: `inventory-5`, `mirror-common-3`, `mirror-common-14`, `remote-codec-24`, `remote-codec-26`, `remote-codec-27`, `remote-proxy-tests-2`, T105, T126, T132.

- [ ] Demonstrate decode-channel progress below a fan and remove per-reply overhead only if measured.
  Sources: `remote-adapter-streams-6`, `remote-proxy-27`, T106.

- [ ] Simplify stream state machines and repeated encoder/decoder plumbing.
  Sources: `async-hazards-6`, `mirror-common-7`, `mirror-common-17`, `mirror-common-24`, `mirror-common-25`, `mirror-common-32`, `remote-adapter-streams-2–5`, `remote-adapter-streams-9`, `remote-adapter-streams-10`, `remote-adapter-streams-12`, `remote-adapter-streams-14`, `remote-adapter-streams-16`, `remote-adapter-streams-20`, `remote-adapter-streams-24–30`, T52, T119, T126, T132.

- [ ] Test adapter failures at the actual conversion and ordering boundaries.
  Sources: `remote-adapter-tests-4`, `remote-adapter-tests-6`, `remote-adapter-tests-7`, `remote-adapter-tests-9–15`, `remote-adapter-tests-17`, `remote-adapter-tests-19–22`, T126, T132.

- [ ] Avoid needless scratch initialization in listing reads.
  Sources: `remote-codec-12`, T132.

- [ ] Simplify shared wire types and clarify the protocol.
  Sources: `mirror-common-1`, `mirror-common-2`, `mirror-common-4`, `mirror-common-6`, `mirror-common-9`, `mirror-common-11`, `mirror-common-13`, `mirror-common-18–23`, `mirror-common-26–31`, `mirror-common-34–37`, T46, T49, T50, T52–54, T132.

- [ ] Clarify stream adapter ownership, names, and behavior.
  Sources: `remote-adapter-streams-1`, `remote-adapter-streams-7`, `remote-adapter-streams-8`, `remote-adapter-streams-11`, `remote-adapter-streams-13`, `remote-adapter-streams-15`, `remote-adapter-streams-17`, `remote-adapter-streams-18`, `remote-adapter-streams-23`, T48, T52, T57, T132.

- [ ] Consolidate adapter test setup and correct its comments.
  Sources: `remote-adapter-tests-1–3`, `remote-adapter-tests-5`, `remote-adapter-tests-8`, `remote-adapter-tests-16`, `remote-adapter-tests-18`, T48, T50, T52, T119, T132.

- [ ] Consolidate codec helpers and constants; clarify units and boundaries.
  Sources: `remote-codec-1`, `remote-codec-2`, `remote-codec-4`, `remote-codec-7`, `remote-codec-13`, `remote-codec-20–23`, `remote-codec-25`, `remote-codec-29`, `remote-codec-31–33`, T46, T50, T52, T99, T132.

## 13. Link contract and transport cleanup

Dependencies: 02, then relevant API/error changes in 09.

- [ ] Simplify link construction, validated headers, and configuration.
  Sources: `inventory-18`, `link-5`, `link-6`, `link-20`, `link-25`, `link-29`, T44, T45, T47, T80, T81, T84, T156.

- [ ] Make focused conformance probes exercise independent streams and cancellation without a backlog assumption.
  Sources: `conformance-6–9`, `conformance-11`, `conformance-13`, `conformance-16`, `conformance-18`, `conformance-20`, `conformance-27`, `conformance-37–39`, `link-11`, `link-12`, `link-31`, `testing-infra-12`, T21, T69, T71, T101, T122, T128, T132.

- [ ] Remove measured or obvious per-connection overhead without changing the transport contract (table lifetime: 02).
  Sources: `link-1`, `link-27`, T132.

- [ ] Simplify conformance fixtures and clarify what each probe establishes.
  Sources: `conformance-2–5`, `conformance-10`, `conformance-12`, `conformance-14`, `conformance-15`, `conformance-17`, `conformance-19`, `conformance-21–23`, `conformance-26`, `conformance-32–34`, `conformance-36`, T48–50, T132.

- [ ] Simplify link/router bookkeeping and clarify public contracts.
  Sources: `link-2`, `link-4`, `link-7–10`, `link-13`, `link-15`, `link-17–19`, `link-22–24`, `link-26`, `link-30`, T46, T49, T55, T80, T100, T128, T132.

## 14. Walk, materialized backend, and proxy simplification

Dependencies: 04, 06, 09 and the relevant codec edits in 12.

- [ ] Share classification and completion paths without hiding backend failures.
  Sources: `materialized-13`, `materialized-14`, `materialized-26`, `materialized-27`, `materialized-31`, `materialized-35`, `materialized-36`, `materialized-40`, `remote-proxy-23`, `remote-proxy-25`, `remote-proxy-28–30`, `streaming-backend-window-20`, `streaming-backend-window-23`, `streaming-backend-window-29`, T26, T40, T118, T128, T132, N04, N05.

- [ ] Collapse the handshake handoff and name its actual premises once.
  Sources: `async-hazards-5`, `materialized-11`, `materialized-34`, `mirror-common-16`, `remote-proxy-4`, `remote-proxy-7`, `remote-proxy-24`, `streaming-tests-24`, T99, T118, T119, T128, T132.

- [ ] Run the height-indexed trait experiment and judge the result.
  Sources: `mirror-common-33`, T117, T132.

- [ ] Exercise meaningful walk and proxy failures, shedding, isolation, and terminal behavior.
  Sources: `materialized-28`, `materialized-37`, `materialized-39`, `remote-proxy-19`, `remote-proxy-20`, `remote-proxy-31`, `remote-proxy-tests-5`, `remote-proxy-tests-7–9`, `remote-proxy-tests-12`, `remote-proxy-tests-15`, `remote-proxy-tests-18`, `remote-proxy-tests-20–24`, `streaming-backend-window-15`, `streaming-backend-window-16`, `streaming-backend-window-18`, `streaming-backend-window-36`, `streaming-backend-window-38`, `streaming-tests-11`, `streaming-tests-17`, `streaming-tests-19`, `streaming-tests-23`, `streaming-tests-26`, T22, T26, T101, T126, T128, T132, T159, N03.

- [ ] Simplify materialized-backend state and explain its ownership.
  Sources: `materialized-1`, `materialized-3–9`, `materialized-12`, `materialized-15`, `materialized-16`, `materialized-18–21`, `materialized-23–25`, `materialized-29`, `materialized-32`, `materialized-33`, `materialized-38`, T46, T48, T49, T52, T54, T128, T132.

- [ ] Simplify proxy state and clarify driver responsibilities.
  Sources: `remote-proxy-1`, `remote-proxy-5`, `remote-proxy-6`, `remote-proxy-8–11`, `remote-proxy-13–18`, `remote-proxy-21`, `remote-proxy-22`, `remote-proxy-26`, `remote-proxy-32`, T48, T49, T52, T57, T126, T132.

- [ ] Consolidate proxy test setup and remove redundant assertions.
  Sources: `remote-proxy-tests-1`, `remote-proxy-tests-3`, `remote-proxy-tests-4`, `remote-proxy-tests-11`, `remote-proxy-tests-13`, `remote-proxy-tests-14`, `remote-proxy-tests-16`, `remote-proxy-tests-17`, `remote-proxy-tests-19`, T48, T50, T52, T132.

- [ ] Simplify scheduling helpers and separate backend and window explanations.
  Sources: `streaming-backend-window-1–8`, `streaming-backend-window-10`, `streaming-backend-window-12–14`, `streaming-backend-window-17`, `streaming-backend-window-19`, `streaming-backend-window-21`, `streaming-backend-window-22`, `streaming-backend-window-24`, `streaming-backend-window-27`, `streaming-backend-window-33`, `streaming-backend-window-35`, T46, T48, T49, T53, T102, T103, T128, T132.

- [ ] Consolidate streaming fixtures and correct test explanations.
  Sources: `streaming-tests-1`, `streaming-tests-4`, `streaming-tests-5`, `streaming-tests-8–10`, `streaming-tests-12–15`, `streaming-tests-21`, `streaming-tests-22`, `streaming-tests-25`, `streaming-tests-27`, T48, T49, T52, T128, T132.

## 15. Generators and property-test runs

Dependencies: Scoped generator repairs can start independently; workspace settings wait for compatible before generators.

- [ ] Generate constrained cases directly while preserving the regressions they must cover.
  Sources: `remote-capture-atlas-24`, T132, T157, T161, N13, N17, N20, N36, N45, N46.

- [ ] Run meaningful high-count release properties with one case-count setting.
  Sources: `suite-economics-4`, T116, T129, T148, T151, T164, N14, N37, N38.

## 16. Shared test and benchmark support

Dependencies: Relevant lifecycle, Bookmark, link and observer signatures from 07–09 and 13.

- [ ] Compile the shared test and bench harness once through a path dev-dependency.
  Sources: `benches-envelope-22`, `suite-economics-8`, `tests-common-8`, `tests-disruption-handshake-28`, T115.

- [ ] Use one set of session drivers, observer readouts, fingerprints, and fault wrappers.
  Sources: `session-bookmark-8`, `testing-infra-8`, `testing-infra-9`, `testing-infra-11`, `testing-infra-13`, `testing-infra-17–20`, `testing-infra-23`, `tests-common-3`, `tests-common-4`, `tests-common-12`, `tests-common-14–16`, `tests-common-18–20`, `tests-common-22`, `tests-common-25`, `tests-common-29`, `tests-common-30`, T76, T129, T130, T132, T144, T150.

- [ ] Remove allocation-count nondeterminism and load-sensitive session deadlines.
  Sources: `tests-resource-link-window-5`, T132, N11, N39, N50.

- [ ] Simplify fault wrappers and clarify testing support behavior.
  Sources: `testing-infra-1`, `testing-infra-3`, `testing-infra-5–7`, `testing-infra-10`, `testing-infra-14–16`, T48, T49, T52, T53, T129, T132.

- [ ] Consolidate common test helpers and use public observers.
  Sources: `tests-common-1`, `tests-common-5`, `tests-common-7`, `tests-common-9–11`, `tests-common-13`, `tests-common-17`, `tests-common-21`, `tests-common-23`, `tests-common-24`, `tests-common-26–28`, T48, T50, T52, T56, T130, T132.

## 17. Lifecycle and observation tests

Dependencies: 07 and 16; 15 for generator changes.

- [ ] Make lifecycle properties check content, versions, and party ownership directly.
  Sources: `api-core-20`, `suite-economics-5`, `tests-lifecycle-4`, `tests-lifecycle-5`, `tests-lifecycle-7`, `tests-lifecycle-9`, `tests-lifecycle-11`, `tests-lifecycle-15–18`, `tests-lifecycle-20`, `tests-lifecycle-21`, `tests-lifecycle-23`, `tests-lifecycle-25`, `tests-lifecycle-26`, `tests-lifecycle-28`, `tests-lifecycle-29`, `tests-lifecycle-33`, T26, T59, T108, T130–132, T158, N16.

- [ ] Check observer progress, duplicates, wakes, and content with varied schedules.
  Sources: `tests-observation-5`, `tests-observation-7–11`, `tests-observation-13`, `tests-observation-14`, `tests-observation-17`, `tests-observation-20`, `tests-observation-22–26`, `tests-observation-28`, `tests-observation-30`, `tests-observation-35`, `tests-observation-36`, T13, T78, T90, T130, T132, T144.

- [ ] Simplify lifecycle fixtures while preserving ownership and version checks.
  Sources: `tests-lifecycle-2`, `tests-lifecycle-3`, `tests-lifecycle-6`, `tests-lifecycle-8`, `tests-lifecycle-12–14`, `tests-lifecycle-19`, `tests-lifecycle-22`, `tests-lifecycle-24`, `tests-lifecycle-27`, `tests-lifecycle-30`, `tests-lifecycle-32`, T52, T130–132.

- [ ] Consolidate observer helpers and correct delivery/checkpoint explanations.
  Sources: `tests-observation-1`, `tests-observation-2`, `tests-observation-4`, `tests-observation-6`, `tests-observation-12`, `tests-observation-15`, `tests-observation-16`, `tests-observation-18`, `tests-observation-19`, `tests-observation-21`, `tests-observation-27`, `tests-observation-29`, `tests-observation-31–34`, T48–50, T52, T130–132.

## 18. Disruption and handshake tests

Dependencies: 04–05 and 16.

- [x] Keep party-conservation fault coverage in the in-process harness — `37bcab3f`, `5619a9ff`, `41129172`, `64964725`.
  Sources: `suite-economics-10`, `tests-disruption-handshake-2–4`, `tests-disruption-handshake-7`, `tests-disruption-handshake-8`, T132, T143, T149, T154, N15.

- [ ] Exercise driver termination, cancellation, redaction, and handshake ordering without vacuous checks.
  Sources: `session-bookmark-18`, `session-bookmark-42`, `testing-infra-4`, `testing-infra-21`, `testing-infra-22`, `tests-disruption-handshake-14–18`, `tests-disruption-handshake-21–23`, `tests-disruption-handshake-25`, `tests-disruption-handshake-26`, T19, T28, T129, T130, T132.

- [ ] Consolidate disruption/handshake fixtures and clarify tested schedules.
  Sources: `tests-disruption-handshake-1`, `tests-disruption-handshake-5`, `tests-disruption-handshake-6`, `tests-disruption-handshake-9`, `tests-disruption-handshake-11–13`, `tests-disruption-handshake-19`, `tests-disruption-handshake-20`, `tests-disruption-handshake-24`, `tests-disruption-handshake-27`, `tests-disruption-handshake-29–32`, T48, T50, T52, T130–132.

## 19. Bookmark behavioral tests

Dependencies: 08 and 16; 15 for schedule generators.

- [ ] Reconcile the completed causality repairs and cover storage failure paths.
  Sources: `tests-bookmark-8`, `tests-bookmark-9`, `tests-bookmark-11`, `tests-bookmark-12`, T8, T26, T149, T150.

- [ ] Exercise durability and checkpoint guarantees across the full schedule.
  Sources: `session-bookmark-31`, `tests-bookmark-4`, `tests-bookmark-7`, `tests-bookmark-15`, `tests-bookmark-19`, `tests-bookmark-21`, `tests-bookmark-24`, `tests-bookmark-26`, T130, T132.

- [ ] Simplify bookmark healing and clarify the tests’ names.
  Sources: `tests-bookmark-13`, `tests-bookmark-23`, T132.

- [ ] Consolidate bookmark test helpers and clarify durability checks.
  Sources: `tests-bookmark-1–3`, `tests-bookmark-5`, `tests-bookmark-10`, `tests-bookmark-14`, `tests-bookmark-16–18`, `tests-bookmark-20`, `tests-bookmark-22`, `tests-bookmark-25`, T48, T49, T131, T132.

## 20. Resource, wire-format, and public-surface tests

Dependencies: 03, 05–09, 13 and 16 as each case requires.

- [ ] Make resource and transport tests establish progress and convergence.
  Sources: `tests-resource-link-window-10`, `tests-resource-link-window-12`, `tests-resource-link-window-24`, `tests-resource-link-window-26`, `tests-resource-link-window-27`, `verification-infra-17`, T98, T132.

- [ ] Strengthen wire examples and errors without accidental snapshot changes.
  Sources: `remote-capture-atlas-20`, `tests-wire-format-4`, `tests-wire-format-5`, `tests-wire-format-12`, `tests-wire-format-14`, `tests-wire-format-18`, `tests-wire-format-22`, `tests-wire-format-27`, T5, T85, T93, T130, T132.

- [ ] Simplify capture rendering and keep one vocabulary definition.
  Sources: `remote-capture-atlas-8`, `remote-capture-atlas-12–17`, `remote-capture-atlas-19`, `remote-capture-atlas-23`, `remote-capture-atlas-27`, `remote-capture-atlas-29`, `remote-capture-atlas-31–35`, T9, T55, T132, T138, T140.

- [ ] Keep seed and snapshot discovery accurate without deleting counterexamples.
  Sources: `streaming-tests-20`, `tests-common-31`, `tests-common-32`, `tests-lifecycle-1`, `tests-observation-38`, `tests-wire-format-19–21`, T59, T91, T132.

- [x] Confirm the public future-size check runs under its intended profile — `713d5854`.
  Sources: `suite-economics-1`, `tests-wire-format-25`, `tests-wire-format-26`, `verification-infra-1`, T7, T26.

- [ ] Simplify capture/atlas helpers and correct renderer descriptions.
  Sources: `remote-capture-atlas-1–7`, `remote-capture-atlas-9–11`, `remote-capture-atlas-18`, `remote-capture-atlas-21`, `remote-capture-atlas-22`, `remote-capture-atlas-25`, `remote-capture-atlas-26`, `remote-capture-atlas-30`, T49, T50, T52, T126, T132.

- [ ] Consolidate resource/transport fixtures and clarify their measurements.
  Sources: `tests-resource-link-window-1–4`, `tests-resource-link-window-6`, `tests-resource-link-window-9`, `tests-resource-link-window-11`, `tests-resource-link-window-13–15`, `tests-resource-link-window-17`, `tests-resource-link-window-21–23`, `tests-resource-link-window-29`, T48, T50, T52, T129, T131, T132.

- [ ] Consolidate wire-test fixtures and clarify protocol expectations.
  Sources: `tests-wire-format-1–3`, `tests-wire-format-6`, `tests-wire-format-8–11`, `tests-wire-format-13`, `tests-wire-format-15–17`, `tests-wire-format-23`, `tests-wire-format-24`, `tests-wire-format-28`, T48, T50, T52, T93, T131, T132.

## 21. Benchmark cost and measured performance

Dependencies: 10–12 for affected implementation baselines; 16 for shared support.

- [ ] Measure useful operations without fixture, runtime, or destructor work in the timed body.
  Sources: `benches-envelope-1–3`, `benches-envelope-11–13`, `benches-envelope-15`, `benches-envelope-16`, `benches-envelope-18`, `benches-envelope-20`, `benches-envelope-23`, `benches-envelope-25`, `suite-economics-2`, `suite-economics-3`, `suite-economics-7`, T89, T109, T112, T114, T129, T132.

- [ ] Locate the linear term in a session with one difference.
  Sources: N52.

- [ ] Use measured test-profile improvements, without blindly optimizing the whole crate.
  Sources: T123, T169, N53.

- [ ] Consolidate benchmark fixtures and correct timing/cost descriptions.
  Sources: `benches-envelope-4`, `benches-envelope-5`, `benches-envelope-7–10`, `benches-envelope-14`, `benches-envelope-17`, `benches-envelope-21`, `benches-envelope-24`, `benches-envelope-26`, `benches-envelope-27`, `benches-envelope-30`, T50, T129, T132.

- [ ] Remove redundant test execution and stale cost explanations.
  Sources: `suite-economics-6`, `suite-economics-11`, T53, T132.

## 22. Verification recipes and dependencies

Dependencies: API lints only after their affected public surfaces are clean; broad settings after 15.

- [ ] Diagnose `before`'s `ff_party_decode` fuel-band failure (13,612 fuel at 136 bits).
  Reproduce with `just fuzzfit`; preserved [seed](../../crates/before/fuzzfit/harness/proptest-regressions/enforce.txt): `ea7f69a7…`.
  Owner approved proceeding despite this failure, 2026-09-10. Keep the test enabled; this specific failure does not block Rumors batches.

- [ ] Make gate and CI run the intended checks reproducibly.
  Sources: `remote-proxy-tests-27`, `tests-disruption-handshake-33`, `tests-observation-37`, `verification-infra-2`, `verification-infra-7–10`, `verification-infra-12`, `verification-infra-13`, T15, T20, T26, T28, T30, T155, N02, N08, N40.

- [ ] Enable the approved compiler lints when their real code fixes are complete.
  Sources: `clippy-pedantic-1–15`, T46, T47, T52, T54, T57, T132, T158.

- [ ] Prune unused dependencies and confirm feature combinations still build.
  Sources: `deps-1–5`, `deps-7–11`, `inventory-2`, T20, T26, T28, T64, T90, T128, T132, T135, T147, T159.

- [ ] Remove stale verification configuration and tool references.
  Sources: `verification-infra-11`, `verification-infra-15`, `verification-infra-18`, T5, T132.

## 23. Public documentation and explanation placement

Dependencies: Contracts and API signatures from earlier batches; prose improves in every batch.

- [ ] Explain the library model and API at the reader’s level.
  Sources: `api-core-17`, `api-core-18`, `api-core-26`, `api-core-27`, `prose-hygiene-2`, `session-bookmark-34`, `session-bookmark-44`, `tree-core-6`, T46, T51, T58, T75, T82, T95, T132, T141, T158, N09.

- [ ] Put sizing and reconciliation explanations where they belong.
  Sources: `mirror-common-12`, `module-graph-4`, T56, T82, T87, T92, T93, T100, T102–104.

- [ ] Clarify public API reachability, errors, and costs.
  Sources: `api-audit-5`, `api-audit-6`, `api-audit-8–10`, `api-audit-16–18`, T55, T82, T92, T132.

- [ ] Repair crate navigation, examples, and model descriptions.
  Sources: `fresh-eyes-1–3`, `fresh-eyes-5–7`, `fresh-eyes-11`, T49, T82, T92, T104, T132.

- [ ] Correct inaccurate prose and remove needless jargon throughout.
  Sources: `prose-hygiene-1`, `prose-hygiene-3–12`, T5, T48, T49, T125, T130, T132.

## 24. Module layout and remaining local cleanup

Dependencies: Affected behavioral/API work first; broad import reflow and path moves last.

- [ ] Simplify module boundaries, then flatten the extra streaming path level.
  Sources: `module-graph-2`, `module-graph-6`, `module-graph-14`, `streaming-tests-2`, T53, T86, T121, T132, T158, T159.

- [ ] Apply import grouping after overlapping changes have settled.
  Sources: T52, T158.

- [ ] Remove duplicated constants, redundant representations, and stale claims.
  Sources: `inventory-3`, `inventory-4`, `inventory-6–17`, `inventory-19`, `inventory-20`, T46, T52, T54, T57, T64, T125, T132.

- [ ] Simplify module boundaries and keep navigation accurate.
  Sources: `module-graph-1`, `module-graph-3`, `module-graph-5`, `module-graph-7–13`, `module-graph-15`, `module-graph-16`, T46, T52, T121, T125, T132.

## 25. Publication preparation

Dependencies: Accepted implementation and documentation work complete.

- [ ] Prepare MPL-2.0 licensing and package metadata; keep publishing disabled.
  Sources: `deps-12`, T79.

## 26. Historical, declined, deferred, and external work

- [x] Verify swarm/memwatch removal and remaining references — `6e5eb555`, `6b258a5f`.
  Sources: `suite-economics-9`, `swarm-example-1–31`, T27, T132, T135, T136.

- [x] Declined: a new Rumors mutation campaign or coverage threshold.
  Sources: `verification-infra-3`, `verification-infra-4`, T11, T12.

- [x] Deferred: Rumors decoder fuzzing.
  Sources: `verification-infra-5`, T14.

- [ ] Retain the human-checked formal transcription without a generated comparison tool.
  Sources: `streaming-tests-28`, T29.

- [ ] Verify historical integration repairs and preserve the before counterexamples.
  Sources: T146, N10, N30, N44.

- [x] Decided: adopt one active batch and editable branch review; retire parallel coordination machinery.
  Sources: T1–4, T25, T133, T134, T137, N43, N47, N55.

## 27. Finish and reconcile

Dependencies: All accepted outcomes above resolved and merged, or a new explicit disposition recorded.

- [ ] Review the remaining code and prose against every unchecked item.
  Sources: T5, T141, T171.

- [ ] Present the Sush compatibility branch for final review, pinned and tested against the final Rumors revision without a local override.
  Source: User request; [Sush setup](sush-pooling.md).
