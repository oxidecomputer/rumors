# API suggestions and expected features

This document collects every finding of the holistic review of `before` and `suanpan` at commit `9e5784fb4dce977cfbdfd1619886d1482b5ce764` whose primary class is *api-surprise* (a public or crate-internal shape that a reader or caller would not expect from the neighbouring API) or *feature-gap* (an operation a user would reach for that does not exist). `before`'s public API is declared stable (`crates/before/AGENTS.md`), so nothing here is an assumed change: each entry is a suggestion with its rationale, and each carries a disposition of **adopt**, **consider**, or **decline** with the reason, on a line marked *Synthesis*. Findings of other classes that bear on an entry are cross-referenced by id and never reproduced.

Ids are `<partition or sweep key>-<n>`; the full record of each finding, with every candidate it absorbed and the verification and history passes behind it, lives in `evidence/partitions/<key>.md` or `evidence/sweeps/<key>.md` beside this file. Severities are **high** (a wrong answer or a contract breach a caller can reach), **medium** (a defect or gap with bounded blast radius), **low** (an inconvenience, or an inconsistency with a cheap fix), and **nit** (a naming or wording slip). Provenance is stated per finding: **demonstrated** means a constructed test ran; **executed** means a run settled the claim; **verified** means mechanically checked or re-derived (a grep, a read of a dependency's source, a hand trace); **assessed** means read. The entries below are the finalizers' text; where I tightened a sentence I changed no anchor, evidence, verdict, severity, or provenance.

Every primary anchor and every related anchor an entry names was re-read in this pass from `git show 9e5784fb:<path>`, and all of them match. At the time of writing the working tree was clean at `7440d1a3`, two commits above `9e5784fb`; `git diff --name-only 9e5784fb HEAD` lists only paths under `.agent-notes/`, so the tree agrees with the commit of record on every file in scope.

## The fresh-eyes report

The fresh-eyes sweep built a scratch application against `before` and `suanpan` from their public documentation alone (the quickstart, a thirteen-version fleet forked from one seed, `Ranked` keys in a `BTreeMap`, the `causally` query language, hostile decode probes, serde_json round trips, a `suanpan` running total across a 2^128 carry) and reported sixteen findings, none of them a correctness defect. Its verification pass confirmed all sixteen against the tree, matched each executed claim to the sweep's own compile and run logs, and settled the one open question by reading the rendered rustdoc. Six are API usability findings and are entries in this document; the other ten are documentation findings, cross-referenced below.

What the application reached for and could not find, each an entry here:

- `coverage` on what `after(v)` and `before(v)` return. `Floor` and `Ceiling` answer `contains` only; the call fails with E0599, and the bridge that would have worked (`Query::from(floor)`) is documented only on the impl (fresh-eyes-6).
- The fork iterator under the name rustdoc shows. `Clock::forks` renders as returning `Forks<'_>`, and `use before::Forks` fails with E0425; the importable path is `before::iter::Clock`, a name the reader has already reserved for the clock type (fresh-eyes-9; the API audit reached the same item independently as api-audit-6).
- `u64::try_from(ticks)` by value. Only `TryFrom<&Ticks>` exists, and the failure is E0277 with std's misleading hint about `From<Ticks>` (fresh-eyes-14; api-audit-13 widens this to the absent `u128`/`usize` targets and the absent `checked_sub`).
- `?` on `Clock::join`. The error is the handed-back `Clock`, which is not an `Error`, so propagation into `Box<dyn Error>` fails, while `sync`/`sync_all` return `Overlap` (fresh-eyes-15).
- `Rank` read back from the text `Display` produces. `Party`, `Version`, `Clock`, and `Ticks` have `FromStr`; `Rank` does not (fresh-eyes-16).
- The `io::Error` behind `Decode::Io` through `Error::source()`, which returns `None` (fresh-eyes-5; the same defect is filed as api-audit-2 under correctness and crate-root-15 under idiom).

What it needed and found only after reading source, filed as documentation findings: the whole-input rule and `# Errors` sections on `Version::decode`, `Party::decode`, and `Clock::decode` (fresh-eyes-2); the serde representation, which is a numeric byte array in JSON (fresh-eyes-3); whether `Query` has equality and why not (fresh-eyes-8); `PartialEq` named as the causal ordering where `PartialOrd` is meant (fresh-eyes-1); an owned-operand `/` in the projection docs that does not compile (fresh-eyes-7); `k` in signatures against `n` in prose (fresh-eyes-10); typos and wrong link targets (fresh-eyes-11); `Party::decode`'s warning written about `Clock` (fresh-eyes-12); the error module's summary line and `Overlap`'s producer list (fresh-eyes-13); "mint" and the "door" metaphor in public rustdoc (fresh-eyes-4).

What worked on the first try, per the sweep's run (not re-run in this pass): the operation tables on `Clock`, `Version`, `Party`, and `Span` were enough to write the whole program without opening private source, and every operator meant what the table said; the hostile decode probes (empty input, a party alone, a value plus trailing bytes) returned the documented `Decode` variant with no panic; `Ranked` keys sorted causes before effects across the fleet and `Ranked::decode` recovered the version from the key alone; the valuation laws `rank(a|b) + rank(a&b) == rank(a) + rank(b)` and `lag(a,b) + lag(b,a) == distance(a,b)` held; thirteen parties encoded in one to two bytes each and the root's version after twenty-three events in nine bytes; `suanpan`'s first example carried the 2^128 boundary correctly.

## The API audit, item by item

The API audit sweep read every public item of both crates (the rendered rustdoc at this commit, each public defining file, and the 1.88 `core` sources for `Option` and `Result`) and reported twenty-three findings. Its verification pass kept all twenty-three at their cited lines, reframing two with factual corrections: `Option` carries no `#[must_use]`, so `Party::without` is not covered as the sweep said (api-audit-1); rustdoc does form the `Clock::forks` link, with the stray backtick inside the link text (api-audit-5). Five findings are API shape and are entries here; the others are documentation (api-audit-3, -4, -5, -8, -9, -10, -15 through -23), correctness (api-audit-2), verification-gap (api-audit-7), and simplification (api-audit-12), cross-referenced where they bear on an entry.

1. api-audit-1 (medium). `Party` and `Clock` carry no `#[must_use]`, so `parent.fork();` compiles and the child's share is gone. **Adopt.**
2. api-audit-6 (low). The fork iterators are defined as `Forks` but reachable only as `iter::Clock`/`iter::Party`; rustdoc shows a name the user cannot import. **Adopt** the prose fix; **consider** the rename for the next breaking version.
3. api-audit-11 (low). `Span`, `shape::Plateau`, `Rise`, and `Region` lack `Hash` while every other value type has it. **Adopt.**
4. api-audit-13 (low, feature gap). `Ticks` converts out only by reference to `u64` and has no subtraction, where `Rank` has `checked_sub` and `saturating_sub`. **Adopt.**
5. api-audit-14 (low). `Rank::decode` labels its representation bound `Decode::NotCanonical`, and neither `Decode` nor `Parse` is `#[non_exhaustive]`. **Adopt** the documentation half; **consider** `#[non_exhaustive]` on `Decode` before the first release.

The same sweep left four owner questions that are API decisions rather than findings (`const fn` constructors, an `encoded_len()` on the composite types, the reason `Query` has no `PartialEq`, and whether suanpan's `claims` roster should be public); they are carried under Open questions below with recommendations.

## Feature gaps

Four findings name something a user would reach for that the crates do not offer. For each, what the user would do with it:

- api-audit-13: `TryFrom<Ticks>` by value, `u128`/`usize`/`u32` targets, and `checked_sub`/`saturating_sub` on `Ticks`. A caller measuring how many events separate two versions writes `b.min_ticks() - a.min_ticks()`; today that is unspellable, and the only way out of a `Ticks` is `u64::try_from(&t)`.
- fresh-eyes-16: `FromStr for Rank`. A rank logged or configured as text (`19/2^4`) is read back; today the only round trip is the wire form through `encode`/`decode`, and the docs do not say the text form is one-way.
- fuelscape-render-15: an append path for the fuelscape dump. A maintainer measures one new operation alone and adds it to the committed atlas; today the only writer truncates the index, and the one accretion on record was a hand merge whose recipe lives in a commit body.
- fuelscape-render-17: a regrid path for the stored `HeatGrid`. A maintainer changes `FUEL_BINS` or an axis margin and re-renders the committed dump; today every op document is refused and nothing in the tool repairs it.

## Highest-value items

1. Put `#[must_use]` on `Party` and `Clock` (and on `Party::without`, whose `Option` return is not inspected by the lint), so the compiler catches the one linearity slip it does not catch today: a discarded fork or seed that leaks id space with no diagnostic (api-audit-1; the same mechanism extended to the pure combinators in clippy-pedantic-4).
2. Give the fork iterators a name a user can import, or at least make the `Clock::forks`/`Party::forks` prose name the public path; today the rendered signature says `Forks<'_>` and no `use before::...` spells it (api-audit-6, fresh-eyes-9).
3. Annotate `Decode::Io`'s field `#[source]` so chain-walking reporters reach the `io::Error` and its `ErrorKind` (fresh-eyes-5; api-audit-2, crate-root-15).
4. Widen `Ticks`'s way out: `TryFrom<Ticks>` by value, the `u128`/`usize` duals of the `From` impls that exist, and `checked_sub`/`saturating_sub` mirroring `Rank` (api-audit-13, fresh-eyes-14).
5. Derive `Hash` on `Span`, `Plateau`, `Rise`, and `Region`; `Span`'s `Eq` is byte equality of two canonical streams and `Version: Hash` already hashes exactly those bytes, so the derive is consistent by construction (span-causally-1, api-audit-11).
6. Let `Floor` and `Ceiling` answer `coverage`, or say in one sentence on `after`, `before`, `Floor`, and `Ceiling` that a bare atom converts into a neutral `Query` with `Query::from` (fresh-eyes-6).
7. Show the propagation idiom for `Clock::join`'s handed-back error in its `# Errors` section, and keep the hand-back form itself: an `Overlap`-returning `join` would drop the un-absorbed clock, which is exactly the id-space leak item 1 exists to catch (fresh-eyes-15).
8. Decide `#[non_exhaustive]` on `Decode` before the first release, and name the representation-bound case in `Decode::NotCanonical`'s variant doc so a canonical-but-unrepresentable input is not called non-canonical without a word (api-audit-14; rank-10 rewrites the same clause from the other side).
9. Rule which `Parse` precedence the paper notation has: the id parser reports `NotCanonical` at the node's close even with trailing junk, the version parser lets trailing junk outrank it, and `Parse`'s docs state neither (codec-base-text-tree-18).
10. Tool the fuelscape dump's documented accretion workflow: `DumpWriter::open`, a params-mismatch and duplicate-name refusal, and a refusal when a plain `atlas.json` sits beside the committed `.gz` (fuelscape-render-15).
11. Decide whether a zero wide operand should retire suanpan's quick register; today `add_wide(&UBig::ZERO)` spills and can flip a domination certificate while `add_magnitude(&UBig::ZERO)` does not, and the exact-touch contract is what blocks the cheaper ordering (suanpan-10).
12. Either parse `Rank` from its `Display` grammar or state that the text form is one-way; the decision belongs with the human-readable serde question, which would need text forms for `Rank`, `Ranked`, and `Span` anyway (fresh-eyes-16).

## Crate-wide patterns

- **Missing duals.** The crate documents operation pairs as pairs and checks for symmetry, and the API findings are where that discipline lapsed: six `From` impls into `Ticks` against one `TryFrom<&Ticks>` out and no subtraction (api-audit-13, fresh-eyes-14); `Display` without `FromStr` on `Rank` alone among the five text-rendering types (fresh-eyes-16); `Eq` without `Hash` on `Span` and the three shape types while every other value type and all four verdict enums derive both (span-causally-1, api-audit-11); a bounded read named `get` on `BitsView` beside an asserting read named `get` on `BitsBuf` (codec-bits-11). Each is additive or crate-private.
- **Linearity is guarded against duplication, not loss.** `assert_not_impl_any!(Party: Clone, Copy)` and its `Clock` twin make a second holder of one share a compile error, and the fallible reunions all hand identity back rather than dropping it; the one slip the compiler does not catch is a discarded by-value share, because no `#[must_use]` exists outside an internal build token (api-audit-1, clippy-pedantic-4). The same invariant is the reason to keep `Clock::join`'s `Err(Clock)` rather than add an `Overlap`-returning variant (fresh-eyes-15).
- **Error types are where a consumer first stumbles.** Four entries concern `error.rs` or its variants: `source()` cut on `Decode::Io` (fresh-eyes-5), `?` not composing on `join` (fresh-eyes-15), two precedence rules under one `Parse` enum (codec-base-text-tree-18), and a mislabelled representation bound with no room to add a variant later (api-audit-14). The related documentation findings (fresh-eyes-2, api-audit-8, version-core-14: `# Errors` sections on the reader decoders and the `FromStr`/`TryFrom` impls) belong to the same pass.
- **A public name that no path spells.** The fork iterator type is defined as `Forks` in two private modules and re-exported as `iter::Clock`/`iter::Party`; rustdoc renders the definition's name in signatures and prose, so the documented name, the linked page's title, and the importable path are three different strings (api-audit-6, fresh-eyes-9). The neighbouring documentation nits (api-audit-5, fresh-eyes-11: the backtick in `iter.rs`'s link) are the same module.
- **Independent convergence.** The fresh-eyes application and the API audit, run without sight of each other, met the fork-iterator name, the `Ticks` conversions, and the `Decode::Io` chain independently, and a partition review and the audit both filed `Span: Hash`. Where two blind passes land on one item, that item is what a new user meets first.
- **Documented workflows without a tool path** (instrument side). The fuelscape dump's own prose describes accretion and styling-independent replay as the design, and the tree has no writer that appends and no path that regrids (fuelscape-render-15, -17); the sampler's preconditions are enforced far from the arithmetic that relies on them (fuelscape-pipeline-14).
- **The meter-surface stability question decides several gates.** Whether `pub mod meter` (with the `skyline` re-export, the board constants, and the counter readers) is held to the stable-API rule was asked independently by five reports (board-frame, codec-base-text-tree, skyline-sweep-place-masked, board-families-floors-judge, clippy-pedantic). None of the entries here turns on it, but the answer sets how future sweeps classify instrument-surface changes; it is carried under Open questions.

## Crate root and public types

### The error module

### fresh-eyes-5: Decode::Io drops the io::Error from the error source chain
- Where: crates/before/src/error.rs:89-91
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (the sweep's run2.log line 48: `P2 Decode::Io display = read error: boom; source() is_some = false`; this pass read the variant: a bare tuple field with no `#[source]`/`#[from]`, and no such attribute exists anywhere in `before` or `suanpan`; thiserror 1.0.69 per Cargo.lock only wires `source()` for a field named `source` or so attributed); executed: yes, by the sweep's scratch run, matched to its log
- Verification: confirmed; history: no-rationale-found (b3f09baa0, a WIP docs pass)
- Owner-gated: yes: adding `source()` is an additive behavior change on a stable public type

The `Io` variant wraps an `io::Error` but `Error::source()` returns `None`, so reporters that walk the chain (anyhow, eyre, tracing) see only the Display text and the `io::ErrorKind` is unreachable from the chain.

Evidence:

        89	    /// The underlying reader failed.
        90	    #[error("read error: {0}")]
        91	    Io(io::Error),

Resolution: Annotate the field `#[source]` (or `#[from]`, which also gives callers `?` from `io::Error`); either keep `{0}` in the message and accept the duplicated text, or change the message to `"read error"` and let the chain carry the cause. Acceptance: a test that decodes from a failing reader asserts `err.source().is_some()`.
Construction: A `Read` impl whose `read` returns `Err(io::Error::other("boom"))`; `Version::decode(reader)` yields `Decode::Io`, and `std::error::Error::source(&err)` is `None` today.

Synthesis: **adopt**, as `Io(#[source] io::Error)` with the Display string unchanged, per crate-root-15's reasoning: the crate wants explicit construction at its seven `map_err(Decode::Io)` sites, so `#[from]` buys nothing, and keeping the string leaves the golden at `testing/snapshots.rs:276` untouched. The same defect is filed under correctness as api-audit-2 and under idiom as crate-root-15; one commit closes all three. Synthesis note on the provenance line: it names thiserror 1.0.69, but before's dependency is `thiserror = "2"` through the workspace table (Cargo.toml:76, crates/before/Cargo.toml:29) and resolves to 2.0.18; the root lock's 1.0.69 is reached only by the termwiz/wezterm crates (verified here by reading Cargo.lock). The source-field rule is the same on both lines, so the conclusion stands unchanged.

### Party and Clock, with the fork iterators

### api-audit-1: Party and Clock carry no #[must_use]; a discarded fork or seed leaks id space silently
- Where: crates/before/src/party.rs:233-237 (related: party.rs:66, party.rs:121, party.rs:458, party.rs:534-536, clock.rs:52, clock.rs:84, clock.rs:153-157, clock.rs:590, clock.rs:610, clock.rs:890-895, party/ops/build.rs:39)
- Class / severity / confidence: api-surprise / medium / high
- Provenance: verified (grep for `must_use` over both crates' `src/`; read every producing method; read `core/src/option.rs:586-589` and `core/src/result.rs:544-548` in the 1.88 toolchain source); executed: no
- Verification: reframed: the sweep said `Party::without` is "already covered by `Option`/`Result`'s built-in must_use"; `Result` has `#[must_use = "this `Result` may be an `Err` variant, which should be handled"]` and `Option` has no `#[must_use]`, so `without` needs its own attribute. The rest holds; history: no-rationale-found
- Owner-gated: yes: the attribute changes what compiles warning-free for consumers under `-D warnings`, so it is an API decision even though the signatures do not move

`Party::fork` and `Clock::fork` return the new share by value with no
`#[must_use]` on the method or on the type, so `parent.fork();` compiles
silently: the parent has already narrowed and the child's share drops. The
crate's only `#[must_use]` is on an internal build token. The same hole covers
`seed`, `from_parts`, `into_parts`, `dangerously_alias`, and `without`.

Evidence:

       233	    pub fn fork(&mut self) -> Party {
       234	        let (keep, give) = self.view().split();
       235	        *self = Party::from_bits(keep);
       236	        Party::from_bits(give)
       237	    }

       458	    pub fn without(self, other: &Party) -> Option<Party> {

    crates/before/src/party/ops/build.rs:39:#[must_use = "an opened node must be closed with close_node"]

The failure is a resource leak, not a safety-rule violation: disjointness
survives, but the dropped share is unrecoverable and the parent's tree deepens
(the hazard `Party::fork`'s own `# Warning` at party.rs:212-218 describes). The
doctrine the crate leans on is that the compiler, not a careful reader, catches
linearity slips; this is the one slip of that family it does not catch.

Resolution: `#[must_use = "..."]` on `pub struct Party` (party.rs:66) and `pub struct Clock` (clock.rs:52), which covers every by-value producer including the `(Party, Version)` tuple from `into_parts` (rustc checks tuple elements); add a separate `#[must_use]` on `Party::without`, because `Option<T>` is not inspected for a must_use payload. Acceptance: `let mut p = Party::seed(); p.fork();` and `p.without(&q);` each produce `unused_must_use`; the crate's doctests, tests, and the rumors consumer compile warning-free.

Synthesis: **adopt.** The type-level attribute is the right altitude: the hazard belongs to the value, not to one producer, and clippy-pedantic-4's grep found no statement-position `fork()` or `dangerously_alias()` in before, suanpan, or rumors, so the required checks stay clean under `-D warnings`. A type-level `#[must_use]` also fires on a discarded `dangerously_alias()`, which is a legitimate warning: an alias nobody reads has no purpose. clippy-pedantic-4 below is the same mechanism widened to the pure combinators; one commit lands both.

### clippy-pedantic-4: The linear types `Party` and `Clock` and the pure `Version`/`Rank` combinators carry no `#[must_use]`
- Where: crates/before/src/party.rs:233-237 (related: crates/before/src/clock.rs:153, crates/before/src/clock.rs:890, crates/before/src/party.rs:534, crates/before/src/version.rs:459, crates/before/src/version.rs:489, crates/before/src/version.rs:519, crates/before/src/version.rs:553, crates/before/src/version/rank.rs:354, crates/suanpan/src/accumulator.rs:1169, crates/before/src/party/ops/build.rs:36-40, crates/before/src/oracle/clock.rs:47, crates/before/src/oracle/party.rs:86, crates/before/src/oracle/party.rs:175)
- Class / severity / confidence: api-surprise / low / high
- Provenance: assessed (read each signature; grepped `must_use` across both crates; grepped statement-position `fork()`/`dangerously_alias()` in before, suanpan, and rumors, and statement-position `join`/`meet` in rumors); executed: no
- Verification: confirmed; the `return_self_not_must_use` hit list also names the oracle twins (oracle/clock.rs:47, oracle/party.rs:86 and 175), feature-gated test apparatus the sweep did not list. No site in before, suanpan, or rumors discards a `fork()` or `dangerously_alias()` result, and rumors' statement-position `join` calls are all `Tree::join`, so adopting the attributes fails nothing under the `-D warnings` gate; history: deliberate-and-holds for the one existing `#[must_use]` (`Open`, with its reason string), no-rationale-found for its absence elsewhere
- Owner-gated: yes (a public-API attribute; AGENTS.md's stable-API rule makes it a suggestion)

`Party::fork(&mut self) -> Party` and `Clock::fork(&mut self) -> Clock` halve
the receiver and return the other half; `p.fork();` permanently loses that
region with no diagnostic. The crate guards duplication with `!Clone` (an
AGENTS.md hard rule) but not its dual, loss. `Version::join`, `join_all`,
`meet`, `meet_all` and `Rank::saturating_sub` are pure and return a fresh value,
so an unused result is always a bug (std marks integer `saturating_sub`
`#[must_use]`). Types-first: the compiler, not a careful reader, should catch a
discarded linear resource.

Evidence:

       233	    pub fn fork(&mut self) -> Party {
       234	        let (keep, give) = self.view().split();
       235	        *self = Party::from_bits(keep);
       236	        Party::from_bits(give)
       237	    }

    crates/before/src/clock.rs
       153	    pub fn fork(&mut self) -> Clock {
    crates/before/src/version.rs
       459	    pub fn join(&self, other: &Version) -> Version {
    crates/before/src/version/rank.rs
       354	    pub fn saturating_sub(&self, other: &Rank) -> Rank {
    crates/before/src/party/ops/build.rs (the crate's one existing must_use)
        39	#[must_use = "an opened node must be closed with close_node"]

Resolution (suggestion): a type-level `#[must_use = "..."]` on `Party` and
`Clock` naming the loss, and method-level `#[must_use]` on `Version::join`,
`join_all`, `meet`, `meet_all` and `Rank::saturating_sub`.
`Accumulator::merge_into_wider` (suanpan accumulator.rs:1169) returns a spare
buffer whose drop is legitimate; leave it, or annotate with a reason string
saying so. Acceptance: `just clippy` clean with the attributes in place, and a
doctest or unit test that `p.fork();` warns is unnecessary (the attribute is
the mechanism).

Synthesis: **adopt**, together with api-audit-1 (the type-level half is the same suggestion; this entry adds the method-level attributes on the pure combinators, and `Rank::checked_sub` at rank.rs:298 belongs on the same list since an unused `Option<Rank>` is equally a bug). Leave `merge_into_wider` unannotated, as the entry says: dropping the spare buffer is the documented pool contract's legitimate outcome, and suanpan-tests-8 below asks a different question of that return.

### api-audit-6: The fork iterators are defined as `Forks` but reachable only as `iter::Clock`/`iter::Party`; rustdoc shows a name the user cannot import
- Where: crates/before/src/iter.rs:20-20 (related: clock.rs:17, clock.rs:178, clock/forks.rs:20, party.rs:29, party.rs:258, party/forks.rs:93)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (read the definitions and re-exports; inspected the sweep's rendered `struct.Clock.html`: the method signature renders `pub fn forks(&mut self, k: u64) -> Forks<'_>` with the return type linked as `<a class="struct" href="iter/struct.Clock.html" title="struct before::iter::Clock">Forks</a>`, and the prose renders `Children are built on demand; see <a href="iter/struct.Clock.html" ...><code>Forks</code></a>`); executed: yes: python extraction from the rendered HTML
- Verification: confirmed; history: no-rationale-found
- Owner-gated: yes: any rename is a breaking change under the stable-API rule

The same type has two names: `Forks` where it is defined (private modules) and
`iter::Clock`/`iter::Party` where it is public. Rustdoc renders the signature
and the "see [`Forks`]" sentences with the unreachable name. `use before::Forks`
fails to resolve, and `use before::iter::Clock` shadows `before::Clock`.

Evidence:

        20	pub use crate::{clock::Forks as Clock, party::Forks as Party};

       178	    /// Children are built on demand; see [`Forks`] for the per-step and early-drop costs.

Resolution: owner's choice between (a) naming each struct as it is exported (define `ClockForks`/`PartyForks`, or define both in `iter.rs`), which fixes the rendered signature, or (b) keeping the names and rewriting the two "see [`Forks`]" sentences (clock.rs:178, party.rs:258) to the public path `[`iter::Clock`](crate::iter::Clock)` / `[`iter::Party`](crate::iter::Party)`, which fixes the prose only. Acceptance: (a) the rendered return type is a path a user can `use`; (b) at minimum the prose names the public path.

Synthesis: **adopt (b) now; consider (a)** at the next breaking version. The re-export names were chosen so that `iter::Party` and `iter::Clock` read as "the iterator over parties / clocks" in the `std::iter` idiom, and that reading survives only if the user meets the type through the module path; the rendered signature defeats it by showing the definition's name. (a) fixes the root cause and is the better end state, but a public re-export rename is breaking under the stable-API rule, so it waits for a version that is allowed to break. The same item was found by the fresh-eyes application (fresh-eyes-9, next); the two documentation nits in the same module (api-audit-5, fresh-eyes-11: the stray backtick in `iter.rs:10`) go in the same commit.

### fresh-eyes-9: The fork iterator's documented name is not importable
- Where: crates/before/src/clock.rs:178-194 (related: crates/before/src/party.rs:274-276, crates/before/src/iter.rs:1-2 and 20, crates/before/src/clock/forks.rs:7 and 20, crates/before/src/party/forks.rs:78 and 93)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (the sweep's check1.log line 36: `error[E0425]: cannot find type 'Forks' in crate 'before'`; this pass read the rustdoc build at target/doc/before (dated Sep 1 22:43, later than the HEAD commit; the sources involved last changed Aug 14): `struct.Clock.html` renders `pub fn forks(&mut self, k: u64) -> Forks<'_>` with the type linked to `iter/struct.Clock.html`, whose title is "Clock in before::iter"); executed: no build was run by this pass; the rendered HTML was read
- Verification: confirmed, and the sweep's open question about the rendered name is settled: rustdoc shows the declared name `Forks`, which no public path spells; history: no-rationale-found (a431eaf1d "Doc editing" added the re-exports)
- Owner-gated: yes: renaming a public re-export reshapes the API; the doc sentence alone is not gated

`Clock::forks` and `Party::forks` render as returning `Forks<'_>`, and the prose says "see [`Forks`]", but the only public paths are `before::iter::Clock` and `before::iter::Party`: `before::Forks` does not exist, and the link lands on a page called `Clock`, a name the reader has already reserved for the clock type.

Evidence:

       178	    /// Children are built on demand; see [`Forks`] for the per-step and early-drop costs.
       ...
       192	    pub fn forks(&mut self, k: u64) -> Forks<'_> {
       193	        Forks::new(self, k)
       194	    }

        20	pub use crate::{clock::Forks as Clock, party::Forks as Party};

Resolution: Either (owner-gated) re-export under the declared names, for example `iter::ClockForks` and `iter::PartyForks`, so the signature, the prose, and the import agree, or add one sentence to `Clock::forks` and `Party::forks` giving the importable spelling (`before::iter::Clock`, `before::iter::Party`). Acceptance: the return type a reader copies from the rendered signature is a path `use before::...` accepts, or the method docs state the path that is.

Synthesis: same item as api-audit-6, reached independently by the scratch application; one disposition covers both (**adopt** the doc sentence; **consider** the rename). Kept as its own entry because its provenance is the compile error the user hit, which is the evidence the audit's rendered-HTML read lacked. Two neighbouring findings on the same iterators are not API shape and live elsewhere: `Forks::len()` panics on 32-bit targets past `usize` shares (clock-17, party-13, correctness), and the "exactly `k`" contract is false at `k == u64::MAX` (api-audit-10, party-14, inventory-13, documentation).

### fresh-eyes-15: Clock::join hands back a Clock in Err, which does not compose with `?`
- Where: crates/before/src/clock.rs:196-218 (related: crates/before/src/clock.rs:229-237 (`join_all`, `Err(Vec<Clock>)`), crates/before/src/clock.rs:298-322 and 377 (`sync`/`sync_all`, `Err(Overlap)`), crates/before/src/lib.rs:65-68)
- Class / severity / confidence: api-surprise / nit / high
- Provenance: verified (the sweep's check1.log line 170: `error[E0277]: '?' couldn't convert the error: 'before::Clock: std::error::Error' is not satisfied`; this pass read the four signatures and grepped the crate and its tests for a `map_err(|_| Overlap)` idiom: none is shown anywhere); executed: yes, by the sweep's compile probe, matched to its log
- Verification: confirmed; history: no-rationale-found beyond the docs' own "handed back in the error"
- Owner-gated: yes for an `Overlap`-returning variant; no for documenting the idiom

`join` returns `Result<&Version, Clock>` and `join_all` returns `Result<&Version, Vec<Clock>>`; neither error type is an `Error`, so `?` into `Box<dyn Error>` fails, while `sync`/`sync_all` return `Overlap`. The drop-nothing hand-back is a sound design; the inconvenience is that the two halves of the join family differ and no doc shows the bridge.

Evidence:

       196	    /// Absorbs a *disjoint* [`Clock`]'s [`Party`] and [`Version`], returning
       197	    /// the new [`Version`] of `self`.
       198	    ///
       199	    /// # Errors
       200	    ///
       201	    /// If the two clocks' [`Party`]s overlap, `self` is unmodified and `other`
       202	    /// is handed back in the error.
       ...
       218	    pub fn join(&mut self, other: Clock) -> Result<&Version, Clock> {

Resolution: Add to `join`/`join_all`'s `# Errors` one sentence with the propagation idiom (`.map_err(|_| Overlap)?` when the handed-back clock is not wanted), or (owner-gated) offer an `Overlap`-returning variant beside the hand-back form. Acceptance: the `join` docs show how to propagate the error as an `Error`.

Synthesis: **adopt** the documented idiom; **decline** the `Overlap`-returning variant. The difference between the two halves of the family is not an accident: `sync` keeps both clocks alive, so it has nothing to hand back and `Overlap` is the whole error, while `join` consumes `other`, and an `Overlap`-returning `join` would have to drop the un-absorbed clock inside its error path, which is exactly the id-space leak api-audit-1 asks the compiler to catch. The `# Errors` sentence should say that too ("the clock comes back so its share is not lost; map it to `Overlap` only when you mean to drop it"), so the idiom carries its own hazard. The public docs' own `unwrap` at lib.rs:65-68 is the right place for the first mention.

### Version core: Ticks

### fresh-eyes-14: Ticks converts out to u64 only by reference
- Where: crates/before/src/version/ticks.rs:185-190 (related: crates/before/src/version/ticks.rs:151-164 and 192-197 (the `From` impls in), crates/before/src/version/ticks.rs:21 (the docs, which do say `TryFrom<&Ticks>`), crates/before/src/shape.rs:54-56 (an example that works because it matches on `&plateau.rise`))
- Class / severity / confidence: api-surprise / nit / high
- Provenance: verified (the sweep's check1.log line 117: `error[E0277]: the trait bound 'u64: TryFrom<Ticks>' is not satisfied`; this pass read every `From`/`TryFrom` impl in ticks.rs); executed: yes, by the sweep's compile probe, matched to its log
- Verification: confirmed; history: no-rationale-found
- Owner-gated: yes: an API addition

Only `TryFrom<&Ticks> for u64` exists; `u64::try_from(ticks)` by value fails to compile with std's misleading hint about `From<Ticks>`, and no conversion to `u128` or `usize` exists although `From<u128>` and `From<usize>` go in. The docs do say `TryFrom<&Ticks>`, so this is a shape surprise, not a doc error.

Evidence:

       185	impl TryFrom<&Ticks> for u64 {
       186	    type Error = TooWide;
       187	    fn try_from(count: &Ticks) -> Result<u64, TooWide> {
       188	        count.0.to_u64().ok_or(TooWide)
       189	    }
       190	}

Resolution: Owner-gated: add `impl TryFrom<Ticks> for u64` delegating to the reference form, and consider `u128`/`usize` duals for the `From` impls that exist. Acceptance: `u64::try_from(Ticks::from(1u64))` compiles.

Synthesis: **adopt**, as part of api-audit-13 (next), which states the whole family; this entry is the compile error a user hits first. `Ticks` is `Clone` and small, so the by-value impl costs one line delegating to the reference form. The one extension I would **decline** is `From<suanpan::UBig> for Ticks`, which the envelope suite's sixteen `.to_string().parse::<Ticks>()` sites make tempting (envelopes-b-19, simplification; meter-core-5, idiom): meter-core's history pass records that the string entry point is the deliberate design, keeping suanpan's type off before's stable surface, and a test-local helper serves the tests.

### api-audit-13: Ticks out-conversions and subtraction are narrower than its in-conversions and than Rank's
- Where: crates/before/src/version/ticks.rs:185-190 (related: ticks.rs:15-24, ticks.rs:152-164, ticks.rs:193-197, error.rs:37-54, version/rank.rs:298, version/rank.rs:354)
- Class / severity / confidence: feature-gap / low / high
- Provenance: verified (read ticks.rs in full: `From` for u8, u16, u32, u64, u128, usize; one `TryFrom<&Ticks> for u64`; `Add`/`AddAssign`/`Sum` only; read rank.rs:278-356 for the subtraction pair); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: yes: additive impls and methods on a stable type

`Ticks` converts in from six unsigned widths but out only by reference to one
target, so `u64::try_from(v.min_ticks())` (by value) and any `u128`/`usize`
target fail to compile, while `TooWide`'s docs speak generically of "the
machine integer it was converted into". There is no `checked_sub` or
`saturating_sub` where `Rank` has both, so `b.min_ticks() - a.min_ticks()` is
unspellable.

Evidence:

       185	impl TryFrom<&Ticks> for u64 {
       186	    type Error = TooWide;
       187	    fn try_from(count: &Ticks) -> Result<u64, TooWide> {
       188	        count.0.to_u64().ok_or(TooWide)
       189	    }
       190	}

Resolution: `TryFrom<Ticks>` by value and `TryFrom<&Ticks>` for u128/usize/u32; `checked_sub`/`saturating_sub` mirroring `Rank`'s. Acceptance: `u64::try_from(Ticks::from(1u8))` compiles; `Ticks::checked_sub` exists with a doc example.

Synthesis: **adopt.** Every piece is additive and follows a naming grammar the crate already uses (`Rank::checked_sub`/`saturating_sub` at rank.rs:298 and :354; the `TryFrom<&Ticks>` form with `TooWide`). The `Ticks` type doc at ticks.rs:18-24 already promises that "every conversion *out* is explicit about width", which reads as a family and today names one member; the `TooWide` doc at error.rs:37-43 speaks of "the machine integer it was converted into" in the same generic voice. A user's first use of the count is arithmetic between two versions (how many events separate them), and `Ticks` has `Add` and `Sum` but no subtraction, so the gap is met early. Owner-gated; both are suggestions.

### Rank

### rank-21: Display honors formatter flags only for integral ranks
- Where: crates/before/src/version/rank.rs:1083-1087 (related: crates/before/src/version/rank/num.rs:505-512, crates/before/src/codec/base.rs:297-301)
- Class / severity / confidence: api-surprise / nit / high
- Provenance: verified (the `exp == 0` arm hands `f` to `Num`'s `Display`, which for the base arm delegates to `UBig`'s and for the wide arm uses `f.pad_integral`; the `exp >= 1` arms use `write!(f, "{}/2...", ...)`, whose inner placeholders take fresh default formatters); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale found (the `write!` arms date to 18206f21; 79a944ab added flag handling to `Num` alone; no test uses width or fill)
- Owner-gated: yes (which contract to fix to)

`format!("{:>6}", five)` pads while `format!("{:>6}", half)` does not; the doc ("Renders as the exact rational") states neither contract. A caller aligning a column of ranks will hit the inconsistency.

Evidence:

      1083	        match self.exp {
      1084	            0 => Display::fmt(&self.num, f),
      1085	            1 => write!(f, "{}/2", self.num),
      1086	            exp => write!(f, "{}/2^{}", self.num, exp),
      1087	        }

Resolution: Render all three forms to a `String` and route through one `f.pad_integral(true, "", &text)` (or `f.pad`), or state in the `Display` doc that formatter flags are ignored. Acceptance: a unit test formatting `{:>6}` over `ZERO`, an integral rank, and a fractional rank asserts the same padding behavior for all three, or the doc states the contract.

Synthesis: **adopt** the uniform-flags branch. A type whose `Display` honours width and alignment for some values and not others is worse than one that ignores flags everywhere, because the caller who tests on integers ships a misaligned column. Of the two spellings, `f.pad(&text)` is the one for the fractional forms (`pad_integral` applies sign and zero-padding rules that mean nothing for `19/2^4`); the integral arm can keep delegating to `Num`, whose wide arm already uses `pad_integral`. The `Display` doc should then say which flags apply where. Behaviour under flags is observable, hence owner-gated, but no caller that formats without flags sees a change.

### api-audit-14: Rank::decode labels its representation bound Decode::NotCanonical, and neither error enum is #[non_exhaustive]
- Where: crates/before/src/version/rank.rs:442-444 (related: version/rank.rs:727-732, error.rs:86-88, error.rs:67-68, error.rs:106-107)
- Class / severity / confidence: api-surprise / low / medium
- Provenance: verified (read rank.rs:425-465 and 716-735, error.rs in full; `grep -rn non_exhaustive crates/before crates/suanpan`: none); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: yes: a new variant or `#[non_exhaustive]` changes what exhaustive matches compile

An integral header of 64 or more unary bits is rejected as `NotCanonical`, whose
variant doc says the structure is well-formed but not in canonical normal form.
The input may be perfectly canonical; the failure is the decoder's bound. The
corner itself is the sanctioned infeasible-work kind (2 EiB of input), so this
is about the label, and about the fact that no variant can be added later
without breaking matches.

Evidence:

       442	    /// - [`Decode::NotCanonical`] when otherwise valid content exceeds the type's
       443	    ///   representation bound (an integral mantissa of `2⁶⁴` or more bits, effectively
       444	    ///   unreachable, since it can only be hit by reading inputs of 2 EiB or more);

        86	    /// The structure is well-formed but not in canonical normal form.
        87	    #[error("input is not canonical")]
        88	    NotCanonical,

Resolution: either a dedicated variant now, or `#[non_exhaustive]` on `Decode` and `Parse` before the first release so one can land later; at minimum extend `NotCanonical`'s variant doc to name the representation-bound case. Acceptance: a decode that fails the representation bound is distinguishable by variant, or the variant doc names both cases.

Synthesis: **adopt** the variant-doc extension; **consider** `#[non_exhaustive]` on `Decode` alone, decided before the first release; **decline** a dedicated variant now. The doc fix is unconditional: `NotCanonical`'s doc at error.rs:86 should say that representability on the target is part of what the decoders admit, which the codec-bits review reached from the other side (its open question 9: 32-bit gamma widths that fit no `usize` land in the same variant, and the genre is the value's, not the machine's). A dedicated variant would exist only to name a corner no canonical input reaches and would itself be the breaking change; `#[non_exhaustive]` on `Decode` is the cheap insurance that lets such a variant land later, and a wire-decoding error set is the kind that grows. `Parse` is closed by the notation's grammar (three variants: syntax, canonicality, anonymity) and gains nothing from the attribute. rumors already matches `Decode` with a wildcard arm (it maps `Decode::Io` and wraps every other defect, per the dependence sweep), so the attribute costs the one known consumer nothing. The same `# Errors` clause is rewritten for a different reason in rank-10 (claim class: a nine-byte header reaches the arm, so "2 EiB" describes the canonical stream, not the rejection); land both rewordings together.

### fresh-eyes-16: Rank renders to text but does not parse
- Where: crates/before/src/version/rank.rs:1059-1060 (related: the four `FromStr` impls at crates/before/src/clock.rs:942, crates/before/src/version.rs:1454, crates/before/src/party.rs:784, crates/before/src/version/ticks.rs:205)
- Class / severity / confidence: feature-gap / nit / medium
- Provenance: verified (the sweep's run2.log records `rank of (0, 1, 0) displays as 1/2`; this pass grepped every `FromStr` impl in the crate: `Clock`, `Version`, `Party`, `Ticks`, and no `Rank`); executed: yes, by the sweep's scratch run, matched to its log
- Verification: confirmed; history: no-rationale-found
- Owner-gated: yes: an API addition

`Party`, `Version`, `Clock`, and `Ticks` have both `Display` and `FromStr`; `Rank` has `Display` (`5`, `1/2`, `19/2^4`) and no `FromStr`, so a rank logged or stored as text cannot be read back except through `encode`, and the docs do not say the text form is one-way.

Evidence:

      1059	/// Renders as the exact rational: the numerator alone when integral,
      1060	/// `num/2` if `exp = 1`, `num/2^exp` otherwise.

Resolution: Owner-gated: add `FromStr for Rank` over the Display grammar with the same strict rejection of non-normal spellings, or state on the `Display` impl that the text form is one-way. Acceptance: either `"19/2^4".parse::<Rank>()` round-trips, or the `Display` docs say the form does not parse.

Synthesis: **adopt** the one-sentence statement now; **consider** `FromStr` together with the human-readable serde question (fresh-eyes open question 1, carried below). A `FromStr` that rejects non-normal spellings (`2/2`, `3/2^0`, `4/2^2`) the way the other four text entry points reject non-canonical notation is a small parser, but it is only worth its roster row if the text form is meant as an interchange form; if the answer to the serde question is that human-readable serializers should emit the paper notation, `Rank`, `Ranked`, and `Span` all need text forms and this becomes part of that work rather than a lone addition. Until then, saying the form is one-way costs nothing and closes the surprise.

### Span and causally

### span-causally-1: `Span` derives `Eq` but not `Hash`, unlike `Version` and every verdict type beside it
- Where: crates/before/src/span.rs:104-108 (related: crates/before/src/span/verdict.rs:7, 24, 56, 74; crates/before/src/causally/query.rs:51; crates/before/src/version.rs:97-106; crates/before/surfacecheck/src/census.rs:156-171)
- Class / severity / confidence: api-surprise / nit / medium
- Provenance: verified (read the derive lists; grep of the surfacecheck census shows `Eq`/`PartialEq` rows for `Span` and no `Hash` row); executed: no
- Seen by: correctness; refutation: confirmed; history: no rationale found (db9dfa3e's absent-`Hash` decision concerned `Query`, not `Span`)
- Owner-gated: yes: an additive public API change on a stable API, and a census change

`Span`'s equality is byte equality of two canonical streams, and `Version: Hash` hashes exactly those bytes (`canonical_hash`), so `#[derive(Hash)]` over the two `Cow<Version>` fields would be consistent with `Eq` by construction. `Placement`, `Endpoint`, `Dominance`, `Precedence`, and `Coverage` all derive `Hash`; a consumer can key a map by version but not by span. Suggestion only: before's API is stable.

Evidence:

       104	#[derive(Debug, Clone, PartialEq, Eq)]
       105	pub struct Span<'a> {
       106	    lo: Cow<'a, Version>,
       107	    hi: Cow<'a, Version>,
       108	}

Resolution: if the owner agrees, add `Hash` to the derive, add the census row, and extend a byte-equality law (the `version_eq_iff_bytes_eq` family) to spans so equal spans hash equal. Acceptance: `HashSet<Span<'static>>` compiles; the law is green; the census diff shows one added row.

Synthesis: **adopt.** I re-read the derive lists at `9e5784fb`: `Span` derives `Debug, Clone, PartialEq, Eq`; all four verdict enums and `Coverage` derive `Hash`; `Version`'s manual `Hash` (version.rs:97-106) hashes the canonical bytes "consistently with `Eq`'s byte compare". A `Cow<Version>` hashes as its `Version`, so the derived `Hash` on `Span` is the composition of two consistent hashes and needs no manual impl. The span-causally partition's own open question 8 recommends the same. api-audit-11 (next) is the same suggestion extended to the shape types.

### api-audit-11: Span (and shape::Plateau/Rise/Region) lack Hash while every other value type in the crate has it
- Where: crates/before/src/span.rs:104-108 (related: shape.rs:87, shape.rs:107, shape.rs:117, version.rs:89)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (read the derives; the sweep's rendered `struct.Span.html` contains zero `impl-Hash` anchors and `struct.Version.html` three); executed: yes: python count over the rendered HTML
- Verification: confirmed; history: no-rationale-found
- Owner-gated: yes: additive trait impl on a stable type

`Span` derives `Debug, Clone, PartialEq, Eq` but not `Hash`; `Version`, `Party`,
`Clock`, `Rank`, `Ranked`, `Ticks`, and the four verdict enums are all `Hash`.
`Span`'s `Eq` is byte equality on two `Cow<Version>` endpoints and `Version`
has a manual `Hash` consistent with that equality, so `#[derive(Hash)]` is
consistent. `shape::Plateau`, `Rise`, and `Region` omit it likewise though
`Ticks` (Rise's payload) has it. A `HashSet<Span>` does not compile.

Evidence:

       104	#[derive(Debug, Clone, PartialEq, Eq)]
       105	pub struct Span<'a> {
       106	    lo: Cow<'a, Version>,
       107	    hi: Cow<'a, Version>,
       108	}

Resolution: derive `Hash` on `Span`, `Plateau`, `Rise`, and `Region`; add a law that `a == b` implies equal hashes for `Span`. Acceptance: `HashSet<Span<'static>>` compiles and the law holds under proptest.

Synthesis: **adopt**, with span-causally-1; the shape types are plain data (`Plateau` and `Region` derive `Clone, Debug, PartialEq, Eq`, `Rise` adds `Copy`; verified at shape.rs:87, 107, 117), so the derive is mechanical and the census gains four rows. One caution that is not this finding's: the shape types' *meaning* is under dispute in crate-root-38 (documentation, medium: "plateau" is defined as a maximal constant run while the walk yields canonical leaves, which can be adjacent and equal); deriving `Hash` does not depend on which definition wins, since equality is structural either way, but the two changes touch the same declarations and should be reviewed together.

### fresh-eyes-6: The causally atoms expose contains but not coverage
- Where: crates/before/src/causally/forms.rs:277-299 (related: crates/before/src/causally/forms.rs:301-315, crates/before/src/causally/convert.rs:11-34, crates/before/src/causally/query.rs:126, crates/before/src/lib.rs:34 and 281-288)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (the sweep's check1.log lines 158 and 164: `error[E0599]: no method named 'coverage' found for struct 'Floor<'a>'` and the same for `Ceiling<'a>`; this pass read the two impl blocks: `contains` and `or_concurrent` only, and `causally.rs`'s module doc never mentions `Query::from` or `.into()`); executed: yes, by the sweep's compile probe, matched to its log
- Verification: confirmed; history: no-rationale-found
- Owner-gated: yes for adding methods; no for the one-sentence pointer

`Floor` and `Ceiling` (what `after` and `before` return) answer `contains` but not `coverage`; the crate docs present them as queries, so the natural first attempt fails to compile, and the `Query::from(Floor)` bridge (convert.rs:13) is documented only on the impl, which the stuck reader is not looking at.

Evidence:

       277	impl<'a> Floor<'a> {
       278	    /// Whether `version` is at or above the bound: `at <= v`.
       ...
       283	    pub fn contains(&self, version: &Version) -> bool {
       284	        le(&self.at, version)
       285	    }
       286	
       287	    /// Widens the query to additionally include all concurrent versions.
       288	    pub fn or_concurrent(self) -> Query<'a, Down> {

Resolution: Either add `coverage` to `Floor` and `Ceiling` (owner-gated), or add one sentence to `after`, `before`, `Floor`, and `Ceiling` saying a bare atom converts into a neutral `Query` with `Query::from` (or `.into()`) for `coverage`. Acceptance: a doctest on `after` calls `coverage` on the atom, directly or through the documented conversion.

Synthesis: **adopt** the pointer now (not gated); **consider** the method (gated, additive). The crate's front page (lib.rs:34) lists `coverage` among the query language's operations without distinguishing atoms from queries, and `Query::coverage` at query.rs:126 takes `impl Into<Span>`, so a reader who has `after(v)` in hand expects the same call. A delegating `Floor::coverage(&self, span) -> Coverage` that goes through `Query::from` is a two-line method per atom and keeps the polarity design untouched (a bare atom is neutral, per convert.rs:11-12), so I would add it; the one-sentence pointer stands on its own if the owner would rather keep the atom surface minimal.

## The codec

### Bits and buf

### codec-bits-11: BitsBuf::get panics where BitsView::get returns Option
- Where: crates/before/src/codec/buf.rs:127-135 (related: crates/before/src/codec/bits.rs:291-312; crates/before/src/codec/literal.rs:44)
- Class / severity / confidence: api-surprise / nit / high
- Provenance: verified (read both definitions); executed: no
- Seen by: structure [11]; refutation: confirmed; history: no-rationale-found (the two names arrived in consecutive commits, 83e61b4d then 5d167a63)
- Owner-gated: no (crate-private)

`BitsBuf::get(pos) -> bool` asserts `pos < live`; on the sibling storage form `BitsView::get(pos) -> Option<bool>` is the bounded read and `BitsView::bit(pos) -> bool` the asserting one. A reader of `bits.get(0)` must know which type is in hand to know whether the call can panic.

Evidence:

       127	    /// The bit at `pos`.
       128	    ///
       129	    /// # Panics
       130	    ///
       131	    /// `pos` must be below the live length.
       132	    pub(crate) fn get(&self, pos: u64) -> bool {
    (bits.rs)
       291	    /// The bit at `pos`, or `None` at or past the live length: the
       292	    /// sequential cursors' bounded read.
       293	    pub(crate) fn get(&self, pos: u64) -> Option<bool> {

Resolution: Rename `BitsBuf::get` to `bit` (callers: literal.rs:44, buf.rs internals, test files). Acceptance: both storage forms spell the asserting read `bit` and the bounded read `get`.

Synthesis: **adopt.** Crate-private, so no API question arises; the two storage forms should share one naming grammar (`bit` asserts, `get` is bounded), matching std's `get` convention, and the rename's only production caller is `literal.rs:44` (`bits.get(0)`/`bits.get(1)` on a two-bit buffer, verified at `9e5784fb`).

### Base, text, and tree

### codec-base-text-tree-18: The id and version text doors disagree on whether trailing junk outranks `NotCanonical`
- Where: crates/before/src/codec/text.rs:161-169 (related: crates/before/src/codec/text.rs:79-81, crates/before/src/version/skyline/text.rs:488-494, crates/before/src/version/skyline/text.rs:617-622, crates/before/src/codec/tests.rs:1709-1711, crates/before/src/error.rs:106-121)
- Class / severity / confidence: api-surprise / nit / high
- Provenance: assessed (read both parsers' close and trailing-check order and the `Parse` docs); executed: no
- Seen by: correctness; refutation: confirmed; history: no rationale found (before 32a655438 both doors deferred `NotCanonical` behind the trailing-junk check; that commit moved the id door's check to node close without mentioning precedence, and e83ef600 and 6aa6fe54 then pinned each door separately)
- Owner-gated: yes: `FromStr` error variants on public types

The id parser returns `Parse::NotCanonical` the moment a collapsible node closes, before the trailing-input check at 79-81; the skyline parser defers `NotCanonical` until after the whole syntax pass, so trailing junk outranks it there. One notation, one error type, two precedence rules, and `Parse`'s docs state neither. The reference parser mirrors the id door's rule, so the differential cannot see the cross-door divergence.

Evidence:

       161	                Some(IdFrame::NeedRight { tag, left }) => {
       162	                    if cur.bump() != Some(b')') {
       163	                        return Err(Parse::Syntax);
       164	                    }
       165	                    match (left, kind) {
       166	                        (IdKind::Empty, IdKind::Empty) => return Err(Parse::NotCanonical), // (0, 0)
       167	                        (IdKind::Terminal, IdKind::Terminal) => {
       168	                            return Err(Parse::NotCanonical); // (1, 1)
       169	                        }

    (skyline/text.rs)
       491	/// sibling leaves) is checked at each close and reported after the whole syntax
       492	/// pass, so syntax errors — including trailing junk — outrank
       493	/// [`Parse::NotCanonical`].

       617	    if cursor.peek().is_some() {
       618	        return Err(Parse::Syntax); // trailing junk
       619	    }
       620	    if !canonical {
       621	        return Err(Parse::NotCanonical);
       622	    }

Resolution: Owner ruling on which rule wins, then align: either defer `NotCanonical` in `parse_id_tree` (a `canonical` flag reported after the trailing check, as the skyline parser does, mirrored in `ref_parse_id_node`), or document the per-node rule on `Parse` and in the skyline parser. Add one cross-door pin with the same trailing-junk-plus-collapsible text through both doors. Acceptance: `"(1, 1) x".parse::<Party>()` and `"(0, 5, 5) x".parse::<Version>()` return the same variant, and `Parse`'s rustdoc states the precedence.

Construction: `"(1, 1) x".parse::<Party>()` returns `Err(Parse::NotCanonical)` (line 168 fires at `)` before line 79 sees ` x`); `"(0, 5, 5) x".parse::<Version>()` returns `Err(Parse::Syntax)` (skyline/text.rs:617 fires before :620).

Synthesis: **adopt**, on the whole-pass rule the version parser already implements and documents (skyline/text.rs:491-493), which the codec-base-text-tree review also recommends (its open question 2): "only well-formed strings receive a canonicality judgment" is the simpler contract, it is what `Parse::Syntax`'s own variant doc at error.rs:108-109 already implies by listing "trailing input" as a syntax defect, and the id parser needs only a `canonical` flag to match, mirrored in the reference parser at codec/tests.rs:1709-1711 so the differential can see the rule. A `Clock` text parses an id and a version in one string, so a clock spelled with a collapsible id and trailing junk today reports a variant that depends on which component the defect sits in; the cross-parser regression test the resolution asks for should include that composite. Owner-gated because the variant a public `FromStr` returns moves for the id parser.

## suanpan

### suanpan-10: A zero wide operand spills the register and can flip a domination certificate
- Where: crates/suanpan/src/accumulator.rs:253-256 (related: 264-267, 324-327, 345-348, 374-377, 396-399, 557, 777-800; crates/suanpan/src/accumulator/tests/witnesses.rs:274-294; crates/suanpan/src/lib.rs:50-53)
- Class / severity / confidence: api-surprise / low / high
- Provenance: assessed (read and trace: register-held `2^80` certifies at floor 1 since `2^80 >= 3 * 2^64` (812-817); after `add_wide(&UBig::ZERO)` the spill deposits `2^16` at digit 2 and the fold decides at index 2 < floor + 2 = 3); executed: no
- Seen by: correctness; refutation: confirmed; history: deliberate-and-holds (spill by operand class, not value, is stated at lib.rs:50-53 and the `quick` field doc 95-98)
- Owner-gated: yes (a stated design rule; a lazy spill also changes touch counts, which lib.rs:283-286 makes a breaking change)

`add_wide`, `add_wide_shl`, `add_limbs_shl` (and the sub twins), and `add_accum` of a literally-zero digit-engine operand call `spill()` before discovering there is nothing to deposit, retiring the register for the epoch; `add_magnitude(&ZERO)` (which routes to `add_u64(0)`) does not. Within the documented contract (`decided` is representation-dependent), but a value-neutral call changing a verdict and costing deposit touches is a surprise the cheaper ordering avoids.

Evidence:

       253	    pub fn add_wide(&mut self, delta: &UBig) {
       254	        self.spill();
       255	        self.apply_limbs(Limbs::new(delta), false, 0);
       256	    }

Resolution: owner decision: spill lazily (move `self.spill()` into `apply_limbs` ahead of the first `add_at`, and skip it in `fold_accum`'s digit path when `other.is_literally_zero()`) and re-derive the affected pins as a named touch-count change; or state at the wide entry points that a zero wide operand retires the register. Acceptance: a witness that `add_wide(&UBig::ZERO)` on a register value leaves `quick.is_some()` and records 0 touches, or the doc states the behavior. Construction (passes today): `let mut a = Accumulator::new(); a.add_u64(1 << 20); a.shl(30); a.shl(30); assert_eq!(a.sign_dominates_at(1), (Ordering::Greater, true)); a.add_wide(&UBig::ZERO); assert_eq!(a.sign_dominates_at(1), (Ordering::Greater, false));`.

Synthesis: **consider**, contingent on suanpan's open question 1 (carried below): the exact-touch contract at lib.rs:283-286 declares any count change breaking, and a lazy spill changes counts on the zero-operand path. If that contract is restated as "deterministic and pinned; a count change is a versioned change named in its commit" (the suanpan review's recommendation), then **adopt** the lazy spill: `add_magnitude(&UBig::ZERO)` already leaves the register alone (it routes to `add_u64(0)`, a no-op), so the two entry points for the same value would agree, and a value-neutral call would no longer change the cost class of every later word-scale call. If the contract stands as written, adopt the documentation half at the six wide entry points. Either way the witness suanpan-tests-4 (verification-gap) asks for is owed, since today no test draws a zero wide operand at all. I re-read lib.rs:50-53 and accumulator.rs:777-800 at `9e5784fb`: the spill-by-operand-class rule and the "representation, not value" contract for `decided` are stated as the finding says, so this is a design decision to revisit, not a defect in the stated contract.

### suanpan-tests-8: merge_into_wider hands back an ordinary Accumulator whose value is unspecified; three test sites carry the reset obligation by convention
- Where: crates/suanpan/src/accumulator/tests/differential.rs:695-698 (related: crates/suanpan/src/accumulator.rs:1146-1149, crates/suanpan/src/accumulator/tests/metered.rs:297, crates/suanpan/src/accumulator/tests/metered.rs:390-391, crates/before/src/version/skyline/watermark.rs:395)
- Class / severity / confidence: api-surprise / nit / medium
- Provenance: assessed (read); executed: no
- Seen by: api-economics [35]; refutation: confirmed (before resets every drained buffer through `retire`, which also takes live accumulators); history: no-rationale-found (the unspecified-value classification was deliberate; a typed obligation was never weighed)
- Owner-gated: yes: a public API reshaping of a stable crate

The rustdoc declares the returned buffer "a valid accumulator holding an unspecified value", and three test sites plus before's `retire` carry the reset by convention. A caller who forgets it reads the narrower operand's digits as a fresh total with no compile-time or runtime signal. Types-first: a `Drained` newtype whose only exit is `reset(self) -> Accumulator` makes the missing reset unrepresentable and deletes the doc sentence and the three comments.

Evidence:

       695	        // The pool contract: a drained buffer re-arms to a clean zero.
       696	        let mut reused = drained;
       697	        reused.reset();
       698	        assert_value(&reused, &IBig::from(0));

Resolution: owner's call: return a newtype that can only become an `Accumulator` by resetting (before's `retire` would need a second entry or a `From`, since it also takes live accumulators); the roster gains one trivially excluded row. Acceptance: a forgotten reset fails to compile; the drained-buffer prose in `merge_into_wider`'s rustdoc is gone.

Synthesis: **consider.** The types-first argument is sound and the failure it removes is real (a drained buffer read as a total answers with the narrower operand's digits), but the newtype reshapes a stable public signature, before's `retire` at watermark.rs:395 takes live accumulators through the same entry and would need a second face, and the roster row is one more thing to price. Weigh it after suanpan-tests-16 (the swap inside `merge_into_wider` is on no metered path and can be deleted) lands, since that change simplifies the same method and may change what the return means; if the answer is no, the doc sentence at accumulator.rs:1146-1149 already states the obligation, so the status quo is documented rather than silent.

## The instruments: fuelscape

### fuelscape-pipeline-14: Preconditions unstated where the arithmetic relies on them: bit_window(0), sample_bytes past the table, ClockSlice at one byte
- Where: crates/before-fuelscape/src/count.rs:286-290 (related: crates/before-fuelscape/src/sample.rs:209-217, 402-409; crates/before-fuelscape/src/count.rs:181-183; crates/before-fuelscape/src/plan.rs:316; crates/before-fuelscape/src/ops.rs:118-128)
- Class / severity / confidence: api-surprise / nit / high
- Provenance: verified (read: `bit_window(0, _)` computes `8 * bytes - 1` and `bytes - 1` with no guard; `sample_bytes` returns `None` only when the window's counts sum to zero, which cannot happen for `bytes >= 1` since `bit_window(1, _) == 2..=7` and the 2-bit leaf and terminal are counted, while a `bytes` past the table indexes `self.subtree[bits]` out of bounds at count.rs:182 before the check; `draw_arity(size - 1, rng)` at plan.rs:316 hands `gen_range(1..=0)` an empty range at size 1; every plan-side caller writes `.expect(...)`); executed: no
- Seen by: adequacy [23], instrument-correctness [52]; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

Every panic message should be a one-line proof; here the unreachable failure has a channel (`Option`) and the reachable one (a size past the table, or zero bytes) has none, so the type misinforms the reader and the premise "callers pass `1 <= bytes <= span`" is named nowhere near the arithmetic. All three are guarded today by `Inputs::min_bytes` and `Samplers::build`, far from the sites.

Evidence:

    (count.rs)
       286	pub fn bit_window(bytes: usize, min_bits: usize) -> std::ops::RangeInclusive<usize> {
       287	    let hi = 8 * bytes - 1;
       288	    let lo = (8 * (bytes - 1)).max(min_bits);
       289	    lo..=hi
       290	}

    (sample.rs)
       209	    /// Draw one version uniformly from the canonical versions whose packed
       210	    /// encoding is exactly `bytes` bytes. `None` if the space is empty
       211	    /// (it is not, for any `bytes >= 1` within the table).
       212	    pub fn sample_bytes(&self, bytes: usize, rng: &mut ChaCha12Rng) -> Option<VersionDraw> {

Resolution: `assert!(bytes >= 1, "a packed encoding has at least one byte")` at the head of `bit_window`; have `sample_bytes` return the draw directly with a `# Panics` section stating `1 <= bytes <= span` (or check the span and make that the `None`), removing the seven `.expect` sites in plan.rs; at plan.rs:316 either assert `size >= 2` naming the `ClockSlice` minimum or route the cap through `Inputs::min_bytes`. Also use the `RangeInclusive` import already at count.rs:41 in the return type. Acceptance: no `.expect` on `sample_bytes` remains in plan.rs; `bit_window(0, _)` panics with the named message in both profiles.

Synthesis: **adopt.** A dev-tool workspace with no stability rule, so nothing is gated; the change makes the `Option` mean what it says (or removes it) and puts each precondition beside the arithmetic that relies on it, which is the one-line-proof discipline the rest of the tree follows. The `sample_bytes` doc at 209-211 already argues that `None` is unreachable within the table, which is the tell that the channel is carrying the wrong case.

### fuelscape-render-15: The dump format accretes, but the only writer truncates the index, and the one accretion on record was a hand merge
- Where: crates/before-fuelscape/src/dump.rs:118-129 (related: crates/before-fuelscape/src/dump.rs:255-261; crates/before-fuelscape/src/bin/fuelscape.rs:161-162; justfile:680-691; crates/before-fuelscape/src/render.rs:54-61; crates/before-fuelscape/src/lib.rs:47-53; crates/before-fuelscape/src/compact.rs:79-81)
- Class / severity / confidence: feature-gap / medium / high
- Provenance: verified (grep finds no open/append entry in dump.rs or bin/fuelscape.rs; `git show --stat 3c64f08a` touches only data files and doc-attachment lines, and its body says "The dump merge appends the four op documents and their index entries"; a read-only script over the committed dump: 104 ops, 100 stamped f77011e3 and 4 stamped 46eb64f9, no plain `atlas.json` present); executed: no (the central claim, no append path, is by reading)
- Seen by: scaffolding [1], adequacy [22]; refutation: confirmed (and raised [22] to medium to agree); history: no rationale found (6f63edb7's owner ruling covers provenance shape only; the justfile's re-pin text predates it and documents only the full re-measure)
- Owner-gated: no

Every hole found becomes a committed check, never a convention held in memory. The format's own docs describe accretion as the workflow (render.rs:56-61, lib.rs:47-53, compact.rs:79-81) and the dataset did accrete, but `DumpWriter::new` always writes an empty index and `append` extends only its in-memory list, so a filtered `--dump` run into the committed dump directory replaces the index with one naming only the new ops; `parse` prefers a plain `atlas.json` over the committed `atlas.json.gz`, so that truncated index then shadows the committed one on every later read, and the failure surfaces only as 100 "Only in" lines from `fuelscape-verify`'s `diff -r`. The workflow that produced 3c64f08a lives only in that commit's body.

Evidence:

       118	    /// Open a dump in `dir` (created if absent) and write the empty
       119	    /// index.
       120	    pub fn new(dir: &Path, meta: RenderMeta) -> io::Result<DumpWriter> {
       121	        std::fs::create_dir_all(dir)?;
       122	        let writer = DumpWriter {
       123	            dir: dir.to_path_buf(),
       124	            meta,
       125	            ops: Vec::new(),
       126	        };
       127	        writer.write_index()?;
       128	        Ok(writer)
       129	    }
       ...
       258	        Err(e) if e.kind() == io::ErrorKind::NotFound => {
       259	            let mut gz = path.as_os_str().to_owned();
       260	            gz.push(".gz");

Resolution: add `DumpWriter::open(dir, meta)` that loads an existing index with `read`-grade strictness (gz-aware via `parse`), refuses a `RunParams` mismatch and a duplicate op name, and appends; make `new` refuse a directory already holding `atlas.json` or `atlas.json.gz`; have `parse` refuse when both the plain file and its `.gz` sibling exist; wire the open path as `--append-to <dump>` (or make `--dump` open-or-create) and document the accretion recipe (measure alone, gzip, append, `fuelscape-compact`) beside the full re-measure at justfile:680-691. Acceptance: a dump test appends a synthetic op to an existing dump and `read` returns the union in index order with the original documents byte-unchanged; a params-mismatch test and a plain-beside-gz test are refused naming the file; `DumpWriter::new` on a directory holding an index errors.
Construction: in crates/before-fuelscape, `cargo run --bin fuelscape -- version_tick --dump --samples 2 --max-bytes 4 --out dump`, then read `dump/atlas.json`: `ops` is `["version_tick"]` while 104 `.json.gz` documents sit unindexed, and `dump::read("dump")` now sees one op.

Synthesis: **adopt.** This is the one medium in the feature-gap class and the clearest case of a documented workflow with no tool behind it: three module docs (render.rs:56-61, lib.rs:47-53, compact.rs:79-81, all re-read at `9e5784fb`) describe accretion as the design, the justfile's re-pin recipe at 680-691 documents only the full re-measure, and the one accretion that happened was a hand merge recorded in a commit body. The three refusals the resolution names (params mismatch, duplicate op, plain-beside-gz) are what turn the hand recipe into a checked one; the plain-beside-gz refusal in `parse` matters on its own, because today a stray `atlas.json` silently shadows the committed `.gz` on every read. The fuelscape-render review's open question 3 asks the same and recommends tooling it.

### fuelscape-render-17: The stored `HeatGrid` has no regrid path, so any change to the aggregation constants makes the committed dump unreadable
- Where: crates/before-fuelscape/src/dump.rs:237-243 (related: crates/before-fuelscape/src/dump.rs:5-8, :25-27; crates/before-fuelscape/src/render.rs:223-224, :240-241, :297, :364; .agent-notes/2026-07-27-skyline-exposition/07-machine.typ:193-194)
- Class / severity / confidence: feature-gap / low / high
- Provenance: verified (grep for regrid/rebin across the crate and justfile hits only prose at dump.rs:27 and compact.rs:88; `render_op` recomputes `aggregate(data)` at render.rs:364 and never reads the stored grid; the only reader of `atlas.grid` outside the loader is the typst chapter, which reads a frozen v1 snapshot under `.agent-notes/.../data/fuelscape-8k-2000`, 64 files, `"version":1`, commit 2db1a20e); executed: no
- Seen by: scaffolding [0]; refutation: reframed (severity medium to low: the refusal is loud, the raw samples are intact, and the libm routing is required independently by `fuelscape-verify`); history: deliberate-and-holds for persisting and checking the grid (dump.rs:19-27, eda7cccd), with no ruling for or against a regrid path
- Owner-gated: yes (persisting the grid is a documented design decision)

The module promises replay "any font scale, any future styling" with no guest, but `read` refuses any document whose stored grid is not bit-equal to today's `aggregate`, which `FUEL_BINS` and the 0.55/0.4/0.7 axis margins shape; a change to any of them refuses every committed op document, and no tool path rewrites the grids from the samples the dump still holds. For the committed v2 dump the grid's only reader is the loader's own equality check.

Evidence:

       237	        if doc.grid != aggregate(&doc.op) {
       238	            return Err(malformed(
       239	                &op_path,
       240	                "stored grid does not match the grid recomputed from its samples \
       241	                 (the dump was altered, or it predates a change to the aggregation)",
       242	            ));
       243	        }

Resolution: add a `--regrid <dump>` mode (or a `DumpWriter` entry) that rewrites each op document's grid from its samples, so an aggregation change is a recipe beside `fuelscape-compact` rather than a hand migration; or move the grid out of the dump into a derived sidecar the way `compact.rs` already derives the widget form; in either case narrow the module doc's styling promise to what it excludes. Acceptance: changing `FUEL_BINS` and running `--regrid dump` then `--render-from dump` succeeds without hand-editing a committed document, and the dump tests still pin raw-sample round-trip and replay byte-identity.
Construction: change `FUEL_BINS` from 56 to 64 and run `cargo run --bin fuelscape -- --render-from dump --out target/x` in crates/before-fuelscape: every op document is refused with "stored grid does not match" and nothing in the tool repairs it.

Synthesis: **adopt** the doc narrowing now; **consider** the regrid path when an aggregation change is next wanted. The refusal is loud, the raw samples are intact, and the stored grid's only production reader is the loader's equality check (render.rs:364 recomputes `aggregate(data)`), so the missing path costs nothing until someone changes `FUEL_BINS` or an axis margin, and the module doc's promise of "any future styling" (dump.rs:5-8) is what needs correcting today: styling that changes the bin geometry is exactly what the format does not survive. Of the two shapes, the derived-sidecar form is the better one because it dissolves the equality check instead of adding a repair mode for it, but it reshapes a documented design decision (persisting and checking the grid, eda7cccd) and is the owner's call.

## Positives

Each item names who established it: "verified here" means re-read at `9e5784fb` in this pass; otherwise the report that established it is named and its claim is carried, not re-verified.

- The operation tables on `Clock` (lib.rs:27-34), `Version` (version.rs:50-56), `Party` (party.rs:38-46), and `Span` (span.rs:38-48), with the "Version vector or vector clock?" section (lib.rs:79-216), were enough for a stranger to write a whole application without opening private source, and every operator meant what the table said (fresh-eyes sweep, its run; the tables read as accurate by its verification pass).
- Linearity is a compile-time fact where it can be: `assert_not_impl_any!(Party: Clone, Copy)` (party.rs:74) and its `Clock` twin (clock.rs:62), plus the four static assertions that pin "no `Clock | Clock` in any borrow shape" and "no `&Clock` receiver" (clock.rs:1072-1082), each with the rejected cell's reason at the definition (api-audit and clock reports).
- Every fallible reunion hands identity back instead of dropping it: `Party::join`/`Clock::join` return the operand in `Err`, `join_all` returns the overlapping set, the fork iterators' `Drop` rejoins unconsumed shares, and `sync`/`sync_all` leave every clock untouched on overlap (api-audit sweep). It is why fresh-eyes-15's `Err(Clock)` should stay.
- The text, literal, and byte entry points each document the linearity hole at the entry (`Party::from_str` at party.rs:767-768, `Clock::from_str` at clock.rs:928-929, `Clock::decode` at clock.rs:768-772), and `dangerously_alias` states the exact protocol it exists for (api-audit sweep).
- `Span::decode`, `Rank::decode`, and `Ranked::decode` carry variant-by-variant `# Errors` sections naming the input class for each variant, including the flush-byte case (span/wire.rs:79-89, rank.rs:435-445, ranked.rs:241-247); `Rank::encode_to` (rank.rs:403-406) shows the one-line form for writers. They are the template the remaining fallible entries should meet (api-audit and fresh-eyes sweeps).
- `error.rs` draws the `Truncated`/`TrailingBits` boundary exactly at the flush-byte edge (69-85), and the sweep's hostile probes (empty input, a party alone, a value plus trailing bytes) each returned the documented variant with no panic (fresh-eyes sweep, its run). `Parse::Syntax`'s doc lists trailing input among syntax defects (error.rs:108-109, verified here), which is the rule codec-base-text-tree-18 asks the id parser to follow.
- `Version`'s manual `Hash` hashes the canonical bytes "consistently with `Eq`'s byte compare" and says so at the impl (version.rs:97-106, verified here); all four span verdict enums and `Coverage` derive `Hash` (verdict.rs:7, 24, 56, 74; query.rs:51, verified here). The consistency argument for `Span: Hash` is already written in the tree.
- `Ticks`'s type doc states its width discipline as a family ("every conversion *into* it is total ... every conversion *out* is explicit about width", ticks.rs:18-24, verified here) and `TooWide`'s doc names the alternatives for a wide count (`limbs`, `Display`; error.rs:37-43, verified here); api-audit-13 asks the impls to catch up with the prose.
- `Split::size_hint` (party/forks.rs:65-73) handles a `u64` count on a narrower `usize` with the standard `(usize::MAX, None)` spelling and a two-line reason (api-audit sweep); the `ExactSizeIterator` interaction that undoes it on 32-bit targets is a correctness finding (clock-17, party-13), not an API-shape one.
- `tests/doc_hidden.rs` and `tests/foreign_reexport.rs` pin the hidden and re-exported surfaces by name, and the rendered docs confirm no foreign re-export in before and exactly the documented `UBig` re-export in suanpan (api-audit sweep).
- `Ranked` did what its docs promise on the application's data: `BTreeMap` order over `ranked().encode()` bytes matched `Ord` on `Ranked`, causes sorted before effects across a thirteen-version fleet, and `Ranked::decode` recovered the version from the key alone (fresh-eyes sweep, its run).
- `Query`'s `Debug` renders the module's expression vocabulary, so a failed assertion reads as source (fresh-eyes sweep); the valuation law `rank(a|b) + rank(a&b) == rank(a) + rank(b)` and `lag(a,b) + lag(b,a) == distance(a,b)` held on the fleet (its run).
- suanpan's crate page is the strongest public prose in the two crates: every bound beside its derivation, hazards stated where a user meets them (`&mut self` on sign reads, the one-sided `is_literally_zero`, no `PartialEq` and why), and a "When not to reach for it" section; its first example was enough to carry a 2^128 boundary on the first try (suanpan report; fresh-eyes sweep, its run).
- The fuelscape dump reader recomputes every stored grid from the raw samples and refuses disagreement, with a committed known-bad demonstration (dump.rs:237-243; dump/tests.rs:170-200), and replay from a dump is held byte-identical to a direct render across the roster (fuelscape-render report). fuelscape-render-17 asks for the repair path that check implies, not for the check to go.

## Open questions for Finch

Deduplicated across the reports and sweeps; each carries a recommendation.

1. **Is the meter-feature surface held to the stable-API rule?** `pub mod meter` (with the `skyline` re-export, the board's `pub use` constants, and the counter readers) and `pub mod surface` compile only under `any(test, feature = "meter")`; five reports asked whether they are stable API or instrument surface (board-frame, codec-base-text-tree, skyline-sweep-place-masked, board-families-floors-judge, clippy-pedantic). The `encoded_bits` re-homing commit (05d87e1b) already speaks of "the instrument surface" as distinct from "the unconditional public API". Recommendation: rule that it is instrument surface, record the ruling at the `pub use` in lib.rs, and let future sweeps classify meter-surface changes as ungated.
2. **`#[must_use]`: type-level on `Party` and `Clock`, or method-level only?** (api-audit-1, clippy-pedantic-4.) The type-level form also fires on a discarded `dangerously_alias()` and on any statement-position constructor. Recommendation: type-level on both types with a reason string naming the loss, plus method-level on `Version::join`/`join_all`/`meet`/`meet_all`, `Rank::checked_sub`/`saturating_sub`, and `Party::without`; no site in before, suanpan, or rumors trips any of them today.
3. **The fork iterators' name** (api-audit-6, fresh-eyes-9): rename to something importable (`iter::ClockForks`/`PartyForks`, or define both in `iter.rs`), which is breaking, or fix the prose only? Recommendation: prose now (the two "see [`Forks`]" sentences become the public path), the rename queued for the next version that may break.
4. **Which `Parse` precedence is the notation's** (codec-base-text-tree-18): the id parser's per-node rule or the version parser's whole-pass rule? Recommendation: whole-pass, documented on `Parse`; it is the simpler statement and the reference parser needs only a `canonical` flag to mirror it.
5. **`#[non_exhaustive]` on the error enums before the first release** (api-audit-14)? Recommendation: on `Decode` only; `Parse` is closed by the grammar. Whichever way, name the representation-bound case in `NotCanonical`'s variant doc (the codec-bits review's open question 9 wants the same sentence for 32-bit width limits).
6. **Does suanpan's exact-touch contract freeze constant-factor improvements** (lib.rs:283-286, "a change to any operation's count is a breaking change")? It blocks the lazy zero spill in suanpan-10 and two other suanpan items. Recommendation, following the suanpan report: restate it as "deterministic and pinned; a count change is a versioned change named in its commit", then adopt the lazy spill with its witness.
7. **A human-readable serde form, and text forms for `Rank`, `Ranked`, and `Span`** (fresh-eyes open question 1; fresh-eyes-16). Today every type serializes as `serialize_bytes` of its canonical encoding, so serde_json carries a number array; branching on `is_human_readable()` would need text forms three types lack, and `Rank`'s `Display` does not parse. Recommendation: document the current representation now (fresh-eyes-3) and say `Rank`'s text form is one-way; decide `FromStr for Rank` and the serde form as one design item.
8. **`From<suanpan::UBig> for Ticks`?** Sixteen envelope-suite sites spell the conversion as `.to_string().parse::<Ticks>()` (envelopes-b-19, meter-core-5). Recommendation: no; the string entry point is the recorded design that keeps suanpan's type off before's stable surface. A test-local helper serves the suite.
9. **`Floor`/`Ceiling::coverage`** (fresh-eyes-6): add the delegating method, or only the pointer to `Query::from`? Recommendation: both; the pointer is unconditional, the method is additive and two lines per atom.
10. **`Clock::join`'s error form** (fresh-eyes-15): keep `Err(Clock)` alone, or add an `Overlap`-returning variant? Recommendation: keep the hand-back and document the `map_err(|_| Overlap)` idiom with its hazard (mapping drops the share); a variant that drops the clock in its error path is the leak `#[must_use]` exists to catch.
11. **`Rank::Display` under formatter flags** (rank-21): honour width and alignment for all three forms, or state that flags are ignored? Recommendation: honour them uniformly (`f.pad` for the fractional forms) and say which sign and zero flags apply where.
12. **The `Drained` newtype for `merge_into_wider`** (suanpan-tests-8): worth a public reshaping? Recommendation: decide after suanpan-tests-16's swap deletion lands; if not adopted, the existing doc sentence already carries the obligation.
13. **Tool the fuelscape dump's accretion and regrid paths** (fuelscape-render-15, -17)? Recommendation: accretion yes, now (the hand merge has no check for run-parameter agreement or duplicate names until `fuelscape-verify` diffs); regrid when an aggregation change is next wanted, with the module doc's styling promise narrowed today.
14. **Three API decisions the audit raised without a finding.** (a) `const fn` on `Version::new`/`Party::seed`/`Clock::seed`: constness hinges on `Bits::from_canonical`'s `debug_assert!`; recommendation: low priority, a nicety with no consumer asking. (b) `encoded_len()` on `Clock` and `Span`, whose only sizing path is `encode()` or a counting writer: recommendation: add it only if a rumors framing path needs it (the dependence sweep flagged none); otherwise document the counting-writer spelling. (c) The reason `Query` has no `PartialEq` (api-audit-9, fresh-eyes-8, span-causally-38): the deleted `causally.rs` section at db9dfa3e carries the argument (the hole antichain is stored in construction order, so structurally different normal forms denote one predicate); recommendation: keep the absence and restore the sentence to `Query`'s type docs.
15. **suanpan's `claims` roster** is `#[cfg(test)] mod claims;` at lib.rs:364-365, while the audit's brief listed it as public API. Recommendation: the brief's premise was stale; keep it private unless an instrument crate needs to bind to it the way before's `surface` is bound under `meter`.

## Counts

Twenty-one entries: seventeen api-surprise, four feature-gap. No high.

| Severity | api-surprise | feature-gap | Total |
|---|---|---|---|
| high | 0 | 0 | 0 |
| medium | 1 | 1 | 2 |
| low | 8 | 2 | 10 |
| nit | 8 | 1 | 9 |
| **Total** | **17** | **4** | **21** |

| Module section | Entries | Ids |
|---|---|---|
| Crate root: the error module | 1 | fresh-eyes-5 |
| Crate root: Party and Clock, with the fork iterators | 5 | api-audit-1, clippy-pedantic-4, api-audit-6, fresh-eyes-9, fresh-eyes-15 |
| Crate root: Version core (Ticks) | 2 | fresh-eyes-14, api-audit-13 |
| Crate root: Rank | 3 | rank-21, api-audit-14, fresh-eyes-16 |
| Crate root: Span and causally | 3 | span-causally-1, api-audit-11, fresh-eyes-6 |
| Codec: bits and buf | 1 | codec-bits-11 |
| Codec: base, text, and tree | 1 | codec-base-text-tree-18 |
| suanpan | 2 | suanpan-10, suanpan-tests-8 |
| Instruments: fuelscape | 3 | fuelscape-pipeline-14, fuelscape-render-15, fuelscape-render-17 |
| **Total** | **21** | |

Dispositions: **adopt** outright, 12 (fresh-eyes-5, api-audit-1, clippy-pedantic-4, fresh-eyes-14, api-audit-13, rank-21, span-causally-1, api-audit-11, codec-bits-11, codec-base-text-tree-18, fuelscape-pipeline-14, fuelscape-render-15); **adopt the unconditional half, consider the owner-gated half**, 6 (api-audit-6 and fresh-eyes-9: the doc sentence now, the rename later; api-audit-14: the variant doc now, `#[non_exhaustive]` before release; fresh-eyes-16: the one-way statement now, `FromStr` with the serde decision; fresh-eyes-6: the pointer now, the method as an additive change; fuelscape-render-17: the doc narrowing now, the regrid path when needed); **adopt the doc, decline the alternative**, 1 (fresh-eyes-15: the `map_err` idiom documented, the `Overlap`-returning `join` declined because it would drop a share); **consider**, 2 (suanpan-10, contingent on open question 6; suanpan-tests-8, after suanpan-tests-16). One extension nobody filed as a finding is declined under open question 8 (`From<UBig> for Ticks`). Owner-gated entries: 18 of 21 (all but codec-bits-11, fuelscape-pipeline-14, and fuelscape-render-15).
