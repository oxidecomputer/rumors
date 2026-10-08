<!-- CAVEAT LECTOR: the round-1 report of the L1 (identity) auditor, Claude Opus 5.5, condensed by the coordinator, who verified the explore branch's signatures and read D1 and MB3 in full; other results are the auditor's own claims. -->

# Lane L1, round 1: report

The explore branch is `explore/l1-identity` at `26cbae22`, with production
code unchanged from `58285ca5`. All its commits are signed. The sibling files
in this directory are the auditor's deliverables, including a full
instrument inventory ([`inventory.md`](inventory.md)).

## Verdict

- One low-severity defect, two questions, three proven test gaps, and no
  behavioral defect. The lane is at diminishing returns.
- A black-box suite checked identity against an interval-set model that
  shares no code with production or the tree oracle. It passed at 100,000
  cases, and again at 20,000 with fresh seeds.
- Leads 2 to 5 hold. Lead 1 is disputed, which produced D1.
- Every fork-step and full-drain cost ratio is flat across four sizes.

## Defect and questions

- **D1 (low):** [`D1-fork-size-hint-width.md`](D1-fork-size-hint-width.md).
  The fork iterators' `size_hint` depends on `usize` width, because the
  initial count is classified against `usize::MAX`. The current rustdoc
  allows this, so the fix waits on question 9 in `QUESTIONS.md`.
- **`Clock::from_parts`:** pairing a live party with an older version
  re-issues a stamp. The model accepts this; whether to document it at
  `from_parts` is question 10.

## Briefs

Each of MB1 to MB3 rests on a mutant the committed suite misses.

- **MB1:** [`MB1-join-all-multiplicity.md`](MB1-join-all-multiplicity.md).
  `join_all` must return each rejected region exactly once. Mutants M19 and
  M20 return a group twice and pass all committed tests, because every
  conservation check compares unions.
- **MB2:** [`MB2-deep-identity-probe.md`](MB2-deep-identity-probe.md). The
  fork iterators' walks have no deep coverage. Mutant M26, a recursive
  removal descent, passes the committed suite.
- **MB3:** [`MB3-opt-level-0-stack-safety.md`](MB3-opt-level-0-stack-safety.md).
  The workspace builds `before` at `opt-level = 2` even in dev, and LLVM
  removes self tail calls there. So no committed test proves stack safety
  for a downstream crate building at `opt-level = 0`. Mutant M25, a
  tail-recursive descent, passes everything at the workspace profile and
  overflows at `opt-level = 0`. The brief adds a gate recipe that runs the
  deep tests at `opt-level = 0`.
- **S1, a constant factor:**
  [`S1-result-buffer-retention.md`](S1-result-buffer-retention.md).
  `BitsWriter::finalize` keeps its reserved capacity. One-byte results of
  `join`, `without`, and `Clock::join` retain about one byte per input byte.
- **SB1, a simplification:**
  [`SB1-sync-all-merge-via-join-all.md`](SB1-sync-all-merge-via-join-all.md).
  `sync_all` repeats `Clock::join_all`'s fold.

## Observations

These are in [`observations.md`](observations.md):

- **O1:** a party taken from a decoded clock keeps the whole buffer alive.
- **O2 to O4:** precision of the fork-cost and size-hint docs.
- **O5:** `fold.rs`'s `weight` is a `usize` level counter.
- **O6:** the fork iterators are fused but don't implement `FusedIterator`.
- **O7:** the `log |self|` term in the `join_all` and `sync_all` complexity
  has no stated derivation.
- **O8:** the committed party generators stop at depth 4.
- **O9:** version writers likely share S1's retention.

## Coverage

See [`coverage.md`](coverage.md) and [`NOTES.md`](NOTES.md).

## Owner's ruling

On `Clock::from_parts`: yes. Its rustdoc states that pairing a party with a
version older than its latest tick reproduces stamps the party already
issued, linking the crate page's model.

## Owner's ruling on the crate page's linearity rule (question 84)

