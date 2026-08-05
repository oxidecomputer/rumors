# perf-probe: where `tick` and `|` spend their cycles, and what parity would take

Investigation on branch `perf-probe` (base `before-hardening` @ 0967f140),
2026-08-04, stelmaria (M-series, macOS). Instruments: `perf_probe.rs` (op
loops over the bench suite's own corpus recipe, same seed and salts),
`emit_probe.rs` (output-primitive microbenchmarks), `/usr/bin/sample`
call-stack profiles of the op loops, and the criterion medians already on
disk from the owner's suite run (`target/criterion/version_*`).

## Ground truth

Criterion medians (owner's run), impl/oracle ratio:

| n | tick | merge (`\|`) |
|---|------|-------------|
| 8 | 1.81x | 2.87x |
| 32 | 5.96x | 2.27x |
| 128 | 6.53x | 2.22x |
| 512 | 6.33x | 2.23x |
| 2048 | 5.88x | 2.15x |
| 8192 | 5.95x | 2.32x |
| 32768 | 5.66x | — |

The probe harness reproduces the impl-side medians almost exactly (e.g.
merge n=8192: 898µs probe vs 883µs criterion), so its profiles attribute
the right workload. Clock-level join measured at parity with the oracle
(the party-join and disjointness legs cost the oracle proportionally
more); version-level cmp on concurrent operands is ~250ns flat — the
early-exit sweep is orders of magnitude ahead of any full walk. The gap
is specifically the two *emitting* sweeps: tick ~6x, join ~2.2x.

Per-byte framing at n=8192 (version 37.8kbit ≈ 4.7KB, 13.9k leaves):
decode ≈ 22ns/byte; join ≈ 100ns/byte of combined input; tick ≈
250ns/byte. Decode is the existence proof that *reading* these streams
is cheap; the emitting sweeps cost 4–10x more per byte than the
validating sweep on the same bytes.

## Where the cycles go (sample profiles, top-of-stack shares)

**join (n=8192):** ~33% emission machinery (`SkylineBuilder::leaf` 12%,
`bitvec` `push`/`extend_from_bitslice`/`sp_copy_from_bitslice`/`resize`
21%); ~10% allocator (`nanov2_malloc`/`free`/`finish_grow`) plus ~8%
dashu `Repr` clone/drop glue and `UBig`/`Base` arithmetic; ~4% suanpan
accumulator; ~13% sweep logic (`advance_diff`, `LeafCursor::descend`,
`emit::join`); ~9% gamma codec proper (`read_int`, `unary_raw`,
`encode_int`, `unzigzag`); ~2.5% `PopStack`.

**tick (n=8192):** ~19% `bitvec` writes; ~20% allocator; ~12% suanpan
(`fold_and_collapse`, `add_at`, `read_magnitude`, `add_u64`, `reset`);
~9% `SkylineBuilder::leaf`; ~14% fill-walk machinery (`FillWalk::walk`,
watermark `MinStack`, `PopStack`, `LeafWalk::descend`); ~7% codec.

The gamma coding itself — the thing one might presume expensive — is
under 10% in both. The costs are (a) the *currency* the sweeps trade in
and (b) the *primitives* the output is written with.

## The drivers, ranked

**D1. Per-leaf heap code buffers + bit-addressed output writes.** Every
emitted plateau allocates a fresh `BitsMut` for its gamma code
(`gamma_code`), which the builder holds, splices via
`extend_from_bitslice`, and frees. Topology flags go through
`BitVec::push` one bit at a time. Measured on the real workload shape
(emit_probe, 5k leaves, 1–9 bit codes):

- current discipline (alloc code + splice per leaf): **35.0 ns/leaf**
- per-bit pushes only: 7.5 ns/leaf
- word-buffered writer, codes as `(u64, len)` values: **0.88 ns/leaf**
- misaligned 37.5kbit bulk copy: bitvec 4.2µs vs word-shift 0.46µs (9x)

A join at n=8192 emits ~21k plateaus (operands 10.8k + 10.9k leaves) in
883µs — a 41ns/leaf total budget of which the emission primitive alone
is ~35ns. This is the single dominant driver of the join gap and a
large share of tick's.

**D2. Arbitrary-precision currency on the hot path.** Every delta
travels as a `Base` (dashu): `Step` clones a `Base` per boundary,
`zigzag_signed`/`unzigzag` do bignum shifts/adds with temporaries and
drop glue per leaf, `signed_sum` allocates intermediate `Base`s, and the
running difference (and tick's whole watermark web) folds through
suanpan's digit machinery. Per the crate's own `code_study`, 85–93% of
coded values are ≤ 15 — machine-word arithmetic with a spill path would
remove dashu and most suanpan work from the organic path entirely while
preserving the wide/cliff-immune worst case (spill preserves the
existing discipline).

**D3. Per-op allocator traffic.** ~10–20% of samples. Sources: D1's
per-leaf codes (the bulk), suanpan accumulator digit buffers, the
per-op path/left-leaf/lens stacks, output growth. D1+D2 eliminate most;
inline-capacity stacks and presizing handle the rest (matters most at
small n, where fixed overhead sets the ratio).

**D4. Tick's fused walk runs its full generality per node.** Tick costs
2x a join despite the grow branch's output being "one splice": the
fill walk's watermark/route machinery (MinStack boundaries, frame
stacks, per-delta accumulator folds) has a per-leaf constant ~15x
decode's (126ns vs ~9ns per leaf at n=8192). The width-circulation
discipline is what makes adversarial wide inputs linear — but on
narrow organic values it is pure generality tax, and it is exactly the
place a narrow-track fast path (D2) pays off twice. The splice itself
also pays the 9x bit-addressed copy gap (D1's writer fixes that).

## What is *not* the problem

The representation and the sweep architecture. Elias gamma decode via
the word-buffered dsi cursor is cheap (<10%); the overlay-advance
bookkeeping is lean; comparisons early-exit; decode proves the streams
read at ~22ns/byte. Nothing here motivates touching the wire format,
the canonical-form discipline, or the one-sweep operation shapes.

## Parity assessment

- **join (`|`), 2.2x:** D1 alone is worth an estimated 300–400µs of the
  883µs at n=8192; D2/D3 another ~150µs. That lands at or below the
  oracle's 381µs — parity is credible, and *beating* the oracle at
  large n is plausible (the oracle's join allocates Rc nodes per output
  node and chases pointers; the packed sweep's floor — read two
  streams, write one, word ops throughout — is below that).
- **tick, 6x:** D1+D2+D3 plausibly halve it (~1.75ms → ~0.9ms vs oracle
  0.29ms at n=8192). Full parity additionally requires the fill walk's
  per-node constant to approach decode's, i.e. the narrow-track fast
  path reaching through the watermark web (accumulator folds, boundary
  pushes, ledger links as machine words until spill). Mechanically the
  floor is below the oracle's (the oracle walks ~2L Rc nodes and
  rebuilds the spine); the engineering risk is how much of the
  width-circulation bookkeeping must run per node even in the narrow
  regime. Honest estimate: 1.5–2.5x oracle achievable with the same
  design; parity a stretch goal, decided by construct-and-measure.

Both are constant-factor deletions of redundant work (same bits, same
walks, cheaper primitives) — sign-fixed changes in the owner's sense:
the design question is code-reading, not benchmarking, and the
byte-identity oracle plus the differential suites pin behavior exactly.

## Recommended attack, in dependency order

1. **Word-buffered `PackedBuilder`** (u64 staging word over a word/byte
   vector; append-k-bits, truncate, patch, extract reworked as shift
   arithmetic; same zero-copy freeze into `Bytes`). The move set
   (truncate/extract/patch) is bespoke to the collapse discipline, so
   in-house rather than dsi's writer — the scan-meter's `record_bits`
   seam moves onto the new primitives unchanged.
2. **Codes as values**: payload codes as `Small(u64, u8)` /
   `Wide(BitsMut)` through `gamma_code`, the builder's held leaf, and
   the emitters. Kills the per-leaf allocation; absorb/cascade checks
   become integer compares. (The `lens` PopStack discipline and its
   depth-bounded transient guarantee are untouched — extract from the
   word buffer is cheap.)
3. **Narrow-track delta vocabulary**: `Step` magnitude, zigzag/unzigzag,
   `signed_sum` on machine words with overflow spill to `Base`.
4. **Suanpan narrow mode** (or a wrapper enum at the `before` seam): the
   running difference and the watermark web on i128 until spill.
   **Instrument interaction, must be planned with the meters**: the
   limb-meter's liveness floors count digit touches that a fast path
   legitimately stops performing — the envelope suite's floors and the
   fill module's width-circulation accounting need re-derivation as
   part of this change, not after it (instruments-before-cures applies
   to retiring the readings too).
5. **Per-op transient reuse/inlining** — measure after 1–4; mostly a
   small-n fixed-cost item.
6. Re-run the full bench suite and re-pin the movement annotations.

Everything above generalizes: all emitting ops (meet, hull/span,
projection materialization, `ticks`, party join's id builder) ride the
same `PackedBuilder`/code/accumulator currency, so the same changes lift
the whole surface, and the read-side ops (rank, min_ticks, distance,
lag) gain from D2/D4's narrow track.
