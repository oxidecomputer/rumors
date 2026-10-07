# L4 measures: observations

Base `58285ca5`. Each item says whether it was verified (by reading or by a
run) or inferred.

## O1. `sum_iter`'s 32-bit argument understates the version-derived exponent bound (prose, private)

`crates/before/src/rank.rs:1035-1043`:

> a version-derived exponent is bounded by its tree's stored bit length
> (under 2^32, the storage bound), so the documented panic is unreachable

`Version::encoded_bits` is a `u64` (`version.rs:1130`), and a 32-bit address
space holds under 2^31 bytes, so a stored version has fewer than about 2^34
bits, and its depth (hence its rank's exponent) is bounded by that, not by
2^32. The conclusion survives: 2^34 is below the 2^37 digit-position panic
threshold the comment guards. Verified by reading. Repair: say "under 2^34
bits (2^31 bytes)" or drop the number and state the margin to 2^37. Small
enough to ride the normalization-prose brief if a builder takes that.

## O2. `Add` and `checked_sub` each spell the alignment dispatch (simplification, optional)

`rank.rs:196-209` (`checked_sub`'s `Greater` arm) and `rank.rs:956-962`
(`Add`) each compute the common exponent, test `alignment_fits`, shift and
combine with `BigUint`, else call `accumulate(rhs, e, subtract)`. One private
helper, `combine(&self, rhs, subtract: bool) -> Rank`, would hold both routes
once, so the 32-bit-only accumulator route and the 64-bit shift route are
chosen in one place for both operations. Behavior-preserving; no format or API
change; covered by the `RANK_TRIPLE` laws, `l4_rank_values_agree`, and the
`wasm32-pins` `RankArithmetic` pin (which exercises both routes for both
operations). I judge the gain small (two short arms); recorded rather than
briefed. Verified by reading.

## O3. The committed integrator suite is adequate against every mutant I built

Eighteen integrator mutants (twelve value defects, three schedule-only
probes) and eight rank mutants: the committed `before` suite catches every
value defect, including narrow ones (a deferred height's sign lost; a single
deferral skipped; a segment width read without its scale; deferred widths
added only when the parked height is nonzero). The schedule-only probes show
the integrator's value is independent of its freeze and deferral schedule
(always defer, always freeze, never freeze all agree with the oracle). This is
a positive finding: the named-shape pools in `version/measure/tests.rs` close
the regime that `arb_oracle_version` cannot reach (its doc says it never
arms; I did not measure that myself).

My multi-scale partition generator reaches that regime from arbitrary inputs
(86% of cases freeze, 51% defer, 26% defer three or more times, 29% are exact
ties between distinct versions) but killed no mutant the committed suite
missed, so I am not briefing it as machinery: it samples a broader space
without a demonstrated failure that only it catches. If the adequacy lane
(L8) finds an integrator survivor, this generator (`explore_l4.rs`,
`arb_leaves` + `arb_related` + `check_pair`) is ready to aim at it.

## O4. Mirror and same-depth rotation give exact rank ties at any scale

Reversing a partition's leaf order, or rotating heights among leaves of one
depth, preserves the area exactly, so it yields distinct versions of equal
rank on any shape. The committed tie coverage (`rank_cmp` reading `Equal`
through the freeze pipeline) uses one family, mirrored arming trains. If a
builder ever extends that test, these two constructions generalize it to
arbitrary shapes cheaply. Recorded as an option, not a gap (no mutant
survived the committed tie tests).

## O5. Touch counts do not see `BigUint` work in the rank kernels

`suanpan::touch_meter` counts accumulator digit work only. `BigUint`
products in `SparseWidth::add_product`, the shifts in `Rank::from_raw`, and
`Rank`'s `Add` shift route are invisible to it; densified digits and wasm fuel
cover part of that. My cost probes therefore establish flat accumulator work,
not flat total work. Known by design (the meter's own doc says so); stated
here so nobody reads my flat readings as total-cost evidence.

## O6. `Sum` aborts on wasm32 in one summand order where `+` and the other order succeed (memory constant)

Verified on ox-east-1 in the `wasm32-pins` guest (prototype on
`explore/l4-measures` at `2c82c7ba`; logs `wasm-2.log`, `wasm-3b.log`).
Operands: the existing pin's rank `deep = (2^(2^32-65) + 1) / 2^(2^32)`
(a 512 MiB numerator) and `half = 1/2` or `one = 1`.

| Operation | Outcome | Guest memory high-water |
|---|---|---|
| `&deep + &half`, `&deep + &one` (committed cases 1-4) | pass | not recorded |
| `[&half, &deep].sum()` | pass | 3.56 GiB |
| `[&one, &deep].sum()` | pass | 3.06 GiB |
| `[&deep, &half].sum()` | `Trapped(UnreachableCodeReached)`, no panic message | 2.56 GiB at the abort |
| `[&deep, &one].sum()` | same abort | 2.56 GiB |

Mechanism, confirmed by replay: `Sum`'s accumulator stores each base-2^32
digit as an `i64` (`suanpan/src/accumulator/digits.rs:64`), so deep's
numerator fills a 1 GiB `Vec<i64>`; the next summand lands one digit past
capacity, and amortized growth requests about 2 GiB, which the 32-bit address
space cannot supply after the decode's high-water mark. Replaying exactly
those two accumulator calls aborts the same way (`pages=42014 panic=""`);
adding an exact `reserve_digits` before them passes within the same 42,014
pages. The `Sum` doc's "`O(n)` space" holds; the constant does not leave
room. In the deep-first order the accumulator transiently holds about six
times the result's width (1 GiB old buffer plus 2 GiB requested, for a 512
MiB result).

Classification: constant-factor observation per the performance ruling,
with a 32-bit consequence worth the owner's eye: whether a value that `+`
handles can make `Sum` abort, depending only on summand order. Candidate
repairs, none built: suanpan growing its digit vector exactly (or at a
smaller factor) when a landing extends past capacity (L7's crate); `Sum`
reserving the aligned width of the first wide summand plus slack; storing
settled digits more compactly. A 64-bit host never sees this, and the
`O(n)` claim is not breached, so I am not filing it as a defect.

## O7. The validation index omits the `wasm32-pins` workspace

`crates/before/src/testing/validation_index.rs` describes every instrument
"that guards this crate" but has no row for the 32-bit boundary pins (its
only wasm mention is the fuzz-fit fuel bands). Verified by reading. A
maintainer orienting from the index would not learn that pointer-width
behavior has its own instrument. Routed to the adequacy lane (L8); the
trap-diagnosis brief proposes the row.

## `usize` invariance (the clause added to `common.md`)

Every `usize` in the lane's production code (`rank.rs`, `ranked.rs`,
`count.rs`, `accumulator.rs`, `version/measure/**`) was read and classed;
verified by reading.

- **Memory roles the clause allows:** decoder and encoder staging
  (`DECODE_CHUNK_BYTES`, `ENCODE_BUFFER_BYTES`, `BitWriter::len`,
  `MatchingWriter::written`), `FromStr`'s digit indexing, `Ranked`'s
  `consumed` index, `Limbs::size_hint`, the deferred reduction's leaf indices,
  `accumulate`'s reservation hint, and `SparseWidth`'s dense cluster length.
- **Digit counts:** `digit_len`, `stored_digit_count`,
  `Integrator::boundary(delta_digits)`, `HEIGHT_FREEZE_ALLOWANCE_DIGITS`. These
  count base-2^32 digits of held values; suanpan's digit base is 32 bits on
  every target, so the freeze schedule is identical on every width. They are
  allowed counts of what memory holds; their `usize` type is incidental.
- **`Display`:** `Formatter::width`/`precision` are `usize` by `std`'s API and
  are widened losslessly to `u64`; text longer than `usize::MAX` characters
  still streams through `write_binary`'s `u64` positions.
- **Brief:** `Rank::alignment_fits` routes `+` and `checked_sub` by whether
  an exponent gap fits `usize`. See
  `briefs/simplification-rank-width-invariant-routing.md`.

### O8. `Count`'s `usize` conversions (design proposal)

`impl From<usize> for Count` (`count.rs:246-250`) is lossless on every
target: the same numeric value becomes the same count, so it is invariant.
`impl TryFrom<Count>/TryFrom<&Count> for usize` (`count.rs:221-243`) returns
`Ok` for a count of `2^32` on 64-bit targets and `TooWide` on wasm32: its
result differs by target by definition. The wasm32 half is already pinned:
the committed `Forks` check (`wasm32-pins/guest/src/checks.rs:80`) asserts
`usize::try_from(&count)` fails for a count above `u32::MAX`. No crate code calls it (callers:
tests and the wasm32 guest), so the crate's own behavior does not depend on
it.

- Option A, keep it, and add one sentence to `Count`'s type docs: "`usize`'s
  range depends on the target's pointer width, so `usize::try_from` succeeds
  for a given count on some targets and not others." This keeps the
  conversion where a caller needs it, to index or allocate, which is the
  memory role the clause allows, and makes the target dependence visible
  where the conversions are documented.
- Option B, remove the `usize` conversions, so callers write
  `usize::try_from(u64::try_from(&count)?)` and the target dependence sits
  in their code. This is a public API change.

Recommendation: A. The conversion is the one place where a count meets
memory, and B only moves the identical target dependence into every caller.

### O9. `DensePart::new`'s `span * 4` relies on an unstated bound

`crates/before/src/version/measure/integral/width.rs:239-245` allocates
`vec![0; span * 4]` with an unchecked `usize` multiplication; `span` came
from `usize::try_from(...).expect("cluster spans are bounded by the stream's
depth")` (`width.rs:197-198`). On wasm32 the product overflows if
`span >= 2^30`. It cannot: width digit indices are bit positions below the
version's depth divided by 32, and a stored version on 32-bit has fewer than
about 2^34 bits, so `span < 2^29` and `span * 4 < 2^31`. The argument holds
but lives nowhere in the code. Repair: `span.checked_mul(4).expect(...)` with
the bound as its proof, or state the bound beside the `expect`. Verified by
reading. Behavior is identical on both widths today.

### O6 and the L6 lead under the clause

Both differ by target, but through address-space capacity (a 4 GiB wasm32
memory and `Vec`'s `isize::MAX` capacity), which is memory held. Neither
stems from a `usize` used as a semantic quantity, so I keep them as an
observation and question (O6) and a cross-lane lead (L6), not as
`usize`-invariance defects.

## Cross-lane

- L6: `Rank::decode` on wasm32 panics with "capacity overflow" past 2^30
  fraction groups (about 1.13 GiB of canonical input) instead of returning an
  error. Inferred, not demonstrated. Record:
  `cross-lane-l6-rank-decode-wasm32.md`.

## Owner's ruling

O6, `Sum`'s order-dependent allocation abort on wasm32, is accepted as an
observation. The documented `O(n)` space bound holds. It is an input to the
`suanpan` growth-policy design work under lane L7's F1, not a defect.
