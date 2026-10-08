# Can the instruments detect `usize`-width-dependent behavior?

The owner's clause (common.md, "`usize` invariance"): both crates should
behave identically whatever the width of `usize`; a `usize` other than an
index, a length, or a capacity actually held is a possible defect.

## Verdict

Partly, and only for values. Every instrument except the wasm32 pins runs on
64-bit hosts, so width dependence is visible only where the pins look, and no
instrument measures cost on a 32-bit target.

| instrument | runs on | can see width-dependent behavior? |
|---|---|---|
| nextest suites (both crates) | x86_64 (box), aarch64 (Mac) | No: both hosts have 64-bit `usize`. |
| amplification board, worst-case pin, focused meters | same | No for width. They can drift by architecture (findings/count-display-heap.md), which is a different axis. |
| wasm32 pins (`crates/before/wasm32-pins`) | wasm32 guest in wasmtime | Yes, at their pinned sites, for values. Calibration: 7 of 11 injected narrowings caught. Lengths and counts crossing 2^32 are caught (decode at three sites, rank exponents, fork counts, digit landings). Positions are not: no pin places a live bit at or past 2^32, so narrowing a cursor position is invisible; the rank pin cannot tell its two routes apart; a writer length held in `usize` is invisible by the writer's design (findings/wasm32-pins-calibration.md). |
| fuzz-fit bands (wasm32 fuel) | wasm32 guest | Cost on 32-bit, in principle; out of the audit's scope. |
| surface check | rustdoc JSON | Not today; it could list `usize` in public signatures (below). |

## A mechanical census of `usize` in public signatures

`l8/usize_census.py` (explore branch) walks `before`'s rustdoc JSON (built
`--all-features` from the probe copy, as the surface check builds it) and
lists every public function-like item whose inputs or output mention
`usize`. Output: usize-census.txt in this scratch directory. Production API
rows (excluding the `meter`-feature instruments under `testing`):

- `Iterator::size_hint` for `Cells`, `ClockForks`, `Limbs`, `Overlay`,
  `PartyForks`, `Plateaus`, `Regions`: the standard trait's type. The fork
  iterators can yield more items than `usize` counts; the wasm32 `forks` pin
  holds their hints sound past 2^32 (`(usize::MAX, None)`).
- `From<usize> for Count` and `TryFrom<Count> for usize`: lossless in, and
  checked out ("succeeds exactly when the value fits"), so their behavior
  differs by width only as the destination type does.

So no public production parameter or stored quantity in a signature is a
`usize` outside standard conversions and iterator hints. Internal arithmetic
is a separate question the grep-based lanes own.

Observation (design proposal for the surface check): pin this list in
`surfacecheck`'s census, so a new `usize` in a public signature becomes a
deliberate API event, like a new trait impl. It costs one more reconciliation
over data the check already parses.
