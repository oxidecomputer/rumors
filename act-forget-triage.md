# Triage: surfacing the causally-prior Forget skip as a poisoned-store indicator

The report recommends surfacing the skip in `Tree::act`'s Forget arm
(`src/tree/traverse/act.rs`, the `version < node.ceiling()` guard at leaf
height) when a locally-issued redaction is dropped because the resident
leaf's version dominates the redact's version. This note triages that
recommendation; no implementation.

## Can the skip still fire for locally-issued redactions once ingestion enforcement lands?

Not through the public API. The store invariant that enforcement restores is:
every resident leaf version is causally contained in the tree's ceiling.

- Local actions maintain it: `Tree::act` ticks every action's version from
  the current ceiling and joins effectual versions back in, so residents
  are always `<=` ceiling.
- Session ingestion now maintains it: every supplied subtree's ceiling must
  be contained in the sender's declared version
  (`Violation::UncontainedSupply`), and the post-session join folds that
  declared version into the ceiling — so absorbed content is `<=` the new
  ceiling.
- A redact ticks strictly past the ceiling, so for any resident leaf,
  `redact_version > ceiling >= leaf_version`: the skip's condition
  (`redact_version < leaf_version`) is unsatisfiable.

The skip remains reachable only for a *poisoned store*: a replica that
absorbed an escaped version before enforcement (it survives only as a live
process — the replica is in-memory, so a restart rebuilds through now-guarded
sessions), or in-crate constructions that bypass sessions (`Tree::react`,
tests). A pre-enforcement poisoned replica is additionally quarantined by its
peers: its own greeting no longer covers the escaped leaf it re-supplies, so
enforced counterparties fail the session — with the caveat that the violation
blames the poisoned *sender*, which in that scenario is itself a victim.

Note the skip is not dead code: it is load-bearing for ordering within one
`act` batch (the causally-latest action at a path wins) and for `react`'s
merge semantics. It cannot be removed; the question is only whether its
firing on a *Forget of a resident leaf* should be observable.

## Surfacing options that preserve the documented contract

"Redacting a key not currently held is a no-op" (`Rumors::redact`) is about
absent keys; the skip drops a redact of a *held* key, a distinct event, so
none of these change the documented semantics:

1. **`debug_assert!` in the Forget arm** when the skipped action is a Forget
   and the leaf exists. Cheapest, but it fires at the symptom (a user redact,
   arbitrarily long after the store was poisoned), and it must carefully
   exempt legitimate same-batch orderings.
2. **Observer plumbing**: extend `act`'s effectual-action observer to report
   dropped Forgets, surfaced as a counter or tracing event. Public-surface
   growth for an event that enforcement makes unreachable.
3. **Whole-store coherence check**: the poisoned state is directly decidable
   in O(1) — the root node's content ceiling must be contained in the tree's
   declared ceiling (`contained(root.ceiling(), tree.latest())`, both
   memoized bounds). A `debug_assert!` at the session-commit join (and/or a
   `test-internals` probe the conformance suite can call) detects the
   poisoned store at the moment it forms, not at the redact that later
   silently fails.

## Recommendation

Leave `act`'s Forget path untouched. Post-enforcement, the skip on a
locally-issued redaction is unreachable from the public API, so instrumenting
it polices a symptom that can no longer arise honestly — and the guard itself
is load-bearing for batch ordering. If a poisoned-store indicator is wanted,
option 3 is strictly better: one O(1) comparison of two memoized bounds at
the join seam, catching the poisoned store at formation with no change to
redact semantics or the observer contract. Frame it, like the ingestion
check, as a conformance tripwire. The immortality mechanism itself is pinned
by `escaped_version_defeats_redaction_in_a_poisoned_store`
(`src/tree/tests.rs`), which documents exactly what option 3 would guard.
