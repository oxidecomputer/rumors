<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a proposal, from a full read of README.md and the six topic documents' introductions, highest-value lists, and crate-wide patterns, plus a mechanical extraction of every entry; not authored, audited, or endorsed by Finch. Every recommendation here is a recommendation. Read with the ground rules in ../README.md. -->

# Triage plan for the holistic review

This addendum proposes how to work through the review's 995 entries with
Finch's judgment applied where it changes the outcome and nowhere else. It
adds three things to the note: this plan, a ledger with one row per finding
(`triage/ledger.tsv`, maintained by `triage/ledger.py`), and a rulings file
(`triage/rulings.md`). The six topic documents and the evidence tree stay as
written; they are the record of what was found, and the ledger is the record
of what was decided.

## The shape in one paragraph

The unit of triage is not the finding. It is the ruling and the lane. A
ruling is a decision only Finch can make: a crate-wide convention, a public
API shape, a reopened prior ruling, a design question, or a high-severity
verdict. A lane is a scoped batch of entries whose resolution the review
already states with an acceptance criterion, executed by an agent in a
worktree and reviewed by Finch as a diff rather than as a list. The review's
own structure makes this efficient: the README's 102 owner decisions already
group 215 entries, and the topic documents' crate-wide patterns group 612
more into fewer than seventy named classes, each of which one ruling or one
sweep disposes. What remains after that are 321 entries that only a
per-module lane reaches, most of them low or nit, each carrying its own
stated resolution. Ordering follows dependency and risk: the instruments that
read failure as success get fixed first, because every later lane is judged
by them; then the production defects; then the convention rulings that
unblock mechanical sweeps, each landing with a committed remainder-detector,
because the review's largest theme is passes that stopped one step short of
their own remainder; then the pattern sweeps, the module lanes, the
pre-release API pass, the measured performance work, and the design
questions. Triage closes when `ledger.py check` reports zero pending rows
(open, or `fix` awaiting its commit) and zero defects.

## Dispositions

Every row ends in exactly one of these. The vocabulary is small on purpose:
the doctrine admits a fix or a declared model and nothing that functions as
a bin for known failures.

| Disposition | Meaning | Terminal when |
|---|---|---|
| `fix` | Land the entry's stated Resolution, meeting its Acceptance. | `sha` names the landed commit. |
| `fix-amended` | Land a resolution Finch amended. | `sha` and a ruling carrying the amendment. |
| `model` | The behavior is intended. The ruling names where the intent is stated so the finding cannot recur: a rustdoc sentence, an AGENTS.md line, a comment at the site. | A ruling with a named home. |
| `defer` | A design proposal or a resource trade that waits on a measurement or a later phase. The ruling names the home: a `design/` document, a shadow-tracker issue, or a named lane. | A ruling with a named home. Never admitted for a high-severity or correctness-class entry. |
| `dispute` | Finch refutes the finding. The ruling records why. | A ruling. |
| `dup` | Fully carried by another entry. | A ruling or the `note` column naming the entry of record. |

`ledger.py check` enforces the terminal conditions and the `defer`
exclusion. A `model` that names no home is the cheapest artifact that would
pass a looser check, and it is exactly the disposition that lets a finding
come back in the next review; the home requirement closes that path.

## The record

`triage/ledger.tsv` has a row per id with two column groups. The seed
columns are derived from the documents by `ledger.py seed` and carry the
document, severity, class, provenance, owner-gating, module heading, the
crate-wide patterns the entry belongs to, the README owner decisions and
highest-value items that cite it, a proposed phase, and (for module-lane
entries) a proposed lane. The triage columns are filled by hand as work
lands: `disposition`, `ruling`, `sha`, `note`. Reseeding preserves the
triage columns, and a `note` beginning `phase!` pins a hand-set phase and
lane against reseeding.

`triage/rulings.md` holds numbered, dated rulings, appended and never
edited, in the format its header states. A ruling disposes an owner
decision number, a pattern, or a list of ids, and says positively what was
decided and where the intent now lives.

Three checks make the record trustworthy, per the doctrine's own rule that
the cheapest passing artifact must be the intended one:

- Coverage: `check` fails while any id in the six documents lacks a row,
  and counts every non-terminal row as pending, so the 343 nits cannot be
  quietly forgotten.
