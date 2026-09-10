# A smaller plan for the Rumors review

The [checklist](checklist.md) owns scope, status, dependencies, and the current
queue. The [inventory](inventory.md) records the source review and prepared
branch evidence. Neither certifies old fixes.

## 1. Choosing a batch

Keep one implementation active and at most two next candidates. The numbered
checklist groups organize outcomes; they do not prescribe patch size or require
finishing every group in numerical order. Work in small batches that can be
understood, edited, and merged separately.

Choose by correctness risk, actual dependencies, and owner priorities. When
eligible work has comparable priority, prefer finishing a partly completed
area over opening another. At each handoff, name the next outcome and explain
why it comes next; distinguish required prerequisites from a convenient
sequence. If an earlier group remains unfinished, say what is still open.
Report completed outcomes, never imply that one merged batch finished its group.

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
- **Document every definition.** Give each function, type, trait, constant,
  and other definition a brief purpose doc comment, including private items,
  trait implementations, and test helpers. Simplification must preserve this
  minimum. Delete redundant inline narration, duplicated contracts, and stale
  claims; retain non-obvious invariants and necessary reasoning.
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

Keep Rumors runtime-independent. Runtime-specific optimizations do not belong
in production code; Tokio runtime support is confined to tests and test helpers.

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

For every external Rumors API change, update and test Sush's
`codex/rumors-compat` branch alongside the Rumors batch. Keep it atop the latest
`locker`, preserving its compatibility work when rebasing. Simplify Sush code
and comments wherever the new API permits. Use the local dependency override
while Rumors changes are unmerged; advance Sush's Git pin and lockfile when
the corresponding Rumors revision is available. Before final review, verify
Sush against the final revision without the override. The user reviews and
approves the completed Sush branch at the end of this refactor effort.
See [Sush setup](sush-pooling.md) for the worktree and override.

## 3. Tracking without another system

Keep the [checklist](checklist.md) as a completion index. It uniquely records:

- Each outcome and its assigned finding, ruling, and later-report IDs.
- Dependencies that affect sequencing.
- Open, working, ready-for-review, or completed status; completion means a
  verified merge with its landing commit, or an explicit non-code disposition.
- One active branch and review base, plus the next candidates.

Do not maintain summary counts of findings, outcomes, tests, or lines. Derive
them from their source only when needed.

Use one short outcome line and one source-reference line per item. Compact
consecutive IDs into inclusive ranges. Add a brief blocker or remaining scope
only when it cannot be recovered from the sources. Split a partly completed
outcome rather than checking off its unfinished work.

Put a blocking dependency beside the affected outcome, naming the prerequisite
outcome or landing commit. Use a group-level dependency only when its scope is
clear. Mark changes that must be designed together as coupled work, rather
than inventing an order between them. A preferred next batch is not a dependency.

Update status in place. After merge, replace active-batch details with the
landing commit. Do not accumulate implementation summaries, test counts,
timings, review transcripts, cleanup receipts, or restated rulings. Git already
records implementation; source reports and amended rulings hold the rationale;
this README holds the workflow. Find a branch's worktree through Git. Keep the
queue only in the checklist, without a second copy here.

Delete duplication rather than moving it into another document. A separate
note is justified only for information with no other durable home, such as
an unresolved design decision or an external consumer experiment; link to it
instead of copying it. Do not create a note for every batch by default.

Add newly discovered work to this same checklist. Preserve source assignments
when regrouping or shortening it; before declaring the effort complete, check
them again and verify every remaining outcome against the integrated code.
Old ledger SHAs are leads, not completion evidence. No tracking tool, source
scanner, gate, or test is needed to enforce this document's style.

## 4. Review and edit in Zed

Use a dedicated worktree for each batch. Open it in a separate Zed window
with `zed -n`, then run `git: compare with branch` and choose `main`.
For a deliberately stacked batch, choose its recorded parent instead.
This shows the whole branch change, including uncommitted edits.

Open this diff when starting each batch and keep it open during implementation
so the user can review continuously. Read saved edits before changing the same
code, and preserve them. Note to the user if anything they change introduces
correctness issues, or if you would do it differently, and correct towards the
intent you infer, but always with notification of such action. At the final
review handoff, stop editing until the user responds.

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

After merging, close the corresponding Zed window, remove the clean, merged
worktree, and delete its Cargo build artifacts under `/Volumes/forge/`.
Before removing the worktree, use `cargo metadata` to identify its
`build_directory`, including those of nested workspaces. Clean up those
specific directories; keep shared caches and other checkouts’ artifacts.
Keep the ongoing Sush compatibility worktree and its build artifacts across
Rumors batches, updating its local override before retiring a Rumors worktree.

## 5. Scope boundaries

The checklist includes public diagnostics, observability, the file-backed
Bookmark, structural experiments, module cleanup and publication preparation;
these have not disappeared behind the early correctness work. Publication
preparation does not authorize publishing. Persistent message-storage
redesign remains separate, although its proposal may inform affected API
boundaries.

Before-specific generator and fuel-band work matters where it blocks an
actual workspace check. It does not make the entire before triage a
prerequisite for Rumors fixes. Maintaining Sush compatibility is part of this
effort; merging or publishing the Sush branch awaits its final review.
