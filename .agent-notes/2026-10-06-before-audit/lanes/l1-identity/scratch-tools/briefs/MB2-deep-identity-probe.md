# Machinery brief MB2: drive every identity operation at depth

## The failure class it catches

The crate's hard rule (`crates/before/AGENTS.md`): "No library traversal
recurses on tree depth: deep walks are iterative." The committed proof is the
depth-100,000 stack-safety tests. For identity, they drive encode/decode,
`fork`, `join`, `sync`, and `is_disjoint` (`deep_tree_stack_safety`,
`src/clock/tests.rs:558-642`), projection through a deep id
(`deep_tree_query_and_causal_stack_safety`), and `without`
(`without_constructed`, `src/party/tests.rs:488-614`).

No committed test calls these public operations at depth: `Party::forks`
and `Clock::forks`, the consuming array split, `join_all`, `sync_all`,
`covers`, `Party::shape`, and `Clock::shape`. Some share walks the deep tests
already drive (`covers` shares `is_disjoint`'s comparison walk; the shape
walks share the region reader that projection drives; the array split,
`join_all`, and `sync_all` reduce to `fork` and `join`). Three walks have no
deep coverage at all, and they are the fork iterators' own: the plan's
spatial path (`SharePath`, `src/party/forks.rs:34-108`), share construction
(`PartyReader::select_path`, `src/party/fork.rs:31-69`), and the removal walk
(`src/party/fork/remove.rs`), the most stateful identity walk in the crate.

Verified by calibration (`calib-run-6.log`): mutant M26 makes the removal
walk's descent recursive (the `for right in path { removal.descend(..) }` loop
in `Removal::run` becomes a self-recursive helper over the path iterator). The
entire existing suite passes it (`--lib --test meter --test forks_count`: 686
run, 686 pass). The explore probe aborts it with a stack overflow.

## Which instrument it extends

The depth-100,000 stack-safety tests in `src/clock/tests.rs`. Add a sibling
`deep_identity_stack_safety` beside `deep_tree_stack_safety` (same depth, same
impl-only rationale), or extend that test; a sibling keeps each test's list of
driven operations readable.

## Inputs

Build them with the `constructed` helpers in `src/party/tests.rs:272-334`
(move or share them so the clock tests can reach them) plus
`testing::generators::deep_left_spine_party`. Cover *both* spines, because a
walk's descent is often asymmetric: a left-leaning chain stresses descent into
left children, a right-leaning chain stresses the climb back out.

- the left cell `leftmost(D)` and its complement `complement_leftmost(D)`;
- the right cell (a right spine over `full()`) and its complement, the right
  comb `[0, 1 - 2^-D)`: `D - 1` nodes `11` each with an owned left child,
  then `10 00`;
- a zigzag spine alternating left and right unary nodes.

## Operations and assertions

For each input `p`, with `D = 100_000`:

- `covers` both ways against the seed and the complement, and against an
  alias (`true` and `false` outcomes).
- `forks(5)` fully drained, then `join_all` of the shares restores `p`;
  `forks(1000).take(3)` then drop, rejoin restores `p`; one share from a count
  above `u128::MAX`, rejoin restores `p`.
- `let [a, b, c, d, e]: [Party; 5] = p.into()` then `a.join_all([b, c, d, e])`
  equals the original.
- `Clock::forks(4)` children plus the complement's clock, then `sync_all`,
  then `join_all` back to the seed.
- `p.shape().count() == D + 1` for the cells and combs; `Clock::shape` drains
  for a ticked clock over each spine.

The explore implementation is
`crates/before/tests/audit_l1/deep.rs` on `explore/l1-identity` (it uses a
256 KiB thread stack; the committed test can keep the default test stack, as
its siblings do, since depth 100,000 already overflows 2 MiB for any frame of
21 bytes or more).

## Calibration the builder must reproduce

1. Base: the new test passes.
2. Apply M26 (exact text in `muts4.py` in the auditor's scratch, or rewrite
   `Removal::run`'s path loop as a self-recursive helper that is not a tail
   call), run the new test: it aborts with a stack overflow. Revert;
   `git diff` empty.

Note the interaction with MB3: a *tail*-recursive rewrite is turned into a
loop by the workspace's `opt-level = 2` for `before`, so this probe catches it
only under MB3's `opt-level = 0` leg.