- Provenance: a `fix` needs a sha, and the coordinator verifies the sha's
  tree meets the entry's Acceptance before writing it; an agent's report
  that it did is data, not the record.
- Remainder: every sweep of a crate-wide class lands with the class's
  regenerating grep (the documents record them: prose-hygiene-9 for
  em-dashes, prose-hygiene-10 for "seam", and so on) or a `tools/` check
  wired into `just gate`, run to zero in the same commit series, so the
  sweep cannot stop one step short the way every prior pass did.

## Phases

The phase of each entry is proposed in the ledger by a stated rule (the
README owner-decision number, the pattern's class, hand placement for the
eleven highs and the gate holes, and the document's default otherwise; the
mapping is in `ledger.py`). Counts below are the seed at the reviewed
commit; `ledger.py summary` is authoritative.

| Phase | Entries | Rulings needed | Depends on |
|---|---|---|---|
| P1 Instruments | 67 | Owner decisions 42-61, and 81 | nothing |
| P2 Production correctness | 20 | 36-41, 94, 95 | P1 for the harnesses their acceptance tests run under |
| P3 Conventions and lints | 138 | 1-14 | nothing; unblocks P4, P5, P6 |
| P4 Pattern sweeps | 359 | 73-78, 80-93 | P3 rulings; P1 where a sweep touches a harness |
| P5 Module lanes | 321 | none new; lane rosters approved | P3, P4; the harness lane before the other test lanes |
| P6 Public API pass | 56 | 15-35 | P3 lints (they enumerate the work) |
| P7 Performance and suite cost | 32 | 62-72 | P1 (honest harnesses); meters before trades |
| P8 Design questions | 2 | 94-102 | nothing; each gets a home |

### P1: Instruments before cures

Goal: every check the later phases rely on reports failure as failure. The
review found harnesses that fold a protocol violation into "did not stall"
(streaming-tests-11), a parent that never joins its serving tasks
(tests-disruption-handshake-7), a causality harness that swallows
unconditionally-bug errors (tests-bookmark-12) and whose recycle oracle is
blind to the recycle the bookmark exists to prevent (tests-bookmark-9), two
census checks that difference identical runs (conformance-28,
tests-resource-link-window-20), a deletion verdict no committed test
discriminates (materialized-27), a renderer whose injectivity the snapshot
discipline rests on and which is not injective (remote-capture-atlas-13,
remote-capture-atlas-17), a test binary compiled out of every committed
check (tests-wire-format-26), streams two and above carried by nothing
(remote-proxy-tests-10), and holes in the gate's own tools. A fix landed in
P2 under any of these is unverified.

Members: `awk -F'\t' '$11=="P1"' triage/ledger.tsv`. Ten of the eleven
highs are here, with the census floors, the unscheduled instruments
(`envelope_sim`, `tradeoff_probe`, the mutation campaign, the coverage
scope, the fuzz target), the `testdoc` blind spots, and the CI and
composite-recipe gaps.

Rulings this phase needs, in the order the work wants them:

1. Owner decision 81 first: the renderer fix (remote-capture-atlas-13)
   changes how container keys render, which moves `insta` snapshots, and
   AGENTS.md's renderer-vocabulary re-accept class demands a hexdump
   witness the corpus cannot produce (prose-hygiene-6). Rule on the
   class's wording before the fix lands, or the fix cannot be re-accepted
   under the hard rule as written.
2. Decision 48 (the backend conformance suite), with the census floor
   landing before `LOCAL_BUDGET` is resized, since the floor is what
   demonstrates the budget binds.
3. Decision 42 (`future_size`): the first run under the dev profile may
   itself fail; that failure is a finding, not a reason to gate it back
   out.
4. Decisions 49, 50, 51: the causality harness, the overlap shadow's
   meta-test, and the injectivity pin's form (inverse parser versus
   leaf-mutation property).
5. Decisions 43-47, 53-58, 60, 61: the certificate's future, mutants
   cadence, coverage scope, fuzz target, composite recipes, the
   `tradeoff_probe` recipe, the envelope figures, `cargo test` as an
   entry, the docs.rs leg, the sync oracle, `ReorderingAcceptor`, and the
   proxy's `Connect` impls. Decision 59 (severity of
   tests-disruption-handshake-7) needs no ruling; the fix lands either way.

