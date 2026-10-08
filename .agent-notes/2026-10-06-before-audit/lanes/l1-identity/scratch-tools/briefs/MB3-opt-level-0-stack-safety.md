# Machinery brief MB3: run the stack-safety tests at `opt-level = 0`

## The failure class it catches

A *tail*-recursive library walk. The workspace builds `before` and `suanpan`
at `opt-level = 2` even in the dev profile (`Cargo.toml:189-193`,
`[profile.dev.package.before]`), and LLVM eliminates self tail calls at any
nonzero optimization level. Every committed stack-safety test therefore
proves iterativity only for the optimized build. A downstream crate that
compiles `before` under its own default dev profile (`opt-level = 0`) gets the
recursion back and overflows on deep inputs, so the crate's hard rule ("No
library traversal recurses on tree depth") is not actually proven.

Verified by calibration:

- Mutant M25 rewrites `PartyRegionReader::descend`
  (`src/party/io/regions.rs:80-97`) from its `loop` into a self tail call.
  At the workspace profile it passes everything: the existing suite (686 run,
  686 pass, `calib-run-6.log`) and the explore deep probe (`calib-run-7.log`).
- Built with `--config profile.dev.package.before.opt-level=0`, the existing
  `clock::tests::deep_tree_query_and_causal_stack_safety` aborts on it
  (`run-m25-opt0-existing.log`, verbatim: "thread
  'clock::tests::deep_tree_query_and_causal_stack_safety' (2) has overflowed
  its stack"), and so does the explore deep probe (`run-m25-opt0.log`).
- The unmutated tree passes all 34 deep and stack tests at `opt-level = 0`
  (`run-opt0-deep.log`), so today's walks are genuinely iterative: this is a
  gap in the proof, not a present defect.

## Which instrument it extends

The same stack-safety tests, run under a second build configuration. No new
test code is needed.

## Construction

A justfile recipe, wired into `gate-streams`, that builds into its own target
directory so it never thrashes the main build:

```
cargo nextest run -p before --all-features --locked \
    --target-dir target/stack-opt0 \
    --config profile.dev.package.before.opt-level=0 \
    --config profile.dev.package.suanpan.opt-level=0 \
    -E 'test(/stack_safety|deep_/)'
```

Pick the filter so it names the deep tests deliberately (renaming them to a
shared suffix such as `_stack_safety` makes the filter exact and keeps new
deep tests from being missed). The recipe's comment should say why it exists:
the dev profile's `opt-level = 2` hides tail recursion.

## Cost

On ox-east-1 the `opt-level = 0` build of the `before` test targets took 59 s
from cold for the lib tests plus one integration target
(`run-opt0-deep.log`, "Finished ... in 58.68s"), and the filtered run took
7 s. Disk: one more `before` test build directory.

## Calibration the builder must reproduce

1. The recipe passes on the base tree.
2. Apply M25 (exact text in the auditor's `muts4.py`): the recipe fails with
   a stack overflow in `deep_tree_query_and_causal_stack_safety`, while
   `just test` still passes. Revert; `git diff` empty.
