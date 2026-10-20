<!-- CAVEAT LECTOR: written by Claude (Opus 5.5), agent audit-rank-fuel, for the before audit at e0956b25d. -->

# Coverage record: rank operations against their time bounds

Revision: `e0956b25d`, the tip of `fix/suanpan-uniform-amortization`, in
`/Users/oxide/src/rumors-slot-56`. The worktree is detached and clean at
handoff. Everything ran on the Mac, under `nice -n 10` with `-j 8` and 8 test
threads, because ox-east-1 was down.

## What is established, and how

| Claim | How | Evidence |
|---|---|---|
| The products in the rank sweep total `O(M(n))` under the backend model | Every `add_product` operand pair logged and modelled with `num-bigint` 0.4.8's `mac3` dispatch, across 10 family ladders | `fz-rank.tsv`, `dense-suffix.tsv`, `wide-arming.tsv`, `attrib.tsv`, `arming.tsv`; the ratio to one full-width product stays below 0.28 with no upward trend |
| The product model can fail | Leftmost-split mutant in `Aggregate::split`, reverted with the diff verified identical | `mutant-split.tsv`: the model doubles per doubling and passes `M(n)` at `k = 256` |
| `rank`, `distance`, and `lag` refill a zero prefix per freeze and per deferral, quadratically in total | Fill counter on `Digits::add_at`'s `resize`, attributed per site | `fz-rank.tsv`, `fz-pair.tsv`, `attrib.tsv` |
| The refill is the whole of the scope's superlinear fuel | The scope's fuel per byte equals 913 plus 8 × fill per byte on every rung; my wasm fuel reproduces the scope within 0.2% | `fuel.tsv` |
| The repair candidate removes it and breaks nothing measured | Prototype: fill, allocation volume and fuel flat; `suanpan` 94/95, the one failure the intended pin move; `before` measure tests 90/90 | `fixproto.tsv`, `fuel.tsv`, `fixproto-suanpan.log`, `fixproto-before.log` |
| Allocation volume witnesses each refill site independently | `alloc_meter` at base, with only the segment site repaired, and with both repaired | `scratch-fill-{base,segonly,full}.log` |
| `rank_sum` scales by `Θ(k log R)` | Tree-operation counter and fuel on `comb-rank` and `dense-suffix` | `ranksum-tree.tsv`, `fuel.tsv` |
| `rank_sum` has no other superlinear term | Touch, fill, allocation volume and peak flat on `freeze-parade`, `dense-suffix`, `wide-arming`, and `comb-rank` | `ranksum-tree.tsv`, `dense-suffix.tsv`, `wide-arming.tsv` |

## Instrument inventory

None of these is committed anywhere. Each lives in the scratch directory and
is evidence, not a deliverable.

1. **`rank_fuel_probe.rs` with `probe-only.patch`.**
   - *What it is.* A `before` example: copy it into
     `crates/before/examples/` and apply the patch. The patch adds `suanpan`
     counters `PROBE_FILLED`, `PROBE_FILL_SEGMENT`, `PROBE_FILL_DEFERRED`,
     `PROBE_TREE_OPS`, `PROBE_TREE_LOG` and `PROBE_TREE_MAX` under the
     `touch-meter` feature. It also adds a `before` product log
     `PROBE_PRODUCTS` under `meter`, exposed as
     `testing::meter::probe_take_products`.
   - *What it checks.* It reports, per operation and size: touches, zero-fill
     digits by site, allocation volume and peak through `alloc_meter`,
     zero-range tree operations and depth, and every product's operand
     widths summed under the `mac3` cost model. It compares the sum against
     one full-width product under the same model.
   - *Oracle.* The cost model and one full-width product. There is no
     semantic oracle, since this instrument measures cost only.
   - *Reach.* `freeze-parade` up to `k = 16,384` (1.6 MB), `dense-suffix` to
     `p = 4,096` (0.98 MB), `wide-arming` to 8,192, `plateau-puncture`,
     `freeze-pos`, `promo-rearm`, `lone-freeze`, `weight-comb`, and
     `arming-train` with `w = g = 32` to `n = 2,048` (1.5 MB) and with
     `n = w = g` to 64. It also includes `comb-rank`, a new adversary for
     `Sum`, to `m = 65,536`. The committed bands use one doubling at
     `FZ(512)`; the board uses its own scales.
   - *Calibration.* Fill: the base reading is positive and the repair
     prototype flattens it. Products: the split mutant above. Tree: the
     `comb-rank` construction is the positive control, and `freeze-parade`
     the negative one (no tree). The committed suite catches the refill with
     none of its instruments: the touch band and the board's peak heap stay
     flat.
2. **`fuelprobe/`.**
   - *What it is.* A wasm32 guest and Wasmtime 48.0.5 host in a detached
     scratch workspace. It reuses the `wasm32-pins` lockfile and builds
     offline.
   - *What it checks.* Fuel spent inside one measured call. The guest builds
     its input in a separate exported call, and each reading uses a fresh
     instance.
   - *Reach.* `freeze-parade` `rank` and `distance`, `comb-rank` `Sum`,
     `dense-suffix` `rank` and `Sum`.
   - *Calibration.* It reproduces the scope's base readings within 0.2%, and
     the fixed guest reads flat.
3. **`scratch_rank_fill.rs`.**
   - *What it is.* A `before` integration test that prints `alloc_meter`
     volume across one doubling for `FZ` `rank`, `distance` and `lag`, and
     for `DS` `rank` at two scale pairs.
   - *Use.* It is the prototype of the demonstrator's bands, and gives their
     exact base numbers.

Patches: `probe-only.patch` (counters only), `fix-only.patch` (the repair
candidate), `seg-only.patch` (the segment-site half of it), and
`probe-plus-fixproto.patch` (both together).

## Blind spots that remain

- I ran neither the board nor the landing check: the box was down, and I
  committed nothing that would need them.
- I did not examine operations outside the rank family for the zero-range
  tree factor (observation 3).
- I did not prototype `Sum` option 2's repair, and I did not try to
  construct the residual `O(k log R)` case through distinct shift gaps.
- Products: the conclusion rests on 10 ladders and the reduction's
  data-weighted balance argument. I constructed no input that pushes the
  products past one full-width product, and none of my attempts approached
  it.
- My fuel readings drove only `rank`, `distance` and `Sum`. The `ranked_*`
  operations are inferred to share the refill (observation 2).

## How to resume

Read `NOTES.md`. To reproduce any table, apply `probe-only.patch` and copy
`rank_fuel_probe.rs` into `crates/before/examples/`. After use, reverse with
`git apply -R` and delete the example, then confirm `git status --short` is
empty. The commands are in `defect-rank-refill.md` under "Reproduction".

Build directories to retire:

- `/Volumes/forge/build/ee/0b6b3d09301937`, the worktree's
- `/Volumes/forge/build/7d/5f64abf5a03aae`, the fuel probe's
- `fuelprobe/target`, in the scratch directory