Option 1: only identity is linear. In the owner's words: "If you want
versions to be monotonic, don't disassemble the clock and use its parts, but
the ability to have non-monotonic versions is not enforced by the library;
only identity linearity is meant to be." The crate page's rule 2 is restated
around identity, with `dangerously_alias` and bytes as its two exceptions,
and a paragraph says a `Version` records knowledge and is freely `Clone`.
`tests/stale_state.rs`'s "Valid by the model" stands.

## Owner's ruling on restoring an identity (question 103)

Confirmed: "you can safely restore an identity (within a clock or not) once
you, the caller, ensure that it doesn't exist anywhere else." Restoring an
older state while no live party holds any part of the identity only rewinds
versions, which is the caller's monotonicity discipline, not a linearity
violation; restoring while any live party holds part of it (the source, a
party that has since received part of it, or another restoration of the same
bytes) violates linearity.

## Owner's direction on the linearity asymmetry (question 105)

The owner's model, in their words: "if you want `Version` monotonicity you
have to treat `Version` as affine. But some callers may not want that, so it's
optional. Whereas treating `Party` as non-affine leads to in-actuality
concurrent `Version`s which are potentially comparable as equal or
sequential. So `Party` non-linearity breaks `Version`'s causal ordering,
whereas `Version` non-linearity merely breaks monotonicity relative to some
application notion which the caller must be ultimately responsible, because
they're the ones who have to decide what `Version`s are even meant to
represent or tag." Direction: fold this notion into the docs, on the crate
page and restated in brief in the `Version` or `Clock` docs, in terms of
`Party` and `Version`. `Clock` is optional, a convenient wrapper for a 1:1
pairing of one `Party` with the `Version` it ticks; `rumors` does not use
`Clock` in its tree, because one `Party` ticks many `Version`s. The
coordinator's proposed wording was not accepted; the text is drafted afresh.

### Correction to the summary of question 103's ruling

The summary above says restoring an older state with no live holder "only
rewinds versions". That overstates the owner's words ("you can safely restore
an identity ... once you, the caller, ensure that it doesn't exist anywhere
else"), and the claim is false: the restored party lacks the events recorded
in its identity since the saved state, and its ticks can then dominate them
(constructed by #104's reviewer). Question 108 asks how the contract should
state that.

## Owner's framing of the parts and the stamp (question 108)

In the owner's words: "fundamentally you're trying to attribute more domain
semantics to `Version` and to a lesser extent `Party` than they actually
*have*. `Clock` is the safest abstraction, the hardest to make mistakes with
(and indeed, `Version` is also unproblematic in the context of `Clock` +
`Version` interactions). In the original ITC paper, the objects of study
don't really directly deal with a naked `Party` the way `before` exposes it,
and the problematic cases all seem to stem from performing operations on
parties which are not tracked by corresponding causal operations on
`Version`s -- *assuming* that you want a particular semantics out of these
operations."

Consequence for the docs: `Clock` corresponds to the paper's stamp (an id
and an event, operated on together); the safety rules are invariants of that
system; `Party` and `Version` are its parts, whose own guarantee is their
algebra. The hazards the reviews constructed (re-ticking a copied version, a
bare `Party::join` without the version join, restoring identity without the
events recorded in it since) are each an operation on one part without its
paired operation on the other, which matters only if the caller wants the
stamp's causal semantics.

## Owner's ruling on the crate page's linearity section (question 111)

The owner drops the audit's rewrites of `before`'s crate page and will
write the linearity material themselves: "I want to drop rewrites of the
crate lib.rs page. I'll tackle this myself." The draft branch
`docs/crate-page-identity-linearity` (`78a180b1`) is withdrawn from the
review order and kept as a reference. Its per-item doc changes link to the
section it adds, so they do not stand without it. Two of them are corrections
the owner's pass may want:
- `Party`'s docs list decoding and `dangerously_alias` as escape hatches
  from linearity, but not parsing through `FromStr`.
- The decode warnings on `Party` and `Clock` say only that the decoded
  value must not coexist with its source. It also must not coexist with a
  party that has since received part of the source's identity, or with a
  second decoding of the same bytes.
