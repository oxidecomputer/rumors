# Handoff: the surviving leaf-verdict mutant in the streaming unknown-filter

An investigation brief. A mutation that inverts the leaf-level supply
verdict in the streaming mirror's deletion-honoring filter survives the
entire workspace test suite. Either the protocol's other machinery
genuinely makes the verdict irrelevant to every observable the suites
pin (in which case the filter's contribution is quantitative and needs a
meter, or the code needs restructuring), or the cross-peer suites have a
blind spot around exactly the property this filter exists to provide —
deletions propagating by the receiver never re-learning the leaf. Both
readings demand action; nobody has determined which is true.

## The site

[`src/tree/mirror/streaming/materialized/unknown.rs`](../../src/tree/mirror/streaming/materialized/unknown.rs),
the height-0 arm of `unknown` (the module's doc explains the filter's
role; read it first):

```rust
// A leaf is known iff its ceiling is causally at or before
// `known`; a concurrent ceiling is beyond the known-at range,
// so those survive.
let verdict = Some(node).filter(|node| !self::known(node, known));
```

The mutation deletes the `!`. Read what that means: the filter's job is
to prune a subtree down to what a counterparty at version `known` is
*missing*. Inverted, the leaf arm supplies exactly the leaves the
counterparty already has — deleted content included — and withholds
every leaf it is missing. This is not a subtle perturbation; it negates
the leaf-level meaning of the walk. The `stats.shed` accounting inverts
with it: shed credits count supplied-instead-of-shed leaves and vice
versa.

The higher arms (`Dominance::Before`/`After` fast paths on the memoized
span bounds) are untouched by the mutation, so whole-subtree verdicts
still classify correctly; only leaves reached individually — the
`Between`-descent bottom — carry the inverted verdict.

## Provenance

- The mutant is survivor #51 of the CBOR-wire mutation campaign
  (cargo-mutants scope A over the packet roster, whole-workspace suites,
  `--all-features`): verdict MISSED, meaning the full suite passed with
  the mutation applied. Campaign artifacts (roster, outcomes, diffs)
  live on ox-east-1 under the `agent` account at
  `~/build/rumors-mutants/scopeA/mutants.out/`; the sweep may have
  progressed past the state described here.
- The campaign's triage agent additionally probed it by hand before the
  campaign was wrapped: 311 streaming/gossip tests pass with the filter
  inverted (agent-reported; not independently re-run).
- The code reading above is verified against the source at the time of
  writing. The campaign ran on the CBOR-wire branch, but the filter is
  byte-identical on main apart from one unrelated doc word (verified by
  diff), so the finding is independent of that branch's changes.
- Nothing about the mutant's *mechanism of survival* has been
  established. Every hypothesis below is exactly that.

## Why survival is surprising

Three suite families look like they should catch an inverted supply
verdict, and none did:

1. **Convergence suites** (cross-peer gossip in `tests/`): peers that
   reconcile must end with equal sets. If missing leaves are withheld at
   the leaf verdict, convergence should fail — unless another exchange
   path supplies them regardless.
2. **Redaction/deletion suites**: deletions propagate by never
   re-learning the leaf (no tombstones — see the crate docs). Inverted,
   deleted-at-the-counterparty leaves are re-supplied. If the receiver
   re-learns them, redaction is broken; if receiver-side version logic
   drops them on ingress, the wire carries resurrection traffic that
   endpoint-state oracles cannot see.
3. **Wire snapshot pins** (`tests/gossip_snapshot.rs`): byte-exact
   transcripts of pinned scenarios. Their passing means that in every
   pinned scenario, no individually-judged leaf's verdict differed — the
   scenarios may simply never reach a `Between` node whose children mix
   known and unknown leaves.

## Hypotheses, in decreasing order of concern

- **H1 — redaction gap**: a redact-then-regossip scenario where the
  deleted leaf sits under a `Between` parent re-supplies the deleted
  content and the receiver re-learns it. If constructible, this is a
  real correctness bug reachable only through the mutant today, and the
  redaction suites are missing the scenario shape.
- **H2 — recovery masking**: the mirror protocols' dispute/recovery
  machinery (early supply, empty listings, per-level comparison) fetches
  every genuinely-missing leaf through paths that do not consult this
  filter, so endpoints converge regardless and the filter is
  load-bearing only for *how much* travels, not *what state results*.
  Then the property is quantitative and the right instrument is a pin on
  `messages_shed` or wire bytes for a scenario where the verdict
  matters, not a behavioral test.
- **H3 — fixture shadow**: the suites' scenarios never construct a
  mixed-knowledge parent at the bottom of the tree (small sets, aligned
  versions), so the leaf arm's verdict never varies within any test. The
  fix is a scenario generator that forces `Between` at height 1 with
  interleaved known/unknown/deleted leaves.

These are not exclusive: H3 can hide H1, and H2 can be true for
convergence while H1 is true for redaction.

## Suggested procedure

1. Reproduce: apply the inversion as a reversible string swap
   (`!self::known(node, known)` → `self::known(node, known)`), run the
   gossip/streaming/redaction suites, confirm the survival; restore by
   the same swap and verify with an empty `git diff`.
2. Construct the discriminating scenario before theorizing: two peers,
   one redacts a message, versions arranged so the redacted leaf's
   parent classifies `Between` for the counterparty's `known` (the
   memoized span bounds decide this — see the module doc and
   `knowledge`). Gossip, and assert the deleted leaf is not re-learned.
   Run it unmutated (must pass) and mutated (H1 predicts failure).
3. If it passes mutated, trace where the missing/deleted leaves actually
   travel or get dropped: the observation hooks (`rumors::observe`) and
   the capture renderer decode live session traffic precisely for this
   kind of differential; diff mutated vs unmutated transcripts of the
   same scenario.
4. Disposition by what you find: a killing behavioral test (H1/H3), a
   quantitative pin plus possibly a restructuring that makes the verdict
   structurally unmutatable (H2), or — only with a written proof of
   equivalence, which the semantics above make unlikely — a roster
   exclusion under `.cargo/mutants.toml`'s documented discipline.

Related but separate: campaign survivor #49 (`recv_msg_with`'s EOF arm
in `remote.rs`, diagnostic-prose-only) is analyzed in the campaign's
triage record and is not part of this handoff.
