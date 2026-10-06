<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as the lane section for the L1 auditor, modeled on the L6 codec lane; under review by Finch before launch. -->

# Lane L1: identity (Party and Clock)

## Scope

This lane's theme is *identity*: how the id space, the unit interval `[0, 1)`,
is partitioned among parties, and whether that partition is conserved through
every operation that splits, merges, compares, or carves it. A `Party` denotes
a set of points. `fork`, `forks`, `join`, and `without` split and merge sets;
`is_disjoint` and `covers` compare them. A `Clock` pairs a party with a version
and inherits all of this, adding `sync`.

Some things to consider, as starting points rather than a checklist. They
are prompts for your own exploration: pursue whatever else the code
suggests, and expect the most valuable questions to be ones not listed
here.

- Can any operation create, lose, or duplicate a region?
- Do the predicates agree with set semantics on every canonical party,
  including parties no history produces?
- Do the stateful fork iterators conserve the partition at every step,
  including partial drains, early drops, and counts beyond the machine word?
- Do the error paths of `join` and `sync` behave exactly as documented, with no
  lost regions and no false errors? The crate page promises that such an error
  is always a definitive diagnosis.
- Do the walks over identity (`Regions`, `Overlay`) render the sets
  faithfully?

Identity is also where linearity lives. Within a process the type system
enforces it; this lane's theme includes what happens at its edges:
`dangerously_alias`, `from_parts`, and stale states restored from bytes.

The identifiers below are a starting map, not a boundary. Follow the theme
wherever the code takes it, including into code this list does not name.

- **Party:** `seed`, `is_seed`, `fork`, `forks` and `PartyForks`, `join`,
  `join_all`, `is_disjoint`, `covers`, `without`, `dangerously_alias`,
  `shape`, `Hash`/`Eq`, and the consuming array split
  (`From<Party> for [Party; N]`).
- **Clock:** `seed`, `fork`, `forks` and `ClockForks`, `join`, `join_all`,
  `sync`, `sync_all`, `from_parts`, `into_parts`, `party`, `version`,
  `own_version`, `shape`, `dangerously_alias`.
- **Implementation:** `src/party/`, `src/clock.rs`, `src/clock/forks.rs`.

Neighboring lanes explore adjacent themes. When your work crosses into one,
establish what you found and report it, rather than continuing the
investigation there. L3 explores events (`tick`, `send`, `recv`, `absorb`), and
L6 the codecs.

## Contracts to read first

- The crate page's safety rules, in particular the claim that a
  `join`/`sync` error "is always definitive: the programmer violated one of
  the above rules".
- `Party::forks`: shares are balanced, `self` keeps a residual and every
  unreturned share, and a full drain costs `O(|self| + k (|self| + log k))`.
- `Party::join_all`: on error it returns every unabsorbed region "without
  dropping any region".
- `Party::is_disjoint`: "Disjoint Parties may always be joined without
  error."
- `Party::without`: `None` exactly when `other` covers `self`.
- `Clock::sync` and `sync_all`: what each promises on `Err(Overlap)`.

## Prior coverage to map

- **Tests:** `src/party/tests.rs`, `src/party/forks/tests.rs`,
  `src/clock/tests.rs`, `tests/forks_count.rs`, `tests/stale_state.rs`, the
  party and clock law modules under `src/testing/laws/`, and the function
  oracle's replay (`src/testing/oracles/function/tests.rs`).
- **September partitions:** `party.md`, `clock.md`.
- **Triage:** checklist items on fork iterators at 32 bits and the maximum
  count, arbitrary-count fork steps, full drains of `forks` and `sync_all`,
  and Party/Clock property consolidation.

## Closed fixes to re-attack

- The fork iterators' contract on 32-bit targets and at the maximum public
  count.
- Each fork step's cost being linear in the stored values and the count's
  representation, including iterator construction, one step, and a partial
  drop.
- Full drains of `Party::forks`, `Clock::forks`, consuming array splits, and
  `Clock::sync_all` honoring their aggregate bounds.

## Candidate leads (evaluate, don't confirm)

1. **Conservation at every prefix of a fork drain.** After any number of
   `next` calls, followed by dropping the iterator, the residual and the
   yielded shares are pairwise disjoint and their union is the original
   party. `size_hint` is exact, including for counts beyond `usize`.
2. **No false errors.** Under any rule-respecting history (forks, joins,
   syncs, and moves through bytes, in any interleaving) `join` and `sync`
   never return an error. Conversely, `is_disjoint` true implies `join`
   succeeds, for arbitrary canonical parties.
3. **Region conservation on failure.** When `join_all` fails, the union of
   `self` and the returned parties equals the union of all inputs.
4. **`without`, `covers`, and `is_disjoint` as set algebra** over arbitrary
   canonical parties, including ones no history reaches.
5. **The `Regions` and `Overlay` walks** partition `[0, 1)` exactly, agree
   with the party's membership function, and stay iterative on deep inputs.
