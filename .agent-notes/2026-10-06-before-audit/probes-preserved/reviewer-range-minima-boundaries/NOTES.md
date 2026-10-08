# reviewer-range-minima-boundaries: resumption notes

Slot /Users/oxide/src/rumors-slot-36, branch audit/range-minima-near-boundaries,
tip de3dc708 (signed G), base 1745d377 ancestor. Clean at start.

## Established
- Witness semantics verified by hand against oracle + prescan/min_ticks code
  (both docs accurate; min_ticks and prescan both reach 106/116 fast paths).
- 7222d49b: RangeMinima input in min_ticks unchanged (advance_height gets raw
  deltas; narrowing affects only HeightPrefixes/contributions). Inferred: min_ticks
  half still reaches. Not run.
- Campaign logs are fail-fast: cannot tell sole killer. Extracted first-failing
  test per mutant: campaign-rm-kills.txt.

## Live experiment (MUST REVERT)
- tests.rs has appended block (experiment-block.rs): old property copy + 2 tick-only
  witnesses. Revert: python3 block.py revert; confirm `git -C slot diff` empty.
- Box run 1 (background, run1.log): cargo-mutants over range_minima files, filter
  range_minima::tests, --no-fail-fast, seed 8008, output
  ~/src/rumors-slot-36/target/reviewer-mutants1.

## Next
- Parse per-mutant failing tests: old-prop vs new-prop kills; 106/116 witnesses + tick-only.
- Run 2: follow-ups (dilution candidates multi-seed, or merge-with-7222d49b check).
- Run 3 left for :106 arm measurement if needed.

## Update (run 1 in flight)
- sim.py/simdrive.py: Python model of kernel+generator. Calibrated: branch 106 ~86%/run
  (builder measured 80%), 116 100%; old strategy 0 for both. Close bound is the limiter:
  closes 0..3 -> 106 6.2%/case; swarm close bound {3,5} -> 3.2%/case (~100%/run),
  keeps 6.7 mid-case retirements/case (old 12.1).
- swarm-block.rs: reviewer copy of property with swarm close bound (apply via block.py apply swarm-block.rs).
- Plan run 2: merge 7222d49b (no-commit) + blocks; unmutated seed loop for swarm; cargo-mutants --re 106|116 witnesses.
- Plan run 3: tip + mutant 106 swap (builder's swap.py logic) + swarm block; seeds 1..20 current vs swarm.
- boundary.rs:9 "Width leading-digit comparisons" confirmed on main, outside branch.

## Run 1 done (run1.log, run1-kills.txt)
- 180 mutants: 116 caught, 26 missed, 38 unviable under range_minima::tests filter, seed 8008.
- OLD-ONLY kills: none. NEW-ONLY: 7 (106, 116, anchor 207, boundary 85, 90, propagate_drop 287 x2).
- 106/116: witness (min_ticks msg first), property (successes: 0 => committed seed), tick-only all fail.
- 46:44 >=, 113:42, 113:75 * survive; reading agrees cost-only.

## Run 2 in flight (MUST UNDO): slot has `git merge --no-commit 7222d49b` + experiment-block + swarm-block.
Undo: block.py revert swarm-block.rs; block.py revert; git -C slot merge --abort; git status/diff empty.

## Run 2 done: merged tree, both witnesses fail on min_ticks + tick-only under 106/116; baseline ok;
   swarm + branch property pass 20 unmutated seeds each. Merge aborted, tree clean at de3dc708.
## Run 3 in flight (MUST UNDO): swap.py 106 apply + block.py apply swarm-block.rs.
Undo: python3 swap.py 106 revert; python3 block.py revert swarm-block.rs; git diff empty.
