# Instrument finding: the `count_display × heap` worst-case pin depends on the target architecture and, on its own platform, rests on a 0.1% margin

Lane L8, auditor-l8. Base 58285ca5 (tree identical to baseline 455e97de outside
`.agent-notes/`).

## Verdict

The baseline's open question asked whether the `count_display × heap` ranking
difference between ox-east-1 and the pin's machine comes from a
platform-dependent heap reading or a near-tie. It is both, and the first
effect is large.

1. **The heap reading depends on the target architecture** (verified by
   measurement). `Count`'s `Display` (`crates/before/src/count.rs:264-268`)
   delegates to `num_bigint::BigUint`'s `Display`, which calls
   `to_str_radix(10)`. `num-bigint` 0.4.8 chooses that conversion's division
   base with a compile-time constant:

   ```rust
   // num-bigint-0.4.8/src/biguint/division.rs:15
   pub(super) const FAST_DIV_WIDE: bool = cfg!(any(target_arch = "x86", target_arch = "x86_64"));
   ```

   `to_radix_digits_le` (`src/biguint/convert.rs:677-741`) seeds its chain of
   divide-and-conquer bases (taken for counts of 32 limbs or more) from
   `10^19` when the constant is true and from `10^9` otherwise, so the chain's
   sizes, and the transient heap of the conversion, differ between x86_64 and
   every other architecture.
2. **On the non-x86 setting, the pinned argmax wins by about 0.1%** (inferred
   from the probe below, not measured on the Mac): `hugeleaf` reads 4.230
   bytes per unit against 4.227 for the 2,000-bit counts of `pure-comb` and
   `memo-comb`.

The board's ceilings are unaffected: every `count_display` cell is green on
the box. The failing artifact is the `worst-cases-pin` leg.

## Evidence

### The probe

A detached workspace on `explore/l8-adequacy` (`l8/nbprobe/`) formats
`BigUint` values of chosen bit widths through a byte-counting `fmt::Write`
sink under `peak_alloc::PeakAlloc` (the board's allocator, which counts
requested layout sizes and so is itself platform-neutral), and reports peak
heap per unit of the board's `count_display` denominator (count bytes plus
decimal digits). It is built twice: `stock` against crates.io `num-bigint`
0.4.8, and `patched` against a vendored copy whose only change is
`FAST_DIV_WIDE = false`, which is the value every non-x86 target compiles.

Command (exit 0 for both builds and runs):

```
on-illumos.sh <worktree> 'R=$HOME/src/rumors-audit-l8-adequacy; for v in stock patched; do (cd $R/l8/nbprobe/$v && CARGO_TARGET_DIR=$R/target/l8/nbprobe-$v nice -n 10 cargo build --release -j 4 -q && ... ./nbprobe-$v > $R/target/l8/nbprobe-$v.txt); done'
```

Full table: `nbprobe-compare.txt` in this scratch directory. The rows at the
families' widths:

| bits | n_io | x86 heap | x86 per unit | non-x86 heap | non-x86 per unit | board family (scale, sample) |
|---:|---:|---:|---:|---:|---:|---|
| 1,000 | 426 | 494 | 1.160 | 494 | 1.160 | pure-comb, memo-comb (small sample) |
| 2,000 | 852 | 5,426 | **6.369** | 3,601 | **4.227** | pure-comb (default), memo-comb (acceptance), large sample |
| 32,000 | 13,633 | 44,953 | 3.297 | 57,465 | 4.215 | hugeleaf (default, small) |
| 64,000 | 27,266 | 89,170 | 3.270 | 115,346 | **4.230** | hugeleaf (default, large) |
| 128,000 | 54,532 | 178,612 | 3.275 | 231,084 | 4.238 | hugeleaf (acceptance, small) |
| 256,000 | 109,064 | 364,616 | 3.343 | 462,552 | **4.241** | hugeleaf (acceptance, large) |

The board's reading for a cell is the larger per-unit density of its two
samples.

### The probe reproduces the box's board readings

From the baseline board log (`coordinator/baseline-455e97de-gate-logs/board.log`,
lines 1978-2033 and 7345-7400), default scale: `pure-comb 427->853 heap 6.4/B`,
`hugeleaf 13633->27266 heap 3.3/B`; acceptance scale: `memo-comb 426->852
heap 6.4/B`, `hugeleaf 54532->109064 heap 3.3/B`. The x86 column above gives
6.369 and 3.297 for the same denominators. The live drift lines
(board.log:16212-16213) name `pure-comb` and `memo-comb`, the 2,000-bit
families, as the x86 argmax.

### The x86 top is itself a near-tie cluster

At the default scale `reveal-hifloor`, `reveal-comb`, `pure-comb`, and
`nested-wide` all render 6.4/B; at the acceptance scale `mirror-wide`,
`memo-comb`, and `memo-chain` do. Their large samples all sit at or near 2,000
bits, the width at which `num-bigint` first takes the divide-and-conquer path
(32 limbs) and allocates a 32-slot `Vec` of bases. Which of them is the exact
argmax depends on the low bits of each family's count.

## The contract clauses in tension

- `crates/before/src/testing/meter/board/worst.rs:30-39` (`NEAR_TIE_RATIO`):
  "The flag never enters the pin: the pin records the exact deterministic
  argmax, and a flip inside the band is still news worth a look." The pin's
  own platform margin (0.1%) sits far inside the 1.25 band.
- `crates/before/examples/amp_board.rs:44-47`: "Every judged quantity is a
  deterministic counter over state a child owns privately". True for one
  build target; not across targets, which the pin (`WORST_RANKINGS`, no
  target qualifier) assumes.
- `justfile` `_gate-board` and `ci-instruments`; `.github/workflows/ci.yml`
  job `instruments` runs `just ci-instruments` on `ubuntu-latest` (x86_64).
  By this mechanism `worst-cases-pin` should drift there exactly as on the box
  (inferred; I cannot see CI results).

## Scope

Only `count_display` reaches `num-bigint`'s decimal conversion under the board:
`Rank` formats in binary with no division (`rank.rs:1117-1150`), and the serde
rows measure deserialization. Within `before` and `suanpan`, the other
platform-sensitive path I found is `party/forks.rs:210-211` (`usize::BITS`),
which agrees across 64-bit hosts. The scan meter records bits read, not
word-sized chunks, and suanpan has no architecture-dependent code.

## Routes (owner's call; this is a design question, not a code defect)

1. Pin per architecture, or drop `count_display × heap` from the pin with a
   stated reason (the reading is a dependency's platform-specific internals).
2. Let the pin accept any family inside `NEAR_TIE_RATIO` of the pinned
   family's reading, reversing the documented policy above.
3. Make `Count`'s `Display` independent of `num-bigint`'s conversion (for
   example, a fixed-base chunked conversion owned by the crate). This removes
   the platform dependence but not the near-tie.

Whatever the route, the board's documentation should say which quantities are
target-dependent.
