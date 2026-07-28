# OwnVersion: the lazy projection view

Status: DECIDED (owner rulings 2026-07-27, recorded below), not yet
implemented. This document is the charter of record for the landing;
it was designed in conversation with the owner and every DECIDED
entry is an owner ruling.

## Motivation

The projection `v / &p` is the one operation in `before` whose output
size is not derivable from its inputs: output is Θ(|v|·|p|) bits on a
Θ(|v|+|p|) input (measured 45–119× the input on the board's probe
shapes), so the output builder walks a doubling chain whose
capacity-phase profile the owner has ratified as the documented honest
cost of materialization (amplification ledger §12, the presize-61
finding and its disposition).

The workspace's only production consumer of the projection is
`rumors`' bookmark reclamation (`src/bookmark.rs`), which computes
`clock.own_version() <= version` — it materializes the full
product-growth projection, consumes it with one linear comparison, and
throws it away. The comparison never needed the projection as an
object: `(v/p) ⋚ w` is decidable by one fused co-walk over the three
streams `(p, v, w)`. Inside `p`'s region, compare heights of `v`
against `w`; outside it, the projected height is zero, which
contributes only toward `<`/`=` (it can never create concurrency by
itself). Cost: O(|p|+|v|+|w|) scan bits, allocation-free, against
materialize-then-compare's Θ(|v|·|p|).

The design goal, per the owner: make the expensive path *unwritable by
accident*, not merely discouraged. Projection is lazy everywhere;
Θ(|v|·|p|) is only ever spelled as an explicit materialization call.

## The shape (DECIDED)

- **`OwnVersion<'a>`**: a ref-owning view, fields `{ party: &'a Party,
  version: &'a Version }`. The name reads as "the version `p` owns of
  `v`" — heights within the party's region — which is exactly the
  projection semantics, so both constructors below are two spellings
  of one honestly-named concept.
- **Two constructors**: `&v / &p` (the `Div` impl on references now
  returns `OwnVersion<'a>`, not `Version`) and
  `Clock::own_version(&self)` (borrows the clock's own fields). The
  by-value `Div` impl (consuming `Version`) is dropped — a view cannot
  borrow a consumed operand — unifying on `&v / &p`.
- **Materialization is explicit**: `.to_version()` and
  `impl From<OwnVersion<'_>> for Version`. These are the only paths to
  the product-growth output; the ratified doubling band is reachable
  through them alone.
- **Comparison surface** (the full reference-forwarding matrix, both
  `PartialEq` and `PartialOrd`, both directions):
  - `OwnVersion` vs `Version` and vs `&Version` — the fused
    three-stream co-walk.
  - `OwnVersion` vs `OwnVersion` and vs `&OwnVersion` — the
    homogeneous four-stream co-walk, `(v₁/p₁)` against `(v₂/p₂)`.
  - Equality is semantic (the projected profiles agree), not byte
    equality: `OwnVersion == w` requires `w` to be zero outside the
    party's region. `OwnVersion` implements no `Hash`.
- **`DivAssign` disposition**: `v /= &p` is the explicit in-place
  eager form; its cost is visible at the call site, so it does not
  reintroduce the footgun. The implementer censuses its consumers
  (at charter time: only its own law and tests) and drops it unless a
  real consumer exists; `.to_version()` covers the composition.

## Specification

The heterogeneous comparison is *defined* as the homogeneous
comparison against the seed-masked view: `OwnVersion { party, version }
⋚ w` means `OwnVersion { party, version } ⋚ OwnVersion { party:
seed, version: w }`, which is sound because projection by the seed
party is the identity (`before::laws`,
`seed_projection_is_identity`). The seed form is definitional only —
constructing a fresh seed party would violate party linearity — so
the *implementation's* oracle is differential:

- `view ⋚ w  ≡  view.to_version() ⋚ w` over arbitrary clocks and
  versions (three-stream law), and
- `view₁ ⋚ view₂ ≡ view₁.to_version() ⋚ view₂.to_version()`
  (four-stream law),

both added to `before::laws` as named predicates with all three
consumers (group proptests, organic populations, the fuzz target).

## Semantic notes for the implementer

- Outside `p`'s region the projected height is zero: those subtrees
  contribute only toward `<`/`=`. The `<=` direction is unambiguously
  linear. The full trichotomy must distinguish `=` from `<`, which
  requires knowing whether the right operand has any positive height
  outside the region — a zero-check on skipped subtrees. The skyline
  encoding likely prices this at O(1) per region boundary; that is a
  claim for the demonstrator to measure and pin, not to assert.
- Subtree skips ride the existing iterative skip machinery and are
  priced at the skipped subtree's own bits, keeping the whole walk
  O(sum of operand sizes).
- The laws re-denominate along the grain: comparison-shaped laws
  (seed identity, the projection/order laws stated as comparisons)
  ride the fused heterogeneous ops lazily with no materialization;
  laws about the materialized object (join/meet homomorphism,
  additivity over fork) grow `.to_version()`, correctly.
- The paper oracle keeps its eager `Div` returning a materialized
  version — it is the other side of the differential, not a consumer
  to convert.

## Landing kit

The instrument treatment for every new public operation, plus the
re-denomination of the old one:

- Census rows for the view's comparison ops; `# Complexity` sections
  consistent with the claims roster (the roster's totality checks
  force this).
- Board rows: `/` becomes O(1) view construction; the materialization
  cost rows move to `.to_version()` (same readings, honest new owner).
  The fused comparisons get their own rows.
- Adversarial family: the three-stream walk wants a correlated-triple
  generator and the four-stream walk a correlated-quadruple — the
  relational-family genre (per-operand-benign, correlated-pair-hot)
  generalized to this arity: correlate one operand's mask boundaries
  with the *other* operand's height drift, in the spirit of the
  jump-pair wedge.
- Envelope rows with liveness floors, as usual.
- The differential laws above, in `before::laws`.
- `rumors/src/bookmark.rs` moves to the fused comparison (its
  `extract_if` predicate keeps its shape; the materialization simply
  disappears).

## Sequencing

After the ticks(n) landing and the distance/lag cure merge, and after
the batch-elimination merge round (which owns the rumors seam), and
before the legibility pass and the adversarial review, so both see
the final API surface.

## Decision record

- DECIDED 2026-07-27 (owner): replacing shape, not additive — there
  is no second method beside `own_version()`; the view *is* what
  `own_version()` returns.
- DECIDED 2026-07-27 (owner): type name `OwnVersion<'a>`;
  `own_version()` keeps its name; laziness lives in the type.
- DECIDED 2026-07-27 (owner): full comparison matrix including the
  homogeneous four-stream case; both `.to_version()` and the `From`
  impl.
- DECIDED 2026-07-27 (owner): `/` itself returns the view — the
  projection is lazy at every spelling, and explicit materialization
  is the only path to Θ(|v|·|p|).
- DECIDED 2026-07-27 (owner, separately recorded in the amplification
  ledger): the materialization doubling-chain band is ratified as-is;
  no pre-walk, no segmented assembly.
