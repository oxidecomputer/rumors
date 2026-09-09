# A smaller plan for the Rumors review

The [checklist](checklist.md) is the work record. It accounts for **all 995
original findings, 171 rulings, and 55 later reports**, grouped into 116
outcomes with prerequisites. A one-off comparison found no missing or
multiply assigned original findings or later reports. This establishes scope;
it does not certify old fixes or prepared branches.

The [inventory](inventory.md) records the source review and branch evidence.
Both were prepared against `main` at `90e4509d` on 2026-09-09.

## 1. Current queue

| Position | Work | First step |
|---|---|---|
| Active | [Routed pooling](checklist.md#02-routed-connection-pooling) | Recover the stream-level reproducer and assess adapter-owned pooling. |
| Next candidate | [Deep fixtures](checklist.md#03-deep-tree-fixtures-and-reproducible-schedules) | Reproduce the deep-tree failures on current main. |
| Following candidate | [Protocol termination](checklist.md#04-departure-malformed-replies-and-error-preservation) | Reconcile departure and semantic-error handling. |

Keep one implementation active and at most two next candidates described
in detail. Destructor safety landed as `2c77220a` after user review and a
clean gate. The next branch is `codex/routed-pooling`, in
`/Users/oxide/src/rumors/.worktrees/routed-pooling`. The numbered checklist groups are navigational aids, not 27
large patches or a demand to finish every group in numerical order.
Work in small batches that can be understood, edited, and merged separately.

Recover the intended behavior from the findings and amended rulings.
Salvage useful code, counterexamples, and experiments from the prepared
branches; do not merge them wholesale. Current user instructions take
precedence over their implementation patterns and older workflow rules.

## 2. How each change proceeds

Read the affected implementation, its callers, and relevant tests. State
the concrete problem and the behavior the change should provide. Implement
that change on a fresh `codex/` branch from current main, simplify the
surrounding code, verify it, and present the actual diff. Existing rulings
settle answered questions. Bring any new contract or wire-format decision
to the user with a concrete recommendation and evidence.

Every batch must improve the code and explanations it touches, including
surrounding prose whose lines the fix itself would leave unchanged:

- **Understand before rewriting.** Check comments against implementation,
  callers, and tests. Resolve inaccurate claims; do not make an unverified
  explanation merely more fluent.
- **Decide whether a comment is needed.** Delete narration of obvious code,
  duplicated contracts, stale claims, and unnecessary commentary. Clearer
  names or simpler control flow may remove the need for an explanation.
  Preserve non-obvious invariants and necessary reasoning.
- **Write for the reader.** Public rustdoc primarily states contracts:
  behavior, caller obligations, guarantees, outcomes, and relevant costs.
  Explain mechanism only when it helps someone use the API. Private docs
  and internal comments explain the why, how, and where for maintainers.
  Test comments state the behavior and invariant being protected.
- **Respect the abstraction.** Modules explain purpose and boundaries;
  types explain meaning and invariants; local comments explain particular
  implementation choices. Put an explanation where the responsibility
  belongs. Avoid binding a public contract to incidental internal structure.
- **Rewrite for simplicity, concision, and clarity.** Rework whole passages
  when needed. Use ordinary language and a logical reading order; remove
  needless jargon, repetition, tangents, and overwrought phrasing. Keep
  enough detail to teach the reasoning. A vocabulary sweep or word limit
  cannot substitute for judgment.
- **Simplify the implementation too.** Remove obsolete states, wrappers,
  helpers, instruments, and duplicate representations. Improve ownership,
  naming and control flow. Related cleanup needs no pre-existing finding;
  if it becomes substantial, give it a checklist item and a separate diff.

Add tests that catch wrong behavior. Prefer the existing harness and a
focused construction. A source-reading test that demands a comment phrase
or compares two copied rosters should disappear with the duplication.
New instrumentation must justify its complexity by answering a concrete
question; do not inherit it merely because an old branch added it.

Use properties for families of cases. Preserve every regression seed and
the deliberate wire-snapshot discipline. Run appropriate tests while
iterating and get `just gate` fully clean before every code commit, as
AGENTS.md requires. Regenerate READMEs after crate-rustdoc changes. Verify
the integrated result after rebasing; an old passing report is not evidence
for changed code. Notes-only changes need no gate run.

Before review, read the changed code and prose together once more for
accuracy, audience, abstraction, and unnecessary complexity. Explain the
final change, validation, and any remaining decision briefly. Do not produce
another annotated copy of the review history.

## 3. Tracking without another system

Use only the [checklist](checklist.md) for progress. Each outcome has its
source IDs and decisions; later amendments and current instructions govern
implementation. The old ledger remains source material. No new tracking
tool, gate, or test is needed.

An item stays unchecked until its full outcome is verified and merged.
Then record the actual landing commit. If only part is finished, split or
name the remainder. Old receipt SHAs are verification leads, not proof of
completion. Declined, deferred, obsolete and external proposals keep an
explicit disposition; they must not silently become new requirements.

Add newly discovered work to this same checklist. During an active batch,
record its branch, worktree, review base, and whether it is working or ready
for review. At a handoff, report what remains. Before claiming the whole
triage complete, recheck source coverage and every unchecked outcome against
the integrated code.

## 4. Review and edit in Zed

Use a dedicated worktree for each batch. Open it in a separate Zed window
with `zed -n`, then run `git: compare with branch` and choose `main`.
For a deliberately stacked batch, choose its recorded parent instead.
This shows the whole branch change, including uncommitted edits.

Zed's Project Diff edits the underlying files. The user can change the
working side, save, and continue reviewing on that branch. See Zed's
[editable diff documentation](https://zed.dev/docs/git#project-diff) and
[branch comparison action](https://zed.dev/docs/all-actions).

When the batch is ready, present its purpose and test results and open the
worktree for review. Stop editing it; do not rebase it or advance its
comparison base while the user reviews. When it comes back, read and
preserve the saved edits, verify the revised code, and show any further
substantive changes. **Merge only after the user's approval of the final
result.** “lgtm” means approval to merge the reviewed batch and proceed to
the next one, without another confirmation. Preserve saved user edits and
complete the required checks, then commit, merge, update the satisfied
checklist items with the landing commit, and start the next batch.

## 5. Scope boundaries

The checklist includes public diagnostics, observability, the file-backed
Bookmark, structural experiments, module cleanup and publication preparation;
these have not disappeared behind the early correctness work. Publication
preparation does not authorize publishing. Persistent message-storage
redesign remains separate, although its proposal may inform affected API
boundaries.

Before-specific generator and fuel-band work matters where it blocks an
actual workspace check. It does not make the entire before triage a
prerequisite for Rumors fixes. Consumer evidence for the pooling interface
may need reassessment; this plan does not authorize changes to sush.