Ordering inside the phase: streaming-tests-11 before anything in P7 that
measures the capacity suite (decision 67), and tests-bookmark-12 before
tests-bookmark-9, since the oracle repair is only observable once the
harness stops swallowing errors. remote-proxy-tests-10 needs a new vehicle
(the second witness pass showed the deep fixtures cannot close it, because
the wire correctly refuses their off-model leaf paths); that is design
work inside a lane, and the brief should say so rather than hand the agent
a fixture that cannot work.

Acceptance for the phase: each repaired instrument has a committed
demonstration that a known-bad artifact fails it (the constructions in
`evidence/witness.md` are those artifacts, and the lane converts each into
a committed negative control), and `just gate` is clean.

### P2: Production correctness

Goal: no production code path a user or a conforming peer reaches misbehaves.
The members are small and contained: the `resume_payload` exactness clamp
(mirror-common-8, remote-codec-14; decision 38), the destructor-under-lock
hazard (async-hazards-3; decision 39: document the constraint now, defer
the pre-image drop past `send_if_modified`), the docs.rs build failure
(deps-3), the debug-only guards that degrade to wrong release behavior
(api-core-2, link-25, benches-envelope-34, and the sibling at
`gossip.rs:1395` the inventory sweep names and no finding covers; decision
94), the wrapping read-id counter (link-29), the unbounded RNG loop
(api-core-11), the `act` version-storage contract (tree-core-30; decision
95), the walk's fault classification (materialized-14), `warm_caches`'
promise (tree-core-8), reclaim timing (session-bookmark-21; decision 37),
and the pooling eviction (link-28; decision 36, the one member that is a
design change).

Rulings: decisions 36-41, 94, 95. Decision 36 is the largest: whether a
pooling `Dial` is a first-class deployment shape decides whether recovered
connections get a pre-`READY` budget or the caller keeps the sizing duty and
the constructed test pins the failure.

Ordering: deps-3, async-hazards-3's documentation half, and the clamp have
acceptance tests independent of the P1 harnesses and can land in parallel
with P1. The rest waits for the harness they run under.

### P3: Conventions and lints, each with a committed check

Goal: rule once on each crate-wide convention, land the mechanical check
where one exists, then sweep. This is the phase that converts the review's
largest theme (prose and instruments left behind by passes that stopped
short) into something a checker holds. The order within each item is
check, then sweep: land the lint or `tools/` check at `warn` or as a
failing gate leg, watch it enumerate the remainder, sweep to zero, promote
to `deny`. A sweep landed without its check is the failure mode the review
documents.

Rulings, owner decisions 1-14, and the check each yields:

| Decision | Ruling | Check that holds it |
|---|---|---|
| 1 | `pub` versus `pub(crate)` under private `tree` | `#![warn(unreachable_pub)]` |
| 2 | A `[lints]` table | the table itself, under `-D warnings` |
| 3 | Em-dashes in `//` and `#` comments | a `tools/` check on U+2014 in non-doc comment lines, in `just gate` |
| 4 | "seam" and "knob" | the recorded greps, run in the sweep commit; no standing check |
| 5 | "honest", "lie", and cousins | same |
| 6 | First-sentence mood | `tools/doclint` extension if ruled; else a one-time sweep |
| 7 | Import grouping | `group_imports = "StdExternalCrate"` if `fmt-check` may move to the pinned nightly; else a hand sweep |
| 8 | Inline `mod tests {}` blocks | a `tools/` check, if the rule is exceptionless |
| 9 | Which `type_complexity` allow to keep | the sweep commit |
| 10 | The `#[non_exhaustive]` rule | stated once in `src/error.rs`'s module doc; each closed enum carries one comment |
| 11 | "V2" in behavioral prose | the sweep commit |
| 12 | The illumos lint-allow comment | one AGENTS.md line plus a platform-free restatement |
| 13 | "genuine(ly)" and em-dash density in rustdoc | a taste ruling; a dedicated prose pass if ruled |
| 14 | Seed files | `seed_liveness.rs` extended to match shrink-note parameters against live `proptest!` signatures |

Members: 138 entries, 103 of them nits, which is the point: the nits are
the sweep's output, not its input.

### P4: Pattern sweeps

