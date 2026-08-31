# Design: the collision-schedule test mode

A parallel way to run the existing test suite in which leaf-path derivation
is swapped from blake3 to an injective, collision-*planning* schedule, so
every behavioral suite exercises tree geometry that hash-derived paths
cannot feasibly produce. Designed with the owner in conversation; this note
is the record of the converged design, ahead of implementation.

Provenance: the
[unknown-pruning survivor investigation](../2026-08-21-unknown-pruning-survivor/README.md).
Its root cause generalizes: any code conditioned on deep branching — a real
branch at height 1 requires two leaves sharing a 31-byte path prefix, a
248-bit blake3 prefix collision — is unreachable through the public API, so
the suites cannot exercise it no matter how their scenarios are shaped. The
streaming unknown-filter's leaf arm is one such site; the typed oracle's
`Unknown for Z` arm, deep `act` edge-splitting, bottom-level protocol
walks, and leaf-parent disputes with real siblings all sit in the same
shadow. A targeted proptest family (forged paths, internal entry) pins the
unknown filter specifically; this mode is the broad instrument that runs
*everything* under collision-rich geometry through the public API, with no
internal-entry exemption.

## The seam

`Path::for_leaf` (`src/tree/typed/path.rs`) is the single point where
versions become paths. The seam:

- Gated by the `test-internals` feature (integration-test binaries link the
  non-`cfg(test)` library, so `cfg(test)` alone cannot carry it).
  Production builds compile the bare blake3 call; the mode costs shipped
  code nothing.
- Activated by an env var carrying the schedule seed (name TBD; e.g.
  `RUMORS_PATH_SCHEDULE=<u64>`). Unset means blake3, byte-identical
  behavior to today. The schedule installs lazily on the first path
  derivation in the process.
- **Scope: leaf paths only.** Merkle content hashing (`Hash::leaf`,
  `Hash::branch`) is untouched: the protocol genuinely owes its
  content-addressing soundness to collision resistance ("equal hash means
  equal subtree"), and breaking that tests nothing the model claims.
- **Full-width injectivity is mandatory.** `answer.rs` legitimately rests
  on "equal path means equal leaf" (its Both arm, and the terminal-leaf
  violation check). Prefixes may collide freely; full 32-byte paths never.
  The schedule keeps a reverse set and resolves any planned full-width
  collision by deterministic spill.

## The schedule

A merely-different hash does not reach the bottom: uniform assignment gives
expected max shared prefixes near log2(pairs) bytes, leaving height-1
branches about as unreachable as blake3 does. The schedule must *plan*
collisions: versions are assigned to clusters, each cluster sharing a
prefix whose length is drawn across the full 0..=31 range with weight on 31
(the only length that makes leaf-arm descent live), distinct final bytes
within a cluster.

Assignment should be a pure function of (seed, version bytes) — a seeded
non-cryptographic PRF choosing the cluster and the tail — with the memo and
reverse set handling spill. Purity in the version bytes makes paths
independent of arrival order, so concurrent tests get reproducible
geometry; memoization on bytes is faithful because blake3 is itself a pure
function of the same bytes.

## nextest enforcement

The memo is a process-global static, which is per-test state exactly
because nextest runs process-per-test — under `cargo test`'s shared
process, two tests' schedules would interact. The installer therefore
checks `NEXTEST_EXECUTION_MODE=process-per-test` when the env var is set
and panics loudly otherwise, with a message naming the requirement and the
reason. This is harness-misuse detection: programmer error by construction,
a sanctioned panic. Doctests and examples never see the var unless someone
exports it globally — which is precisely the case worth catching.

## `assume_blake3()`

A plain `test-internals` function (no macro: it injects no early return),
called as the first line of any test whose *subject* pins blake3-derived
geometry. It forces the seam to blake3 for the whole process — sound
because process-per-test scopes it to exactly one test — so the marked
minority runs its real assertions in both modes. No test ever passes
vacuously.

- **Ordering guard**: it panics if the schedule already initialized in this
  process. This mechanically enforces call-before-first-derivation and
  keeps the call from hiding inside shared helpers where it would silently
  widen.
- **Marking criterion**: the test derives paths *and* pins their geometry —
  not "uses insta." `before`'s snapshot suites pin version encodings and
  never touch paths; they need no mark. Known candidate classes: the
  byte-exact wire snapshot suites (`tests/gossip_snapshot.rs` and kin);
  structure pins over concrete inserts (`compressed_prefix_len`, child
  counts); the geometry-searching generators in `src/tree/arb.rs` (their
  simulation self-checks survive the swap, but their search predicates can
  exhaust the attempt budget under clustered geometry); possibly the
  uniform-hash quantitative suites (window census/knee/operator, capacity
  stress) — empirical, see triage.
- **Roster value**: the call sites are a greppable, tamper-evident, in-code
  inventory of every test resting on the uniform-hash model of record. Each
  call carries a one-line rationale saying why the subject assumes blake3.

## First-sweep triage

Run the full suite once in the mode. Every failure resolves to exactly one
of:

- a genuine geometry-shadow bug — a finding, the mode's purpose; or
- a legitimate blake3 assumption — mark with `assume_blake3()` plus its
  rationale line.

The hazard to write into any brief that runs this sweep: an agent under
completion pressure will mark-to-green. The stated-rationale-per-mark
requirement is the check; the marks are reviewed as a set.

## Deferred: a skip knob

A second env var skipping the marked tests (they run twice across the two
modes) is deferred: a runtime skip can only express itself as an early
return, which reads as PASS and reintroduces the vacuous-pass mechanism
this design eliminates. Build it only if the duplicative weight is
measured to matter, and then as a local iteration knob no gate or CI leg
ever sets.

## Integration

- **Recipes**: a justfile leg running nextest with the env var set (seed
  pinned; whether to sweep several seeds is open). Tier placement — gate
  vs. `ci` — is open, pending a timing measurement.
- **Mutation campaigns**: the campaign's test command must include the
  collision leg, or bottom-of-tree mutants keep surviving it — the mode
  only kills what the mutation run executes.
- **Relationship to the targeted family**: keep both. The forged-path
  proptests in `unknown/tests.rs` are the fast deterministic pin at the
  module that documents the filter's bottom semantics; the mode is the
  suite-wide sweep that needs no per-site foresight.

## Open questions

1. Env var name and seed format.
2. One pinned seed per leg vs. a small seed sweep.
3. Gate tier vs. `ci` tier (measure the run first).
4. Whether the schedule should also plan *partial* clusters (shared
   30-, 29-byte prefixes) at higher weight to stress `act` edge-splitting
   at every depth, or whether the 0..=31 draw already covers it.
