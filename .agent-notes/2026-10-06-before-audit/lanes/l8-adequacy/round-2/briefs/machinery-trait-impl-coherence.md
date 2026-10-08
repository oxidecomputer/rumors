# Machinery brief: a law group for public trait-impl behavior

## The failure class it catches

Public trait implementations whose behavior no test checks: replacing the
body with a constant passes the whole suite. The surface check pins 453
public trait impls by existence and resource family; nothing pins what they
compute. Evidence so far (mutation survivors under the full suites, and
never-executed lines from branch coverage; the campaign is still running, so
the list will grow, and survivors/INDEX.md class T keeps it current):

| impl | evidence | constructible wrong behavior that passes today |
|---|---|---|
| `Hash for Version` (version.rs:160-164) | survivor: body replaced by `()` | every version hashes alike; maps and sets degrade to linear probing; the doc says it "Hashes the canonical bytes" |
| `Debug for Count` (count.rs:301-305) | survivor, never executed | doc: "The same format as `Display`" |
| `Add<&Count> for Count` (count.rs:321-326) | survivor, never executed | returns `Count::ZERO` |
| `Sum<&Count> for Count` (count.rs:357-364) | survivor | returns `Count::ZERO` |
| `Debug for Ranked` (ranked.rs:326-332) | survivor (lib tests), never executed | doc: "Renders the viewed version, tagged with the type's name" |
| `Iterator::size_hint` for `Plateaus`, `Regions`, `Overlay` (shape.rs:176, 228, 286) | every constant replacement survives | `(0, Some(0))` on a nonempty walk: an upper bound below the items yielded, breaking `Iterator`'s contract |
| `OwnVersion` comparisons in the `&lhs` and `&rhs` forms (version/own.rs:178-205) | never executed in any macro expansion | `gt`/`ge` call `lt(o, *self)`/`le(o, *self)`; an argument-order slip there answers the converse |
| `Span` operators with a version receiver (span/algebra.rs:653-662) | never executed | |
| `Debug for After`, `Before` (causally/forms.rs:401-409); `From<&Query> for Query` (causally/convert.rs:44-46) | never executed | |
| suanpan `Debug for Accumulator` (accumulator.rs:432), `ZeroRanges` (zero_ranges.rs:51) | survivors | |

## Which instrument it extends

The law registry (`crates/before/src/testing/laws/`), driven by
`algebraic_laws` over arbitrary and organic populations and shared with the
fuzz targets. A law needs no oracle: each of these is a predicate production
must satisfy. One new group per signature the registry already drives
(`VERSION_SOLO`, `VERSION_PAIR`, `PARTY_PAIR`, and so on) keeps the wiring
free; suanpan's spellings belong in suanpan's own surface model
(`accumulator/tests/surface.rs`), which already enumerates owned and borrowed
operator spellings for `+`/`-`.

## The laws

1. **Spellings agree.** For every operator with several owned and borrowed
   spellings (`Count`'s `+`, `+=`, `Sum`; `Version`'s `|`, `&`, `|=`, `&=`;
   `Span`'s operators; `OwnVersion`'s comparisons in all three forms, and
   each of `lt`, `le`, `gt`, `ge` against `partial_cmp`), every spelling
   equals the canonical one on the same operands. Generate the spelling list
   from the impls the surface census pins, or keep it beside them, so a new
   spelling without a law is visible in review.
2. **Hash depends on content where the doc says it hashes canonical bytes.**
   With the deterministic `DefaultHasher::new()` already used by `hash_of`
   (laws/mod.rs:231-235): `hash_of(v) == hash_of_bytes(v.as_bytes())` for
   `Version`, `Party`, `Ranked`, which pins the documented derivation and
   kills a constant hash. (Content dependence alone, `a != b` implying
   different hashes, is not a contract and should not be asserted.) If the
   owner prefers not to pin the derivation, assert instead that a fixed
   battery of distinct values does not all hash alike.
3. **`Debug` matches its documented format.** `format!("{c:?}") ==
   format!("{c}")` for `Count`; `Ranked`'s `Debug` names the type and renders
   the version's `Debug`.
4. **`size_hint` is sound.** For the shape walks and the fork iterators,
   after every `next()`, `lo <= remaining <= hi.unwrap_or(MAX)`, with
   `remaining` counted by draining a clone or a second walk.

## Calibration

Each survivor in the table above is a known-bad implementation: the new laws
must fail on each by reversible string swap (the cargo-mutants replacements
listed are exactly those swaps), and pass on the base. The `OwnVersion`
argument-order slip is the one to construct by hand: swap `lt(o, *self)` to
`lt(*self, o)` in one `gt`.

## Budget

All four law families are a few comparisons per case over existing
populations; no measurable time.

## Amendment (round 2): narrow to `Hash`

Per the coordinator's rule for this round (brief only contract-level trait
behavior), keep law 2 and drop laws 1, 3, and 4:

- Keep: `Hash` derived from the canonical bytes, for `Version` and `Party`
  (both survive replacement by `()`: `version.rs:162`, `party.rs:150`) and
  `Ranked` (documented as "equal to the `Version`'s own hash",
  `ranked.rs:344-345`).
- Drop: operator spellings (each surviving form is a one-line delegation to a
  tested canonical operator; `rumors` uses `Rank` only for ordering and calls
  none of the surviving forms), `Debug` content, and `size_hint` (advisory
  under `Iterator`'s own contract; no `before` rustdoc promises exact hints
  for the shape walks).

The bits group adds a third `Hash` survivor: `Hash for Bits` replaced by `()`
(`bits/storage.rs:156`). `Bits` is crate-private, and both `Hash for Version`
and `Hash for Party` delegate to it (`self.0.hash(state)`), so the
content-dependence law for `Version` and `Party` kills this mutant as well;
it needs no test of its own.

## Owner's ruling on the hash contract (question 79)

`Version` and `Party` hash exactly as their byte views do: "it should hash
exactly like its bytes. Why should anything not?" The trait-coherence branch
promises this in the public rustdoc of both types, and the byte-hash laws
stand as written.