Goal: dispose each remaining crate-wide class in one pass with its full
site list, so the fixer sweeps the whole list rather than one partition.
The documents state each pattern once with the union of its sites, and the
ledger's `patterns` column gives the roster per class. The large classes:
hand-maintained counts and measurements (convert to named constants,
derived values, or delete the number), the V1 and wire-respelling residues,
testdocs that claim more than their bodies check, harness helpers
re-spelled per binary (this one is P5's harness lane; see below), guards
that recompute what types establish, duplicated halves of one mechanism,
one-caller wrappers, positional runs wanting a bundle, roster tags and
incident narrative in code, rewrap residue, the `send_all` braces, manual
impls identical to derives, redaction absent from generated populations,
convergence oracles weaker than whole-root equality, conformance-bug
detectors with no committed demonstration.

Rulings: owner decisions 73-78 (the constants' ownership, criterion ids,
`encoded_bits`, `tempfile`) and 80-93 (documentation placement: where the
sizing guide lives, the CBOR evolution rules, the off-model digest note,
the `Bookmark` example, and the rest). Most of P4 needs no ruling at all;
each entry states its resolution and acceptance, and Finch approves the
sweep's roster.

Acceptance per sweep: every member entry's Acceptance met; the class's
regenerating grep, where the documents record one, at zero; `just gate`
clean. Sweeps that touch test harnesses run after P1 has repaired them.

### P5: Module lanes

Goal: reach the 321 entries no pattern or owner decision groups, most of
them low and nit, each with a stated resolution. The ledger proposes nine
lanes from the documents' module headings:

| Lane | Entries | Covers |
|---|---|---|
| harness | 30 | `tests/common`, `src/testing`, `src/tests.rs` |
| tests | 72 | the integration suites |
| remote | 60 | codec, capture, adapter, proxy, and their suites |
| streaming | 41 | mirror common, backend and window, materialized, streaming tests |
| core | 33 | crate root, peer, session, bookmark |
| benches | 31 | benches, `envelope_sim`, `swarm` |
| link | 29 | link, routed, conformance |
| tree | 25 | tree core, typed |

The harness lane goes first among the test lanes: the shared drivers the
suites re-implement (tests-bookmark-3, tests-lifecycle-3,
tests-common-30) are the prerequisite for the tests lane's consolidation
entries, and the blanket `allow(dead_code, unused_imports)` on
`tests/common` hides work the tests lane would otherwise find by hand.

Protocol per lane: Finch approves the roster (striking or amending
entries; each strike is a `dispute` or `defer` with a ruling), the agent
works in a worktree from a named SHA, commits per logical unit, runs
`just gate` clean before each commit, and reports the shas and each
entry's acceptance evidence. The coordinator verifies acceptance against
the tree before writing `fix`, and Finch reviews the diff. Nits are swept
inside their module's lane and reviewed in the diff, never ruled on
individually.

### P6: The public API pass

Goal: one coordinated pre-release edit of the public surface, while it is
cheap. Owner decisions 15-35 are all owner-gated by nature and interlock:
the `Snapshot` iterator (15; the api and simplification documents disagree,
and the api document's position also closes the class mechanically via
`unnameable_types`), `Snapshot: PartialEq` (16), the `Bookmark` trait's
shape (17), the depth of `rumors::error` (18, the largest item, which also
reopens ruling R2 through mirror-common-10), `seed_rng` and `warm_caches`
(19, three positions across the reports), `send`'s return (20), direct
readers (21), observability (22), the conformance `bookmark` suite (23),
`Joined::Bailed` (24), payload bounds placement (25), `pub use ::before`
(26), and the rest through 35.

Ordering: after P3's lints land, because `unnameable_types`,
`missing_docs`, and `missing_debug_implementations` enumerate this phase's
mechanical half and turn the review's hand-listed gaps into diagnostics.
One decision session rules on 15-35 as a block; one lane executes; the
`insta` snapshots move only if the wire moves, which none of these should
cause.

### P7: Performance and suite cost, meters first

Goal: every claimed improvement moves a committed number. The review ran
no benchmark; every performance entry is a code reading. The phase
therefore opens with instruments: a walk-side allocation meter
(materialized-30; decision 68), a residency meter for message slack
(api-core-10), and per-action allocation counts on the commit path
(tree-core-27). Then the fixed-sign deletions (tree-core-27's single sort,
tree-typed-6's leaf half, streaming-backend-window-9, link-14's
`set_nodelay`, link-27), measured at the parent before crediting. Then the
trades, each gated on its measurement (tree-typed-23, tree-typed-30,
remote-adapter-streams-6 after the sub-FAN construction in decision 63,
remote-codec-24 which reopens ruling B2 in decision 62). Suite economics
(decisions 65-67, 71, 72) follow the same rule: measure `cargo build
--tests --timings` at HEAD before folding binaries; run the capacity test
at four parents once streaming-tests-11 makes its verdict honest.

Rulings: 62-72.

### P8: Design questions with a home

Goal: none of decisions 94-102 is left dangling. Each is either scheduled
into a lane (94's `expect` now, its design item later; 95's `react`
documentation; 99's `FAN` derivation) or recorded as a `defer` with a home:
a `design/` document (the crate has `design/rumors-frame-fuzz.md` as the
precedent) or a shadow-tracker issue, named in the ruling.

## Session protocol

Decisions come to Finch as numbered blocks, each item re-grounded with
enough context to rule on without scrollback, each with a recommendation,
per the working doctrine. The README's owner-decision list is already in
that form, so the sessions follow its groups:

| Session | Rules on | Approximate rulings | Unblocks |
|---|---|---|---|
| S1 | this plan's open questions (below) and P1's decisions 42-61, 81 | 22 | the P1 lane; P2's independent members |
| S2 | P2's decisions 36-41, 94, 95 | 8 | the P2 lane |
| S3 | P3's decisions 1-14 | 14 | the check-then-sweep series; P4 and P5 rosters |
| S4 | P6's decisions 15-35 | 21 | the API lane |
| S5 | P4's decisions 73-78, 80-93 | 20 | the remaining pattern sweeps |
| S6 | P7's decisions 62-72 | 11 | the meters, then the deletions |
| S7 | P8's decisions 96-102 and any reopened rulings | 7 | closing the ledger |

Lanes run between sessions and Finch reviews their diffs as they land;
lane roster approvals ride along with the session that unblocks them.
Ten owner-gated entries sit under no README decision (api-audit-1,
tree-core-2, api-core-1, api-core-30, link-6, link-5, remote-codec-19,
swarm-example-3, tests-common-6, materialized-22; `awk -F'\t'
'$6=="yes" && $9==""'` lists them); they join S4 or the relevant lane
roster rather than getting a session of their own.

## How the two of us work it

Finch's attention is the scarce resource, so it goes to rulings and diffs
and nothing else; the coordinator (Claude) carries everything between,
and the two are pipelined so ruling time overlaps agent compute.

1. **Rule by exception where the review converged.** Most owner decisions
   carry one recommendation and reopen nothing. Those arrive as a
   numbered block, one re-grounded paragraph per item with the
   recommendation first, and Finch flags only the items he wants
   changed; silence on an item means the recommendation stands, recorded
   as a ruling citing the block. An explicit affirmative answer is
   required for any item that reopens a recorded ruling (R2, B2, the
   snapshot re-accept class), touches public API or wire bytes, or where
   two documents disagree.
2. **Structured prompts at the console, a written block otherwise.** At
   the console, one interview question per real decision, options with
   the recommendation first, "Other" for an amendment. Away, the same
   block goes in the message and the coordinator stops; the answer
   arrives when Finch returns. Either way the ruling lands in
   `rulings.md` in the turn it is given, and the ledger rows move with
   it.
3. **Sessions pipeline against lanes.** While Finch rules on session N+1,
   lane N runs in worktrees in the background. Lane briefs are the
   coordinator's to write (base SHA, roster from the ledger, each entry's
   resolution and acceptance verbatim, the ordering hazards below, the
   standard ground rules) and Finch reads one only if he wants to.
4. **Finch reviews artifacts, not reports.** A landed lane is
   cherry-picked onto a clean review branch, each entry's acceptance is
   verified against the tree by the coordinator, and the walkthrough is
   ordered by risk: entries where the agent deviated from the stated
   resolution first, mechanical sweeps last. In narration mode, one file
   at a time in the editor; asynchronously, the branch, the SHA, and a
   per-entry evidence table. Merge to main only at Finch's word.
5. **Never a question to Finch.** Nits, grep remainders, ledger
   bookkeeping, agent supervision, seed files, gate runs. These surface
   as outcomes in the diff and in `ledger.py summary` at each session's
   end.
6. **Always a stop.** Anything that would move a snapshot, change a
   public signature the plan did not already name, or contradict a prior
   ruling. A lane agent that meets one reports it up, the coordinator
   reports it to Finch, and the entry stays open until ruled.

## Dependencies and hazards

These are the orderings a lane brief must state, because a literal agent
handed the entry alone would land it in the wrong order.

- Decision 81 before remote-capture-atlas-13: the renderer fix moves
  snapshots under a re-accept class whose witness cannot be produced as
  worded.
- The census floor (conformance-28) before any resizing of `LOCAL_BUDGET`
  and before conformance-25, -31, -24, whose faults the suite today
  convicts only incidentally.
- streaming-tests-11 before suite-economics-7 (the capacity test's width).
- tests-bookmark-12 before tests-bookmark-9.
- The harness lane before the tests lane; `tests/common`'s shared drivers
  before any suite's consolidation entry.
- P3's lints before P6: they enumerate the API work.
- mirror-common-10 (`Preamble::decode` over the fixed array) reopens
  ruling R2 on a new fact; the brief names the ruling and the commit
  states the reopening.
- remote-codec-24 reopens ruling B2 (decision 62); same discipline.
- link-28 before any router counters (decision 22's last clause).
- tree-core-27 before the `send_all`-under-lock measurement (decision 39).
- module-graph-2 before `tokio-stream` leaves the manifest.
- remote-proxy-7 before decision 98's premise bundle.
- Wire bytes: nothing in P1-P5 should move the wire. Any snapshot
  movement outside the renderer class is a finding, not a re-accept.
- Anchors: the tree outside `.agent-notes/` is byte-identical to the
  reviewed commit `9e5784fb` at the time of writing (verified by `git
  diff --stat`), so every line anchor in the documents holds. Lanes that
  land will move anchors for later lanes; each brief names its base SHA
  and the agent re-anchors from the entry's quoted evidence, not its
  line numbers.

## What the review left unsettled, and which lane measures it

The README's Limits name items the review traced but did not run. Each
belongs to a lane rather than a ruling:

- materialized-27's survival under the whole committed suite: the P1 lane
  runs the inverted verdict against the full suite before landing the
  discriminating test.
- link-28 against a live TCP pool: the P2 lane, if decision 36 rules
  pooling first-class.
- "attempt 1581" under SHA3-256: the P7 geometry-fixture entry
  (decision 66) recomputes it as an executable constant.
- The compile cost of `tests/common` forty-two times: the P7 layout entry
  (decision 71) measures before folding.
- Whether `future_size`'s three tests pass: the first P1 run answers it.
- `bookmark_causality`'s nondeterminism under unbiased `select!`: the P1
  causality-harness lane threads a seeded RNG (decision 49) and reports
  whether the schedule is then deterministic.

## Open questions for Finch

Numbered so they can be ruled on as a block; each carries a recommendation.

1. **The record's form.** Recommendation: the ledger and rulings file under
   `triage/`, with the topic documents left untouched. The CBOR review's
   precedent amended the packet in place as rulings landed; at 995
   entries that would make the documents unreadable as evidence, and the
   ledger gives a mechanical completeness check the in-place form cannot.
   The alternative is to add a one-line disposition under each entry
   heading in the topic documents, generated from the ledger.
2. **Delegation tier for lows and nits.** Recommendation: approve lane
   rosters and review diffs; rule individually only on highs, owner-gated
   entries, and anything an agent flags. The alternative, ruling on each
   of the 822 low and nit entries, is the exhaustive reviewer's instinct,
   and the diff review preserves it at the artifact rather than the list.
3. **Phase order.** Recommendation: instruments first (P1), with P2's
   three harness-independent members (deps-3, the clamp, async-hazards-3's
   documentation half) landing in parallel. The alternative, production
   correctness first, lands fixes under harnesses that read failure as
   success.
4. **Where deferrals live.** Recommendation: design proposals in
   `design/` when they reshape a module (decisions 94, 96, 98), the
   shadow tracker for everything else deferred past this triage, and the
   ruling names which. The alternative is a `deferred.md` in this note,
   which is a bin by another name.
5. **Whether this plan links from the review README.** Recommendation:
   one line under "Reading the note" pointing here and to `triage/`, so a
   reader of the review finds its disposition. Not done, pending your
   word, since the README is the review's own record.
