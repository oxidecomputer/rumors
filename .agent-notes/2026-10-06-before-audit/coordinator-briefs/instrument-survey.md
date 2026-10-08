<!-- CAVEAT LECTOR: a survey brief written by the coordinator (Claude Opus 5.5) for the audit's final phase. -->

# Brief: the instrument survey

Kind: a read-mostly survey producing one ranked document. No production code
changes. Proposal branches come afterwards, from the coordinator, for the
top-ranked candidates only.

## The goal, and the rule that governs it

The owner set this survey as the audit's last phase (`instruments.md`, "The
instrument survey"). The owner has since cautioned: do not spiral into test
infrastructure built to catch imaginary bugs made up on the fly; it leads to
unbounded work. The survey therefore ranks every candidate by one question:

> Name a concrete failure (a defect, a false pass, a false failure, or a
> regression) that this candidate catches and the committed instruments
> miss, and point at where that failure was constructed or observed.

- A candidate with a constructed or observed failure is **eligible** for a
  proposal branch.
- A candidate whose value is reach, independence, or "extra safety" without a
  named failure is **listed, not proposed**. Record what it reaches and why no
  failure is known, so the owner can decide.

Where `instruments.md`'s seven areas and this rule disagree, the rule wins;
say where it cut something.

## Inputs

1. **The coordinator's collected candidates** (`STATE.md`, "Final phase",
   item 1). Each already names its failure; verify each against the tree at
   `main` and the cited evidence. Highlights:
   - `num-bigint` work is unmetered: the `min_ticks` records fix was quadratic
     in big-integer limbs with every touch and scan reading identical
     (`lanes/l3-events/round-1/defect-D1-min-ticks-heap.md`, last section;
     probe at the session scratchpad `reviewer-min-ticks-final/`). This one
     depends on the owner's question 87; survey it, and note the dependency.
   - Heap exponent fits rounded to 0.01 tripped two stop rules on fixed-part
     changes, and missed a 40% flat rise (`wide-arming`); exact Δheap(n)
     reporting is the candidate.
   - `PeakAlloc` false failures from libtest's main thread.
   - `STOPPING_DIFF_BAND` admitted a one-touch-per-hop rise.
   - Board ceilings drifting from their stated rule (open question 65 rules
     on the rule itself).
   - The worst-case pin is unchecked while `amp-board-acceptance` is red.
   - No gate leg builds docs for the detached wasm32-pins workspace.
   - `bounded_corpus_manifest_snapshot` false failures under load.
   - Five C-class mutation survivors invisible to every counting meter
     (`lanes/l8-adequacy/round-2/addendum-bits-party/`).
   - `RED_ZONE`'s rule unchecked.
2. **The explore-branch inventory** (`instruments.md`, "Inventory"), one row
   per lane. Read each branch's instruments at the cited commit. For each,
   classify eligible or listed by the rule above. The lanes' own mutant
   results are the evidence: an instrument that killed a mutant the
   committed suite missed (L1's M19, M20, M26; L3's M6, M8, M15, M19) is
   eligible only if that mutant is still missed at `main` plus the ready
   branches that target it (check #40, #81, and the L3 co-generation branch
   if landed or ready).
3. **The mutation campaign** (`lanes/l8-adequacy/round-2/survivors/INDEX.md`
   and the addendum): which survivor classes no instrument covers.

Do not re-run the mutation campaign, run survivors against every explore
instrument, or generate new mutants. The audit's scope ruling forbids new
mutant generation, and the owner's caution forbids open-ended search.

## Output

One document, `instrument-survey.md`, in your scratch directory:

1. **Eligible candidates, ranked.** For each:
   - the failure, with its evidence;
   - the smallest change that would catch it, extending a shared instrument
     (`testing::generators`, `testing::laws`, the board, the oracles,
     `justfile` legs) rather than a parallel harness;
   - its cost under nextest's time limit;
   - its dependencies on unlanded branches or open questions.
2. **Listed candidates.** Name, what it reaches or checks, and why no failure
   is known. One or two lines each.
3. **The verification map.** Which entries the validation index and the
   crates' `# Testing` sections lack (the index has no entry for
   `wasm32-pins` or any suanpan instrument). List them; don't write them.
4. **Where the rule cut `instruments.md`'s scope**, briefly.

Write for the owner, who reads it without the chat: plain sentences, no
coined terms, every claim marked verified (you checked it) or reported (an
agent's record says so).

## Ground rules

- Base: `main` at the coordinator's current SHA (check `git -C
  /Users/oxide/src/rumors log -1`). Read explore branches with `git show` or
  `git -C ... worktree`-free reads; do not check them out in the main
  worktree. Never use EnterWorktree.
- Box runs only where a claim needs verifying and reading cannot settle it:
  at most four, via `on-illumos.sh` on a scratch worktree of your own under
  `/Users/oxide/src/` named `rumors-survey`, removed with plain `git worktree
  remove` at the end.
- Scratch: `<session scratchpad>/surveyor/`.
- Make no commits; the coordinator commits the survey.
